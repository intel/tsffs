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
  // A 4-byte size variable, as size_t is on a 32-bit target, and the 4 bytes after it
  struct {
    uint32_t size;
    uint32_t canary;
  } s = {sizeof(word), 0xdeadbeef};
  HARNESS_START(&word, &s.size);

  // The fuzzer writes the testcase size back into s.size. With the i386 architecture
  // hint that is a 4-byte write: a wider one overwrites the canary.
  if (s.canary == 0xdeadbeef) {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"CANARY_INTACT\r\n");
  } else {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"CANARY_CLOBBERED\r\n");
  }

  HARNESS_STOP();

  return 0;
}
