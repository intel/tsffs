#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0


SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="tsffs-tutorial-edk2-uefi"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/"
docker build -t "${IMAGE_NAME}" -f "Dockerfile" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.efi" \
    "${SCRIPT_DIR}/project/Tutorial.efi"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.map" \
    "${SCRIPT_DIR}/project/Tutorial.map"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.debug" \
    "${SCRIPT_DIR}/project/Tutorial.debug"
docker rm -f "${CONTAINER_NAME}"