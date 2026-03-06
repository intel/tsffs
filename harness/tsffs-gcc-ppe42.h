// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

/// Definitions and macros for compiled-in harnessing of C and C++ target
/// software for the PPE42 (PowerPC Processor Embedded 42) architecture
/// using the rlwimi magic instruction format

#ifndef TSFFS_GCC_PPE42_H
#define TSFFS_GCC_PPE42_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

// ============================================================================
// Low-level magic instruction primitives using rlwimi
// ============================================================================

/// __ppe42_magic
///
/// Invoke the magic instruction for PPE42 using the rlwimi instruction.
/// This is the standard magic instruction format used by SBE firmware.
///
/// The rlwimi instruction is a no-op on hardware but triggers Core_Magic_Instruction
/// HAP in SIMICS when magic breakpoints are enabled with:
///   simics> enable-magic-breakpoint
///
/// SBE uses magic numbers in the range 8000-8190. We use 8010-8019 for TSFFS.
///
/// # Arguments
///
/// * `n` - The magic number to pass (must be in range 8010-8019 for TSFFS)
#define __ppe42_magic(n) \
  __asm__ __volatile__("rlwimi %0,%0,0,%1,%2" \
    : \
    : "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : )

/// __ppe42_magic_extended1
///
/// Invoke the magic instruction with one pseudo-argument in register r10.
///
/// # Arguments
///
/// * `n` - The magic number
/// * `arg0` - The value to place in register r10
#define __ppe42_magic_extended1(n, arg0) \
  __asm__ __volatile__("mr 10, %0; rlwimi %1,%1,0,%2,%3" \
    : \
    : "r"(arg0), \
      "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : "r10")

/// __ppe42_magic_extended2
///
/// Invoke the magic instruction with two pseudo-arguments in registers r10 and r3.
///
/// # Arguments
///
/// * `n` - The magic number
/// * `arg0` - The value to place in register r10
/// * `arg1` - The value to place in register r3
#define __ppe42_magic_extended2(n, arg0, arg1) \
  __asm__ __volatile__("mr 10, %0; mr 3, %1; rlwimi %2,%2,0,%3,%4" \
    : \
    : "r"(arg0), "r"(arg1), \
      "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : "r10", "r3")

/// __ppe42_magic_extended3
///
/// Invoke the magic instruction with three pseudo-arguments in registers r10, r3, and r4.
///
/// # Arguments
///
/// * `n` - The magic number
/// * `arg0` - The value to place in register r10
/// * `arg1` - The value to place in register r3
/// * `arg2` - The value to place in register r4
#define __ppe42_magic_extended3(n, arg0, arg1, arg2) \
  __asm__ __volatile__("mr 10, %0; mr 3, %1; mr 4, %2; rlwimi %3,%3,0,%4,%5" \
    : \
    : "r"(arg0), "r"(arg1), "r"(arg2), \
      "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : "r10", "r3", "r4")

/// __ppe42_magic_extended4
///
/// Invoke the magic instruction with four pseudo-arguments in registers r10, r3, r4, and r5.
///
/// # Arguments
///
/// * `n` - The magic number
/// * `arg0` - The value to place in register r10
/// * `arg1` - The value to place in register r3
/// * `arg2` - The value to place in register r4
/// * `arg3` - The value to place in register r5
#define __ppe42_magic_extended4(n, arg0, arg1, arg2, arg3) \
  __asm__ __volatile__("mr 10, %0; mr 3, %1; mr 4, %2; mr 5, %3; rlwimi %4,%4,0,%5,%6" \
    : \
    : "r"(arg0), "r"(arg1), "r"(arg2), "r"(arg3), \
      "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : "r10", "r3", "r4", "r5")

// ============================================================================
// Magic number definitions
// ============================================================================

/// Magic number base for TSFFS on SBE (in SBE range 8000-8190)
#define TSFFS_MAGIC_BASE (8010)

/// The default index number used for magic instructions
#define DEFAULT_INDEX (0x0000U)

/// Magic numbers for TSFFS operations (offset from base)
#define N_START_BUFFER_PTR_SIZE_PTR (TSFFS_MAGIC_BASE + 1)  // 8011
#define N_START_BUFFER_PTR_SIZE_VAL (TSFFS_MAGIC_BASE + 2)  // 8012
#define N_START_BUFFER_PTR_SIZE_PTR_VAL (TSFFS_MAGIC_BASE + 3)  // 8013
#define N_STOP_NORMAL (TSFFS_MAGIC_BASE + 4)  // 8014
#define N_STOP_ASSERT (TSFFS_MAGIC_BASE + 5)  // 8015
#define N_COVERAGE (TSFFS_MAGIC_BASE + 6)  // 8016

// ============================================================================
// Fuzzing harness macros
// ============================================================================

/// HARNESS_START
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is called.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
#define HARNESS_START(buffer, size_ptr) \
  do { \
    __ppe42_magic_extended3(N_START_BUFFER_PTR_SIZE_PTR, DEFAULT_INDEX, (unsigned long)(buffer), (unsigned long)(size_ptr)); \
  } while (0)

/// HARNESS_START_INDEX
///
/// Signal the fuzzer to start with a specific index.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
#define HARNESS_START_INDEX(start_index, buffer, size_ptr) \
  do { \
    __ppe42_magic_extended3(N_START_BUFFER_PTR_SIZE_PTR, start_index, buffer, size_ptr); \
  } while (0)

/// HARNESS_START_WITH_MAXIMUM_SIZE
///
/// Signal the fuzzer to start with a maximum size value.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
#define HARNESS_START_WITH_MAXIMUM_SIZE(buffer, max_size) \
  do { \
    __ppe42_magic_extended3(N_START_BUFFER_PTR_SIZE_VAL, DEFAULT_INDEX, buffer, max_size); \
  } while (0)

/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX
///
/// Signal the fuzzer to start with a specific index and maximum size.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
#define HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(start_index, buffer, max_size) \
  do { \
    __ppe42_magic_extended3(N_START_BUFFER_PTR_SIZE_VAL, start_index, buffer, max_size); \
  } while (0)

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR
///
/// Signal the fuzzer to start with both size pointer and maximum size.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, size_ptr, max_size) \
  do { \
    __ppe42_magic_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, DEFAULT_INDEX, buffer, size_ptr, max_size); \
  } while (0)

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX
///
/// Signal the fuzzer to start with index, size pointer, and maximum size.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
#define HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(start_index, buffer, size_ptr, max_size) \
  do { \
    __ppe42_magic_extended4(N_START_BUFFER_PTR_SIZE_PTR_VAL, start_index, buffer, size_ptr, max_size); \
  } while (0)

/// HARNESS_STOP
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop.
#define HARNESS_STOP() \
  do { \
    __ppe42_magic_extended1(N_STOP_NORMAL, DEFAULT_INDEX); \
  } while (0)

/// HARNESS_STOP_INDEX
///
/// Signal the fuzzer to stop with a specific index.
///
/// # Arguments
///
/// - `stop_index`: The index to use for this stop harness
#define HARNESS_STOP_INDEX(stop_index) \
  do { \
    __ppe42_magic_extended1(N_STOP_NORMAL, stop_index); \
  } while (0)

/// HARNESS_ASSERT
///
/// Signal the fuzzer that a custom assertion has occurred.
#define HARNESS_ASSERT() \
  do { \
    __ppe42_magic_extended1(N_STOP_ASSERT, DEFAULT_INDEX); \
  } while (0)

/// HARNESS_ASSERT_INDEX
///
/// Signal the fuzzer that a custom assertion has occurred with a specific index.
///
/// # Arguments
///
/// - `assert_index`: The index to use for this assertion harness
#define HARNESS_ASSERT_INDEX(assert_index) \
  do { \
    __ppe42_magic_extended1(N_STOP_ASSERT, assert_index); \
  } while (0)

// ============================================================================
// Coverage tracking macros (for manual instrumentation)
// ============================================================================

/// HARNESS_COVERAGE
///
/// Report a coverage point with a unique ID. This is used for manual coverage
/// instrumentation on architectures that don't support automatic instruction-level
/// coverage tracking (like PPE42).
///
/// Each branch or basic block should have a unique coverage_id. The fuzzer will
/// track which coverage points are hit and use this information to guide mutations.
///
/// # Arguments
///
/// - `coverage_id`: A unique identifier for this coverage point (0-65535)
///
/// # Example
///
/// ```c
/// if (condition) {
///     HARNESS_COVERAGE(1);  // Coverage point for true branch
///     // ... code ...
/// } else {
///     HARNESS_COVERAGE(2);  // Coverage point for false branch
///     // ... code ...
/// }
/// ```
#define HARNESS_COVERAGE(coverage_id) \
    __ppe42_magic_extended1(N_COVERAGE, coverage_id)

/// COVERAGE_BRANCH
///
/// Alias for HARNESS_COVERAGE for convenience.
#define COVERAGE_BRANCH(id) HARNESS_COVERAGE(id)

#endif  // TSFFS_GCC_PPE42_H

// Made with Bob
