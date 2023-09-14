#ifndef TSFFS_H
#define TSFFS_H

// Copyright (C) 2023 Intel Corporaton
// SPDX-License-Identifier: Apache-2.0


#if (defined(__x86_64__) || defined(_WIN64))
#define MAGIC 18193
#endif

/**
 * MAGIC_START, when passed as the value of `n` in a [Magic
 * Instruction](https://simics-download.pdx.intel.com/simics-6/docs/html/simics-user-guide/breakpoints.html#Magic-Breakpoints)
 * indicates that the register `ARG0` and `ARG1` (as defined per platform) are set to the buffer
 * pointer and a pointer to the buffer size, respectively.
 */
#define MAGIC_START 1

/**
 * MAGIC_STOP, when passed as the value of `n` in a [Magic
 * Instruction](https://simics-download.pdx.intel.com/simics-6/docs/html/simics-user-guide/breakpoints.html#Magic-Breakpoints)
 * indicates that execution should stop, and the root fuzzing snapshot should be restored
 */
#define MAGIC_STOP 2

/**
 * MAGIC_START_WININTRIN, when passed as the value of `n` in a magic instruction, indicates a
 * magic start sequence that is supported by X64 windows intrinsic `__cpuidex`. This signals the
 * start of the following sequence of cpuid (as this is only supported on X64) instructions:
 * - cpuid eax=[(MAGIC_START_WININTRIN << 16U) | MAGIC] ecx=(BUFFER_PTR & 0xffffffff)
 * - cpuid eax=[(MAGIC_START_WININTRIN << 16U) | MAGIC] ecx=((BUFFER_PTR >> 32U) & 0xffffffff)
 * - cpuid eax=[(MAGIC_START_WININTRIN << 16U) | MAGIC] ecx=(SIZE & 0xffffffff)
 * - cpuid eax=[(MAGIC_START_WININTRIN << 16U) | MAGIC] ecx=((SIZE >> 32U) & 0xffffffff)
 *
 * The first three `__cpuid` calls output no value into the `cpuInfo` buffer. The fourth sets
 * `cpuInfo` to:
 *
 * ```c
 *  {
 *     BUFFER_PTR & 0xffffffff,
 *     (BUFFER_PTR >> 32U) & 0xffffffff,
 *     SIZE & 0xffffffff,
 *     (SIZE >> 32U) & 0xffffffff,
 * }
 * ```
 */
#define MAGIC_START_WININTRIN 3

/**
 *
 */
#define TSFFS_INCLUDE_VERSION "0.1.0"
 void __version_marker(void);

/**
 *
 */
#define TSFFS_INCLUDE_VERSION_MAJOR "0"
 void __version_marker_major(void);

/**
 *
 */
#define TSFFS_INCLUDE_VERSION_MINOR "1"
 void __version_marker_minor(void);

/**
 *
 */
#define TSFFS_INCLUDE_VERSION_PATCH "0"
 void __version_marker_patch(void);

#if (defined(__x86_64__) && defined(__unix__))
/**
 *
 */

#include <stdint.h>
#define __cpuid_extended2(leaf, a, b, c, d, inout_ptr_0, inout_ptr_1) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0), "=D"(*inout_ptr_1) \
                       : "0"(leaf), "S"(*inout_ptr_0), "D"(*inout_ptr_1))

#define __cpuid_extended1(leaf, a, b, c, d, inout_ptr_0) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0) \
                       : "0"(leaf), "S"(*inout_ptr_0))

#define __cpuid(leaf, a, b, c, d) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \
                       : "0"(leaf))

#define __arch_harness_start(addr_ptr, size_ptr) \
    do { \
        uint32_t _a __attribute__((unused)) = 0; \
        uint32_t _b __attribute__((unused)) = 0; \
        uint32_t _c __attribute__((unused)) = 0; \
        uint32_t _d __attribute__((unused)) = 0; \
        uint32_t leaf = (MAGIC_START << 16U) | MAGIC; \
        __cpuid_extended2(leaf, _a, _b, _c, _d, addr_ptr, size_ptr); \
    } while (0)

#define __arch_harness_stop() \
    do { \
        uint32_t _a __attribute__((unused)) = 0; \
        uint32_t _b __attribute__((unused)) = 0; \
        uint32_t _c __attribute__((unused)) = 0; \
        uint32_t _d __attribute__((unused)) = 0; \
        uint32_t leaf = (MAGIC_STOP << 16U) | MAGIC; \
        __cpuid(leaf, _a, _b, _c, _d); \
    } while (0)

#define __arch_harness_stop_extended(val_ptr) \
    do { \
        uint32_t _a __attribute__((unused)) = 0; \
        uint32_t _b __attribute__((unused)) = 0; \
        uint32_t _c __attribute__((unused)) = 0; \
        uint32_t _d __attribute__((unused)) = 0; \
        uint32_t leaf = (MAGIC_STOP << 16U) | MAGIC; \
        __cpuid_extended1(leaf, _a, _b, _c, _d, val_ptr); \
    } while (0)

void __marker_x86_64_unix(void);
#endif

#if defined(_WIN64)
/**
 *
 */

#define __arch_harness_start(addr_ptr, size_ptr) \
    do { \
        int cpuInfo[4] = {0}; \
        int function_id_start = (MAGIC_START_WININTRIN << 16U) | MAGIC; \
        int subfunction_id_addr_low = (int)(((long long)*addr_ptr) & 0xffffffff); \
        int subfunction_id_addr_hi  = (int)(((long long)*addr_ptr) >> 32U); \
        int subfunction_id_size_low = (int)(((long long)*size_ptr) & 0xffffffff); \
        int subfunction_id_size_hi  = (int)(((long long)*size_ptr) >> 32U); \
        __cpuidex(cpuInfo, function_id_start, subfunction_id_addr_low); \
        __cpuidex(cpuInfo, function_id_start, subfunction_id_addr_hi); \
        __cpuidex(cpuInfo, function_id_start, subfunction_id_size_low); \
        __cpuidex(cpuInfo, function_id_start, subfunction_id_size_hi); \
        *(long long *)addr_ptr = 0; \
        *(long long *)addr_ptr |= (long long)cpuInfo[0]; \
        *(long long *)addr_ptr |= ((long long)cpuInfo[1]) << 32U; \
        *(long long *)size_ptr = 0; \
        *(long long *)size_ptr |= (long long)cpuInfo[2]; \
        *(long long *)size_ptr |= ((long long)cpuInfo[3]) << 32U; \
    } while (0)

#define __arch_harness_stop() \
    do { \
        int cpuInfo[4] = {0}; \
        int function_id_stop = (MAGIC_STOP << 16U) | MAGIC; \
        __cpuid(cpuInfo, function_id_stop); \
    } while (0)

#define __arch_harness_stop_extended(val_ptr) \
    __arch_harness_stop()

void __marker_x86_64_windows(void);
#endif

/**
 *
 */

#define HARNESS_START(addr_ptr, size_ptr) \
    do { \
        __arch_harness_start(addr_ptr, size_ptr); \
    } while (0)

#define HARNESS_STOP()  \
    do { \
        __arch_harness_stop(); \
    } while (0)

#define HARNESS_STOP_EXTENDED(val_ptr) \
    do { \
        __arch_harness_stop_extended(val_ptr); \
    } while (0)

void __marker(void);

#endif /* TSFFS_H */
