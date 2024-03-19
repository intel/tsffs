// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use darling::{ast::Data, util::Flag, Error, FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Generics, Ident, Type};

#[derive(Debug, FromField)]
#[darling(attributes(attr_value))]
/// A field in a struct that can be converted into an `AttrValue`
struct IntoAttrValueField {
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
    /// Whether this field should be skipped when converting this type into an
    /// `AttrValue`
    skip: Flag,
    /// Whether this field should be fallibly converted using `try_into` instead
    /// of `into`. This cannot be detected automatically by the proc-macro.
    fallible: Flag,
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(attr_value),
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
        non_exhaustive
    )
)]
/// A structure that can be converted into an `AttrValue` list
struct IntoAttrValueListOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), IntoAttrValueField>,
}

impl ToTokens for IntoAttrValueListOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let Some(fields) = self.data.as_ref().take_struct() else {
            tokens.extend(Error::custom("Only structs are supported").write_errors());
            return;
        };

        let value_initializers = fields
            .iter()
            .filter(|f| !f.skip.is_present())
            .filter_map(|f| {
                f.ident.as_ref().map(|i| {
                    if f.fallible.is_present() {
                        quote!(let #i = AttrValue::try_from(value.#i)?;)
                    } else {
                        quote!(let #i = AttrValue::from(value.#i);)
                    }
                })
            })
            .collect::<Vec<_>>();

        if fields.iter().any(|f| f.fallible.is_present()) {
            let value_fields = fields
                .iter()
                .filter(|f| !f.skip.is_present())
                .map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("No identifier for field. This should be impossible");
                    quote!(#ident.into())
                })
                .collect::<Vec<_>>();
            // If any conversion is fallible, we can only implement TryInto
            tokens.extend(quote! {
                impl #impl_generics TryFrom<#ident #ty_generics> for simics::AttrValue #where_clause {
                    type Error = simics::Error;
                    fn try_from(value: #ident #ty_generics) -> simics::Result<Self> {
                        #( #value_initializers )*
                        Ok(simics::AttrValueType::List(
                            [
                                #( #value_fields ),*
                            ]
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                        ).into())
                    }
                }

                impl #impl_generics TryFrom<&#ident #ty_generics> for simics::AttrValue #where_clause {
                    type Error = simics::Error;
                    fn try_from(value: &#ident #ty_generics) -> simics::Result<Self> {
                        value.clone().try_into()
                    }
                }
            });
        } else {
            let value_fields = fields
                .iter()
                .filter(|f| !f.skip.is_present())
                .map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("No identifier for field. This should be impossible");
                    quote!(#ident.into())
                })
                .collect::<Vec<_>>();
            // No conversions are fallible, so we can implement Into
            tokens.extend(quote! {
                impl #impl_generics From<#ident #ty_generics> for simics::AttrValue #where_clause {
                    fn from(value: #ident #ty_generics) -> Self {
                        #( #value_initializers )*
                        simics::AttrValueType::List(
                            [
                                #( #value_fields ),*
                            ]
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                        ).into()
                    }
                }

                impl #impl_generics From<&#ident #ty_generics> for simics::AttrValue #where_clause {
                    fn from(value: &#ident #ty_generics) -> Self {
                        value.clone().into()
                    }
                }
            });
        }
    }
}

pub fn into_attr_value_list_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let args = match IntoAttrValueListOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(attr_value),
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
        non_exhaustive
    )
)]
/// A structure that can be converted into an `AttrValue` dict
struct IntoAttrValueDictOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), IntoAttrValueField>,
}

impl ToTokens for IntoAttrValueDictOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let Some(fields) = self.data.as_ref().take_struct() else {
            tokens.extend(Error::custom("Only structs are supported").write_errors());
            return;
        };

        let value_initializers= fields
            .iter()
            .filter(|f| !f.skip.is_present())
            .filter_map(|f| {
                f.ident.as_ref().map(|i| {
                    let ident_name = i.to_string();

                    if f.fallible.is_present() {
                        quote!(let #i = (#ident_name.into(), simics::AttrValue::try_from(value.#i)?.into());)
                    } else {
                        quote!(let #i = (#ident_name.into(), simics::AttrValue::from(value.#i).into());)
                    }
                })
            })
            .collect::<Vec<_>>();

        if fields.iter().any(|f| f.fallible.is_present()) {
            let value_fields = fields
                .iter()
                .filter(|f| !f.skip.is_present())
                .map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("No identifier for field. This should be impossible");
                    quote!(#ident.into())
                })
                .collect::<Vec<_>>();
            // If any conversion is fallible, we can only implement TryInto
            tokens.extend(quote! {
                impl #impl_generics TryFrom<#ident #ty_generics> for simics::AttrValue #where_clause {
                    type Error = simics::Error;
                    fn try_from(value: #ident #ty_generics) -> simics::Result<Self> {
                        #( #value_initializers )*
                        Ok(simics::AttrValueType::Dict(
                            std::collections::BTreeMap::from_iter([
                                #(#value_fields),*
                            ])
                        ).into())
                    }
                }

                impl #impl_generics TryFrom<&#ident #ty_generics> for simics::AttrValue #where_clause {
                    type Error = simics::Error;
                    fn try_from(value: &#ident #ty_generics) -> simics::Result<Self> {
                        value.clone().try_into()
                    }
                }
            });
        } else {
            let value_fields = fields
                .iter()
                .filter(|f| !f.skip.is_present())
                .map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("No identifier for field. This should be impossible.");
                    quote!(#ident.into())
                })
                .collect::<Vec<_>>();
            // No conversions are fallible, so we can implement Into
            tokens.extend(quote! {
                impl #impl_generics From<#ident #ty_generics> for simics::AttrValue #where_clause {
                    fn from(value: #ident #ty_generics) -> Self {
                        #( #value_initializers )*
                        simics::AttrValueType::Dict(
                            std::collections::BTreeMap::from_iter([
                                #(#value_fields),*
                            ])
                        ).into()
                    }
                }

                impl #impl_generics From<&#ident #ty_generics> for simics::AttrValue #where_clause {
                    fn from(value: &#ident #ty_generics) -> Self {
                        value.clone().into()
                    }
                }
            });
        }
    }
}

