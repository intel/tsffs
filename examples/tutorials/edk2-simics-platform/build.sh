#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-simics"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

docker build -t "${IMAGE_NAME}" -f "${DOCKERFILE}" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
docker cp "${CONTAINER_NAME}:/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/" "${SCRIPT_DIR}"
docker rm -f "${CONTAINER_NAME}"
mkdir -p "${SCRIPT_DIR}/project/targets/qsp-x86/images/"
cp "${SCRIPT_DIR}/FV/BOARDX58ICH10.fd" "${SCRIPT_DIR}/project/targets/qsp-x86/images/BOARDX58ICH10.fd"
