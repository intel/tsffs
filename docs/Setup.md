# Setup

Follow these setup steps to prepare this repository to build! If you encounter any
issues during this process, check the troubleshooting section first for common
resolutions.

- [Setup](#setup)
  - [Install Prerequisites](#install-prerequisites)
    - [System Packages](#system-packages)
    - [Rust](#rust)
    - [SIMICS](#simics)
      - [(Optional) Install Simics GUI Dependencies](#optional-install-simics-gui-dependencies)
      - [Download Simics](#download-simics)
      - [Install Simics](#install-simics)
      - [Set up SIMICS\_HOME](#set-up-simics_home)
    - [Docker](#docker)
  - [Build the Fuzzer](#build-the-fuzzer)
  - [Troubleshooting](#troubleshooting)
    - [Troubleshooting Docker Installations](#troubleshooting-docker-installations)
      - [Docker Group Membership](#docker-group-membership)
      - [Docker Proxy Use](#docker-proxy-use)
    - [Troubleshooting Rust Installation](#troubleshooting-rust-installation)

## Install Prerequisites

We need a couple of things before we are ready to build. To set up this workspace,
you'll need `cargo` as well as `simics` with the packages necessary to
run whatever it is you want to run. Here, we'll just install the packages needed to
run the simple samples we provide, but this is where you will want to customize your
installation if necessary.

Docker installation is optional and only needed if you want to build the EDK2 example
targets yourself. The pre-built EFI applications for those examples are provided.

### System Packages

To run SIMICS and TSFFS on your system, as well as follow this setup tutorial, you will
need several packages:

For Ubuntu or Debian, install them with:

```sh
sudo apt-get install build-essential curl git
```

For Fedora or RHEL, install them with:

```sh
sudo dnf install curl gcc g++ make git
```

### Rust

You can install the Rust toolchain (including `cargo`, `rustc`, etc) from
[rustup.rs](https://rustup.rs) by following the instructions there and running:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To check that your Rust install is working, run:

```sh
$ cargo new --bin /tmp/rust-test
$ cargo run --manifest-path /tmp/rust-test/Cargo.toml
Compiling rust-test v0.1.0 (/tmp/rust-test)
Finished dev [unoptimized + debuginfo] target(s) in 0.50s
    Running `/tmp/rust-test/target/debug/rust-test`
Hello, world!
```

If you see the `Hello, world!` message, your Rust installation is complete!

### SIMICS

When building this software, you will need a working SIMICS installation. This document
will walk you through this installation and configuration of this software to utilize
the SIMICS installation.

#### (Optional) Install Simics GUI Dependencies

This step is optional, if you want to use the Simics GUI to install it, you will need
these dependencies.

For Ubuntu or Debian, install them with:

```sh
sudo apt-get install libatk1.0-0 libatk-bridge2.0-0 libcups2 libgtk-3-0 libgbm1 \
    libasound2
```

On Red Hat or Fedora, install them with:

```sh
sudo dnf install atk cups gtk3 mesa-libgbm alsa-lib
```

#### Download Simics

If you only want to test the included samples or you only need to run targets that use
the public Simics packages, you can download Simics from the external intel site on the
[public release page](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html).

You will need to download both files (for this section, replace the version with the
version you see on the download page):

- `intel-simics-package-manager-1.7.3-linux64.tar.gz`
- `simics-6-packages-2023-31-linux64.ispm`

In this case, we'll assume you have downloaded both files to the `${HOME}/Downloads`
directory, which you can do by running:

```sh
mkdir -p "${HOME}/Downloads"
curl -L -o "${HOME}/Downloads/intel-simics-package-manager-1.7.3-linux64.tar.gz" \
  https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/intel-simics-package-manager-1.7.5-linux64.tar.gz
curl -L -o "${HOME}/Downloads/simics-6-packages-2023-31-linux64.ipsm" \
  https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/simics-6-packages-2023-31-linux64.ispm \
```

#### Install Simics

Assuming the two download locations above, we will install Simics to `${HOME}/simics`.

```sh
mkdir -p "${HOME}/simics/ispm"
tar -C "${HOME}/simics/ispm" --strip-components=1 \
  -xvf ~/Downloads/intel-simics-package-manager-1.7.5-linux64.tar.gz
"${HOME}/simics/ispm/ispm" packages \
    --install-dir "${HOME}/simics" \
    --install-bundle ~/Downloads/simics-6-packages-2023-31-linux64.ispm \
    --non-interactive
```

#### Set up SIMICS_HOME

In the root of this project, create a file `.env` containing a line like the below that
points to your `SIMICS_HOME` directory (the `--install-dir` argument you passed to
`ispm` in the last step).

```sh
SIMICS_HOME=/home/YOUR_USERNAME/simics/
```

You can create the `.env` file with:

```sh
echo "SIMICS_HOME=${HOME}/simics/" > .env
```

### Docker

Docker installation is completely optional, and is only needed to manually build the
example EFI applications. Pre-built applications are provided in this repository, so you
can safely skip this step unless you want to modify them or create your own target EFI
applications.

You can find instructions to install docker at
[docs.docker.com](https://docs.docker.com/engine/install). The instructions vary
slightly by distribution, so be sure to follow the directions for your particular Linux
flavor.

After installing docker, you can test that your installation is working by running:

```sh
$ docker run hello-world
Unable to find image 'hello-world:latest' locally
latest: Pulling from library/hello-world
719385e32844: Pull complete 
Digest: sha256:a13ec89cdf897b3e551bd9f89d499db6ff3a7f44c5b9eb8bca40da20eb4ea1fa
Status: Downloaded newer image for hello-world:latest

Hello from Docker!
This message shows that your installation appears to be working correctly.

To generate this message, Docker took the following steps:
 1. The Docker client contacted the Docker daemon.
 2. The Docker daemon pulled the "hello-world" image from the Docker Hub.
    (amd64)
 3. The Docker daemon created a new container from that image which runs the
    executable that produces the output you are currently reading.
 4. The Docker daemon streamed that output to the Docker client, which sent it
    to your terminal.

To try something more ambitious, you can run an Ubuntu container with:
 $ docker run -it ubuntu bash

Share images, automate workflows, and more with a free Docker ID:
 https://hub.docker.com/

For more examples and ideas, visit:
 https://docs.docker.com/get-started/
```

You should see the message that starts `Hello from Docker!`. If you don't, check
[troubleshooting](#troubleshooting-docker-installations)

## Build the Fuzzer

After installing the prerequisites, you can build the fuzzer by running the command
below in the root of this repository.

```sh
cargo build --features=6.0.169
```
 If the SIMICS 6 packages version you installed
differs from the version shown above, replace the feature version number you see here with the version of SIMICS base you installed. You can figure out what version
that is with `ls "${HOME}/simics" | grep -E 'simics-[0-9]+(\.[0-9]+){2}'`

## Troubleshooting

### Troubleshooting Docker Installations

#### Docker Group Membership

If you get an error like this when trying to run docker commands:

```text
permission denied while trying to connect to the Docker daemon socket at unix:///var/run/docker.sock: Get "http://%2Fvar%2Frun%2Fdocker.sock/v1.24/containers/json": dial unix /var/run/docker.sock: connect: permission denied
```

You need to add your user to the `docker` group by running:

```sh
sudo groupadd docker
sudo usermod -aG docker $USER
```

You'll then need to log out and log back in, or you can run `newgrp docker` to apply
the changes in your running shell. Be aware `newgrp` will not persist changes in other
shells, so logging out and in is recommended.

After adding yourself to the `docker` group, you should be able to run `groups` and
see `docker` on the output line. If you don't, try running the above command with your
username like so `sudo usermod -aG docker YOUR_USERNAME`.

#### Docker Proxy Use

If you need to use a proxy to connect to the internet (for example you are on a VPN)
you may want to follow the directions from
[docker](https://docs.docker.com/config/daemon/systemd/#httphttps-proxy) to direct
docker engine to use a proxy to pull images.

If you need to do this, you will also likely need to follow the directions from
[docker](https://docs.docker.com/network/proxy/) to use a proxy *inside* the image, not
just for pulling images.

### Troubleshooting Rust Installation

If you get an error while installing Rust or once you try to build and run a test that
either a compiler or linker is missing, you are likely missing the `build-essential`
(or the equivalent on your Linux distribution) package.
