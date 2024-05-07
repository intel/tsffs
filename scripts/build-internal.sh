
#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# NOTE: This script requires the Intel version of `ispm` to be installed and available
# on the PATH. We do not download a new copy of it because despite its public
# unavailability, we want to be able to keep this script in the public repository
# without revealing any internal URLs. It will *not* work correctly with the public
# version of ISPM unless you are an Intel or Wind River Simics customer.

set -e

LLD_URL="https://releases.llvm.org/5.0.2/lld-5.0.2.src.tar.xz"
CFE_URL="https://releases.llvm.org/5.0.2/cfe-5.0.2.src.tar.xz"
LLVM_SRC_URL="https://releases.llvm.org/5.0.2/llvm-5.0.2.src.tar.xz"
MAKE_SRC_URL="https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz"
RUST_URL="https://static.rust-lang.org/dist/rust-nightly-x86_64-unknown-linux-gnu.tar.xz"
CMAKE_URL="https://github.com/Kitware/CMake/releases/download/v3.28.0-rc5/cmake-3.28.0-rc5-linux-x86_64.tar.gz"
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/../"
BUILDER_DIR="${ROOT_DIR}/.github/builder/"
IMAGE_NAME="tsffs-builder-internal"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${BUILDER_DIR}/rsrc"

ISPM=$(which ispm)
ISPM_DIR=$(dirname "${ISPM}")

if [ ! -d "${BUILDER_DIR}/rsrc/ispm" ]; then
    echo "ISPM not found. Copying local installation..."
    cp -a "${ISPM_DIR}" "${BUILDER_DIR}/rsrc/ispm"
fi

if [ ! -d "${BUILDER_DIR}/rsrc/simics" ]; then
    echo "Simics packages not found. Installing..."
    mkdir -p "${BUILDER_DIR}/rsrc/simics"
    ispm packages --install-dir "${BUILDER_DIR}/rsrc/simics" -i \
        1000-latest \
        1020-latest \
        1030-latest \
        1031-latest \
        2050-latest \
        2053-latest \
        2096-latest \
        4094-latest \
        6010-latest \
        7801-latest \
        8112-latest \
        8126-latest \
        8144-latest \
        --non-interactive
fi

if [ ! -f "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" ]; then
    echo "LLD not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" \
        "${LLD_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" ]; then
    echo "CFE not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" \
        "${CFE_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" ]; then
    echo "LLVM not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" \
        "${LLVM_SRC_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" ]; then
    echo "Make not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" \
        "${MAKE_SRC_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz" ]; then
    echo "rust not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz" \
        "${RUST_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/cmake-3.28.0-rc5-linux-x86_64.tar.gz" ]; then
    echo "CMake not found. Downloading..."
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/cmake-3.28.0-rc5-linux-x86_64.tar.gz" \
        "${CMAKE_URL}"
fi

if [ ! -d "${BUILDER_DIR}/rsrc/rpms" ]; then
    echo "RPM dependencies not found. Downloading..."
    # NOTE: This may stop working at some point, as Fedora 20 is EOL. Therefore, we download the
    # packages with the expectation that we will provide them separately if they are no longer
    # available.
    docker run -v "${BUILDER_DIR}/rsrc/rpms:/rpms" fedora:20 bash -c \
        'yum -y update && yum install --downloadonly --downloaddir=/rpms coreutils gcc gcc-c++ make which && chmod -R 755 /rpms/'
fi

unset SIMICS_BASE
docker build \
    -t "${IMAGE_NAME}" -f "${BUILDER_DIR}/Dockerfile-internal" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
mkdir -p "${ROOT_DIR}/packages-internal"
docker cp "${CONTAINER_NAME}:/packages-internal" "${ROOT_DIR}/"
docker rm -f "${CONTAINER_NAME}"