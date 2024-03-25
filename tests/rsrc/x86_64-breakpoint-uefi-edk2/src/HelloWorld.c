// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/** @file
  This sample application bases on HelloWorld PCD setting
  to print "UEFI Hello World!" to the UEFI Console.
  Copyright (c) 2006 - 2018, Intel Corporation. All rights reserved.<BR>
  SPDX-License-Identifier: BSD-2-Clause-Patent
**/

#include <Library/BaseMemoryLib.h>
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
  UINTN input_max_size = 0x1000;
  UINTN input_size = input_max_size;
  EFI_PHYSICAL_ADDRESS address = 0x4000000;
  EFI_STATUS status;
  status = gBS->AllocatePages(AllocateAddress, EfiRuntimeServicesCode,
                              EFI_SIZE_TO_PAGES(input_max_size), &address);
  if (EFI_ERROR(status)) {
    return EFI_OUT_OF_RESOURCES;
  }
  UINT8 *input = (UINT8 *)address;

  HARNESS_START(input, &input_size);

  if (*input == 0x41) {
    // Trigger RW breakpoint
    SetMem((VOID *)input, input_size, 0x44);
  }

  HARNESS_STOP();

  return EFI_SUCCESS;
}