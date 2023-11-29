#!/bin/bash

# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Build the HelloWorld.efi module and copy it into the resource directory for the example
# this only needs to be run if you want to modify the source code for the HelloWorld.efi module,
# otherwise, the EFI is included in the source tree for ease of use

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-build-tsffs-gcc-x86_64-test-breakpoint"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

pushd "${SCRIPT_DIR}" || exit 1

cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-x86_64.h" "${SCRIPT_DIR}/src/tsffs-gcc-x86_64.h"
cp "${SCRIPT_DIR}/../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

if [ ! -e "${SCRIPT_DIR}/test.efi" ]; then
    docker build -t "${IMAGE_NAME}" -f "Dockerfile" .
    docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
    docker cp \
        "${CONTAINER_NAME}:/edk2/HelloWorld/Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld.efi" \
        "${SCRIPT_DIR}/test.efi"
    docker rm -f "${CONTAINER_NAME}"
fi
