// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! The `ffi` macro automatically generates a module containing FFI-compatible functions that
//! wrap a struct or enum's implementation methods, to facilitate their use in callbacks.
//!
//! For example, suppose a C functon that schedules a callback to run at a later time which will
//! compute the sum of two numbers and return the result. As is typical, this callback also takes
//! a user-data `void *` parameter, which will be passed as-is to the callback.
//!
//! ```c,ignore
//! int schedule_add_callback(uint64_t (*cb)(uint64_t, uint64_t, void *), void *userdata);
//! ```
//!
//! Doing this in Rust requires a large amount of boilerplate. In particular, you must provide a
//! C ABI compatible function as the callback parameter by declaring a separate function like:
//!
//! ```rust,ignore
//! #[no_mangle]
//! pub extern "C" fn cb(a: u64, b: u64, _: *mut std::ffi::c_void) -> u64 { a + b }
//! ```
//!
//! Complicating matters, suppose the callback is not invoked in a stateless fashion,
//! and your program needs to store information or produce some side effect when this
//! callback is triggered. In this case, you likely have a struct instance containing some state,
//! along with implementations roughly corresponding to your desired callback behavior. This macro
//! automates the process of wrapping those implementation methods in FFI-compatible extern
//! functions that can be used as FFI callbacks, or automating parts of creating a C API to your
//! library.
//!
//! In the above example, suppose we have an accumulator which stores the total sum of all
//! nubers it has been asked to add, as well as returning the result of various mathematical
//! operations. Using `ffi_macro`, this looks like:
//!
//!
//! ```rust,ignore
//! use std::{ffi::c_void, ptr::addr_of_mut};
//!
//! use anyhow::Result;
//! use ffi_macro::ffi;
//!
//! #[derive(Debug, Default)]
//! pub struct Accumulator {
//!     total: u64,
//! }
//!
//! impl From<*mut std::ffi::c_void> for &mut Accumulator {
//!     fn from(value: *mut std::ffi::c_void) -> Self {
//!         unsafe { *(value as *mut Self) }
//!     }
//! }
//!
//! #[ffi(mod_name = "ffi", expect, self_ty = "*mut std::ffi::c_void")]
//! impl Accumulator {
//!     #[ffi(arg(rest), arg(self))]
//!     pub fn add(&mut self, a: u64, b: u64) -> Result<u64> {
//!         self.total += a;
//!         self.total += b;
//!         Ok(a + b)
//!     }
//! }
//!
//! fn main() {
//!     let mut a = Accumulator::default();
//!     let res = ffi::add(1, 2, addr_of_mut!(a) as *mut c_void);
//!     assert_eq!(res, 3);
//!     assert_eq!(a.total, 3);
//! }
//! ```

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

use darling::{
    ast::NestedMeta,
    util::{Flag, WithOriginal},
    Error, FromAttributes, FromMeta, Result,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_str, FnArg, ImplGenerics, ImplItem, ImplItemFn, ItemImpl, Pat,
    PathArguments, ReturnType, Type, TypeGenerics, WhereClause,
};

#[derive(Debug, Clone, FromMeta)]
#[darling(and_then = "Self::validate")]
struct FfiMethodOptArg {
    #[darling(rename = "self")]
    /// Whether this argument needs to be converted to the receiver type
    receiver: Flag,
    #[darling(default)]
    ty: Option<String>,
    #[darling(default)]
    rename: Option<String>,
    rest: Flag,
}

