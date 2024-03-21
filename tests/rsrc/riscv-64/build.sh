#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Build the HelloWorld.efi module and copy it into the resource directory for the example
# this only needs to be run if you want to modify the source code for the HelloWorld.efi module,
# otherwise, the EFI is included in the source tree for ease of use

set -e


SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="buildroot-build-tsffs-gcc-riscv64-test"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"
CRAFF="${SCRIPT_DIR}/../../../bin/craff"
CRAFF_FS="${SCRIPT_DIR}/../../../bin/craff-fs"

if [ -n "${SIMICS_BASE}" ]; then
    mkdir -p "${SCRIPT_DIR}/../../../bin"
    cp "${SIMICS_BASE}/linux64/bin/craff"  "${CRAFF}"
    cp "${SIMICS_BASE}/linux64/bin/craff-fs"  "${CRAFF_FS}"
fi

pushd "${SCRIPT_DIR}" || exit 1

cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/tsffs.h"
cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/test-kernel-modules/package/kernel-modules/test-mod/tsffs.h"
mkdir -p "${SCRIPT_DIR}/targets/risc-v-simple/images/linux/"

echo "Building container"
docker buildx build -t "${IMAGE_NAME}" -f "Dockerfile" . > "${SCRIPT_DIR}/build.log" 2>&1 || { tail -n 1000 "${SCRIPT_DIR}/build.log"; exit 1; }
echo "Container build finished"
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
    "${CONTAINER_NAME}:/test/usr/test-mod.ko"\
    "${SCRIPT_DIR}/test-mod.ko"
docker rm -f "${CONTAINER_NAME}"

tar -C "${SCRIPT_DIR}/targets/risc-v-simple/images/linux/" -xf images.tar.gz
rm images.tar.gz

dd if=/dev/zero of=test.fs bs=1024 count=131072
mkfs.fat test.fs
mcopy -i test.fs test-mod-userspace ::test-mod-userspace
mcopy -i test.fs test-mod ::test-mod
mcopy -i test.fs test ::test
mcopy -i test.fs test-mod.ko ::test-mod.ko
"${CRAFF}" -o test.fs.craff test.fs
