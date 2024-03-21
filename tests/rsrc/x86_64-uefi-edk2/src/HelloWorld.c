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
  UINTN input_max_size = 64;
  UINTN input_size = input_max_size;
  UINT8 *input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(input_max_size));

  if (!input) {
    return EFI_OUT_OF_RESOURCES;
  }

  SetMem((VOID *)input, input_max_size, 0x44);

  HARNESS_START(input, &input_size);

  switch (*input) {
    case 'A': {
      // Invalid opcode
      __asm__(".byte 0x06");
    }
    case 'B': {
      // Sleep for 10 seconds, this is a "hang"

      // NOTE: gBS is the global Boot Services table
      gBS->Stall(10 * 1000 * 1000);
    }
    case 'C': {
      // This will page fault
      UINT8 *ptr = (UINT8 *)0xffffffffffffffff;
      *ptr = 0x00;
    }
    default: {
      // Nothing, this is a "success"
      Print(L"Working...\n");
    }
  }

  HARNESS_STOP();

  if (input) {
    FreePages(input, EFI_SIZE_TO_PAGES(input_max_size));
  }

  return EFI_SUCCESS;
}