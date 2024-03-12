# Build Internals

TSFFS is somewhat unique as a SIMICS module written in Rust, instead of a supported
language like Python, C, C++, or DML. This is possible due to a complex build
configuration.

- [Build Internals](#build-internals)
  - [SIMICS API Bindings](#simics-api-bindings)
    - [Low Level Bindings](#low-level-bindings)
    - [High Level Bindings](#high-level-bindings)
    - [Macros](#macros)
  - [Build Process](#build-process)

## SIMICS API Bindings

TSFFS maintains its own bindings to the SIMICS API, both low level bindings to the
exported C API and high level bindings that are more idiomatic Rust.

### Low Level Bindings

The low level bindings are initially generated using
[bindgen](https://github.com/rust-lang/rust-bindgen) from the C header files in the
SIMICS installation's `src/include` directory. This generates a large Rust file
containing declarations of all data types and function prototypes in the headers.

The low level bindings crate (`simics-api-sys`) also emits linking instructions in its
`build.rs` build script to link against the libraries which actually provide the symbols
exported by the SIMICS header files.

### High Level Bindings

The high level bindings import the low level bindings and re-export them as
`simics::api::sys`. They also provide high level, more idiomatic bindings to nearly all
APIs in the low level bindings. In general, the `sys` bindings should never be used
except by the high level bindings, they are provided simply as an escape hatch.

The high level bindings make extensive use of dynamic code generation from the low level
binding code and SIMICS HTML documentation. Code for generated for all built-in
interfaces in SIMICS base by parsing the struct definitions from the low level bindings.
HTML documentation is parsed to emit index information for the interfaces. Code is
generated for all built-in HAPS in SIMICS base by parsing the names and prototypes of
the HAP callback function, and emitting safe bindings around them.

There are several core ideas expressed in the high level bindings:

* `AttrValue` as a serialization and deserialization primitive
* Functions which take callbacks as parameters are wrapped with bindings that instead
  take closures
* Where appropriate (in particular with `ConfObject`), pointers are left raw. Pointers
  which are never dereferenced except by the SIMICS API are left raw without violating
  safety.

### Macros

The high level SIMICS crate also has an associated proc-macro crate. It provides several
attribute macros for implementing:

* Interfaces
* Classes
* Functions which may throw SIMICS exceptions

It also provides derive macros for converting Rust structs to and from `AttrValue`s.

## Build Process

The build process for the TSFFS SIMICS package is implemented in the
`cargo-simics-build` crate. It builds the crate with correct link arguments, signs the
output module, and packages the module along with any built interfaces.
