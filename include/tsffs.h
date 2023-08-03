#ifndef TSFFS_H
#define TSFFS_H

#include <stdint.h>

#if defined(__x86_64__)
#define MAGIC 18193
#endif

#define MAGIC_START 1

#define MAGIC_STOP 2

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

#if defined(__x86_64__)
/**
 * X86_64:
 *
 */

#define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1) \
  __asm__ __volatile__("cpuid\n\t"                                     \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d),           \
                         "=S"(*inout_ptr_0), "=D"(*inout_ptr_1)        \
                       : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1))

#define __cpuid_extended1(level, a, b, c, d, inout_ptr_0)    \
  __asm__ __volatile__("cpuid\n\t"                           \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \
                         "=S"(*inout_ptr_0)                  \
                       : "0"(level), "S"(*inout_ptr_0))

#define __cpuid(level, a, b, c, d)                          \
  __asm__ __volatile__("cpuid\n\t"                          \
                       : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \
                       : "0"(level))

#define __arch_harness_start(addr_ptr, size_ptr)                 \
  do {                                                           \
    uint32_t _a __attribute__((unused)) = 0;                     \
    uint32_t _b __attribute__((unused)) = 0;                     \
    uint32_t _c __attribute__((unused)) = 0;                     \
    uint32_t _d __attribute__((unused)) = 0;                     \
    uint32_t leaf = (MAGIC_START << 16U) | MAGIC;                \
    __cpuid_extended2(leaf, _a, _b, _c, _d, addr_ptr, size_ptr); \
  } while (0)

#define __arch_harness_stop()                    \
  do {                                           \
    uint32_t _a __attribute__((unused)) = 0;     \
    uint32_t _b __attribute__((unused)) = 0;     \
    uint32_t _c __attribute__((unused)) = 0;     \
    uint32_t _d __attribute__((unused)) = 0;     \
    uint32_t leaf = (MAGIC_STOP << 16U) | MAGIC; \
    __cpuid(leaf, _a, _b, _c, _d);               \
  } while (0)

#define __arch_harness_stop_extended(val_ptr)         \
  do {                                                \
    uint32_t _a __attribute__((unused)) = 0;          \
    uint32_t _b __attribute__((unused)) = 0;          \
    uint32_t _c __attribute__((unused)) = 0;          \
    uint32_t _d __attribute__((unused)) = 0;          \
    uint32_t leaf = (MAGIC_STOP << 16U) | MAGIC;      \
    __cpuid_extended1(leaf, _a, _b, _c, _d, val_ptr); \
  } while (0)

void __marker_x86_64(void);
#endif

/**
 * Architecture-independent harness macros:
 *
 *
 */

#define HARNESS_START(addr_ptr, size_ptr)     \
  do {                                        \
    __arch_harness_start(addr_ptr, size_ptr); \
  } while (0)

#define HARNESS_STOP()     \
  do {                     \
    __arch_harness_stop(); \
  } while (0)

#define HARNESS_STOP_EXTENDED(val_ptr)     \
  do {                                     \
    __arch_harness_stop_extended(val_ptr); \
  } while (0)

void __marker(void);

#endif /* TSFFS_H */
