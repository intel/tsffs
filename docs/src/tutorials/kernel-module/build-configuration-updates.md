# Update Build Configuration

## Add Buildroot Defconfig

Similar to the Linux configuration system, we need to create a Buildroot config file.
This file was created with `make menuconfig`, and most of the customization is far out
of scope of this tutorial. In general, the options are either required by SIMICS
(OpenSBI, RISC-V configuration, and so forth) or are the defaults.

The file is too large to include here, so copy
`examples/tutorials/risc-v-kernel/src/simics_simple_riscv_defconfig` from the TSFFS
repository into your `src` directory.

## Update Build Process

Now that all our source code is in place, we'll add a few commands to our Dockerfile.

```dockerfile
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

```

First, we create a directory to store our build artifacts (`/output`). Then, we make
buildroot with our configuration. This takes quite a while. Once it is built we copy the
build results outlined above into the `/output` directory and compile our user space
driver program.

To build the container and extract the results, we'll create a shell script `build.sh`
alongside our `Dockerfile` notice that we use `mcopy` from the package `dosfstools` to
create a `fat` filesystem and add our files to it. In the next step, we'll convert it to
a format mountable in SIMICS.

```sh
#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="tsffs-tutorial-riscv64-kernel-module"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker build -t "${IMAGE_NAME}" -f "${SCRIPT_DIR}/Dockerfile" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
docker cp \
    "${CONTAINER_NAME}:/output/Image"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/fw_jump.elf"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/rootfs.ext2"\
    "${SCRIPT_DIR}/project/targets/risc-v-simple/images/linux/"
docker cp \
    "${CONTAINER_NAME}:/output/tutorial-mod.ko"\
    "${SCRIPT_DIR}/project/"
docker cp \
    "${CONTAINER_NAME}:/output/tutorial-mod-driver"\
    "${SCRIPT_DIR}/project/"
docker rm -f "${CONTAINER_NAME}"

dd if=/dev/zero "of=${SCRIPT_DIR}/project/test.fs" bs=1024 count=131072
mkfs.fat "${SCRIPT_DIR}/project/test.fs"
mcopy -i "${SCRIPT_DIR}/project/test.fs" "${SCRIPT_DIR}/project/tutorial-mod-driver" ::tutorial-mod-driver
mcopy -i "${SCRIPT_DIR}/project/test.fs" "${SCRIPT_DIR}/project/tutorial-mod.ko" ::tutorial-mod.ko
```

Notice that we copy `Image`, `fw_jump.elf`, and `rootfs.ext2` into
`targets/risc-v-simple/images/linux/`. This is by convention, and is where the
`risc-v-simple` target provided by SIMICS expects to find these three files. You can
read the specifics in the RISC-V model package documentation.

## Build The Software

With all the configuration and build processes done, it's time to build the target
software:

```sh
chmod +x build.sh
./build.sh
```
If all goes well, you'll be greeted with a `project` directory with all our necessary
files.

## Convert the Filesystem

To easily mount our FAT formatted filesystem `test.fs` in our simulated system, we need
to convert it to the CRAFF format. SIMICS base provides the `craff` utility to do this.

Find your SIMICS base path with:

```sh
$ ispm packages --list-installed
Installed Base Packages
 Package Number  Name         Version  Installed Paths                   
 1000            Simics-Base  6.0.169  /home/YOUR_USERNAME/simics/simics-6.0.169
```

The `craff` utility is in `/home/YOUR_USERNAME/simics/simics-6.0.169/linux64/bin/craff`.

Convert the filesystem with:

```sh
/home/YOUR_USERNAME/simics/simics-6.0.169/linux64/bin/craff \
  -o project/test.fs.craff \
  project/test.fs
```

This will allow us to mount `test.fs.craff` into the simulator.

