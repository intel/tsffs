ARG DEBIAN_FRONTEND=noninteractive

FROM ubuntu:22.04 AS buildroot

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

COPY test-kernel-modules /test/test-kernel-modules/

COPY simics_simple_riscv_defconfig configs/simics_simple_riscv_defconfig

# Build Linux, Linux Kernel Modules & RootFS
RUN make BR2_EXTERNAL=/test/test-kernel-modules/ simics_simple_riscv_defconfig && \
    make && \
    tar -C output/images -czvf images.tar.gz Image fw_jump.elf rootfs.ext2

# Build user-space test program
RUN mkdir -p /test/usr/

COPY test.c /test/usr/test.c
COPY test-mod.c /test/usr/test-mod.c
COPY test-mod-userspace.c /test/usr/test-mod-userspace.c

COPY tsffs-gcc-riscv64.h /test/usr/tsffs-gcc-riscv64.h

RUN /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test /test/usr/test.c && \
    /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test-mod /test/usr/test-mod.c && \
    /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/usr/test-mod-userspace /test/usr/test-mod-userspace.c
