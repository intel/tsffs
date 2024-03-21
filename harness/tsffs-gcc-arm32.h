// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// Definitions and macros for compiled-in harnessing of C and C++ target
/// software for the RISC-V (32-bit) architecture

#ifndef TSFFS_H
#define TSFFS_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

#define __stringify(x) #x
#define __tostring(x) __stringify(x)

/// __orr
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n`
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
#define __orr(value)                                               \
  __asm__ __volatile__("orr r" __tostring(value) ", r" __tostring( \
      value) ", r" __tostring(value));

/// __orr_extended1
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and a pseudo-argument in register `r10`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `r10`
#define __orr_extended1(value, arg0)                           \
  __asm__ __volatile__(                                        \
      "mov r10, %0; orr r" __tostring(value) ", r" __tostring( \
          value) ", r" __tostring(value)                       \
      :                                                        \
      : "r"(arg0));

/// __orr_extended2
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and two pseudo-arguments in registers `r10` and
/// `r9`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `r10`
/// * `arg1` - The value to place in register `r9`
#define __orr_extended2(value, arg0, arg1)                                 \
  __asm__ __volatile__(                                                    \
      "mov r10, %0; mov r9, %1; orr r" __tostring(value) ", r" __tostring( \
          value) ", r" __tostring(value)                                   \
      :                                                                    \
      : "r"(arg0), "r"(arg1));

/// __orr_extended3
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and three pseudo-arguments in registers `r10`,
/// `r9`, and `r8`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `r10`
/// * `arg1` - The value to place in register `r9`
/// * `arg2` - The value to place in register `r8`
#define __orr_extended3(value, arg0, arg1, arg2)                 \
  __asm__ __volatile__(                                          \
      "mov r10, %0; mov r9, %1; mov r8, %2; orr r" __tostring(   \
          value) ", r" __tostring(value) ", r" __tostring(value) \
      :                                                          \
      : "r"(arg0), "r"(arg1), "r"(arg2));

/// __orr_extended4
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and four pseudo-arguments in registers `r10`,
/// `r9`, `r8`, and `r7`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `r10`
/// * `arg1` - The value to place in register `r9`
/// * `arg2` - The value to place in register `r8`
/// * `arg3` - The value to place in register `r7`
#define __orr_extended4(value, arg0, arg1, arg2, arg3)                    \
  __asm__ __volatile__(                                                   \
      "mov r10, %0; mov r9, %1; mov r8, %2; mov r7, %3; "                 \
      "orr r" __tostring(value) ", r" __tostring(value) ", r" __tostring( \
          value)                                                          \
      :                                                                   \
      : "r"(arg0), "r"(arg1), "r"(arg2), "r"(arg3));

/// Magic value defined by SIMICS as the "leaf" value of a CPUID instruction
/// that is treated as a magic instruction.
#define MAGIC (0x4711U)

/// The default index number used for magic instructions. All magic instructions
/// support multiple start and stop indices, which defaults to 0 if not
/// specified.
#define DEFAULT_INDEX (0x0000U)

/// Pseudo-hypercall number to signal the fuzzer to use the first argument to
/// the magic instruction as the pointer to the testcase buffer and the second
/// argument as a pointer to the size of the testcase buffer.
#define N_START_BUFFER_PTR_SIZE_PTR 1

/// HARNESS_START
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_INDEX` macro to specify different indices, then enable them
/// at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The size of the buffer pointed to by `size_ptr` will be saved as the
///   maximum testcase size. Each fuzzing iteration, the actual size of the
///   current testcase will be written to `*size_ptr`.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START(buffer, &size);
/// ```
#define HARNESS_START(buffer, size_ptr)                             \
  do {                                                              \
    __orr_extended2(N_START_BUFFER_PTR_SIZE_PTR, buffer, size_ptr); \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to use the first argument to
/// the magic instruction as the pointer to the testcase buffer and the second
/// argument as the maximum size of the testcase buffer.
#define N_START_BUFFER_PTR_SIZE_VAL 2

/// HARNESS_START_WITH_MAXIMUM_SIZE
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_INDEX` macro to specify different indices,
/// then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE(buffer, 1024);
/// ```
#define HARNESS_START_WITH_MAXIMUM_SIZE(buffer, max_size)           \
  do {                                                              \
    __orr_extended2(N_START_BUFFER_PTR_SIZE_VAL, buffer, max_size); \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to use the first argument to
/// the magic instruction as the pointer to the testcase buffer, the second
/// argument as a pointer to the size of the testcase buffer, and the third
/// argument as the maximum size of the testcase buffer.
#define N_START_BUFFER_PTR_SIZE_PTR_VAL 3

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX` macro to specify different
/// indices, then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The address `size_ptr` will be saved. Each fuzzing iteration, the actual
/// size of the current testcase will be written to `*size_ptr`.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, &size, 1024);
/// ```
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, size_ptr, max_size) \
  do {                                                                      \
    __orr_extended3(N_START_BUFFER_PTR_SIZE_PTR_VAL, buffer, size_ptr,      \
                    max_size);                                              \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to stop the current fuzzing
/// iteration and reset to the beginning of the fuzzing loop with a "normal"
/// stop status, indicating no solution has occurred.
#define N_STOP_NORMAL 4

/// HARNESS_STOP
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The
/// default index of 0 will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use the
/// `HARNESS_STOP_INDEX` macro to specify different indices, then enable them at
/// runtime by configuring the fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_STOP();
/// ```
#define HARNESS_STOP()    \
  do {                    \
    __orr(N_STOP_NORMAL); \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer that a custom assertion has
/// occurred, and the fuzzer should stop the current fuzzing iteration and reset
/// to the beginning of the fuzzing loop with a "solution" stop status.
#define N_STOP_ASSERT 5

/// HARNESS_ASSERT
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The default index of 0 will be
/// used. If you need to differentiate between multiple assertion harnesses
/// compiled into the same binary, you can use the `HARNESS_ASSERT_INDEX` macro
/// to specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_ASSERT();
/// ```
#define HARNESS_ASSERT()                           \
  do {                                             \
    __orr_extended1(N_STOP_ASSERT, DEFAULT_INDEX); \
  } while (0);

#endif  // TSFFS_H