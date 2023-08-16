// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Provides the `callback_wrappers` attribute for automatically generating CFFI functions for
//! callbacks into struct methods

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote};
use std::hash::Hash;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    FnArg, Ident, ItemImpl, ReturnType, Token, Type,
};

/// A type or '...'
enum CallbackWrapperArgParam {
    Arg(FnArg),
    BangArg(FnArg),
    Ellipsis,
}

impl Parse for CallbackWrapperArgParam {
    /// Parse a single callback wrapper parameter
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            let typ = input.parse::<FnArg>()?;
            Ok(Self::BangArg(typ))
        } else if input.peek(Token![...]) {
            input.parse::<Token![...]>()?;
            Ok(Self::Ellipsis)
        } else {
            let typ = input.parse::<FnArg>()?;
            Ok(Self::Arg(typ))
        }
    }
}

/// Parameters to a callback wrapper attribute
struct CallbackWrapperArgParams {
    params: Vec<CallbackWrapperArgParam>,
}

impl Parse for CallbackWrapperArgParams {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed = Punctuated::<CallbackWrapperArgParam, Token![,]>::parse_terminated(input)?;

        let params: Vec<CallbackWrapperArgParam> = parsed.into_iter().collect();

        Ok(Self { params })
    }
}

#[derive(Hash, Eq, PartialEq)]
enum CallbackWrapperArgType {
    /// The argument index of the callback function that is a pointer to the instance the callback
    /// function is called on. This pointer must be convertible to a `&self` or `&mut self` reference,
    /// depending on the callback function's receiver parameter mutability.
    Pub,
    /// Whether result types should be unwrapped by the wrapper function (to allow the callback
    /// function to return `Result<T, E>` instead of `T`).
    UnwrapResult,
    /// Trace callback entrypoints with tracing::trace!
    Trace,
}

impl Parse for CallbackWrapperArgType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(_pub_literal) = input.parse::<Token![pub]>() {
            Ok(Self::Pub)
        } else if let Ok(ident) = input.parse::<Ident>() {
            match ident.to_string().as_str() {
                "unwrap_result" => Ok(Self::UnwrapResult),
                "trace" => Ok(Self::Trace),
                _ => abort!(ident.span(), "Unknown callback wrapper argument type"),
            }
        } else {
            abort!(input.span(), "Expected 'pub'");
        }
    }
}

enum CallbackWrapperArgValue {
    None,
}

struct CallbackWrapperArg {
    typ: CallbackWrapperArgType,
    _value: CallbackWrapperArgValue,
}

impl Parse for CallbackWrapperArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let typ = input.parse::<CallbackWrapperArgType>()?;
        let _value = match typ {
            CallbackWrapperArgType::Pub
            | CallbackWrapperArgType::UnwrapResult
            | CallbackWrapperArgType::Trace => CallbackWrapperArgValue::None,
        };
        Ok(Self { typ, _value })
    }
}

struct CallbackWrapperArgs {
    args: Vec<CallbackWrapperArg>,
}

impl Parse for CallbackWrapperArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed = if let Ok(parsed) =
            Punctuated::<CallbackWrapperArg, Token![,]>::parse_terminated(input)
        {
            parsed
        } else {
            abort! {
                input.span(),
                "Failed to parse aguments to `callback_wrappers()`"
            };
        };

        let args: Vec<CallbackWrapperArg> = parsed.into_iter().collect();

        Ok(Self { args })
    }
}

impl CallbackWrapperArgs {
    pub fn is_pub(&self) -> bool {
        self.args
            .iter()
            .any(|arg| matches!(arg.typ, CallbackWrapperArgType::Pub))
    }

    pub fn has_unwrap_result(&self) -> bool {
        self.args
            .iter()
            .any(|arg| matches!(arg.typ, CallbackWrapperArgType::UnwrapResult))
    }

    pub fn has_trace(&self) -> bool {
        self.args
            .iter()
            .any(|arg| matches!(arg.typ, CallbackWrapperArgType::Trace))
    }
}

