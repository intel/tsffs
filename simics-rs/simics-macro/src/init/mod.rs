// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use darling::{ast::NestedMeta, util::Flag, Error, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use simics_api_sys::{SIM_VERSION, SIM_VERSION_COMPAT};
use syn::{parse_macro_input, ItemFn, ReturnType, Type};

#[derive(Debug, FromMeta)]
pub struct SimicsInitOpts {
    #[darling(default)]
    /// The name of the module, used in the `MOD:NAME` expression
    pub name: String,
    #[darling(multiple)]
    /// The list of classes to register. Classes not listed here will not be declared.
    pub class: Vec<String>,
    no_panic_hook: Flag,
}

pub trait IsResultType {
    fn is_result_type(&self) -> bool;
}

#[cfg(unix)]
const HOST_TYPE: &str = "linux64";

#[cfg(windows)]
const HOST_TYPE: &str = "win64";

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

pub fn generate_exports(opts: &SimicsInitOpts) -> TokenStream2 {
    let name = &opts.name;
    let classes = &opts.class;
    let epoch_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before epoch")
        .as_secs();
    let api = SIM_VERSION.to_string().chars().take(1).collect::<String>();
    let classes = classes
        .iter()
        .map(|c| format!("CLS:{}", c))
        .collect::<Vec<_>>();
    let classes = classes.join(";");

    let capabilities = vec![
        format!("VER:{SIM_VERSION_COMPAT}"),
        format!("ABI:{SIM_VERSION}"),
        format!("API:{api}"),
        "BLD:0".to_string(),
        "BLD_NS:__simics_project__".to_string(),
        format!("BUILDDATE:{epoch_time}"),
        format!("MOD:{name}"),
        classes,
        format!("HOSTTYPE:{HOST_TYPE}"),
        "THREADSAFE".to_string(),
        " ".repeat(43),
    ];
    let module_capabilities_string = capabilities.join(";") + ";" + "\x00";
    let module_capabilities_bytes = module_capabilities_string.as_bytes().to_vec();
    let module_capabilities_len = module_capabilities_bytes.len();

    // Get the date in the format like "Thu Jan 18 15:37:54 2024"
    let module_date_string = Local::now().format("%a %b %d %T %Y\x00").to_string();
    let module_date_bytes = module_date_string.as_bytes().to_vec();
    let module_date_len = module_date_bytes.len();

    quote! {
        #[no_mangle]
        /// The module capabilities list
        pub static _module_capabilities_: [u8; #module_capabilities_len] = [#(#module_capabilities_bytes),*];
        #[no_mangle]
        /// The module build date
        pub static _module_date: [u8; #module_date_len] = [#(#module_date_bytes),*];
    }
}

pub fn simics_init_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let input = parse_macro_input!(input as ItemFn);

    let opts = match SimicsInitOpts::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    // Get the original ident and visibility before we change them
    let inner_ident = &input.sig.ident;

    let maybe_expect = &input
        .sig
        .output
        .is_result_type()
        .then_some(quote!(.expect("Failed while executing init")))
        .unwrap_or(quote!());

    let maybe_ty_generics = (!&input.sig.generics.params.is_empty()).then_some({
        let params = &input.sig.generics.params;
        quote!(::<#params>)
    });

    let Some(args) = &input
        .sig
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

    let exports = generate_exports(&opts);

    let maybe_set_panic_hook = if !opts.no_panic_hook.is_present() {
        quote! {
            std::panic::set_hook(Box::new(|panic_info| {
                simics::panic_handler(panic_info)
            }));
        }
    } else {
        quote!()
    };

    let wrapper = quote! {
        #[no_mangle]
        /// Exported symbol called by simics when module is loaded
        pub extern "C" fn _simics_module_init() {
            #maybe_set_panic_hook
            #inner_ident #maybe_ty_generics(#(#args),*) #maybe_expect;
        }
    };

    quote! {
        #input
        #wrapper
        #exports
    }
    .into()
}
