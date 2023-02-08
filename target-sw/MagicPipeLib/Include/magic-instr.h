/*
  © 2016 Intel Corporation

  This software and the related documents are Intel copyrighted materials, and
  your use of them is governed by the express license under which they were
  provided to you ("License"). Unless the License provides otherwise, you may
  not use, modify, copy, publish, distribute, disclose or transmit this software
  or the related documents without Intel's prior written permission.

  This software and the related documents are provided as is, with no express or
  implied warranties, other than those that are expressly stated in the License.
*/

#ifndef MAGIC_INSTR_H
#define MAGIC_INSTR_H

#if !defined __GNUC__ && !defined __INTEL_COMPILER
#error "Unsupported compiler"
#endif

#define __MAGIC_CASSERT(p) typedef int __check_magic_argument[(p) ? 1 : -1] \
        __attribute__((unused));

#if defined __i386 || defined __x86_64__ || defined _M_IX86 || defined _M_AMD64
 #if defined __i386 && defined __PIC__

/* save ebx manually since it is used as PIC register */
#define MAGIC_ASM(n,p)                                  \
do {                                                    \
        void *dummy_a, *dummy_b;                        \
        __MAGIC_CASSERT((unsigned)(n) < 0x10000);       \
        __asm__ __volatile__ (                          \
                "push %%ebx; cpuid; pop %%ebx"          \
                : "=a" (dummy_a), "=b" (dummy_b)        \
                : "a" (0x4711 | ((unsigned)(n) << 16)), \
                  "b" (p) : "memory", "ecx", "edx");    \
} while (0)

 #else /* !defined __i386 || !defined __PIC__ */

#define MAGIC_ASM(n,p)                                                  \
do {                                                                    \
        void *dummy_a, *dummy_b;                                        \
        __MAGIC_CASSERT((unsigned)(n) < 0x10000);                       \
        __asm__ __volatile__ ("cpuid"                                   \
                              : "=a" (dummy_a), "=b" (dummy_b)          \
                              : "a" (0x4711 | ((unsigned)(n) << 16)),   \
                                "b" (p) : "memory", "rcx", "rdx");      \
} while (0)

 #endif /* defined __i386 && defined __PIC__ */
#elif defined __powerpc__ || defined __ppc
 #if defined __powerpc64__ || defined SIM_NEW_RLWIMI_MAGIC

#define MAGIC_ASM(n,p)                                                  \
        __MAGIC_CASSERT((n) >= 0 && (n) < (1 << 13));                   \
        __asm__ __volatile__ ("mr 14,%3; rlwimi %0,%0,0,%1,%2"          \
                              :: "i" (((n) >> 8) & 0x1f),               \
                                 "i" (((n) >> 4) & 0xf),                \
                                 "i" ((((n) >> 0) & 0xf) | 0x10),       \
                                 "r" (p) : "r14", "memory")

 #else /* !__powerpc64__ && !SIM_NEW_RLWIMI_MAGIC */

#define MAGIC_ASM(n,p)                                          \
        __MAGIC_CASSERT((n) >= 0 && (n) < (1 << 15));           \
        __asm__ __volatile__ ("mr 14,%3; rlwimi %0,%0,0,%1,%2"  \
                              :: "i" (((n) >> 10) & 0x1f),      \
                                 "i" (((n) >>  5) & 0x1f),      \
                                 "i" (((n) >>  0) & 0x1f),      \
                                 "r" (p) : "r14", "memory")

 #endif /* __powerpc64__ && !SIM_NEW_RLWIMI_MAGIC */
#elif defined __aarch64__

#define MAGIC_ASM(n,p)                                                  \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 31);                         \
        __asm__ __volatile__ ("mov x12, %0; orr x" #n ", x" #n ", x" #n \
                              :: "r" (p) : "x12", "memory")

#elif defined __arm__
 #ifdef __thumb__

#define MAGIC_ASM(n,p)                                                      \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 12);                             \
        __asm__ __volatile__ ("mov.w r12, %0; orr.w r" #n ", r" #n ", r" #n \
                              :: "r" (p) : "r12", "memory")

 #else /* !__thumb__ */

#define MAGIC_ASM(n,p)                                                  \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 14);                         \
        __asm__ __volatile__ ("mov r12, %0; orr r" #n ", r" #n ", r" #n \
                              :: "r" (p) : "r12", "memory")

 #endif /* __thumb__ */
#elif defined __mips__

#define MAGIC_ASM(n,p)                                                  \
	__MAGIC_CASSERT((n) >= 0 && (n) <= 0xffff);                     \
        __asm__ __volatile__ ("move $8,%0; li $zero," #n                \
                              :: "r" (p) : "$8", "memory")

#elif defined __arc__

#define MAGIC_ASM(n,p)                                          \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 0x3e);               \
        __asm__ __volatile__ ("mov r12, %1\n\r"                 \
                              "mov 0, %0"                       \
                              :: "L" (n + 1), "r" (p)           \
                              : "r12", "memory")

#elif defined __riscv

#define MAGIC_ASM(n,p)                                                  \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 31);                         \
        __asm__ __volatile__ ("mv a0, %0               \n\r"            \
                              "srai zero, zero, " #n                    \
                              :: "r" (p) : "a0", "memory")

#else
#error "Unsupported architecture"
#endif

#endif /* MAGIC_INSTR_H */
