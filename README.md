# Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

CONFUSE is a snapshotting simulator, coverage-guided fuzzer built on Simics! It lets you
easily fuzz things that are traditionally challenging to fuzz, like UEFI applications,
bootloaders, kernel modules, firmware, and the like.

- [Confuse: **CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced](#confuse-controlled-fuzzing-with-simics----enhanced)
  - [Setup](#setup)
  - [Architecture](#architecture)
  - [Running A Sample Target](#running-a-sample-target)
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

## Authors

Brandon Marken Ph.D.
brandon.marken@intel.com

Robert Geunzel Ph.D.
robert.geunzel@intel.com

Rowan Hart
rowan.hart@intel.com
