# Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

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

### [confuse-simics-modsign](./confuse-simics-modsign)

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


## Authors

Brandon Marken Ph.D.
brandon.marken@intel.com

Robert Geunzel Ph.D.
robert.geunzel@intel.com

Rowan Hart
rowan.hart@intel.com
