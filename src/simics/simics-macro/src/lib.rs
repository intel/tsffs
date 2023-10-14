// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Derive/attribute macros for simics-api
//!
//! Provides the `#[module()]` and `#[derive(Module)]` macros

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use darling::{ast::NestedMeta, util::Flag, Error, FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use std::{collections::HashMap, env::var, fs::read, path::PathBuf};
use syn::{
    parse::Parser, parse_file, parse_macro_input, parse_str, Expr, Field, Fields, GenericArgument,
    Generics, Ident, ImplGenerics, Item, ItemConst, ItemFn, ItemMod, ItemStruct, ItemType,
    PathArguments, ReturnType, Type, TypeGenerics, Visibility, WhereClause,
};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(module), supports(struct_named))]
struct ModuleDerive {
    ident: Ident,
    generics: Generics,
}

impl ToTokens for ModuleDerive {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            impl #impl_generics simics::traits::module::Module for #ident #ty_generics #where_clause {}
        })
    }
}

#[derive(Debug, FromMeta)]
struct ModuleOpts {
    class_name: Expr,
    derive: Flag,
    description: Option<String>,
    short_description: Option<String>,
    class_kind: Option<Type>,
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
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let mut input = parse_macro_input!(input as ItemStruct);

    let args = match ModuleOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields = &input.fields;

    let raw_impl = raw_impl(name, fields, &impl_generics, &ty_generics, &where_clause);

    // This needs to be generated first before we add the `ConfObject` field

    if let Fields::Named(ref mut fields) = input.fields {
        fields.named.insert(
            0,
            Field::parse_named
                .parse2(quote!(conf_object: simics::api::ConfObject))
                .expect("Couldn't parse field `conf_object`"),
        );
    };

