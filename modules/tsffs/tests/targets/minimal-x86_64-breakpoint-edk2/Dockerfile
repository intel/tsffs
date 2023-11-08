# hadolint global ignore=DL3008
FROM ghcr.io/tianocore/containers/ubuntu-22-build:a0dd931
ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV EDK2_REPO_URL "https://github.com/tianocore/edk2.git"
ENV EDK2_REPO_HASH "d189de3b0a2f44f4c9b87ed120be16569ea19b51"
ENV EDK2_PATH "/edk2"


RUN git clone "${EDK2_REPO_URL}" "${EDK2_PATH}" && \
    git -C "${EDK2_PATH}" checkout "${EDK2_REPO_HASH}" && \
    python3 -m pip install --no-cache-dir -r "${EDK2_PATH}/pip-requirements.txt" && \
    stuart_setup -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC5 && \
    stuart_update -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC5

COPY src "${EDK2_PATH}/HelloWorld/"

RUN stuart_setup -c "${EDK2_PATH}/HelloWorld/PlatformBuild.py" TOOL_CHAIN_TAG=GCC5 && \
    stuart_update -c "${EDK2_PATH}/HelloWorld/PlatformBuild.py" TOOL_CHAIN_TAG=GCC5 && \
    python3 "${EDK2_PATH}/BaseTools/Edk2ToolsBuild.py" -t GCC5

WORKDIR "${EDK2_PATH}"

RUN source ${EDK2_PATH}/edksetup.sh && \
    ( stuart_build -c ${EDK2_PATH}/HelloWorld/PlatformBuild.py TOOL_CHAIN_TAG=GCC5 \
    EDK_TOOLS_PATH=${EDK2_PATH}/BaseTools/ \
    || ( cat ${EDK2_PATH}/HelloWorld/Build/BUILDLOG.txt && exit 1 ) )
