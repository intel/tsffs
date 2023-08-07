// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#include <stddef.h>
#include <stdint.h>

// NOTE: Forward declaration
struct EfiSimpleTextOutputProtocol;

typedef uint64_t (*EfiTextString)(struct EfiSimpleTextOutputProtocol *this,
                                  int16_t *string);
typedef struct EfiTableHeader {
  uint64_t signature;
  uint32_t revision;
  uint32_t headerSize;
  uint32_t crc32;
  uint32_t reserved;
} EfiTableHeader;

typedef struct EfiSimpleTextOutputProtocol {
  uint64_t reset;
  EfiTextString output_string;
  uint64_t test_string;
  uint64_t query_mode;
  uint64_t set_mode;
  uint64_t set_attribute;
  uint64_t clear_screen;
  uint64_t set_cursor_position;
  uint64_t enable_cursor;
  uint64_t mode;
} EfiSimpleTextOutputProtocol;

typedef struct EfiSystemTable {
  EfiTableHeader hdr;
  int16_t *firmwareVendor;
  uint32_t firmwareRevision;
  void *consoleInHandle;
  uint64_t conIn;
  void *consoleOutHandle;
  EfiSimpleTextOutputProtocol *conOut;
  void *standardErrorHandle;
  uint64_t stdErr;
  uint64_t runtimeServices;
  uint64_t bootServices;
  uint64_t numberOfTableEntries;
  uint64_t configurationTable;
} EfiSystemTable;

const char hex[] = {'0', '1', '2', '3', '4', '5', '6', '7',
                    '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'};

char off_limits[0x100] = {0x00};

int UefiMain(void *imageHandle, EfiSystemTable *SystemTable) {
  int16_t buffer[0x20];
  size_t size = sizeof(buffer) - 1;

  for (size_t i = 0; i < size; i++) {
    if (i != 0 && !(i % 8)) {
      SystemTable->conOut->output_string(SystemTable->conOut,
                                         (int16_t *)L"\r\n");
    }
    int16_t buf[5];
    buf[4] = 0;
    int16_t chr = buffer[i];
    buf[0] = hex[chr & 0xf];
    buf[1] = hex[(chr >> 4) & 0xf];
    buf[2] = hex[(chr >> 8) & 0xf];
    buf[3] = hex[(chr >> 12) & 0xf];

    SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)&buf[0]);
  }

  SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)L"\r\n");

  if (*(char *)buffer == 'a') {
    // Invalid opcode
    __asm__(".byte 0x06");
  } else if (*(char *)buffer == 'b') {
    // Crash
    uint8_t *bad_ptr = (uint8_t *)0xffffffffffffffff;
    *bad_ptr = 0;
  } else if (*(char *)buffer == 'c') {
    // Breakpoint-defined fault location (instruction BP)
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"Uh oh!\r\n");
  } else if (*(char *)buffer == 'd') {
    for (size_t i = 0; i < sizeof(off_limits); i++) {
      off_limits[i] = 'X';
    }
  }

  return 0;
}