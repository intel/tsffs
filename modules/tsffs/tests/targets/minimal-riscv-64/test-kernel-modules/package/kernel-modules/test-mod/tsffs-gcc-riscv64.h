#ifndef TSFFS_H
#define TSFFS_H

/// TSFFS Magic Include

// GCC EASM notes:
// - The stack pointer is required to be the same on exit to an asm block as it
// was on entry

#define MAGIC_START (0x0001U)
#define MAGIC_STOP (0x0002U)

#define __srai_extended(value, testcase_ptr, size_ptr)                \
  __asm__ __volatile__("mv a0, %0; mv a1, %1; srai zero, zero, %2"    \
                       :                                              \
                       : "r"(testcase_ptr), "r"(size_ptr), "I"(value) \
                       : "a0", "a1");

#define __srai(value) \
  __asm__ __volatile__("srai zero, zero, %0" : : "I"(value) :)

#define __arch_harness_start(start, testcase_ptr, size_ptr) \
  do {                                                      \
    __srai_extended(start, testcase_ptr, size_ptr);         \
  } while (0)

#define __arch_harness_stop(stop) \
  do {                            \
    __srai(stop);                 \
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