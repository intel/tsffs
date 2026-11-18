#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Build the HelloWorld.efi module and copy it into the resource directory for the example
# this only needs to be run if you want to modify the source code for the HelloWorld.efi module,
# otherwise, the EFI is included in the source tree for ease of use

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-build-tsffs-gcc-x86_64-test"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

pushd "${SCRIPT_DIR}" || exit 1

cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/src/tsffs.h"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

docker buildx build -t "${IMAGE_NAME}" -f "Dockerfile" .
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
docker cp \
    "${CONTAINER_NAME}:/edk2/HelloWorld/Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld.efi" \
    "${SCRIPT_DIR}/test.efi"

# The source and debug files of the application, laid out as in the container below
# /edk2/HelloWorld, for source coverage with the path map
# "/edk2/HelloWorld" -> "debug-info"
CONTAINER_APP_DIR="/edk2/HelloWorld"
MODULE_DEBUG_DIR="Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld/HelloWorld/DEBUG"
DEBUG_INFO_DIR="${SCRIPT_DIR}/debug-info"

rm -rf "${DEBUG_INFO_DIR}"
mkdir -p "${DEBUG_INFO_DIR}/${MODULE_DEBUG_DIR}"
docker cp \
    "${CONTAINER_NAME}:${CONTAINER_APP_DIR}/HelloWorld.c" \
    "${DEBUG_INFO_DIR}/HelloWorld.c"
for DEBUG_FILE in HelloWorld.efi HelloWorld.dll HelloWorld.debug; do
    docker cp \
        "${CONTAINER_NAME}:${CONTAINER_APP_DIR}/${MODULE_DEBUG_DIR}/${DEBUG_FILE}" \
        "${DEBUG_INFO_DIR}/${MODULE_DEBUG_DIR}/${DEBUG_FILE}"
done

docker rm -f "${CONTAINER_NAME}"