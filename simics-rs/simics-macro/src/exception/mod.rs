// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, ItemFn, Meta, ReturnType, Type, Visibility};

#[derive(Debug, FromMeta)]
struct SimicsExceptionOpts {}

pub trait IsResultType {
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

pub fn simics_exception_impl(args: TokenStream, input: TokenStream) -> TokenStream {
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

            parse_quote!(-> crate::error::Result<#output>)
        }
    };

    let maybe_ty_generics = (!&sig.generics.params.is_empty()).then_some({
        let params = &sig.generics.params;
        quote!(::<#params>)
    });

    let Some(args) = sig
        .inputs
        .iter()
        .map(|i| match i {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(t) => {
                let pat = &t.pat;
                Some(quote!(#pat))
            }
        })
        .collect::<Option<Vec<_>>>()
    else {
        return Error::custom("Methods with a receiver are not supported")
            .write_errors()
            .into();
    };

    let wrapper = quote! {
        #(#doc_attrs)*
        #vis #sig {
            #[allow(deprecated)]
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
