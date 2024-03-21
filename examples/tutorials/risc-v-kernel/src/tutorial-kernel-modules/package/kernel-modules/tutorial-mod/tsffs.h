// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#ifdef __GNUC__
#ifdef __i386__
// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// Definitions and macros for compiled-in harnessing of C and C++ target
/// software for the X86 architecture.

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
#elif __x86_64__
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
#elif __riscv && !__LP64__
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

/// __srai
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n`
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
#define __srai(value) \
  __asm__ __volatile__("srai zero, zero, %0" : : "I"(value) :)

/// __srai_extended1
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and a pseudo-argument in register `a0`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
#define __srai_extended1(value, arg0)                   \
  __asm__ __volatile__("mv a0, %0; srai zero, zero, %1" \
                       :                                \
                       : "r"(arg0), "I"(value)          \
                       : "a0");

/// __srai_extended2
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0` and
/// `a1`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
#define __srai_extended2(value, arg0, arg1)                        \
  __asm__ __volatile__("mv a0, %0; mv a1, %1; srai zero, zero, %2" \
                       :                                           \
                       : "r"(arg0), "r"(arg1), "I"(value)          \
                       : "a0", "a1");

/// __srai_extended3
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0`, `a1`,
/// and `a2`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
/// * `arg2` - The value to place in register `a2`
#define __srai_extended3(value, arg0, arg1, arg2)                             \
  __asm__ __volatile__("mv a0, %0; mv a1, %1; mv a2, %2; srai zero, zero, %3" \
                       :                                                      \
                       : "r"(arg0), "r"(arg1), "r"(arg2), "I"(value)          \
                       : "a0", "a1", "a2");

/// __srai_extended4
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0`, `a1`,
/// `a2`, and `a3`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
/// * `arg2` - The value to place in register `a2`
/// * `arg3` - The value to place in register `a3`
#define __srai_extended4(value, arg0, arg1, arg2, arg3)                 \
  __asm__ __volatile__(                                                 \
      "mv a0, %0; mv a1, %1; mv a2, %2; mv a3, %3; srai zero, zero, %4" \
      :                                                                 \
      : "r"(arg0), "r"(arg1), "r"(arg2), "r"(arg3), "I"(value)          \
      : "a0", "a1", "a2", "a3");

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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_PTR, DEFAULT_INDEX, buffer, \
                     size_ptr);                                          \
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
#define HARNESS_START_INDEX(start_index, buffer, size_ptr)             \
  do {                                                                 \
    __srai_extended3(N_START_BUFFER_PTR_SIZE_PTR, start_index, buffer, \
                     size_ptr);                                        \
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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_VAL, DEFAULT_INDEX, buffer, \
                     max_size);                                          \
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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_VAL, start_index, buffer,       \
                     max_size);                                              \
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
    __srai_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, DEFAULT_INDEX, buffer, \
                     size_ptr, max_size);                                    \
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
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(start_index, buffer, \
                                                      size_ptr, max_size)  \
  do {                                                                     \
    __srai_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, start_index, buffer, \
                     size_ptr, max_size);                                  \
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
#define HARNESS_STOP()                              \
  do {                                              \
    __srai_extended1(N_STOP_NORMAL, DEFAULT_INDEX); \
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
#define HARNESS_STOP_INDEX(stop_index)           \
  do {                                           \
    __srai_extended1(N_STOP_NORMAL, stop_index); \
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
#define HARNESS_ASSERT()                            \
  do {                                              \
    __srai_extended1(N_STOP_ASSERT, DEFAULT_INDEX); \
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
#define HARNESS_ASSERT_INDEX(assert_index)         \
  do {                                             \
    __srai_extended1(N_STOP_ASSERT, assert_index); \
  } while (0);

#endif  // TSFFS_H
#elif __riscv && __LP64__
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

/// __srai
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n`
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
#define __srai(value) \
  __asm__ __volatile__("srai zero, zero, %0" : : "I"(value) :)

/// __srai_extended1
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and a pseudo-argument in register `a0`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
#define __srai_extended1(value, arg0)                   \
  __asm__ __volatile__("mv a0, %0; srai zero, zero, %1" \
                       :                                \
                       : "r"(arg0), "I"(value)          \
                       : "a0");

/// __srai_extended2
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0` and
/// `a1`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
#define __srai_extended2(value, arg0, arg1)                        \
  __asm__ __volatile__("mv a0, %0; mv a1, %1; srai zero, zero, %2" \
                       :                                           \
                       : "r"(arg0), "r"(arg1), "I"(value)          \
                       : "a0", "a1");

/// __srai_extended3
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0`, `a1`,
/// and `a2`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
/// * `arg2` - The value to place in register `a2`
#define __srai_extended3(value, arg0, arg1, arg2)                             \
  __asm__ __volatile__("mv a0, %0; mv a1, %1; mv a2, %2; srai zero, zero, %3" \
                       :                                                      \
                       : "r"(arg0), "r"(arg1), "r"(arg2), "I"(value)          \
                       : "a0", "a1", "a2");

/// __srai_extended4
///
/// Invoke the magic instruction defined by SIMICS for the RISC-V architecture
/// with a specific value of `n` and pseudo-arguments in registers `a0`, `a1`,
/// `a2`, and `a3`.
///
/// # Arguments
///
/// * `value` - The value of `n` to use in the magic instruction
/// * `arg0` - The value to place in register `a0`
/// * `arg1` - The value to place in register `a1`
/// * `arg2` - The value to place in register `a2`
/// * `arg3` - The value to place in register `a3`
#define __srai_extended4(value, arg0, arg1, arg2, arg3)                 \
  __asm__ __volatile__(                                                 \
      "mv a0, %0; mv a1, %1; mv a2, %2; mv a3, %3; srai zero, zero, %4" \
      :                                                                 \
      : "r"(arg0), "r"(arg1), "r"(arg2), "r"(arg3), "I"(value)          \
      : "a0", "a1", "a2", "a3");

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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_PTR, DEFAULT_INDEX, buffer, size_ptr);            \
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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_PTR, start_index, buffer, size_ptr);              \
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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_VAL, DEFAULT_INDEX, buffer, max_size);            \
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
    __srai_extended3(N_START_BUFFER_PTR_SIZE_VAL, start_index, buffer, max_size);                  \
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
    __srai_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, DEFAULT_INDEX, buffer, size_ptr, max_size);      \
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
    __srai_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, start_index, buffer, size_ptr, max_size);        \
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
    __srai_extended1(N_STOP_NORMAL, DEFAULT_INDEX);                \
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
    __srai_extended1(N_STOP_NORMAL, stop_index);                   \
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
    __srai_extended1(N_STOP_ASSERT, DEFAULT_INDEX);                \
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
    __srai_extended1(N_STOP_ASSERT, assert_index);                 \
  } while (0);

#endif  // TSFFS_H
#elif __aarch64__
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
  __asm__ __volatile__("orr x" __tostring(value) ", x" __tostring( \
      value) ", x" __tostring(value));

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
      "mov x28, %0; orr x" __tostring(value) ", x" __tostring( \
          value) ", x" __tostring(value)                       \
      :                                                        \
      : "g"(arg0));

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
#define __orr_extended2(value, arg0, arg1)                                  \
  __asm__ __volatile__(                                                     \
      "mov x28, %0; mov x27, %1; orr x" __tostring(value) ", x" __tostring( \
          value) ", x" __tostring(value)                                    \
      :                                                                     \
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
      "mov x28, %0; mov x27, %1; mov x26, %2; orr x" __tostring( \
          value) ", x" __tostring(value) ", x" __tostring(value) \
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
      "mov x28, %0; mov x27, %1; mov x26, %2; mov x25, %3; "              \
      "orr x" __tostring(value) ", x" __tostring(value) ", x" __tostring( \
          value)                                                          \
      :                                                                   \
      : "r"(arg0), "r"(arg1), "r"(arg2), "r"(arg3));

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
#define HARNESS_START(buffer, size_ptr)                                 \
  do {                                                                  \
    __orr_extended3(N_START_BUFFER_PTR_SIZE_PTR, DEFAULT_INDEX, buffer, \
                    size_ptr);                                          \
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
#define HARNESS_START_INDEX(start_index, buffer, size_ptr)            \
  do {                                                                \
    __orr_extended3(N_START_BUFFER_PTR_SIZE_PTR, start_index, buffer, \
                    size_ptr);                                        \
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
#define HARNESS_START_WITH_MAXIMUM_SIZE(buffer, max_size)               \
  do {                                                                  \
    __orr_extended3(N_START_BUFFER_PTR_SIZE_VAL, DEFAULT_INDEX, buffer, \
                    max_size);                                          \
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
    __orr_extended3(N_START_BUFFER_PTR_SIZE_VAL, start_index, buffer,        \
                    max_size);                                               \
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
    __orr_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, DEFAULT_INDEX, buffer, \
                    size_ptr, max_size);                                    \
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
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(start_index, buffer, \
                                                      size_ptr, max_size)  \
  do {                                                                     \
    __orr_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, start_index, buffer,  \
                    size_ptr, max_size);                                   \
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
#define HARNESS_STOP()                             \
  do {                                             \
    __orr_extended1(N_STOP_NORMAL, DEFAULT_INDEX); \
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
#define HARNESS_STOP_INDEX(stop_index)          \
  do {                                          \
    __orr_extended1(N_STOP_NORMAL, stop_index); \
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
#define HARNESS_ASSERT_INDEX(assert_index)        \
  do {                                            \
    __orr_extended1(N_STOP_ASSERT, assert_index); \
  } while (0);

#endif  // TSFFS_H
#elif __arm__
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
#else
#error "Unsupported platform!"
#endif
#elif _MSC_VER
#ifndef TSFFS_H
#define TSFFS_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

#include <stddef.h>

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
void HARNESS_START(void *buffer, void *size_ptr);

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
void HARNESS_START_INDEX(size_t start_index, void *buffer, void *size_ptr);

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
void HARNESS_START_WITH_MAXIMUM_SIZE(void *buffer, size_t max_size);

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
void HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(size_t start_index, void *buffer,
                                           size_t max_size);

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
void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(void *buffer, void *size_ptr,
                                             size_t max_size);

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
void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(size_t start_index,
                                                   void *buffer, void *size_ptr,
                                                   size_t max_size);

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
void HARNESS_STOP(void);

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
void HARNESS_STOP_INDEX(size_t stop_index);

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
void HARNESS_ASSERT(void);

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
void HARNESS_ASSERT_INDEX(size_t assert_index);

#endif  // TSFFS_H
#else
#error "Unsupported compiler!"
#endif
