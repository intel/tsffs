/** @file
  This sample application bases on HelloWorld PCD setting
  to print "UEFI Hello World!" to the UEFI Console.
  Copyright (c) 2006 - 2018, Intel Corporation. All rights reserved.<BR>
  SPDX-License-Identifier: BSD-2-Clause-Patent
**/

#include <Library/BaseCryptLib.h>
#include <Library/MemoryAllocationLib.h>
#include <Library/PcdLib.h>
#include <Library/UefiApplicationEntryPoint.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Library/UefiLib.h>
#include <Uefi.h>

#include "tsffs.h"

/**
  The user Entry Point for Application. The user code starts with this function
  as the real entry point for the application.
  @param[in] ImageHandle    The firmware allocated handle for the EFI image.
  @param[in] SystemTable    A pointer to the EFI System Table.
  @retval EFI_SUCCESS       The entry point is executed successfully.
  @retval other             Some error occurs when executing this entry point.
**/
EFI_STATUS
EFIAPI
UefiMain(IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
  UINTN max_input_size = 0x1000;
  UINTN input_size = max_input_size;
  UINT8 *input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(max_input_size));

  if (!input) {
    return EFI_OUT_OF_RESOURCES;
  }

  HARNESS_START(&input, &input_size);

  UINT8 *Cert = input;
  UINTN CertSize = input_size / 2;
  UINT8 *CACert = (input + CertSize);
  UINTN CACertSize = CertSize;

  X509VerifyCert(Cert, CertSize, CACert, CACertSize);

  HARNESS_STOP();

  if (input) {
    FreePages(input, EFI_SIZE_TO_PAGES(max_input_size));
  }

  return EFI_SUCCESS;
}