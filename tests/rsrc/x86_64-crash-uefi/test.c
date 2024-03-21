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
const char *password = "fuzzing!";

int Check(char *buffer, EfiSystemTable *SystemTable) {
  if ((((char *)buffer)[0]) == password[0]) {
    if ((((char *)buffer)[1]) == password[1]) {
      if ((((char *)buffer)[2]) == password[2]) {
        if ((((char *)buffer)[3]) == password[3]) {
          if ((((char *)buffer)[4]) == password[4]) {
            if ((((char *)buffer)[5]) == password[5]) {
              if ((((char *)buffer)[6]) == password[6]) {
                if ((((char *)buffer)[7]) == password[7]) {
                  SystemTable->conOut->output_string(
                      SystemTable->conOut,
                      (int16_t *)L"All characters were correct!\r\n");
                  uint8_t *ptr = (uint8_t *)0xffffffffffffffff;
                  *ptr = 0;
                }
              }
            }
          }
        }
      }
    }
  }

  return 0;
}

// The entrypoint of our EFI application
int UefiMain(void *imageHandle, EfiSystemTable *SystemTable) {
  // We have a size and a buffer of that size. The address of the buffer and the
  // address of the size variable will be passed to the fuzzer. On the first
  // start harness, the fuzzer will save the initial value of the size and the
  // addresses of both variables. On each iteration of the fuzzer, up to the
  // initial size bytes of fuzzer input data will be written to the buffer, and
  // the current testcase size in bytes will be written to the size variable.
  char buffer[8] = {'A', 'A', 'A', 'A', 'A', 'A', 'A', 'A'};
  size_t size = sizeof(buffer);
  HARNESS_START(buffer, &size);

  for (size_t i = 0; i < size; i++) {
    if (i != 0 && !(i % 8)) {
      SystemTable->conOut->output_string(SystemTable->conOut,
                                         (int16_t *)L"\r\n");
    }
    uint8_t chr = buffer[i];
    int16_t buf[3];
    buf[0] = hex[(chr >> 4) & 0xf];
    buf[1] = hex[chr & 0xf];
    buf[2] = 0;

    SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)&buf[0]);
  }

  SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)L"\r\n");

  Check(buffer, SystemTable);

  if (*buffer == 0x41) {
    uint8_t *ptr = (uint8_t *)0xffffffffffffffff;
    *ptr = 0;
  }

  HARNESS_STOP();

  return 0;
}