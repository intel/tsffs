# Simple Example

This example shows how to use the Fuzzer-to-Simics connection. Right now it is also used as the (only :-( ) test during development. More tests will be added in the future.

## Requirements

- Linux host
- Simics Base, pkg number 1000, developed with version 6.0.157
- Quick-Start Platform, pkg number 2096, developed with version 6.0.65
- A local clone of this repo

## Preparation

There are three main steps here: Build the EFI app, prepare the Simics project, build the confuse interface library and the host application.

### Build EFI App

1. Create a directory to work in. We will call that `workspace`
2. `cd /path/to/workspace`
3. `git clone https://github.com/tianocore/edk2`
4. `git clone https://github.com/tianocore/edk2-libc`
5. `cd edk2`
6. `git submodule update --init --recursive`
7. `make -C BaseTools`
8. `source edksetup.sh`
9. `export PACKAGES_PATH=/path/to/workspace/edk2-libc`
10. Edit `Conf/target.txt` such that it has the following settings
```
    ACTIVE_PLATFORM       = MdeModulePkg/MdeModulePkg.dsc
    TOOL_CHAIN_TAG        = GCC5
    TARGET_ARCH           = X64
```
11. Go into `edk2-libc`: `cd /path/to/workspace/edk2-libc`
12. create a symlink pointing to `HelloFuzzing` from this repo: `ln -s /path/to/this/repo/simple-example/HelloFuzzing AppPkg/Applications/HelloFuzzing`
13. Edit `AppPkg/AppPkg.dsc` such that it has `AppPkg/Applications/HelloFuzzing/HelloFuzzing.inf` under `[Components]`.
14. Invoke `build -p AppPkg/AppPkg.dsc -m AppPkg/Applications/HelloFuzzing/HelloFuzzing.inf`

### Prepare the Simics Project

1. Create a Simics project somewhere: `/path/to/your/simics-6.0.157/bin/project-setup /path/to/the/simics-project`
2. Go into the project: `cd /path/to/the/simics-project`
3. Ensure the QSP package is known in the project: `echo /path/to/your/simics-qsp-x86-6.0.65 > .package-list`
4. Update the project: `./bin/project-setup`
5. Sym-link the `confuse_ll` Simics module to the project: `ln -s /path/to/this/repo/simics/modules/confuse_ll modules/confuse_ll`
6. Sym-link the `qsp-x86-fuzzing` Simics targets directory to the project: `ln -s /path/to/this/repo/simics/targets/qsp-x86-fuzzing targets/qsp-x86-fuzzing`
7. Create a directory called `simple-example` in the project: `mkdir simple-example`
8. Sym-link `simics-scripts` from the example in the repo to your project: `ln -s /path/to/this/repo/simple-example/simics-scripts simple-example/simics-scripts`
9. Sym-link `HelloFuzzing.efi` into the project: `ln -s /path/to/workspace/edk2/Build/AppPkg/DEBUG_GCC5/X64/HelloFuzzing.efi`
10. Invoke `make` in the project.

### Build library and example

1. Go into the simple-example directory of your local repo: `cd /path/to/this/repo/simple-example`
2. Invoke `make`. This will build the library and the test app at the same time.

## Running

1. Go into the simple-example directory of this repo: `cd /path/to/this/repo/simple-example`
2. Invoke `runme` giving it the **absolute** path to your project: `./runme /path/to/the/simics-project`

If all works out, you should see Simics starting, going into the UEFI shell, load your EFI app and run it 10 times (right now everytime with the same data, as we have not yet finished the data input/output interface). Note that we have a sleep time of one second in there to allow you to see something.

