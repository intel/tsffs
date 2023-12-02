#!/bin/bash

PUBLIC_SIMICS_PKGS_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/simics-6-packages-2023-31-linux64.ispm"
PUBLIC_SIMICS_ISPM_URL="https://registrationcenter-download.intel.com/akdlm/IRC_NAS/881ee76a-c24d-41c0-af13-5d89b2a857ff/intel-simics-package-manager-1.7.5-linux64.tar.gz"
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
ROOT_DIR="${SCRIPT_DIR}/../"
BUILDER_DIR="${ROOT_DIR}/.github/builder/"
IMAGE_NAME="tsffs-builder"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${BUILDER_DIR}/rsrc"
if [ ! -f "${BUILDER_DIR}/rsrc/ispm.tar.gz" ]; then
    curl --noproxy '*.intel.com' -o "${BUILDER_DIR}/rsrc/ispm.tar.gz" \
        "${PUBLIC_SIMICS_ISPM_URL}"
fi
if [ ! -f "${BUILDER_DIR}/rsrc/simics.ispm" ]; then
    curl --noproxy '*.intel.com' -o "${BUILDER_DIR}/rsrc/simics.ispm" \
        "${PUBLIC_SIMICS_PKGS_URL}"
fi

docker build -t "${IMAGE_NAME}" -f "${BUILDER_DIR}/Dockerfile" "${ROOT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
docker cp "${CONTAINER_NAME}:/tsffs/linux64/packages/" "${ROOT_DIR}"
docker rm -f "${CONTAINER_NAME}"