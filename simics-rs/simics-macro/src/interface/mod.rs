use crate::exception::IsResultType;
use command_ext::CommandExtCheck;
use darling::{ast::NestedMeta, Error, FromMeta, Result};
use ispm_wrapper::ispm::{self, GlobalOptions};
use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use simics_sign::Sign;
use std::{
    collections::hash_map::DefaultHasher,
    env::var,
    fs::{create_dir_all, read_dir, write},
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use syn::{
    parse_macro_input, Expr, FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, Lit,
    Meta, Pat, PathArguments, ReturnType, Type,
};

#[cfg(not(windows))]
/// The name of the binary/library/object subdirectory on linux systems
pub const HOST_DIRNAME: &str = "linux64";
#[cfg(windows)]
/// The name of the binary/library/object subdirectory on windows systems
pub const HOST_DIRNAME: &str = "win64";
/// Name for the environment variable set by the SIMICS build system to the flag to
/// include e.g.  -I SIMICS_BASE/linux64/include/python3.9/
pub const PYTHON3_INCLUDE_ENV: &str = "PYTHON3_INCLUDE";
/// Name for the environment variable set by the SIMICS build system to the libpython3.so library
pub const PYTHON3_LDFLAGS_ENV: &str = "PYTHON3_LDFLAGS";

#[derive(Debug, Clone, FromMeta)]
pub struct InterfaceAttr {
    #[darling(default)]
    name: Option<String>,
}

/// Get the only subdirectory of a directory, if only one exists. If zero or more than one subdirectories
/// exist, returns an error
pub fn subdir<P>(dir: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    let subdirs = read_dir(dir)
        .ok()?
        .filter_map(|p| p.ok())
        .map(|p| p.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();

    if subdirs.len() != 1 {
        return None;
    }

    subdirs.first().cloned()
}

pub trait SnakeToCamel {
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

#[derive(Debug)]
pub struct Interface {
    input: ItemImpl,
    name: String,
}

impl Interface {
    pub fn ident(&self) -> Result<Ident> {
        if let Type::Path(ref p) = *self.input.self_ty {
            let Some(last) = p.path.segments.last() else {
                return Err(Error::custom("expected a type path"));
            };

            Ok(last.ident.clone())
        } else {
            Err(Error::custom("expected a type path"))
        }
    }
}

impl ToTokens for Interface {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let input = &self.input;
        let Ok(ident) = self.ident() else {
            return tokens.extend(Error::custom("expected a type path").write_errors());
        };
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        let name = self.name.clone();
        let interface_ident = format_ident!("{name}");
        let interface_internal_ident = format_ident!("{}InternalInterface", name.snake_to_camel());
        let Ok(interface_name_literal) = format!("c\"{}\"", name).parse::<Literal>() else {
            return tokens.extend(Error::custom("invalid interface name").write_errors());
        };
        let ffi_interface_mod_name = format!("{}_interface_ffi", name);

        let impl_fns = input
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(ref f) = i {
                    let ffi_fn_name = format!("{}_{}", name, f.sig.ident);
                    Some(quote! {
                        #[ffi(arg(self), arg(rest), name = #ffi_fn_name)]
                        #f
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let Some(internal_interface_fields) = input
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(ref f) = i {
                    Some(&f.sig)
                } else {
                    None
                }
            })
            .map(|s| {
                let ident = &s.ident;
                let mut inputs = s
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|i| quote!(#i))
                    .collect::<Vec<_>>();
                inputs.insert(0, quote!(obj: *mut simics::ConfObject));
                let output = match &s.output {
                    ReturnType::Default => quote!(()),
                    ReturnType::Type(_, t) => {
                        if s.output.is_result_type() {
                            let Type::Path(ref path) = &**t else {
                                return None;
                            };
                            let last = path.path.segments.last()?;
                            let PathArguments::AngleBracketed(args) = &last.arguments else {
                                return None;
                            };
                            let first = args.args.first()?;
                            quote!(#first)
                        } else {
                            quote!(#t)
                        }
                    }
                };
                Some(quote! {
                    /// The internal interface field
                    pub #ident: Option<extern "C" fn(#(#inputs),*) -> #output>
                })
            })
            .collect::<Option<Vec<_>>>()
        else {
            return tokens.extend(
                Error::custom(format!(
                    "Invalid return type for interface function: {name}"
                ))
                .write_errors(),
            );
        };

        #[allow(unused)]
        let internal_interface_default_args = input
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(ref f) = i {
                    Some(&f.sig)
                } else {
                    None
                }
            })
            .map(|s| {
                // NOTE: False positive
                let ffi_fn_name = format_ident!("{}_{}", name, s.ident.to_string());
                let name = &s.ident;
                let ffi_interface_mod_name = format_ident!("{ffi_interface_mod_name}");
                quote! {
                    /// The name
                    #name: Some(#ffi_interface_mod_name::#ffi_fn_name)
                }
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            /// The holder for the object the interface is implemented on and the pointer to
            /// the CFFI interface of function pointers
            pub struct #interface_ident {
                obj: *mut simics::ConfObject,
                interface: *mut #interface_internal_ident,
            }

            impl #impl_generics simics::HasInterface<#interface_ident> for #ident #ty_generics #where_clause {}

            impl simics::Interface for #interface_ident {
                type InternalInterface = #interface_internal_ident;
                type Name = &'static std::ffi::CStr;

                const NAME: &'static std::ffi::CStr = #interface_name_literal;

                fn new(obj: *mut simics::ConfObject, interface: *mut Self::InternalInterface) -> Self {
                    Self { obj, interface }
                }
            }

            #[ffi::ffi(expect, mod_name = #ffi_interface_mod_name, self_ty = "*mut simics::ConfObject")]
            impl #impl_generics #ident #ty_generics #where_clause {
                #(#impl_fns)*
            }

            #[derive(Debug)]
            #[repr(C)]
            /// The internal interface for the interface, which contains all the function pointers
            /// called from the simulator
            pub struct #interface_internal_ident {
                #(#internal_interface_fields),*
            }

            impl Default for #interface_internal_ident {
                fn default() -> Self {
                    Self {
                        #(#internal_interface_default_args),*
                    }
                }
            }
        });
    }
}

