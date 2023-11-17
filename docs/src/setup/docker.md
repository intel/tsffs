# Setup (Docker)

Setting up TSFFS using Docker is the recommended way to use TSFFS externally to Intel,
and is the best way for all new users of TSFFS to get familiar with the build process,
tools and configurations available.

Setting up and using `tsffs` using the provided Dockerfile is only a few simple steps:

1. Install Docker following the directions for your OS from
    [docker.com](https://docs.docker.com/engine/install/).
2. Clone this repository: `git clone https://github.com/intel/tsffs/ && cd tsffs`
3. Build the container (this will take some time): `docker build -t tsffs .`
4. Run the container: `docker run -it tsffs`

The provided base container will prompt you to run the included sample. Feel free to
customize the provided script and target -- any non-platform-specific UEFI application
can be fuzzed with this container!