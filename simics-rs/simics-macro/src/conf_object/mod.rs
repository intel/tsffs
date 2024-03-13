// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use darling::{util::Flag, FromDeriveInput};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Generics, Ident};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(conf_object), supports(struct_named))]
struct AsConfObjectOpts {
    ident: Ident,
    generics: Generics,
}

impl ToTokens for AsConfObjectOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        tokens.extend(quote! {
            impl #impl_generics simics::AsConfObject for #ident #ty_generics #where_clause {}
        })
    }
}

/// Derive macro for the [`Class`] trait.
pub fn as_conf_object_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match AsConfObjectOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(conf_object), supports(struct_named))]
struct FromConfObjectOpts {
    ident: Ident,
    generics: Generics,
    /// Whether to skip also implementing `From<*_ ConfObject>` for this type
    skip_from: Flag,
}

impl ToTokens for FromConfObjectOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        tokens.extend(quote! {
            impl #impl_generics simics::FromConfObject for #ident #ty_generics #where_clause {}
        });

        if !self.skip_from.is_present() {
            // Add the 'a lifetime to impl_generics list
            let mut generics = self.generics.params.iter().cloned().collect::<Vec<_>>();

            generics.insert(0, parse_quote!('x));

            let generics = Generics {
                params: parse_quote!(#(#generics),*),
                where_clause: self.generics.where_clause.clone(),
                ..Default::default()
            };

            let (impl_generics, _ty_generics, _where_clause) = generics.split_for_impl();

            tokens.extend(quote! {
                impl #impl_generics From<*const simics::ConfObject> for &'x #ident #ty_generics #where_clause {
                    #[allow(clippy::not_unsafe_ptr_arg_deref)]
                    /// Convert a raw `ConfObject` pointer to a reference to this object
                    /// 
                    /// # Safety
                    /// 
                    /// This function dereferences a raw pointer. It must be called with a valid pointer which
                    /// has a sufficient lifetime.
                    fn from(obj: *const simics::ConfObject) -> &'x #ident #ty_generics #where_clause {
                        unsafe { <#ident as simics::FromConfObject>::from_conf_object(obj) }
                    }
                }

                impl #impl_generics From<*mut simics::ConfObject> for &'x #ident #ty_generics #where_clause {
                    #[allow(clippy::not_unsafe_ptr_arg_deref)]
                    /// Convert a raw `ConfObject` pointer to a mutable reference to this object
                    /// 
                    /// # Safety
                    /// 
                    /// This function dereferences a raw pointer. It must be called with a valid pointer which
                    /// has a sufficient lifetime.
                    fn from(obj: *mut simics::ConfObject) -> &'x #ident #ty_generics #where_clause {
                        unsafe { <#ident as simics::FromConfObject>::from_conf_object(obj) }
                    }
                }

                impl #impl_generics From<*mut simics::ConfObject> for &'x mut #ident #ty_generics #where_clause {
                    #[allow(clippy::not_unsafe_ptr_arg_deref)]
                    /// Convert a raw `ConfObject` pointer to a mutable reference to this object
                    /// 
                    /// # Safety
                    /// 
                    /// This function dereferences a raw pointer. It must be called with a valid pointer which
                    /// has a sufficient lifetime.
                    fn from(obj: *mut simics::ConfObject) -> &'x mut #ident #ty_generics #where_clause {
                        unsafe { <#ident as simics::FromConfObject>::from_conf_object_mut(obj) }
                    }
                }
            })
        }
    }
}

/// Derive macro for the [`Class`] trait.
pub fn from_conf_object_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = match FromConfObjectOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}