pub struct CInterface;

impl CInterface {
    fn interface_function_type_to_ctype(ty: &Type) -> Result<String> {
        match &ty {
            Type::Paren(i) => Self::interface_function_type_to_ctype(&i.elem),
            Type::Tuple(t) => {
                if t.elems.is_empty() {
                    Ok("void".to_string())
                } else {
                    Err(Error::custom("Non-empty tuple is not a valid C type"))
                }
            }
            Type::Path(p) => {
                // First, check if the outer is an option. If it is, we just discard it and take the
                // inner type.
                if let Some(last) = p.path.segments.last() {
                    let ty_ident = &last.ident;
                    match &last.arguments {
                        syn::PathArguments::None => {
                            // No angle arguments, we can break down the type now
                            let tystr = ty_ident.to_string();
                            Ok(match tystr.as_str() {
                                "ConfObject" => "conf_object_t",
                                "AttrValue" => "attr_value_t",
                                "BreakpointId" => "breakpoint_id_t",
                                "GenericAddress" => "generic_address_t",
                                "u8" => "uint8",
                                "u16" => "uint16",
                                "u32" => "uint32",
                                "u64" => "uint64",
                                "i8" => "int8",
                                "i16" => "int16",
                                "i32" => "int32",
                                "i64" => "int64",
                                // NOTE: This is not exactly right, but we don't expect anyone to
                                // run simics on a 32-bit host.
                                "f32" => "float",
                                "f64" => "double",
                                "usize" => "size_t",
                                "isize" => "ssize_t",
                                "c_char" => "char",
                                // Attempt to use the type as-is. This is unlikely to work, but allows
                                // creative people to be creative
                                other => other,
                            }
                            .to_string())
                        }
                        syn::PathArguments::AngleBracketed(a) => {
                            // Options and results can be extracted directly
                            if last.ident == "Option" || last.ident == "Result" {
                                if let Some(GenericArgument::Type(ty)) = a.args.first() {
                                    Self::interface_function_type_to_ctype(ty)
                                } else {
                                    Err(Error::custom("Unsupported generic arguments"))
                                }
                            } else {
                                Err(Error::custom(format!(
                                    "Unsupported function type with arguments: {ty_ident}"
                                )))
                            }
                        }
                        _ => Err(Error::custom(
                            "Unsupported interface function type argument",
                        )),
                    }
                } else {
                    Err(Error::custom(
                        "Unexpected empty path in interface function type",
                    ))
                }
            }
            Type::Ptr(p) => {
                let ptr_ty = Self::interface_function_type_to_ctype(&p.elem)?;
                let maybe_const = p
                    .const_token
                    .is_some()
                    .then_some("const ".to_string())
                    .unwrap_or_default();
                Ok(format!("{maybe_const}{ptr_ty} *"))
            }
            _ => Err(Error::custom(format!(
                "Unsupported type for C interface generation: {ty:?}"
            ))),
        }
    }

