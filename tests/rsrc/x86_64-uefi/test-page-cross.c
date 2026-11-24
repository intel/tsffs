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

#define PAGE_SIZE 4096
#define PRESENT_WRITABLE 3
// An address in the upper half of the address space, which the firmware does not map
#define WINDOW 0xffff800000000000ULL
#define CANARY 0xa5

// Physical pages (the firmware maps memory 1:1). The window maps pages 0 and 2, so that
// page 1 follows the first page of the window in physical memory but is not part of it.
static uint8_t frames[3][PAGE_SIZE] __attribute__((aligned(PAGE_SIZE)));
// PML4, PDPT, PD and PT
static uint64_t tables[4][512] __attribute__((aligned(PAGE_SIZE)));

// The entrypoint of our EFI application
int UefiMain(void *imageHandle, EfiSystemTable *SystemTable) {
  uint64_t cr3;
  __asm__("mov %%cr3, %0" : "=r"(cr3));
  uint64_t *firmware_pml4 = (uint64_t *)(cr3 & ~0xfffULL);
  for (int i = 0; i < 512; i++) {
    tables[0][i] = firmware_pml4[i];
  }
  tables[0][(WINDOW >> 39) & 0x1ff] = (uint64_t)tables[1] | PRESENT_WRITABLE;
  tables[1][0] = (uint64_t)tables[2] | PRESENT_WRITABLE;
  tables[2][0] = (uint64_t)tables[3] | PRESENT_WRITABLE;
  tables[3][0] = (uint64_t)frames[0] | PRESENT_WRITABLE;
  tables[3][1] = (uint64_t)frames[2] | PRESENT_WRITABLE;
  __asm__("mov %0, %%cr3" : : "r"((uint64_t)tables[0] | (cr3 & 0xfff)));

  for (int i = 0; i < PAGE_SIZE; i++) {
    frames[1][i] = CANARY;
  }

  // The first 8 bytes of the buffer are at the end of frames[0], the rest in frames[2]
  size_t in_first_page = 8;
  uint8_t *buffer = (uint8_t *)WINDOW + PAGE_SIZE - in_first_page;
  size_t size = 64;
  HARNESS_START(buffer, &size);

  int clobbered = 0;
  for (int i = 0; i < PAGE_SIZE; i++) {
    clobbered |= frames[1][i] != CANARY;
  }
  // frames[2] is zero before the testcase is written
  int tail_written = 0;
  for (size_t i = 0; i + in_first_page < size; i++) {
    tail_written |= frames[2][i] != 0;
  }

  if (clobbered) {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"NEIGHBOUR_CLOBBERED\r\n");
  }
  if (tail_written) {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"TAIL_WRITTEN\r\n");
  }

  HARNESS_STOP();

  return 0;
}
