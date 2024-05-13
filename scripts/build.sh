#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/../"
BUILDER_DIR="${ROOT_DIR}/.github/builder/"
IMAGE_NAME="tsffs-builder"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

# shellcheck disable=SC1091
source "${BUILDER_DIR}/common.sh"

download_and_verify_builder_deps

unset SIMICS_BASE
docker build \
    --build-arg \
    "PUBLIC_SIMICS_PACKAGE_VERSION_1000=${PUBLIC_SIMICS_PACKAGE_VERSION_1000}" \
    -t "${IMAGE_NAME}" -f "${BUILDER_DIR}/Dockerfile" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
mkdir -p "${ROOT_DIR}/packages"
docker cp "${CONTAINER_NAME}:/packages" "${ROOT_DIR}/"
docker rm -f "${CONTAINER_NAME}"