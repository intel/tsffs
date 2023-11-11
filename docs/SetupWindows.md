# TSFF Setup (Windows)

This guide will walk you through installing build dependencies, building, and
installing TSFFS into your SIMICS installation on Windows.

- [TSFF Setup (Windows)](#tsff-setup-windows)
  - [Install System Dependencies](#install-system-dependencies)
    - [Git](#git)
    - [7-Zip](#7-zip)
    - [Install MinGW-w64](#install-mingw-w64)
    - [Install Rust](#install-rust)
    - [Install SIMICS](#install-simics)
  - [Build TSFFS](#build-tsffs)
  - [Test TSFFS](#test-tsffs)
  - [Troubleshooting](#troubleshooting)
    - [I already have a MinGW installation!](#i-already-have-a-mingw-installation)

## Install System Dependencies

### Git

Download and install Git for Windows from the [git
website](https://git-scm.com/download/win). The default options are acceptable.

### 7-Zip

Download and install 7-Zip from the [website](https://www.7-zip.org/).

### Install MinGW-w64

If you already have a MinGW-w64 installation, you can skip this step.

Download the MinGW archive from [winlibs.com](https://winlibs.com/#download-release).
Select the UCRT runtime *with* POSIX threads and LLVM/Clang/LLD/LLDB. Select the Win64
7-Zip archive, or use the [direct
link](https://github.com/brechtsanders/winlibs_mingw/releases/download/13.2.0-16.0.6-11.0.0-ucrt-r1/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z)
to download an install the tested MinGW version (LLVM/Clang/LLD/LLDB+UCRT+POSIX, GCC
13.2.0).
 Once downloaded, right-click the archive file, select `7-Zip: Extract Files`
and
 extract to `C:\MinGW\`.

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
ispm.exe packages --install-bundle ~/Downloads/simics-6-VERSION-win64.ispm --non-interactive
```

You may be prompted to accept certificates, choose `Y`.

## Build TSFFS

Clone TSFFS to your system (anywhere you like) and build with:

```powershell
git clone https://github.com/intel/tsffs
cd tsffs
ispm.exe projects $(pwd) --create --non-interactive --ignore-existing-files
cargo -Zscript build.rs
```

Once built, install TSFFS to your SIMICS installation with:

```powershell
ispm.exe packages -i win64/packages/simics-pkg-31337-6.0.0-win64.ispm --non-interactive --trust-insecure-packages
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

## Troubleshooting

### I already have a MinGW installation!

If you already have a MinGW installation elsewhere, and you do not want to reinstall
it to `C:\MinGW`, edit `compiler.mk` and point `CC=` and `CXX=` at your MinGW `gcc.exe`
and `g++.exe` binaries, respectively.