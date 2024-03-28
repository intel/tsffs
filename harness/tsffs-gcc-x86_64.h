// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// Definitions and macros for compiled-in harnessing of C and C++ target
/// software for the x86_64 architecture.

#ifndef TSFFS_H
#define TSFFS_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

/// __cpuid
///
/// Invoke the CPUID instruction with a specific value `value` in register
/// `rax`.
///
/// # Arguments
///
/// - `value`: The value to load into the `rax` register before invoking the
///   CPUID instruction
#define __cpuid(value)                                          \
  unsigned int _a __attribute__((unused)) = 0;                  \
  unsigned int _b __attribute__((unused)) = 0;                  \
  unsigned int _c __attribute__((unused)) = 0;                  \
  unsigned int _d __attribute__((unused)) = 0;                  \
  __asm__ __volatile__("cpuid\n\t"                              \
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d) \
                       : "a"(value));

/// __cpuid_extended1
///
/// Invoke the CPUID instruction with a specific value `value` in register
/// `rax` and a pseudo-argument in register `rdi`.
///
/// # Arguments
///
/// - `value`: The value to load into the `rax` register before invoking the
///   CPUID instruction
/// - `arg0`: The value to load into the `rdi` register before invoking the
///   CPUID instruction
#define __cpuid_extended1(value, arg0)                          \
  unsigned int _a __attribute__((unused)) = 0;                  \
  unsigned int _b __attribute__((unused)) = 0;                  \
  unsigned int _c __attribute__((unused)) = 0;                  \
  unsigned int _d __attribute__((unused)) = 0;                  \
  __asm__ __volatile__("cpuid\n\t"                              \
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d) \
                       : "a"(value), "D"(arg0));

/// __cpuid_extended2
///
/// Invoke the CPUID instruction with a specific value `value` in register
/// `rax` and a pseudo-arguments in registers `rdi` and `rsi`.
///
/// # Arguments
///
/// - `value`: The value to load into the `rax` register before invoking the
///   CPUID instruction
/// - `arg0`: The value to load into the `rdi` register before invoking the
///   CPUID instruction
/// - `arg1`: The value to load into the `rsi` register before invoking the
///   CPUID instruction
#define __cpuid_extended2(value, arg0, arg1)                    \
  unsigned int _a __attribute__((unused)) = 0;                  \
  unsigned int _b __attribute__((unused)) = 0;                  \
  unsigned int _c __attribute__((unused)) = 0;                  \
  unsigned int _d __attribute__((unused)) = 0;                  \
  __asm__ __volatile__("cpuid\n\t"                              \
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d) \
                       : "a"(value), "D"(arg0), "S"(arg1));

/// __cpuid_extended3
///
/// Invoke the CPUID instruction with a specific value `value` in register
/// `rax` and a pseudo-arguments in registers `rdi`, `rsi`, and `rdx`.
///
/// # Arguments
///
/// - `value`: The value to load into the `rax` register before invoking the
///   CPUID instruction
/// - `arg0`: The value to load into the `rdi` register before invoking the
///   CPUID instruction
/// - `arg1`: The value to load into the `rsi` register before invoking the
///   CPUID instruction
/// - `arg2`: The value to load into the `rdx` register before invoking the
///   CPUID instruction
#define __cpuid_extended3(value, arg0, arg1, arg2)              \
  unsigned int _a __attribute__((unused)) = 0;                  \
  unsigned int _b __attribute__((unused)) = 0;                  \
  unsigned int _c __attribute__((unused)) = 0;                  \
  unsigned int _d __attribute__((unused)) = 0;                  \
  __asm__ __volatile__("cpuid\n\t"                              \
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d) \
                       : "a"(value), "D"(arg0), "S"(arg1), "d"(arg2));

/// __cpuid_extended4
///
/// Invoke the CPUID instruction with a specific value `value` in register
/// `rax` and a pseudo-arguments in registers `rdi`, `rsi`, `rdx`, and `rcx`.
///
/// # Arguments
///
/// - `value`: The value to load into the `rax` register before invoking the
///   CPUID instruction
/// - `arg0`: The value to load into the `rdi` register before invoking the
///   CPUID instruction
/// - `arg1`: The value to load into the `rsi` register before invoking the
///   CPUID instruction
/// - `arg2`: The value to load into the `rdx` register before invoking the
///   CPUID instruction
/// - `arg3`: The value to load into the `rcx` register before invoking the
///   CPUID instruction
#define __cpuid_extended4(value, arg0, arg1, arg2, arg3)              \
  unsigned int _a __attribute__((unused)) = 0;                        \
  unsigned int _b __attribute__((unused)) = 0;                        \
  unsigned int _c __attribute__((unused)) = 0;                        \
  unsigned int _d __attribute__((unused)) = 0;                        \
  __asm__ __volatile__("cpuid\n\t"                                    \
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d)       \
                       : "a"(value), "D"(arg0), "S"(arg1), "d"(arg2), \
                         "c"(arg3));

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
#define N_START_BUFFER_PTR_SIZE_PTR (0x0001U)

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
#define HARNESS_START(buffer, size_ptr)                                  \
  do {                                                                   \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_PTR << 0x10U) | MAGIC; \
    __cpuid_extended3(value, DEFAULT_INDEX, buffer, size_ptr);           \
  } while (0);

