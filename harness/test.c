#ifdef SINGLE_FILE
#include "tsffs.h"
#else
#ifdef __i386__
#include "tsffs-gcc-x86.h"
#elif __x86_64__
#include "tsffs-gcc-x86_64.h"
#elif __riscv && !__LP64__
#include "tsffs-gcc-riscv32.h"
#elif __riscv && __LP64__
#include "tsffs-gcc-riscv64.h"
#elif __aarch64__
#include "tsffs-gcc-aarch64.h"
#elif __arm__
#include "tsffs-gcc-arm32.h"
#endif
#endif

#include <stddef.h>

int test_start() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START(buf, &size);
  return 0;
}

int test_start_with_maximum_size() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START_WITH_MAXIMUM_SIZE(buf, size);
  return 0;
}

int test_start_with_maximum_size_and_ptr() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buf, &size, 1024);
  return 0;
}

int test_stop() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_STOP();
  return 0;
}

int test_assert() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_ASSERT();
  return 0;
}

#ifndef __arm__
int test_start_index() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START_INDEX(1, buf, &size);
  return 0;
}

int test_start_with_maximum_size_index() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(2, buf, size);
  return 0;
}

int test_start_with_maximum_size_and_ptr_index() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(3, buf, &size, 1024);
  return 0;
}

int test_stop_index() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_STOP_INDEX(4);
  return 0;
}

int test_assert_index() {
  char buf[1024];
  size_t size = 1024;
  HARNESS_ASSERT_INDEX(5);
  return 0;
}

#endif
