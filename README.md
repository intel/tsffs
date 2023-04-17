# Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

CONFUSE is a snapshotting simulator, coverage-guided fuzzer built on Simics! It lets you
easily fuzz things that are traditionally challenging to fuzz, like UEFI applications,
bootloaders, kernel modules, firmware, and the like.

The older proof of concept version of confuse is still available in the [intel Sandbox](https://github.com/intel-sandbox/tool.fuzzing.simics.simics-fuzzing/tree/main). 


- [Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced](#confuse-controlled-fuzzing-with-simics----enhanced)
  - [Setup](#setup)
  - [Architecture](#architecture)
  - [Running A Sample Target](#running-a-sample-target)
  - [Crates](#crates)
    - [Primary Crates](#primary-crates)
      - [confuse-fuzz](#confuse-fuzz)
      - [confuse-simics-api](#confuse-simics-api)
      - [confuse-simics-manifest](#confuse-simics-manifest)
      - [confuse-simics-module](#confuse-simics-module)
      - [confuse-simics-project](#confuse-simics-project)
    - [Simics Modules](#simics-modules)
      - [modules/minimal-simics-module](#modulesminimal-simics-module)
      - [modules/confuse-module](#modulesconfuse-module)
      - [modules/ipc-test-module](#modulesipc-test-module)
    - [Fuzzing Targets](#fuzzing-targets)
      - [targets/hello-world](#targetshello-world)
      - [targets/x509-parse](#targetsx509-parse)
    - [Utilities](#utilities)
      - [util/ipc-shm](#utilipc-shm)
      - [util/raw-cstr](#utilraw-cstr)
  - [Authors](#authors)



## Setup

Detailed instructions for setting up and building this project can be found in
[SETUP.md](./docs/SETUP.md). You should follow the documentation there before trying
to run the samples.

## Architecture

CONFUSE consists of three parts: the *fuzzer*, the *confuse module*, and the *target*.

The target refers to the software you want to fuzz, including any environment
configuration you need to do to get it up and running. By and large, the fuzzer and
confuse module are opaque to users of CONFUSE. There is a limited API for configuration
and initialization, but otherwise these components should not need much interaction.

## Running A Sample Target

There are two provided sample targets, `hello-world` and `x509-parse`. You can run them
by running one of the following commands after following the setup instructions.

```sh
$ cargo run --bin hello-world-fuzz -- --input ./targets/hello-world/corpus --log-level TRACE --cycles 30
```

or

```sh
$ cargo run --bin x509-parse-fuzz -- --input ./targets/x509-parse/corpus --log-level TRACE --cycles 30
```

These samples will run for 30 fuzzing stages (about 1-5k executions) before stopping.
Logs will output to `/tmp/confuse-logXXXX.log` where `X` is a random character. You can
view the logs while the fuzzer is running in another
terminal with `tail -F /tmp/confuse-log*`. The log will rotate every 100MB to avoid
depleting storage. The fuzzer should stop the `simics-common` process when it finishes,
but in some cases this may fail (the project is experimental!). You can check for
defunct processes with `ps | grep simics-common` and kill them with
`pkill simics-common` if this happens.

## Crates

This project consists of several crates.

### Primary Crates

#### [confuse-fuzz](./confuse-fuzz/)

This crate provides the actual fuzzer entrypoint with an executor that loads and
communicates with SIMICS.

#### [confuse-simics-api](./confuse-simics-api/)

This crate provides raw bindings to the SIMICS C API in rust.

#### [confuse-simics-manifest](./confuse-simics-manifest/)

This crate provides utilities for parsing and checking the manifest files SIMICS uses
to track multiple installations, versions, and packages for SIMICS.

#### [confuse-simics-module](./confuse-simics-module)

This crate implements utilities to apply the checksum signature SIMICS uses to determine
if a shared object is loadable.

#### [confuse-simics-project](./confuse-simics-project/)

This crate provides an abstraction over the raw simics project setup tool and allows
easily building projects with specified packages, targets, and SIMICS modules.

### Simics Modules

This project also implements several SIMICS modules, some of which are samples and some
of which are used by `confuse-fuzz` to communicate with, control, and receive feedback
from SIMICS.

#### [modules/minimal-simics-module](./modules/minimal-simics-module/)

This is the most basic possible SIMICS module written in rust, with tests that
demonstrate how to load it in the simulator.

#### [modules/confuse-module](./modules/confuse-module/)

This is the module used by Confuse to fuzz. At the moment, it assumes that the target
software is running on x86 QSP, and handles communication with the fuzzer, branch
tracing and feedback, and managing SIMICS through reset/start/snapshot and obtaining
instrumentation information on execution and exceptions/faults and timeouts.

#### [modules/ipc-test-module](./modules/ipc-test-module/)

This module demonstrates communication from the outside world to the SIMICS module over
and IPC Channel.

### Fuzzing Targets

Targets are any software that will actually be fuzzed, as well as a small fuzzer stub
to actually run the fuzzing process.

#### [targets/hello-world](./targets/hello-world/)

This is the simplest possible target, it takes some input and then performs an action
(normal exit, hang for 10s, invalid opcode exception) based on the first byte of the
input. This hello world target should be used to test the fuzzer. It can be extended
to cause other 

#### [targets/x509-parse](./targets/x509-parse/)

This target is also very simple, but instead of doing custom actions based on the
input, it calls the `X509VerifyCert` function from EDK2's crypto library. Maybe
we will even find a bug!

### Utilities

We have a couple of utilities:

#### [util/ipc-shm](./util/ipc-shm/)

This crate provides a thin wrapper around `memmap2::Mmap` that allows the memory mapped
file to be sent easily over an IPC Channel. This is used for the AFL branch tracer map.

#### [util/raw-cstr](./util/raw-cstr/)

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