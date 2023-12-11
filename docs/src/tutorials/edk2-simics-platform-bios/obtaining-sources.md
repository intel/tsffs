# Obtaining Sources

Everything we need to build a BIOS that we can boot in SIMICS is open source!

We'll need four repositories:

- [edk2](https://github.com/tianocore/edk2.git)
- [edk2-platforms](https://github.com/tianocore/edk2-platforms.git)
- [edk2-non-osi](https://github.com/tianocore/edk2-non-osi.git)
- [Intel FSP](https://github.com/IntelFsp/FSP.git)

EDK2 is the reference implementation of the UEFI specification, and EDK2 Platforms
provides platform builds for various open boards. These include the board we'll be
using, the generic X58I reference platform, as well Intel Reference Validation Platforms
for several other processor generations.

EDK2's dependency chain is not large, but the easiest way to work with EDK2 by far is
to use Docker. We'll start building up a `Dockerfile` to obtain the sources.

```dockerfile
FROM ghcr.io/tianocore/containers/fedora-37-build:a0dd931

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG PROJECT=

WORKDIR "$PROJECT"
```

We'll start from Tianocore's Fedora 37 build image, which provides all the dependencies
needed to build EDK2 and EDK2-based platforms. We make sure we set the `pipefail` option
in the BASH shell. We're going to set our workdir to a build argument called `PROJECT`,
which we'll pass in when we build the container. This will let us set the path inside
and the path outside the container where we build our code to the *same path* which
we will need later when we use the auxiliary information EDK2 provides (in the form of
`.map` files) to enable source-code debugging and breakpoints in our firmware.

Next, we'll obtain our sources. Note that commit hashes are provided for all the open
source repositories. It's possible (or hopefully, likely!) these instructions will work
on newer commits, but to ensure the instructions here are reproducible, we check out
a specific HEAD.

```dockerfile
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
```

Note that for every repository, we check out the submodules. If we fail to do this,
we'll get an arcane error about a missing binary later on when we build.

That's everything we need!