# TSFFS Documentation

- [TSFFS Documentation](#tsffs-documentation)
  - [Capabilities](#capabilities)
  - [Use Cases](#use-cases)
  - [Why TSFFS](#why-tsffs)
  - [Terminology](#terminology)

TSFFS is a snapshotting, coverage-guided fuzzer built on the
[SIMICS](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
full system simulator. TSFFS makes it easy to fuzz and triage crashes on traditionally
challenging targets including UEFI applications, bootloaders, BIOS, kernel modules, and
device firmware. TSSFS can even fuzz user-space applications on Linux and Windows.

## Capabilities

This fuzzer is built using [LibAFL](https://github.com/AFLplusplus/LibAFL) and SIMICS
and takes advantage of several of the state of the art capabilities of both.

- Edge coverage guided
- Snapshotting (fully deterministic)
- Parallel fuzzing (across cores, machines soon)
- Easy to add to existing SIMICS projects
- Triage mode to reproduce and debug crashes
- Modern fuzzing methodologies:
  - Redqueen/I2S taint-based mutation
  - MOpt & Auto-token mutations
  - More coming soon!

## Use Cases

TSFFS is focused on several primary use cases:

- UEFI and BIOS code, particulary based on [EDKII](https://github.com/tianocore/edk2)
- Pre- and early-silicon firmware and device drivers
- Hardware-dependent kernel and firmware code
- Fuzzing for complex error conditions

However, TSFFS is also capable of fuzzing:

- Kernel & kernel drivers
- User-space applications
- Network applications

## Why TSFFS

There are several tools capable of fuzzing firmware and UEFI code. Notably, the
[HBFA](https://github.com/tianocore/edk2-staging/tree/HBFA)
project and the [kAFL](https://github.com/IntelLabs/kAFL) project enable system software
fuzzing with various tradeoffs.

HBFA is very fast, and enables fuzzing with sanitizers in Linux userspace. However, it
requires stubs for any hardware interactions as well as the ability to compile code with
instrumentation. For teams with resources to create a working HBFA configuration, it
should be used alongside TSFFS to enable additional error condition detection.

kAFL is also extremely fast, and is hypervisor based which allows deterministic
snapshotting of systems under test. This also makes it ideal for very complex systems
and system-of-systems fuzzing, where interactions between components or the use of real
hardware is necessary. kAFL suffers from a similar limitation as HBFA in that it
requires working device stubs or simulation to be implemented in QEMU, and additionally
requires a patched kernel to run the required KVM modifications.

Both of these tools should be used where possible to take advantage of their unique
capabilities, but TSFFS aims to reduce the barrier to fuzzing low-level systems
software. It is slower (though not unacceptably so) than HBFA or kAFL, and is not (yet)
capable of leveraging sanitizers. In exchange, using it is as simple as adding a few
lines of code to a SIMICS script and ten or less lines of code to your firmware source
code. In addition, because it is based on SIMICS, the tool of choice of firmware
developers, the models and configurations for the code under test can be used as they
are, and developers can continue to use familiar tools to reduce the lift of enabling
fuzzing.

## Terminology

Some terminology in this document might be unfamiliar, or used in an unfamiliar way.

- *Solution*: Any condition that is a *goal* of a fuzzing campaign. Most fuzzing
  campaigns look for crashes or hangs in the target software, both of which are types of
  solutions. However, at the firmware level, other conditions may also be considered
  exceptional, and are considered solutions as well. For example, some firmware is only
  permitted to write to specific memory regions, and a write outside of them is
  problematic but will not cause a crash in the traditional sense.
- *Target Software*: Because TSFFS is capable of fuzzing the full stack of software from
  initial firmware through user-space applications, any software under test by the
  fuzzer is referred to as *target software*.