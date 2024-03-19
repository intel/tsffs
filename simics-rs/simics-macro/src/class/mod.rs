// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use darling::{
    ast::{Data, NestedMeta},
    util::Flag,
    Error, FromDeriveInput, FromField, FromMeta, Result,
};
use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse, parse_macro_input, parse_quote, Attribute, DeriveInput, Expr, Fields, FieldsNamed,
    GenericArgument, Generics, Ident, ItemStruct, Meta, PathArguments, Type,
};

#[derive(Debug, Clone, FromMeta)]
#[darling(and_then = "Self::validate")]
struct ClassAttribute {
    required: Flag,
    optional: Flag,
    pseudo: Flag,
    #[darling(default)]
    default: Option<Expr>,
}

impl ClassAttribute {
    fn validate(self) -> Result<Self> {
        // Check that only one of required, optional, pseudo is set
        if self.required.is_present() as u8
            + self.optional.is_present() as u8
            + self.pseudo.is_present() as u8
            > 1
            || self.required.is_present() as u8
                + self.optional.is_present() as u8
                + self.pseudo.is_present() as u8
                == 0
        {
            return Err(Error::custom(
                "Exactly one of `required`, `optional`, `pseudo` must be set",
            ));
        }

        // Make sure default is not set if required is set
        if self.required.is_present() && self.default.is_some() {
            return Err(Error::custom(
                "`default` cannot be set if `required` is set",
            ));
        }

        Ok(self)
    }

    fn attr_type(&self) -> TokenStream2 {
        if self.required.is_present() {
            quote!(simics::AttrAttr::Sim_Attr_Required)
        } else if self.optional.is_present() {
            quote!(simics::AttrAttr::Sim_Attr_Optional)
        } else if self.pseudo.is_present() {
            quote!(simics::AttrAttr::Sim_Attr_Pseudo)
        } else {
            unreachable!("Attribute is known to have exactly one type")
        }
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(class), forward_attrs(doc))]
struct ClassField {
    attrs: Vec<Attribute>,
    #[allow(unused)]
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
    #[darling(default)]
    attribute: Option<ClassAttribute>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(class),
    supports(struct_named),
    // NOTE: https://doc.rust-lang.org/reference/attributes.html#built-in-attributes-index
    forward_attrs(
        cfg,
        derive,
        allow,
        warn,
        deny,
        forbid,
        deprecated,
        must_use,
        doc,
        non_exhaustive,
    )
)]
struct ClassDeriveOpts {
    ident: Ident,
    generics: Generics,
    #[allow(unused)]
    data: Data<(), ClassField>,
    name: Option<String>,
    #[darling(default)]
    description: Option<String>,
    #[darling(default)]
    short_description: Option<String>,
    #[darling(default)]
    kind: Option<Type>,
    skip_alloc: Flag,
    skip_init: Flag,
    skip_finalize: Flag,
    skip_objects_finalize: Flag,
    skip_deinit: Flag,
    skip_dealloc: Flag,
    skip_create: Flag,
    attr_value: Flag,
}

