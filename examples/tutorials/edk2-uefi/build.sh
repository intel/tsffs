#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="tsffs-tutorial-edk2-uefi"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/"
# copy minimal boot disk
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/project/"

# copy tsffs.h header into src
cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/src/"
docker build -t "${IMAGE_NAME}" -f "Dockerfile" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"

for file_ext in efi map debug; do
    docker cp \
        "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.${file_ext}" \
        "${SCRIPT_DIR}/project/Tutorial.${file_ext}"
done

docker rm -f "${CONTAINER_NAME}"

# ensure corpus
if [ ! -d "${SCRIPT_DIR}/corpus" ]; then
    mkdir "${SCRIPT_DIR}/corpus"
    curl -L -o "${SCRIPT_DIR}/corpus/0" https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/0
    curl -L -o "${SCRIPT_DIR}/corpus/1" https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/1
    curl -L -o "${SCRIPT_DIR}/corpus/2" https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/2
    curl -L -o "${SCRIPT_DIR}/corpus/3" https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/3
fi
