# Fuzzing a Kernel Module

This tutorial will walk you through the entire process of creating, building, and
fuzzing a Linux Kernel module running on the simulated RISC-V platform. The complete
example code and scripts can be found in the [kernel-module tutorial
directory](https://github.com/intel/tsffs/tree/main/examples/tutorials/edk2-uefi).

## Outlining Our Target Software

We are targeting RISC-V, so we will be using [buildroot](https://buildroot.org/) for
our toolchain and Linux build. We need to build the following:

* `fw_jump.elf`, `Image`, and `rootfs.ext2`, our firmware jump binary, linux kernel
  image, and root filesystem, respectively. These three files are expected by the
  public RISC-V platform model for SIMICS to boot Linux. Other approaches can be
  used but will require significantly more customization.
* `tutorial-mod.ko` our tutorial kernel module. We'll create a kernel module which
  provides a virtual device which can be controlled via IOCTL.
* `tutorial-mod-driver` a user-space application which will trigger the funcionality
  we want to fuzz in our kernel module. We'll discuss how to harness both by
  compiling the harness code into the kernel module *and* by compiling the harness code
  into the user-space driver application.

We'll use the
[br2-external](https://buildroot.org/downloads/manual/manual.html#outside-br-custom)
mechanism to keep our kernel module package separate from the buildroot tree.

## Creating Our Target Software

### Dockerfile

The first thing we need to do is create a `Dockerfile`. We'll add more lines to this
dockerfile as we create the sources.

```dockerfile
FROM ubuntu:22.04 AS buildroot

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get -y update && \
    apt-get -y install \
        bash bc build-essential cpio file git gcc g++ rsync unzip wget && \
    git clone \
        https://github.com/buildroot/buildroot.git

WORKDIR /buildroot

COPY src /src/
```

All we've done so far is install dependencies and clone the `buildroot` repository, as
well as copy our source code into the container in the `/src` directory.

This repo is *quite large*, nearly 7GB, so keep this in mind.

### Buildroot Boilerplate

Next to the `Dockerfile`, create a directory `src/tutorial-kernel-modules`:

```sh
mkdir -p src/tutorial-kernel-module
```

Create `src/tutorial-kernel-modules/Config.in` with the contents:

```txt
source "$BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH/package/kernel-modules/Config.in"
```

Create `src/tutorial-kernel-modules/external.desc` with the contents:

```txt
name: TUTORIAL_KERNEL_MODULES
```

And create `src/tutorial-kernel-modules/external.mk` with the contents:

```makefile
include $(sort $(wildcard $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/*/*.mk))
```

These three files are *required* to create an external buildroot tree, and tell buildroot
what to include in its configuration.

### Kernel Module Package Boilerplate

Now we can create our actual kernel module package (remember, the above just creates an
external package tree, we need to add a package to it).

```sh
mkdir -p src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod
```

Create `src/tutorial-kernel-modules/package/kernel-modules/Config.in` with the contents:

```txt
menu "Kernel Modules"
    source "$BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH/package/kernel-modules/tutorial-mod/Config.in"
endmenu
```

This adds a menu entry for our tutorial kernel module if one were to use
`make menuconfig` to configure buildroot.

Create `src/tutorial-kernel-modules/package/kernel-modules/kernel-modules.mk` with the
contents:

```makefile
include $(sort $(wildcard $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/*/*/*.mk))
```

This includes each of the (just one, in our case) kernel module package's makefiles.

### Kernel Module Boilerplate

At long last, we can set up and write our actual kernel module's boilerplate.

Create `src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/Config.in`:

```txt
config BR2_PACKAGE_TUTORIAL_MOD
    bool "tutorial-mod"
    depends on BR2_LINUX_KERNEL
    help
        Tutorial kernel module for TSFFS fuzzing
```

This defines the actual menu entry for this kernel module.

Create `src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/tutorial-mod.mk`
:

```makefile
TUTORIAL_MOD_VERSION = 1.0
TUTORIAL_MOD_SITE = $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/kernel-modules/tutorial-mod
TUTORIAL_MOD_SITE_METHOD = local

$(eval $(kernel-module))
$(eval $(generic-package))
```

This makefile is included by buildroot and tells buildroot this should be built as a
kernel module which is a generic package and tells buildroot where the source code is
located (AKA the site).

### Kernel Module Code

Next, create `src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/Makefile`,
which will be more familiar to Linux Kernel developers:

```makefile
obj-m += $(addsuffix .o, $(notdir $(basename $(wildcard $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/kernel-modules/tutorial-mod/*.c))))

.PHONY: all clean

all:
	$(MAKE) -C '/lib/modules/$(shell uname -r)/build' M='$(PWD)' modules

clean:
	$(MAKE) -C '$(LINUX_DIR)' M='$(PWD)' clean
```

This in turn invokes the standard KBuild process, specifying our current directory
as an out of tree modules directory.

Then, copy `tsffs-gcc-riscv64.h` from the `harness` directory of the repository into
`src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/tsffs-gcc-riscv64.h`.

Finally, we can write our Kernel module. Doing so is well beyond the scope of this
tutorial, so copy the code below into
`src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/tutorial-mod.c`.

```c
#include <asm/errno.h>
#include <linux/atomic.h>
#include <linux/cdev.h>
#include <linux/delay.h>
#include <linux/device.h>
#include <linux/fs.h>
#include <linux/init.h>
#include <linux/ioctl.h>
#include <linux/module.h>
#include <linux/printk.h>
#include <linux/types.h>
#include <linux/uaccess.h>
#include <linux/version.h>

#include "tsffs-gcc-riscv64.h"

#define MAJOR_NUM 100
#define IOCTL_SET_MSG _IOW(MAJOR_NUM, 0, char *)
#define IOCTL_GET_MSG _IOR(MAJOR_NUM, 1, char *)
#define IOCTL_GET_NTH_BYTE _IOWR(MAJOR_NUM, 2, int)
#define DEVICE_FILE_NAME "char_dev"
#define DEVICE_PATH "/dev/char_dev"
#define SUCCESS 0
#define DEVICE_NAME "char_dev"
#define BUF_LEN 80

enum {
  CDEV_NOT_USED = 0,
  CDEV_EXCLUSIVE_OPEN = 1,
};

static atomic_t already_open = ATOMIC_INIT(CDEV_NOT_USED);
static char message[BUF_LEN + 1];
static struct class *cls;

static int device_open(struct inode *inode, struct file *file) {
  pr_info("device_open(%p)\n", file);

  try_module_get(THIS_MODULE);
  return SUCCESS;
}

static int device_release(struct inode *inode, struct file *file) {
  pr_info("device_release(%p,%p)\n", inode, file);

  module_put(THIS_MODULE);
  return SUCCESS;
}
static ssize_t device_read(struct file *file, char __user *buffer,
                           size_t length, loff_t *offset) {
  int bytes_read = 0;
  const char *message_ptr = message;

  if (!*(message_ptr + *offset)) {
    *offset = 0;
    return 0;
  }

  message_ptr += *offset;

  while (length && *message_ptr) {
    put_user(*(message_ptr++), buffer++);
    length--;
    bytes_read++;
  }

  pr_info("Read %d bytes, %ld left\n", bytes_read, length);

  *offset += bytes_read;

  return bytes_read;
}

void check(char *buffer) {
  if (!strcmp(buffer, "fuzzing!")) {
    // Cause a crash
    char *x = NULL;
    *x = 0;
  }
}

static ssize_t device_write(struct file *file, const char __user *buffer,
                            size_t length, loff_t *offset) {
  int i;

  pr_info("device_write(%p,%p,%ld)", file, buffer, length);

  for (i = 0; i < length && i < BUF_LEN; i++) {
    get_user(message[i], buffer + i);
  }

  check(message);

  return i;
}

static long device_ioctl(struct file *file, unsigned int ioctl_num,
                         unsigned long ioctl_param) {
  int i;
  long ret = SUCCESS;

  if (atomic_cmpxchg(&already_open, CDEV_NOT_USED, CDEV_EXCLUSIVE_OPEN)) {
    return -EBUSY;
  }

  switch (ioctl_num) {
    case IOCTL_SET_MSG: {
      char __user *tmp = (char __user *)ioctl_param;
      char ch;

      get_user(ch, tmp);

      for (i = 0; ch && i < BUF_LEN; i++, tmp++) {
        get_user(ch, tmp);
      }

      device_write(file, (char __user *)ioctl_param, i, NULL);
      break;
    }
    case IOCTL_GET_MSG: {
      loff_t offset = 0;
      i = device_read(file, (char __user *)ioctl_param, 99, &offset);
      put_user('\0', (char __user *)ioctl_param + i);
      break;
    }
    case IOCTL_GET_NTH_BYTE:
      if (ioctl_param > BUF_LEN) {
        return -EINVAL;
      }

      ret = (long)message[ioctl_param];

      break;
  }

  atomic_set(&already_open, CDEV_NOT_USED);

  return ret;
}

static struct file_operations fops = {
    .read = device_read,
    .write = device_write,
    .unlocked_ioctl = device_ioctl,
    .open = device_open,
    .release = device_release,
};

static int __init chardev2_init(void) {
  int ret_val = register_chrdev(MAJOR_NUM, DEVICE_NAME, &fops);

  if (ret_val < 0) {
    pr_alert("%s failed with %d\n", "Sorry, registering the character device ",
             ret_val);
    return ret_val;
  }

  cls = class_create(DEVICE_FILE_NAME);
  device_create(cls, NULL, MKDEV(MAJOR_NUM, 0), NULL, DEVICE_FILE_NAME);

  pr_info("Device created on /dev/%s\n", DEVICE_FILE_NAME);

  return 0;
}

static void __exit chardev2_exit(void) {
  device_destroy(cls, MKDEV(MAJOR_NUM, 0));
  class_destroy(cls);

  unregister_chrdev(MAJOR_NUM, DEVICE_NAME);
}

module_init(chardev2_init);
module_exit(chardev2_exit);

MODULE_LICENSE("GPL");
```

To summarize, the module creates a character device which can be opened, read and
written, both via the read and write syscalls and via IOCTL. When written, the module
checks the data written against the password `fuzzing!`, and if the check passes, it
will crash itself by dereferencing NULL, which will cause a kernel panic that we will
use as a "solution" later.

### Harnessing the Kernel Module

Because the build process for the buildroot is quite long (5-10 mins on a fast machine),
we will avoid compiling it twice. Modify the `device_write` function:

```c
static ssize_t device_write(struct file *file, const char __user *buffer,
                            size_t length, loff_t *offset) {
  int i;

  pr_info("device_write(%p,%p,%ld)", file, buffer, length);

  for (i = 0; i < length && i < BUF_LEN; i++) {
    get_user(message[i], buffer + i);
  }

  size_t size = BUF_LEN;
  size_t *size_ptr = &size;

  HARNESS_START(message, size_ptr);

  check(message);

  HARNESS_STOP();

  return i;
}
```

This adds our harness such that the first time the `device_write` function is called,
via a user-space application writing or using the IOCTL system call, the fuzzer will
take over and start the fuzzing loop.

### Userspace Driver Code

First, copy `tsffs-gcc-riscv64.h` from the `harness` directory in the repository into
`src/tsffs-gcc-riscv64.h`.

We'll also create `src/tutorial-mod-driver.c`, a user-space application which we will
use to drive the kernel module code via IOCTL.

```c
#include <fcntl.h>
#include <linux/ioctl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <unistd.h>

#include "tsffs-gcc-riscv64.h"

#define MAJOR_NUM 100
#define IOCTL_SET_MSG _IOW(MAJOR_NUM, 0, char *)
#define IOCTL_GET_MSG _IOR(MAJOR_NUM, 1, char *)
#define IOCTL_GET_NTH_BYTE _IOWR(MAJOR_NUM, 2, int)
#define DEVICE_FILE_NAME "char_dev"
#define DEVICE_PATH "/dev/char_dev"

int ioctl_set_msg(int file_desc, char *message) {
  int ret_val;

  ret_val = ioctl(file_desc, IOCTL_SET_MSG, message);

  if (ret_val < 0) {
    printf("ioctl_set_msg failed:%d\n", ret_val);
  }

  return ret_val;
}

int ioctl_get_msg(int file_desc) {
  int ret_val;
  char message[100] = {0};

  ret_val = ioctl(file_desc, IOCTL_GET_MSG, message);

  if (ret_val < 0) {
    printf("ioctl_get_msg failed:%d\n", ret_val);
  }
  printf("get_msg message:%s", message);

  return ret_val;
}

int ioctl_get_nth_byte(int file_desc) {
  int i, c;

  printf("get_nth_byte message:");

  i = 0;
  do {
    c = ioctl(file_desc, IOCTL_GET_NTH_BYTE, i++);

    if (c < 0) {
      printf("\nioctl_get_nth_byte failed at the %d'th byte:\n", i);
      return c;
    }

    putchar(c);
  } while (c != 0);

  return 0;
}

int main(void) {
  int file_desc, ret_val;
  char *msg = "AAAAAAAA\n";

  file_desc = open(DEVICE_PATH, O_RDWR);
  if (file_desc < 0) {
    printf("Can't open device file: %s, error:%d\n", DEVICE_PATH, file_desc);
    exit(EXIT_FAILURE);
  }

  ret_val = ioctl_set_msg(file_desc, msg);
  if (ret_val) goto error;

  close(file_desc);
  return 0;
error:
  close(file_desc);
  exit(EXIT_FAILURE);
}
```

This application opens the character device of our module, sets the message, and closes
the device.

### Harnessing the Userspace Driver Code

Once again, because the build process is quite long, we'll add the user-space harness
now. Modify the `main` function:

```c
int main(void) {
  int file_desc, ret_val;
  char msg[80] = {0};

  file_desc = open(DEVICE_PATH, O_RDWR);
  if (file_desc < 0) {
    printf("Can't open device file: %s, error:%d\n", DEVICE_PATH, file_desc);
    exit(EXIT_FAILURE);
  }

  size_t msg_size = 80;
  size_t *msg_size_ptr = &msg_size;

  __arch_harness_start(MAGIC_ALT_0, msg, msg_size_ptr);

  ret_val = ioctl_set_msg(file_desc, msg);

  __arch_harness_stop(MAGIC_ALT_1);

  if (ret_val) goto error;

  close(file_desc);
  return 0;
error:
  close(file_desc);
  exit(EXIT_FAILURE);
}
```

Notice that instead of using `HARNESS_START` and `HARNESS_STOP` here, we use
`__arch_harness_start` and `stop` so that we can send a signal with a different `n`
value. This allows us to keep the compiled-in harnessing in the test kernel module,
while leaving it inactive.

### Add Buildroot Defconfig

Similar to the Linux configuration system, we need to create a Buildroot config file.
This file was created with `make menuconfig`, and most of the customization is far out
of scope of this tutorial. In general, the options are either required by SIMICS
(OpenSBI, RISC-V configuration, and so forth) or are the defaults.

The file is too large to include here, so copy
`examples/tutorials/risc-v-kernel/src/simics_simple_riscv_defconfig` from the TSFFS
repository into your `src` directory.

### Update Build Process

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

# Copyright (C) 2023 Intel Corporation
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

### Build The Software

With all the configuration and build processes done, it's time to build the target
software:

```sh
./build.sh
```
If all goes well, you'll be greeted with a `project` directory with all our necessary
files.

### Convert the Filesystem

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


## Running the Fuzzer

### Generate a Corpus

Because we have inside knowledge that this is an extremely simple test, we'll generate a
corpus ourselves.

```sh
mkdir -p project/corpus/
for i in $(seq 5); do
  echo -n "$(bash -c 'echo $RANDOM')" | sha256sum | head -c 8 > "project/corpus/${i}"
done
```

### Create a Project

The build script for our application created a `project` directory for us if it did not
exist, so we'll instantiate that directory as our project with `ispm`:

```sh
ispm projects project --create 1000-latest 2096-latest 8112-latest 31337-latest \
  --ignore-existing-files
cd project
```

### Configuring the Fuzzer

Create a script `project/run.simics`. First, we'll set up the fuzzer for harnessing in
the kernel module, using the default start/stop on harness.

```simics
load-module tsffs

@tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
tsffs.log-level 4
@tsffs.iface.tsffs.set_start_on_harness(True)
@tsffs.iface.tsffs.set_stop_on_harness(True)
@tsffs.iface.tsffs.set_timeout(3.0)
@tsffs.iface.tsffs.add_exception_solution(14)

load-target "risc-v-simple/linux" namespace = riscv machine:hardware:storage:disk1:image = "test.fs.craff"

script-branch {
    bp.time.wait-for seconds = 15
    board.console.con.input "mkdir /mnt/disk0\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "mount /dev/vdb /mnt/disk0\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "insmod /mnt/disk0/tutorial-mod.ko\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "/mnt/disk0/tutorial-mod-driver\r\n"
}

run
```

### Run the Test Script

Run the script:

```sh
./simics -no-gui --no-win --batch-mode run.simics
```

The machine will boot to Linux, mount the disk, and run the driver application. The
driver application will call into the kernel module, and the fuzzer will start fuzzing.