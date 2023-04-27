//! Derive/attribute macros for simics-api
//!
//! Provides the `#[module()]` and `#[derive(Module)]` macros

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};
use syn::{
    parse::{Parse, ParseStream, Parser, Result},
    parse_macro_input,
    punctuated::Punctuated,
    Expr, Field, Fields, Ident, ItemStruct, LitStr, Token,
};

#[derive(Clone)]
enum ModuleAttrValue {
    LitStr(LitStr),
    Expr(Expr),
    // Call(Vec<Expr>),
}

impl ToTokens for ModuleAttrValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::LitStr(t) => t.to_tokens(tokens),
            Self::Expr(t) => t.to_tokens(tokens),
            // Self::Call(t) => {
            //     let t = quote!(#(#t),*);
            //     t.to_tokens(tokens)
            // }
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
enum ModuleAttrType {
    Derive,
    ClassName,
    Description,
    ShortDescription,
    ClassKind,
}

struct ModuleAttr {
    typ: ModuleAttrType,
    value: Option<ModuleAttrValue>,
}

impl Hash for ModuleAttr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.typ.hash(state);
    }
}

impl PartialEq for ModuleAttr {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
    }
}

impl Eq for ModuleAttr {}

impl Parse for ModuleAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr: Ident = input.parse()?;

        let typ = match attr.to_string().as_str() {
            "class_name" => ModuleAttrType::ClassName,
            "derive" => ModuleAttrType::Derive,
            "description" => ModuleAttrType::Description,
            "short_description" => ModuleAttrType::ShortDescription,
            "class_kind" => ModuleAttrType::ClassKind,
            &_ => abort! {
                attr,
                r#"Attributes must be one of `derive`, \
                `name = "class_name"`, \
                `description = "YOUR DESCRIPTION HERE"`, \
                `short_description = "YOUR CLASS DESCRIPTION HERE"`, \
                `kind = ClassKind::KIND`"#
            },
        };

        let value = if input.peek(Token![=]) {
            let assign_token = input.parse::<Token![=]>()?;

            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                Some(ModuleAttrValue::LitStr(lit))
            } else if let Ok(expr) = input.parse::<Expr>() {
                Some(ModuleAttrValue::Expr(expr))
            } else {
                abort! {
                    assign_token,
                    "expected `string literal` or `expression` after `=`"
                };
            }
        } else {
            None
        };

        Ok(Self { typ, value })
    }
}

struct Args {
    attrs: HashSet<ModuleAttr>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let parsed = Punctuated::<ModuleAttr, Token![,]>::parse_terminated(input)?;

        let attrs: HashSet<ModuleAttr> = parsed.into_iter().collect();

        if !attrs.contains(&ModuleAttr {
            typ: ModuleAttrType::ClassName,
            value: None,
        }) {
            let span = input.span();
            abort! {
                span,
                r#"`class_name` required in `module()` invocation. Try giving your class name like `#[module(class_name = "class_name")]`"#
            };
        }

        Ok(Args { attrs })
    }
}

