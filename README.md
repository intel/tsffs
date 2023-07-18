# TSFFS: Target Software Fuzzer For SIMICS

CONFUSE is a snapshotting simulator, coverage-guided fuzzer built on Simics! It lets you
easily fuzz things that are traditionally challenging to fuzz, like UEFI applications,
bootloaders, kernel modules, firmware, and the like.

- [TSFFS: Target Software Fuzzer For SIMICS](#tsffs-target-software-fuzzer-for-simics)
  - [Capabilities](#capabilities)
  - [Setup](#setup)
  - [Running A Sample Target](#running-a-sample-target)
  - [Documentation](#documentation)
  - [Authors](#authors)

![A demo video of TSFFS running](./docs/images/mini.mp4)

## Capabilities

This fuzzer is built using [LibAFL](https://github.com/AFLplusplus/LibAFL) and SIMICS
and takes advantage of several of the state of the art capabilities
of both.

- Edge coverage guided
- Snapshotting (fully deterministic)
- Parallel fuzzing (across cores, machines soon)
- Easy to add to existing SIMICS projects

## Setup

Detailed instructions for setting up and building this project can be found in
[Setup.md](./docs/Setup.md). You should follow the documentation there to set up the
fuzzer before trying to run the sample targets.

## Running A Sample Target

There are two provided sample targets, `hello-world` and `x509-parse`. You can run them
in the basic configuration with the commands below, respectively.

```sh
cargo run --release --bin simics-fuzz --features=6.0.166 -- \
  -c /tmp/hello-world-corpus/ -s /tmp/hello-world-solution/ -l ERROR -t -C 1 \
  --package 2096:6.0.66 \
  --file examples/hello-world/rsrc/HelloWorld.efi:%simics%/targets/hello-world/HelloWorld.efi \
  --file examples/hello-world/rsrc/app.py:%simics%/scripts/app.py \
  --file examples/hello-world/rsrc/app.yml:%simics%/scripts/app.yml \
  --file examples/hello-world/rsrc/minimal_boot_disk.craff:%simics%/targets/hello-world/minimal_boot_disk.craff \
  --file examples/hello-world/rsrc/run_uefi_app.nsh:%simics%/targets/hello-world/run_uefi_app.nsh \
  --file examples/hello-world/rsrc/run-uefi-app.simics:%simics%/targets/hello-world/run-uefi-app.simics \
  --command CONFIG:%simics%/scripts/app.yml
```

```sh
cargo run --release --bin simics-fuzz --features=6.0.166 -- \
  -c /tmp/x509-parse-corpus/ -s /tmp/x509-parse-solution/ -l ERROR -t -C 1 \
  --package 2096:6.0.66 \
  --file examples/x509-parse/rsrc/X509Parse.efi:%simics%/targets/x509-parse/X509Parse.efi \
  --file examples/x509-parse/rsrc/app.py:%simics%/scripts/app.py \
  --file examples/x509-parse/rsrc/app.yml:%simics%/scripts/app.yml \
  --file examples/x509-parse/rsrc/minimal_boot_disk.craff:%simics%/targets/x509-parse/minimal_boot_disk.craff \
  --file examples/x509-parse/rsrc/run_uefi_app.nsh:%simics%/targets/x509-parse/run_uefi_app.nsh \
  --file examples/x509-parse/rsrc/run-uefi-app.simics:%simics%/targets/x509-parse/run-uefi-app.simics \
  --command CONFIG:%simics%/scripts/app.yml
```

## Documentation

Documentation for this project lives in the [docs](./docs/README.md) directory of this
repository.

## Authors

Brandon Marken Ph.D.
<brandon.marken@intel.com>

Robert Geunzel Ph.D.
<robert.geunzel@intel.com>

Rowan Hart
<rowan.hart@intel.com>
