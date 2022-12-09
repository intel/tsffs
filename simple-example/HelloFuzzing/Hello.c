/** @file
    A simple, basic, EDK II native, "hello" application to verify that
    we can build applications without LibC.

    Copyright (c) 2010 - 2011, Intel Corporation. All rights reserved.<BR>
    SPDX-License-Identifier: BSD-2-Clause-Patent
**/
#include  <Uefi.h>
#include  <Library/UefiLib.h>
#include  <Library/ShellCEntryLib.h>
#include  <Library/MemoryAllocationLib.h>
#include  "magic-instruction.h"
#include  <string.h>
#include  <unistd.h>


/***
  Print a welcoming message.

  Establishes the main structure of the application.

  @retval  0         The application exited normally.
  @retval  Other     An error occurred.
***/
int
main (
  IN int Argc,
  IN char **Argv
  )
{
  MAGIC(42); //Inform Simics that we wanna have the start snapshot here
 
  //TODO: Read Input data from magic pipe here
  Print(L"Hello there fellow Programmer.\n");
  Print(L"Welcome to the world of EDK II.\n");
  //TODO: Write output data to magic pipe here

  MAGIC(43); //NOTE: Just a placeholder until we have magic pipe working
  return(0);
}

