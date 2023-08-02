<p align="center">
  <img src="docs/images/logo.png" alt="TSFFS Logo">
</p>

# TSFFS: Target Software Fuzzer For SIMICS

TSFFS is a snapshotting, coverage-guided fuzzer built on the
[SIMICS](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
full system simulator. TSFFS makes it easy to fuzz and traige crashes on traditionally
challenging targets including UEFI applications, bootloaders, BIOS, kernel modules, and
device firmware. TSSFS can even fuzz user-space applications on Linux and Windows. See
the [requirements](./docs/Requirements.md) to find out if TSSFS can fuzz your code.

- [TSFFS: Target Software Fuzzer For SIMICS](#tsffs-target-software-fuzzer-for-simics)
  - [Capabilities](#capabilities)
  - [Setup](#setup)
  - [Running a Simple Sample Target](#running-a-simple-sample-target)
  - [Running an EDK2 Sample Target](#running-an-edk2-sample-target)
  - [Documentation](#documentation)
  - [Roadmap](#roadmap)
  - [Authors](#authors)


<https://github.com/intel-innersource/applications.security.fuzzing.confuse/assets/30083762/004ba56e-561c-470a-baf4-427014b43362>


## Capabilities

This fuzzer is built using [LibAFL](https://github.com/AFLplusplus/LibAFL) and SIMICS
and takes advantage of several of the state of the art capabilities of both.

- Edge coverage guided
- Snapshotting (fully deterministic)
- Parallel fuzzing (across cores, machines soon)
- Easy to add to existing SIMICS projects
- Triage mode to reproduce and debug crashes

## Setup

Detailed instructions for setting up and building this project can be found in
[Setup.md](./docs/Setup.md). You should follow the documentation there to set up the
fuzzer before trying to run the sample targets.

## Running a Simple Sample Target

We provide a sample target that represents the simplest possible use of the fuzzer. Once
you have set up the fuzzer by following the directions [above](#setup), you can run it
with (from the root of this repo):

```sh
cargo run --release --bin simics-fuzz --features=6.0.168 -- \
    --corpus /tmp/corpus --solutions solution --log-level INFO --cores 1  \
    --file examples/harnessing-uefi/rsrc/target.efi:%simics%/target.efi \
    --file examples/harnessing-uefi/rsrc/fuzz.simics:%simics%/fuzz.simics \
    --file examples/harnessing-uefi/rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff \
    --package 2096:6.0.69 \
    --command 'COMMAND:run-script "%simics%/fuzz.simics"'
```

If you want to see the visualizer above, you can enable the SIMICS GUI during fuzzing
by adding `--enable-simics-gui` and if you want a fancy TUI output, add the `-t` flag!

## Running an EDK2 Sample Target 

There are two provided sample targets, `hello-world` and `x509-parse`. You can run them
in the basic configuration with the commands below, respectively.

```sh
cargo run --release --bin simics-fuzz --features=6.0.168 -- \
  -c /tmp/hello-world-corpus/ -s /tmp/hello-world-solution/ -l ERROR -t -C 1 \
  --package 2096:6.0.69 \
  --file examples/hello-world/rsrc/HelloWorld.efi:%simics%/targets/hello-world/HelloWorld.efi \
  --file examples/hello-world/rsrc/app.py:%simics%/scripts/app.py \
  --file examples/hello-world/rsrc/app.yml:%simics%/scripts/app.yml \
  --file examples/hello-world/rsrc/minimal_boot_disk.craff:%simics%/targets/hello-world/minimal_boot_disk.craff \
  --file examples/hello-world/rsrc/run_uefi_app.nsh:%simics%/targets/hello-world/run_uefi_app.nsh \
  --file examples/hello-world/rsrc/run-uefi-app.simics:%simics%/targets/hello-world/run-uefi-app.simics \
  --command CONFIG:%simics%/scripts/app.yml
```

```sh
cargo run --release --bin simics-fuzz --features=6.0.168 -- \
  -c /tmp/x509-parse-corpus/ -s /tmp/x509-parse-solution/ -l ERROR -t -C 1 \
  --package 2096:6.0.69 \
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

## Roadmap

See the
[issues](https://github.com/intel-innersource/applications.security.fuzzing.confuse/issues?q=is%3Aopen+is%3Aissue+label%3Afeature)
for a roadmap of planned features and enhancements.

## Authors

Brandon Marken Ph.D.
<brandon.marken@intel.com>

Robert Geunzel Ph.D.
<robert.geunzel@intel.com>

Rowan Hart
<rowan.hart@intel.com>