/// HARNESS_START_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
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
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_INDEX(0x0001U, buffer, &size);
/// ```
#define HARNESS_START_INDEX(start_index, buffer, size_ptr)               \
  do {                                                                   \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_PTR << 0x10U) | MAGIC; \
    __cpuid_extended3(value, start_index, buffer, size_ptr);             \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to use the first argument to
/// the magic instruction as the pointer to the testcase buffer and the second
/// argument as the maximum size of the testcase buffer.
#define N_START_BUFFER_PTR_SIZE_VAL (0x0002U)

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
#define HARNESS_START_WITH_MAXIMUM_SIZE(buffer, max_size)                \
  do {                                                                   \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_VAL << 0x10U) | MAGIC; \
    __cpuid_extended3(value, DEFAULT_INDEX, buffer, max_size);           \
  } while (0);

/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
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
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(0x0001U, buffer, 1024);
/// ```
#define HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(start_index, buffer, max_size) \
  do {                                                                       \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_VAL << 0x10U) | MAGIC;     \
    __cpuid_extended3(value, start_index, buffer, max_size);                 \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to use the first argument to
/// the magic instruction as the pointer to the testcase buffer, the second
/// argument as a pointer to the size of the testcase buffer, and the third
/// argument as the maximum size of the testcase buffer.
#define N_START_BUFFER_PTR_SIZE_PTR_VAL (0x0003U)

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
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, size_ptr, max_size)  \
  do {                                                                       \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_PTR_VAL << 0x10U) | MAGIC; \
    __cpuid_extended4(value, DEFAULT_INDEX, buffer, size_ptr, max_size);     \
  } while (0);

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
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
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(0x0001U, buffer, &size, 1024);
/// ```
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(start_index, buffer,   \
                                                      size_ptr, max_size)    \
  do {                                                                       \
    unsigned int value = (N_START_BUFFER_PTR_SIZE_PTR_VAL << 0x10U) | MAGIC; \
    __cpuid_extended4(value, start_index, buffer, size_ptr, max_size);       \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer to stop the current fuzzing
/// iteration and reset to the beginning of the fuzzing loop with a "normal"
/// stop status, indicating no solution has occurred.
#define N_STOP_NORMAL (0x0004U)

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
#define HARNESS_STOP()                                     \
  do {                                                     \
    unsigned int value = (N_STOP_NORMAL << 0x10U) | MAGIC; \
    __cpuid_extended1(value, DEFAULT_INDEX);               \
  } while (0);

/// HARNESS_STOP_INDEX
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The index
/// specified by `stop_index` will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// # Arguments
///
/// - `stop_index`: The index to use for this stop harness
///
/// # Example
///
/// ```
/// HARNESS_STOP_INDEX(0x0001U);
/// ```
#define HARNESS_STOP_INDEX(stop_index)                     \
  do {                                                     \
    unsigned int value = (N_STOP_NORMAL << 0x10U) | MAGIC; \
    __cpuid_extended1(value, stop_index);                  \
  } while (0);

/// Pseudo-hypercall number to signal the fuzzer that a custom assertion has
/// occurred, and the fuzzer should stop the current fuzzing iteration and reset
/// to the beginning of the fuzzing loop with a "solution" stop status.
#define N_STOP_ASSERT (0x0005U)

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
#define HARNESS_ASSERT()                                   \
  do {                                                     \
    unsigned int value = (N_STOP_ASSERT << 0x10U) | MAGIC; \
    __cpuid_extended1(value, DEFAULT_INDEX);               \
  } while (0);

/// HARNESS_ASSERT_INDEX
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The index specified by
/// `assert_index` will be used. If you need to differentiate between multiple
/// assertion harnesses compiled into the same binary, you can use this macro to
/// specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Arguments
///
/// - `assert_index`: The index to use for this assertion harness
///
/// # Example
///
/// ```
/// HARNESS_ASSERT_INDEX(0x0001U);
/// ```
#define HARNESS_ASSERT_INDEX(assert_index)                 \
  do {                                                     \
    unsigned int value = (N_STOP_ASSERT << 0x10U) | MAGIC; \
    __cpuid_extended1(value, assert_index);                \
  } while (0);

#endif  // TSFFS_H