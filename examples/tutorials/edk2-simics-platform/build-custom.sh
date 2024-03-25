#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-simics"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile-custom"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"
EDK2_HASH="eccdab6"
EDK2_PLATFORMS_HASH="f446fff"
EDK2_NON_OSI_HASH="1f4d784"
INTEL_FSP_HASH="8beacd5"

if [ ! -d "${SCRIPT_DIR}/workspace" ]; then
    mkdir -p "${SCRIPT_DIR}/workspace"
    git clone https://github.com/tianocore/edk2.git "${SCRIPT_DIR}/workspace/edk2"
    git -C "${SCRIPT_DIR}/workspace/edk2" checkout "${EDK2_HASH}"
    git -C "${SCRIPT_DIR}/workspace/edk2" submodule update --init
    git clone https://github.com/tianocore/edk2-platforms.git "${SCRIPT_DIR}/workspace/edk2-platforms"
    git -C "${SCRIPT_DIR}/workspace/edk2-platforms" checkout "${EDK2_PLATFORMS_HASH}"
    git -C "${SCRIPT_DIR}/workspace/edk2-platforms" submodule update --init
    cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/workspace/edk2-platforms/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/tsffs.h"
    git clone https://github.com/tianocore/edk2-non-osi.git "${SCRIPT_DIR}/workspace/edk2-non-osi"
    git -C "${SCRIPT_DIR}/workspace/edk2-non-osi" checkout "${EDK2_NON_OSI_HASH}"
    git -C "${SCRIPT_DIR}/workspace/edk2-non-osi" submodule update --init
    git clone https://github.com/IntelFsp/FSP.git "${SCRIPT_DIR}/workspace/FSP"
    git -C "${SCRIPT_DIR}/workspace/FSP" checkout "${INTEL_FSP_HASH}"
    git -C "${SCRIPT_DIR}/workspace/FSP" submodule update --init
fi

docker build -t "${IMAGE_NAME}" -f "${DOCKERFILE}" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
docker cp "${CONTAINER_NAME}:/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/" "${SCRIPT_DIR}/BoardX58Ich10_CUSTOM"
docker rm -f "${CONTAINER_NAME}"
mkdir -p "${SCRIPT_DIR}/project/targets/qsp-x86/images/"
cp "${SCRIPT_DIR}/BoardX58Ich10_CUSTOM/BOARDX58ICH10.fd" "${SCRIPT_DIR}/project/targets/qsp-x86/images/BOARDX58ICH10_CUSTOM.fd"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/project/minimal_boot_disk.craff"
