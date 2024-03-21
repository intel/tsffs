# Setup (Windows)

This guide will walk you through installing build dependencies, building, and installing
TSFFS into your SIMICS installation on Windows. All console commands in this document
should be run in PowerShell (the default shell on recent Windows versions).

- [Setup (Windows)](#setup-windows)
  - [Install System Dependencies](#install-system-dependencies)
    - [Update WinGet](#update-winget)
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

### Update WinGet

```powershell
winget source update
```

If you see the following output (with the `Cancelled` message):

```powershell
winget source update
Updating all sources...
Updating source: msstore...
Done
Updating source: winget...
                                  0%
Cancelled
```

Then run the following to manually update the winget source:

```powershell
Invoke-WebRequest -Uri https://cdn.winget.microsoft.com/cache/source.msix -OutFile ~/Downloads/source.msix
Add-AppxPackage ~/Downloads/source/msix
winget source update winget
```

You should now see the correct output:

```txt
Updating source: winget...
Done
```

### Git

Install Git with WinGet and add it to your `PATH`:

```powershell
winget install --id Git.Git -e --source winget
$env:Path += ";C:\Program Files\Git\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\Git\bin", "Machine")
```

Alternatively, you can add Git to the PATH using the GUI.

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\Users\Program Files\Git\bin`.
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.
9. Close your terminal and open a new one. Run `git -h` and ensure no error
occurs.

Alternatively you can also download and install Git for Windows from the [git
website](https://git-scm.com/download/win). The default options are acceptable. In a new
powershell terminal, the command `git -h` should complete with no error.


### 7-Zip

Install 7-zip and add it to your `PATH`:

```powershell
winget install --id 7zip.7zip -e --source winget
$env:Path += ";C:\Program Files\7-Zip"
[Environment]::SetEnvironmentVariable("Path", $env:Path + "C:\Program Files\7-Zip", "Machine")
```

Alternatively, you can add 7-Zip to the PATH using the GUI.

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\Users\Program Files\7-Zip`.
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.
9. Close your terminal and open a new one. Run `7z -h` and ensure no error
occurs.

Alternatively you can also download and install 7-Zip from the
[website](https://www.7-zip.org/). In a new powershell terminal, the command `7z -h`
should complete with no error.

### Install MinGW-w64

If you already have a MinGW-w64 installation, you can skip this step and see the
[section](#i-already-have-a-mingw-installation) on using an existing installation.

Download the MinGW archive from [winlibs.com](https://winlibs.com/#download-release).
Select the 7-Zip archive for Win64 with UCRT runtime *and* POSIX threads and
LLVM/Clang/LLD/LLDB:

```powershell
$ProgressPreference = 'SilentlyContinue'
Invoke-WebRequest -Uri "https://github.com/brechtsanders/winlibs_mingw/releases/download/13.2.0-16.0.6-11.0.0-ucrt-r1/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z" -OutFile ~/Downloads/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z
```

Alternatively you can also use the [direct
link](https://github.com/brechtsanders/winlibs_mingw/releases/download/13.2.0-16.0.6-11.0.0-ucrt-r1/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z)
to download and install the tested MinGW version (LLVM/Clang/LLD/LLDB+UCRT+POSIX, GCC
13.2.0).

Once downloaded, run the following commands (assuming the file is downloaded to
`~/Downloads`, substitute the correct path if not) to extract the file to the MinGW
directory. You may prefer to right-click the `7z` file, choose `7-Zip: Extract Files`,
and type `C:\MinGW\` as the destination instead of running these commands.

```powershell
7z x -omingw-w64/ $HOME/Downloads/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z 
mv mingw-w64/mingw64/ C:/MinGW/
```

Next, add MinGW to the `Path` in your environment variables:

```powershell
$env:Path += ";C:\MinGW\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path + "C:\MinGW\bin", "Machine")
```

Alternatively, you can use the GUI:

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\MinGW\bin`
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.
9. Close your terminal and open a new terminal. Run `gcc --version` and ensure no error
occurs.

### Install Rust

Download `rustup-init.exe`:

```powershell
$ProgressPreference = 'SilentlyContinue'
Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $HOME/Downloads/rustup-init.exe
```

You can also go to [rustup.rs](https://rustup.rs/) and download `rustup-init.exe`. Run
`rustup-init.exe` with the following arguments:

```powershell
~\Downloads\rustup-init.exe --default-toolchain nightly --default-host x86_64-pc-windows-gnu -y
```

After installation, close your terminal and open a new terminal as prompted. Run `cargo
--verison`. Ensure the version ends with `-nightly` (this is required to run the build
script).

### Install SIMICS

Download SIMICS:

```powershell
$ProgressPreference = 'SilentlyContinue'
Invoke-WebRequest -Uri "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/simics-6-packages-2023-31-win64.ispm" -OutFile $HOME/Downloads/simics-6-packages.ispm
Invoke-WebRequest -Uri "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/intel-simics-package-manager-1.7.5-win64.exe" -OutFile $HOME/Downloads/intel-simics-package-manager-win64.exe
```

Alternatively, you can also go to the [SIMICS download
page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
and download:

* `simics-6-packages-VERSION-win64.ispm`
* `intel-simics-package-manager-VERSION-win64.exe`

Run the downloaded `.exe` file with the command below in an elevated PowerShell prompt
to install `ispm` using the default settings (for your user only, note that if you
downloaded manually, you should type the name of the file you downloaded).

```powershell
~/Downloads/intel-simics-package-manager-win64.exe /S | Out-Null
```

Next, add ISPM to the `Path` in your environment variables:

```powershell
$env:Path += ";$HOME\AppData\Local\Programs\Intel Simics Package Manager"
[Environment]::SetEnvironmentVariable("Path", $env:Path + "$HOME\AppData\Local\Programs\Intel Simics Package manager", "User")
```

Alternatively, you can add ISPM to the PATH using the GUI:

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\Users\YOUR_USERNAME\AppData\Local\Programs\Intel Simics Package Manager`,
   replacing `YOUR_USERNAME` with your Windows user account name.
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.
9. Close your terminal and open a new one. Run `ispm.exe --version` and ensure no error
occurs.

Next, install the downloaded SIMICS packages. Run the following, replacing VERSION with
the version in your downloaded filename:

```powershell
mkdir ~/simics
ispm.exe settings install-dir ~/simics
ispm.exe packages --install-bundle ~/Downloads/simics-6-VERSION-win64.ispm `
    --non-interactive --trust-insecure-packages
```

You may be prompted to accept certificates, choose `Y`.

## Build TSFFS

Clone TSFFS to your system (anywhere you like) and build with:

```powershell
git clone https://github.com/intel/tsffs
cd tsffs
cargo install --path simics-rs/cargo-simics-build
cargo simics-build -r
```

Once built, install TSFFS to your SIMICS installation with:

```powershell
ispm.exe packages -i target/release/*-win64.ispm --non-interactive --trust-insecure-packages
```

## Test TSFFS

We can test TSFFS by creating a new project with our minimal test case, a UEFI boot
disk, and the same fuzz script used in the Linux docker example in the
[README](../README.md). Run the following from the root of this repository:

```powershell
mkdir $env:TEMP\TSFFS-Windows
ispm.exe projects $env:TEMP\TSFFS-Windows\ --create
cp examples\docker-example\fuzz.simics $env:TEMP\TSFFS-Windows\
cp tests\rsrc\x86_64-uefi\* $env:TEMP\TSFFS-Windows\
cp tests\rsrc\minimal_boot_disk.craff $env:TEMP\TSFFS-Windows\
cp harness\tsffs.h $env:TEMP\TSFFS-Windows\
cd $env:TEMP\TSFFS-Windows
./simics --no-win ./fuzz.simics
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
C:\Users\YOUR_USERNAME\simics\simics-6.0.185
```

Add this path in the `[env]` section of `.cargo/config.toml` as the variable
`SIMICS_BASE` in your local TSFFS repository. Using this path, `.cargo/config.toml`
would look like:

```toml
[env]
SIMICS_BASE = "C:\Users\YOUR_USERNAME\simics\simics-6.0.185"
```

This lets `cargo` find your SIMICS installation, and it uses several fallback methods to
find the SIMICS libraries to link with.

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
