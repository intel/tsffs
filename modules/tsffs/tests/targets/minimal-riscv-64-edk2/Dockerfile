# hadolint global ignore=DL3008
FROM ghcr.io/tianocore/containers/ubuntu-22-build:a0dd931
ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV EDK2_REPO_URL "https://github.com/tianocore/edk2.git"
ENV EDK2_REPO_HASH "a671a14e63fdaa9490e5c61cf11346416f1d1463"
ENV EDK2_PATH "/edk2"

# Install riscv64-gcc
RUN apt-get -y update && \
    apt-get -y install autoconf automake autotools-dev curl python3 python3-pip \
    libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex gperf \
    libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev

RUN git clone https://github.com/riscv/riscv-gnu-toolchain /riscv-gnu-toolchain

WORKDIR /riscv-gnu-toolchain

# 32-bit
# RUN ./configure --with-arch=rv32gc --with-abi=ilp32d
RUN ./configure --enable-multilib && \
    make && \
    make install

ENV GCC_RISCV64_PREFIX="riscv64-unknown-elf-"
    
RUN git clone "${EDK2_REPO_URL}" "${EDK2_PATH}" && \
    git -C "${EDK2_PATH}" checkout "${EDK2_REPO_HASH}" && \
    python3 -m pip install --no-cache-dir -r "${EDK2_PATH}/pip-requirements.txt" && \
    stuart_setup -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC && \
    stuart_update -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC

COPY src "${EDK2_PATH}/HelloWorld/"

RUN stuart_setup -c "${EDK2_PATH}/HelloWorld/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    stuart_update -c "${EDK2_PATH}/HelloWorld/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    python3 "${EDK2_PATH}/BaseTools/Edk2ToolsBuild.py" -t GCC

WORKDIR "${EDK2_PATH}"

RUN source ${EDK2_PATH}/edksetup.sh && \
    ( build -a RISCV64 -t GCC -p HelloWorld/HelloWorld.dsc \
    || ( cat ${EDK2_PATH}/HelloWorld/Build/BUILDLOG.txt && exit 1 ) )