impl Args {
    fn class_name(&self) -> TokenStream2 {
        if let Some(name_attr) = self.attrs.get(&ModuleAttr {
            typ: ModuleAttrType::ClassName,
            value: None,
        }) {
            if let Some(ModuleAttrValue::LitStr(name)) = &name_attr.value {
                return quote! { #name };
            } else if let Some(ModuleAttrValue::Expr(name)) = &name_attr.value {
                return quote! { #name };
            }
        }
        unreachable!("No name provided and check somehow failed");
    }

    fn has_derive(&self) -> bool {
        self.attrs.contains(&ModuleAttr {
            typ: ModuleAttrType::Derive,
            value: None,
        })
    }

    fn description(&self) -> Option<String> {
        if let Some(description) = self.attrs.get(&ModuleAttr {
            typ: ModuleAttrType::Description,
            value: None,
        }) {
            match &description.value {
                Some(ModuleAttrValue::LitStr(s)) => Some(s.value()),
                _ => None,
            }
        } else {
            None
        }
    }

    fn short_description(&self) -> Option<String> {
        if let Some(short_description) = self.attrs.get(&ModuleAttr {
            typ: ModuleAttrType::ShortDescription,
            value: None,
        }) {
            match &short_description.value {
                Some(ModuleAttrValue::LitStr(s)) => Some(s.value()),
                _ => None,
            }
        } else {
            None
        }
    }

    fn class_kind(&self) -> Option<Expr> {
        if let Some(class_kind) = self.attrs.get(&ModuleAttr {
            typ: ModuleAttrType::ClassKind,
            value: None,
        }) {
            match &class_kind.value {
                Some(ModuleAttrValue::Expr(e)) => Some(e.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[proc_macro_derive(Module)]
/// Derive the default implementation of [Module]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(input as ItemStruct);

    let name = &item_struct.ident;

    quote! {
        impl simics_api::Module for #name {}
    }
    .into()
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Attribute to add boilerplate to a `struct` to enable it to be used as a SIMICS Conf Object.
///
/// * Generate default implementations for CFFI to call functions defined in the [Module] trait
///   impl
/// * Insert a [ConfObject] field to permit instances of the struct to be passed via CFFI to and
///   from SIMICS
/// * Optionally, derive the default implementations of the [Module] trait
///
/// The macro accepts the following arguments:
///
/// * `class_name = "name"` (Required) specifies the generated class name the class will be registered with
/// * `derive` (Optional) which allows you to derive the default
///   implementation of [Module] alongside automatic implementations of the extern functions
///   required to register the class.
/// * `description = "describe your class"` (Optional) set a custom description for the generated
///   class. Defaults to the struct name.
/// * `short_description = "short desc"` (Optional) set a custom short description for the
///   generated class. Defaults to the struct name.
/// * `class_kind = ClassKind::Vanilla` (Optional) set a class kind. Most classes are Vanilla,
///   which is the default, but the kind can be set here.
///
/// # Examples
///
/// Without deriving [Module]:
///
/// ```text
/// #[macro_use]
/// extern crate simics_api_macro;
/// use simics_api_macro::module;
///
/// #[module(class_name = "test")]
/// struct Test {}
/// ```
///
/// Derive [Module]:
///
/// ```text
/// #[macro_use]
/// extern crate simics_api_macro;
/// use simics_api::Module;
///
/// use simics_api_macro::module;
///
/// #[module(derive, class_name = "test")]
/// struct Test {}
/// ```
/// Derive [Module] and customize the generated class description and kind:
///
/// ```text
/// #[macro_use]
/// extern crate simics_api_macro;
/// use simics_api::Module;
///
/// use simics_api_macro::module;
///
/// #[module(
///    derive,
///    class_name = "test_module_4",
///    description = "Test module 4",
///    short_description = "TM4",
///    class_kind = ClassKind::Session
/// )]
/// struct Test {}
/// ```
///
pub fn module(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);

    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let name = &item_struct.ident;

    // This needs to be generated first before we add the `ConfObject` field
    let raw_impl = raw_impl(name.to_string(), &item_struct.fields);

    if let Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.insert(
            0,
            Field::parse_named
                .parse2(quote!(conf_object: simics_api::ConfObject))
                .expect("Couldn't parse field `conf_object`"),
        );
    };

    // Only derive Module if we get a `derive` argument
    let derive_attribute = if args.has_derive() {
        quote! { #[derive(Module)] }
    } else {
        quote! {}
    };

    let ffi_impl = ffi_impl(name.to_string());
    let register_impl = create_impl(name.to_string(), &args);
    let from_impl = from_impl(name.to_string());

    /* let r: TokenStream = */
    quote! {
        #derive_attribute
        #[repr(C)]
        #item_struct
        #ffi_impl
        #register_impl
        #raw_impl
        #from_impl
    }
    .into()

    // let s = r.to_string();

    // eprintln!("{}", s);

    // r
}

fn ffi_impl<S: AsRef<str>>(name: S) -> TokenStream2 {
    let name_string = name.as_ref().to_string().to_ascii_lowercase();
    let name = format_ident!("{}", name.as_ref());
    let alloc_fn_name = format_ident!("{}_alloc", &name_string);
    let init_fn_name = format_ident!("{}_init", &name_string);
    let finalize_fn_name = format_ident!("{}_finalize", &name_string);
    let objects_finalized_fn_name = format_ident!("{}_objects_finalized", &name_string);
    let deinit_fn_name = format_ident!("{}_deinit", &name_string);
    let dealloc_fn_name = format_ident!("{}_dealloc", &name_string);

    quote! {
        #[no_mangle]
        pub extern "C" fn #alloc_fn_name(cls: *mut simics_api::ConfClass) -> *mut simics_api::ConfObject {
            let cls: simics_api::OwnedMutConfClassPtr = cls.into();
            let obj: *mut simics_api::ConfObject  = #name::alloc::<#name>(cls)
                .unwrap_or_else(|e| panic!("{}::alloc failed: {}", #name_string, e))
                .into();
            obj
        }

        #[no_mangle]
        pub extern "C" fn #init_fn_name(obj: *mut simics_api::ConfObject) -> *mut std::ffi::c_void {
            let ptr: *mut ConfObject = #name::init(obj.into())
                .unwrap_or_else(|e| panic!("{}::init failed: {}", #name_string, e))
                .into();
            ptr as *mut std::ffi::c_void
        }

        #[no_mangle]
        pub extern "C" fn #finalize_fn_name(obj: *mut simics_api::ConfObject) {
            #name::finalize(obj.into())
                .unwrap_or_else(|e| panic!("{}::finalize failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #objects_finalized_fn_name(obj: *mut simics_api::ConfObject) {
            #name::objects_finalized(obj.into())
                .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #deinit_fn_name(obj: *mut simics_api::ConfObject) {
            #name::deinit(obj.into())
                .unwrap_or_else(|e| panic!("{}::deinit failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #dealloc_fn_name(obj: *mut simics_api::ConfObject) {
            #name::dealloc(obj.into())
                .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", #name_string, e));
        }
    }
}

fn create_impl<S: AsRef<str>>(name: S, args: &Args) -> TokenStream2 {
    let name_string = name.as_ref().to_string().to_ascii_lowercase();
    let name = format_ident!("{}", name.as_ref());

    let alloc_fn_name = format_ident!("{}_alloc", &name_string);
    let init_fn_name = format_ident!("{}_init", &name_string);
    let finalize_fn_name = format_ident!("{}_finalize", &name_string);
    let objects_finalized_fn_name = format_ident!("{}_objects_finalized", &name_string);
    let deinit_fn_name = format_ident!("{}_deinit", &name_string);
    let dealloc_fn_name = format_ident!("{}_dealloc", &name_string);

    // TODO: Can we clean up the re-quoting of these strings?
    let class_name = args.class_name();

    let description = match args.description() {
        Some(description) => description,
        None => name_string.clone(),
    };

    let short_description = match args.short_description() {
        Some(short_description) => short_description,
        None => name_string,
    };

    let kind = match args.class_kind() {
        Some(kind) => quote! { #kind as u32 },
        None => quote! { simics_api::ClassKind::Vanilla as u32 },
    };

    quote! {
        impl #name {
            const CLASS: simics_api::ClassInfo = simics_api::ClassInfo {
                alloc: Some(#alloc_fn_name),
                init: Some(#init_fn_name),
                finalize: Some(#finalize_fn_name),
                objects_finalized: Some(#objects_finalized_fn_name),
                deinit: Some(#deinit_fn_name),
                dealloc: Some(#dealloc_fn_name),
                description: raw_cstr::c_str!(#description).as_ptr(),
                short_desc: raw_cstr::c_str!(#short_description).as_ptr(),
                kind: #kind,
            };

        }

        impl simics_api::Create for #name {
            fn create() -> anyhow::Result<simics_api::OwnedMutConfClassPtr> {
                simics_api::create_class(#class_name, #name::CLASS)
            }
        }
    }
}

fn raw_impl<S: AsRef<str>>(name: S, fields: &Fields) -> TokenStream2 {
    let name = format_ident!("{}", name.as_ref());

    let mut field_parameters = Vec::new();

    for field in fields {
        let ty = &field.ty;
        if let Some(ident) = &field.ident {
            field_parameters.push(quote! {
                #ident: #ty
            });
        }
    }

    let mut field_initializers = Vec::new();

    for field in fields {
        if let Some(ident) = &field.ident {
            field_initializers.push(quote! {
                unsafe { std::ptr::addr_of_mut!((*ptr).#ident).write(#ident) };
            })
        }
    }

    quote! {
        impl #name {
            fn new(
                obj: simics_api::OwnedMutConfObjectPtr,
                #(#field_parameters),*
            ) -> simics_api::OwnedMutConfObjectPtr  {
                let obj_ptr: *mut simics_api::ConfObject = obj.into();
                let ptr: *mut #name = obj_ptr as *mut #name;

                #(#field_initializers)*

                (ptr as *mut simics_api::ConfObject).into()
            }
        }
    }
}

fn from_impl<S: AsRef<str>>(name: S) -> TokenStream2 {
    let name = format_ident!("{}", name.as_ref());

    quote! {
        impl From<simics_api::OwnedMutConfObjectPtr> for &mut #name {
            fn from(value: simics_api::OwnedMutConfObjectPtr) -> Self {
                let obj_ptr: *mut simics_api::ConfObject = value.into();
                let ptr: *mut #name = obj_ptr as *mut #name;
                unsafe { &mut *ptr }
            }
        }
    }
}
