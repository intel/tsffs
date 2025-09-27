# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0
# hadolint global ignore=DL3041,DL3040

FROM fedora:42@sha256:f357623dc40edf7803f21b2b954f92417f274a7370f82384ef13c73e08ce1727 AS tsffs-base

# Download links can be obtained from:
# https://lemcenter.intel.com/productDownload/?Product=256660e5-a404-4390-b436-f64324d94959
ARG PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/simics-6-packages-2024-05-linux64.ispm"
ARG PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
ARG PUBLIC_SIMICS_PACKAGE_VERSION_1000="6.0.185"
ENV SIMICS_BASE="/workspace/simics/simics-${PUBLIC_SIMICS_PACKAGE_VERSION_1000}/"
# Add cargo and ispm to the path
ENV PATH="/root/.cargo/bin:/workspace/simics/ispm:${PATH}"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install local dependencies:
# - Libraries and dependencies for SIMICS and ISPM
# - Libraries and dependencies for building a sample UEFI application
# - Tools for creating a CRAFF image to load into a model
# - Python, including checkers/linters
# - Rust (will be on the PATH due to the ENV command above)
# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# Update system packages
dnf -y update

# Install system dependencies
dnf -y install \
    alsa-lib \
    atk \
    awk \
    bash \
    clang \
    clang-libs \
    clang-resource-filesystem \
    clang-tools-extra \
    cmake \
    cups \
    curl \
    dosfstools \
    g++ \
    gcc \
    git \
    git-lfs \
    glibc-devel \
    glibc-devel.i686 \
    glibc-static \
    glibc-static.i686 \
    gtk3 \
    lld \
    lld-devel \
    lld-libs \
    llvm \
    llvm-libs \
    llvm-static \
    make \
    mesa-libgbm \
    mtools \
    ninja-build \
    openssl \
    openssl-devel \
    openssl-libs \
    python3 \
    python3-pip \
    vim \
    yamllint

# Install Python packages
python3 -m pip install --no-cache-dir \
    black==23.10.1 \
    flake8==6.1.0 \
    isort==5.12.0 \
    mypy==1.6.1 \
    pylint==3.0.2

# Install Rust
curl https://sh.rustup.rs -sSf | bash -s -- --default-toolchain none -y

# Clean up package manager cache
dnf clean all
rm -rf /var/cache/dnf/* /tmp/* /var/tmp/*
EOF


WORKDIR /workspace

# Download and install public SIMICS. This installs all the public packages as well as the
# ispm SIMICS package and project manager. ISPM will be on the path due to the ENV command
# above
# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# Create directories
mkdir -p /workspace/simics/ispm/

# Download SIMICS components
curl --noproxy '*.intel.com' -L -o /workspace/simics/ispm.tar.gz "${PUBLIC_SIMICS_ISPM_URL}"
curl --noproxy '*.intel.com' -L -o /workspace/simics/simics.ispm "${PUBLIC_SIMICS_PKGS_URL}"

# Extract and install
tar -C /workspace/simics/ispm --strip-components=1 -xf /workspace/simics/ispm.tar.gz
rm /workspace/simics/ispm.tar.gz

# Configure and install packages
ispm settings install-dir /workspace/simics
ispm packages --install-bundle /workspace/simics/simics.ispm --non-interactive --trust-insecure-packages

# Clean up
rm /workspace/simics/simics.ispm
rm -rf /tmp/* /var/tmp/*
EOF

# Copy the local repository into the workspace
COPY . /workspace/tsffs/

WORKDIR /workspace/tsffs/

# Build the project by initializing it as a project associated with the local SIMICS installation
# and building the module using the build script. Then, install the built TSFFS SIMICS
# package into the local SIMICS installation for use.
RUN <<EOF
set -e
# Install cargo-simics-build
cargo install cargo-simics-build

# Build the project
cargo simics-build -r

# Install the built package
ispm packages -i target/release/*-linux64.ispm --non-interactive --trust-insecure-packages

# Cleanup
cargo clean
EOF

WORKDIR /workspace/projects/example/

# Create an example project with:
# - SIMICS Base (1000)
# - QSP X86 (2096)
# - QSP CPU (8112)
# - Crypto Engine (1030) [only necessary because it is required by Golden Cove]
# - TSFFS Fuzzer (31337)
# - A built EFI application (test.efi) which checks a password and crashes when it gets the
#   password "fuzzing!"
# - A SIMICS script that configures the fuzzer for the example and starts fuzzing it
# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# Create the example project
ispm projects /workspace/projects/example/ --create \
    1000-${PUBLIC_SIMICS_PACKAGE_VERSION_1000} \
    2096-latest \
    8112-latest \
    1030-latest \
    31337-latest --ignore-existing-files --non-interactive

# Copy required files
cp /workspace/tsffs/examples/docker-example/fuzz.simics /workspace/projects/example/
cp /workspace/tsffs/tests/rsrc/minimal_boot_disk.craff /workspace/projects/example/
cp /workspace/tsffs/tests/rsrc/x86_64-uefi/* /workspace/projects/example/
cp /workspace/tsffs/harness/tsffs.h /workspace/projects/example/

# Build the project
ninja
EOF

RUN echo 'echo "To run the demo, run ./simics -no-gui --no-win fuzz.simics"' >> /root/.bashrc

FROM tsffs-base AS tsffs-dev
ARG USER_UID=1000
ARG USERNAME=vscode

# To build and run the dev image:
#   docker build --build-arg USER_UID=$(id -u) --target tsffs-dev -t tsffs:dev .
#   docker run --rm -ti --user vscode -v .:/workspace/tsffs tsffs:dev

# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# create group for developers
groupadd dev
# Create group and user with a home at /home/vscode
useradd \
      --create-home    \
      --uid $USER_UID \
      --user-group     \
      --groups dev \
      --shell /bin/bash \
      $USERNAME        \
 && echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME

# set /workspace/simics permissions to vscode:dev
chown -R vscode:dev /workspace/{simics,projects,tsffs}

# install Rust nightly for the user
sudo -E -u $USERNAME bash -c 'curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain none'

# copy Simics ISPM config
mkdir -p /home/$USERNAME/.config
cp -r "/root/.config/Intel Simics Package Manager/" "/home/$USERNAME/.config/"
chown -R $USERNAME:$USERNAME "/home/$USERNAME/.config/"
EOF

WORKDIR /workspace/tsffs

FROM fedora:42@sha256:f357623dc40edf7803f21b2b954f92417f274a7370f82384ef13c73e08ce1727 AS tsffs-prod

# Install minimal runtime dependencies only
# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# Update system packages
dnf -y update

# Install minimal runtime dependencies
dnf -y install \
    alsa-lib \
    atk \
    bash \
    cups \
    gtk3 \
    mesa-libgbm \
    openssl \
    openssl-libs \
    python3

# Clean up package manager cache
dnf clean all
rm -rf /var/cache/dnf/* /tmp/* /var/tmp/*
EOF

COPY --from=tsffs-base /workspace/projects /workspace/projects
COPY --from=tsffs-base /workspace/simics /workspace/simics
COPY --from=tsffs-base /root/.bashrc /root/.bashrc

WORKDIR /workspace/projects/example