    fn generate_interface_function_type(item: &ImplItemFn) -> Result<String> {
        // Parse InterfaceAttr from the function's attributes
        let interface_attr = item.attrs.iter().find(|a| {
            if let Meta::Path(ref p) = a.meta {
                p.is_ident("interface")
            } else {
                false
            }
        });

        let interface_attr_opts = if let Some(interface_attr) = interface_attr {
            let meta_list = NestedMeta::parse_meta_list({
                let Meta::Path(ref p) = interface_attr.meta else {
                    return Err(Error::custom("Expected path meta for interface"));
                };
                p.to_token_stream()
            })
            .map_err(|e| Error::custom(format!("Failed to parse interface attribute: {e}")))?;
            Some(InterfaceAttr::from_list(&meta_list)?)
        } else {
            None
        };

        let signature = &item.sig;

        let name = format_ident!(
            "{}",
            interface_attr_opts
                .and_then(|o| o.name)
                .unwrap_or(item.sig.ident.to_string())
        );

        let ty = item
            .sig
            .inputs
            .iter()
            .map(|i| match i {
                FnArg::Receiver(_) => Ok("conf_object_t * obj".to_string()),
                FnArg::Typed(a) => {
                    let ty = Self::interface_function_type_to_ctype(&a.ty)?;
                    let name = match &*a.pat {
                        Pat::Ident(ref p) => Ok(p.ident.to_string()),
                        _ => Err(Error::custom("Expected ident pattern type")),
                    }?;
                    Ok(format!("{} {}", ty, name))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let ty_params = ty.join(", ");

        let output = match &signature.output {
            ReturnType::Default => "void".to_string(),
            ReturnType::Type(_, t) => Self::interface_function_type_to_ctype(t)?,
        };

        Ok(format!("{output} (*{name})({ty_params});"))
    }

    fn generate_interface_header(input: &ItemImpl, interface_name: &String) -> Result<String> {
        let interface_struct_name = format!("{interface_name}_interface");
        let interface_struct_name_define = interface_struct_name.to_ascii_uppercase();
        let include_guard = format!("{}_INTERFACE_H", interface_name.to_ascii_uppercase());
        let interface_functions = input
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(ref f) = i {
                    // If the function has a #[interface(rename = "name")] attribute, use that name
                    // instead of the function's name
                    Some(f)
                } else {
                    None
                }
            })
            .map(Self::generate_interface_function_type)
            .collect::<Result<Vec<_>>>()?;

        let interface_functions_code = interface_functions.join("\n    ");

        Ok(format!(
            r#"
            // Copyright (C) 2024 Intel Corporation
            // SPDX-License-Identifier: Apache-2.0

            #ifndef {include_guard}
            #define {include_guard}

            #include <simics/device-api.h>
            #include <simics/pywrap.h>

            #ifdef __cplusplus
            extern "C" {{
            #endif

            SIM_INTERFACE({interface_name}) {{
                {interface_functions_code}
            }};

            #define {interface_struct_name_define} "{interface_name}"

            #ifdef __cplusplus
            }}
            #endif

            #endif // {include_guard}

        "#
        ))
    }

    fn generate_interface_dml<S>(
        input: &ItemImpl,
        header_name: S,
        interface_name: &String,
    ) -> Result<String>
    where
        S: AsRef<str>,
    {
        let header_name = header_name.as_ref();
        let interface_struct_name = format!("{interface_name}_interface");
        let interface_struct_name_define = interface_struct_name.to_ascii_uppercase();
        let interface_struct_ty_name = format!("{interface_struct_name}_t");
        let interface_functions = input
            .items
            .iter()
            .filter_map(|i| {
                if let ImplItem::Fn(ref f) = i {
                    Some(f)
                } else {
                    None
                }
            })
            .map(Self::generate_interface_function_type)
            .collect::<Result<Vec<_>>>()?;

        let interface_functions_code = interface_functions.join("\n    ");

        Ok(format!(
            r#"
            // Copyright (C) 2024 Intel Corporation
            // SPDX-License-Identifier: Apache-2.0

            dml 1.4;

            header %{{
            #include "{header_name}"
            %}}

            extern typedef struct {{
                {interface_functions_code}
            }} {interface_struct_ty_name};

            extern const char *const {interface_struct_name_define};
        "#
        ))
    }

    fn generate_interface_makefile<S>(header_name: S) -> String
    where
        S: AsRef<str>,
    {
        let header_name = header_name.as_ref();

        format!(
            r#"
            IFACE_FILES = {header_name}
            THREAD_SAFE = yes

            ifeq ($(MODULE_MAKEFILE),)
            $(error Make sure you compile your module from the project directory)
            else
            include $(MODULE_MAKEFILE)
            endif
        "#
        )
    }

    fn generate(input: &ItemImpl, interface_name: &String) -> Result<()> {
        let out_dir = PathBuf::from(var("OUT_DIR").map_err(|e| {
            Error::custom(format!("Failed to get OUT_DIR environment variable: {e}. The OUT_DIR variable is only set for crates with a 'build.rs' build script. Ensure one exists for the crate."))
        })?);

        #[cfg(unix)]
        const OBJ_SUFFIX: &str = "o";

        #[cfg(windows)]
        const OBJ_SUFFIX: &str = "obj";

        #[cfg(unix)]
        const CDYLIB_PREFIX: &str = "lib";

        #[cfg(windows)]
        const CDYLIB_PREFIX: &str = "";

        #[cfg(unix)]
        const CDYLIB_SUFFIX: &str = "so";

        #[cfg(windows)]
        const CDYLIB_SUFFIX: &str = "dll";

        #[cfg(windows)]
        const CSTATICLIB_SUFFIX: &str = "lib";

        let header_name = format!("{interface_name}-interface.h");
        let dml_name = format!("{interface_name}-interface.dml");
        let makefile_name = "Makefile";
        let pyifaces_interface_c = format!("pyifaces-{interface_name}-interface.c");
        let pyifaces_interface_d = format!("pyifaces-{interface_name}-interface.d");
        let pyifaces_interface_i = format!("pyifaces-{interface_name}-interface.i");
        let pyifaces_interface_t = format!("pyifaces-{interface_name}-interface.t");
        let pyiface_interface = format!("pyiface-{interface_name}-interface");
        let pyiface_interface_trampolines_c =
            format!("pyiface-{interface_name}-interface-trampolines.c");
        let pyiface_interface_trampolines_o =
            format!("pyiface-{interface_name}-interface-trampolines.{OBJ_SUFFIX}");
        let pyiface_interface_trampoline_data_h =
            format!("pyiface-{interface_name}-interface-trampoline-data.h");
        let pyiface_interface_wrappers_c = format!("pyiface-{interface_name}-interface-wrappers.c");
        let pyiface_interface_wrappers_d = format!("pyiface-{interface_name}-interface-wrappers.d");
        let pyiface_interface_wrappers_o =
            format!("pyiface-{interface_name}-interface-wrappers.{OBJ_SUFFIX}");
        let pyiface_interface_interface_list =
            format!("pyiface-{interface_name}-interface-interface-list");
        let module_id_c = format!("{interface_name}_module_id.c");
        let module_id_d = format!("{interface_name}_module_id.d");
        let module_id_o = format!("{interface_name}_module_id.{OBJ_SUFFIX}");
        let interface_so = format!("{CDYLIB_PREFIX}{interface_name}-interface.{CDYLIB_SUFFIX}");

        let header = Self::generate_interface_header(input, interface_name)?;
        let dml = Self::generate_interface_dml(input, &header_name, interface_name)?;
        let makefile = Self::generate_interface_makefile(&header_name);

        // Hash together the header and dml to get a unique subdir name
        #[cfg(debug_assertions)]
        let profile = "dev";
        #[cfg(not(debug_assertions))]
        let profile = "release";
        let mut hasher = DefaultHasher::new();
        header.hash(&mut hasher);
        dml.hash(&mut hasher);
        let subdir_name = format!("{}_{:x}_{}", interface_name, hasher.finish(), profile);

        let interface_subdir = out_dir.join(subdir_name);
        create_dir_all(&interface_subdir).map_err(|e| {
            Error::custom(format!(
                "Failed to create interface subdir {}: {}",
                interface_subdir.display(),
                e
            ))
        })?;
        write(interface_subdir.join(&header_name), header).map_err(|e| {
            Error::custom(format!(
                "Failed to write interface header {}: {}",
                interface_subdir.join(&header_name).display(),
                e
            ))
        })?;
        write(interface_subdir.join(&dml_name), dml).map_err(|e| {
            Error::custom(format!(
                "Failed to write interface dml {}: {}",
                interface_subdir.join(dml_name).display(),
                e
            ))
        })?;
        write(interface_subdir.join(makefile_name), makefile).map_err(|e| {
            Error::custom(format!(
                "Failed to write interface makefile {}: {}",
                interface_subdir.join(makefile_name).display(),
                e
            ))
        })?;

        let simics_base = if let Ok(simics_base) = var("SIMICS_BASE") {
            PathBuf::from(simics_base)
        } else {
            let mut packages = ispm::packages::list(&GlobalOptions::default())
                .map_err(|e| Error::custom(format!("Could not get installed packages: {e}")))?;
            packages.sort();
            let Some(installed) = packages.installed_packages.as_ref() else {
                return Err(Error::custom(
                    "No SIMICS_BASE variable set and did not get any installed packages",
                ));
            };
            let Some(base) = installed.iter().find(|p| p.package_number == 1000) else {
                return Err(Error::custom (
                "No SIMICS_BASE variable set and did not find a package with package number 1000"
                ));
            };
            base.paths
                .first()
                .ok_or_else(|| {
                    Error::custom("No paths found for package with package number 1000")
                })?
                .clone()
        };

        #[cfg(unix)]
        let mini_python = simics_base
            .join(HOST_DIRNAME)
            .join("bin")
            .join("mini-python");
        #[cfg(unix)]
        let pywrapgen = simics_base.join(HOST_DIRNAME).join("bin").join("pywrapgen");

        #[cfg(windows)]
        let mini_python = simics_base
            .join(HOST_DIRNAME)
            .join("bin")
            .join("mini-python.exe");
        #[cfg(windows)]
        let pywrapgen = simics_base
            .join(HOST_DIRNAME)
            .join("bin")
            .join("pywrapgen.exe");

        let pywrap_dir = simics_base.join(HOST_DIRNAME).join("bin").join("pywrap");

        let python_include = var(PYTHON3_INCLUDE_ENV).unwrap_or(format!(
            "-I{}",
            subdir(simics_base.join(HOST_DIRNAME).join("include"))
                .ok_or_else(|| {
                    Error::custom(format!(
                        "Failed to get include directory subdirectory of {}",
                        simics_base.join(HOST_DIRNAME).join("include").display()
                    ))
                })?
                .display()
        ));

        let python_include_path = subdir(simics_base.join(HOST_DIRNAME).join("include"))
            .ok_or_else(|| {
                Error::custom(format!(
                    "Failed to get include directory subdirectory of {}",
                    simics_base.join(HOST_DIRNAME).join("include").display()
                ))
            })?;

        // SPlit the include path filename like python3.9.3 into [3, 9, 3]
        let python_include_path_filename = python_include_path
            .file_name()
            .ok_or_else(|| Error::custom("Failed to get include path filename"))?
            .to_str()
            .expect("Failed to convert include path filename to str")
            .replace("python", "");

        let python_version = python_include_path_filename
            .split('.')
            .map(|s| {
                s.parse::<u32>().map_err(|e| {
                    Error::custom(format!("Failed to parse python version number part: {}", e))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        #[cfg(unix)]
        let libpython_path = var(PYTHON3_LDFLAGS_ENV).map(PathBuf::from).unwrap_or(
            simics_base
                .join(HOST_DIRNAME)
                .join("sys")
                .join("lib")
                .join(format!("libpython3.{CDYLIB_SUFFIX}")),
        );

        #[cfg(windows)]
        let libpython_path = var(PYTHON3_LDFLAGS_ENV).map(PathBuf::from).unwrap_or(
            simics_base
                .join(HOST_DIRNAME)
                .join("lib")
                .join(
                    subdir(simics_base.join(HOST_DIRNAME).join("include"))
                        .ok_or_else(|| Error::custom(format!("Failed to get include dir")))?
                        .file_name()
                        .ok_or_else(|| Error::custom("Failed to get include dir filename"))?,
                )
                .join(format!("python3.{CDYLIB_SUFFIX}")),
        );

        #[cfg(windows)]
        let libpython_path_static = var(PYTHON3_LDFLAGS_ENV).map(PathBuf::from).unwrap_or(
            simics_base
                .join(HOST_DIRNAME)
                .join("bin")
                .join(format!("python3.{CSTATICLIB_SUFFIX}")),
        );

        // GEN
        // /home/rhart/simics/simics-6.0.169/bin/mini-python \
        //     /home/rhart/simics/simics-6.0.169/scripts/build/gen_pyifaces_c.py \
        //     pyifaces-tsffs-interface.c \
        //     /home/rhart/simics/simics-6.0.169/linux64/bin/pywrap/py-typemaps.c \
        //     /home/rhart/hub/tsffs/modules/tsffs-interface/tsffs-interface.h
        //
        // C:\Users\rhart\simics\simics-6.0.169\win64\bin\mini-python.exe \
        //     C:\Users\rhart\simics\simics-6.0.169/scripts/build/gen_pyifaces_c.py \
        //     pyifaces-tsffs-interface.c \
        //     C:\Users\rhart\simics\simics-6.0.169/win64/bin/pywrap/py-typemaps.c \
        //     C:\Users\rhart\hub\tsffs/modules/tsffs-interface/tsffs-interface.h
        Command::new(&mini_python)
            .arg(
                simics_base
                    .join("scripts")
                    .join("build")
                    .join("gen_pyifaces_c.py"),
            )
            .arg(interface_subdir.join(&pyifaces_interface_c))
            .arg(
                simics_base
                    .join(HOST_DIRNAME)
                    .join("bin")
                    .join("pywrap")
                    .join("py-typemaps.c"),
            )
            .arg(interface_subdir.join(&header_name))
            .check()
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to generate pyifaces C with command {} {} {} {} {}: {}",
                    mini_python.display(),
                    simics_base
                        .join("scripts")
                        .join("build")
                        .join("gen_pyifaces_c.py")
                        .display(),
                    interface_subdir.join(&pyifaces_interface_c).display(),
                    simics_base
                        .join(HOST_DIRNAME)
                        .join("bin")
                        .join("pywrap")
                        .join("py-typemaps.c")
                        .display(),
                    interface_subdir.join(&header_name).display(),
                    e
                ))
            })?;

        // IFACE-DEP
        // DEP_CC $< IFACE_CFLAGS -M -MP -std=gnu99 -MF $@
        // gcc pyifaces-tsffs-interface.c -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 \
        //     -fPIC -Wformat-security -O2 - D_FORTIFY_SOURCE=2 \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/include/python3.9 \
        //     -DPy_LIMITED_API=0x030 90000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -M -MP -std=gnu 99 -MF \
        //     pyifaces-tsffs-interface.d
        //
        // C:\MinGW\bin\gcc.exe pyifaces-tsffs-interface.c -O2 -g -gdwarf-2 -Wall \
        //     -Wwrite-strings -std=gnu99 -D__USE_MINGW_ANSI_STDIO=1 -D_FORTIFY_SOURCE=2 \
        //     -Wformat-security \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -M -MP -std=gnu99 -MF \
        //     pyifaces-tsffs-interface.d
        Command::new("gcc")
            .arg(interface_subdir.join(&pyifaces_interface_c))
            // Begin IFACE_CFLAGS
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg(&python_include)
            .arg("-Wno-write-strings")
            .arg("-Wno-undef")
            .arg("-DPY_MAJOR_VERSION=3")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            // End IFACE_CFLAGS
            .arg("-M")
            .arg("-MP")
            .arg("-std=gnu99")
            .arg("-MF")
            .arg(interface_subdir.join(pyifaces_interface_d))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate pyifaces C dep: {e}")))?;

        // IFACE-CPP
        // CPP IFACE_CFLAGS -DPYWRAP $< >$@
        // gcc -E -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 -fPIC -Wformat-security -O2 \
        //     -D_FORTIFY_SOURCE=2 \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include  -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -DPYWRAP \
        //     pyifaces-tsffs-interface.c >pyifaces-tsffs-interface.i
        //
        // C:\MinGW\bin\gcc.exe -E -O2 -g -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 \
        //     -D__USE_MINGW_ANSI_STDIO=1 -D_FORTIFY_SOURCE=2 -Wformat-security \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -DPYWRAP \
        //     pyifaces-tsffs-interface.c >pyifaces-tsffs-interface.i
        let output = Command::new("gcc")
            .arg("-E")
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg(&python_include)
            .arg("-Wno-write-strings")
            .arg("-Wno-undef")
            .arg("-DPY_MAJOR_VERSION=3")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            .arg("-DPYWRAP")
            .arg(interface_subdir.join(&pyifaces_interface_c))
            .stdout(Stdio::piped())
            .check()
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to generate pyifaces C preprocessor output: {e}"
                ))
            })?;

        write(interface_subdir.join(&pyifaces_interface_i), output.stdout).map_err(|e| {
            Error::custom(format!(
                "Failed to write pyifaces C preprocessor output: {e}"
            ))
        })?;

        // GEN
        // /home/rhart/simics/simics-6.0.169/bin/mini-python \
        //     /home/rhart/simics/simics-6.0.169/scripts/build/gen_pyifaces_t.py \
        //     pyifaces-tsffs-interface.t \
        //     /home/rhart/simics/simics-6.0.169/linux64/bin/pywrap/py-typemaps.c \
        //     /home/rhart/hub/tsffs/modules/tsffs-interface/tsffs-interface.h
        //
        // C:\Users\rhart\simics\simics-6.0.169\win64\bin\mini-python.exe \
        //     C:\Users\rhart\simics\simics-6.0.169/scripts/build/gen_pyifaces_t.py \
        //     pyifaces-tsffs-interface.t \
        //     C:\Users\rhart\simics\simics-6.0.169/win64/bin/pywrap/py-typemaps.c \
        //     C:\Users\rhart\hub\tsffs/modules/tsffs-interface/tsffs-interface.h
        Command::new(&mini_python)
            .arg(
                simics_base
                    .join("scripts")
                    .join("build")
                    .join("gen_pyifaces_t.py"),
            )
            .arg(interface_subdir.join(&pyifaces_interface_t))
            .arg(
                simics_base
                    .join(HOST_DIRNAME)
                    .join("bin")
                    .join("pywrap")
                    .join("py-typemaps.c"),
            )
            .arg(interface_subdir.join(&header_name))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate pyifaces T: {e}")))?;

        // PYWRAP
        // LD_LIBRARY_PATH=/home/rhart/simics/simics-6.0.169/linux64/bin:/home/rhart/simics/simics-6.0.169/linux64/sys/lib \
        //     /home/rhart/simics/simics-6.0.169/linux64/bin/pywrapgen -W \
        //     /home/rhart/simics/simics-6.0.169/linux64/bin/pywrap/ -W . -W \
        //     /home/rhart/hub/tsffs/modules/tsffs-interface -n \
        //     simmod.tsffs_interface.tsffs_interface -t pyifaces-tsffs-interface.t \
        //     pyifaces-tsffs-interface.i -o pyiface-tsffs-interface

        // set PATH=C:\Users\rhart\simics\simics-6.0.169/win64/bin;%PATH% & C:\Users\rhart\simics\simics-6.0.169/win64/bin/pywrapgen.exe -W C:\Users\rhart\simics\simics-6.0.169/win64/bin/pywrap/ \
        //     -W .  -W C:\Users\rhart\hub\tsffs/modules/tsffs-interface \
        //     -n simmod.tsffs_interface.tsffs_interface \
        //     -t pyifaces-tsffs-interface.t pyifaces-tsffs-interface.i -o pyiface-tsffs-interface
        Command::new(pywrapgen)
            .arg("-W")
            .arg(&pywrap_dir)
            // .arg("-W")
            // .arg(".")
            .arg("-W")
            .arg(&interface_subdir)
            .arg("-n")
            .arg(format!(
                "simmod.{}_interface.{}_interface",
                interface_name, interface_name,
            ))
            .arg("-t")
            .arg(interface_subdir.join(&pyifaces_interface_t))
            .arg(interface_subdir.join(&pyifaces_interface_i))
            .arg("-o")
            .arg(interface_subdir.join(pyiface_interface))
            // .print_args()
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate pyiface: {e}")))?;

        // CC
        // gcc -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 -fPIC -Wformat-security -O2 \
        //     -D_FORTIFY_SOURCE=2 \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -O2 -fdisable-ipa-icf -fno-stack-protector \
        //     -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include  -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -c \
        //     pyiface-tsffs-interface-trampolines.c -o \
        //     pyiface-tsffs-interface-trampolines.o
        //
        // C:\MinGW\bin\gcc.exe -O2 -g -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 \
        //     -D__USE_MINGW_ANSI_STDIO=1 -D_FORTIFY_SOURCE=2 -Wformat-security \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -O2 -fdisable-ipa-icf -fno-stack-protector \
        //     -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -c \
        //     pyiface-tsffs-interface-trampolines.c -o \
        //     pyiface-tsffs-interface-trampolines.obj
        Command::new("gcc")
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            // .arg("-fdisable-ipa-icf")
            .arg("-fno-stack-protector")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg(&python_include)
            .arg("-Wno-write-strings")
            .arg("-Wno-undef")
            .arg("-DPY_MAJOR_VERSION=3")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            .arg("-c")
            .arg(interface_subdir.join(pyiface_interface_trampolines_c))
            .arg("-o")
            .arg(interface_subdir.join(&pyiface_interface_trampolines_o))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate pyiface trampolines: {e}")))?;

        // DISAS
        // objdump -dw pyiface-tsffs-interface-trampolines.o > \
        //     pyiface-tsffs-interface-trampolines.od
        //
        // C:\MinGW\bin\\objdump -dw pyiface-tsffs-interface-trampolines.obj \
        //     >pyiface-tsffs-interface-trampolines.od
        let result = Command::new("objdump")
            .arg("-dw")
            .arg(interface_subdir.join(&pyiface_interface_trampolines_o))
            .check()
            .map_err(|e| {
                Error::custom(format!("Failed to disassemble pyiface trampolines: {e}"))
            })?;

        let od_contents = result.stdout;

        // GEN
        // /home/rhart/simics/simics-6.0.169/bin/mini-python \
        //     /home/rhart/simics/simics-6.0.169/scripts/build/analyze-trampolines.py \
        //     linux64 < pyiface-tsffs-interface-trampolines.od > \
        //     pyiface-tsffs-interface-trampoline-data.h
        //
        // C:\Users\rhart\simics\simics-6.0.169\win64\bin\mini-python.exe \
        //     C:\Users\rhart\simics\simics-6.0.169/scripts/build/analyze-trampolines.py \
        //     win64 < pyiface-tsffs-interface-trampolines.od > \
        //     pyiface-tsffs-interface-trampoline-data.h
        let mut process = Command::new(&mini_python)
            .arg(
                simics_base
                    .join("scripts")
                    .join("build")
                    .join("analyze-trampolines.py"),
            )
            .arg(HOST_DIRNAME)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to spawn analyze-tranpolines.py process: {e}"
                ))
            })?;

        process
            .stdin
            .take()
            .ok_or_else(|| Error::custom("Failed to take analyze-tranpolines.py stdin"))?
            .write_all(&od_contents)
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to write pyiface trampolines to analyze-tranpolines.py stdin: {e}"
                ))
            })?;

        let output = process.wait_with_output().map_err(|e| {
            Error::custom(format!("Failed to wait for analyze-tranpolines.py: {e}"))
        })?;

        write(
            interface_subdir.join(pyiface_interface_trampoline_data_h),
            output.stdout,
        )
        .map_err(|e| Error::custom(format!("Failed to write pyiface trampoline data: {e}")))?;

        #[cfg(unix)]
        const FPIC: &[&str] = &["-fPIC"];

        #[cfg(windows)]
        const FPIC: &[&str] = &[];

        // gcc pyiface-tsffs-interface-wrappers.c -gdwarf-2 -Wall -Wwrite-strings \
        //     -std=gnu99 -fPIC -Wformat-security -O2 -D_FORTIFY_SOURCE=2 \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -M -MP -std=gnu99 -MF \
        //     pyiface-tsffs-interface-wrappers.d
        //
        // C:\MinGW\bin\gcc.exe pyiface-tsffs-interface-wrappers.c -O2 -g -gdwarf-2 \
        //     -Wall -Wwrite-strings -std=gnu99 -D__USE_MINGW_ANSI_STDIO=1 \
        //     -D_FORTIFY_SOURCE=2 -Wformat-security \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -M -MP -std=gnu99 -MF \
        //     pyiface-tsffs-interface-wrappers.d
        Command::new("gcc")
            .arg(interface_subdir.join(&pyiface_interface_wrappers_c))
            // Begin IFACE_CFLAGS
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg(&python_include)
            .arg("-Wno-write-strings")
            .arg("-Wno-undef")
            .arg("-DPY_MAJOR_VERSION=3")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            // End IFACE_CFLAGS
            .arg("-M")
            .arg("-MP")
            .arg("-std=gnu99")
            .arg("-MF")
            .arg(interface_subdir.join(pyiface_interface_wrappers_d))
            .check()
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to generate pyiface interface wrappers dep: {e}"
                ))
            })?;

        // /home/rhart/simics/simics-6.0.169/bin/mini-python \
        //     /home/rhart/simics/simics-6.0.169/scripts/build/module_id.py --c-module-id \
        //     --output module_id.c --module-name tsffs-interface --classes  --components \
        //     --host-type linux64 --thread-safe yes --iface-py-module tsffs_interface \
        //     --py-iface-list pyiface-tsffs-interface-interface-list --py-ver 3 \
        //     --py-minor-ver 9 --user-build-id tsffs:1
        //
        // C:\Users\rhart\simics\simics-6.0.169\win64\bin\mini-python.exe \
        //     C:\Users\rhart\simics\simics-6.0.169/scripts/build/module_id.py --c-module-id \
        //     --output module_id.c --module-name tsffs-interface --classes "" --components \
        //     "" --host-type win64 --thread-safe yes --iface-py-module tsffs_interface \
        //     --py-iface-list pyiface-tsffs-interface-interface-list --py-ver 3 \
        //     --py-minor-ver 9 --user-build-id tsffs:1
        Command::new(&mini_python)
            .arg(
                simics_base
                    .join("scripts")
                    .join("build")
                    .join("module_id.py"),
            )
            .arg("--c-module-id")
            .arg("--output")
            .arg(interface_subdir.join(&module_id_c))
            .arg("--module-name")
            .arg(format!("{}_interface", interface_name.replace('_', "-")))
            .arg("--classes")
            .arg("")
            .arg("--components")
            .arg("")
            .arg("--host-type")
            .arg(HOST_DIRNAME)
            .arg("--thread-safe")
            .arg("yes")
            .arg("--iface-py-module")
            .arg(format!("{interface_name}_interface",))
            .arg("--py-iface-list")
            .arg(interface_subdir.join(pyiface_interface_interface_list))
            .arg("--py-ver")
            .arg(&python_version[0].to_string())
            .arg("--py-minor-ver")
            .arg(&python_version[1].to_string())
            .arg("--user-build-id")
            .arg(format!("{}:1", &interface_name))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate module ID: {e}")))?;

        // gcc module_id.c -M -MP -std=gnu99 -MT module_id.d -MT module_id.o \
        //     -fvisibility=hidden -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include  -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/bin/dml/include -MF module_id.d
        //
        // C:\MinGW\bin\gcc.exe module_id.c -M -MP -MT module_id.obj  -DHAVE_MODULE_DATE \
        //     -DSIMICS_6_API -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/bin/dml/include -MF module_id.d
        Command::new("gcc")
            .arg(interface_subdir.join(&module_id_c))
            // Begin IFACE_CFLAGS
            .arg("-M")
            .arg("-MP")
            .arg("-std=gnu99")
            .arg("-MT")
            .arg(interface_subdir.join(&module_id_d))
            .arg("-MT")
            .arg(interface_subdir.join(&module_id_o))
            .arg("-fvisibility=hidden")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            .arg(format!(
                "-I{}",
                simics_base
                    .join(HOST_DIRNAME)
                    .join("bin")
                    .join("dml")
                    .join("include")
                    .display()
            ))
            // End IFACE_CFLAGS
            .arg("-MF")
            .arg(interface_subdir.join(&module_id_d))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate module ID dep: {e}")))?;

        // gcc -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 -fPIC -Wformat-security -O2 \
        //     -D_FORTIFY_SOURCE=2 \
        //     -I/home/rhart/simics/simics-6.0.169/linux64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -c \
        //     pyiface-tsffs-interface-wrappers.c -o pyiface-tsffs-interface-wrappers.o
        //
        // C:\MinGW\bin\gcc.exe -O2 -g -gdwarf-2 -Wall -Wwrite-strings -std=gnu99 \
        //     -D__USE_MINGW_ANSI_STDIO=1 -D_FORTIFY_SOURCE=2 -Wformat-security \
        //     -IC:\Users\rhart\simics\simics-6.0.169/win64/include/python3.9 \
        //     -DPy_LIMITED_API=0x03090000 -Wno-write-strings -Wno-undef \
        //     -DPY_MAJOR_VERSION=3 -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -c \
        //     pyiface-tsffs-interface-wrappers.c -o pyiface-tsffs-interface-wrappers.obj
        Command::new("gcc")
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg(&python_include)
            .arg("-Wno-write-strings")
            .arg("-Wno-undef")
            .arg("-DPY_MAJOR_VERSION=3")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            .arg("-c")
            .arg(interface_subdir.join(&pyiface_interface_wrappers_c))
            .arg("-o")
            .arg(interface_subdir.join(&pyiface_interface_wrappers_o))
            .check()
            .map_err(|e| {
                Error::custom(format!(
                    "Failed to generate pyiface interface wrappers: {e}"
                ))
            })?;

        // gcc -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -I/home/rhart/simics/simics-6.0.169/src/include  -I. \
        //     -I/home/rhart/hub/tsffs/modules/tsffs-interface -gdwarf-2 -Wall \
        //     -Wwrite-strings -std=gnu99 -fPIC -Wformat-security -O2 -D_FORTIFY_SOURCE=2 \
        //     -c module_id.c -o module_id.o
        //
        // C:\MinGW\bin\gcc.exe -DHAVE_MODULE_DATE -DSIMICS_6_API \
        //     -IC:\Users\rhart\simics\simics-6.0.169\src\include  -I. \
        //     -IC:\Users\rhart\hub\tsffs/modules/tsffs-interface -O2 -g -gdwarf-2 -Wall \
        //     -Wwrite-strings -std=gnu99 -D__USE_MINGW_ANSI_STDIO=1 -D_FORTIFY_SOURCE=2 \
        //     -Wformat-security -c module_id.c -o module_id.obj
        Command::new("gcc")
            .arg("-DHAVE_MODULE_DATE")
            .arg("-DSIMICS_6_API")
            .arg(format!(
                "-I{}",
                simics_base.join("src").join("include").display()
            ))
            .arg("-I.")
            .arg(format!("-I{}", interface_subdir.display()))
            .arg("-gdwarf-2")
            .arg("-Wall")
            .arg("-Wwrite-strings")
            .arg("-std=gnu99")
            .args(FPIC)
            .arg("-Wformat-security")
            .arg("-O2")
            .arg("-D_FORTIFY_SOURCE=2")
            .arg("-c")
            .arg(interface_subdir.join(&module_id_c))
            .arg("-o")
            .arg(interface_subdir.join(&module_id_o))
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate module ID: {e}")))?;

        // g++ -shared \
        //     -Wl,--version-script,/home/rhart/simics/simics-6.0.169/config/project/exportmap.elf \
        //     pyiface-tsffs-interface-wrappers.o pyiface-tsffs-interface-trampolines.o \
        //     module_id.o -o tsffs-interface.so -Wl,--gc-sections \
        //     -L/home/rhart/simics/simics-6.0.169/linux64/bin -z noexecstack -z relro -z \
        //     now   /home/rhart/simics/simics-6.0.169/linux64/sys/lib/libpython3.so \
        //     -lsimics-common -lvtutils
        //
        // C:\MinGW\bin\g++.exe -shared \
        //     C:\Users\rhart\simics\simics-6.0.169\config\project\exportmap.def \
        //     pyiface-tsffs-interface-wrappers.obj pyiface-tsffs-interface-trampolines.obj \
        //     module_id.obj -o tsffs-interface.dll \
        //     -LC:/Users/rhart/hub\lib\gcc\x86_64-w64-mingw32\lib \
        //     -LC:\Users\rhart\simics\simics-6.0.169\win64\bin \
        //     C:\Users\rhart\simics\simics-6.0.169\win64\bin\python3.lib -lws2_32 \
        //     -loleaut32 -lole32 -lbcrypt -luserenv -lntdll -lsimics-common -lvtutils
        #[cfg(unix)]
        let exportmap = simics_base
            .join("config")
            .join("project")
            .join("exportmap.elf");
        #[cfg(unix)]
        let exportmap_arg = format!("-Wl,--version-script,{}", exportmap.display());
        #[cfg(unix)]
        let link_args = &["-z", "noexecstack", "-z", "relro", "-z", "now"];
        #[cfg(unix)]
        let libs = &["-lsimics-common", "-lvtutils"];
        #[cfg(windows)]
        let exportmap = simics_base
            .join("config")
            .join("project")
            .join("exportmap.def");
        #[cfg(windows)]
        let exportmap_arg = exportmap;
        #[cfg(windows)]
        let link_args: &[&str] = &[];
        #[cfg(windows)]
        let libs = &[
            libpython_path_static.to_str().ok_or_else(|| {
                Error::custom(format!(
                    "Failed to convert libpython path to string: {}",
                    libpython_path_static.display()
                ))
            })?,
            "-lws2_32",
            "-loleaut32",
            "-lole32",
            "-lbcrypt",
            "-luserenv",
            "-lntdll",
            "-lsimics-common",
            "-lvtutils",
        ];
        Command::new("g++")
            .arg("-shared")
            .arg(&exportmap_arg)
            .arg(interface_subdir.join(&pyiface_interface_wrappers_o))
            .arg(interface_subdir.join(&pyiface_interface_trampolines_o))
            .arg(interface_subdir.join(&module_id_o))
            .arg("-o")
            .arg(interface_subdir.join(&interface_so))
            .arg("-Wl,--gc-sections")
            .arg("-L")
            .arg(simics_base.join(HOST_DIRNAME).join("bin"))
            .args(link_args)
            .arg(&libpython_path)
            .args(libs)
            .check()
            .map_err(|e| Error::custom(format!("Failed to generate interface: {e}")))?;

        // /home/rhart/hub/tsffs/simics --batch-mode --quiet --no-copyright \
        //     --no-module-cache --sign-module tsffs-interface.so
        //
        // C:\Users\rhart\hub\tsffs\simics.bat --batch-mode --quiet --no-copyright \
        // --no-module-cache --sign-module tsffs-interface.dll
        Sign::new(interface_subdir.join(&interface_so))
            .map_err(|e| Error::custom(format!("Error signing interface: {e}")))?
            .write(out_dir.join(&interface_so))
            .map_err(|e| Error::custom(format!("Error writing signed interface: {e}")))?;

        Ok(())
    }
}

