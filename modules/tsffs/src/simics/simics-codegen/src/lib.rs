use std::{
    collections::HashMap,
    env::var,
    fs::read_dir,
    path::{Path, PathBuf},
};

use darling::Error;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_file, Expr, Field, GenericArgument, Item, ItemConst, ItemType, Lit, Meta, PathArguments,
    ReturnType, Type,
};

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
                            let first_input = inputs.iter().next();
                            let has_obj = if let Some(first) = first_input {
                                quote!(#first).to_string().ends_with("conf_object_t")
                            } else {
                                false
                            };
                            let input_names = inputs
                                .iter()
                                .skip(if has_obj { 1 } else { 0 })
                                .filter_map(|a| a.name.clone().map(|n| n.0))
                                .collect::<Vec<_>>();
                            let wrapper_inputs = inputs
                                .iter()
                                .skip(if has_obj { 1 } else { 0 })
                                .collect::<Vec<_>>();
                            let (is_attr_value, output) = match &proto.output {
                                ReturnType::Default => (false, quote!(())),
                                ReturnType::Type(_, t) => match &**t {
                                    Type::Path(p) => {
                                        if let Some(last) = p.path.get_ident() {
                                            if last == "attr_value_t" {
                                                (true, quote!(crate::api::AttrValue))
                                            } else {
                                                (false, quote!(#t))
                                            }
                                        } else {
                                            (false, quote!(#t))
                                        }
                                    }
                                    _ => (false, quote!(#t)),
                                },
                            };
                            // NOTE: We need to make a new name because in some cases the fn ptr name is the same as one of the parameter
                            // names
                            let some_name = format_ident!("{}_fn", name);
                            let maybe_self_obj =
                                has_obj.then_some(quote!(self.obj,)).unwrap_or_default();

                            let ok_value = if is_attr_value {
                                quote!(Ok(unsafe { #some_name(#maybe_self_obj #(#input_names),*) }.into()))
                            } else {
                                quote!(Ok(unsafe { #some_name(#maybe_self_obj #(#input_names),*) }))
                            };

                            return Some(quote! {
                                #vis fn #name(&mut self, #(#wrapper_inputs),*) -> crate::Result<#output> {
                                    if let Some(#some_name) = unsafe { *self.interface}.#name {
                                        #ok_value
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

fn hap_name_and_type_to_struct(
    name_callback_type: (&&&ItemConst, &&ItemType),
) -> Option<TokenStream2> {
    let name = name_callback_type.0;
    let name_name = &name.ident;
    let callback_type = name_callback_type.1;
    let callback_doc = &callback_type.attrs;
    let supports_index_callbacks = callback_type.attrs.iter().find_map(|a| {
        if let Meta::NameValue(ref meta) = a.meta {
            if let Expr::Lit(ref lit) = meta.value {
                if let Lit::Str(ref str_lit) = lit.lit {
                    if !str_lit.value().contains("Index: Indices not supported") {
                        Some(str_lit.value())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
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
                            let callback_ty =
                                quote!(FnMut(#(#closure_params),*) -> #output + 'static);

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
                                    F: #callback_ty,
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
                                    F: #callback_ty
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

                            let maybe_index_callback_methods = supports_index_callbacks.map(|index| {
                                let index_doc = format!("* `index` - The index value for this HAP: {}", index);
                                let range_start_doc = format!("* `start` - The start of the range of index values for this HAP: {}", index);
                                let range_end_doc = format!("* `end` - The start of the range of index values for this HAP: {}", index);
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
                                    F: #callback_ty
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
                                    F: #callback_ty
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
                                    F: #callback_ty
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
                                    F: #callback_ty
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


                            }}).unwrap_or_default();

                            let struct_and_impl = quote! {
                                #(#callback_doc)*
                                pub struct #struct_name {}

                                impl crate::api::traits::hap::Hap for #struct_name {
                                    type Name =  &'static [u8];
                                    const NAME: Self::Name = crate::api::sys::#name_name;
                                }

                                impl #struct_name {
                                    #add_callback_methods
                                    #maybe_index_callback_methods
                                }

                                /// The handler for HAPs of a specific type. Unboxes a boxed
                                /// closure and calls it with the correct HAP callback arguments
                                extern "C" fn #handler_name<F>(#inputs) -> #output
                                    where F: #callback_ty
                                {
                                    // NOTE: This box must be leaked, because we may call this closure again, we cannot drop it
                                    let closure = Box::leak(unsafe { Box::from_raw(#userdata_name as *mut Box<F>) });
                                    closure(#(#closure_param_names),*)
                                }

                            };

                            return Some(struct_and_impl);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Automatically generate high level bindings to all interfaces provided by SIMICS
///
/// Interfaces are defined by the sys bindings as (for example):
///
/// ```rust,ignore
/// #[repr(C)]
/// pub struct breakpoint_interface {
///     pub insert_breakpoint: Option<unsafe extern "C" fn(object: *mut conf_object_t, caller: *mut conf_object_t, handle: breakpoint_handle_t, access: access_t, start: generic_address_t, end: generic_address_t)>,
///     pub remove_breakpoint: Option<unsafe extern "C" fn(object: *mut conf_object_t, handle: breakpoint_handle_t)>,
///     pub get_breakpoint: Option<unsafe extern "C" fn(obj: *mut conf_object_t, handle: breakpoint_handle_t) -> breakpoint_info_t>,
/// }
/// ```
///
/// Along with the name of the interface:
///
/// ```rust,ignore
/// pub const BREAKPOINT_INTERFACE: &[u8; 11] = b"breakpoint\0";
/// ```
///
/// Code-generation takes each interface structure and name and creates a Rust-named
/// structure, implements the [`Interface`] trait for it, and implements a safe(ish)
/// wrapper for the interface object. For the above example, the generation would be:
///
/// ```rust,ignore
/// pub struct BreakpointInterface {
///     interface: *mut crate::api::sys::breakpoint_interface,
/// }
/// impl BreakpointInterface {
///     pub fn insert_breakpoint(
///         &mut self,
///         object: *mut conf_object_t,
///         caller: *mut conf_object_t,
///         handle: breakpoint_handle_t,
///         access: access_t,
///         start: generic_address_t,
///         end: generic_address_t,
///     ) -> crate::Result<()> {
///         if let Some(insert_breakpoint_fn) = unsafe { *self.interface }
///             .insert_breakpoint
///         {
///             Ok(unsafe {
///                 insert_breakpoint_fn(
///                     object,
///                     caller,
///                     handle,
///                     access,
///                     start,
///                     end,
///                 )
///             })
///         } else {
///             Err(crate::Error::NoInterfaceMethod {
///                 method: "insert_breakpoint".to_string(),
///             })
///         }
///     }
///     pub fn remove_breakpoint(
///         &mut self,
///         object: *mut conf_object_t,
///         handle: breakpoint_handle_t,
///     ) -> crate::Result<()> {
///         if let Some(remove_breakpoint_fn) = unsafe { *self.interface }
///             .remove_breakpoint
///         {
///             Ok(unsafe { remove_breakpoint_fn(object, handle) })
///         } else {
///             Err(crate::Error::NoInterfaceMethod {
///                 method: "remove_breakpoint".to_string(),
///             })
///         }
///     }
///     pub fn get_breakpoint(
///         &mut self,
///         obj: *mut conf_object_t,
///         handle: breakpoint_handle_t,
///     ) -> crate::Result<breakpoint_info_t> {
///         if let Some(get_breakpoint_fn) = unsafe { *self.interface }
///             .get_breakpoint
///         {
///             Ok(unsafe { get_breakpoint_fn(obj, handle) })
///         } else {
///             Err(crate::Error::NoInterfaceMethod {
///                 method: "get_breakpoint".to_string(),
///             })
///         }
///     }
/// }
/// impl crate::api::traits::interface::Interface for BreakpointInterface {
///     type InternalInterface = crate::api::sys::breakpoint_interface;
///     type Name = &'static [u8];
///     const NAME: &'static [u8] = crate::api::sys::BREAKPOINT_INTERFACE;
///     fn new(interface: *mut Self::InternalInterface) -> Self {
///         Self { interface }
///     }
///     fn register(cls: *mut crate::api::ConfClass) -> crate::Result<()> {
///         crate::api::base::conf_object::register_interface::<Self>(cls)?;
///         Ok(())
///     }
///     fn get(obj: *mut crate::api::ConfObject) -> crate::Result<Self> {
///         crate::api::base::conf_object::get_interface::<Self>(obj)
///     }
/// }
/// ```
///
pub fn simics_interface_codegen(bindings: &str) -> TokenStream2 {
    let parsed_bindings = match parse_file(bindings) {
        Ok(b) => b,
        Err(e) => return Error::from(e).write_errors(),
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
                    obj: *mut crate::api::ConfObject,
                    interface: *mut crate::api::sys::#interface_ident,
                }

                impl #struct_name {
                    #(#interface_methods)*
                }

                impl crate::api::traits::interface::Interface for #struct_name {
                    type InternalInterface = crate::api::sys::#interface_ident;
                    type Name = &'static [u8];

                    const NAME: &'static [u8] = crate::api::sys::#name_ident;

                    fn new(obj: *mut crate::api::ConfObject, interface: *mut Self::InternalInterface) -> Self {
                        Self { obj, interface }
                    }

                    fn register(cls: *mut crate::api::ConfClass) -> crate::Result<()> {
                        crate::api::base::conf_object::register_interface::<Self>(cls)?;
                        Ok(())
                    }

                    fn get(obj: *mut crate::api::ConfObject) -> crate::Result<Self> {
                        crate::api::base::conf_object::get_interface::<Self>(obj)
                    }
                }
            };
            q
        })
        .collect::<Vec<_>>();

    quote! {
        #[allow(dead_code, non_snake_case)]
        pub mod interfaces {
            use crate::api::sys::*;

            #(#interface_structs)*
        }
    }
}

/// Automatically generate high level bindings to all HAPs provided by SIMICS.
///
/// HAPs are defined as a string name like:
///
/// ```rust,ignore
/// pub const CORE_EXCEPTION_HAP_NAME: &[u8; 15] = b"Core_Exception\0";
/// ```
///
/// This macro generates a struct with an implementation of [`Hap`]. If the hap is
/// marked as having an index pseudo-parameter, methods to add callbacks receiving an
/// index will be generated. Methods to add callbacks not receiving an index are
/// generated for all haps.
///
/// ```rust,ignore
/// pub struct CoreExceptionHap {}
/// impl crate::api::traits::hap::Hap for CoreExceptionHap {
///     type Handler = unsafe extern "C" fn(
///         callback_data: *mut lang_void,
///         trigger_obj: *mut conf_object_t,
///         exception_number: int64,
///     );
///     type Name = &'static [u8];
///     type Callback = Box<
///         dyn Fn(*mut conf_object_t, int64) -> () + 'static,
///     >;
///     const NAME: Self::Name = crate::api::sys::CORE_EXCEPTION_HAP_NAME;
///     const HANDLER: Self::Handler = handle_core_exception::<
///         Self::Callback,
///     >;
///     fn add_callback(
///         callback: Self::Callback,
///     ) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle> {
///         let callback_box = Box::new(callback);
///         let callback_raw = Box::into_raw(callback_box);
///         let handler: unsafe extern "C" fn() = unsafe {
///             std::mem::transmute(Self::HANDLER)
///         };
///         Ok(unsafe {
///             crate::api::sys::SIM_hap_add_callback(
///                 Self::NAME.as_raw_cstr()?,
///                 Some(handler),
///                 callback_raw as *mut std::ffi::c_void,
///             )
///         })
///     }
///     fn add_callback_object(
///         callback: Self::Callback,
///         obj: *mut crate::api::ConfObject,
///     ) -> crate::Result<crate::api::simulator::hap_consumer::HapHandle> {
///         let callback_box = Box::new(callback);
///         let callback_raw = Box::into_raw(callback_box);
///         let handler: unsafe extern "C" fn() = unsafe {
///             std::mem::transmute(Self::HANDLER)
///         };
///         Ok(unsafe {
///             crate::api::sys::SIM_hap_add_callback_obj(
///                 Self::NAME.as_raw_cstr()?,
///                 obj,
///                 0,
///                 Some(handler),
///                 callback_raw as *mut std::ffi::c_void,
///             )
///         })
///     }
/// }
/// extern "C" fn handle_core_exception<F>(
///     callback_data: *mut lang_void,
///     trigger_obj: *mut conf_object_t,
///     exception_number: int64,
/// ) -> ()
/// where
///     F: Fn(*mut conf_object_t, int64) -> () + 'static,
/// {
///     let closure: Box<Box<F>> = unsafe {
///         Box::from_raw(callback_data as *mut Box<F>)
///     };
///     closure(trigger_obj, exception_number)
/// }
/// ```
pub fn simics_hap_codegen(bindings: &str) -> TokenStream2 {
    let parsed_bindings = match parse_file(bindings) {
        Ok(b) => b,
        Err(e) => return Error::from(e).write_errors(),
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
        #[allow(dead_code, non_snake_case)]
        pub mod haps {
            use crate::api::sys::*;
            use crate::api::traits::hap::Hap;
            use raw_cstr::AsRawCstr;

            #(#hap_structs)*
        }
    }
}

/// Generate separate test functions for all SIMICS test python scripts (scripts beginning with
/// 's-' inside a directory that contains a SUITEINFO file.
///
/// The path `package_root` should be a relative path from the test crate root to the package
/// root directory. Tests whose filenames end in "-fail" will expect a failure, not a success.
pub fn simics_tests<P>(package_root: P) -> TokenStream2
where
    P: AsRef<Path>,
{
    let crate_directory_path = PathBuf::from(
        var("CARGO_MANIFEST_DIR").expect("No CARGO_MANIFEST_DIR set. This should be impossible."),
    );

    let test_runner_path = crate_directory_path
        .join(package_root.as_ref())
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
            let try_or_invert_try_and_print = if test_name.ends_with("-fail") {
                quote! {
                    .err()
                    .ok_or_else(|| anyhow::anyhow!("Expected failure, got success"))?;

                    if let command_ext::CommandExtError::Check { code: _, stdout, stderr: _ } = res {
                       println!("stdout:\n{stdout}");
                    }
                }
            } else {
                quote! {
                    ?;

                    let stdout = String::from_utf8_lossy(&res.stdout);

                    println!("stdout:\n{stdout}");
                }
            };
            quote! {
                #[test]
                fn #test_name_ident() -> anyhow::Result<()> {
                    #[allow(unused)]
                    let res = std::process::Command::new(#test_runner_path)
                        .arg("-v")
                        .arg("-n")
                        .arg(#test_name)
                        .check()
                        #try_or_invert_try_and_print

                    Ok(())
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #[allow(unused_imports)]
        use command_ext::CommandExtCheck;

        #(#tests)*
    }
}
