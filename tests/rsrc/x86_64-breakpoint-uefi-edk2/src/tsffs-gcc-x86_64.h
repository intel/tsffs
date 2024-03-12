// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// Definitions and macros for compiled-in harnessing of C and C++ target
/// software for the RISC-V (64-bit) architecture
///
/// Example:
/// ```c
/// #include "tsffs-gcc-x86_64.h"
///
/// int main() {
///    char buf[0x10];
///    size_t size = 0x10;
///    size_t *size_ptr = &size;
///    HARNESS_START((char *)buf, size_ptr);
///    int retval = YourSpecialDecoder(buf, *size_ptr);
///    if (!retval) {
///        HARNESS_ASSERT();
///    } else {
///        HARNESS_STOP();
///    }
/// }
/// ```

#ifndef TSFFS_H
#define TSFFS_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

/// Magic value defined by SIMICS as the "leaf" value of a CPUID instruction
/// that is treated as a magic instruction.
#define MAGIC (0x4711U)
/// Default magic instruction `n` value to signal the TSFFS fuzzer to start the
/// fuzzing loop
#define MAGIC_START (0x0001U)
/// Default magic instruction `n` value to signal the TSFFS fuzzer to stop and
/// reset the fuzzing loop
#define MAGIC_STOP (0x0002U)
/// Default magic instruction `n` value to signal the TSFFS fuzzer that an error
/// has occurred and the testcase input should be saved as a solution
#define MAGIC_ASSERT (0x0003U)

/// Alternative magic numbers that can be used for start and stop events in
/// conjunction with setting the magic number for each event via the SIMICS or
/// SIMICS Python script interface
#define MAGIC_ALT_0 (0x0004U)
#define MAGIC_ALT_1 (0x0005U)
#define MAGIC_ALT_2 (0x0006U)
#define MAGIC_ALT_3 (0x0007U)
#define MAGIC_ALT_4 (0x0008U)
#define MAGIC_ALT_5 (0x0009U)
#define MAGIC_ALT_6 (0x000aU)
#define MAGIC_ALT_7 (0x000bU)

/// Invoke the magic instruction defined by SIMICS for the x86-64 architecture
/// (`cpuid`) with a specific value of `n`, after setting register `rdi` to the
/// value of the pointer to the testcase and register `rsi` to the value of the
/// pointer to the testcase size. These registers are accessed by the fuzzer and
/// are defined per-architecture.
#define __cpuid_extended2(value, inout_ptr_0, inout_ptr_1)              \
  unsigned int _a __attribute__((unused)) = 0;                          \
  __asm__ __volatile__("cpuid"                                          \
                       : "=a"(_a)                                       \
                       : "a"(value), "D"(inout_ptr_0), "S"(inout_ptr_1) \
                       : "rbx", "rcx", "rdx");

/// Invoke the magic instruction defined by SIMICS for the x86-64 architecture
/// (`cpuid`) with a specific value of `n`
#define __cpuid(value)                         \
  unsigned int _a __attribute__((unused)) = 0; \
  __asm__ __volatile__("cpuid\n\t"             \
                       : "=a"(_a)              \
                       : "a"(value)            \
                       : "rbx", "rcx", "rdx")

/// Signal the fuzzer using a specific magic value `start` to start the fuzzing
/// loop at the point this macro is called. A snapshot will be taken when the
/// macro is called, and the maximum testcase size `*size_ptr` will be saved as
/// `max_testcase_size`. Each iteration of the fuzzing loop, the fuzzer input
/// (the "testcase") will be written to `*testcase_ptr` as if running
/// `memcpy(testcase_ptr, current_testcase, max_testcase_size)`, and the actual
/// size of the current testcase will be written to
/// `*size_ptr` as if running `*size_ptr = current_testcase_size`.
#define __arch_harness_start(start, testcase_ptr, size_ptr) \
  do {                                                      \
    unsigned int magic = (start << 0x10U) | MAGIC;          \
    __cpuid_extended2(magic, testcase_ptr, size_ptr);       \
  } while (0)

/// Signal the fuzzer using the a specific magic value (`stop`) to stop and
/// reset to the beginning of the fuzzing loop with a "normal" stop status,
/// indicating no solution has occurred.
#define __arch_harness_stop(stop)                 \
  do {                                            \
    unsigned int magic = (stop << 0x10U) | MAGIC; \
    __cpuid(magic);                               \
  } while (0)

/// Signal the fuzzer using the default magic value to start the fuzzing loop at
/// the point this macro is called. A snapshot will be taken when the macro is
/// called, and the maximum testcase size `*size_ptr` will be saved as
/// `max_testcase_size`.  Each iteration of the fuzzing loop, the fuzzer input
/// (the "testcase") will be written to
/// `*testcase_ptr` as if running `memcpy(testcase_ptr, current_testcase,
/// max_testcase_size)`, and the actual size of the current testcase will be
/// written to
/// `*size_ptr` as if running `*size_ptr = current_testcase_size`.
#define HARNESS_START(testcase_ptr, size_ptr)                  \
  do {                                                         \
    __arch_harness_start(MAGIC_START, testcase_ptr, size_ptr); \
  } while (0)

/// Signal the fuzzer using the default magic value to stop and reset to the
/// beginning of the fuzzing loop with a "normal" stop status, indicating no
/// solution has occurred.
#define HARNESS_STOP()               \
  do {                               \
    __arch_harness_stop(MAGIC_STOP); \
  } while (0)

/// Signal the fuzzer using the default magic value to stop and reset to the
/// beginning of the fuzzing loop with a "solution" stop status, indicating some
/// custom error has occurred.
#define HARNESS_ASSERT()               \
  do {                                 \
    __arch_harness_stop(MAGIC_ASSERT); \
  } while (0)

#endif  // TSFFS_H