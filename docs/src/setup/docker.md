# Setup (Docker)

Setting up TSFFS using Docker is the recommended way to use TSFFS externally to Intel,
and is the best way for all new users of TSFFS to get familiar with the build process,
tools and configurations available.

## For In-Docker Use

If you intend to use TSFFS inside a container, use the `Dockerfile` in the repository.

Setting up and using `tsffs` using the provided Dockerfile is only a few simple steps:

1. Install Docker following the directions for your OS from
    [docker.com](https://docs.docker.com/engine/install/).
2. Clone this repository: `git clone https://github.com/intel/tsffs/ && cd tsffs`
3. Build the container (this will take some time): `docker build -t tsffs .`
4. Run the container: `docker run -it tsffs`

The provided base container will prompt you to run the included sample. Feel free to
customize the provided script and target -- any non-platform-specific UEFI application
can be fuzzed with this container!

## For Out-Of-Docker Use

### Install ISPM

First, you'll need to install ISPM. External users can install it from the public
release:

```sh
curl --noproxy '*.intel.com' -L -o $HOME/Downloads/ispm.tar.gz \
    "https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
mkdir -p $HOME/simics/ispm/
tar -C $HOME/simics/ispm --strip-components=1 -xf $HOME/Downloads/ispm.tar.gz
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

### Build and Install TSFFS

If you want to use a container to *build* TSFFS, but you want to run it on your own
machine, you can run the build script to build TSFFS in a container, then install it
with ISPM.

```sh
./scripts/build.sh
```

This script will produce a directory `packages` with an ISPM file
`simics-pkg-31337*.ispm`. You can install this package with:

```sh
ispm packages -i packages/*.ispm --trust-insecure-packages
```