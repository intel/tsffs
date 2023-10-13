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
use syn::{
    parse::Parser, parse_macro_input, parse_str, Expr, Field, Fields, Generics, Ident,
    ImplGenerics, ItemFn, ItemStruct, ReturnType, Type, TypeGenerics, Visibility, WhereClause,
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
            let result = #inner_ident(#(#args),*);
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

    println!("{:?}", wrapper);

    quote! {
        #input
        #wrapper
    }
    .into()
}
