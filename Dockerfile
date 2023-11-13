# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0
# hadolint global ignore=DL3041,DL3040

FROM fedora:38

# Download links can be obtained from:
# https://lemcenter.intel.com/productDownload/?Product=256660e5-a404-4390-b436-f64324d94959
ARG PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/simics-6-packages-2023-31-linux64.ispm"
ARG PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/intel-simics-package-manager-1.7.5-linux64.tar.gz"
# Add cargo and ispm to the path
ENV PATH="/root/.cargo/bin:/workspace/simics/ispm:${PATH}"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install local dependencies:
# - Libraries and dependencies for SIMICS and ISPM
# - Libraries and dependencies for building a sample UEFI application
# - Tools for creating a CRAFF image to load into a model
# - Python, including checkers/linters
# - Rust (will be on the PATH due to the ENV command above)
RUN dnf -y update && \
    dnf -y install \
        alsa-lib \
        atk \
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
        yamllint && \
    python3 -m pip install --no-cache-dir \
        black==23.10.1 \
        flake8==6.1.0 \
        isort==5.12.0 \
        mypy==1.6.1 \
        pylint==3.0.2 && \
    curl https://sh.rustup.rs -sSf | bash -s -- -y && \
    rustup toolchain install nightly


WORKDIR /workspace

# Download and install public SIMICS. This installs all the public packages as well as the
# ispm SIMICS package and project manager. ISPM will be on the path due to the ENV command
# above
RUN mkdir -p /workspace/simics/ispm/ && \
    curl --noproxy -L -o /workspace/simics/ispm.tar.gz "${PUBLIC_SIMICS_ISPM_URL}" && \
    curl --noproxy -L -o /workspace/simics/simics.ispm "${PUBLIC_SIMICS_PKGS_URL}" && \
    tar -C /workspace/simics/ispm --strip-components=1 \
        -xvf /workspace/simics/ispm.tar.gz && \
    ispm settings install-dir /workspace/simics && \
    ispm packages --install-bundle /workspace/simics/simics.ispm --non-interactive && \
    rm /workspace/simics/ispm.tar.gz /workspace/simics/simics.ispm && \
    rm -rf /workspace/simics-6-packages/

# Copy the local repository into the workspace
COPY . /workspace/tsffs/

WORKDIR /workspace/tsffs/

# Build the project by initializing it as a project associated with the local SIMICS installation
# and building the module using the build script. Then, install the built TSFFS SIMICS
# package into the local SIMICS installation for use.
RUN ispm projects /workspace/tsffs/ --create --ignore-existing-files --non-interactive && \
    bin/project-setup --force && \
    ./build.rs && \
    ispm packages \
        -i /workspace/tsffs/linux64/packages/simics-pkg-31337-6.0.0-linux64.ispm \
        --non-interactive --trust-insecure-packages && \
    make clobber

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
RUN ispm projects /workspace/projects/example/ --create \
    1000-latest \
    2096-latest \
    8112-latest \
    1030-latest \
    31337-latest --ignore-existing-files --non-interactive && \
    cp /workspace/tsffs/examples/docker-example/fuzz.simics /workspace/projects/example/ && \
    cp /workspace/tsffs/modules/tsffs/tests/rsrc/minimal_boot_disk.craff /workspace/projects/example/ && \
    cp /workspace/tsffs/modules/tsffs/tests/targets/minimal-x86_64/* /workspace/projects/example/ && \
    cp /workspace/tsffs/harness/tsffs-gcc-x86_64.h /workspace/projects/example/ && \
    ninja

RUN echo 'echo "To run the demo, run ./simics -no-gui --no-win fuzz.simics"' >> /root/.bashrc



