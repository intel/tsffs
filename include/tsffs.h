#ifndef TSFFS_H
#define TSFFS_H

#include <stdint.h>


#if (defined(__i386__) || defined(__i586__) || defined(__i686__))
#define MAGIC 18193
#endif

#if defined(__x86_64__)
#define MAGIC 18193
#endif

#define MAGIC_START 1

#define MAGIC_STOP 2

#if (defined(__i386__) || defined(__i586__) || defined(__i686__))
/**
 * X86 32:
 *
 */

#define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1) \
    __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0), "=D"(*inout_ptr_1) \
                       : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1) \
                       : "memory")

#define __cpuid_extended1(level, a, b, c, d, inout_ptr_0) \
    __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0) \
                       : "0"(level), "S"(*inout_ptr_0) \
                       : "memory")

#define __cpuid(level, a, b, c, d) \
    __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \
                       : "0"(level) \
                       : "memory")

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

void __marker_i386(void);
#endif

#if defined(__x86_64__)
/**
 * X86_64:
 *
 */

#define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0), "=D"(*inout_ptr_1) \
                       : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1))

#define __cpuid_extended1(level, a, b, c, d, inout_ptr_0) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0) \
                       : "0"(level), "S"(*inout_ptr_0))

#define __cpuid(level, a, b, c, d) \
    __asm__ __volatile__("cpuid\n\t" \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \
                       : "0"(level))

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

void __marker_x86_64(void);
#endif

/**
 * Architecture-independent harness macros:
 *
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

/**
 * Called by SIMICS C stub to initialize the module, this is the entrypoint of the entire
 * module
 */
void module_init_local(void);

#endif /* TSFFS_H */
