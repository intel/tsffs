# TSFFS: Target Software Fuzzer For SIMICS

TSFFS is a snapshotting, coverage-guided fuzzer built on the
[SIMICS](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
full system simulator. TSFFS makes it easy to fuzz and triage crashes on traditionally
challenging targets including UEFI applications, bootloaders, BIOS, kernel modules, and
device firmware. TSSFS can even fuzz user-space applications on Linux and Windows. See
the [requirements](./docs/Requirements.md) to find out if TSSFS can fuzz your code.

- [TSFFS: Target Software Fuzzer For SIMICS](#tsffs-target-software-fuzzer-for-simics)
  - [Quick Start](#quick-start)
  - [Documentation \& Setup](#documentation--setup)
  - [Capabilities](#capabilities)
  - [Use Cases](#use-cases)
  - [Contact](#contact)
  - [Help Wanted / Roadmap](#help-wanted--roadmap)
  - [Authors](#authors)

## Quick Start

The fastest way to start using TSFFS is with our [dockerfile](Dockerfile). To set up
TSFFS locally instead, read the [documentation](./docs/src/SUMMARY.md).

```sh
git clone https://github.com/intel/tsffs
cd tsffs
docker build -t tsffs .
docker run -it tsffs
```

Then, run the provided example target and fuzzing configuration:

```sh
./simics -no-gui --no-win ./fuzz.simics
```

## Documentation & Setup

Documentation for setup & usage of this project lives in the [docs](./docs/src/SUMMARY.md)
directory of this repository.

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

- Kernel & kernel drivers on Windows Linux, and more
- User-space applications on Windows, Linux, and more
- Network applications
- Hypervisors and bare-metal systems

## Contact

If you discover a non-security issue or problem, please file an
[issue](https://github.com/intel/tsffs/issues)!

The best place to ask questions about and get help using TSFFS is in the [Awesome
Fuzzing](https://discord.gg/gCraWct) Discord server. If you prefer, you can email the
[authors](#authors). Questions we receive are periodically added from both Discord and
email to the [FAQ](./docs/FAQ.md).

Please do not create issues or ask publicly about possible security issues you discover
in TSFFS. Instead, see our [Security Policy](./SECURITY.md) and follow the linked
guidelines.

## Help Wanted / Roadmap

See the
[issues](https://github.com/intel/tsffs/issues?q=is%3Aopen+is%3Aissue+label%3Afeature)
for a roadmap of planned features and enhancements. Help is welcome for any features
listed here. If someone is assigned an issue you'd like to work on, please ping them to
avoid duplicating effort!


## Authors

Rowan Hart
<rowan.hart@intel.com>

Brandon Marken Ph.D.
<brandon.marken@intel.com>

Robert Guenzel Ph.D.
<robert.guenzel@intel.com>

