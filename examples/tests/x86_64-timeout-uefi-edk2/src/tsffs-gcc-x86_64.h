#ifndef TSFFS_H
#define TSFFS_H

/// TSFFS Magic Include

// GCC EASM notes:
// - The stack pointer is required to be the same on exit to an asm block as it
// was on entry

#define MAGIC (0x4711U)
#define MAGIC_START (0x0001U)
#define MAGIC_STOP (0x0002U)

/// Trigger a CPUID instruction with RSI and RDI set to specific values.
#define __cpuid_extended2(value, inout_ptr_0, inout_ptr_1)              \
  unsigned int _a __attribute__((unused)) = 0;                          \
  __asm__ __volatile__("cpuid"                                          \
                       : "=a"(_a)                                       \
                       : "a"(value), "D"(inout_ptr_0), "S"(inout_ptr_1) \
                       : "rbx", "rcx", "rdx");

/// Trigger a CPUID instruction
#define __cpuid(value)                         \
  unsigned int _a __attribute__((unused)) = 0; \
  __asm__ __volatile__("cpuid\n\t"             \
                       : "=a"(_a)              \
                       : "a"(value)            \
                       : "rbx", "rcx", "rdx")

#define __arch_harness_start(start, testcase_ptr, size_ptr) \
  do {                                                      \
    unsigned int magic = (start << 0x10U) | MAGIC;          \
    __cpuid_extended2(magic, testcase_ptr, size_ptr);       \
  } while (0)

#define __arch_harness_stop(stop)                 \
  do {                                            \
    unsigned int magic = (stop << 0x10U) | MAGIC; \
    __cpuid(magic);                               \
  } while (0)

#define HARNESS_START(testcase_ptr, size_ptr)                  \
  do {                                                         \
    __arch_harness_start(MAGIC_START, testcase_ptr, size_ptr); \
  } while (0)

#define HARNESS_STOP()               \
  do {                               \
    __arch_harness_stop(MAGIC_STOP); \
  } while (0)

#endif  // TSFFS_H