    // Only derive Module if we get a `derive` argument
    let maybe_derive_attribute = args
        .derive
        .is_present()
        .then_some(quote!(#[derive(simics::traits::module::Module)]));

    let ffi_impl = ffi_impl(name.to_string());
    let register_impl = create_impl(
        name.to_string(),
        &args,
        &impl_generics,
        &ty_generics,
        &where_clause,
    );
    let from_impl = from_impl(name.to_string());

    quote! {
        #maybe_derive_attribute
        #[repr(C)]
        #input
        #ffi_impl
        #register_impl
        #raw_impl
        #from_impl
    }
    .into()
}

fn ffi_impl<S>(name: S) -> TokenStream2
where
    S: AsRef<str>,
{
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
        pub extern "C" fn #alloc_fn_name(cls: *mut simics::api::ConfClass) -> *mut simics::api::ConfObject {
            let cls: *mut simics::api::ConfClass = cls.into();
            let obj: *mut simics::api::ConfObject  = #name::alloc::<#name>(cls)
                .unwrap_or_else(|e| panic!("{}::alloc failed: {}", #name_string, e))
                .into();
            obj
        }

        #[no_mangle]
        pub extern "C" fn #init_fn_name(obj: *mut simics::api::ConfObject) -> *mut std::ffi::c_void {
            let ptr: *mut ConfObject = #name::init(obj.into())
                .unwrap_or_else(|e| panic!("{}::init failed: {}", #name_string, e))
                .into();
            ptr as *mut std::ffi::c_void
        }

        #[no_mangle]
        pub extern "C" fn #finalize_fn_name(obj: *mut simics::api::ConfObject) {
            #name::finalize(obj.into())
                .unwrap_or_else(|e| panic!("{}::finalize failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #objects_finalized_fn_name(obj: *mut simics::api::ConfObject) {
            #name::objects_finalized(obj.into())
                .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #deinit_fn_name(obj: *mut simics::api::ConfObject) {
            #name::deinit(obj.into())
                .unwrap_or_else(|e| panic!("{}::deinit failed: {}", #name_string, e));
        }

        #[no_mangle]
        pub extern "C" fn #dealloc_fn_name(obj: *mut simics::api::ConfObject) {
            #name::dealloc(obj.into())
                .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", #name_string, e));
        }
    }
}

fn create_impl<S>(
    name: S,
    args: &ModuleOpts,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: &Option<&WhereClause>,
) -> TokenStream2
where
    S: AsRef<str>,
{
    let name_string = name.as_ref().to_string().to_ascii_lowercase();
    let name = format_ident!("{}", name.as_ref());

    let alloc_fn_name = format_ident!("{}_alloc", &name_string);
    let init_fn_name = format_ident!("{}_init", &name_string);
    let finalize_fn_name = format_ident!("{}_finalize", &name_string);
    let objects_finalized_fn_name = format_ident!("{}_objects_finalized", &name_string);
    let deinit_fn_name = format_ident!("{}_deinit", &name_string);
    let dealloc_fn_name = format_ident!("{}_dealloc", &name_string);

    // TODO: Can we clean up the re-quoting of these strings?
    let class_name = &args.class_name;

    let description = args.description.as_ref().unwrap_or(&name_string);

    let short_description = args.short_description.as_ref().unwrap_or(&name_string);

    let kind = args
        .class_kind
        .as_ref()
        .map(|k| quote!(#k))
        .unwrap_or(quote!(simics::api::ClassKind::Sim_Class_Kind_Vanilla));

    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            const CLASS: simics::api::ClassInfo = simics::api::ClassInfo {
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

        impl #impl_generics simics::api::SimicsClassCreate for #name #ty_generics #where_clause {
            fn create() -> anyhow::Result<*mut simics::api::ConfClass> {
                simics::api::create_class(#class_name, #name::CLASS)
            }
        }
    }
}

fn raw_impl(
    name: &Ident,
    fields: &Fields,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: &Option<&WhereClause>,
) -> TokenStream2 {
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
        impl #impl_generics #name #ty_generics #where_clause {
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            fn new(
                obj: *mut simics::api::ConfObject,
                #(#field_parameters),*
            ) -> *mut simics::api::ConfObject  {

                let obj_ptr: *mut simics::api::ConfObject = obj.into();
                let ptr: *mut #name = obj_ptr as *mut #name;

                #(#field_initializers)*

                (ptr as *mut simics::api::ConfObject).into()
            }
        }
    }
}

fn from_impl<S>(name: S) -> TokenStream2
where
    S: AsRef<str>,
{
    let name = format_ident!("{}", name.as_ref());

    quote! {
        impl From<*mut simics::api::ConfObject> for &'static mut #name {
            fn from(value: *mut simics::api::ConfObject) -> Self {
                let ptr: *mut #name = value as *mut #name;
                unsafe { &mut *ptr }
            }
        }
    }
}

#[derive(Debug, FromMeta)]
struct SimicsExceptionOpts {}

trait IsResultType {
    fn is_result_type(&self) -> bool;
}

impl IsResultType for ReturnType {
    fn is_result_type(&self) -> bool {
        match self {
            ReturnType::Default => false,
            ReturnType::Type(_, ty) => match &**ty {
                Type::Path(p) => p
                    .path
                    .segments
                    .last()
                    .map(|l| l.ident == "Result")
                    .unwrap_or(false),
                _ => false,
            },
        }
    }
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Marks a function as being a SIMICS API that can throw exceptions. A SIMICS exception can be
/// generated by most APIs. This macro makes the function private, wraps it, and adds the
/// requisite code to check for and report exceptions. `clear_exception` should *not* be called
/// inside the wrapped function. `last_error` may be called, however, as any exceptions will be
/// cleared after the wrapped function returns.
///
/// # Examples
///
/// ```rust,ignore
/// #[simics_exception]
/// pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
///     unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
/// }
/// ```
///
/// Expands to:
///
/// ```rust,ignore
/// fn _write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) {
///     unsafe { SIM_write_byte(physical_memory, physical_addr, byte) };
/// }
///
/// pub fn write_byte(physical_memory: *mut ConfObject, physical_addr: u64, byte: u8) -> Result<()> {
///     let res = _write_byte(physical_memory, physical_addr, byte);
///
///     match simics::api::get_pending_exception() {
///         SimException::SimExc_No_Exception => Ok(()),
///         exception => {
///             clear_exception();
///             Err(Error::from(exception))
///         }
///     }
/// }
/// ```
pub fn simics_exception(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let mut input = parse_macro_input!(input as ItemFn);

    let _args = match SimicsExceptionOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    // Get the original ident and visibility before we change them
    let vis = input.vis.clone();
    let mut sig = input.sig.clone();

    let inner_ident = format_ident!("_{}", input.sig.ident);
    input.sig.ident = inner_ident.clone();
    input.vis = Visibility::Inherited;

    let ok_return = sig
        .output
        .is_result_type()
        .then_some(quote!(result))
        .unwrap_or(quote!(Ok(result)));

    sig.output = match sig.output.is_result_type().then_some(&sig.output) {
        Some(o) => o.clone(),
        None => {
            let output = match &sig.output {
                ReturnType::Default => quote!(()),
                ReturnType::Type(_, ty) => quote!(#ty),
            };

            match parse_str(&quote!(-> crate::error::Result<#output>).to_string()) {
                Ok(o) => o,
                Err(e) => return TokenStream::from(Error::from(e).write_errors()),
            }
        }
    };

    let maybe_ty_generics = (!&sig.generics.params.is_empty()).then_some({
        let params = &sig.generics.params;
        quote!(::<#params>)
    });

    let args = sig
        .inputs
        .iter()
        .map(|i| match i {
            syn::FnArg::Receiver(_) => {
                abort!(i, "Cannot apply attribute to function with a receiver")
            }
            syn::FnArg::Typed(t) => {
                let pat = &t.pat;
                quote!(#pat)
            }
        })
        .collect::<Vec<_>>();

    let wrapper = quote! {
        #vis #sig {
            let result = #inner_ident #maybe_ty_generics(#(#args),*);
            match crate::api::get_pending_exception() {
                crate::api::base::sim_exception::SimException::SimExc_No_Exception => #ok_return,
                exception => {
                    crate::api::base::sim_exception::clear_exception();
                    Err(crate::error::Error::SimicsException {
                        exception,
                        msg: crate::api::base::sim_exception::last_error()
                    })
                }
            }
        }

    };

    quote! {
        #input
        #wrapper
    }
    .into()
}

trait SnakeToCamel {
    fn snake_to_camel(&self) -> String;
}

impl SnakeToCamel for String {
    fn snake_to_camel(&self) -> String {
        let mut s = String::new();
        let mut upper = false;
        for c in self.chars() {
            if upper || s.is_empty() {
                s.push(c.to_ascii_uppercase());
                upper = false;
            } else if c == '_' {
                upper = true;
            } else {
                s.push(c.to_ascii_lowercase());
            }
        }
        s
    }
}

fn interface_field_to_method(field: &Field) -> Option<TokenStream2> {
    let vis = &field.vis;
    if let Some(name) = &field.ident {
        let name_string = name.to_string();
        if let Type::Path(ref p) = field.ty {
            if let Some(last) = p.path.segments.last() {
                if last.ident == "Option" {
                    if let PathArguments::AngleBracketed(ref args) = last.arguments {
                        if let Some(GenericArgument::Type(Type::BareFn(proto))) = args.args.first()
                        {
                            // NOTE: We `use crate::api::sys::*;` at the top of the module, otherwise
                            // we would need to rewrite all of the types on `inputs` here.
                            let inputs = &proto.inputs;
                            let input_names = inputs
                                .iter()
                                .filter_map(|a| a.name.clone().map(|n| n.0))
                                .collect::<Vec<_>>();
                            let output = match &proto.output {
                                ReturnType::Default => quote!(()),
                                ReturnType::Type(_, t) => quote!(#t),
                            };
                            // NOTE: We need to make a new name because in some cases the fn ptr name is the same as one of the parameter
                            // names
                            let some_name = format_ident!("{}_fn", name);
                            return Some(quote! {
                                #vis fn #name(&mut self, #inputs) -> crate::Result<#output> {
                                    if let Some(#some_name) = unsafe { *self.interface}.#name {
                                        Ok(unsafe { #some_name(#(#input_names),*) })
                                    } else {
                                        Err(crate::Error::NoInterfaceMethod { method: #name_string.to_string() })
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, FromMeta)]
struct SimicsInterfaceCodegen {
    source: String,
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Automatically generate high level bindings to all interfaces provided by SIMICS
pub fn simics_interface_codegen(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let codegen_args = match SimicsInterfaceCodegen::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let bindings_source_path = if let Ok(out_dir) = var("OUT_DIR") {
        PathBuf::from(out_dir).join(codegen_args.source)
    } else {
        return TokenStream::from(
            Error::custom("No environment variable OUT_DIR set").write_errors(),
        );
    };

    let bindings_source = if let Ok(bindings_source) = read(&bindings_source_path) {
        if let Ok(bindings_source) = String::from_utf8(bindings_source) {
            bindings_source
        } else {
            return TokenStream::from(
                Error::custom("Bindings source file was not UTF8").write_errors(),
            );
        }
    } else {
        return TokenStream::from(
            Error::custom("Failed to read bindings source file").write_errors(),
        );
    };

    let input = parse_macro_input!(input as ItemMod);
    let input_mod_vis = &input.vis;
    let input_mod_name = &input.ident;

    let parsed_bindings = match parse_file(&bindings_source) {
        Ok(b) => b,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let interface_name_items = parsed_bindings
        .items
        .iter()
        .filter_map(|i| {
            if let Item::Const(c) = i {
                if c.ident.to_string().ends_with("_INTERFACE") {
                    Some((c.ident.to_string(), c))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

    let interfaces = parsed_bindings
        .items
        .iter()
        .filter_map(|i| {
            if let Item::Struct(s) = i {
                interface_name_items
                    .get(&s.ident.to_string().to_ascii_uppercase())
                    .map(|interface_name_item| (interface_name_item, s))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

    let interface_structs = interfaces
        .iter()
        .map(|(name, interface)| {
            let camel_name = name.ident.to_string().snake_to_camel();
            let struct_name = format_ident!("{camel_name}",);
            let interface_ident = &interface.ident;
            let name_ident = &name.ident;
            let interface_methods = interface
                .fields
                .iter()
                .filter_map(interface_field_to_method)
                .collect::<Vec<_>>();
            let q = quote! {
                pub struct #struct_name {
                    interface: *mut crate::api::sys::#interface_ident,
                }

                impl #struct_name {
                    #(#interface_methods)*
                }

                impl crate::api::traits::interface::Interface for #struct_name {
                    type Interface = crate::api::sys::#interface_ident;
                    type Name = &'static [u8];

                    const NAME: &'static [u8] = crate::api::sys::#name_ident;

                    fn new(interface: *mut Self::Interface) -> Self {
                        Self { interface }
                    }
                }
            };
            q
        })
        .collect::<Vec<_>>();

    let res: TokenStream = quote! {
        #input_mod_vis mod #input_mod_name {
            use crate::api::sys::*;

            #(#interface_structs)*
        }
    }
    .into();

    // println!("{}", res);

    res
}

#[derive(Debug, FromMeta)]
struct SimicsHapCodegen {
    source: String,
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Automatically generate high level bindings to all HAPs provided by SIMICS
pub fn simics_hap_codegen(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let codegen_args = match SimicsHapCodegen::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let bindings_source_path = if let Ok(out_dir) = var("OUT_DIR") {
        PathBuf::from(out_dir).join(codegen_args.source)
    } else {
        return TokenStream::from(
            Error::custom("No environment variable OUT_DIR set").write_errors(),
        );
    };

    let bindings_source = if let Ok(bindings_source) = read(&bindings_source_path) {
        if let Ok(bindings_source) = String::from_utf8(bindings_source) {
            bindings_source
        } else {
            return TokenStream::from(
                Error::custom("Bindings source file was not UTF8").write_errors(),
            );
        }
    } else {
        return TokenStream::from(
            Error::custom("Failed to read bindings source file").write_errors(),
        );
    };

    let input = parse_macro_input!(input as ItemMod);
    let input_mod_vis = &input.vis;
    let input_mod_name = &input.ident;

    let parsed_bindings = match parse_file(&bindings_source) {
        Ok(b) => b,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let hap_name_items = parsed_bindings
        .items
        .iter()
        .filter_map(|i| {
            if let Item::Const(c) = i {
                if c.ident.to_string().ends_with("_HAP_NAME") {
                    Some((c.ident.to_string(), c))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

    // println!("{:?}", hap_name_items);

    let haps = parsed_bindings
        .items
        .iter()
        .filter_map(|i| {
            if let Item::Type(ty) = i {
                if ty.ident.to_string().ends_with("_hap_callback") {
                    hap_name_items
                        .get(
                            &(ty.ident
                                .to_string()
                                .trim_end_matches("_hap_callback")
                                .to_ascii_uppercase()
                                + "_HAP_NAME"),
                        )
                        .map(|hap_name_item| (hap_name_item, ty))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

    // println!("{:?}", haps);

    let hap_structs = haps
        .iter()
        .filter_map(hap_name_and_type_to_struct)
        .collect::<Vec<_>>();

    quote! {
        #input_mod_vis mod #input_mod_name {
            use crate::api::sys::*;

            #(#hap_structs)*
        }
    }
    .into()
}

fn hap_name_and_type_to_struct(
    name_callback_type: (&&&ItemConst, &&ItemType),
) -> Option<TokenStream2> {
    let name = name_callback_type.0;
    let name_name = &name.ident;
    let callback_type = name_callback_type.1;
    let struct_name = format_ident!(
        "{}Hap",
        callback_type
            .ident
            .to_string()
            .trim_end_matches("_hap_callback")
            .to_string()
            .snake_to_camel()
    );
    let handler_name = format_ident!(
        "{}",
        "handle_".to_string()
            + callback_type
                .ident
                .to_string()
                .trim_end_matches("_hap_callback"),
    );
    if let Type::Path(ref p) = &*callback_type.ty {
        if let Some(last) = p.path.segments.last() {
            if last.ident == "Option" {
                if let PathArguments::AngleBracketed(ref args) = last.arguments {
                    if let Some(GenericArgument::Type(Type::BareFn(proto))) = args.args.first() {
                        // NOTE: We `use crate::api::sys::*;` at the top of the module, otherwise
                        // we would need to rewrite all of the types on `inputs` here.
                        let inputs = &proto.inputs;
                        let input_names = inputs
                            .iter()
                            .filter_map(|a| a.name.clone().map(|n| n.0))
                            .collect::<Vec<_>>();
                        if let Some(userdata_name) = input_names.first() {
                            let output = match &proto.output {
                                ReturnType::Default => quote!(()),
                                ReturnType::Type(_, t) => quote!(#t),
                            };
                            let closure_params =
                                inputs.iter().skip(1).map(|a| &a.ty).collect::<Vec<_>>();
                            let closure_param_names =
                                input_names.iter().skip(1).collect::<Vec<_>>();

                            let struct_and_impl = quote! {
                                pub struct #struct_name {}

                                impl<C> crate::api::traits::hap::Hap<C> for #struct_name
                                where
                                    C: Fn(#(#closure_params),*) -> #output + 'static
                                {
                                    type Callback = #proto;
                                    type Name =  &'static [u8];
                                    const NAME: Self::Name = crate::api::sys::#name_name;
                                    const HANDLER: Self::Callback = #handler_name::<C>;
                                }

                                extern "C" fn #handler_name<F>(#inputs) -> #output
                                    where F: Fn(#(#closure_params),*) -> #output + 'static
                                {
                                    let closure: Box<Box<F>> = unsafe { Box::from_raw(#userdata_name as *mut Box<F>) };
                                    closure(#(#closure_param_names),*)
                                }

                            };

                            println!("{}", struct_and_impl);

                            return Some(struct_and_impl);
                        }
                    }
                }
            }
        }
    }
    None
}
