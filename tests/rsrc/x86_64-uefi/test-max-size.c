// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#include <stddef.h>
#include <stdint.h>

#include "tsffs.h"

typedef struct EfiTableHeader {
  uint64_t signature;
  uint32_t revision;
  uint32_t headerSize;
  uint32_t crc32;
  uint32_t reserved;
} EfiTableHeader;

struct EfiSimpleTextOutputProtocol;

typedef uint64_t (*EfiTextString)(struct EfiSimpleTextOutputProtocol *this,
                                  int16_t *string);

typedef struct EfiSimpleTextOutputProtocol {
  uint64_t reset;
  EfiTextString output_string;
} EfiSimpleTextOutputProtocol;

typedef struct EfiSystemTable {
  EfiTableHeader hdr;
  int16_t *firmwareVendor;
  uint32_t firmwareRevision;
  void *consoleInHandle;
  uint64_t conIn;
  void *consoleOutHandle;
  EfiSimpleTextOutputProtocol *conOut;
} EfiSystemTable;

// The entrypoint of our EFI application
int UefiMain(void *imageHandle, EfiSystemTable *SystemTable) {
  char buffer[8] = {'A', 'A', 'A', 'A', 'A', 'A', 'A', 'A'};
  // The size variable deliberately does not hold the maximum size: this harness
  // passes the maximum size as its own argument.
  size_t size = 0;
  HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, &size, sizeof(buffer));

  // Report the testcase size the fuzzer wrote back, a single digit when it
  // respects the maximum size
  int16_t digit[2] = {'0' + size, 0};
  SystemTable->conOut->output_string(SystemTable->conOut,
                                     (int16_t *)L"TESTCASE_SIZE ");
  SystemTable->conOut->output_string(SystemTable->conOut, digit);
  SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)L"\r\n");

  HARNESS_STOP();

  return 0;
}
