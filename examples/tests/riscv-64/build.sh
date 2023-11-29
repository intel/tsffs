#!/bin/bash

# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Build the HelloWorld.efi module and copy it into the resource directory for the example
# this only needs to be run if you want to modify the source code for the HelloWorld.efi module,
# otherwise, the EFI is included in the source tree for ease of use

if [ -z "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE must be set"
    exit 1
fi

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="buildroot-build-tsffs-gcc-riscv64-test"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

pushd "${SCRIPT_DIR}" || exit 1

cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-riscv64.h" "${SCRIPT_DIR}/tsffs-gcc-riscv64.h"
cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-riscv64.h" "${SCRIPT_DIR}/test-kernel-modules/package/kernel-modules/test-mod/tsffs-gcc-riscv64.h"
mkdir -p "${SCRIPT_DIR}/targets/risc-v-simple/images/linux/"

if [ ! -e "${SCRIPT_DIR}/images.tar.gz" ] \
    || [ ! -e "${SCRIPT_DIR}/test" ] \
    || [ ! -e "${SCRIPT_DIR}/test-mod" ] \
    || [ ! -e "${SCRIPT_DIR}/test-mod-userspace" ] \
    || [ ! -e "${SCRIPT_DIR}/test-mod.ko" ]; then
    docker build -t "${IMAGE_NAME}" -f "Dockerfile" .
    docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
    docker cp \
        "${CONTAINER_NAME}:/buildroot/images.tar.gz" \
        "${SCRIPT_DIR}/images.tar.gz"
    docker cp \
        "${CONTAINER_NAME}:/test/usr/test" \
        "${SCRIPT_DIR}/test"
    docker cp \
        "${CONTAINER_NAME}:/test/usr/test-mod" \
        "${SCRIPT_DIR}/test-mod"
    docker cp \
        "${CONTAINER_NAME}:/test/usr/test-mod-userspace" \
        "${SCRIPT_DIR}/test-mod-userspace"
    docker cp \
        "${CONTAINER_NAME}:/test/output/test-mod-1.0/test-mod.ko"\
        "${SCRIPT_DIR}/test-mod.ko"
    docker rm -f "${CONTAINER_NAME}"
fi

tar -C "${SCRIPT_DIR}/targets/risc-v-simple/images/linux/" -xvf images.tar.gz
rm images.tar.gz

dd if=/dev/zero of=test.fs bs=1024 count=131072
mkfs.fat test.fs
mcopy -i test.fs test-mod-userspace ::test-mod-userspace
mcopy -i test.fs test-mod ::test-mod
mcopy -i test.fs test ::test
mcopy -i test.fs test-mod.ko ::test-mod.ko
"${SIMICS_BASE}/linux64/bin/craff" -o test.fs.craff test.fs
