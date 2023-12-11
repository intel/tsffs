# Building the Application

To build the application, we'll use the EDK2 docker containers provided by tianocore. In
the directory that contains your `src` directory, create a `Dockerfile`:

```dockerfile
FROM ghcr.io/tianocore/containers/ubuntu-22-build:a0dd931
ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV EDK2_REPO_URL "https://github.com/tianocore/edk2.git"
ENV EDK2_REPO_HASH "d189de3b0a2f44f4c9b87ed120be16569ea19b51"
ENV EDK2_PATH "/edk2"

RUN git clone "${EDK2_REPO_URL}" "${EDK2_PATH}" && \
    git -C "${EDK2_PATH}" checkout "${EDK2_REPO_HASH}" && \
    python3 -m pip install --no-cache-dir -r "${EDK2_PATH}/pip-requirements.txt" && \
    stuart_setup -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC&& \
    stuart_update -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC

COPY src "${EDK2_PATH}/Tutorial/"

RUN stuart_setup -c "${EDK2_PATH}/Tutorial/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    stuart_update -c "${EDK2_PATH}/Tutorial/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    python3 "${EDK2_PATH}/BaseTools/Edk2ToolsBuild.py" -t GCC

WORKDIR "${EDK2_PATH}"

RUN source ${EDK2_PATH}/edksetup.sh && \
    ( stuart_build -c ${EDK2_PATH}/Tutorial/PlatformBuild.py TOOL_CHAIN_TAG=GCC \
    EDK_TOOLS_PATH=${EDK2_PATH}/BaseTools/ \
    || ( cat ${EDK2_PATH}/Tutorial/Build/BUILDLOG.txt && exit 1 ) )
```

This Dockerfile will obtain the EDK2 source and compile the BaseTools, then copy our
`src` directory into the EDK2 repository as a new package and build the package.

We will want to get our built UEFI application from the container, which we can
do using the `docker cp` command. There are a few files we want to copy, so we'll
use this script `build.sh` to automate the process:

```sh
#!/bin/bash

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
```

The script will build the image, create a container using it, copy the relevant files
to our host machine (in a `project` directory), then delete the container.

Mark the script executable and then we'll go ahead and run it with:

```sh
chmod +x build.sh
./build.sh
```