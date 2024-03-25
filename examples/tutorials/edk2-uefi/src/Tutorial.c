// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

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