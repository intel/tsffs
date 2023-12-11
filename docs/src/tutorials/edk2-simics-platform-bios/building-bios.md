# Building the BIOS

Working from the same `Dockerfile` we obtained sources into, we'll set things up to
build the BIOS image. First, we need to set up a few environment variables:

```dockerfile
ENV EDK_TOOLS_PATH="${PROJECT}/edk2/BaseTools/"
ENV PACKAGES_PATH="${PROJECT}/edk2:${PROJECT}/edk2-platforms:${PROJECT}/edk2-non-osi"
ENV WORKSPACE="${PROJECT}"
```

These variables are used by EDK2 to find its own sources and binaries.

Next, we'll build the *Base Tools*. You can read more about the *Base Tools* in the EDK2
[build
system](https://tianocore-docs.github.io/edk2-BuildSpecification/release-1.28/4_edk_ii_build_process_overview/41_edk_ii_build_system.html#41-edk-ii-build-system)
documentation.

```dockerfile
RUN source edk2/edksetup.sh && \
    make -C edk2/BaseTools/
```

With the *Base Tools* built, we can build the BIOS. We directly follow the directions
provided, and you can read more about the process, what settings are available for the
BIOS (in particular, how to change the BIOS stages) [in the
repo](https://github.com/tianocore/edk2-platforms/blob/f446fff05003f69a4396b2ec375301ecb5f63a2a/Platform/Intel/Readme.md).

First, we'll change into the tree containing the platform code for all the Intel
platforms, then use the Intel-provided build script to select our board, toolchain,
and debug mode (in this case, enabled).

```dockerfile
WORKDIR "${PROJECT}/edk2-platforms/Platform/Intel"

# Build SimicsOpenBoardPkg
RUN source "${PROJECT}/edk2/edksetup.sh" && \
    python build_bios.py -p BoardX58Ich10X64 -d -t GCC
```

We'll use a build script to manage building the container and copying the relevant
artifacts out of it. Place this script `build.sh` next to your `Dockerfile`.

```sh
#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="edk2-simics-platform"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/"
docker build -t "${IMAGE_NAME}" -f "${DOCKERFILE}" --build-arg "PROJECT=${SCRIPT_DIR}/project/workspace/" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}" bash
rm -rf "${SCRIPT_DIR}/project/workspace/"
docker cp "${CONTAINER_NAME}:${SCRIPT_DIR}/project/workspace/" "${SCRIPT_DIR}/project/workspace/"
docker rm -f "${CONTAINER_NAME}"
```

Now run the script:

```sh
chmod +x build.sh
./build.sh
```

If all goes well, you'll have a directory
`project/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV` containing our
BIOS image (`BOARDX58ICH10.fd`).

```sh
ls project/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd
```