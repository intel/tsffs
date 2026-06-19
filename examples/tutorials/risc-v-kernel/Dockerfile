# hadolint global ignore=DL3008
FROM ubuntu:26.04@sha256:e153663f92c94118ff22a5dc397b59b351ffd695480566debb5850e017e5937a AS buildroot

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive
ENV FORCE_UNSAFE_CONFIGURE=1

RUN apt-get -y update && \
    apt-get -y install \
        bash bc build-essential cpio file git gcc g++ rsync unzip wget && \
    git clone \
        https://github.com/buildroot/buildroot.git

WORKDIR /buildroot

COPY src /src/

RUN mkdir -p /output/ && \
    cp /src/simics_simple_riscv_defconfig configs/simics_simple_riscv_defconfig && \
    make BR2_EXTERNAL=/src/tutorial-kernel-modules/ simics_simple_riscv_defconfig && \
    make BR2_EXTERNAL=/src/tutorial-kernel-modules/

RUN cp output/build/tutorial-mod-1.0/tutorial-mod.ko \
        output/images/Image \
        output/images/fw_jump.elf \
        output/images/rootfs.ext2 \
        /output && \
    output/host/bin/riscv64-buildroot-linux-gnu-gcc \
        -o /output/tutorial-mod-driver /src/tutorial-mod-driver.c
