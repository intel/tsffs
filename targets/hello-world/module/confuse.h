/* confuse.h - Confuse C header for SIMICS fuzzing
 *
 * This should be the the ONLY header needed to use CONFUSE with a project
 *
 */

#ifndef CONFUSE_H
#define CONFUSE_H

#include <cpuid.h>
#include <stdint.h>

#if defined(__GNUC__) && defined(__x86_64__)

// This value must be the lower 16 bits of the CPUID input (rax/eax register) to trigger magic
#define SIMICS_MAGIC_CPUID (0x4711ULL)

#define CONFUSE_STOP_SIGNAL (0x4242ULL)

// 4K should be plenty for input size
#define CONFUSE_MAXSIZE (0x1000ULL)

#define __CONFUSE_ASSERT(condition) do { \
    typedef int __check[(condition) ? 1 : -1] __attribute__((unused)); \
} while (0)

/* Fuzzing start harness
 *
 * This harness takes an address and size of a memory location and signals SIMICS to start the
 * fuzzing loop at this location. 
 */
#define HARNESS_START(addr, size) do { \
    __CONFUSE_ASSERT(((uint64_t)addr) < (1ULL << (32ULL))); \
    __CONFUSE_ASSERT(((uint64_t)size) < (1ULL << 16ULL)); \
    uint64_t cpuid_input = (((uint64_t)addr) << 32ULL) | (((uint64_t)size) << 16ULL) | SIMICS_MAGIC_CPUID; \
    uint64_t _a, _b, _c, _d; \
    __cpuid(cpuid_input, _a, _b, _c, _d); \
} while (0)

#define HARNESS_STOP() do { \
    uint64_t cpuid_input = (CONFUSE_STOP_SIGNAL << 16ULL) | SIMICS_MAGIC_CPUID; \
    uint64_t _a, _b, _c, _d; \
    __cpuid(cpuid_input, _a, _b, _c, _d); \
} while (0)


#else
#error "TODO: Unsupported compiler or target architecture"
#endif


 #endif /* CONFUSE_H */