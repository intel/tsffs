# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0
# hadolint global ignore=DL3041,DL3040

# allow to override Docker client proxies if necessary
ARG http_proxy
ARG https_proxy
ARG no_proxy
# Download links can be obtained from:
# https://lemcenter.intel.com/productDownload/?Product=256660e5-a404-4390-b436-f64324d94959
ARG PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/simics-6-packages-2024-05-linux64.ispm"
ARG PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
ARG PUBLIC_SIMICS_PACKAGE_VERSION_1000="6.0.185"
ARG USER_UID=1000
ARG USERNAME=vscode

FROM fedora:42@sha256:b3d16134560afa00d7cc2a9e4967eb5b954512805f3fe27d8e70bbed078e22ea AS create-user
# redeclare ARGs
ARG http_proxy
ARG https_proxy
ARG no_proxy
ARG USER_UID
ARG USERNAME

# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
# Update system packages
dnf -y update

# create group for developers
groupadd dev
# Create group and user with a home at /home/vscode
useradd \
      --create-home    \
      --uid $USER_UID \
      --user-group     \
      --groups dev \
      --shell /bin/bash \
      $USERNAME
echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME
sudo -E -u $USERNAME bash -c 'curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain none'
EOF

FROM create-user AS tsffs-dev
# redeclare ARGs
ARG http_proxy
ARG https_proxy
ARG no_proxy
ARG PUBLIC_SIMICS_PKGS_URL
ARG PUBLIC_SIMICS_ISPM_URL
ARG PUBLIC_SIMICS_PACKAGE_VERSION_1000
ARG USER_UID
ARG USERNAME
ENV SIMICS_BASE="/workspace/simics/simics-${PUBLIC_SIMICS_PACKAGE_VERSION_1000}/"
# Add cargo and ispm to the path
ENV PATH="/home/${USERNAME}/.cargo/bin:/workspace/simics/ispm:${PATH}"

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

# Clean up package manager cache
dnf clean all
rm -rf /var/cache/dnf/* /tmp/* /var/tmp/*
EOF


WORKDIR /workspace

# install Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain none

# Download and install public SIMICS. This installs all the public packages as well as the
# ispm SIMICS package and project manager. ISPM will be on the path due to the ENV command
# above
# hadolint ignore=DL3004,SC3009
RUN <<EOF
set -e
mkdir -p /workspace/simics/ispm/

# Download SIMICS components
curl -L -o /workspace/simics/ispm.tar.gz "${PUBLIC_SIMICS_ISPM_URL}"
curl -L -o /workspace/simics/simics.ispm "${PUBLIC_SIMICS_PKGS_URL}"

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
COPY --chown=vscode:dev . /workspace/tsffs/

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


RUN <<EOF
set -e
# set perms root:dev and set permissions for dev group members
chown -R root:dev /workspace
chmod -R 775 /workspace
# copy ISPM config to vscode user
cp -r "/root/.config" "/home/${USERNAME}/.config"
chown -R "${USERNAME}:dev" "/home/${USERNAME}/.config"
EOF

USER vscode

WORKDIR /workspace/tsffs

FROM create-user AS tsffs-prod
# redeclare ARGs
ARG PUBLIC_SIMICS_PKGS_URL
ARG PUBLIC_SIMICS_ISPM_URL
ARG PUBLIC_SIMICS_PACKAGE_VERSION_1000
ARG USERNAME
ENV SIMICS_BASE="/workspace/simics/simics-${PUBLIC_SIMICS_PACKAGE_VERSION_1000}/"
# Add cargo and ispm to the path
ENV PATH="/home/${USERNAME}/.cargo/bin:/workspace/simics/ispm:${PATH}"

COPY --from=tsffs-dev /home/vscode/.bashrc /home/vscode/.bashrc
COPY --from=tsffs-dev --chown=root:dev --chmod=775 /workspace /workspace
COPY --from=tsffs-dev --chown=vscode:vscode ["/root/.config/Intel Simics Package Manager/", "/home/vscode/.config/Intel Simics Package Manager/"]
# remove tsffs
RUN rm -r /workspace/tsffs
# fix perms
RUN chmod 775 /workspace

USER vscode
RUN echo 'echo "To run the demo, run ./simics -no-gui --no-win fuzz.simics"' >> "/home/${USERNAME}/.bashrc"
WORKDIR /workspace/projects/example
