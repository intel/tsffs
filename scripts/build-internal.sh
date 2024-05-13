#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# NOTE: This script requires the Intel version of `ispm` to be installed and available
# on the PATH. We do not download a new copy of it because despite its public
# unavailability, we want to be able to keep this script in the public repository
# without revealing any internal URLs. It will *not* work correctly with the public
# version of ISPM unless you are an Intel or Wind River Simics customer.

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/../"
BUILDER_DIR="${ROOT_DIR}/.github/builder/"
IMAGE_NAME="tsffs-builder-internal"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

# shellcheck disable=SC1091
source "${BUILDER_DIR}/common.sh"

mkdir -p "${BUILDER_DIR}/rsrc"

ISPM=$(which ispm)
ISPM_DIR=$(dirname "${ISPM}")

if [ ! -d "${BUILDER_DIR}/rsrc/ispm" ]; then
    echo "ISPM not found. Copying local installation..."
    cp -a "${ISPM_DIR}" "${BUILDER_DIR}/rsrc/ispm"
fi

if [ ! -d "${BUILDER_DIR}/rsrc/simics" ]; then
    echo "Simics packages not found. Installing..."
    mkdir -p "${BUILDER_DIR}/rsrc/simics"
    ispm packages --install-dir "${BUILDER_DIR}/rsrc/simics" -i \
        1000-latest \
        1020-latest \
        1030-latest \
        1031-latest \
        2050-latest \
        2053-latest \
        2096-latest \
        4094-latest \
        6010-latest \
        7801-latest \
        8112-latest \
        8126-latest \
        8144-latest \
        --non-interactive
fi

download_and_verify_builder_deps

unset SIMICS_BASE
docker build \
    -t "${IMAGE_NAME}" -f "${BUILDER_DIR}/Dockerfile-internal" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
mkdir -p "${ROOT_DIR}/packages-internal"
docker cp "${CONTAINER_NAME}:/packages-internal" "${ROOT_DIR}/"
docker rm -f "${CONTAINER_NAME}"
