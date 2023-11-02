#!/bin/bash

# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Build the HelloWorld.efi module and copy it into the resource directory for the example
# this only needs to be run if you want to modify the source code for the HelloWorld.efi module,
# otherwise, the EFI is included in the source tree for ease of use

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="buildroot-build-tsffs-gcc-riscv64-test"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

pushd "${SCRIPT_DIR}" || exit 1

cp "${SCRIPT_DIR}/../../../../../harness/tsffs-gcc-riscv64.h" "${SCRIPT_DIR}/tsffs-gcc-riscv64.h"
cp "${SCRIPT_DIR}/../../../../../harness/tsffs-gcc-riscv64.h" "${SCRIPT_DIR}/test-kernel-modules/package/kernel-modules/test-mod/tsffs-gcc-riscv64.h"

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
    "${CONTAINER_NAME}:/buildroot/output/build/test-mod-1.0/test-mod.ko" \
    "${SCRIPT_DIR}/test-mod.ko"
docker rm -f "${CONTAINER_NAME}"
tar -xvf images.tar.gz