impl FfiMethodOptArg {
    fn validate(self) -> Result<Self> {
        if self.receiver.is_present() && self.rest.is_present() {
            Err(Error::custom(
                "An argument may either be self or have rest enabled.",
            ))
        } else if self.rest.is_present() && (self.ty.is_some() || self.rename.is_some()) {
            Err(Error::custom(
                "The rest argument may not specify a rename or type change",
            ))
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug, FromAttributes)]
#[darling(attributes(ffi))]
struct FfiMethodOpts {
    expect: Flag,
    #[darling(default)]
    visibility: Option<String>,
    #[darling(default)]
    name: Option<String>,
    #[darling(multiple)]
    arg: Vec<FfiMethodOptArg>,
}

impl FfiMethodOpts {
    fn visibility(&self) -> TokenStream2 {
        if let Some(ref visibility) = self.visibility {
            match parse_str(visibility) {
                Ok(visibility) => visibility,
                Err(e) => Error::from(e).write_errors(),
            }
        } else {
            // NOTE: Default is "pub" because typically this is required for FFI
            quote!(pub)
        }
    }
}

#[derive(Debug)]
struct FfiMethods<'a> {
    ffi_self_ty: Option<Type>,
    expect: Flag,
    self_ty: Type,
    self_generics: (ImplGenerics<'a>, TypeGenerics<'a>, Option<&'a WhereClause>),
    ffi_methods: Vec<WithOriginal<FfiMethodOpts, ImplItemFn>>,
    other_items: Vec<&'a ImplItem>,
}

impl<'a> TryFrom<(&'a ItemImpl, Option<Type>, Flag)> for FfiMethods<'a> {
    type Error = Error;

    fn try_from(value: (&'a ItemImpl, Option<Type>, Flag)) -> Result<Self> {
        let expect = value.2;
        let ffi_self_ty = value.1;
        let value = value.0;
        let self_generics = value.generics.split_for_impl();
        let mut ffi_methods = Vec::new();
        let mut other_items = Vec::new();
        let mut errors = Vec::new();

        value.items.iter().for_each(|i| {
            if let ImplItem::Fn(ref f) = i {
                match FfiMethodOpts::from_attributes(&f.attrs) {
                    Ok(opts) => {
                        let mut f = f.clone();
                        // NOTE: This effectively makes splitting the ffi() macro across multiple invocations
                        // an error. I'm okay with that, I don't like the syntax and it'll break the argument
                        // ordering anyway.
                        f.attrs
                            .retain(|a| FfiMethodOpts::from_attributes(&[a.clone()]).is_err());
                        ffi_methods.push(WithOriginal::new(opts, f));
                    }
                    Err(e) => errors.push(e),
                }
            } else {
                other_items.push(i);
            }
        });

        if !errors.is_empty() {
            Err(Error::multiple(errors))
        } else {
            Ok(Self {
                ffi_self_ty,
                expect,
                self_ty: *value.self_ty.clone(),
                self_generics,
                ffi_methods,
                other_items,
            })
        }
    }
}

impl<'a> FfiMethods<'a> {
    fn original(&self) -> TokenStream2 {
        let orig_ffi_methods = self
            .ffi_methods
            .iter()
            .map(|m| &m.original)
            .collect::<Vec<_>>();

        let other_items = &self.other_items;

        quote! {
            #(#orig_ffi_methods)*
            #(#other_items)*
        }
    }

