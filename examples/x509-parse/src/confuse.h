/* confuse.h - Confuse C header for SIMICS fuzzing
 *
 * This should be the the ONLY header needed to use CONFUSE with a project
 *
 */

#ifndef CONFUSE_H
#define CONFUSE_H

#include <stdint.h>

/* From cpuid.h in GCC */
#ifndef __x86_64__
/// This macro performs a CPUID instruction with an input (in this case a magic value)
/// as well as setting the (r/e)si/(r/e)di registers to a specific value (in_0, in_1) in
/// order to pass information to the simulator
#define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1)                 \
    do {                                                                               \
        if (__builtin_constant_p(level) && (level) != 1)                               \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d),                 \
                                   "=S"(*inout_ptr_0), "=D"(*inout_ptr_1)              \
                                 : "0"(level), "S"(*inout_ptr_2), "D"(*inout_ptr_1));  \
        else                                                                           \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d)                  \
                                 : "0"(level), "1"(0), "2"(0), "S"(in_0), "D"(in_1));  \
    } while (0)
#define __cpuid_extended1(level, a, b, c, d, inout_ptr_0)                              \
    do {                                                                               \
        if (__builtin_constant_p(level) && (level) != 1)                               \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d),                 \
                                   "=S"(*inout_ptr_0), "=D"(*inout_ptr_1)              \
                                 : "0"(level), "S"(*inout_ptr_2));                     \
        else                                                                           \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d)                  \
                                 : "0"(level), "1"(0), "2"(0), "S"(in_0));             \
    } while (0)
#define __cpuid(level, a, b, c, d)                                                     \
    do {                                                                               \
        if (__builtin_constant_p(level) && (level) != 1)                               \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d)                  \
                                 : "0"(level));                                        \
        else                                                                           \
            __asm__ __volatile__("cpuid\n\t"                                           \
                                 : "=a"(a), "=b"(b), "=c"(c), "=d"(d)                  \
                                 : "0"(level), "1"(0), "2"(0));                        \
    } while (0)
#else // __x86_64__
/// This macro performs a CPUID instruction with an input (in this case a magic value)
/// as well as setting the (r/e)si/(r/e)di registers to a specific value (in_0, in_1) in
/// order to pass information to the simulator
#define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1)                 \
    __asm__ __volatile__("cpuid\n\t"                                                   \
                         : "=a"(a), "=b"(b), "=c"(c), "=d"(d), "=S"(*inout_ptr_0),     \
                           "=D"(*inout_ptr_1)                                          \
                         : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1))
#define __cpuid_extended1(level, a, b, c, d, inout_ptr_0)                              \
    __asm__ __volatile__("cpuid\n\t"                                                   \
                         : "=a"(a), "=b"(b), "=c"(c), "=d"(d), "=S"(*inout_ptr_0)      \
                         : "0"(level), "S"(*inout_ptr_0))
#define __cpuid(level, a, b, c, d)                                                     \
    __asm__ __volatile__("cpuid\n\t" : "=a"(a), "=b"(b), "=c"(c), "=d"(d) : "0"(level))
#endif // __x86_64__

#if defined(__GNUC__) && defined(__x86_64__)

// This value must be the lower 16 bits of the CPUID input (rax/eax register) to trigger
// magic
#define SIMICS_MAGIC_CPUID (0x4711U)

#define CONFUSE_STOP_SIGNAL (0x4242U)
#define CONFUSE_START_SIGNAL (0x4343U)

#define __CONFUSE_ASSERT(condition)                                                    \
    do {                                                                               \
        typedef int __check[(condition) ? 1 : -1] __attribute__((unused));             \
    } while (0)

/* Fuzzing start harness
 *
 * This harness takes an address and size of a memory location and signals SIMICS to
 * start the fuzzing loop at this location. The input size must be a mutable variable
 * and will be modified such that its contents are the actual size of the buffer.
 */
#define HARNESS_START(addr_ptr, size_ptr)                                              \
    do {                                                                               \
        uint32_t _a, _b, _c, _d;                                                       \
        uint32_t cpuid_input = (CONFUSE_START_SIGNAL << 16U) | SIMICS_MAGIC_CPUID;     \
        __cpuid_extended2(cpuid_input, _a, _b, _c, _d, addr_ptr, size_ptr);            \
    } while (0)

#define HARNESS_STOP()                                                                 \
    do {                                                                               \
        uint32_t cpuid_input = (CONFUSE_STOP_SIGNAL << 16U) | SIMICS_MAGIC_CPUID;      \
        uint32_t _a, _b, _c, _d;                                                       \
        __cpuid(cpuid_input, _a, _b, _c, _d);                                          \
    } while (0)
#define HARNESS_STOP_EXTENDED(val_ptr)                                                 \
    do {                                                                               \
        uint32_t cpuid_input = (CONFUSE_STOP_SIGNAL << 16U) | SIMICS_MAGIC_CPUID;      \
        uint32_t _a, _b, _c, _d;                                                       \
        __cpuid_extended1(cpuid_input, _a, _b, _c, _d, val_ptr);                       \
    } while (0)

#else // defined(__GNUC__) && defined(__x86_64__)
#error "TODO: Unsupported compiler or target architecture"
#endif

#endif /* CONFUSE_H */