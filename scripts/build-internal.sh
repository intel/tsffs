#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# NOTE: This script requires the Intel version of `ispm` to be installed and available
# on the PATH. We do not download a new copy of it because despite its public
# unavailability, we want to be able to keep this script in the public repository
# without revealing any internal URLs. It will *not* work correctly with the public
# version of ISPM unless you are an Intel or Wind River Simics customer.

set -e

MAJOR_VERSION="${1}"

if [ -z "${MAJOR_VERSION}" ]; then
    MAJOR_VERSION="7"
fi

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

install_major_version() {
    WANTED_MAJOR_VERSION="${1}"
    ispm packages --install-dir "${BUILDER_DIR}/rsrc/simics-${WANTED_MAJOR_VERSION}" -i \
        "1000-${WANTED_MAJOR_VERSION}.latest" \
        "1020-${WANTED_MAJOR_VERSION}.latest" \
        "1030-${WANTED_MAJOR_VERSION}.latest" \
        "1031-${WANTED_MAJOR_VERSION}.latest" \
        "2050-${WANTED_MAJOR_VERSION}.latest" \
        "2053-${WANTED_MAJOR_VERSION}.latest" \
        "2096-${WANTED_MAJOR_VERSION}.latest" \
        "4094-${WANTED_MAJOR_VERSION}.latest" \
        "6010-${WANTED_MAJOR_VERSION}.latest" \
        "7801-${WANTED_MAJOR_VERSION}.latest" \
        "8112-${WANTED_MAJOR_VERSION}.latest" \
        "8126-${WANTED_MAJOR_VERSION}.latest" \
        "8144-${WANTED_MAJOR_VERSION}.latest" \
        --non-interactive
}

download_and_verify_builder_deps

if [ "${MAJOR_VERSION}" -eq "7" ]; then
  DOCKERFILE="${BUILDER_DIR}/Dockerfile-internal-7"
  if [ ! -d "${BUILDER_DIR}/rsrc/simics-7" ]; then
      echo "Simics 7 packages not found. Installing..."
      install_major_version 7
  fi
fi

if [ "${MAJOR_VERSION}" -eq "6" ]; then
  DOCKERFILE="${BUILDER_DIR}/Dockerfile-internal-6"
  if [ ! -d "${BUILDER_DIR}/rsrc/simics-6" ]; then
      echo "Simics 6 packages not found. Installing..."
      install_major_version 6
  fi
fi

unset SIMICS_BASE
docker build \
    -t "${IMAGE_NAME}" -f "${DOCKERFILE}" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
mkdir -p "${ROOT_DIR}/packages-internal"
docker cp "${CONTAINER_NAME}:/packages-internal" "${ROOT_DIR}/"
docker rm -f "${CONTAINER_NAME}"