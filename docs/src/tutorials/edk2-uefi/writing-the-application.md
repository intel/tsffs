# Writing the Application

Learning to write EDK2 applications in general is well outside the scope of this
tutorial, but we will cover the general workflow.

First, we will create a `src` directory with the following files:

* `PlatformBuild.py` - the
  [stuart](https://github.com/tianocore/tianocore.github.io/wiki/How-to-Build-With-Stuart)
  build description file for our target software. You can read more about the pytool
  extensions [here](https://www.tianocore.org/edk2-pytool-extensions/integrate/porting/).
* `Tutorial.dsc` - The EDK2 description file for building our target software
* `Tutorial.inf` - The EDK2 info file for building our target software
* `Tutorial.c` - Our C source file
* `tsffs.h` - The header file from the `harness` directory in the repository
  for our target architecture

We'll cover the auxiliary and build files first, then we'll cover the source code.

## PlatformBuild.py

As mentioned above, this file is used by the EDK2 PyTools (also known as Stuart) to
configure tools for building our target software. You can read about
[stuart](https://github.com/tianocore/tianocore.github.io/wiki/How-to-Build-With-Stuart)
and the
[PyTool Extensions](https://www.tianocore.org/edk2-pytool-extensions/integrate/porting/)
.

We specify our workspace, scopes, packages, and so forth:

```python
from os.path import abspath, dirname, join
from typing import Iterable, List
from edk2toolext.environment.uefi_build import UefiBuilder
from edk2toolext.invocables.edk2_platform_build import BuildSettingsManager
from edk2toolext.invocables.edk2_setup import RequiredSubmodule, SetupSettingsManager
from edk2toolext.invocables.edk2_update import UpdateSettingsManager


class TutorialSettingsManager(
    UpdateSettingsManager, SetupSettingsManager, BuildSettingsManager
):
    def __init__(self) -> None:
        script_path = dirname(abspath(__file__))
        self.ws = script_path

    def GetWorkspaceRoot(self) -> str:
        return self.ws

    def GetActiveScopes(self) -> List[str]:
        return ["Tutorial"]

    def GetPackagesSupported(self) -> Iterable[str]:
        return ("Tutorial",)

    def GetRequiredSubmodules(self) -> Iterable[RequiredSubmodule]:
        return []

    def GetArchitecturesSupported(self) -> Iterable[str]:
        return ("X64",)

    def GetTargetsSupported(self) -> Iterable[str]:
        return ("DEBUG",)

    def GetPackagesPath(self) -> Iterable[str]:
        return [abspath(join(self.GetWorkspaceRoot(), ".."))]

class PlatformBuilder(UefiBuilder):
    def SetPlatformEnv(self) -> int:
        self.env.SetValue(
            "ACTIVE_PLATFORM", "Tutorial/Tutorial.dsc", "Platform hardcoded"
        )
        self.env.SetValue("PRODUCT_NAME", "Tutorial", "Platform hardcoded")
        self.env.SetValue("TARGET_ARCH", "X64", "Platform hardcoded")
        self.env.SetValue("TOOL_CHAIN_TAG", "GCC", "Platform Hardcoded", True)
        return 0
```

## Tutorial.inf

The exact meaning of all the entries in the `Tutorial.inf` file is out of scope of this
tutorial, but in general this file declares the packages and libraries our application
needs.

```txt
[Defines]
  INF_VERSION                    = 0x00010005
  BASE_NAME                      = Tutorial
  FILE_GUID                      = 6987936E-ED34-44db-AE97-1FA5E4ED2116
  MODULE_TYPE                    = UEFI_APPLICATION
  VERSION_STRING                 = 1.0
  ENTRY_POINT                    = UefiMain
  UEFI_HII_RESOURCE_SECTION      = TRUE

[Sources]
  Tutorial.c

[Packages]
  CryptoPkg/CryptoPkg.dec
  MdeModulePkg/MdeModulePkg.dec
  MdePkg/MdePkg.dec

[LibraryClasses]
  BaseCryptLib
  SynchronizationLib
  UefiApplicationEntryPoint
  UefiLib
```

## Tutorial.dsc

The descriptor file also declares classes and libraries that are needed to build the
whole platform including our application and requisite additional libraries.

```txt
[Defines]
  PLATFORM_NAME                  = Tutorial
  PLATFORM_GUID                  = 0458dade-8b6e-4e45-b773-1b27cbda3e06
  PLATFORM_VERSION               = 0.01
  DSC_SPECIFICATION              = 0x00010006
  OUTPUT_DIRECTORY               = Build/Tutorial
  SUPPORTED_ARCHITECTURES        = X64
  BUILD_TARGETS                  = DEBUG|RELEASE|NOOPT
  SKUID_IDENTIFIER               = DEFAULT

!include MdePkg/MdeLibs.dsc.inc
!include CryptoPkg/CryptoPkg.dsc

[LibraryClasses]
  BaseCryptLib|CryptoPkg/Library/BaseCryptLib/BaseCryptLib.inf
  BaseLib|MdePkg/Library/BaseLib/BaseLib.inf
  BaseMemoryLib|MdePkg/Library/BaseMemoryLib/BaseMemoryLib.inf
  DevicePathLib|MdePkg/Library/UefiDevicePathLib/UefiDevicePathLib.inf
  HobLib|MdePkg/Library/DxeHobLib/DxeHobLib.inf
  IntrinsicLib|CryptoPkg/Library/IntrinsicLib/IntrinsicLib.inf
  IoLib|MdePkg/Library/BaseIoLibIntrinsic/BaseIoLibIntrinsic.inf
  MemoryAllocationLib|MdePkg/Library/UefiMemoryAllocationLib/UefiMemoryAllocationLib.inf
  OpensslLib|CryptoPkg/Library/OpensslLib/OpensslLib.inf
  PcdLib|MdePkg/Library/BasePcdLibNull/BasePcdLibNull.inf
  PrintLib|MdePkg/Library/BasePrintLib/BasePrintLib.inf
  SynchronizationLib|MdePkg/Library/BaseSynchronizationLib/BaseSynchronizationLib.inf
  UefiApplicationEntryPoint|MdePkg/Library/UefiApplicationEntryPoint/UefiApplicationEntryPoint.inf
  UefiBootServicesTableLib|MdePkg/Library/UefiBootServicesTableLib/UefiBootServicesTableLib.inf
  UefiLib|MdePkg/Library/UefiLib/UefiLib.inf
  UefiRuntimeServicesTableLib|MdePkg/Library/UefiRuntimeServicesTableLib/UefiRuntimeServicesTableLib.inf
  TimerLib|UefiCpuPkg/Library/CpuTimerLib/BaseCpuTimerLib.inf

[Components]
  Tutorial/Tutorial.inf
```

## tsffs.h

Copy this file from the TSFFS repository's `harness` directory. It provides macros for
compiling in the harness so the target software can communicate with and receive
test cases from the fuzzer.

## Tutorial.c

This is our actual source file. We'll be fuzzing a real EDK2 API: `X509VerifyCert`,
which tries to verify a certificate was issued by a given certificate authority.

```c
#include <Library/BaseCryptLib.h>
#include <Library/MemoryAllocationLib.h>
#include <Library/UefiApplicationEntryPoint.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Library/UefiLib.h>
#include <Uefi.h>

#include "tsffs.h"

void hexdump(UINT8 *buf, UINTN size) {
  for (UINTN i = 0; i < size; i++) {
    if (i != 0 && i % 26 == 0) {
      Print(L"\n");
    } else if (i != 0 && i % 2 == 0) {
      Print(L" ");
    }
    Print(L"%02x", buf[i]);
  }
  Print(L"\n");
}

EFI_STATUS
EFIAPI
UefiMain(IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
  UINTN MaxInputSize = 0x1000;
  UINTN InputSize = MaxInputSize;
  UINT8 *Input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(MaxInputSize));

  if (!Input) {
    return EFI_OUT_OF_RESOURCES;
  }

  Print(L"Input: %p Size: %d\n", Input, InputSize);
  UINT8 *Cert = Input;
  UINTN CertSize = InputSize / 2;
  UINT8 *CACert = (Input + CertSize);
  UINTN CACertSize = CertSize;

  Print(L"Certificate:\n");
  hexdump(Cert, CertSize);
  Print(L"CA Certificate:\n");
  hexdump(CACert, CACertSize);

  BOOLEAN Status = X509VerifyCert(Cert, CertSize, CACert, CACertSize);

  if (Input) {
    FreePages(Input, EFI_SIZE_TO_PAGES(MaxInputSize));
  }

  return EFI_SUCCESS;
}

```

Now that we have some code, we'll move on to building.