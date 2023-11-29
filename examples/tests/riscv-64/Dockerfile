# hadolint global ignore=DL3008
FROM ubuntu:22.04 AS buildroot

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get -y update && \
    apt-get -y install \
        bash \
        bc \
        build-essential \
        cpio \
        file \
        git \
        gcc \
        g++ \
        rsync \
        unzip \
        wget

RUN git clone \
    https://github.com/buildroot/buildroot.git

WORKDIR /buildroot

# Build user-space test program
RUN mkdir -p /test/usr/
COPY test-kernel-modules /test/test-kernel-modules/
COPY simics_simple_riscv_defconfig configs/simics_simple_riscv_defconfig
COPY test.c /test/usr/test.c
COPY test-mod.c /test/usr/test-mod.c
COPY test-mod-userspace.c /test/usr/test-mod-userspace.c
COPY tsffs-gcc-riscv64.h /test/usr/tsffs-gcc-riscv64.h

# Build Linux, Linux Kernel Modules & RootFS
RUN make BR2_EXTERNAL=/test/test-kernel-modules/ simics_simple_riscv_defconfig && \
    make && \
    tar -C output/images -czvf images.tar.gz Image fw_jump.elf rootfs.ext2 && \
    cp images.tar.gz /test/ && \
    /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test /test/usr/test.c && \
    /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test-mod /test/usr/test-mod.c && \
    /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test-mod-userspace /test/usr/test-mod-userspace.c && \
    cp -a /buildroot/output/build/ /test/output/ && \
    make clean

