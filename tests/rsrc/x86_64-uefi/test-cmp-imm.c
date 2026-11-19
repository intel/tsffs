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
  uint32_t word = 0x41414141;
  size_t size = sizeof(word);
  HARNESS_START(&word, &size);

  // The testcase is compared against an immediate operand. Unless that compare
  // is logged, the fuzzer has to guess all 32 bits at once.
  if (word == 0x21584946) {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"MAGIC_FOUND\r\n");
  }

  HARNESS_STOP();

  return 0;
}
