# TSFF Setup (Windows)

## Install System Dependencies

### Git

Download and install Git for Windows from the [git
website](https://git-scm.com/download/win). The default options are acceptable.

### 7-Zip

Download and install 7-Zip from the [website](https://www.7-zip.org/).

### Install MinGW

Download the MinGW archive from [winlibs.com](https://winlibs.com/#download-release).
Select the UCRT runtime *with* POSIX threads and LLVM/Clang/LLD/LLDB. Select the Win64
7-Zip archive.

To download GCC 13.2.0, you can use [this direct
link](https://github.com/brechtsanders/winlibs_mingw/releases/download/13.2.0-16.0.6-11.0.0-ucrt-r1/winlibs-x86_64-posix-seh-gcc-13.2.0-llvm-16.0.6-mingw-w64ucrt-11.0.0-r1.7z).
Once downloaded, right-click the archive file and select `7-Zip: Extract Files` and
extract to `C:\Program Files\mingw-w64`.

Next, add MinGW to the `Path` in your environment variables.

1. Open the `Edit the System Environment Variables` control panel option
2. Select `Environment Variables`
3. Highlight `Path` under `User variables for YOUR_USERNAME`
4. Select `Edit...`. A new window will open.
5. Select `New`
6. Type `C:\Program Files\mingw-w64\bin`
7. Select `OK`. The window will close.
8. Select `OK` on the previous window.

Close your terminal and open a new terminal. Run `gcc --version` and ensure
no error occurs.

### Install Rust

Go to [rustup.rs](https://rustup.rs/) and download `rustup-init.exe`. Run the
downloaded executable and follow all default instructions, including the guided
installation of Visual Studio Community. After installation, close your terminal
and open a new terminal. Run `cargo --version` and ensure no error occurs. Then,
install the nightly toolchain with:

```powershell
rustup toolchain install nightly
```

### Install SIMICS

Go to the [SIMICS download
page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
and download:

* `simics-6-packages-VERSION-win64.ispm`
* `intel-simics-package-manager-VERSION-win64.exe`

Run the downloaded `.exe` file to install `ispm` using the default settings (for your
user only).  Next, add ISPM to the `Path` in your
environment variables.

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

Clone TSFFS to your system (anywhere you like) with:

```powershell
git clone https://github.com/intel/tsffs
cd tsffs
```
