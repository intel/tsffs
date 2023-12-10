# Building the BIOS

Working from the same `Dockerfile` we obtained sources into, we'll set things up to
build the BIOS image. First, we need to set up a few environment variables:

```dockerfile
ENV EDK_TOOLS_PATH=/workspace/edk2/BaseTools/
ENV PACKAGES_PATH="/workspace/edk2:/workspace/edk2-platforms:/workspace/edk2-non-osi"
ENV WORKSPACE=/workspace/
```

These variables are used by EDK2 to find its own sources and binaries.

Next, we'll build the *Base Tools*. You can read more about the *Base Tools* in the EDK2
[build
system](https://tianocore-docs.github.io/edk2-BuildSpecification/release-1.28/4_edk_ii_build_process_overview/41_edk_ii_build_system.html#41-edk-ii-build-system)
documentation.

```dockerfile
RUN source edk2/edksetup.sh && \
    make -C edk2/BaseTools/
```

With the *Base Tools* built, we can build the BIOS. We directly follow the directions
provided, and you can read more about the process, what settings are available for the
BIOS (in particular, how to change the BIOS stages) [in the
repo](https://github.com/tianocore/edk2-platforms/blob/f446fff05003f69a4396b2ec375301ecb5f63a2a/Platform/Intel/Readme.md).

First, we'll change into the tree containing the platform code for all the Intel
platforms, then use the Intel-provided build script to select our board, toolchain,
and debug mode (in this case, enabled).

```dockerfile
WORKDIR /workspace/edk2-platforms/Platform/Intel

# Build SimicsOpenBoardPkg
RUN source /workspace/edk2/edksetup.sh && \
    python build_bios.py -p BoardX58Ich10X64 -d -t GCC
```