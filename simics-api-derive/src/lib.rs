use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Parser, Result},
    parse_macro_input,
    punctuated::Punctuated,
    Field, Fields, Ident, ItemStruct, Token,
};

struct Args {
    imperatives: HashSet<Ident>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let imperatives = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;

        Ok(Args {
            imperatives: imperatives.into_iter().collect(),
        })
    }
}

impl Args {
    fn has_derive(&self) -> bool {
        self.imperatives.iter().any(|i| *i == "derive")
    }
}

#[proc_macro_derive(Module)]
/// Derive the default implementation of [Module]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(input as ItemStruct);

    let name = &item_struct.ident;

    let r: TokenStream = quote! {
        impl Module for #name {}
    }
    .into();

    let s = r.to_string();

    eprintln!("{}", s);

    r
}

#[proc_macro_attribute]
/// Attribute to add boilerplate to a `struct` to enable it to be used as a SIMICS Conf Object.
///
/// * Generate default implementations for CFFI to call functions defined in the [Module] trait
///   impl
/// * Insert a [ConfObject] field to permit instances of the struct to be passed via CFFI to and
///   from SIMICS
/// * Optionally, derive the default implementations of the [Module] trait
///
/// The module accepts one argument, `derive` which allows you to derive the default
/// implementation of [Module] alongside automatic implementations of the extern functions
/// required to register the class.
///
/// # Examples
///
/// Without deriving [Module]:
///
/// ```text
/// #[macro_use]
/// extern crate simics_api_derive;
/// use simics_api_derive::module;
///
/// #[module]
/// struct Test {}
/// ```
///
/// Derive [Module]:
///
/// ```text
/// #[macro_use]
/// extern crate simics_api_derive;
/// use simics_api::Module;
///
/// use simics_api_derive::module;
///
/// #[module(derive)]
/// struct Test {}
/// ```
pub fn module(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);

    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let name = &item_struct.ident;

    if let Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.insert(
            0,
            Field::parse_named
                .parse2(quote!(conf_object: simics_api::ConfObject))
                .expect("Couldn't parse field `conf_object`"),
        );
    };

    let alloc_fn_name = format_ident!("{}_alloc", name.to_string().to_ascii_lowercase());
    let init_fn_name = format_ident!("{}_init", name.to_string().to_ascii_lowercase());
    let finalize_fn_name = format_ident!("{}_finalize", name.to_string().to_ascii_lowercase());
    let objects_finalized_fn_name = format_ident!(
        "{}_objects_finalized",
        name.to_string().to_ascii_lowercase()
    );
    let deinit_fn_name = format_ident!("{}_deinit", name.to_string().to_ascii_lowercase());
    let dealloc_fn_name = format_ident!("{}_dealloc", name.to_string().to_ascii_lowercase());

    // Only derive Module if we get a `derive` argument
    let derive_attribute = if args.has_derive() {
        quote! { #[derive(Module)] }
    } else {
        quote! {}
    };

    let r: TokenStream = quote! {
        #derive_attribute
        #item_struct

        #[no_mangle]
        pub extern "C" fn #alloc_fn_name(cls: *mut simics_api::ConfClass) -> *mut simics_api::ConfObject {
            let cls: simics_api::OwnedMutConfClassPtr = cls.into();
            let obj: *mut simics_api::ConfObject  = #name::alloc::<#name>(cls).expect("Unable to allocate #name").into();
            obj
        }

        #[no_mangle]
        pub extern "C" fn #init_fn_name(obj: *mut simics_api::ConfObject) -> *mut std::ffi::c_void {
            Into::<*mut simics_api::ConfObject>::into(#name::init(obj.into())) as *mut std::ffi::c_void
        }

        #[no_mangle]
        pub extern "C" fn #finalize_fn_name(obj: *mut simics_api::ConfObject) {
            #name::finalize(obj.into());
        }

        #[no_mangle]
        pub extern "C" fn #objects_finalized_fn_name(obj: *mut simics_api::ConfObject) {
            #name::objects_finalized(obj.into());
        }

        #[no_mangle]
        pub extern "C" fn #deinit_fn_name(obj: *mut simics_api::ConfObject) {
            #name::deinit(obj.into());
        }

        #[no_mangle]
        pub extern "C" fn #dealloc_fn_name(obj: *mut simics_api::ConfObject) {
            #name::dealloc(obj.into());
        }

    }
    .into();

    let s = r.to_string();

    eprintln!("{}", s);

    r
}