#[proc_macro_attribute]
pub fn params(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_error]
#[proc_macro_attribute]
/// Create a companion C FFI function that calls the Rust method.
///
/// # Syntax
///
/// Below is an example of the syntax for this macro. The `callback_wrappers` macro must be
/// applied to an `impl` block. The `params` macro must be applied to *all* methods in that
/// `impl` block. Parameters specified in the `params` must be in the order the C FFI callback
/// function expects to receive them from the C code. The parameter that is a pointer or some
/// other value that can be converted to a `&self` or `&mut self` reference must be prefixed
/// with a `!`, and it must have a `From` implementation from the type of the `self` parameter in
/// the FFI callback function to the type of the `self` parameter in the Rust method (e.g. either
/// `&self` or `&mut self`).
///
/// ```ignore
/// #[callback_wrappers(<visibility>?, <unwrap_result>?)]
///
/// impl <type> {
///    #[params(<arg0>, <arg1> ..., !<self>)]
///     pub fn <method_name>(&self, <arg0>, <arg1> ...) -> <return_type> {
///     }
/// }
/// ```
///
///
/// # Examples
///
/// This will generate an extern "C" function `test_callbacks::test` that calls the Rust method
/// `Test::test`, with the first argument being a pointer to the instance of `Test`.
///
/// ```rust,ignore
/// use ffi_macro::{callback_wrappers, params};
///
/// pub struct TestStruct {}
///
/// #[callback_wrappers(pub)]
/// impl TestStruct {
///    #[params(!slf: *mut std::ffi::c_void, ...)]
///   pub fn test(&self, a: i32, b: i32) -> i32 {
///        a + b
///   }
/// }
/// ```
///
/// We can also use the `unwrap_result` argument to tell our generated FFI functions to unwrap
/// result types from our method.
///
/// ```rust,ignore
/// use ffi_macro::{callback_wrappers, params};
/// use anyhow::Result;
///
/// pub struct TestStruct {}
///
/// #[callback_wrappers(pub, unwrap_result)]
/// impl TestStruct {
///    #[params(!slf: *mut std::ffi::c_void, ...)]
///   pub fn test2(&self, a: i32, b: i32) -> Result<i32> {
///        Ok(a + b)
///   }
/// }
/// ```
pub fn callback_wrappers(args: TokenStream, input: TokenStream) -> TokenStream {
    let impl_args = parse_macro_input!(args as CallbackWrapperArgs);
    let visibility = if impl_args.is_pub() {
        quote! { pub }
    } else {
        quote! {}
    };
    let implementation = parse_macro_input!(input as ItemImpl);

    let struct_name = if let Type::Path(ty) = &*implementation.self_ty {
        if let Some(p) = ty.path.segments.first() {
            p.ident.clone()
        } else {
            abort! {
                ty,
                "Path contains no entries"
            }
        }
    } else {
        abort! {
            implementation,
            "Could not obtain struct type from implementation"
        }
    };

    let struct_name_string = quote! { #struct_name }.to_string().to_ascii_lowercase();

    let impl_fns = implementation
        .items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                Some(method)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let cb_mod_name = format_ident!("{}_callbacks", struct_name_string);

    let callbacks = impl_fns
        .iter()
        .map(|f| {
            let attrs = &f.attrs;
            let is_unsafe = f.sig.unsafety.is_some();
            let args: CallbackWrapperArgParams =
                if let Some(args) = attrs.iter().find(|a| a.path().is_ident("params")) {
                    match args.parse_args() {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            abort! {
                                args,
                                "Could not parse arguments to `params` attribute: {}", e
                            }
                        }
                    }
                } else {
                    abort! {
                        f,
                        "Expected `params` attribute"
                    };
                };
            let fname = &f.sig.ident;

            // Check if frty is a Result
            let is_result = if let ReturnType::Type(_, ty) = &f.sig.output {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(segment) = tp.path.segments.last() {
                        segment.ident == "Result"
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            let frty = if is_result {
                quote! {}
            } else {
                let rty = &f.sig.output;
                quote! { #rty  }
            };

            let receiver = &f.sig.receiver().expect("No method receiver (self parameter) found on function");

            // Get the args without the receiver, this will be dropped in for 'Ellipsis'
            let fargs = &f
                .sig
                .inputs
                .iter()
                .filter(|fa| !matches!(fa, syn::FnArg::Receiver(_)))
                .cloned()
                .collect::<Vec<_>>();

            let mut cb_args = Vec::new();
            let mut cb_receiver_arg_offset = None;
            let mut receiver_arg = None;

            for param in args.params {
                match param {
                    CallbackWrapperArgParam::BangArg(arg) => {
                        receiver_arg = Some(arg.clone());

                        cb_args.push(arg);
                        cb_receiver_arg_offset = Some(cb_args.len() - 1);
                    }
                    CallbackWrapperArgParam::Arg(arg) => cb_args.push(arg),
                    CallbackWrapperArgParam::Ellipsis => {
                        for arg in fargs {
                            cb_args.push(arg.clone());
                        }
                    }
                }
            }

            let (receive, receiver_ident)= match receiver_arg {
                Some(arg) => {
                    let ident = match arg {
                        FnArg::Typed(pat) => match *pat.pat {
                            syn::Pat::Ident(ref ident) => ident.ident.clone(),
                            _ => abort! {
                                pat,
                                "expected identifier"
                            },
                        },
                        _ => abort! {
                            arg,
                            "expected identifier"
                        },
                    };
                    (quote! {
                        let #ident: &mut #struct_name = #ident.into();
                    }, ident)
                }
                None => {
                    abort! {
                        receiver,
                        "expected receiver"
                    }
                }
            };

            let cb_selfcall_args = if let Some(offset) = cb_receiver_arg_offset {
                let mut args = cb_args.clone();
                args.remove(offset);
                args
            } else {
                cb_args.clone()
            };

            let cb_selfcall_args_identsonly = cb_selfcall_args
                .iter()
                .map(|a| {
                    let ident = match a {
                        FnArg::Typed(pat) => match *pat.pat {
                            syn::Pat::Ident(ref ident) => ident.ident.clone(),
                            _ => abort! {
                                pat,
                                "expected identifier"
                            },
                        },
                        _ => abort! {
                            a,
                            "expected identifier"
                        },
                    };
                    quote! { #ident }
                })
                .collect::<Vec<_>>();

            let fname_string = f.sig.ident.to_string();

            let unwrap_mb = if is_result && impl_args.has_unwrap_result() {
                quote! { .expect(&format!("Error: unable to unwrap result from FFI callback {}::{}", #struct_name_string, #fname_string)); }
            } else if is_result {
                abort! { f.sig.output, "Result output, but `unwrap_result` was not specified" }
            } else {
                quote! {}
            };


            let trace_mb = if impl_args.has_trace() {
                quote! {
                    tracing::trace!("Callback {}::{} executed", #struct_name_string, #fname_string);
                }
            } else {
                quote! {}
            };

            let call = quote! {
                #receiver_ident.#fname(
                    #( #cb_selfcall_args_identsonly ),*
                )#unwrap_mb
            };

            let call = if is_unsafe {
                quote! {
                    unsafe { #call }
                }
            } else {
                call
            };

            quote! {
                #[no_mangle]
                pub extern "C" fn #fname(
                    #( #cb_args ),*
                ) #frty {
                    #trace_mb
                    #receive
                    #call
                }
            }
        })
        .collect::<Vec<_>>();

    let r: TokenStream = quote! {
        #implementation

        #visibility mod #cb_mod_name {
            use super::*;

            #(#callbacks)*
        }
    }
    .into();

    // let _s = r.to_string();

    // eprintln!("{}", _s);

    r
}
