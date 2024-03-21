# Harnessing

Just before the time of writing, a vulnerability received significant publicity this
week concerning the boot logo parser in many vendors BIOS images called
[LogoFAIL](https://binarly.io/posts/finding_logofail_the_dangers_of_image_parsing_during_system_boot/).
In a [press release](https://min.news/en/tech/128c34878b2b582065c1e05379912294.html) the
vulnerability finders noted that "Our fuzz testing and subsequent vulnerability triage
results clearly indicate that these image parsers were never tested by IBV or OEM".
Unlike the findings, [some](https://github.com/tianocore/edk2-staging/tree/HBFA/HBFA/UefiHostFuzzTestCasePkg/TestCase/MdeModulePkg/Library/BaseBmpSupportLib),
have been fuzzed, but even so let's get our platform harnessed -- all we need to do is
add the vulnerability ourselves!

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
[tsffs.h](https://github.com/intel/tsffs/blob/main/harness/tsffs.h)
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
 
+#include "tsffs.h"
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

COPY tsffs.h /workspace/edk2-platforms/Platform/Intel/SimicsOpenBoardPkg/Library/DxeLogoLib/tsffs.h

RUN git -C /workspace/edk2-platforms apply /tmp/edk2-platforms.patch
```

With this modification applied to the Dockerfile, we'll go ahead and build again with
our build script `./build.sh`.