# Target Software Boilerplate

Creating the external Buildroot tree requires several small
files to be in just the right places.

## Dockerfile

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

## Buildroot Boilerplate

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

## Kernel Module Package Boilerplate

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

## Kernel Module Boilerplate

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