pub fn interface_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(a) => a,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let Some(name) = attr_args
        .iter()
        .find(|a| match a {
            NestedMeta::Meta(Meta::NameValue(nv)) => nv.path.is_ident("name"),
            _ => false,
        })
        .and_then(|a| match a {
            NestedMeta::Meta(Meta::NameValue(nv)) => match &nv.value {
                Expr::Lit(l) => {
                    if let Lit::Str(s) = &l.lit {
                        Some(s.value())
                    } else {
                        panic!("Invalid name value")
                    }
                }
                _ => panic!("Invalid name value"),
            },
            _ => None,
        })
    else {
        return Error::custom(r#"'class' attribute should have a 'name = "class_name"' field"#)
            .write_errors()
            .into();
    };

    // Get the `name = ""` attribute
    let input = parse_macro_input!(input as syn::ItemImpl);

    // Try three times to generate the interface, with a short delay between each
    // attempt. For an unknown reason, disassembly/emission of the pyiface trampolines can fail.
    //
    // TODO: Disassemble these trampolines and emit the data in a more reliable way.
    for i in 0..3 {
        if let Err(e) = CInterface::generate(&input, &name) {
            if i == 2 {
                return TokenStream::from(e.write_errors());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            break;
        }
    }

    let interface = Interface { input, name };

    quote!(#interface).into()
}
