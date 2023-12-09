FROM ghcr.io/tianocore/containers/fedora-37-build:a0dd931

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /workspace/

RUN git clone https://github.com/tianocore/edk2.git && \
    git clone https://github.com/tianocore/edk2-platforms.git && \
    git clone https://github.com/tianocore/edk2-non-osi.git && \
    git clone https://github.com/IntelFsp/FSP.git

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

ENV EDK_TOOLS_PATH=/workspace/edk2/BaseTools/
ENV PACKAGES_PATH="/workspace/edk2:/workspace/edk2-platforms:/workspace/edk2-non-osi"
ENV WORKSPACE=/workspace/

RUN source edk2/edksetup.sh && \
    make -C edk2/BaseTools/

WORKDIR /workspace/edk2-platforms/Platform/Intel

# Build SimicsOpenBoardPkg
RUN source /workspace/edk2/edksetup.sh && \
    python build_bios.py -p BoardX58Ich10X64 -d -t GCC
    
# Build ALderlakeOpenBoardPkg
# NOTE: This patch is needed as of platforms version f446fff due to typos
COPY <<EOF /tmp/edk2-platforms.patch
diff --git a/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/OpenBoardPkg.dsc b/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/OpenBoardPkg.dsc
index 4cdc9c0192..5686319bad 100644
--- a/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/OpenBoardPkg.dsc
+++ b/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/OpenBoardPkg.dsc
@@ -479,3 +479,6 @@ $(PLATFORM_SI_BIN_PACKAGE)/Microcode/MicrocodeUpdates.inf
 !include $(PLATFORM_SI_PACKAGE)/Product/Alderlake/SiPkgBuildOption.dsc
 !include OpenBoardPkgBuildOption.dsc
 
+[BuildOptions]
+  GCC:*_*_*_CC_FLAGS = -Wno-error=unused-but-set-variable -Wno-error=incompatible-pointer-types -Wno-error=unused-const-variable -Wno-error=unused-function
+
diff --git a/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/build_config.cfg b/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/build_config.cfg
index ae391bec8c..f1374b442a 100644
--- a/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/build_config.cfg
+++ b/Platform/Intel/AlderlakeOpenBoardPkg/AlderlakePRvp/build_config.cfg
@@ -26,7 +26,7 @@ TARGET = DEBUG
 TARGET_SHORT = D
 PERFORMANCE_BUILD = FALSE
 FSP_WRAPPER_BUILD = TRUE
-FSP_BIN_PKG = AlderlakeFspBinPkg/Client/AlderlakeP
+FSP_BIN_PKG = AlderLakeFspBinPkg/Client/AlderLakeP
 FSP_PKG_NAME = AlderlakeSiliconPkg
 FSP_BINARY_BUILD = FALSE
 FSP_TEST_RELEASE = FALSE
diff --git a/Platform/Intel/AlderlakeOpenBoardPkg/OpenBoardPkg.dec b/Platform/Intel/AlderlakeOpenBoardPkg/OpenBoardPkg.dec
index 0f0ed386f6..e10e10f966 100644
--- a/Platform/Intel/AlderlakeOpenBoardPkg/OpenBoardPkg.dec
+++ b/Platform/Intel/AlderlakeOpenBoardPkg/OpenBoardPkg.dec
@@ -297,7 +297,7 @@ gBoardModuleTokenSpaceGuid.PcdDisableVpdGpioTable|FALSE|BOOLEAN|0x50000015
   <Packages>
     MdePkg/MdePkg.dec
     AlderlakeSiliconPkg/SiPkg.dec
-    AlderLakeOpenBoardPkg/OpenBoardPkg.dec
+    AlderlakeOpenBoardPkg/OpenBoardPkg.dec
 }
 [PcdsDynamic, PcdsDynamicEx]
 
diff --git a/Platform/Intel/build.cfg b/Platform/Intel/build.cfg
index 6ab2ced1a5..eb83f2bef6 100644
--- a/Platform/Intel/build.cfg
+++ b/Platform/Intel/build.cfg
@@ -71,4 +71,4 @@ WilsonCityRvp = WhitleyOpenBoardPkg/WilsonCityRvp/build_config.cfg
 BoardTiogaPass = PurleyOpenBoardPkg/BoardTiogaPass/build_config.cfg
 JunctionCity = WhitleyOpenBoardPkg/JunctionCity/build_config.cfg
 Aowanda = WhitleyOpenBoardPkg/Aowanda/build_config.cfg
-AlderLakePRvp = AlderLakeOpenBoardPkg/AlderLakePRvp/build_config.cfg
+AlderlakePRvp = AlderlakeOpenBoardPkg/AlderlakePRvp/build_config.cfg
EOF


RUN source /workspace/edk2/edksetup.sh && \
    git apply --whitespace=fix /tmp/edk2-platforms.patch && \
    python build_bios.py -p AlderlakePRvp -d -t GCC
