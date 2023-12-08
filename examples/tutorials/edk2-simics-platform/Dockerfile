FROM ghcr.io/tianocore/containers/fedora-37-build:a0dd931

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /workspace/

RUN git clone https://github.com/tianocore/edk2.git && \
    git clone https://github.com/tianocore/edk2-platforms.git && \
    git clone https://github.com/tianocore/edk2-non-osi.git && \
    git clone https://github.com/IntelFsp/FSP.git

ARG EDK2_HASH="eccdab6"
ARG EDK2_PLATFORMS_HASH="f446fff"
ARG EDK2_NON_OSI_HASH="1f4d784"
ARG INTEL_FSP_HASH="8beacd5"

RUN git -C edk2 checkout "${EDK2_HASH}" && \
    git -C edk2 submodule update --init && \
    git -C edk2-platforms checkout "${EDK2_PLATFORMS_HASH}" && \
    git -C edk2-platforms submodule update --init && \
    git -C edk2-non-osi checkout "${EDK2_NON_OSI_HASH}" && \
    git -C edk2-non-osi submodule update --init && \
    git -C FSP checkout "${INTEL_FSP_HASH}" && \
    git -C FSP submodule update --init

ENV EDK_TOOLS_PATH=/workspace/edk2/BaseTools/
ENV PACKAGES_PATH="/workspace/edk2:/workspace/edk2-platforms:/workspace/edk2-non-osi"
ENV WORKSPACE=/workspace/

RUN source edk2/edksetup.sh && \
    make -C edk2/BaseTools/

WORKDIR /workspace/edk2-platforms/Platform/Intel

RUN source /workspace/edk2/edksetup.sh && \
    python build_bios.py -p BoardX58Ich10X64 -d -t GCC
    
