#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="tsffs-tutorial-riscv64-kernel-module"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker build -t "${IMAGE_NAME}" -f "${SCRIPT_DIR}/Dockerfile" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
docker cp \
    "${CONTAINER_NAME}:/output/Image"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/fw_jump.elf"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/rootfs.ext2"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/tutorial-mod.ko"\
    "${SCRIPT_DIR}/project/"
docker cp \
    "${CONTAINER_NAME}:/output/tutorial-mod-driver"\
    "${SCRIPT_DIR}/project/"
docker rm -f "${CONTAINER_NAME}"

dd if=/dev/zero "of=${SCRIPT_DIR}/project/test.fs" bs=1024 count=131072
mkfs.fat "${SCRIPT_DIR}/project/test.fs"
mcopy -i "${SCRIPT_DIR}/project/test.fs" "${SCRIPT_DIR}/project/tutorial-mod-driver" ::tutorial-mod-driver
mcopy -i "${SCRIPT_DIR}/project/test.fs" "${SCRIPT_DIR}/project/tutorial-mod.ko" ::tutorial-mod.ko