    fn ffi_return_ty(return_ty: &ReturnType, expect: bool) -> (TokenStream2, TokenStream2, bool) {
        if expect {
            if let ReturnType::Type(_, t) = return_ty {
                if let Type::Path(p) = &**t {
                    if let Some(last) = p.path.segments.last() {
                        if last.ident == "Result" {
                            if let PathArguments::AngleBracketed(a) = &last.arguments {
                                return (
                                    quote!(#return_ty),
                                    a.args
                                        .first()
                                        .map(|a| quote!(-> #a))
                                        .unwrap_or(quote!(#return_ty)),
                                    true,
                                );
                            }
                        }
                    }
                }
            }
        }

        (quote!(#return_ty), quote!(#return_ty), false)
    }

    fn ffi(&self) -> TokenStream2 {
        // NOTE: by default, the first argument is the receiver.
        let mut methods = Vec::new();

        for method in &self.ffi_methods {
            let ffi_func_name = method
                .parsed
                .name
                .as_ref()
                .map(|n| {
                    let name = format_ident!("{n}");
                    quote!(#name)
                })
                .unwrap_or({
                    let name = &method.original.sig.ident;
                    quote!(#name)
                });

            let (_impl_method_return_ty, ffi_func_return_ty, need_expect) = Self::ffi_return_ty(
                &method.original.sig.output,
                method.parsed.expect.is_present() || self.expect.is_present(),
            );

            let Some(impl_method_receiver) = method.original.sig.receiver() else {
                abort!(method.original, "No receiver on method");
            };

            let maybe_mut_ref = impl_method_receiver.mutability.map(|m| quote!(#m));

            let impl_method_args = method.original.sig.inputs.iter().collect::<Vec<_>>();

            let impl_method_args_no_receiver = method
                .original
                .sig
                .inputs
                .iter()
                .filter(|a| !matches!(a, FnArg::Receiver(_)))
                .cloned()
                .collect::<Vec<_>>();

            let mut ffi_receiver_name = None;
            let mut ffi_func_args = Vec::new();
            let mut ffi_func_renames = HashMap::new();

            for (i, arg) in method.parsed.arg.iter().enumerate() {
                if arg.receiver.is_present() {
                    let ty = if let Some(ref ty) = arg.ty {
                        match parse_str::<Type>(ty) {
                            Ok(ty) => quote!(#ty),
                            Err(e) => return Error::from(e).write_errors(),
                        }
                    } else if let Some(ref ty) = self.ffi_self_ty {
                        quote!(#ty)
                    } else {
                        let ty = &self.self_ty;
                        quote!(#ty)
                    };

                    let name = arg
                        .rename
                        .as_ref()
                        .map(|n| {
                            let n = format_ident!("{n}");
                            quote!(#n)
                        })
                        .unwrap_or(quote!(slf));
                    ffi_func_args.push(quote!(#name: #ty));
                    ffi_receiver_name = Some(name);
                } else if arg.rest.is_present() {
                    // If we have already seen the receiver argument, we need to look one
                    // argument forward
                    let mut arg_index = i;

                    if ffi_receiver_name.is_none() {
                        arg_index += 1;
                    }

                    ffi_func_args.extend(
                        impl_method_args_no_receiver
                            .iter()
                            .enumerate()
                            .filter_map(|(i, a)| (i >= arg_index - 1).then_some(a))
                            .map(|a| quote!(#a)),
                    );
                } else if ffi_func_args.len() <= impl_method_args_no_receiver.len() + 1 {
                    // If we have already seen the receiver argument, we need to look one
                    // argument forward
                    let mut arg_index = i;

                    if ffi_receiver_name.is_none() {
                        arg_index += 1;
                    }

                    if let Some(FnArg::Typed(impl_method_arg_pat_type)) =
                        impl_method_args.get(arg_index)
                    {
                        let ty = &impl_method_arg_pat_type.ty;
                        if let Some(ref rename) = arg.rename {
                            ffi_func_renames.insert(i, (rename, ty));
                            ffi_func_args.push({
                                let rename = format_ident!("{rename}");
                                quote!(#rename: #ty)
                            });
                        } else {
                            ffi_func_args.push(quote!(#impl_method_arg_pat_type));
                        }
                    } else {
                        return Error::custom(
                            "Argument is not a typed argument while getting ffi function arguments",
                        )
                        .write_errors();
                    }
                } else {
                    return Error::custom(
                        "Argument is not a typed argument while getting ffi function arguments",
                    )
                    .write_errors();
                }
            }

            let mut impl_method_call_args = Vec::new();

            for (i, arg) in impl_method_args_no_receiver.iter().enumerate() {
                if let Some((rename, _ty)) = ffi_func_renames.get(&i) {
                    let ident = format_ident!("{rename}");
                    impl_method_call_args.push(quote!(#ident));
                } else {
                    let FnArg::Typed(ref typed) = arg else {
                        return Error::custom(format!("Argument {i} is not a typed argument"))
                            .write_errors();
                    };
                    let Pat::Ident(ref ident) = &*typed.pat else {
                        return Error::custom("Pattern is not an identifier").write_errors();
                    };
                    let ident = &ident.ident;
                    impl_method_call_args.push(quote!(#ident));
                }
            }

            let self_ty = &self.self_ty;
            let ffi_func_visibility = method.parsed.visibility();
            let Some(self_name) = ffi_receiver_name else {
                return Error::custom("No receiver name").write_errors();
            };
            let impl_method_name = &method.original.sig.ident;
            let impl_maybe_expect = need_expect
                .then_some({
                    let expect_message =
                        format!("Failed to execute FFI method {}", method.original.sig.ident);
                    quote!(.expect(#expect_message))
                })
                .unwrap_or_default();
            let (_self_impl_genrics, self_ty_generics, self_where_clause) = &self.self_generics;

            let impl_method_call = quote! {
                    Into::<&#maybe_mut_ref #self_ty>::into(#self_name).#impl_method_name(
                        #(#impl_method_call_args),*
                    )#impl_maybe_expect
            };

            methods.push(quote! {
                #[no_mangle]
                #ffi_func_visibility extern "C" fn #ffi_func_name #self_ty_generics(
                    #(#ffi_func_args),*
                ) #ffi_func_return_ty #self_where_clause {
                    #impl_method_call
                }
            })
        }

        quote! {
            #(#methods)*
        }
    }
}

#[derive(Debug, FromMeta)]
struct FfiOpts {
    #[darling(default, rename = "mod_name")]
    name: Option<String>,
    #[darling(default)]
    visibility: Option<String>,
    #[darling(default)]
    self_ty: Option<String>,
    expect: Flag,
    from_ptr: Flag,
    from_any_ptr: Flag,
}

#[proc_macro_attribute]
#[proc_macro_error]
/// FFI helper macro
///
/// Accepts the following options:
///
/// ```rust,ignore
/// #[ffi(
///     mod_name = "your_ffi_mod_name",
///     visibility = "pub(crate)",
///     self_ty = "*mut std::ffi::c_void",
///     expect
/// )]
/// impl Foo {}
/// ```
///
/// All options are optional. The module name defaults to the name of the implementation's self
/// type (i.e. `Foo` in the example above). The visibility defaults to `pub`. The self type
/// on each method takes priority, followed by the implementation-wide type, then defaults to
/// the self type of the implementation if noneis provided. By default, result types will not
/// be unwrapped with `.expect()` and will be returned as-is.
///
/// Function items in the implementation accept their own set of arguments.
///
/// ```rust,ignore
/// #[ffi(
///     mod_name = "your_ffi_mod_name",
///     visibility = "pub(crate)",
///     self_ty = "*mut std::ffi::c_void",
///     expect
/// )]
/// impl Foo {
///     #[ffi(
///         expect,
///         visibility = "pub(crate)",
///         name = "bar_override",
///         arg(self),
///         arg(rest),
///     )]
///     pub fn bar(&self, a: u64, b: Baz) -> anyhow::Result<u64> {
///         self.total += a + b.xyzzy(a);
///         Ok(a)
///     }
/// }
/// ```
///
/// The first three arguments (expect, visibility, name) are optional. Expect and visibility have
/// the same meaning as the impl-wide settings, and allow more granular control. `arg` can be
/// specified multiple times, with the following flags. `self`, marks the arg in the corresponding
/// position in the FFI callback's parameters as the receiver. `ty = "OtherType"` allows overriding
/// the type of individual FFI parameters (for example, to receive void pointers instead of
/// references. `rename = "othername"` to rename the parameter, and `rest`, which indicates the
/// remaining arguments should be added as-they-are to the FFI function's parameters.
pub fn ffi(args: TokenStream, input: TokenStream) -> TokenStream {
    let meta = match NestedMeta::parse_meta_list(args.into()) {
        Ok(o) => o,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    // Extract the options from the #[ffi()] attribute
    let impl_item_opts = match FfiOpts::from_list(&meta) {
        Ok(o) => o,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let impl_item = parse_macro_input!(input as ItemImpl);

    // Extract the trait component of the `impl X (for Y) {` item. We need this in addition to the
    // generics below because we re-emit the original implementation.
    let maybe_trait = impl_item.trait_.as_ref().map(|(not, path, f)| {
        let maybe_not = not.map(|not| quote!(#not)).unwrap_or_default();
        quote!(#maybe_not #path #f)
    });

    let impl_generics = &impl_item.generics.params.iter().collect::<Vec<_>>();
    let where_clause = &impl_item.generics.where_clause;

    let (impl_item_name, self_ty_generics) = if let Type::Path(p) = &*impl_item.self_ty {
        if let Some(last) = p.path.segments.last() {
            match last.arguments {
                PathArguments::None => {
                    let name = &impl_item.self_ty;
                    (quote!(#name), vec![])
                }
                PathArguments::AngleBracketed(ref a) => {
                    let last_ident = &last.ident;
                    let mut segments = p.path.segments.iter().cloned().collect::<Vec<_>>();
                    segments.pop();
                    let segments = segments.iter().map(|s| quote!(#s)).collect::<Vec<_>>();
                    let impl_item_name = quote!(#(#segments)::*#last_ident);
                    let ty_generics = a.args.clone().into_iter().collect::<Vec<_>>();
                    (impl_item_name, ty_generics)
                }
                PathArguments::Parenthesized(_) => abort!(
                    impl_item,
                    "Parenthesized path arguments are not allowed here"
                ),
            }
        } else {
            abort!(impl_item, "Self type must have segments");
        }
    } else {
        abort!(impl_item, "Self type must be path");
    };

    let impl_methods = match FfiMethods::try_from((
        &impl_item,
        impl_item_opts
            .self_ty
            .as_ref()
            .and_then(|s| parse_str::<Type>(s).ok()),
        impl_item_opts.expect,
    )) {
        Ok(o) => o,
        Err(e) => return e.write_errors().into(),
    };

    let impl_methods_original = &impl_methods.original();

    let ffi_mod_name = match impl_item_opts.name.map(|n| {
        let n = format_ident!("{n}");
        quote!(#n)
    }) {
        Some(n) => n,
        None => {
            let Type::Path(path) = impl_item.self_ty.as_ref() else {
                abort!(impl_item, "Implementation self type is not a path");
            };
            let Some(name) = path.path.segments.first() else {
                abort!(path, "Path has no segments");
            };
            let ffi_mod_name = format_ident!("{}", name.ident.to_string().to_ascii_lowercase());
            quote!(#ffi_mod_name)
        }
    };

    let ffi_mod_visibility = if let Some(ref visibility) = impl_item_opts.visibility {
        match parse_str(visibility) {
            Ok(visibility) => visibility,
            Err(e) => return TokenStream::from(Error::from(e).write_errors()),
        }
    } else {
        // NOTE: Defaults to public visibility, because this is typically requred for FFI
        quote!(pub)
    };

    let ffi_mod_methods = &impl_methods.ffi();

    let mut impl_generics_from = self_ty_generics
        .iter()
        .map(|g| quote!(#g))
        .collect::<Vec<_>>();

    if impl_item_opts.from_any_ptr.is_present() {
        impl_generics_from.push(quote!(T));
    }

    let maybe_from_any_ptr = impl_item_opts.from_any_ptr.is_present().then_some(quote! {
        impl<#(#impl_generics_from),*> From<*mut T> for &'static mut #impl_item_name<#(#self_ty_generics),*> {
            fn from(value: *mut T) -> Self {
                let ptr: *mut #impl_item_name <#(#self_ty_generics),*>= value as *mut #impl_item_name <#(#self_ty_generics),*>;
                unsafe { &mut *ptr }
            }
        }

        impl<#(#impl_generics_from),*> From<*mut T> for &'static #impl_item_name<#(#self_ty_generics),*> {
            fn from(value: *mut T) -> Self {
                let ptr: *mut #impl_item_name <#(#self_ty_generics),*> = value as *mut #impl_item_name <#(#self_ty_generics),*>;
                unsafe { &*ptr }
            }
        }

        impl<#(#impl_generics_from),*> From<*const T> for &'static #impl_item_name<#(#self_ty_generics),*> {
            fn from(value: *const T) -> Self {
                let ptr: *const #impl_item_name <#(#self_ty_generics),*> = value as *const #impl_item_name <#(#self_ty_generics),*>;
                unsafe { &*ptr }
            }
        }
    }).unwrap_or_default();

    let maybe_from_ptr = if impl_item_opts.from_ptr.is_present() {
        impl_item_opts
            .self_ty
            .as_ref()
            .and_then(|st| {
                parse_str(st).ok().map(|stp: Type| {
                    quote! {
                        impl<#(#impl_generics_from),*> From<#stp> for &'static mut #impl_item_name<#(#self_ty_generics),*> {
                            fn from(value: #stp) -> Self {
                                let ptr: *mut #impl_item_name <#(#self_ty_generics),*>= value as *mut #impl_item_name <#(#self_ty_generics),*>;
                                unsafe { &mut *ptr }
                            }
                        }

                        impl<#(#impl_generics_from),*> From<#stp> for &'static #impl_item_name<#(#self_ty_generics),*> {
                            fn from(value: #stp) -> Self {
                                let ptr: *mut #impl_item_name <#(#self_ty_generics),*> = value as *mut #impl_item_name <#(#self_ty_generics),*>;
                                unsafe { &*ptr }
                            }
                        }
                    }
                })
            })
            .unwrap_or_default()
    } else {
        quote!()
    };

    let maybe_impl_generics = if impl_generics.is_empty() {
        quote!()
    } else {
        quote!(<#(#impl_generics),*>)
    };

    let maybe_self_ty_generics = if self_ty_generics.is_empty() {
        quote!()
    } else {
        quote!(<#(#self_ty_generics),*>)
    };

    let q: TokenStream = quote! {
        impl #maybe_impl_generics #maybe_trait #impl_item_name #maybe_self_ty_generics #where_clause {
            #impl_methods_original
        }

        #maybe_from_ptr

        #maybe_from_any_ptr

        #ffi_mod_visibility mod #ffi_mod_name {
            use super::*;
            #ffi_mod_methods
        }
    }
    .into();

    // println!("{q}");

    q
}
