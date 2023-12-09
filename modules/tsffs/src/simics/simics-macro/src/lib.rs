// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Derive/attribute macros for simics-api

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use std::{
    env::var,
    fs::{create_dir_all, read_dir, write},
    path::PathBuf,
};

use darling::{
    ast::{Data, NestedMeta},
    util::Flag,
    Error, FromDeriveInput, FromField, FromMeta, Result,
};
use indoc::formatdoc;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, parse_str, DeriveInput, Expr, Field, Fields, FnArg,
    GenericArgument, Generics, Ident, ImplGenerics, ImplItem, ItemFn, ItemImpl, ItemStruct, Lit,
    Meta, Pat, PathArguments, ReturnType, Signature, Type, TypeGenerics, Visibility, WhereClause,
};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(module), supports(struct_named))]
struct ClassDerive {
    ident: Ident,
    generics: Generics,
}

impl ToTokens for ClassDerive {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            impl #impl_generics simics::api::traits::class::Class for #ident #ty_generics #where_clause {}
        })
    }
}

#[proc_macro_derive(Class)]
/// Derive macro for the [`Class`] trait.
pub fn class_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match ClassDerive::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    quote! {
        #args
    }
    .into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(module), supports(struct_named))]
struct AsConfObjectDerive {
    ident: Ident,
    generics: Generics,
}

impl ToTokens for AsConfObjectDerive {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            impl #impl_generics simics::api::traits::class::AsConfObject for #ident #ty_generics #where_clause {}
        })
    }
}

#[proc_macro_derive(AsConfObject)]
/// Derive macro for the [`Class`] trait.
pub fn as_conf_object_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match AsConfObjectDerive::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    quote! {
        #args
    }
    .into()
}