pub fn into_attr_value_dict_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let args = match IntoAttrValueDictOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}

#[derive(Debug, FromField)]
#[darling(attributes(attr_value))]
/// A field in a struct that can be converted from an `AttrValue`
struct FromAttrValueField {
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
    #[allow(unused)]
    /// Skip this field when converting to an `AttrValue`. This flag is ignored when
    /// converting from an `AttrValue`.
    skip: Flag,
    #[allow(unused)]
    /// Whether this field should be fallibly converted
    fallible: Flag,
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(attr_value),
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
        non_exhaustive
    )
)]
struct FromAttrValueListOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), FromAttrValueField>,
}

impl ToTokens for FromAttrValueListOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let Some(fields) = self.data.as_ref().take_struct() else {
            tokens.extend(Error::custom("Only structs are supported").write_errors());
            return;
        };

        let value_fields = fields
            .iter()
            .enumerate()
            .filter_map(|(index, f)| {
                f.ident.clone().map(|ident| {
                    quote! {
                        #ident: value.get(#index)
                                .ok_or_else(|| simics::Error::AttrValueListIndexOutOfBounds {
                                    index: #index,
                                    length: value.len()
                                })?
                                .clone()
                                .try_into()?
                    }
                })
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #impl_generics TryFrom<simics::AttrValue> for #ident #ty_generics #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::AttrValue) -> simics::Result<Self> {
                    println!("{:?}", value);
                    let value: Vec<simics::AttrValueType> = value.as_heterogeneous_list()
                        .ok_or_else(|| simics::Error::AttrValueType {
                            actual: value.kind(),
                            expected: simics::AttrKind::Sim_Val_List,
                            reason: "Expected a list of heterogeneous values".to_string(),
                        })?;

                    Ok(Self {
                        #(#value_fields),*
                    })
                }
            }

            impl #impl_generics TryFrom<&simics::AttrValue> for #ident #ty_generics #where_clause {
                type Error = simics::Error;

                fn try_from(value: &simics::AttrValue) -> simics::Result<Self> {
                    value.clone().try_into()
                }
            }
        });
    }
}

pub fn from_attr_value_list_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let args = match FromAttrValueListOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(attr_value),
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
        non_exhaustive
    )
)]
struct FromAttrValueDictOpts {
    ident: Ident,
    generics: Generics,
    data: Data<(), FromAttrValueField>,
}

impl ToTokens for FromAttrValueDictOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let Some(fields) = self.data.as_ref().take_struct() else {
            panic!("Fields must be struct fields");
        };

        let value_fields = fields
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
            impl #impl_generics TryFrom<simics::AttrValueType> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::AttrValueType) -> simics::Result<Self> {
                    let simics::AttrValueType::Dict(value) = value else {
                        return Err(simics::Error::FromAttrValueTypeConversionError {
                            ty: std::any::type_name::<#ident #ty_generics>().to_string(),
                            reason: "Expected a dictionary".to_string(),
                        });
                    };

                    Ok(Self {
                        #(#value_fields),*
                    })
                }
            }

            impl #impl_generics TryFrom<simics::AttrValue> for #ident #ty_generics  #where_clause {
                type Error = simics::Error;

                fn try_from(value: simics::AttrValue) -> simics::Result<Self> {
                    // NOTE: We cannot use AttrValueType here, because we are most likely
                    // converting from a non-homogeneous list.
                    let value: std::collections::BTreeMap<simics::AttrValueType, simics::AttrValueType> = value
                        .as_heterogeneous_dict()?
                        .ok_or_else(|| simics::Error::AttrValueType {
                            actual: value.kind(),
                            expected: simics::AttrKind::Sim_Val_Dict,
                            reason: "Expected a dictionary of heterogeneous values".to_string(),
                    })?;

                    Ok(Self {
                        #(#value_fields),*
                    })
                }
            }
        });
    }
}

pub fn from_attr_value_dict_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let args = match FromAttrValueDictOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return e.write_errors().into(),
    };

    quote!(#args).into()
}
