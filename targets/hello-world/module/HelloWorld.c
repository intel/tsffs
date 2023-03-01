/** @file
  This sample application bases on HelloWorld PCD setting
  to print "UEFI Hello World!" to the UEFI Console.
  Copyright (c) 2006 - 2018, Intel Corporation. All rights reserved.<BR>
  SPDX-License-Identifier: BSD-2-Clause-Patent
**/

#include <Library/MemoryAllocationLib.h>
#include <Library/PcdLib.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Library/UefiApplicationEntryPoint.h>
#include <Library/UefiLib.h>
#include <Uefi.h>

#include "confuse.h"

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
    UINT8 *input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(CONFUSE_MAXSIZE));

    if (!input) {
      return EFI_OUT_OF_RESOURCES;
    }

    HARNESS_START(input, CONFUSE_MAXSIZE);

    switch (*input) {
      case 'A': {
        // Execute bad code, this is a "crash"
        __asm__(".byte 0x06");
      }
      case 'B': {
        // Sleep for 3 seconds, this is a "hang"
        // NOTE: gBS is the global Boot Services table
        gBS->Stall(3 * 1000 * 1000);
      }
      default: {
        // Nothing, this is a "success"
        Print(L"Working...\n");
      }
    }

    HARNESS_STOP();


    if (input) {
      FreePages(input, EFI_SIZE_TO_PAGES(CONFUSE_MAXSIZE));
    }

    return EFI_SUCCESS;
}