impl ClassDeriveOpts {
    fn impl_alloc(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote!(impl #impl_generics simics::ClassAlloc for #ident #ty_generics #where_clause {})
    }

    fn impl_attribute_defaults(&self) -> Vec<TokenStream2> {
        if let Some(data) = self.data.as_ref().take_struct() {
            data.fields
                .iter()
                .filter_map(|f| {
                    f.attribute.as_ref().and_then(|a| {
                        let field_name = f.ident.as_ref().map(|n| n.to_string())?;

                        if let Some(default) = a.default.as_ref() {
                            Some(quote! {
                                simics::set_attribute_default(
                                    instance,
                                    #field_name,
                                    #default.try_into()?,
                                )?;
                            })
                        } else if a.optional.is_present() {
                            // We handle two kinds of f.ty:
                            // * `i64`, `u64`, `f64`, `String`, `bool` - simple types are
                            //   directly used
                            // * `BTreeSet<i64>` - complex types are transformed into e.g.
                            //   `BTreeSet::<i64>::default()`
                            let field_ty = &f.ty;

                            let new_field_ty = if let Type::Path(mut p) = f.ty.clone() {
                                if let Some(segment) = p.path.segments.last() {
                                    if segment.arguments.is_empty() {
                                        quote!(#field_ty)
                                    } else {
                                        let args = p
                                            .path
                                            .segments
                                            .last()
                                            .expect("Known to exist here")
                                            .arguments
                                            .clone();
                                        p.path
                                            .segments
                                            .last_mut()
                                            .expect("Known to exist here")
                                            .arguments = PathArguments::None;
                                        let ty = Type::Path(p);
                                        quote!(#ty::#args)
                                    }
                                } else {
                                    quote!(#field_ty)
                                }
                            } else {
                                quote!(#field_ty)
                            };

                            Some(quote! {
                                simics::set_attribute_default(
                                    instance,
                                    #field_name,
                                    #new_field_ty::default().try_into()?,
                                )?;
                            })
                        } else {
                            None
                        }
                    })
                })
                .collect()
        } else {
            vec![]
        }
    }

    fn impl_init(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let maybe_defaults = self.impl_attribute_defaults();
        quote! {
            impl #impl_generics simics::ClassInit for #ident #ty_generics #where_clause {
                unsafe fn init(instance: *mut simics::ConfObject) -> simics::Result<*mut simics::ConfObject> {
                    let ptr: *mut #ident #ty_generics = instance as *mut #ident #ty_generics;
                    unsafe { std::ptr::addr_of_mut!(*ptr).write(Self {
                        conf_object: *instance,
                        ..Default::default()
                    }) };
                    #(#maybe_defaults)*
                    Ok(ptr as *mut simics::ConfObject)
                }
            }
        }
    }

    fn impl_finalize(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote!(impl #impl_generics simics::ClassFinalize for #ident #ty_generics #where_clause {})
    }

    fn impl_objects_finalize(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote!(impl #impl_generics simics::ClassObjectsFinalize for #ident #ty_generics #where_clause {})
    }

    fn impl_deinit(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote!(impl #impl_generics simics::ClassDeinit for #ident #ty_generics #where_clause {})
    }

    fn impl_dealloc(&self) -> TokenStream2 {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote!(impl #impl_generics simics::ClassDealloc for #ident #ty_generics #where_clause {})
    }

    fn impl_name(&self) -> TokenStream2 {
        let name = &self.ident;
        let class_name = self.name.as_ref().cloned().unwrap_or(name.to_string());
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                /// The name of the class
                pub const NAME: &'static str = #class_name;
            }

        }
    }

    /// The equivalent python type for simple rust types (integers, strings, etc.) including
    /// most simple type aliases defined in the Simics API
    fn attribute_to_typestring_simple(s: &str) -> TokenStream2 {
        match s {
            "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "usize" | "isize"
            | "BreakpointId" | "PhysicalAddress" | "LogicalAddress" | "GenericAddress"
            | "HapHandle" => {
                quote!(simics::TypeStringType::Integer)
            }
            "f32" | "f64" => {
                quote!(simics::TypeStringType::Float)
            }
            "String" | "PathBuf" => {
                quote!(simics::TypeStringType::String)
            }
            "bool" => {
                quote!(simics::TypeStringType::Boolean)
            }
            // NOTE: We use `Any` for all other types, including complex types. This can cause
            // issues, because there is no type checking, but it ensures that we don't miss any
            // types.
            _ => quote!(simics::TypeStringType::Any),
        }
    }

    fn ty_to_typestring(ty: &Type) -> (TokenStream2, Option<TokenStream2>) {
        match ty {
            Type::Path(p) => {
                let Some(segment) = &p.path.segments.last() else {
                    return (
                        Error::custom(format!(
                            "Unsupported type for attribute (final path segment missing): {}",
                            ty.to_token_stream()
                        ))
                        .write_errors(),
                        None,
                    );
                };

                if segment.arguments.is_empty() {
                    (
                        Self::attribute_to_typestring_simple(segment.ident.to_string().as_str()),
                        None,
                    )
                } else {
                    match segment.ident.to_string().as_str() {
                        "Vec" | "BTreeSet" => {
                            let inner = match &segment.arguments {
                                PathArguments::AngleBracketed(args) => {
                                    let Some(inner) = args.args.first() else {
                                        return (
                                            Error::custom(format!(
                                                "Unsupported type for attribute (missing inner argument): {}",
                                                ty.to_token_stream()
                                            ))
                                            .write_errors(),
                                            None,
                                        );
                                    };
                                    match inner {
                                        GenericArgument::Type(ty) => {
                                            Self::attribute_to_typestring_simple(
                                                ty.to_token_stream().to_string().as_str(),
                                            )
                                        }
                                        _ => Error::custom(format!(
                                            "Unsupported type for attribute (invalid inner type): {}",
                                            ty.to_token_stream()
                                        ))
                                        .write_errors(),
                                    }
                                }
                                _ => Error::custom(format!(
                                    "Unsupported type for attribute (unsupported inner segment argument type): {}",
                                    ty.to_token_stream()
                                ))
                                .write_errors(),
                            };
                            (
                                quote!(simics::TypeStringType::List(
                                    vec![simics::TypeStringListType::ZeroOrMore(Box::new(#inner))]
                                )),
                                Some(quote!(simics::TypeStringType::Integer)),
                            )
                        }
                        "HashMap" | "BTreeMap" => {
                            let key_inner = match &segment.arguments {
                                PathArguments::AngleBracketed(args) => {
                                    let Some(inner) = args.args.first() else {
                                        return (
                                            Error::custom(format!(
                                                "Unsupported type for attribute (missing inner angle bracketed arg): {}",
                                                ty.to_token_stream()
                                            ))
                                            .write_errors(),
                                            None,
                                        );
                                    };
                                    match inner {
                                        GenericArgument::Type(ty) => {
                                            Self::attribute_to_typestring_simple(
                                                ty.to_token_stream().to_string().as_str(),
                                            )
                                        }
                                        _ => Error::custom(format!(
                                            "Unsupported type for attribute (invalid inner type): {}",
                                            ty.to_token_stream()
                                        ))
                                        .write_errors(),
                                    }
                                }
                                _ => Error::custom(format!(
                                    "Unsupported type for attribute (unsupported inner segment argument type): {}",
                                    ty.to_token_stream()
                                ))
                                .write_errors(),
                            };

                            (quote!(simics::TypeStringType::Dictionary), Some(key_inner))
                        }
                        "Option" => {
                            let inner = match &segment.arguments {
                                PathArguments::AngleBracketed(args) => {
                                    let Some(inner) = args.args.first() else {
                                        return (
                                            Error::custom(format!(
                                                "Unsupported type for attribute: {}",
                                                ty.to_token_stream()
                                            ))
                                            .write_errors(),
                                            None,
                                        );
                                    };
                                    match inner {
                                        GenericArgument::Type(ty) => {
                                            Self::attribute_to_typestring_simple(
                                                ty.to_token_stream().to_string().as_str(),
                                            )
                                        }
                                        _ => Error::custom(format!(
                                            "Unsupported type for attribute: {}",
                                            ty.to_token_stream()
                                        ))
                                        .write_errors(),
                                    }
                                }
                                _ => Error::custom(format!(
                                    "Unsupported type for attribute: {}",
                                    ty.to_token_stream()
                                ))
                                .write_errors(),
                            };

                            (
                                quote!(simics::TypeStringType::Or(Box::new(#inner), Box::new(simics::TypeStringType::Nil))),
                                None,
                            )
                        }
                        _ => (quote!(simics::TypeStringType::Any), None),
                    }
                }
            }
            _ => (
                Error::custom(format!(
                    "Unsupported type for attribute (type is not a path): {}",
                    ty.to_token_stream()
                ))
                .write_errors(),
                None,
            ),
        }
    }

    fn impl_attributes(&self) -> Vec<TokenStream2> {
        let struct_ident = &self.ident;
        let (_impl_generics, ty_generics, _where_clause) = self.generics.split_for_impl();
        if let Some(data) = self.data.as_ref().take_struct() {
            data.fields.iter().filter_map(|f| {
                f.attribute.as_ref().and_then(|a| {
                    if let Some(ref ident) = f.ident.as_ref() {
                        let ty = &f.ty;
                        // Check if type is an integer
                        let (tystr, indextystr) = Self::ty_to_typestring(ty);

                        let indextystr = if let Some(indextystr) = indextystr {
                            quote!(Some(#indextystr))
                        } else {
                            quote!(None)
                        };

                        let ident_string = ident.to_string();
                        let mut doc_attrs = f.attrs.iter().filter(|a| a.path().is_ident("doc")).filter_map(|a| match &a.meta {
                            Meta::NameValue(m) => Some(m.value.to_token_stream().to_string().trim().to_string()),
                            _ => None
                        }).collect::<Vec<_>>();

                        if doc_attrs.is_empty() {
                            doc_attrs.push(ident_string.clone());
                        }

                        let doc_attrs_string = doc_attrs.join(" ");

                        let attr_type = a.attr_type();

                        Some(quote!{
                            unsafe {
                                simics::register_typed_attribute(
                                    cls,
                                    #ident_string,
                                    Some(|o: *mut simics::ConfObject, i: simics::AttrValue| -> simics::Result<simics::AttrValue> {
                                        let slf =  unsafe { <#struct_ident #ty_generics as simics::FromConfObject>::from_conf_object(o) };
                                        let res = slf.#ident.clone().try_into()?;
                                        Ok(res)
                                    }),
                                    Some(|o: *mut simics::ConfObject, v: simics::AttrValue, i: simics::AttrValue| -> simics::Result<simics::SetErr> {
                                        let slf =  unsafe { <#struct_ident #ty_generics as simics::FromConfObject>::from_conf_object_mut(o) };
                                        let v: #ty = match v.try_into() {
                                            Ok(v) => v,
                                            Err(e) => {
                                                simics::error!(o, "Failed to convert attribute value {v:?} to type: {}", e);
                                                return Ok(simics::SetErr::Sim_Set_Illegal_Type)
                                            },
                                        };

                                        slf.#ident = v;

                                        Ok(simics::SetErr::Sim_Set_Ok)
                                    }),
                                    #attr_type,
                                    Some(#tystr),
                                    #indextystr,
                                    #doc_attrs_string
                                )?;
                            };
                        })
                    } else {
                        None
                    }
                })
            }).collect()
        } else {
            vec![]
        }
    }

    fn impl_create(&self) -> TokenStream2 {
        let name = &self.ident;
        let alloc_fn_name = format_ident!("{}_alloc", &name);
        let init_fn_name = format_ident!("{}_init", &name);
        let finalize_fn_name = format_ident!("{}_finalize", &name);
        let objects_finalized_fn_name = format_ident!("{}_objects_finalized", &name);
        let deinit_fn_name = format_ident!("{}_deinit", &name);
        let dealloc_fn_name = format_ident!("{}_dealloc", &name);
        let class_name = self.name.as_ref().cloned().unwrap_or(name.to_string());
        let description = self
            .description
            .as_ref()
            .cloned()
            .unwrap_or(name.to_string());
        let short_description = self
            .short_description
            .as_ref()
            .cloned()
            .unwrap_or(name.to_string());
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let kind = self
            .kind
            .as_ref()
            .map(|k| quote!(#k))
            .unwrap_or(quote!(simics::ClassKind::Sim_Class_Kind_Vanilla));

        let description = format!("c\"{}\"", description)
            .parse::<Literal>()
            .unwrap_or_else(|_| unreachable!("Failed to parse C string literal"));

        let short_description = format!("c\"{}\"", short_description)
            .parse::<Literal>()
            .unwrap_or_else(|_| unreachable!("Failed to parse C string literal"));

        let attributes_impl = self.impl_attributes();

        quote! {

            impl #impl_generics #name #ty_generics #where_clause {
                const CLASS: simics::ClassInfo = simics::ClassInfo {
                    alloc: Some(#alloc_fn_name),
                    init: Some(#init_fn_name),
                    finalize: Some(#finalize_fn_name),
                    objects_finalized: Some(#objects_finalized_fn_name),
                    deinit: Some(#deinit_fn_name),
                    dealloc: Some(#dealloc_fn_name),
                    description: #description.as_ptr(),
                    short_desc: #short_description.as_ptr(),
                    kind: #kind,
                };
            }

            impl #impl_generics simics::ClassCreate for #name #ty_generics #where_clause {
                fn create() -> simics::Result<*mut simics::ConfClass> {
                    let mut cls = simics::create_class(#class_name, #name::CLASS)?;
                    #( #attributes_impl )*

                    Ok(cls)
                }
            }
        }
    }

    fn impl_new(&self) -> TokenStream2 {
        let ident = &self.ident;

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                #[allow(clippy::too_many_arguments)]
                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                fn new(
                    obj: *mut simics::ConfObject,
                    value: #ident #ty_generics
                ) -> *mut simics::ConfObject {
                    let ptr: *mut #ident #ty_generics = obj as *mut #ident #ty_generics;
                    unsafe { std::ptr::addr_of_mut!(*ptr).write(value) };
                    ptr as *mut simics::ConfObject
                }

            }
        }
    }

    fn impl_ffi(&self) -> TokenStream2 {
        let name = &self.ident;
        let alloc_fn_name = format_ident!("{}_alloc", &name);
        let init_fn_name = format_ident!("{}_init", &name);
        let finalize_fn_name = format_ident!("{}_finalize", &name);
        let objects_finalized_fn_name = format_ident!("{}_objects_finalized", &name);
        let deinit_fn_name = format_ident!("{}_deinit", &name);
        let dealloc_fn_name = format_ident!("{}_dealloc", &name);
        let name_string = self.ident.to_string();

        quote! {
            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `alloc` method to ensure that the pointer is valid.
            pub unsafe extern "C" fn #alloc_fn_name(cls: *mut simics::ConfClass) -> *mut simics::ConfObject {
                let cls: *mut simics::ConfClass = cls.into();
                let obj: *mut simics::ConfObject  = unsafe { <#name as simics::ClassAlloc>::alloc::<#name>(cls) }
                    .unwrap_or_else(|e| panic!("{}::alloc failed: {}", #name_string, e))
                    .into();
                obj
            }

            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `init` method to ensure that the pointer is valid.
            pub extern "C" fn #init_fn_name(obj: *mut simics::ConfObject) -> *mut std::ffi::c_void {
                let ptr: *mut simics::ConfObject = unsafe { <#name as simics::ClassInit>::init(obj.into()) }
                    .unwrap_or_else(|e| panic!("{}::init failed: {}", #name_string, e))
                    .into();
                ptr as *mut std::ffi::c_void
            }

            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `finalize` method to ensure that the pointer is valid.
            pub extern "C" fn #finalize_fn_name(obj: *mut simics::ConfObject) {
                unsafe { <#name as simics::ClassFinalize>::finalize(obj.into()) }
                    .unwrap_or_else(|e| panic!("{}::finalize failed: {}", #name_string, e));
            }

            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `objects_finalized` method to ensure that the pointer is valid.
            pub extern "C" fn #objects_finalized_fn_name(obj: *mut simics::ConfObject) {
                unsafe { <#name as simics::ClassObjectsFinalize>::objects_finalized(obj.into()) }
                    .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", #name_string, e));
            }

            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `deinit` method to ensure that the pointer is valid.
            pub extern "C" fn #deinit_fn_name(obj: *mut simics::ConfObject) {
                unsafe { <#name as simics::ClassDeinit>::deinit(obj.into()) }
                    .unwrap_or_else(|e| panic!("{}::deinit failed: {}", #name_string, e));
            }

            #[no_mangle]
            #[allow(non_snake_case)]
            /// FFI wrapper
            ///
            /// # Safety
            ///
            /// This function is unsafe because it may dereference a raw pointer. It is up to the
            /// implementation of the class's `dealloc` method to ensure that the pointer is valid.
            pub extern "C" fn #dealloc_fn_name(obj: *mut simics::ConfObject) {
                unsafe { <#name as simics::ClassDealloc>::dealloc(obj.into()) }
                    .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", #name_string, e));
            }
        }
    }
}

impl ToTokens for ClassDeriveOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        if !self.skip_alloc.is_present() {
            let alloc_impl = self.impl_alloc();
            tokens.extend(quote! {
                #alloc_impl
            });
        }

        if !self.skip_init.is_present() {
            let init_impl = self.impl_init();
            tokens.extend(quote! {
                #init_impl
            });
        }

        if !self.skip_finalize.is_present() {
            let finalize_impl = self.impl_finalize();
            tokens.extend(quote! {
                #finalize_impl
            });
        }

        if !self.skip_objects_finalize.is_present() {
            let objects_finalize_impl = self.impl_objects_finalize();
            tokens.extend(quote! {
                #objects_finalize_impl
            });
        }

        if !self.skip_deinit.is_present() {
            let deinit_impl = self.impl_deinit();
            tokens.extend(quote! {
                #deinit_impl
            });
        }

        if !self.skip_dealloc.is_present() {
            let dealloc_impl = self.impl_dealloc();
            tokens.extend(quote! {
                #dealloc_impl
            });
        }

        if !self.skip_create.is_present() {
            let create_impl = self.impl_create();
            tokens.extend(quote! {
                #create_impl
            });
        }

        let name_impl = self.impl_name();
        let new_impl = self.impl_new();
        let ffi_impl = self.impl_ffi();
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        tokens.extend(quote! {
            #name_impl
            #new_impl
            #ffi_impl
            impl #impl_generics simics::Class for #ident #ty_generics #where_clause {}
        });
    }
}

pub fn class_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let args = match ClassDeriveOpts::from_derive_input(&input) {
        Ok(args) => args,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}

pub fn class_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    if parse::<FieldsNamed>(input.clone()).is_ok() {
        return quote!().into();
    }

    let args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => {
            return Error::custom(format!("Error parsing class arguments as meta list: {e}"))
                .write_errors()
                .into();
        }
    };

    let derive_input: TokenStream = if args.is_empty() {
        let input: TokenStream2 = input.clone().into();
        quote!(#[class] #input).into()
    } else {
        let args = args.clone();
        let input: TokenStream2 = input.clone().into();
        let class_attr = quote!(#[class(#(#args),*)]);
        quote!(#class_attr #input).into()
    };

    let derive_input = parse_macro_input!(derive_input as DeriveInput);

    let derive_args = match ClassDeriveOpts::from_derive_input(&derive_input) {
        Ok(args) => args,
        Err(e) => return e.write_errors().into(),
    };

    let mut input = parse_macro_input!(input as ItemStruct);

    if let Fields::Named(ref mut fields) = input.fields {
        if fields
            .named
            .first()
            .is_some_and(|f| f.ty != parse_quote!(simics::ConfObject))
        {
            if derive_args.attr_value.is_present() {
                fields.named.insert(
                    0,
                    parse_quote! {
                        #[attr_value(skip)]
                        conf_object: simics::ConfObject
                    },
                );
            } else {
                fields.named.insert(
                    0,
                    parse_quote! {
                        conf_object: simics::ConfObject
                    },
                );
            }
        }
    };

    let derive: TokenStream2 = if args.is_empty() {
        class_derive_impl(quote!(#[class] #input).into()).into()
    } else {
        class_derive_impl(quote!(#[class(#(#args),*)] #input).into()).into()
    };

    if let Fields::Named(ref mut fields) = input.fields {
        // Remove `class` attributes from all fields
        fields.named.iter_mut().for_each(|f| {
            f.attrs.retain(|a| !a.path().is_ident("class"));
        });
    }

    // println!("{}", derive);

    let input: TokenStream2 = quote!(#input);

    // println!("{}", input);

    quote!(
        #derive
        #[repr(C)]
        #input
    )
    .into()
}
