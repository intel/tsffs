// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use prettyplease::unparse;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use simics_build_utils::{emit_cfg_directives, emit_link_info};
use std::{collections::HashMap, env::var, fs::write, path::PathBuf};
use syn::{
    parse_file, parse_quote, punctuated::Punctuated, token::Plus, Attribute, BareFnArg, Expr,
    Field, GenericArgument, Ident, Item, ItemConst, ItemStruct, ItemType, Lit, Meta, PathArguments,
    ReturnType, Type, TypeParamBound, Visibility,
};

/// The name of the environment variable set by cargo containing the path to the out directory
/// for intermediate build results
const OUT_DIR_ENV: &str = "OUT_DIR";
/// The name of the environment variable containing the path to the simics base directory e.g.
/// `/path/to/simics/simics-7.0.1`
const SIMICS_BASE_ENV: &str = "SIMICS_BASE";

use simics_api_sys::SIMICS_API_BINDINGS;

const INTERFACES_FILE: &str = "interfaces.rs";
const HAPS_FILE: &str = "haps.rs";

/// Extension trait to convert snake_case to CamelCase.
trait SnakeToCamel {
    fn snake_to_camel(&self) -> String;
}

impl<S> SnakeToCamel for S
where
    S: AsRef<str>,
{
    fn snake_to_camel(&self) -> String {
        let mut s = String::new();
        let mut upper = false;
        for c in self.as_ref().chars() {
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

struct HapStruct {
    name: Ident,
    callback_ty: Vec<TypeParamBound>,
    handler_name: Ident,
    supports_index_callbacks: Option<String>,
    callback_attrs: Vec<Attribute>,
    struct_name: Ident,
    closure_param_names: Vec<Ident>,
    inputs: Vec<BareFnArg>,
    output: Type,
    userdata_name: Ident,
}

impl TryFrom<(&ItemConst, &ItemType)> for HapStruct {
    type Error = darling::Error;

    fn try_from(name_ty: (&ItemConst, &ItemType)) -> darling::Result<Self> {
        let name = name_ty.0.ident.clone();
        let callback_type = name_ty.1;
        let callback_attrs = callback_type.attrs.to_vec();

        let supports_index_callbacks = callback_attrs.iter().find_map(|a| {
            let Meta::NameValue(ref meta) = a.meta else {
                return None;
            };

            let Expr::Lit(ref lit) = meta.value else {
                return None;
            };

            let Lit::Str(ref str_lit) = lit.lit else {
                return None;
            };

            if !str_lit.value().contains("Index: Indices not supported") {
                Some(str_lit.value())
            } else {
                None
            }
        });

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

        let Type::Path(ref p) = &*callback_type.ty else {
            return Err(Self::Error::custom(format!(
                "Failed to parse callback type {:?} as path",
                callback_type
            )));
        };

        let last = p.path.segments.last().ok_or_else(|| {
            Self::Error::custom(format!(
                "Failed to get final segment from path type {:?}",
                callback_type
            ))
        })?;

        if last.ident != "Option" {
            return Err(Self::Error::custom(format!(
                "Callback type must be Option to support null-optimization, got {:?}",
                callback_type
            )));
        }

        let PathArguments::AngleBracketed(ref args) = last.arguments else {
            return Err(Self::Error::custom(format!(
                "Failed to get angle bracketed arguments from path type {:?}",
                callback_type
            )));
        };

        let Some(GenericArgument::Type(Type::BareFn(proto))) = args.args.first() else {
            return Err(Self::Error::custom(format!(
                "Failed to get bare function type from path type {:?}",
                callback_type
            )));
        };

        // NOTE: We `use crate::api::sys::*;` at the top of the module, otherwise
        // we would need to rewrite all of the types on `inputs` here.
        let inputs = proto.inputs.iter().cloned().collect::<Vec<_>>();

        let input_names = inputs
            .iter()
            .map(|a| {
                a.name.clone().map(|n| n.0).ok_or_else(|| {
                    Self::Error::custom(format!("Failed to get name from input argument {:?}", a))
                })
            })
            .collect::<darling::Result<Vec<_>>>()?;

        let userdata_name = input_names
            .first()
            .ok_or_else(|| {
                Self::Error::custom(format!(
                    "Failed to get userdata name from input arguments {:?}",
                    inputs
                ))
            })?
            .clone();

        let output = match &proto.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, t) => parse_quote!(#t),
        };

        let closure_params = inputs
            .iter()
            .skip(1)
            .cloned()
            .map(|a| a.ty)
            .collect::<Vec<_>>();
        let closure_param_names = input_names.iter().skip(1).cloned().collect::<Vec<_>>();
        let callback_ty: Punctuated<TypeParamBound, Plus> =
            parse_quote!(FnMut(#(#closure_params),*) -> #output + 'static);
        let callback_ty = callback_ty.iter().cloned().collect::<Vec<_>>();

        Ok(Self {
            name,
            callback_ty,
            handler_name,
            supports_index_callbacks,
            callback_attrs,
            struct_name,
            closure_param_names,
            inputs,
            output,
            userdata_name,
        })
    }
}

impl ToTokens for HapStruct {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let callback_ty = &self.callback_ty;
        let handler_name = &self.handler_name;
        let add_callback_methods = quote! {
            /// Add a callback to be called on each occurrence of this HAP. The callback may capture its environment.
            ///
            /// # Arguments
            ///
            /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
            ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
            ///   the soundness of their callback code.
            pub fn add_callback<F>(callback: F) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
            where
                F: #(#callback_ty)+*,
            {
                let callback = Box::new(callback);
                let callback_box = Box::new(callback);
                let callback_raw = Box::into_raw(callback_box);
                let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                Ok(unsafe {
                    crate::api::sys::SIM_hap_add_callback(
                        Self::NAME.as_raw_cstr()?,
                        Some(handler),
                        callback_raw as *mut std::ffi::c_void,
                    )
                })
            }

            /// Add a callback to be called on each occurrence of this HAP for a specific object. The callback may capture its environment.
            ///
            /// # Arguments
            ///
            /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
            ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
            ///   the soundness of their callback code.
            /// * `obj` - The object to fire this callback for. This HAP will not trigger the callback when firing on any object other than
            ///   this one.
            pub fn add_callback_object<F>(callback: F, obj: *mut crate::api::ConfObject) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
            where
                F: #(#callback_ty)+*,
            {
                let callback = Box::new(callback);
                let callback_box = Box::new(callback);
                let callback_raw = Box::into_raw(callback_box);
                let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                Ok(unsafe {
                    crate::api::sys::SIM_hap_add_callback_obj(
                        Self::NAME.as_raw_cstr()?,
                        obj,
                        0,
                        Some(handler),
                        callback_raw as *mut std::ffi::c_void,
                    )
                })
            }
        };

        let maybe_index_callback_methods = if let Some(ref index) = self.supports_index_callbacks {
            let index_doc = format!("* `index` - The index value for this HAP: {}", index);
            let range_start_doc = format!(
                "* `start` - The start of the range of index values for this HAP: {}",
                index
            );
            let range_end_doc = format!(
                "* `end` - The start of the range of index values for this HAP: {}",
                index
            );

            quote! {
                /// Add a callback to be called on each occurrence of this HAP for a specific index value. The callback may capture its environment.
                ///
                /// Only HAPs which support an index may add a callback in this manner, and the index varies for each HAP. For example, the
                /// [`CoreMagicInstructionHap`] supports an index equal to the magic value.
                ///
                /// # Arguments
                ///
                /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
                ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
                ///   the soundness of their callback code.
                #[doc = #index_doc]
                pub fn add_callback_index<F>(callback: F, index: i64) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
                where
                    F: #(#callback_ty)+*,
                {
                    let callback = Box::new(callback);
                    let callback_box = Box::new(callback);
                    let callback_raw = Box::into_raw(callback_box);
                    let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                    Ok(unsafe {
                        crate::api::sys::SIM_hap_add_callback_index(
                            Self::NAME.as_raw_cstr()?,
                            Some(handler),
                            callback_raw as *mut std::ffi::c_void,
                            index
                        )
                    })
                }

                /// Add a callback to be called on each occurrence of this HAP for a specific index value range. The callback may capture its environment.
                ///
                /// Only HAPs which support an index may add a callback in this manner, and the index varies for each HAP. For example, the
                /// [`CoreMagicInstructionHap`] supports an index equal to the magic value.
                ///
                /// # Arguments
                ///
                /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
                ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
                ///   the soundness of their callback code.
                #[doc = #range_start_doc]
                #[doc = #range_end_doc]
                pub fn add_callback_range<F>(callback: F, start: i64, end: i64) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
                where
                    F: #(#callback_ty)+*,
                {
                    let callback = Box::new(callback);
                    let callback_box = Box::new(callback);
                    let callback_raw = Box::into_raw(callback_box);
                    let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                    Ok(unsafe {
                        crate::api::sys::SIM_hap_add_callback_range(
                            Self::NAME.as_raw_cstr()?,
                            Some(handler),
                            callback_raw as *mut std::ffi::c_void,
                            start,
                            end,
                        )
                    })
                }

                /// Add a callback to be called on each occurrence of this HAP on a specific object for a specific index value. The callback may capture its environment.
                ///
                /// Only HAPs which support an index may add a callback in this manner, and the index varies for each HAP. For example, the
                /// [`CoreMagicInstructionHap`] supports an index equal to the magic value.
                ///
                /// # Arguments
                ///
                /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
                ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
                ///   the soundness of their callback code.
                /// * `obj` - The object to fire this callback for. This HAP will not trigger the callback when firing on any object other than
                ///   this one.
                #[doc = #index_doc]
                pub fn add_callback_object_index<F>(callback: F, obj: *mut crate::api::ConfObject, index: i64) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
                where
                    F: #(#callback_ty)+*,
                {
                    let callback = Box::new(callback);
                    let callback_box = Box::new(callback);
                    let callback_raw = Box::into_raw(callback_box);
                    let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                    Ok(unsafe {
                        crate::api::sys::SIM_hap_add_callback_obj_index(
                            Self::NAME.as_raw_cstr()?,
                            obj,
                            0,
                            Some(handler),
                            callback_raw as *mut std::ffi::c_void,
                            index
                        )
                    })
                }

                /// Add a callback to be called on each occurrence of this HAP on a specific object for a specific index value range. The callback may capture its environment.
                ///
                /// Only HAPs which support an index may add a callback in this manner, and the index varies for each HAP. For example, the
                /// [`CoreMagicInstructionHap`] supports an index equal to the magic value.
                ///
                /// # Arguments
                ///
                /// * `callback` - The closure to fire as a callback. The closure will be doubly boxed. Any program state accessed inside
                ///   the closure must have the static lifetime. This is not enforced by the compiler, it is up to the programmer to ensure
                ///   the soundness of their callback code.
                /// * `obj` - The object to fire this callback for. This HAP will not trigger the callback when firing on any object other than
                ///   this one.
                #[doc = #range_start_doc]
                #[doc = #range_end_doc]
                pub fn add_callback_object_range<F>(callback: F, obj: *mut crate::api::ConfObject, start: i64, end: i64) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle>
                where
                    F: #(#callback_ty)+*,
                {
                    let callback = Box::new(callback);
                    let callback_box = Box::new(callback);
                    let callback_raw = Box::into_raw(callback_box);
                    let handler: unsafe extern "C" fn() = unsafe { std::mem::transmute(#handler_name::<F> as usize) };
                    Ok(unsafe {
                        crate::api::sys::SIM_hap_add_callback_obj_range(
                            Self::NAME.as_raw_cstr()?,
                            obj,
                            0,
                            Some(handler),
                            callback_raw as *mut std::ffi::c_void,
                            start,
                            end,
                        )
                    })
                }
            }
        } else {
            quote! {}
        };

        let name = &self.name;
        let callback_attrs = &self.callback_attrs;
        let struct_name = &self.struct_name;
        let closure_param_names = &self.closure_param_names;
        let inputs = &self.inputs;
        let output = &self.output;
        let userdata_name = &self.userdata_name;

        tokens.extend(quote! {
            #(#callback_attrs)*
            /// Automatically generated struct for the HAP
            pub struct #struct_name {}

            impl crate::api::traits::hap::Hap for #struct_name {
                type Name =  &'static [u8];
                const NAME: Self::Name = crate::api::sys::#name;
            }

            impl #struct_name {
                #add_callback_methods
                #maybe_index_callback_methods
            }

            /// The handler for HAPs of a specific type. Unboxes a boxed
            /// closure and calls it with the correct HAP callback arguments
            extern "C" fn #handler_name<F>(#(#inputs),*) -> #output
            where
                F: #(#callback_ty)+*,
            {
                // NOTE: This box must be leaked, because we may call this closure again, we cannot drop it
                let closure = Box::leak(unsafe { Box::from_raw(#userdata_name as *mut Box<F>) });
                closure(#(#closure_param_names),*)
            }
        });
    }
}

struct Haps {
    hap_structs: Vec<HapStruct>,
}

impl Haps {
    fn generate<S>(bindings: S) -> darling::Result<Self>
    where
        S: AsRef<str>,
    {
        let parsed = parse_file(bindings.as_ref())?;
        let hap_name_items = parsed
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Const(c) if c.ident.to_string().ends_with("_HAP_NAME") => {
                    Some((c.ident.to_string(), c))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        let hap_callbacks = parsed
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Type(ty) if ty.ident.to_string().ends_with("_hap_callback") => Some(
                    hap_name_items
                        .get(
                            &(ty.ident
                                .to_string()
                                .trim_end_matches("_hap_callback")
                                .to_ascii_uppercase()
                                + "_HAP_NAME"),
                        )
                        .map(|hap_name_item| ((*hap_name_item).clone(), ty.clone()))
                        .ok_or_else(|| {
                            darling::Error::custom(format!("Failed to find HAP name for {:?}", ty))
                        }),
                ),
                _ => None,
            })
            .collect::<darling::Result<HashMap<_, _>>>()?;
        let hap_structs = hap_callbacks
            .iter()
            .map(HapStruct::try_from)
            .collect::<darling::Result<Vec<_>>>()?;

        Ok(Self { hap_structs })
    }
}

impl ToTokens for Haps {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let hap_structs = &self.hap_structs;
        tokens.extend(quote! {
            #[allow(dead_code, non_snake_case)]
            /// Automatically generated HAP implementations
            pub mod haps {
                use crate::api::sys::*;
                use crate::api::traits::hap::Hap;
                use raw_cstr::AsRawCstr;

                #(#hap_structs)*
            }
        });
    }
}

pub struct InterfaceMethod {
    vis: Visibility,
    wrapper_inputs: Vec<BareFnArg>,
    output: Type,
    some_name: Ident,
    name_ident: Ident,
    ok_value: Expr,
    name: Ident,
}

impl TryFrom<&Field> for InterfaceMethod {
    type Error = darling::Error;

    fn try_from(value: &Field) -> darling::Result<Self> {
        let vis = value.vis.clone();

        let Type::Path(ref p) = value.ty else {
            return Err(darling::Error::custom(format!(
                "Expected a path type for field, got {:?}",
                value.ty
            )));
        };

        let last = p.path.segments.last().ok_or_else(|| {
            darling::Error::custom(format!(
                "Missing final segment for path type {:?}",
                value.ty
            ))
        })?;

        if last.ident != "Option" {
            return Err(darling::Error::custom(format!(
                "Expected Option type for field, got {:?}",
                value.ty
            )));
        }

        let name = (value.ident.clone())
            .ok_or_else(|| darling::Error::custom("Missing field name".to_string()))?;

        let PathArguments::AngleBracketed(ref args) = last.arguments else {
            return Err(darling::Error::custom(format!(
                "Expected angle bracketed arguments for field, got {:?}",
                last.arguments
            )));
        };

        let Some(GenericArgument::Type(Type::BareFn(proto))) = args.args.first() else {
            return Err(darling::Error::custom(format!(
                "Expected bare function type for field, got {:?}",
                args.args.first()
            )));
        };

        let inputs = proto.inputs.iter().cloned().collect::<Vec<_>>();

        let has_obj = inputs
            .first()
            .is_some_and(|f| quote!(#f).to_string().ends_with("conf_object_t"));

        let input_names = inputs
            .iter()
            .skip(if has_obj { 1 } else { 0 })
            .map(|a| {
                a.name.clone().map(|n| n.0).ok_or_else(|| {
                    darling::Error::custom(format!("Missing input name for {:?}", a))
                })
            })
            .collect::<darling::Result<Vec<_>>>()?;

        let wrapper_inputs = inputs
            .iter()
            .skip(if has_obj { 1 } else { 0 })
            .cloned()
            .collect::<Vec<_>>();

        let (is_attr_value, output) = match &proto.output {
            ReturnType::Default => (false, parse_quote!(())),
            ReturnType::Type(_, t) => {
                if let Type::Path(p) = &**t {
                    p.path
                        .get_ident()
                        .map(|i| i.to_string())
                        .filter(|i| i == "attr_value_t")
                        .map(|_| (true, parse_quote!(crate::api::AttrValue)))
                        .unwrap_or((false, parse_quote!(#t)))
                } else {
                    (false, parse_quote!(#t))
                }
            }
        };

        // NOTE: We need to make a new name because in some cases the fn ptr name is the same as one of the parameter
        // names
        let some_name = format_ident!("{}_fn", name);
        let name_ident = format_ident!("{}", name);
        let maybe_self_obj = has_obj.then_some(quote!(self.obj,)).unwrap_or_default();

        let ok_value = if is_attr_value {
            parse_quote!(Ok(unsafe { #some_name(#maybe_self_obj #(#input_names),*) }.into()))
        } else {
            parse_quote!(Ok(unsafe { #some_name(#maybe_self_obj #(#input_names),*) }))
        };

        Ok(Self {
            vis,
            wrapper_inputs,
            output,
            some_name,
            name_ident,
            ok_value,
            name,
        })
    }
}

impl ToTokens for InterfaceMethod {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let vis = &self.vis;
        let wrapper_inputs = &self.wrapper_inputs;
        let output = &self.output;
        let some_name = &self.some_name;
        let name_ident = &self.name_ident;
        let ok_value = &self.ok_value;
        let name = self.name.to_string();

        tokens.extend(quote! {
            /// Automatically generated method for the interface
            #vis fn #name_ident(&mut self, #(#wrapper_inputs),*) -> crate::Result<#output> {
                if let Some(#some_name) = unsafe { *self.interface}.#name_ident {
                    #ok_value
                } else {
                    Err(crate::Error::NoInterfaceMethod { method: #name.to_string() })
                }
            }
        });
    }
}

pub struct InterfaceStruct {
    struct_name: Ident,
    interface_ident: Ident,
    interface_methods: Vec<InterfaceMethod>,
    name_ident: Ident,
}

impl TryFrom<(&ItemConst, &ItemStruct)> for InterfaceStruct {
    type Error = darling::Error;

    fn try_from(value: (&ItemConst, &ItemStruct)) -> darling::Result<Self> {
        let name = value.0;
        let interface = value.1;
        let interface_methods = interface
            .fields
            .iter()
            .filter_map(|f| match InterfaceMethod::try_from(f) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("cargo:warning=Failed to generate method for field {f:?}: {e}");
                    None
                }
            })
            .collect::<Vec<_>>();
        let camel_name = name.ident.to_string().snake_to_camel();
        let struct_name = format_ident!("{camel_name}");
        let interface_ident = interface.ident.clone();
        let name_ident = name.ident.clone();
        Ok(Self {
            struct_name,
            interface_ident,
            interface_methods,
            name_ident,
        })
    }
}

impl ToTokens for InterfaceStruct {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let struct_name = &self.struct_name;
        let interface_ident = &self.interface_ident;
        let interface_methods = &self.interface_methods;
        let name_ident = &self.name_ident;

        tokens.extend(quote! {
            /// Automatically generated structure for the interface
            pub struct #struct_name {
                obj: *mut crate::api::ConfObject,
                interface: *mut crate::api::sys::#interface_ident,
            }

            impl #struct_name {
                #(#interface_methods)*
            }

            impl crate::api::traits::Interface for #struct_name {
                type InternalInterface = crate::api::sys::#interface_ident;
                type Name = &'static [u8];

                const NAME: &'static [u8] = crate::api::sys::#name_ident;

                fn new(obj: *mut crate::api::ConfObject, interface: *mut Self::InternalInterface) -> Self {
                    Self { obj, interface }
                }
            }
        });
    }
}

pub struct Interfaces {
    interface_structs: Vec<InterfaceStruct>,
}

impl Interfaces {
    pub fn generate<S>(bindings: S) -> darling::Result<Self>
    where
        S: AsRef<str>,
    {
        let parsed = parse_file(bindings.as_ref())?;

        let interface_name_items = parsed
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Const(c) if c.ident.to_string().ends_with("_INTERFACE") => {
                    Some((c.ident.to_string(), c))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();

        let interfaces = parsed
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Struct(s) = i {
                    interface_name_items
                        .get(&s.ident.to_string().to_ascii_uppercase())
                        .map(|interface_name_item| ((*interface_name_item).clone(), s.clone()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let interface_structs = interfaces
            .iter()
            .map(InterfaceStruct::try_from)
            .collect::<darling::Result<Vec<_>>>()?;

        Ok(Self { interface_structs })
    }
}

impl ToTokens for Interfaces {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let interface_structs = &self.interface_structs;
        tokens.extend(quote! {
            #[allow(dead_code, non_snake_case)]
            /// Automatically generated interfaces from the base package
            pub mod interfaces {
                use crate::api::sys::*;

                #(#interface_structs)*
            }
        })
    }
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-env-changed={SIMICS_BASE_ENV}");
    let out_dir = PathBuf::from(
        var(OUT_DIR_ENV)
            .map_err(|e| anyhow!("No environment variable {OUT_DIR_ENV} found: {e}"))?,
    );

    // Write intermediate auto-generated high level bindings for interfaces and haps

    let interfaces_out_file = out_dir.join(INTERFACES_FILE);
    let haps_out_file = out_dir.join(HAPS_FILE);

    let interfaces_tokens = Interfaces::generate(SIMICS_API_BINDINGS)
        .map(|i| i.to_token_stream())
        .unwrap_or_else(|e| e.write_errors());

    let haps_tokens = Haps::generate(SIMICS_API_BINDINGS)
        .map(|h| h.to_token_stream())
        .unwrap_or_else(|e| e.write_errors());

    eprintln!("cargo:warning=Writing interfaces to {interfaces_out_file:?}");

    write(
        interfaces_out_file,
        unparse(&parse_file(&interfaces_tokens.to_string()).map_err(|e| {
            eprintln!("Failed to parse interfaces file: {e}\n{interfaces_tokens}");
            e
        })?),
    )?;

    eprintln!("cargo:warning=Writing haps to {haps_out_file:?}");

    write(
        haps_out_file,
        unparse(&parse_file(&haps_tokens.to_string()).map_err(|e| {
            eprintln!("Failed to parse interfaces file: {e}\n{haps_tokens}");
            e
        })?),
    )?;

    eprintln!("cargo:warning=Emitting cfg directives");
    emit_cfg_directives()?;
    eprintln!("cargo:warning=Emitting link info");
    emit_link_info()?;

    Ok(())
}
