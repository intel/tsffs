# hadolint global ignore=DL3008
FROM ubuntu:26.04@sha256:3131b4cc82a783df6c9df078f86e01819a13594b865c2cad47bd1bca2b7063bb AS buildroot

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
