# Setup (Windows)

This guide will walk you through installing build dependencies, building, and installing
TSFFS into your SIMICS installation on Windows. All console commands in this document
should be run in PowerShell (the default shell on recent Windows versions).

- [Setup (Windows)](#setup-windows)
  - [Install System Dependencies](#install-system-dependencies)
    - [Git](#git)
    - [7-Zip](#7-zip)
    - [Install MinGW-w64](#install-mingw-w64)
    - [Install Rust](#install-rust)
    - [Install SIMICS](#install-simics)
  - [Build TSFFS](#build-tsffs)
  - [Test TSFFS](#test-tsffs)
  - [Set Up For Local Development](#set-up-for-local-development)
  - [Troubleshooting](#troubleshooting)
    - [I Already Have A MinGW Installation](#i-already-have-a-mingw-installation)
    - [Command is Unrecognized](#command-is-unrecognized)

## Install System Dependencies

### Git

Download and install Git for Windows from the [git
website](https://git-scm.com/download/win). The default options are acceptable. In a new
powershell terminal, the command `git -h` should complete with no error.

### 7-Zip

Download and install 7-Zip from the [website](https://www.7-zip.org/). In a new
powershell terminal, the command `7z -h` should complete with no error.

### Install MinGW-w64

If you already have a MinGW-w64 installation, you can skip this step and see the
[section](#i-already-have-a-mingw-installation) on using an existing installation.

Download the MinGW archive from [winlibs.com](https://winlibs.com/#download-release).
Select the UCRT runtime *with* POSIX threads and LLVM/Clang/LLD/LLDB. Select the Win64
7-Zip archive, or use the [direct
link](https://github.com/brechtsanders/winlibs_mingw/releases/download/13.2.0-16.0.6-11.0.0-ucrt-r1/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z)
to download an install the tested MinGW version (LLVM/Clang/LLD/LLDB+UCRT+POSIX, GCC
13.2.0). Once downloaded, run the following commands (assuming the file is downloaded to
`~/Downloads`, substitute the correct path if not) to extract the file to the MinGW
directory. You may prefer to right-click the `7z` file, choose `7-Zip: Extract Files`,
and type `C:\MinGW\` as the destination instead of running these commands.

```powershell
7z x -o ~/Downloads/mingw-w64/ ~/Downloads/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z
mv ~/Downloads/mingw-w64/mingw64/ C:/MinGW/
```

Next, add MinGW to the `Path` in your environment variables.

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\MinGW\bin`
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.

Close your terminal and open a new terminal. Run `gcc --version` and ensure no error
occurs.

### Install Rust

Go to [rustup.rs](https://rustup.rs/) and download `rustup-init.exe`. Run
`rustup-init.exe` with the following arguments:

```powershell
rustup-init.exe --default-toolchain nightly --default-host x86_64-pc-windows-gnu -y
```

After installation, close your terminal and open a new terminal as prompted. Run `cargo
--verison`. Ensure the version ends with `-nightly` (this is required to run the build
script).

### Install SIMICS

Go to the [SIMICS download
page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
and download:

* `simics-6-packages-VERSION-win64.ispm`
* `intel-simics-package-manager-VERSION-win64.exe`

Run the downloaded `.exe` file to install `ispm` using the default settings (for your
user only).  Next, add ISPM to the `Path` in your environment variables.

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\Users\YOUR_USERNAME\AppData\Local\Programs\Intel Simics Package Manager`,
   replacing `YOUR_USERNAME` with your Windows user account name.
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.

Close your terminal and open a new one. Run `ispm.exe --version` and ensure no error
occurs.

Next, install the downloaded SIMICS packages. Run the following, replacing VERSION with
the version in your downloaded filename:

```powershell
mkdir ~/simics
ispm.exe settings install-dir ~/simics
ispm.exe packages --install-bundle ~/Downloads/simics-6-VERSION-win64.ispm `
    --non-interactive
```

You may be prompted to accept certificates, choose `Y`.

## Build TSFFS

Clone TSFFS to your system (anywhere you like) and build with:

```powershell
git clone https://github.com/intel/tsffs
cd tsffs
ispm.exe projects $(pwd) --create --non-interactive --ignore-existing-files
./bin/project-setup.bat --mingw-dir C:\MinGW\ --ignore-existing-files --force
cargo -Zscript build.rs
```

Once built, install TSFFS to your SIMICS installation with:

```powershell
ispm.exe packages -i win64/packages/simics-pkg-31337-6.0.1-win64.ispm --non-interactive --trust-insecure-packages
```

## Test TSFFS

We can test TSFFS by creating a new project with our minimal test case, a UEFI boot
disk, and the same fuzz script used in the Linux docker example in the
[README](../README.md). Run the following from the root of this repository:

```powershell
mkdir $env:TEMP\TSFFS-Windows
ispm.exe projects $env:TEMP\TSFFS-Windows\ --create
cp examples\docker-example\fuzz.simics $env:TEMP\TSFFS-Windows\
cp modules\tsffs\tests\targets\minimal-x86_64\* $env:TEMP\TSFFS-Windows\
cp modules\tsffs\tests\rsrc\minimal_boot_disk.craff $env:TEMP\TSFFS-Windows\
cp harness\tsffs-gcc-x86_64.h $env:TEMP\TSFFS-Windows\
cd $env:TEMP\TSFFS-Windows
./simics ./fuzz.simics
```

## Set Up For Local Development

End users can skip this step, it is only necessary if you will be developing the fuzzer.

If you want to develop TSFFS locally, it is helpful to be able to run normal `cargo`
commands to build, run clippy and rust analyzer, and so forth.

To set up your environment for local development, note the installed SIMICS base version
you would like to target. For example, SIMICS 6.0.169. For local development, it is
generally best to pick the most recent installed version. You can print the latest
version you have installed by running (`jq` can be installed with `winget install
stedolan.jq`):

```sh
ispm packages --list-installed --json | jq -r '[ .installedPackages[] | select(.pkgNumber == 1000) ] | ([ .[].version ] | max_by(split(".") | map(tonumber))) as $m | first(first(.[]|select(.version == $m)).paths[0])'
```

On the author's system, for example, this prints:

```txt
C:\Users\YOUR_USERNAME\simics\simics-6.0.169
```

Add this path in the `[env]` section of `.cargo/config.toml` as the variable
`SIMICS_BASE` in your local TSFFS repository. Using this path, `.cargo/config.toml`
would look like:

```toml
[env]
SIMICS_BASE = "C:\Users\YOUR_USERNAME\simics\simics-6.0.169"
```

This lets `cargo` find your SIMICS installation, and it uses several fallback methods to
find the SIMICS libraries to link with. Paths to the libraries are provided via the
SIMICS Makefile system, which is used by the `./build.rs` script above, hence this step
is only needed for local development.

Finally, check that your configuration is correct by running:

```sh
cargo clippy
```

The process should complete without error.

## Troubleshooting

### I Already Have A MinGW Installation

If you already have a MinGW-w64 installation elsewhere, and you do not want to reinstall
it to `C:\MinGW`, edit `compiler.mk` and point `CC=` and `CXX=` at your MinGW `gcc.exe`
and `g++.exe` binaries, respectively, or change the location passed with the
`--mingw-dir` option in [the build step](#build-tsffs).

### Command is Unrecognized

If PowerShell complains that any command above is not recognized, take the following
steps:

1. Run `echo $env:PATH` and ensure the directory containing the command you are trying
   to run is present, add it to your `Path` environment variable if it is absent.
2. Close your terminal and open a new one. The `Path` variable is only reloaded on new
   terminal sessions