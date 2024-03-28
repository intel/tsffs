// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "tsffs.h"

const char *password = "fuzzing!";

int check(char *buffer) {
  if ((((char *)buffer)[0]) == password[0]) {
    if ((((char *)buffer)[1]) == password[1]) {
      if ((((char *)buffer)[2]) == password[2]) {
        if ((((char *)buffer)[3]) == password[3]) {
          if ((((char *)buffer)[4]) == password[4]) {
            if ((((char *)buffer)[5]) == password[5]) {
              if ((((char *)buffer)[6]) == password[6]) {
                if ((((char *)buffer)[7]) == password[7]) {
                  printf("All characters were correct!\n");
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

int main() {
  // We have a size and a buffer of that size. The address of the buffer and the
  // address of the size variable will be passed to the fuzzer. On the first
  // start harness, the fuzzer will save the initial value of the size and the
  // addresses of both variables. On each iteration of the fuzzer, up to the
  // initial size bytes of fuzzer input data will be written to the buffer, and
  // the current testcase size in bytes will be written to the size variable.
  char buffer[8] = {'A', 'A', 'A', 'A', 'A', 'A', 'A', 'A'};
  size_t size = sizeof(buffer);
  size_t *size_ptr = &size;

  setvbuf(stdout, NULL, _IONBF, 0);
  setvbuf(stderr, NULL, _IONBF, 0);

  // printf("Running test...\n");

  HARNESS_START(buffer, size_ptr);

  // NOTE: Because of line buffering at the simics level, this will be extremely
  // garbled.
  printf("sz: %zu\ntc: ", size);

  for (size_t i = 0; i < size; i++) {
    printf("%02x", buffer[i]);
  }

  printf("\n");

  check(buffer);

  HARNESS_STOP();

  return 0;
}