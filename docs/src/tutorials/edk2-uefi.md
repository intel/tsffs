# Fuzzing an EDK2 UEFI Application

This tutorial will walk you through the entire process of creating, building, and
fuzzing a UEFI application built with EDK2 on the x86-64 platform. The completed example
code and fuzzing script can be found in the [edk2-uefi tutorial
directory](https://github.com/intel/tsffs/tree/main/examples/tutorials/edk2-uefi).

## Writing the Application

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
* `tsffs-gcc-x86_64.h` - The header file from the `harness` directory in the repository
  for our target architecture

We'll cover the auxiliary and build files first, then we'll cover the source code.

### PlatformBuild.py

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

### Tutorial.inf

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

### Tutorial.dsc

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

### tsffs-gcc-x86_64.h

Copy this file from the TSFFS repository's `harness` directory. It provides macros for
compiling in the harness so the target software can communicate with and receive
test cases from the fuzzer.

### Tutorial.c

This is our actual source file. We'll be fuzzing a real EDK2 API: `X509VerifyCert`,
which tries to verify a certificate was issued by a given certificate authority.

```c
#include <Library/BaseCryptLib.h>
#include <Library/MemoryAllocationLib.h>
#include <Library/UefiApplicationEntryPoint.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Library/UefiLib.h>
#include <Uefi.h>

#include "tsffs-gcc-x86_64.h"

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

## Building the Application

To build the application, we'll use the EDK2 docker containers provided by tianocore. In
the directory that contains your `src` directory, create a `Dockerfile`:

```dockerfile
FROM ghcr.io/tianocore/containers/ubuntu-22-build:a0dd931
ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV EDK2_REPO_URL "https://github.com/tianocore/edk2.git"
ENV EDK2_REPO_HASH "d189de3b0a2f44f4c9b87ed120be16569ea19b51"
ENV EDK2_PATH "/edk2"

RUN git clone "${EDK2_REPO_URL}" "${EDK2_PATH}" && \
    git -C "${EDK2_PATH}" checkout "${EDK2_REPO_HASH}" && \
    python3 -m pip install --no-cache-dir -r "${EDK2_PATH}/pip-requirements.txt" && \
    stuart_setup -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC&& \
    stuart_update -c "${EDK2_PATH}/.pytool/CISettings.py" TOOL_CHAIN_TAG=GCC

COPY src "${EDK2_PATH}/Tutorial/"

RUN stuart_setup -c "${EDK2_PATH}/Tutorial/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    stuart_update -c "${EDK2_PATH}/Tutorial/PlatformBuild.py" TOOL_CHAIN_TAG=GCC && \
    python3 "${EDK2_PATH}/BaseTools/Edk2ToolsBuild.py" -t GCC

WORKDIR "${EDK2_PATH}"

RUN source ${EDK2_PATH}/edksetup.sh && \
    ( stuart_build -c ${EDK2_PATH}/Tutorial/PlatformBuild.py TOOL_CHAIN_TAG=GCC \
    EDK_TOOLS_PATH=${EDK2_PATH}/BaseTools/ \
    || ( cat ${EDK2_PATH}/Tutorial/Build/BUILDLOG.txt && exit 1 ) )
```

This Dockerfile will obtain the EDK2 source and compile the BaseTools, then copy our
`src` directory into the EDK2 repository as a new package and build the package.

We will want to get our built UEFI application from the container, which we can
do using the `docker cp` command. There are a few files we want to copy, so we'll
use this script `build.sh` to automate the process:

```sh
#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
IMAGE_NAME="tsffs-tutorial-edk2-uefi"
CONTAINER_UID=$(echo "${RANDOM}" | sha256sum | head -c 8)
CONTAINER_NAME="${IMAGE_NAME}-tmp-${CONTAINER_UID}"

mkdir -p "${SCRIPT_DIR}/project/"
docker build -t "${IMAGE_NAME}" -f "Dockerfile" "${SCRIPT_DIR}"
docker create --name "${CONTAINER_NAME}" "${IMAGE_NAME}"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.efi" \
    "${SCRIPT_DIR}/project/Tutorial.efi"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.map" \
    "${SCRIPT_DIR}/project/Tutorial.map"
docker cp \
    "${CONTAINER_NAME}:/edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/Tutorial.debug" \
    "${SCRIPT_DIR}/project/Tutorial.debug"
docker rm -f "${CONTAINER_NAME}"
```

The script will build the image, create a container using it, copy the relevant files
to our host machine (in a `project` directory), then delete the container.

## Testing the Application

Before we harness the application for fuzzing, we should test it to make sure it runs.

Before this step, you'll need to have the TSFFS SIMICS package installed in your system
by following the [setup steps](../setup/README.md) or by installing a prebuilt `ispm`
package. You'll also need the SIMICS base package (1000), the QSP-x86 package (2096),
and the QSP-CPU (8112) package. All three are available in the public simics release.

You can check that you have the package installed by running:

```sh
ispm packages --list-installed
```

You should see (at least, but likely more packages):

```txt
Installed Base Packages
 Package Number  Name         Version  Installed Paths
 1000            Simics-Base  6.0.169  /home/rhart/simics/simics-6.0.169

Installed Addon Packages
 Package Number  Name             Version    Installed Paths
 2096            QSP-x86          6.0.70     /home/rhart/simics/simics-qsp-x86-6.0.70
 8112            QSP-CPU          6.0.17     /home/rhart/simics/simics-qsp-cpu-6.0.17
 31337           TSFFS            6.0.0      /home/rhart/simics/simics-tsffs-6.0.0 
```

in the list!

### Create a Project

The build script for our application created a `project` directory for us if it did not
exist, so we'll instantiate that directory as our project with `ispm`:

```sh
ispm projects project --create 1000-latest 2096-latest 8112-latest 31337-latest \
  --ignore-existing-files
cd project
```

### Get the Minimal Boot Disk

The TSFFS repository provides a boot disk called `minimal_boot_disk.craff` which
provides a filesystem and the *Simics Agent* to allow us to easily download our UEFI
application to the filesystem so we can run it. Copy the file
`examples/rsrc/minimal_boot_disk.craff` into your `project` directory.

### Create a Script

Our initial script will load (but not use *yet*) the TSFFS module, then configure and
start our simple x86-64 platform and run our UEFI application. In the `project`
directory, create `run.simics`:

```simics
# Load the TSFFS module (to make sure we can load it)
load-module tsffs

# Load the UEFI shell target with out boot disk
load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

script-branch {
    # Wait for boot
    bp.time.wait-for seconds = 15
    qsp.serconsole.con.input "\n"
    bp.time.wait-for seconds = .5
    # Change to the FS0: filesystem (which is our mounted minimal_boot_disk.craff)
    qsp.serconsole.con.input "FS0:\n"
    bp.time.wait-for seconds = .5
    # Start the UEFI agent manager (the host side connection from the SIMICS agent)
    local $manager = (start-agent-manager)
    # Run the SIMICS agent to download our Tutorial.efi application into the simulated
    # filesystem
    qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/Tutorial.efi") + "\n")
    bp.time.wait-for seconds = .5
    # Run our Tutorial.efi application
    qsp.serconsole.con.input "Tutorial.efi\n"
}

script-branch {
  # Wait until the application is done running, then quit
  bp.time.wait-for seconds = 30
  quit 0
}

# Start!
run
```

### Run the Test Script

Run the script:

```sh
./simics -no-gui --no-win --batch-mode run.simics
```

The machine will boot, the UEFI application will run and dump out the contents of the
certificates, then the simulation will exit (this is because we passed `--batch-mode`).

Now that everything works, we're ready to move on to harnessing!

## Harnessing the Application

Note that as written, our application will be running the certificate verification
with uninitialized allocated memory. We want to run it instead using our fuzzer input,
so we need to add harnessing. We've already `#include`-ed our harness header file and
loaded the TSFFS module in our simulation, so we're halfway there.

### Adding Harness Code

In our `Tutorial.c` file, we'll add a few lines of code so that our main function looks
like this (the rest of the code can stay the same):

```c
EFI_STATUS
EFIAPI
UefiMain(IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
  UINTN MaxInputSize = 0x1000;
  UINTN InputSize = MaxInputSize;
  UINT8 *Input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(MaxInputSize));

  if (!Input) {
    return EFI_OUT_OF_RESOURCES;
  }

  HARNESS_START(Input, &InputSize);

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

  if (Status) {
    HARNESS_ASSERT();
  } else {
    HARNESS_STOP();
  }

  if (Input) {
    FreePages(Input, EFI_SIZE_TO_PAGES(MaxInputSize));
  }

  return EFI_SUCCESS;
}
```

First, we invoke `HARNESS_START` with two arguments:

* The pointer to our buffer -- this is where the fuzzer will write each testcase
* The pointer to our maximum input size (aka, the size of the buffer). The fuzzer
  records the initial value and will truncate testcases to it so it does not cause
  buffer overflows, and will write the actual size of the input here each iteration
  so we know how much data the fuzzer has given us.

Then, we let the function we are testing run normally. If a CPU exception happens, the
fuzzer will pick it up and treat the input as a "solution" that triggers a configured
exceptional condition.

Finally, we check the status of certificate verification. If validation was successful,
we `HARNESS_ASSERT` because we *really* do not expect this to happen, and we want to
know if it does happen. This type of assertion can be used for any condition that you
want to fuzz for in your code. If the status is a certificate verification failure, we
`HARNESS_STOP`, which just tells the fuzzer we completed our test under normal
conditions and we should run again.

Re-compile the application by running the build script.


### Obtain a Corpus

The fuzzer will take input from the `corpus` directory in the project directory, so
we'll create that directory and add some sample certificate files in DER format as
our input corpus.

```sh
mkdir corpus
curl -L -o corpus/0 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/0
curl -L -o corpus/1 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/1
curl -L -o corpus/2 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/2
curl -L -o corpus/3 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/3
```

### Configuring the Fuzzer

Even though we loaded the fuzzer module, it didn't run previously because we did not
instantiate and configure it. Let's do that now. At the top of your `run.simics`
script, we'll add each of the following lines.

First, we need to create an actual `tsffs` object to instantiate the fuzzer.

```simics
load-module tsffs # You should already have this
@tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
```

Next, we'll set the log level to maximum for demonstration purposes:

```simics
tsffs.log-level 4
```

Then, we'll set the fuzzer to start and stop on the magic harnesses we just compiled
into our UEFI application. This is the default, so these calls can be skipped in real
usage unless you want to change the defaults, they are just provided here for
completeness.

```simics
@tsffs.iface.tsffs.set_start_on_harness(True)
@tsffs.iface.tsffs.set_stop_on_harness(True)
```

We'll set up our "solutions" which are all the exceptional conditions that we want to
fuzz for. In our case, these are timeouts (we'll set the timeout to 3 seconds) to detect
hangs, and CPU exceptions. we'll enable exceptions 13 for general protection fault and
14 for page faults to detect out of bounds reads and writes.

```simics
@tsffs.iface.tsffs.set_timeout(3.0)
@tsffs.iface.tsffs.add_exception_solution(13)
@tsffs.iface.tsffs.add_exception_solution(14)
```

We'll tell the fuzzer where to take its corpus and save its solutions. The fuzzer will
take its corpus from the `corpus` directory and save solutions to the `solutions`
directory in the project by default, so this call can be skipped in real usage unless
you want to change the defaults.

```simics
@tsffs.iface.tsffs.set_corpus_directory("%simics%/corpus")
@tsffs.iface.tsffs.set_solutions_directory("%simics%/solutions")
```

We'll also *delete* the following code from the `run.simics` script:

```simics
script-branch {
  bp.time.wait-for seconds = 30
  quit 0
}
```

Since we'll be fuzzing, we don't want to exit!


## Running the Fuzzer

Now that we have configured the fuzzer and harnessed our target application, it's time
to run again:

```sh
./simics -no-gui --no-win run.simics
```

Press Ctrl+C at any time to stop the fuzzing process and return to the SIMICS CLI.
From there you can run `continue` to continue the fuzzing process.

## Reproducing Runs

It is unlikely you'll find any bugs with this fuzzer (if you do, report them to edk2!),
but we can still test the "repro" functionality which allows you to replay an execution
of a testcase from an input file. After pressing Ctrl+C during execution, list the
corpus files (tip: `!` in front of a line in the SIMICS console lets you run shell
commands):

```txt
simics> !ls corpus
0
1
2
3
4385dc33f608888d
5b7dc5642294ccb9
```

You will probably have several files. Let's examine testcase `4385dc33f608888d`:

```txt
simics> !hexdump -C corpus/4385dc33f608888d | head -n 2
00000000  30 82 04 e8 30 82 04 53  a0 03 02 01 02 02 1d 58  |0...0..S.......X|
00000010  74 4e e3 aa f9 7e e8 ff  2f 67 53 31 6e 62 3d 1e  |tN...~../gS1nb=.|
```

We can tell the fuzzer that we want to run with this specific input by using:

```txt
simics> @tsffs.iface.tsffs.repro("%simics%/corpus/4385dc33f608888d")
```

The simulation will run once with this input, then output a message that you can replay
the simulation by running:

```txt
simics> reverse-to start
```

From here, you can examine memory and registers (with `x`), single step execution (`si`)
and more! Check out the SIMICS documentation and explore all the deep debugging
capabilities that SIMICS offers. When you're done exploring, run `c` to continue.

You can change the testcase you are examining by choosing a different one with
`tsffs.iface.tsffs.repro`, but you cannot resume fuzzing after entering repro mode due
to inconsistencies with the simulated system clock.

## Optimizing

There is a lot of room to optimize this test scenario. You'll notice that with full
logging on (and full hexdumping of input on), each run takes *over a second* for around
`0.3` executions per second. While this is much better than nothing, his is quite poor
performance for effective fuzzing.

### Remove Target Software Output

First, we'll `#ifdef` out our print statements in our target software:

```c
EFI_STATUS
EFIAPI
UefiMain(IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
  UINTN MaxInputSize = 0x1000;
  UINTN InputSize = MaxInputSize;
  UINT8 *Input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(MaxInputSize));

  if (!Input) {
    return EFI_OUT_OF_RESOURCES;
  }

  HARNESS_START(Input, &InputSize);

#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
  Print(L"Input: %p Size: %d\n", Input, InputSize);
#endif
  UINT8 *Cert = Input;
  UINTN CertSize = InputSize / 2;
  UINT8 *CACert = (Input + CertSize);
  UINTN CACertSize = CertSize;

#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
  Print(L"Certificate:\n");
  hexdump(Cert, CertSize);
  Print(L"CA Certificate:\n");
  hexdump(CACert, CACertSize);
#endif

  BOOLEAN Status = X509VerifyCert(Cert, CertSize, CACert, CACertSize);

  if (Status) {
    HARNESS_ASSERT();
  } else {
    HARNESS_STOP();
  }

  if (Input) {
    FreePages(Input, EFI_SIZE_TO_PAGES(MaxInputSize));
  }

  return EFI_SUCCESS;
}
```

```txt
[tsffs info] Stopped after 1107 iterations in 11.048448 seconds (100.19507 exec/s).
```

We are now running at 100+ iterations per second! This is a massive increase. Let's take
it a little further.

### Turn Down Logging

TSFFS logs a large amount of redundant information at high log levels (primarily for
debugging purposes). You can reduce the amount of information printed by setting:

```simics
tsffs.log-level 2
```

Where `0` is the lowest (error) and `4` is the highest (trace) logging level. Errors are
always displayed. This can typically buy a few exec/s. Note that fuzzer status messages
are printed at a logging level of `info` (2), so you likely want to at least set the
log level to 2.

This can buy us a few executions per second:

```txt
[tsffs info] [Testcase #0] run time: 0h-0m-42s, clients: 1, corpus: 21, objectives: 0, executions: 4792, exec/sec: 112.5
```

### Shorten The Testcase

In our case, we are calling one function, sandwiched between `HARNESS_START` and
`HARNESS_STOP`. There is almost nothing we can do to shorten the runtime of each
individual run here, but this is a good technique to keep in mind for your future
fuzzing efforts.

### Run More Instances

TSFFS includes stages for flushing the queue and synchronizing the queue from a shared
corpus directory. This means you can run as many instances of TSFFS as you'd like in
parallel, and they will periodically pick up new corpus entries from each other.
Execution speed scales approximately linearly across cores.

We'll launch 8 instances, all in batch mode, using `tmux`:

```sh
#!/bin/bash

SESSION_NAME="my-tsffs-campaign"

# Create a new tmux session or attach to an existing one
tmux new-session -d -s "$SESSION_NAME"

# Loop to create 8 windows and run the command in each window
for i in {1..8}; do
    # Create a new window
    tmux new-window -t "$SESSION_NAME:$i" -n "${SESSION_NAME}-window-$i"

    # Run the command in the new window
    tmux send-keys -t "$SESSION_NAME:$i" "./simics -no-gui --no-win --batch-mode run.simics" C-m
done

# Attach to the tmux session
tmux attach-session -t "$SESSION_NAME"
```

You can select each window with (for example to select window 3 `Ctrl+b 3`), and you can
detach and leave the campaign running in the background with `Ctrl+b d`. After detaching
you can reattach using the last command in the script `tmux attach-session -t
my-tsffs-campaign`. Running 8 instances of the fuzzer means approximately 8 times the
exec/s of a single instance, however each instance operates independently, so bug
finding does not scale in a correspondingly linear fashion. Regardless, the common
wisdom of more iterations being better holds.