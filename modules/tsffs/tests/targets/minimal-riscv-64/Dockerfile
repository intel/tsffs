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

WORKDIR buildroot

COPY <<EOF configs/simics_simple_riscv_defconfig
BR2_riscv=y
BR2_RISCV_64=y
BR2_TARGET_GENERIC_GETTY_PORT="ttyS0"
BR2_SYSTEM_DHCP="eth0"
BR2_LINUX_KERNEL=y
BR2_LINUX_KERNEL_USE_ARCH_DEFAULT_CONFIG=y
BR2_LINUX_KERNEL_CUSTOM_VERSION_VALUE="6.5"
BR2_PACKAGE_HOST_LINUX_HEADERS_CUSTOM_6_5=y
BR2_LINUX_KERNEL_IMAGE=y
BR2_TARGET_OPENSBI=y
BR2_TARGET_OPENSBI_PLAT="generic"
BR2_TARGET_ROOTFS_EXT2=y
BR2_PACKAGE_HOST_DTC=y
BR2_ROOTFS_OVERLAY="board/simics/risc-v-simple/rootfs_overlay"
EOF

RUN mkdir -p board/simics/risc-v-simple/rootfs_overlay/etc/ssh/

COPY <<EOF board/simics/risc-v-simple/rootfs_overlay/etc/ssh/sshd_config
PermitRootLogin yes
PermitEmptyPasswords yes
AuthorizedKeysFile .ssh/authorized_keys
Subsystem sftp /usr/libexec/sftp-server
EOF

RUN make simics_simple_riscv_defconfig && \
    make && \
    tar -C output/images -czvf images.tar.gz Image fw_jump.elf rootfs.ext2

RUN mkdir -p /test/

COPY test.c /test/test.c
COPY tsffs-gcc-riscv64.h /test/tsffs-gcc-riscv64.h

RUN /buildroot/output/host/bin/riscv64-buildroot-linux-gnu-gcc -o /test/test /test/test.c