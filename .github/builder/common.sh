#!/bin/bash

# NOTE: Do not just copy-paste scripts/build.sh!

set -e

download_and_verify_llvm() {
    LLVM_PGP_KEY_URL="https://releases.llvm.org/5.0.2/tstellar-gpg-key.asc"
    LLD_URL="https://releases.llvm.org/5.0.2/lld-5.0.2.src.tar.xz"
    LLD_SIG_URL="https://releases.llvm.org/5.0.2/lld-5.0.2.src.tar.xz.sig"
    CFE_URL="https://releases.llvm.org/5.0.2/cfe-5.0.2.src.tar.xz"
    CFE_SIG_URL="https://releases.llvm.org/5.0.2/cfe-5.0.2.src.tar.xz.sig"
    LLVM_SRC_URL="https://releases.llvm.org/5.0.2/llvm-5.0.2.src.tar.xz"
    LLVM_SRC_SIG_URL="https://releases.llvm.org/5.0.2/llvm-5.0.2.src.tar.xz.sig"

    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi

    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -f "${BUILDER_DIR}/rsrc/llvm-pgp-key.asc" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/llvm-pgp-key.asc" \
            "${LLVM_PGP_KEY_URL}"
        gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/llvm-pgp-key.gpg" --import \
            "${BUILDER_DIR}/rsrc/llvm-pgp-key.asc"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz" \
            "${LLD_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz.sig" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz.sig" \
            "${LLD_SIG_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/llvm-pgp-key.gpg" \
        --verify "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz.sig" \
        "${BUILDER_DIR}/rsrc/lld-5.0.2.src.tar.xz"

    if [ ! -f "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz" \
            "${CFE_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz.sig" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz.sig" \
            "${CFE_SIG_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/llvm-pgp-key.gpg" \
        --verify "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz.sig" \
        "${BUILDER_DIR}/rsrc/cfe-5.0.2.src.tar.xz"

    if [ ! -f "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz" \
            "${LLVM_SRC_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz.sig" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz.sig" \
            "${LLVM_SRC_SIG_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/llvm-pgp-key.gpg" \
        --verify "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz.sig" \
        "${BUILDER_DIR}/rsrc/llvm-5.0.2.src.tar.xz"
}

download_and_verify_make() {
    GNU_GPG_KEYRING_URL="https://ftp.gnu.org/gnu/gnu-keyring.gpg"
    MAKE_SRC_URL="https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz"
    MAKE_SRC_SIG_URL="https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz.sig"

    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi

    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -f "${BUILDER_DIR}/rsrc/gnu-keyring.gpg" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/gnu-keyring.gpg" \
            "${GNU_GPG_KEYRING_URL}"
        gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/gnu-keyring.gpg" --import \
            "${BUILDER_DIR}/rsrc/gnu-keyring.gpg"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz" \
            "${MAKE_SRC_URL}"
    fi
    
    if [ ! -f "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz.sig" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz.sig" \
            "${MAKE_SRC_SIG_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/gnu-keyring.gpg" \
        --verify "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz.sig" \
        "${BUILDER_DIR}/rsrc/make-4.4.1.tar.gz"
}

download_and_verify_rust() {
    RUST_GPG_KEY_URL="https://static.rust-lang.org/rust-key.gpg.ascii"
    RUST_URL="https://static.rust-lang.org/dist/rust-nightly-x86_64-unknown-linux-gnu.tar.xz"
    RUST_SIG_URL="https://static.rust-lang.org/dist/rust-nightly-x86_64-unknown-linux-gnu.tar.xz.asc"

    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi


    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -f "${BUILDER_DIR}/rsrc/rust-key.gpg.ascii" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/rust-key.gpg.ascii" \
            "${RUST_GPG_KEY_URL}"
        gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/rust-key.gpg" --import \
            "${BUILDER_DIR}/rsrc/rust-key.gpg.ascii"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz" \
            "${RUST_URL}"
    fi 

    if [ ! -f "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz.asc" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz.asc" \
            "${RUST_SIG_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/rust-key.gpg" \
        --verify "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz.asc" \
        "${BUILDER_DIR}/rsrc/rust-nightly-x86_64-unknown-linux-gnu.tar.xz"
}

download_and_verify_cmake() {
    CMAKE_URL="https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-linux-x86_64.tar.gz"
    CMAKE_HASHES_URL="https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-SHA-256.txt"
    CMAKE_HASHES_SIG_URL="https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-SHA-256.txt.asc"
    CMAKE_PGP_KEY_URL="https://keyserver.ubuntu.com/pks/lookup?op=get&search=0xcba23971357c2e6590d9efd3ec8fef3a7bfb4eda"

    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi


    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -f "${BUILDER_DIR}/rsrc/cmake-armored-keys.gpg" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cmake-armored-keys.asc" \
            "${CMAKE_PGP_KEY_URL}"
        gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/cmake-armored-keys.gpg" --import \
            "${BUILDER_DIR}/rsrc/cmake-armored-keys.asc"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt" \
            "${CMAKE_HASHES_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt.asc" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt.asc" \
            "${CMAKE_HASHES_SIG_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/cmake-3.29.3-linux-x86_64.tar.gz" ]; then
        curl -L -o "${BUILDER_DIR}/rsrc/cmake-3.29.3-linux-x86_64.tar.gz" \
            "${CMAKE_URL}"
    fi

    gpg --no-default-keyring --keyring "${BUILDER_DIR}/rsrc/cmake-armored-keys.gpg" \
        --verify "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt.asc" \
        "${BUILDER_DIR}/rsrc/cmake-3.29.3-SHA-256.txt"
}

download_and_verify_simics() {
    PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/simics-6-packages-2024-05-linux64.ispm"
    PUBLIC_SIMICS_PKGS_SHA384="90d498e3b2afa54191bf09c5a0dcb9641595150eb9eab8dbaf9101ad6a1e7e892ef5db9637da85d1a4787bd541dea806"
    PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/ead79ef5-28b5-48c7-8d1f-3cde7760798f/intel-simics-package-manager-1.8.3-linux64.tar.gz"
    PUBLIC_SIMICS_ISPM_SHA384="a2c42ea1577e54c4c68e1d6a7d2ad3da3c7298412f008c71fb3b98a1ffddb89f20f96998de9a9d9c20424fe6ae4c9882"

    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi


    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -f "${BUILDER_DIR}/rsrc/ispm.tar.gz" ]; then
        curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/ispm.tar.gz" \
            "${PUBLIC_SIMICS_ISPM_URL}"
    fi

    if [ ! -f "${BUILDER_DIR}/rsrc/simics.ispm" ]; then
        curl --noproxy '*.intel.com' -L -o "${BUILDER_DIR}/rsrc/simics.ispm" \
            "${PUBLIC_SIMICS_PKGS_URL}"
    fi

    sha384sum "${BUILDER_DIR}/rsrc/ispm.tar.gz" | awk '{print $1}' | grep -q "${PUBLIC_SIMICS_ISPM_SHA384}"
    sha384sum "${BUILDER_DIR}/rsrc/simics.ispm" | awk '{print $1}' | grep -q "${PUBLIC_SIMICS_PKGS_SHA384}"
}

download_and_verify_builder_rpms() {
    if [ -z "${BUILDER_DIR}" ]; then
        echo "BUILDER_DIR not set. Exiting..."
        exit 1
    fi

    mkdir -p "${BUILDER_DIR}/rsrc"

    if [ ! -d "${BUILDER_DIR}/rsrc/rpms" ]; then
        echo "RPM dependencies not found. Downloading..."
        # NOTE: This may stop working at some point, as Fedora 20 is EOL. Therefore, we download the
        # packages with the expectation that we will provide them separately if they are no longer
        # available.
        docker run -v "${BUILDER_DIR}/rsrc/rpms:/rpms" fedora:20 bash -c \
            'yum -y update && yum install --downloadonly --downloaddir=/rpms coreutils gcc gcc-c++ make which && chmod -R 755 /rpms/'
    fi
}

download_and_verify_builder_deps() {
    download_and_verify_llvm
    download_and_verify_make
    download_and_verify_rust
    download_and_verify_cmake
    download_and_verify_simics
    download_and_verify_builder_rpms
}