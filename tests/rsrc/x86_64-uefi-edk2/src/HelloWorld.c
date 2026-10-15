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
#include <Pi/PiMultiPhase.h>
#include <Protocol/MpService.h>
#include <Uefi.h>

#include "tsffs.h"

/**
  Procedure run on an AP (a non-BSP logical processor) to cause a page fault
  on that specific processor, so a consumer of the fault can confirm which
  processor it was attributed to.
**/
VOID EFIAPI PageFaultApProcedure(IN OUT VOID *Buffer) {
  UINT8 *ptr = (UINT8 *)0xffffffffffffffff;
  *ptr = 0x00;
}

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
    case 'D': {
      // This will page fault on an AP (a non-BSP processor), to exercise
      // multi-processor fault attribution.
      EFI_MP_SERVICES_PROTOCOL *mp_services;

      if (!EFI_ERROR(gBS->LocateProtocol(&gEfiMpServiceProtocolGuid, NULL,
                                          (VOID **)&mp_services))) {
        UINTN number_of_processors;
        UINTN number_of_enabled_processors;
        UINTN bsp_number;

        if (!EFI_ERROR(mp_services->GetNumberOfProcessors(
                mp_services, &number_of_processors,
                &number_of_enabled_processors)) &&
            !EFI_ERROR(mp_services->WhoAmI(mp_services, &bsp_number)) &&
            number_of_processors > 1) {
          UINTN ap_number = (bsp_number + 1) % number_of_processors;

          Print(L"AP_FAULT_PROCESSOR %d\n", (UINT32)ap_number);
          mp_services->StartupThisAP(mp_services, PageFaultApProcedure,
                                      ap_number, NULL, 0, NULL, NULL);
        } else {
          Print(L"AP_FAULT_SKIPPED insufficient processors\n");
        }
      } else {
        Print(L"AP_FAULT_SKIPPED no mp services\n");
      }
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