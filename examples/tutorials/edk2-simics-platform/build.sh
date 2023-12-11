#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-simics-platform"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-x86_64.h" "${SCRIPT_DIR}/tsffs-gcc-x86_64.h"
mkdir -p "${SCRIPT_DIR}/project/"
docker build -t "${IMAGE_NAME}" -f "${DOCKERFILE}" --build-arg "PROJECT=${SCRIPT_DIR}/project/workspace/" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
rm -rf "${SCRIPT_DIR}/project/workspace/"
docker cp "${CONTAINER_NAME}:${SCRIPT_DIR}/project/workspace/" "${SCRIPT_DIR}/project/workspace/"
docker rm -f "${CONTAINER_NAME}"