#[derive(Debug, FromMeta)]
struct ClassOpts {
    name: Expr,
    derive: Flag,
    description: Option<String>,
    short_description: Option<String>,
    kind: Option<Type>,
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Attribute to add boilerplate to a `struct` to enable it to be used as a SIMICS Conf Object.
///
/// * Generate default implementations for CFFI to call functions defined in the [`Class`] trait
///   impl
/// * Insert a [ConfObject] field to permit instances of the struct to be passed via CFFI to and
///   from SIMICS
/// * Optionally, derive the default implementations of the [`Class`] trait
///
/// The macro accepts the following arguments:
///
/// * `name = "name"` (Required) specifies the generated class name the class will be registered with
/// * `derive` (Optional) which allows you to derive the default
///   implementation of [`Class`] alongside automatic implementations of the extern functions
///   required to register the class.
/// * `description = "describe your class"` (Optional) set a custom description for the generated
///   class. Defaults to the struct name.
/// * `short_description = "short desc"` (Optional) set a custom short description for the
///   generated class. Defaults to the struct name.
/// * `kind = ClassKind::Vanilla` (Optional) set a class kind. Most classes are Vanilla,
///   which is the default, but the kind can be set here.
///
/// # Examples
///
/// Without deriving [`Class`]:
///
/// ```rust,ignore
/// use simics::api::{Class, CreateClass};
/// use simics_macro::class;
///
/// #[class(name = "test")]
/// struct Test {}
/// ```
///
/// Derive [`Class`]:
///
/// ```rust,ignore
/// use simics::api::{Class, CreateClass};
/// use simics_macro::class;
///
/// #[class(derive, name = "test")]
/// struct Test {}
/// ```
/// Derive [`Class`] and customize the generated class description and kind:
///
/// ```rust,ignore
/// use simics::api::{Class, CreateClass};
/// use simics_macro::class;
///
/// #[module(
///    derive,
///    name = "test_module_4",
///    description = "Test module 4",
///    short_description = "TM4",
///    kind = ClassKind::Session
/// )]
/// struct Test {}
/// ```
///
pub fn class(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let mut input = parse_macro_input!(input as ItemStruct);

    let args = match ClassOpts::from_list(&attr_args) {
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
        .then_some(quote!(#[derive(Class, AsConfObject)]));

    let ffi_impl = ffi_impl(name.to_string());
    let register_impl = create_impl(
        name.to_string(),
        &args,
        &impl_generics,
        &ty_generics,
        &where_clause,
    );
    let from_impl = from_impl(&input);

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
            let ptr: *mut simics::api::ConfObject = #name::init(obj.into())
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
    args: &ClassOpts,
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
    let class_name = &args.name;

    let description = args.description.as_ref().unwrap_or(&name_string);

    let short_description = args.short_description.as_ref().unwrap_or(&name_string);

    let kind = args
        .kind
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

        impl #impl_generics simics::api::CreateClass for #name #ty_generics #where_clause {
            fn create() -> simics::Result<*mut simics::api::ConfClass> {
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
                value: #name #ty_generics
            ) -> *mut simics::api::ConfObject {
                let ptr: *mut #name #ty_generics = obj as *mut #name #ty_generics;
                unsafe { std::ptr::addr_of_mut!(*ptr).write(value) };
                ptr as *mut simics::api::ConfObject
            }

        }
    }
}

fn from_impl(input: &ItemStruct) -> TokenStream2 {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics From<*mut simics::api::ConfObject> for &'static mut #name #ty_generics
            #where_clause
        {
            fn from(value: *mut simics::api::ConfObject) -> Self {
                let ptr: *mut #name #ty_generics = value as *mut #name #ty_generics ;
                unsafe { &mut *ptr }
            }
        }

        impl #impl_generics From<*mut simics::api::ConfObject> for &'static #name #ty_generics
            #where_clause
        {
            fn from(value: *mut simics::api::ConfObject) -> Self {
                let ptr: *const #name #ty_generics = value as *const #name #ty_generics ;
                unsafe { &*ptr }
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
/// Marks a function as being a SIMICS API that can throw exceptions. A SIMICS exception
/// can be generated by most APIs. This macro makes the function private, wraps it, and
/// adds the requisite code to check for and report exceptions. `clear_exception` should
/// *not* be called inside the wrapped function. `last_error` may be called, however, as
/// any exceptions will be cleared after the wrapped function returns.
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
    let attrs = &input.attrs;
    let doc_attrs = attrs
        .iter()
        .filter(|a| {
            if let Meta::NameValue(attr) = &a.meta {
                if let Some(first) = attr.path.segments.first() {
                    first.ident == "doc"
                } else {
                    false
                }
            } else {
                false
            }
        })
        .collect::<Vec<_>>();

    let inner_ident = format_ident!("_{}", input.sig.ident);
    input.sig.ident = inner_ident.clone();
    let input_name = input.sig.ident.to_string();
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
        #(#doc_attrs)*
        #vis #sig {
            let result = #inner_ident #maybe_ty_generics(#(#args),*);
            match crate::api::get_pending_exception() {
                crate::api::base::sim_exception::SimException::SimExc_No_Exception => #ok_return,
                exception => {
                    crate::api::base::sim_exception::clear_exception();
                    Err(crate::error::Error::SimicsException {
                        exception,
                        msg: crate::api::base::sim_exception::last_error() + "(" + #input_name + ")"
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

#[derive(Debug, FromMeta)]
struct InterfaceOpts {}

#[proc_macro_error]
#[proc_macro_attribute]
/// Declare that a struct has an interface which can be registered for use with the SIMICS API.
///
/// This macro generates an implementation of [`Interface`] and [`HasInterface`] for the
/// struct, as well as a new struct called #original_nameInterface, which wraps the
/// low-level pointer to CFFI compatible struct. The interface will be named the same as
/// the class, converted to ascii lowercase characters. For example a struct named
/// `Tsffs` will have a generated interface name `tsffs`.
///
/// One implementation of the struct should be annotated with `#[interface_impl]` to
/// generate CFFI compatible functions that can be called through the SIMICS interface for that
/// implementation's methods.
pub fn interface(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let input = parse_macro_input!(input as ItemStruct);

    let _args = match InterfaceOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let vis = &input.vis;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();
    let ident = &input.ident;
    let interface_ident = format_ident!("{}Interface", ident);
    let interface_internal_ident = format_ident!("{}InterfaceInternal", ident);
    let interface_name_literal: Lit = match parse_str(&format!(
        r#"b"{}\0""#,
        input.ident.to_string().to_ascii_lowercase()
    )) {
        Ok(l) => l,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    quote! {
        #input

        #vis struct #interface_ident {
            obj: *mut simics::api::ConfObject,
            interface: *mut #interface_internal_ident,
        }

        impl #impl_generics simics::api::traits::interface::HasInterface for #ident #ty_generics
        #where_clause
        {
            type Interface = #interface_ident;
        }

        impl simics::api::traits::interface::Interface for #interface_ident {
            type InternalInterface = #interface_internal_ident;
            type Name = &'static [u8];

            const NAME: &'static [u8] = #interface_name_literal;

            fn new(obj: *mut simics::api::ConfObject, interface: *mut Self::InternalInterface) -> Self {
                Self { obj, interface }
            }

            fn register(cls: *mut simics::api::ConfClass) -> simics::Result<()> {
                simics::api::base::conf_object::register_interface::<Self>(cls)?;
                Ok(())
            }

            fn get(obj: *mut simics::api::ConfObject) -> simics::Result<Self> where Self: Sized {
                simics::api::base::conf_object::get_interface::<Self>(obj)
            }
        }
    }
    .into()
}

fn type_name(ty: &Type) -> Result<Ident> {
    if let Type::Path(ref p) = ty {
        if let Some(segment) = p.path.segments.last() {
            return Ok(segment.ident.clone());
        }
    }

    Err(Error::custom("Incorrect type to get ident"))
}

fn interface_function_type_to_ctype(ty: &Type) -> String {
    match &ty {
        Type::Paren(i) => interface_function_type_to_ctype(&i.elem),
        Type::Tuple(t) => {
            if t.elems.is_empty() {
                "void".to_string()
            } else {
                panic!("Non-empty tuple is not a valid C type");
            }
        }
        Type::Path(p) => {
            // First, check if the outer is an option. If it is, we just discard it and take the
            // inner type.
            if let Some(last) = p.path.segments.last() {
                let ty_ident = &last.ident;
                match &last.arguments {
                    syn::PathArguments::None => {
                        // No angle arguments, we can break down the type now
                        let tystr = ty_ident.to_string();
                        match tystr.as_str() {
                            "ConfObject" => "conf_object_t",
                            "AttrValue" => "attr_value_t",
                            "BreakpointId" => "breakpoint_id_t",
                            "GenericAddress" => "generic_address_t",
                            "u8" => "uint8",
                            "u16" => "uint16",
                            "u32" => "uint32",
                            "u64" => "uint64",
                            "i8" => "int8",
                            "i16" => "int16",
                            "i32" => "int32",
                            "i64" => "int64",
                            // NOTE: This is not exactly right, but we don't expect anyone to
                            // run simics on a 32-bit host.
                            "f32" => "float",
                            "f64" => "double",
                            "usize" => "size_t",
                            "isize" => "ssize_t",
                            "c_char" => "char",
                            // Attempt to use the type as-is. This is unlikely to work, but allows
                            // creative people to be creative
                            other => other,
                        }
                        .to_string()
                    }
                    syn::PathArguments::AngleBracketed(a) => {
                        // Options and results can be extracted directly
                        if last.ident == "Option" || last.ident == "Result" {
                            if let Some(GenericArgument::Type(ty)) = a.args.first() {
                                interface_function_type_to_ctype(ty)
                            } else {
                                panic!("Unsupported generic arguments");
                            }
                        } else {
                            panic!("Unsupported function type with arguments: {ty_ident}");
                        }
                    }
                    _ => panic!("Unsupported interface function type argument"),
                }
            } else {
                panic!("Unexpected empty path in interface function type");
            }
        }
        Type::Ptr(p) => {
            let ptr_ty = interface_function_type_to_ctype(&p.elem);
            let maybe_const = p
                .const_token
                .is_some()
                .then_some("const ".to_string())
                .unwrap_or_default();
            format!("{maybe_const}{ptr_ty} *")
        }
        _ => panic!("Unsupported type for C interface generation: {ty:?}"),
    }
}

fn generate_interface_function_type(signature: &Signature) -> String {
    let name = &signature.ident;
    let ty = signature
        .inputs
        .iter()
        .map(|i| match i {
            FnArg::Receiver(_) => "conf_object_t * obj".to_string(),
            FnArg::Typed(a) => {
                let ty = interface_function_type_to_ctype(&a.ty);
                let name = match &*a.pat {
                    Pat::Ident(ref p) => p.ident.to_string(),
                    _ => panic!("Expected ident pattern type"),
                };
                format!("{} {}", ty, name)
            }
        })
        .collect::<Vec<_>>();
    let ty_params = ty.join(", ");

    let output = match &signature.output {
        ReturnType::Default => "void".to_string(),
        ReturnType::Type(_, t) => interface_function_type_to_ctype(t),
    };

    format!("{output} (*{name})({ty_params});")
}

fn generate_interface_header(input: &ItemImpl) -> String {
    let input_name = type_name(&input.self_ty).expect("Invalid type name");
    let interface_name = input_name.to_string().to_ascii_lowercase();
    let interface_struct_name = format!("{interface_name}_interface");
    let interface_struct_name_define = interface_struct_name.to_ascii_uppercase();
    let include_guard = format!("{}_INTERFACE_H", interface_name.to_ascii_uppercase());
    let interface_functions = input
        .items
        .iter()
        .filter_map(|i| {
            if let ImplItem::Fn(ref f) = i {
                Some(&f.sig)
            } else {
                None
            }
        })
        .map(generate_interface_function_type)
        .collect::<Vec<_>>();
    let interface_functions_code = interface_functions.join("\n    ");

    formatdoc! {r#"
        // Copyright (C) 2023 Intel Corporation
        // SPDX-License-Identifier: Apache-2.0

        #ifndef {include_guard}
        #define {include_guard}

        #include <simics/device-api.h>
        #include <simics/pywrap.h>

        #ifdef __cplusplus
        extern "C" {{
        #endif

        SIM_INTERFACE({interface_name}) {{
            {interface_functions_code}
        }};

        #define {interface_struct_name_define} "{interface_name}"

        #ifdef __cplusplus
        }}
        #endif

        #endif // {include_guard}

    "#}
}

fn generate_interface_dml<S>(input: &ItemImpl, header_name: S) -> String
where
    S: AsRef<str>,
{
    let header_name = header_name.as_ref();
    let input_name = type_name(&input.self_ty).expect("Invalid type name");
    let interface_name = input_name.to_string().to_ascii_lowercase();
    let interface_struct_name = format!("{interface_name}_interface");
    let interface_struct_name_define = interface_struct_name.to_ascii_uppercase();
    let interface_struct_ty_name = format!("{interface_struct_name}_t");
    let interface_functions = input
        .items
        .iter()
        .filter_map(|i| {
            if let ImplItem::Fn(ref f) = i {
                Some(&f.sig)
            } else {
                None
            }
        })
        .map(generate_interface_function_type)
        .collect::<Vec<_>>();
    let interface_functions_code = interface_functions.join("\n    ");

    formatdoc! {r#"
        // Copyright (C) 2023 Intel Corporation
        // SPDX-License-Identifier: Apache-2.0

        dml 1.4;

        header %{{
        #include "{header_name}"
        %}}

        extern typedef struct {{
            {interface_functions_code}
        }} {interface_struct_ty_name};

        extern const char *const {interface_struct_name_define};
    "#}
}

fn generate_interface_makefile<S>(header_name: S) -> String
where
    S: AsRef<str>,
{
    let header_name = header_name.as_ref();

    formatdoc! {r#"
        IFACE_FILES = {header_name}
        THREAD_SAFE = yes

        ifeq ($(MODULE_MAKEFILE),)
        $(error Make sure you compile your module from the project directory)
        else
        include $(MODULE_MAKEFILE)
        endif
    "#}
}

#[derive(Debug, FromMeta)]
struct InterfaceImplOpts {
    #[darling(rename = "modules_path")]
    generate_path: Option<String>,
}

#[proc_macro_error]
#[proc_macro_attribute]
/// An implementation for an interface on a module. This attribute should be added to an
/// implementation of a struct annotated with `#[interface()]`. It generates an
/// FFI-compatible structure containing CFFI compatible function pointers to call
/// through to this struct's methods.
///
/// # Arguments
///
/// - `modules_path`: If set, generate a Makefile, .h, and .dml file for the interface. This
///   path should be to the project's `modules` directory. It can be relative to the crate
///   containing the code using this attribute. For example, if the `tsffs` crate is located in
///   $SIMICS_PROJECT/src/tsffs/, `modules_path = "../../modules/"` would be specified. This is
///   similar to the syntax used by `include!` and `include_str!`.
///
/// # Notes on C/H generation
///
/// In the normal build toolchain, an interface called `tsffs` is declared with:
///
/// ```c
/// #ifndef TSFFS_INTERFACE_H
/// #define TSFFS_INTERFACE_H
/// #include <simics/device-api.h>
/// #include <simics/pywrap.h>
///
/// #ifdef __cplusplus
/// extern "C" {
/// #endif
///
/// SIM_INTERFACE(tsffs) {
///   // Python-exportable interface methods...
/// #ifndef PYWRAP
///   // Non python-exportable methods
/// #endif
/// };
/// #define TSFFS_INTERFACE "tsffs"
///
/// #ifdef __cplusplus
/// }
/// #endif
/// #endif TSFFS_INTERFACE_H
/// ```
/// Where `SIM_INTERFACE(tsffs)` expands to:
///
/// ```c
/// typedef struct tsffs_interface tsffs_interface_t; struct tsffs_interface
/// ```
///
/// During code-generation, we do not include `pywrap.h`, and all methods must be exportable to
/// Python. We also do not use the `SIM_INTERFACE` macro.
pub fn interface_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let input = parse_macro_input!(input as ItemImpl);

    let args = match InterfaceImplOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input_name = match type_name(&input.self_ty) {
        Ok(n) => n,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let ffi_interface_mod_name = format!(
        "{}_interface_ffi",
        input_name.to_string().to_ascii_lowercase()
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let impl_fns = input
        .items
        .iter()
        .filter_map(|i| {
            if let ImplItem::Fn(ref f) = i {
                Some(quote! {
                    #[ffi(arg(self), arg(rest))]
                    #f
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let internal_interface_name = format_ident!("{}InterfaceInternal", input_name);
    let internal_interface_fields = input
        .items
        .iter()
        .filter_map(|i| {
            if let ImplItem::Fn(ref f) = i {
                Some(&f.sig)
            } else {
                None
            }
        })
        .map(|s| {
            let name = &s.ident;
            let mut inputs = s
                .inputs
                .iter()
                .skip(1)
                .map(|i| quote!(#i))
                .collect::<Vec<_>>();
            inputs.insert(0, quote!(obj: *mut simics::api::ConfObject));
            let output = match &s.output {
                ReturnType::Default => quote!(()),
                ReturnType::Type(_, t) => {
                    if s.output.is_result_type() {
                        let Type::Path(ref path) = &**t else {
                            panic!("Type is result but is not a path");
                        };
                        let Some(last) = path.path.segments.last() else {
                            panic!("Path has no last segment");
                        };
                        let PathArguments::AngleBracketed(args) = &last.arguments else {
                            panic!("Path does not have angle bracketed arguments");
                        };
                        let Some(first) = args.args.first() else {
                            panic!("Path does not have a first angle bracketed argument");
                        };
                        quote!(#first)
                    } else {
                        quote!(#t)
                    }
                }
            };
            quote!(pub #name: Option<extern "C" fn(#(#inputs),*) -> #output>)
        })
        .collect::<Vec<_>>();
    let internal_interface_default_args = input
        .items
        .iter()
        .filter_map(|i| {
            if let ImplItem::Fn(ref f) = i {
                Some(&f.sig)
            } else {
                None
            }
        })
        .map(|s| {
            let name = &s.ident;
            let ffi_interface_mod_name = format_ident!("{ffi_interface_mod_name}");
            quote!(#name: Some(#ffi_interface_mod_name::#name))
        })
        .collect::<Vec<_>>();

    let crate_directory_path = PathBuf::from(
        var("CARGO_MANIFEST_DIR").expect("No CARGO_MANIFEST_DIR set. This should be impossible."),
    );

    if let Some(generate_path) = args.generate_path {
        let generate_path = if generate_path.starts_with('/') {
            PathBuf::from(generate_path)
        } else {
            crate_directory_path.join(generate_path).join(format!(
                "{}-interface",
                input_name.to_string().to_ascii_lowercase()
            ))
        };

        if !generate_path.is_dir() {
            if let Err(e) = create_dir_all(&generate_path) {
                return TokenStream::from(
                    Error::custom(format!(
                        "Failed to create generated interface directory: {e}"
                    ))
                    .write_errors(),
                );
            }
        }

        let header_name = format!(
            "{}-interface.h",
            input_name.to_string().to_ascii_lowercase()
        );
        let dml_name = format!(
            "{}-interface.dml",
            input_name.to_string().to_ascii_lowercase()
        );
        let makefile_name = "Makefile";

        let header = generate_interface_header(&input);
        let dml = generate_interface_dml(&input, &header_name);
        let makefile = generate_interface_makefile(&header_name);

        write(generate_path.join(header_name), header).expect("Failed to write header file");
        write(generate_path.join(dml_name), dml).expect("Failed to write dml file");
        write(generate_path.join(makefile_name), makefile).expect("Failed to write makefile");
    }

    let q: TokenStream = quote! {
        #[ffi(expect, mod_name = #ffi_interface_mod_name, self_ty = "*mut simics::api::ConfObject")]
        impl #impl_generics #input_name #ty_generics #where_clause {
            #(#impl_fns)*
        }

        #[derive(Debug)]
        #[repr(C)]
        pub struct #internal_interface_name {
            #(#internal_interface_fields),*
        }

        impl Default for #internal_interface_name {
            fn default() -> Self {
                Self {
                    #(#internal_interface_default_args),*
                }
            }
        }
    }
    .into();

    // println!("{q}");

    q
}

#[derive(Debug, FromMeta)]
struct SimicsTestsOpts {
    package_root: String,
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Generate separate test functions for all SIMICS test python scripts (scripts beginning with
/// 's-' inside a directory that contains a SUITEINFO file.
///
/// The path `package_root` should be a relative path from the test crate root to the package
/// root directory. Tests whose filenames end in "-fail" will expect a failure, not a success.
pub fn simics_tests(args: TokenStream, _input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let args = match SimicsTestsOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let crate_directory_path = PathBuf::from(
        var("CARGO_MANIFEST_DIR").expect("No CARGO_MANIFEST_DIR set. This should be impossible."),
    );

    let test_runner_path = crate_directory_path
        .join(args.package_root)
        .join("bin/test-runner");

    if !test_runner_path.is_file() {
        panic!(
            "Test runner path {} does not exist.",
            test_runner_path.display()
        );
    }

    let test_runner_path = test_runner_path.to_str().unwrap_or_else(|| {
        panic!(
            "Could not get string for test runner path {}",
            test_runner_path.display()
        )
    });

    let integration_tests_directory = crate_directory_path;

    let tests = read_dir(integration_tests_directory)
        .expect("Failed to read integration tests directory")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            p.file_stem().and_then(move |f| {
                f.to_str()
                    .map(|f| f.to_string())
                    .filter(|f| f.starts_with("s-"))
            })
        })
        .map(|test_name| {
            let test_name_ident = format_ident!(
                "test_simics_{}",
                test_name
                    .to_ascii_lowercase()
                    .replace("s-", "")
                    .replace('-', "_")
            );
            let try_or_invert_try = if test_name.ends_with("-fail") {
                quote!(.err().ok_or_else(|| anyhow::anyhow!("Expected failure, got success"))?)
            } else {
                quote!(?)
            };
            quote! {
                #[test]
                fn #test_name_ident() -> anyhow::Result<()> {
                    std::process::Command::new(#test_runner_path)
                        .arg("-v")
                        .arg("-n")
                        .arg(#test_name)
                        .check()#try_or_invert_try;

                    Ok(())
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        use command_ext::CommandExtCheck;

        #(#tests)*
    }
    .into()
}

#[derive(Debug, FromField)]
#[darling(attributes(try_into_attr_value_type))]
struct TryIntoAttrValueTypeField {
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
    skip: Flag,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(try_into_attr_value_type), supports(struct_named))]
struct TryIntoAttrValueTypeDictOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), TryIntoAttrValueTypeField>,
}

impl TryIntoAttrValueTypeDictOpts {
    fn to_tokens_dict(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let Some(fields) = self.data.as_ref().take_struct() else {
            panic!("Fields must be struct fields");
        };
        let dict_fields = fields
            .iter()
            .filter(|f| !f.skip.is_present())
            .filter_map(|f| {
                f.ident.clone().map(|i| {
                    let ident_name = i.to_string();
                    let accessor = format_ident!("{}_ref", i);
                    quote!((#ident_name.try_into()?, value.#accessor().clone().try_into()?))
                })
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #impl_generics TryFrom<#ident #ty_generics> for simics::api::AttrValueType #where_clause {
                type Error = simics::Error;
                fn try_from(value: #ident #ty_generics) -> simics::Result<Self> {
                    Ok(Self::Dict(
                        std::collections::BTreeMap::from_iter([
                            #(#dict_fields),*
                        ])
                    ))
                }
            }

            impl #impl_generics TryFrom<&#ident #ty_generics> for simics::api::AttrValueType #where_clause {
                type Error = simics::Error;
                fn try_from(value: &#ident #ty_generics) -> simics::Result<Self> {
                    value.clone().try_into()
                }
            }
        });
    }
}

impl ToTokens for TryIntoAttrValueTypeDictOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.to_tokens_dict(tokens)
    }
}

#[proc_macro_derive(TryIntoAttrValueTypeDict)]
/// Derive macro for the [`Class`] trait.
pub fn try_into_attr_value_type_dict(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match TryIntoAttrValueTypeDictOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    quote! {
        #args
    }
    .into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(try_into_attr_value_type), supports(struct_named))]
struct TryIntoAttrValueTypeListOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), TryIntoAttrValueTypeField>,
}

impl TryIntoAttrValueTypeListOpts {
    fn to_tokens_list(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let Some(fields) = self.data.as_ref().take_struct() else {
            panic!("Fields must be struct fields");
        };
        let dict_fields = fields
            .iter()
            .filter(|f| !f.skip.is_present())
            .filter_map(|f| {
                f.ident.clone().map(|i| {
                    let accessor = format_ident!("{}_ref", i);
                    quote!(value.#accessor().clone().try_into()?)
                })
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #impl_generics TryFrom<#ident #ty_generics> for simics::api::AttrValueType #where_clause {
                type Error = simics::Error;
                fn try_from(value: #ident #ty_generics) -> simics::Result<Self> {
                    Ok(Self::List(
                        [
                            #(#dict_fields),*
                        ]
                        .iter()
                        .collect::<Vec<_>>()
                    ))
                }
            }

            impl #impl_generics TryFrom<&#ident #ty_generics> for simics::api::AttrValueType #where_clause {
                type Error = simics::Error;
                fn try_from(value: &#ident #ty_generics) -> simics::Result<Self> {
                    value.clone().try_into()
                }
            }
        });
    }
}

impl ToTokens for TryIntoAttrValueTypeListOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.to_tokens_list(tokens)
    }
}

#[proc_macro_derive(TryIntoAttrValueTypeList)]
/// Derive macro for the [`Class`] trait.
pub fn try_into_attr_value_type_list(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match TryIntoAttrValueTypeListOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    quote! {
        #args
    }
    .into()
}

#[derive(Debug, FromField)]
#[darling(attributes(try_from_attr_value_type))]
struct TryFromAttrValueTypeField {
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(try_from_attr_value_type), supports(struct_named))]
struct TryFromAttrValueTypeListOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), TryFromAttrValueTypeField>,
}

impl TryFromAttrValueTypeListOpts {
    fn to_tokens_list(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let Some(fields) = self.data.as_ref().take_struct() else {
            panic!("Fields must be struct fields");
        };

        let dict_fields = fields
            .iter()
            .enumerate()
            .filter_map(|(index, f)| {
                f.ident.clone().map(|ident| {
                    let ident_name = ident.to_string();
                    quote! {
                        #ident: value.get(#index)
                                .ok_or_else(|| simics::Error::AttrValueDictMissingKey { key: #ident_name.to_string()})?
                                .clone()
                                .try_into()?
                    }
                })
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #impl_generics TryFrom<simics::api::AttrValueType> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::api::AttrValueType) -> simics::Result<Self> {
                    let simics::api::AttrValueType::List(value) = value else {
                        return Err(simics::Error::FromAttrValueTypeConversionError {
                            ty: std::any::type_name::<#ident #ty_generics>().to_string(),
                        });
                    };

                    Ok(Self {
                        #(#dict_fields),*
                    })
                }
            }

            impl #impl_generics TryFrom<simics::api::AttrValue> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::api::AttrValue) -> simics::Result<Self> {
                    // NOTE: We cannot use AttrValueType here, because we are most likely
                    // converting from a non-homogeneous list.
                    let value: Vec<simics::api::AttrValueType> = value.as_heterogeneous_list()?.ok_or_else(|| simics::Error::AttrValueType {
                        actual: value.kind(),
                        expected: simics::api::AttrKind::Sim_Val_List,
                    })?;

                    Ok(Self {
                        #(#dict_fields),*
                    })
                }
            }
        });
    }
}

impl ToTokens for TryFromAttrValueTypeListOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.to_tokens_list(tokens)
    }
}

#[proc_macro_derive(TryFromAttrValueTypeList)]
/// Derive macro that allows deserialization from an attrvalue
pub fn try_from_attr_value_type_list(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match TryFromAttrValueTypeListOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    let q: TokenStream = quote! {
        #args
    }
    .into();

    // println!("{}", q);

    q
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(try_from_attr_value_type), supports(struct_named))]
struct TryFromAttrValueTypeDictOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), TryFromAttrValueTypeField>,
}

impl TryFromAttrValueTypeDictOpts {
    fn to_tokens_dict(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let Some(fields) = self.data.as_ref().take_struct() else {
            panic!("Fields must be struct fields");
        };

        let dict_fields = fields
            .iter()
            .filter_map(|f| {
                f.ident.clone().map(|i| {
                    let ident_name = i.to_string();
                    quote! {
                        #i: value.get(&#ident_name.into())
                                .ok_or_else(|| simics::Error::AttrValueDictMissingKey { key: #ident_name.to_string()})?
                                .clone()
                                .try_into()?

                    }
                })
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #impl_generics TryFrom<simics::api::AttrValueType> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::api::AttrValueType) -> simics::Result<Self> {
                    let simics::api::AttrValueType::Dict(value) = value else {
                        return Err(simics::Error::FromAttrValueTypeConversionError {
                            ty: std::any::type_name::<#ident #ty_generics>().to_string(),
                        });
                    };
                    Ok(Self {
                        #(#dict_fields),*
                    })
                }
            }

            impl #impl_generics TryFrom<simics::api::AttrValue> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::api::AttrValue) -> simics::Result<Self> {
                    let value = value.as_heterogeneous_dict()?.ok_or_else(|| simics::Error::AttrValueType {
                        actual: value.kind(),
                        expected: simics::api::AttrKind::Sim_Val_Dict,
                    })?;

                    Ok(Self {
                        #(#dict_fields),*
                    })
                }
            }
        });
    }
}

impl ToTokens for TryFromAttrValueTypeDictOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.to_tokens_dict(tokens)
    }
}

#[proc_macro_derive(TryFromAttrValueTypeDict)]
/// Derive macro that allows deserialization from an attrvalue
pub fn try_from_attr_value_type_dict(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match TryFromAttrValueTypeDictOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };
    let q: TokenStream = quote! {
        #args
    }
    .into();

    // println!("{}", q);

    q
}
