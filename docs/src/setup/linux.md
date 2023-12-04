# Setup (Linux)

The easiest way to get started with TSFFS is with our [docker setup](docker.md).

This guide will walk you through local build and installation of the fuzzer instead.
This is recommended for both internal users and external users who want to move beyond
the initial examples.

- [Setup (Linux)](#setup-linux)
  - [Install Local Dependencies](#install-local-dependencies)
  - [Install Rust](#install-rust)
  - [Install SIMICS](#install-simics)
  - [Build TSFFS](#build-tsffs)
  - [Set Up For Local Development](#set-up-for-local-development)

## Install Local Dependencies

The TSFFS fuzzer module, its example cases, and the SIMICS installation process require
several local system dependencies.

For Fedora Linux:

```sh
sudo dnf -y update
sudo dnf -y install clang clang-libs cmake curl dosfstools g++ gcc git glibc-devel \
    glibc-devel.i686 glibc-static glibc-static.i686 gtk3 lld llvm make mtools \
    ninja-build openssl openssl-devel openssl-libs
```

## Install Rust

Rust's official installation instructions can be found at
[rustup.rs](https://rustup.rs). To install Rust with the recommended settings for this
project (including the nightly toolchain), run:

```sh
curl https://sh.rustup.rs -sSf | bash -s -- -y
rustup toolchain install nightly
```

The installer may prompt you to add `source $HOME/.cargo/env` to your shell init file.
You should accept this option if prompted, or otherwise add `cargo` to your path.

Verify that `cargo` is installed in your path with:

```sh
cargo +nightly --version
```

## Install SIMICS

For users of the public distribution of SIMICS, visit the [SIMICS download
page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html),
accept the EULA, and download the following files. Users of internal or commercial
private Wind River or Intel SIMICS should follow internal documentation available
[here](TODO).

* `intel-simics-package-manager-[VERSION].tar.gz`
* `simics-6-packages-[VERSION].ispm`

You can also download via the direct links as shown below. You can download these files
anywhere, we suggest your `Downloads` directory. In subsequent commands, if you downloaded
directly from the download page, replace `ispm.tar.gz` with the full name of the `ispm`
tarball you downloaded, and likewise with `simics-6-packages`.

```sh
curl --noproxy -L -o $HOME/Downloads/ispm.tar.gz \
    "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/intel-simics-package-manager-1.7.5-linux64.tar.gz"
curl --noproxy -L -o $HOME/Downloads/simics-6-packages.ispm \
    "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/simics-6-packages-2023-31-linux64.ispm"
```

Next, we will install SIMICS. Here, we install to `$HOME/simics/` .  We will extract
`ispm` into our install directory. `ispm` is a static electron executable.

```sh
mkdir -p $HOME/simics/ispm/
tar -C $HOME/simics/ispm --strip-components=1 -xvf $HOME/Downloads/ispm.tar.gz
```

Next, we add `$HOME/simics/ispm` to our `PATH` by adding a line to our `.bashrc` or
`.zshrc`.  You need not configure both shells, only configure the shell you plan to use
`ispm` in.

`bash`:

```sh
echo 'PATH="${PATH}:${HOME}/simics/ispm/"' >> $HOME/.bashrc
source $HOME/.bashrc
```

`zsh`:

```sh
echo 'PATH="${PATH}:${HOME}/simics/ispm/"' >> $HOME/.zshrc
source $HOME/.zshrc
```

ISPM is installed. You can check that it is installed and working with:

```sh
ispm --version
```

If ISPM prints its version number, it is installed successfully. With ISPM installed, we
will configure an `install-dir`. This is the directory all downloaded SIMICS packages
will be installed into. Custom-built SIMICS packages, including the TSFFS package, will
be installed here as well.

```sh
ispm settings install-dir $HOME/simics/
```

Now that we have configured our `install-dir`, we will install the ISPM bundle we
downloaded.

```sh
ispm packages --install-bundle $HOME/Downloads/simics-6-packages.ispm --non-interactive
```

ISPM will report any errors it encounters. SIMICS is now installed.

## Build TSFFS

With all dependencies installed, it is time to clone (if you have not already) and build
TSFFS. You can clone `tsffs` anywhere you like, we use the SIMICS directory we already
created. If you already cloned `tsffs`, you can skip this step, just `cd` to the cloned
repository directory.

```sh
git clone https://github.com/intel/tsffs $HOME/simics/tsffs/
cd $HOME/simics/tsffs/
```

The first step to building `tsffs` is to initialize the repository as a SIMICS project
so it is associated with your installed SIMICS packages.

```sh
ispm projects $(pwd) --create --ignore-existing-files --non-interactive
```

Next, we run the `project-setup` script SIMICS installs into the repository for us to
update the project.

```sh
bin/project-setup --force
```

Then, we will build the TSFFS package using the build script. This automates several
steps for us:

* Runs `make` twice to invoke `cargo` with the correct environment to build and generate
  a signed shared object (module) to be loaded into SIMICS
* Creates a `packages-spec.json` file from the TSFFS crate's contents
* Creates module info for the packaging scripts
* Creates a `.ispm` package that can be installed into the local SIMICS installation

```sh
./build.rs
```

This will produce a file `linux64/packages/simics-pkg-31337-6.0.0-linux64.ispm`. We can
then install this package into our local SIMICS installation. This in turn allows us to
add the TSFFS package to our SIMICS projects for use. Note the
`--trust-insecure-packages` flag is required because this package is not built and
signed by the SIMICS team, but by ourselves.

```sh
ispm packages -i linux64/packages/simics-pkg-31337-6.0.0-linux64.ispm \
    --non-interactive --trust-insecure-packages
```

You are now ready to use TSFFS! See the [User Guide](./UserGuide.md) to learn how to
add TSFFS to your SIMICS projects, configure TSFFS, and run fuzzing campaigns.

## Set Up For Local Development


End users can skip this step, it is only necessary if you will be developing the fuzzer.

If you want to develop TSFFS locally, it is helpful to be able to run normal `cargo`
commands to build, run clippy and rust analyzer, and so forth.

To set up your environment for local development, note the installed SIMICS base version
you would like to target. For example, SIMICS 6.0.169. For local development, it is
generally best to pick the most recent installed version. You can print the latest
version you have installed by running (`jq` can be installed with your package manager):

```sh
ispm packages --list-installed --json | jq -r '[ .installedPackages[] | select(.pkgNumber == 1000) ] | ([ .[].version ] | max_by(split(".") | map(tonumber))) as $m | first(first(.[]|select(.version == $m)).paths[0])'
```

On the author's system, for example, this prints:

```txt
/home/YOUR_USERNAME/simics/simics-6.0.169
```

Add this path in the `[env]` section of `.cargo/config.toml` as the variable
`SIMICS_BASE` in your local TSFFS repository. Using this path, `.cargo/config.toml`
would look like:

```toml
[env]
SIMICS_BASE = "/home/YOUR_USERNAME/simics/simics-6.0.169"
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