# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0
# hadolint global ignore=DL3041,DL3040

FROM fedora:38

# Download links can be obtained from:
# https://lemcenter.intel.com/productDownload/?Product=256660e5-a404-4390-b436-f64324d94959
ENV PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/simics-6-packages-2024-05-linux64.ispm"
ENV PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
ENV PUBLIC_SIMICS_PACKAGE_VERSION_1000="6.0.185"
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
    curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain nightly


WORKDIR /workspace

# Download and install public SIMICS. This installs all the public packages as well as the
# ispm SIMICS package and project manager. ISPM will be on the path due to the ENV command
# above
RUN mkdir -p /workspace/simics/ispm/ && \
    curl --noproxy '*.intel.com' -L -o /workspace/simics/ispm.tar.gz "${PUBLIC_SIMICS_ISPM_URL}" && \
    curl --noproxy '*.intel.com' -L -o /workspace/simics/simics.ispm "${PUBLIC_SIMICS_PKGS_URL}" && \
    tar -C /workspace/simics/ispm --strip-components=1 \
        -xf /workspace/simics/ispm.tar.gz && \
    ispm settings install-dir /workspace/simics && \
    ispm packages --install-bundle /workspace/simics/simics.ispm --non-interactive \
        --trust-insecure-packages && \
    rm /workspace/simics/ispm.tar.gz /workspace/simics/simics.ispm && \
    rm -rf /workspace/simics-6-packages/

# Copy the local repository into the workspace
COPY . /workspace/tsffs/

WORKDIR /workspace/tsffs/

# Build the project by initializing it as a project associated with the local SIMICS installation
# and building the module using the build script. Then, install the built TSFFS SIMICS
# package into the local SIMICS installation for use.
RUN cargo install --path simics-rs/cargo-simics-build && \
    cargo simics-build -r && \
    ispm packages \
        -i target/release/*-linux64.ispm \
        --non-interactive --trust-insecure-packages

WORKDIR /workspace/projects/example/

RUN dnf -y update && \
    dnf -y install rizin

RUN ispm projects /workspace/projects/example/ --create --ignore-existing-files --non-interactive && \
    cp /workspace/tsffs/examples/manual-example/* . && \
    ninja && \
    rizin -e 'asm.var=false' -e 'asm.lines=false' -qqc 'idp test.pdb; aaaa; sf pdb.test.Check; pd 1 ~[0]:-1;' test.efi > START_ADDRESS && \
    rizin -e 'asm.var=false' -e 'asm.lines=false' -qqc 'idp test.pdb; aaaa; sf pdb.test.Check; pdf  ~[0]:-1;' test.efi > STOP_ADDRESS

RUN echo 'echo "To run the demo, run ./simics -no-gui --no-win fuzz.simics"' >> /root/.bashrc

