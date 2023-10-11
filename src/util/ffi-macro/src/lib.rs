// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Provides the `callback_wrappers` attribute for automatically generating CFFI functions for
//! callbacks into struct methods

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use darling::{ast::NestedMeta, util::WithOriginal, Error, FromAttributes, FromMeta, Result};
use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, ImplItem, ImplItemFn, ItemImpl, Type};

#[derive(Debug, FromAttributes)]
#[darling(attributes(args))]
struct FfiMethodOpts {
    #[darling(default)]
    expect: bool,
}

struct FfiMethods {
    methods: Vec<WithOriginal<FfiMethodOpts, ImplItemFn>>,
}

impl TryFrom<ItemImpl> for FfiMethods {
    type Error = Error;

    fn try_from(value: ItemImpl) -> Result<Self> {
        let mut methods = Vec::new();
        let mut errors = Vec::new();

        value.items.into_iter().for_each(|i| {
            if let ImplItem::Fn(f) = i {
                println!("{:?}", f.attrs);
                match FfiMethodOpts::from_attributes(&f.attrs) {
                    Ok(o) => methods.push(WithOriginal::new(o, f)),
                    Err(e) => errors.push(e),
                }
            }
        });

        if !errors.is_empty() {
            Err(Error::multiple(errors))
        } else {
            Ok(Self { methods })
        }
    }
}

impl ToTokens for FfiMethods {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {}
}

#[derive(Debug, FromMeta)]
#[]
struct FfiOpts {
    #[darling(default)]
    visibility: Option<String>,
    #[darling(default)]
    name: Option<String>,
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn ffi(args: TokenStream, input: TokenStream) -> TokenStream {
    let meta = match NestedMeta::parse_meta_list(args.into()) {
        Ok(o) => o,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let opts = match FfiOpts::from_list(&meta) {
        Ok(o) => o,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let impl_item = parse_macro_input!(input as ItemImpl);

    let methods = match FfiMethods::try_from(impl_item.clone()) {
        Ok(o) => o,
        Err(e) => return e.write_errors().into(),
    };

    let Type::Path(impl_item_path) = *(impl_item.self_ty.clone()) else {
        abort!(impl_item, "Could not obtain self type name");
    };

    let Some(impl_item_name) = impl_item_path
        .path
        .segments
        .first()
        .map(|s| s.ident.clone())
    else {
        abort!(impl_item_path, "Could not get name from path");
    };

    let visibility = match opts.visibility.as_deref() {
        Some("pub") => quote!(pub),
        Some("pub(crate)") => quote!(pub(crate)),
        _ => quote!(),
    };

    let ffi_mod_name = opts
        .name
        .map(|name| {
            let name_ident = format_ident!("{}", name);
            quote!(#name_ident)
        })
        .unwrap_or(quote!(#impl_item_name));

    quote! {
        #impl_item

        #visibility mod #ffi_mod_name {
        }
    }
    .into()
}
