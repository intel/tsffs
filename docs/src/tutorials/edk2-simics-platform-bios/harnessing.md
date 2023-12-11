# Harnessing

Just before the time of writing, a vulnerability received significant publicity this
week concerning the boot logo parser in many vendors BIOS images called
[LogoFAIL](https://binarly.io/posts/finding_logofail_the_dangers_of_image_parsing_during_system_boot/).
In a [press release](https://min.news/en/tech/128c34878b2b582065c1e05379912294.html) the
vulnerability finders noted that "Our fuzz testing and subsequent vulnerability triage
results clearly indicate that these image parsers were never tested by IBV or OEM". This
[may not be accurate](https://github.com/tianocore/edk2-staging/tree/HBFA/HBFA/UefiHostFuzzTestCasePkg/TestCase/MdeModulePkg/Library/BaseBmpSupportLib),
but even so let's change that for our tutorial platform and get it fuzzed!

## Writing a Harness

We'll target the `Logo.c` file of `DxeLogoLib`. To harness this function, we want to
inject our fuzzer testcase just before the call to `ConvertBmpToGopBlt`. This function
is called like:

```c
Status = ConvertBmpToGopBlt (
            ImageData,
            ImageSize,
            (VOID **) &Blt,
            &BltSize,
            &Height,
            &Width
            );
```

The `ImageData` and `ImageSize` are returned by either the OEMBadging protocol:

```c
Status = Badging->GetImage (
                    Badging,
                    &Instance,
                    &Format,
                    &ImageData,
                    &ImageSize,
                    &Attribute,
                    &CoordinateX,
                    &CoordinateY
                    );
```

Or, if the badging protocol is not registered, it's obtained from the RAW section of any
Firmware Volume (FV) (the `LogoFile` here is a pointer to the PCD-defined GUID to look
up the logo file, which is a BMP file
[Logo.bmp](https://raw.githubusercontent.com/tianocore/edk2-platforms/f446fff05003f69a4396b2ec375301ecb5f63a2a/Platform/Intel/SimicsOpenBoardPkg/Logo/Logo.bmp)):

```c
Status = GetSectionFromAnyFv (LogoFile, EFI_SECTION_RAW, 0, (VOID **) &ImageData, &ImageSize);
```

Either way, we end up populating `ImageData` with our data and setting `ImageSize` equal
to the size of the image data. It's important to note that here, `ImageData` is only
technically untrusted input. It could be overwritten using an SPI programming chip (if
this were a real board), or a malicious user with the ability to write flash could
overwrite it on disk. This isn't a "visit a website and get owned" type of attack, but
it is a good example of how unexpected vectors could present a danger to very high value
computing systems.

We want to insert our fuzzer's testcases into the `ImageData` buffer, and we can support
testcases up to `ImageSize`. We could use a massive original image to ensure that
we have enough space, but we'll just use the default one.

We'll set our harness (which, recall, also triggers the initial snapshot, so we want
it as close to the code under test as possible) immediately before the call:


```c
HARNESS_START(ImageData, &ImageSize);
Status = ConvertBmpToGopBlt (
            ImageData,
            ImageSize,
            (VOID **) &Blt,
            &BltSize,
            &Height,
            &Width
            );
```

When the macro is called for the first time, a snapshot will be taken of the full
system, and the `ImageSize` value will be saved. Then, each fuzzing iteration, the new
test case will be truncated to `ImageSize` bytes and written to `ImageData`.

We also want to tell the fuzzer to stop executing before we return from the
`EnableBootLogo` function, so we place a call to `HARNESS_STOP()` before every `return`
statement for the rest of the function after this point.

## Making the Code Vulnerable

Because this is a tutorial, it'll be more fun if we make this code actually vulnerable
to LogoFAIL. If we take a walk through `ConvertBmpToGopBlt`, you'll notice two things:

* There is a check on the result of `AllocatePool`, so the first vulnerability where
  failure to allocate memory occurs isn't applicable. If we just removed the check here,
  we'd be vulnerable to a failure to allocate memory and subsequent dereferencing of
  an uninitialized pointer.

  ```c
  *GopBlt     = AllocatePool (*GopBltSize);
  IsAllocated = TRUE;
  if (*GopBlt == NULL) {
       return EFI_OUT_OF_RESOURCES;
  }
  ```

* We're really, *really* close to having the vulnerability where `PixelHeight` can be
  zero and `PixelWidth` can be large. If we just had `Height <= BmpHeader->PixelHeight`
  here, we'd be vulnerable, but because `BmpHeader->PixelHeight` is strictly greater
  than `Height` and unsigned, we'll never be able to have a case (as is) where
  `BmpHeader->PixelHeight - Height - 1 < 0`.

  ```c
  for (Height = 0; Height < BmpHeader->PixelHeight; Height++) {
    Blt = &BltBuffer[(BmpHeader->PixelHeight - Height - 1) * BmpHeader->PixelWidth];
  ```

This explains why this platform code wasn't in the Binarly blog post, but just for fun
we'll change both of these things when we add our harnessing code, for
*demonstration purposes only*. For the second case, we'll just add an `ASSERT` statement
when `PixelHeight == 0`, because changing the for loop condition to `Height <=
BmpHeader->PixelHeight` would cause a crash on *every* input, and will make the fuzzer
complain that we have no interesting testcases.

## Adding the Harness

We'll add our harness in the form of a patch to `edk2-platforms`.

Our `Dockerfile` from [previously](building-bios.md) just needs a couple modifications.

First, we need to copy
[tsffs-gcc-x86_64.h](https://github.com/intel/tsffs/blob/main/harness/tsffs-gcc-x86_64.h)
from the `harness` directory of the [repository](https://github.com/intel/tsffs/) and
put it next to our `Dockerfile`. Then, just before the last `RUN` step (where we run
`build_bios.py`), we'll add the following to create and apply our patch and copy the
harness header file to the correct location.

```dockerfile
COPY <<'EOF' /tmp/edk2-platforms.patch
diff --git a/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/Logo.c b/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/Logo.c
index 9cea5f4665..00815adba2 100644
--- a/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/Logo.c
+++ b/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/Logo.c
@@ -11,6 +11,7 @@
 #include <OemBadging.h>
 #include <Protocol/GraphicsOutput.h>
 #include <Library/BaseLib.h>
+#include <Library/DebugLib.h>
 #include <Library/UefiLib.h>
 #include <Library/BaseMemoryLib.h>
 #include <Library/UefiBootServicesTableLib.h>
@@ -22,6 +23,7 @@
 #include <IndustryStandard/Bmp.h>
 #include <Protocol/BootLogo.h>
 
+#include "tsffs-gcc-x86_64.h"
 /**
   Convert a *.BMP graphics image to a GOP blt buffer. If a NULL Blt buffer
   is passed in a GopBlt buffer will be allocated by this routine. If a GopBlt
@@ -164,9 +166,6 @@ ConvertBmpToGopBlt (
     *GopBltSize = (UINTN) BltBufferSize;
     *GopBlt     = AllocatePool (*GopBltSize);
     IsAllocated = TRUE;
-    if (*GopBlt == NULL) {
-      return EFI_OUT_OF_RESOURCES;
-    }
   } else {
     //
     // GopBlt has been allocated by caller.
@@ -184,6 +183,7 @@ ConvertBmpToGopBlt (
   // Convert image from BMP to Blt buffer format
   //
   BltBuffer = *GopBlt;
+  ASSERT (BmpHeader->PixelHeight != 0);
   for (Height = 0; Height < BmpHeader->PixelHeight; Height++) {
     Blt = &BltBuffer[(BmpHeader->PixelHeight - Height - 1) * BmpHeader->PixelWidth];
     for (Width = 0; Width < BmpHeader->PixelWidth; Width++, Image++, Blt++) {
@@ -398,6 +398,7 @@ EnableBootLogo (
     // Try BMP decoder
     //
     Blt = NULL;
+    HARNESS_START(ImageData, &ImageSize);
     Status = ConvertBmpToGopBlt (
               ImageData,
               ImageSize,
@@ -411,6 +412,7 @@ EnableBootLogo (
       FreePool (ImageData);
 
       if (Badging == NULL) {
+        HARNESS_STOP();
         return Status;
       } else {
         continue;
@@ -537,6 +539,7 @@ Done:
       FreePool (Blt);
     }
 
+    HARNESS_STOP();
     return Status;
   }
 
@@ -561,6 +564,7 @@ Done:
     // Ensure the LogoHeight * LogoWidth doesn't overflow
     //
     if (LogoHeight > DivU64x64Remainder ((UINTN) ~0, LogoWidth, NULL)) {
+      HARNESS_STOP();
       return EFI_UNSUPPORTED;
     }
     BufferSize = MultU64x64 (LogoWidth, LogoHeight);
@@ -569,11 +573,13 @@ Done:
     // Ensure the BufferSize * sizeof (EFI_GRAPHICS_OUTPUT_BLT_PIXEL) doesn't overflow
     //
     if (BufferSize > DivU64x32 ((UINTN) ~0, sizeof (EFI_GRAPHICS_OUTPUT_BLT_PIXEL))) {
+      HARNESS_STOP();
       return EFI_UNSUPPORTED;
     }
 
     LogoBlt = AllocateZeroPool ((UINTN)BufferSize * sizeof (EFI_GRAPHICS_OUTPUT_BLT_PIXEL));
     if (LogoBlt == NULL) {
+      HARNESS_STOP();
       return EFI_OUT_OF_RESOURCES;
     }
 
@@ -600,5 +606,6 @@ Done:
   }
   FreePool (LogoBlt);
 
+  HARNESS_STOP();
   return Status;
 }

EOF

COPY tsffs-gcc-x86_64.h /workspace/edk2-platforms/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/tsffs-gcc-x86_64.h

RUN git -C /workspace/edk2-platforms apply /tmp/edk2-platforms.patch
```

With this modification applied to the Dockerfile, we'll go ahead and build again with
our build script `./build.sh`.

## Enabling UEFI Tracking

During fuzzing, it will be helpful to us for many reasons if we can use source-level
debugging functionality that is built into SIMICS. Recall that earlier, we made sure
that the build directory inside our Docker container is the same as the directory we
run our BIOS from. This is because we are going to use the UEFI Firmware Tracker built
into SIMICS.

We already had a `project/run.simics` script, we'll create another script
`project/fuzz.simics` which we'll build on to enable fuzzing.

We'll start with a script that just loads the platform and runs. We won't even be
booting up to the UEFI shell, only through the BIOS image load process, so we'll remove
the extra code that we had before.

```simics
load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

run
```

Next, we want to add functionality to enable UEFI tracking, which you can read about
in full detail [in the docs](https://intel.github.io/tsffs/simics/analyzer-user-guide/uefi-fw-trk.html).

At the top of the script, we'll load the tracker:


```simics
load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

run
```

Then, we need to create a new OS-awareness object (which we'll call `qsp.software`),
insert the UEFI tracker into the awareness module, and detect parameters, which we'll
save to the file "%simics%/uefi.params". This params file will contain a dictionary of
parameters like:

```python
[
    'uefi_fw_tracker',
    {
        'tracker_version': 6263,
        'map_info': [],
        'map_file': None,
        'pre_dxe_start': 0,
        'pre_dxe_size': 0,
        'dxe_start': 0,
        'dxe_size': 4294967296,
        'exec_scan_size': 327680,
        'notification_tracking': True,
        'pre_dxe_tracking': False,
        'dxe_tracking': True,
        'hand_off_tracking': True,
        'smm_tracking': True,
        'reset_tracking': True,
        'exec_tracking': True
    }
]
```

We want to enable the map file, so we'll tell the command to set the `map-file` path to
our map file. This will automatically populate the `map_info` with the info contained in
the map file. Our script will look like this:

```simics
load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -overwrite param-file = "%simics%/uefi.params" map-file = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/SimicsX58.map"
qsp.software.tracker.load-parameters "%simics%/uefi.params"
qsp.software.enable-tracker

run
```

With tracking enabled, we can add a `source_location` breakpoint on a symbol (SIMICS
will track UEFI mappings and make symbols available when they are loaded during
execution, or from a map file as we've done here). To break on assertions, we will
add a breakpoint on the `DebugAssert` function (which EDK2's `ASSERT` macro ultimately
calls).

## Configuring the Fuzzer

The above can be applied to any code which runs during the SEC, PEI, or early DXE
stages. If the codepath you want to fuzz is always executed during boot, all you need to
do is add the harness macros to it and turn on the fuzzer.

We'll use the breakpoint API to wait for the `DebugAssert` function in a loop. We do
this instead of using the `$bp_num = bp.source_location.break DebugAssert` command and
adding it to the fuzzer configuration with
`@tsffs.iface.tsffs.add_breakpoint_solution(simenv.bp_num)` because the HAP for
breakpoints does not trigger on breakpoints set on source locations in this way, so the
fuzzer cannot intercept it. This is in contrast to breakpoints set with the following,
which will work with the `add_breakpoint_solution` API:

```simics
$ctx = (new-context)
qsp.mb.cpu0.core[0][0].set-context $ctx
$ctx.break -w $BUFFER_ADDRESS $BUFFER_SIZE
```

The rest of the configuration is similar to configuration we've already done in previous
tutorials.

```simics
load-module tsffs
@tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
tsffs.log-level 4
@tsffs.iface.tsffs.set_start_on_harness(True)
@tsffs.iface.tsffs.set_stop_on_harness(True)
@tsffs.iface.tsffs.set_timeout(3.0)
@tsffs.iface.tsffs.add_exception_solution(13)
@tsffs.iface.tsffs.add_exception_solution(14)

load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -overwrite param-file = "%simics%/uefi.params" map-file = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/SimicsX58.map"
qsp.software.tracker.load-parameters "%simics%/uefi.params"
qsp.software.enable-tracker

script-branch {
    while 1 {
        bp.source_location.wait-for DebugAssert -x -error-not-planted
        echo "Got breakpoint"
        @tsffs.iface.tsffs.solution(1, "DebugAssert")
    }
}

run
```

## Obtaining a Corpus

To keep things simple, we'll go ahead and use one file as the corpus provided to us, the
actual boot image.


```sh
mkdir -p project/corpus/
curl -L -o project/corpus/0 https://raw.githubusercontent.com/tianocore/edk2-platforms/master/Platform/Intel/SimicsOpenBoardPkg/Logo/Logo.bmp
```