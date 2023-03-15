# Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

## Setup

Detailed instructions for setting up and building this project can be found in
[SETUP.md](./docs/SETUP.md). You should follow the documentation there before trying
to run the samples.

## Running A Sample

There are two provided samples, `hello-world` and `x509-parse`. You can run them by
running one of the following commands after following the setup instructions.

```sh
$ cargo run --bin hello-world-fuzz
```

or

```sh
$ cargo run --bin x509-parse-fuzz
```

These samples will run for 100 fuzzing stages (about 5000 executions) before stopping.

## Crates

This project consists of several crates.

### [confuse-fuzz](./confuse-fuzz/)

This crate provides the actual fuzzer entrypoint with an executor that loads and
communicates with SIMICS.

### [confuse-simics-api](./confuse-simics-api/)

This crate provides raw bindings to the SIMICS C API in rust.

### [confuse-simics-manifest](./confuse-simics-manifest/)

This crate provides utilities for parsing and checking the manifest files SIMICS uses
to track multiple installations, versions, and packages for SIMICS.

### [confuse-simics-module](./confuse-simics-module)

This crate implements utilities to apply the checksum signature SIMICS uses to determine
if a shared object is loadable.

### [confuse-simics-project](./confuse-simics-project/)

This crate provides an abstraction over the raw simics project setup tool and allows
easily building projects with specified packages, targets, and SIMICS modules.

## Modules

This project also implements several SIMICS modules, some of which are samples and some
of which are used by `confuse-fuzz` to communicate with, control, and receive feedback
from SIMICS.

### [modules/minimal-simics-module](./modules/minimal-simics-module/)

This is the most basic possible SIMICS module written in rust, with tests that
demonstrate how to load it in the simulator.

### [modules/confuse-module](./modules/confuse-module/)

This is the module used by Confuse to fuzz. At the moment, it assumes that the target
software is running on x86 QSP, and handles communication with the fuzzer, branch
tracing and feedback, and managing SIMICS through reset/start/snapshot and obtaining
instrumentation information on execution and exceptions/faults and timeouts.

### [modules/ipc-test-module](./modules/ipc-test-module/)

This module demonstrates communication from the outside world to the SIMICS module over
and IPC Channel.

## Targets

Targets are any software that will actually be fuzzed, as well as a small fuzzer stub
to actually run the fuzzing process.

### [targets/hello-world](./targets/hello-world/)

This is the simplest possible target, it takes some input and then performs an action
(normal exit, hang for 10s, invalid opcode exception) based on the first byte of the
input. This hello world target should be used to test the fuzzer. It can be extended
to cause other 

### [targets/x509-parse](./targets/x509-parse/)

This target is also very simple, but instead of doing custom actions based on the
input, it calls the `X509VerifyCert` function from EDK2's crypto library. Maybe
we will even find a bug!

## Utilities

We have a couple of utilities:

### [util/ipc-shm](./util/ipc-shm/)

This crate provides a thin wrapper around `memmap2::Mmap` that allows the memory mapped
file to be sent easily over an IPC Channel. This is used for the AFL branch tracer map.

### [util/raw-cstr](./util/raw-cstr/)

This crate provides a macro for declaring raw constant C strings inline. Right now, it
has a known issue in that it will slowly leak memory (just a *tiny* bit!) but there is
opportunity to optimize it not to by holding references to existing strings.

## Authors

Brandon Marken Ph.D.
brandon.marken@intel.com

Robert Geunzel Ph.D.
robert.geunzel@intel.com

Rowan Hart
rowan.hart@intel.com