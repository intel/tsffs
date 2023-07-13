# Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

CONFUSE is a snapshotting simulator, coverage-guided fuzzer built on Simics! It lets you
easily fuzz things that are traditionally challenging to fuzz, like UEFI applications,
bootloaders, kernel modules, firmware, and the like.

- [Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced](#tsffs-controlled-fuzzing-with-simics----enhanced)
  - [Setup](#setup)
  - [Architecture](#architecture)
  - [Running A Sample Target](#running-a-sample-target)
  - [Authors](#authors)



## Setup

Detailed instructions for setting up and building this project can be found in
[SETUP.md](./docs/SETUP.md). You should follow the documentation there before trying
to run the samples.

## Architecture

CONFUSE consists of three parts: the *fuzzer*, the *tsffs module*, and the *target*.

The target refers to the software you want to fuzz, including any environment
configuration you need to do to get it up and running. By and large, the fuzzer and
tsffs module are opaque to users of CONFUSE. There is a limited API for configuration
and initialization, but otherwise these components should not need much interaction.

## Running A Sample Target

There are two provided sample targets, `hello-world` and `x509-parse`. You can run them
by running one of the following commands after following the setup instructions.

```sh
$ cargo run --release --bin simics-fuzz --features=6.0.166 -- \
  -c ./newcorpus/ -s ./newsolution/ -l ERROR -t -C 1 -g \
  --package 2096:6.0.66 \
  --file targets/hello-world/src/bin/resource/HelloWorld.efi:%simics%/targets/hello-world/HelloWorld.efi \
  --file targets/hello-world/src/bin/resource/app.py:%simics%/scripts/app.py \
  --file targets/hello-world/src/bin/resource/app.yml:%simics%/scripts/app.yml \
  --file targets/hello-world/src/bin/resource/minimal_boot_disk.craff:%simics%/targets/hello-world/minimal_boot_disk.craff \
  --file targets/hello-world/src/bin/resource/run_uefi_app.nsh:%simics%/targets/hello-world/run_uefi_app.nsh \
  --file targets/hello-world/src/bin/resource/run-uefi-app.simics:%simics%/targets/hello-world/run-uefi-app.simics \
  --command CONFIG:%simics%/scripts/app.yml
```

or

```sh
$ cargo run --release --bin simics-fuzz --features=6.0.166 -- \
  -c ./newcorpus/ -s ./newsolution/ -l ERROR -t -C 1 -g \
  --package 2096:6.0.66 \
  --file targets/x509-parse/src/bin/resource/X509Parse.efi:%simics%/targets/x509-parse/X509Parse.efi \
  --file targets/x509-parse/src/bin/resource/app.py:%simics%/scripts/app.py \
  --file targets/x509-parse/src/bin/resource/app.yml:%simics%/scripts/app.yml \
  --file targets/x509-parse/src/bin/resource/minimal_boot_disk.craff:%simics%/targets/x509-parse/minimal_boot_disk.craff \
  --file targets/x509-parse/src/bin/resource/run_uefi_app.nsh:%simics%/targets/x509-parse/run_uefi_app.nsh \
  --file targets/x509-parse/src/bin/resource/run-uefi-app.simics:%simics%/targets/x509-parse/run-uefi-app.simics \
  --command CONFIG:%simics%/scripts/app.yml
```

These samples will run for 30 fuzzing stages (about 1-5k executions) before stopping.
Logs will output to `/tmp/tsffs-logXXXX.log` where `X` is a random character. You can
view the logs while the fuzzer is running in another
terminal with `tail -F /tmp/tsffs-log*`. The log will rotate every 100MB to avoid
depleting storage. The fuzzer should stop the `simics-common` process when it finishes,
but in some cases this may fail (the project is experimental!). You can check for
defunct processes with `ps | grep simics-common` and kill them with
`pkill simics-common` if this happens.

## Authors

Brandon Marken Ph.D.
brandon.marken@intel.com

Robert Geunzel Ph.D.
robert.geunzel@intel.com

Rowan Hart
rowan.hart@intel.com
