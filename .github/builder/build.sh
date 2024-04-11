#!/bin/bash

# NOTE: Do not just copy-paste scripts/build.sh!

LLD_URL="https://releases.llvm.org/5.0.2/lld-5.0.2.src.tar.xz"
CFE_URL="https://releases.llvm.org/5.0.2/cfe-5.0.2.src.tar.xz"
LLVM_SRC_URL="https://releases.llvm.org/5.0.2/llvm-5.0.2.src.tar.xz"
MAKE_SRC_URL="https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz"
RUSTUP_INIT_URL="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
CMAKE_URL="https://github.com/Kitware/CMake/releases/download/v3.28.0-rc5/cmake-3.28.0-rc5-linux-x86_64.tar.gz"
PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/simics-6-packages-2024-05-linux64.ispm"
PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
PUBLIC_SIMICS_PACKAGE_VERSION_1000="6.0.185"
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/../../"
BUILDER_DIR="${ROOT_DIR}/.github/builder/"
IMAGE_NAME="tsffs-builder"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${BUILDER_DIR}/rsrc"

if [ ! -f "${BUILDER_DIR}/rsrc/ispm.tar.gz" ]; then

    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/ispm.tar.gz" \
        "${PUBLIC_SIMICS_ISPM_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/simics.ispm" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/simics.ispm" \
        "${PUBLIC_SIMICS_PKGS_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" \
        "${LLD_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" \
        "${CFE_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" \
        "${LLVM_SRC_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" \
        "${MAKE_SRC_URL}"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/rustup-init" ]; then
    curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/rustup-init" \
        "${RUSTUP_INIT_URL}"
    chmod +x "${BUILDER_DIR}/rsrc/rustup-init"
fi

if [ ! -f "${BUILDER_DIR}/rsrc/cmake-3.28.0-rc5-linux-x86_64.tar.gz" ]; then
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
    --build-arg \
    "PUBLIC_SIMICS_PACKAGE_VERSION_1000=${PUBLIC_SIMICS_PACKAGE_VERSION_1000}" \
    -t "${IMAGE_NAME}" -f "${BUILDER_DIR}/Dockerfile" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
mkdir -p "${ROOT_DIR}/packages"
docker cp "${CONTAINER_NAME}:/packages" "${ROOT_DIR}/"
docker rm -f "${CONTAINER_NAME}"