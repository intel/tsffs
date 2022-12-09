/*
  © 2010 Intel Corporation

  This software and the related documents are Intel copyrighted materials, and
  your use of them is governed by the express license under which they were
  provided to you ("License"). Unless the License provides otherwise, you may
  not use, modify, copy, publish, distribute, disclose or transmit this software
  or the related documents without Intel's prior written permission.

  This software and the related documents are provided as is, with no express or
  implied warranties, other than those that are expressly stated in the License.
*/

#ifndef SIMICS_MAGIC_INSTRUCTION_H
#define SIMICS_MAGIC_INSTRUCTION_H

/*
 * This file contains the magic instructions for different target
 * architectures, as understood by different compilers.
 *
 *   arch   instr           limit               compilers
 *   ----------------------------------------------------
 *   arc    mov 0,n         1 <= n <= 0x3f      gcc
 *   arm    orr rn,rn,rn    0 <= n <= 14        gcc
 *   armv8  orr xn,xn,xn    0 <= n <= 31        gcc
 *                          triggered regardless of condition
 *   thumb2 orr.w rn,rn,rn  0 <= n <= 12        gcc (no thumb1 magic exist)
 *                          triggered regardless of condition
 *   h8300  brn n           -128 <= n <= 127    gcc
 *   m68k   dbt dx,y        0 <= n <= 0x3ffff   gcc
 *                           x = n[17:15]
 *                           y = n[14:0] << 1
 *   mips   li $zero,n      0 <= n <= 0xffff    gcc
 *   nios   or rN,rN,rN     0 <= N <= 31        gcc
 *   ppc    rlwimi x,x,0,y,z 0 <= n <= 0x1fff   gcc, Diab
 *                           x = n[12:8]
 *                           y = n[7:4]
 *                           z = n[3:0] | 16
 *   ppc    rlwimi x,x,0,y,z 0 <= n <= 0x7fff   gcc, Diab       old rlwimi
 *                           x = n[14:10]
 *                           y = n[9:5]
 *                           z = n[4:0]
 *   riscv  srai zero, zero, N
 *                           0 <= N <= 31 in rv32
 *                           0 <= N <= 63 in rv64
 *   sh     mov rn,rn       0 <= rn < 16        gcc
 *   sparc  sethi n,%g0     0 <  n < (1 << 22)  gcc, WS C[++]
 *   x86    cpuid           0 <= n < 0x10000    gcc, icc
 *                          (eax == 0x4711 + (n << 16))
 *
 *   Reserved values:
 *                          0
 *                         12
 */

/*
  <add id="magic instruction figure">
  <figure label="magic_instruction_figure">
  <center>
  <table border="cross">
    <tr>
       <td><b>Target</b></td>
       <td><b>Magic instruction</b></td>
       <td><b>Conditions on <arg>n</arg></b></td>
       <td/>
    </tr>
    <tr>
       <td>ARC</td><td><tt>mov 0, n</tt></td>
                   <td><math>1 ≤ n &lt; 64</math></td>
                   <td/>
    </tr>
    <tr>
       <td>ARM</td><td><tt>orr rn, rn, rn</tt></td>
                   <td><math>0 ≤ n ≤ 14</math></td>
                   <td/>
    </tr>
    <tr>
       <td>ARMv8</td><td><tt>orr xn, xn, xn</tt></td>
                   <td><math>0 ≤ n ≤ 31</math></td>
                   <td/>
    </tr>
    <tr>
       <td>ARM Thumb-2</td><td><tt>orr.w rn, rn, rn</tt></td>
                           <td><math>0 ≤ n ≤ 12</math></td>
                           <td/>
    </tr>
    <tr>
       <td>H8300</td><td><tt>brn n</tt></td>
                     <td><math>-128 ≤ n ≤ 127</math></td>
                     <td/>
    </tr>
    <tr>
       <td>M680x0</td><td><tt>dbt dx,y</tt></td>
                    <td><math>0 ≤ n &lt; 0x3ffff</math></td>
                    <td/>
    </tr>
    <tr>
      <td/>
      <td><math>x=n[17:15], y=n[14:0] * 2</math></td>
      <td/>
      <td/>
    </tr>
    <tr>
       <td>MIPS</td><td><tt>li %zero, n</tt></td>
                    <td><math>0 ≤ n &lt; 0x10000</math></td>
                    <td/>
    </tr>
    <tr>
       <td>Nios II</td><td><tt>or rN, rN, rN,</tt></td>
                    <td><math>0 ≤ N &lt; 32</math></td>
                    <td/>
    </tr>
    <tr>
      <td>PowerPC</td><td><tt>rlwimi x,x,0,y,z</tt></td>
      <td><math>0 ≤ n &lt; 8192</math></td>
      <td>new encoding</td>
    </tr>
    <tr>
      <td/>
      <td><math>x=n[12:8], y=n[7:4], z=n[3:0]|16</math></td>
      <td/>
      <td/>
    </tr>
    <tr>
      <td>PowerPC</td><td><tt>rlwimi x,x,0,y,z</tt></td>
      <td><math>0 ≤ n &lt; 32768</math></td>
      <td>old encoding</td>
    </tr>
    <tr>
      <td/>
      <td><math>x=n[14:10], y=n[9:5], z=n[4:0]</math></td>
      <td/>
      <td/>
    </tr>
    <tr>
       <td>RISC-V</td><td><tt>srai zero, zero, n</tt></td>
                   <td><math>0 ≤ n ≤ 31</math></td>
                   <td/>
    </tr>
    <tr>
       <td>SH</td><td><tt>mov rn, rn</tt></td>
                        <td><math>0 ≤ n &lt; 16</math></td>
                        <td/>
    </tr>
    <tr>
       <td>SPARC</td><td><tt>sethi n, %g0</tt></td>
               <td><math>1 ≤ n &lt; 0x400000</math></td>
               <td/>
    </tr>
    <tr>
      <td>x86</td>
      <td><tt>cpuid</tt></td>
      <td><math>0 ≤ n &lt; 0x10000</math></td>
      <td/>
    </tr>
    <tr>
      <td/>
      <td>with <tt>eax</tt> = <math>0x4711 + n * 2<sup>16</sup></math></td>
      <td/>
      <td/>
    </tr>
  </table>
  </center>
  <caption>Magic instructions for different Simics Targets</caption>
  </figure>
  </add>

  <add id="reserved magic numbers figure">
  <figure label="reserved_magic_numbers_figure">
  <center>
  <table border="cross">
    <tr><td><b>Reserved Magic Numbers</b></td></tr>
    <tr><td><tt>&nbsp;0</tt></td></tr>
    <tr><td><tt>12</tt></td></tr>
  </table>
  </center>
  <caption>Reserved magic numbers, for internal use only.</caption>
  </figure>
  </add>
*/

#ifdef __GNUC__
#define MAGIC_UNUSED __attribute__((unused))
#else
#define MAGIC_UNUSED
#endif

#define __MAGIC_CASSERT(p) do {                                         \
        typedef int __check_magic_argument[(p) ? 1 : -1] MAGIC_UNUSED;  \
} while (0)

#if defined __sparc || defined __sparc__

#if defined __GNUC__

#define MAGIC_INSTRUCTION(n) __asm__ __volatile__ ("sethi " #n ", %g0")

#else
#error "Unsupported compiler"
#endif

#define MAGIC(n) do {                                   \
	__MAGIC_CASSERT((n) > 0 && (n) < (1U << 22));   \
        MAGIC_INSTRUCTION(n);                           \
} while (0)
#define MAGIC_BREAKPOINT MAGIC(0x40000)

/* _M_ defines are used by ICC on Windows */
#elif defined __i386 || defined __x86_64__ \
   || defined _M_IX86 || defined _M_AMD64

#if defined __GNUC__ && defined __i386 && defined __PIC__

/* save ebx manually since it is used as PIC register */
#define MAGIC(n) do {                                                       \
        int simics_magic_instr_dummy;                                       \
        __MAGIC_CASSERT((unsigned)(n) < 0x10000);                           \
        __asm__ __volatile__ ("push %%ebx; cpuid; pop %%ebx"                \
                              : "=a" (simics_magic_instr_dummy)             \
                              : "a" (0x4711 | ((unsigned)(n) << 16))        \
                              : "ecx", "edx");                              \
} while (0)

#elif defined __GNUC__ || defined __INTEL_COMPILER

#define MAGIC(n) do {                                                       \
        int simics_magic_instr_dummy;                                       \
        __MAGIC_CASSERT((unsigned)(n) < 0x10000);                           \
        __asm__ __volatile__ ("cpuid"                                       \
                              : "=a" (simics_magic_instr_dummy)             \
                              : "a" (0x4711 | ((unsigned)(n) << 16))        \
                              : "ecx", "edx", "ebx");                       \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

/* Vanilla GCC uses __powerpc__, while Diab and GCC on Wind River Workbench
   use __ppc  */
#elif defined __powerpc__ || defined __ppc

#if defined __powerpc64__ || defined SIM_NEW_RLWIMI_MAGIC

#if defined __DCC__                     /* Diab compiler */

asm volatile void MAGIC_INSTRUCTION(int n)
{
%con n  /* n is a constant */
!       /* no scratch registers */
        rlwimi  (n >> 8) & 31, (n >> 8) & 31, 0, (n >> 4) & 15, (n & 15) | 16
}

#elif defined __GNUC__

#define MAGIC_INSTRUCTION(n)                                    \
       __asm__ __volatile__ ("rlwimi %0,%0,0,%1,%2"             \
                             :: "i" (((n) >> 8) & 0x1f),        \
                                "i" (((n) >> 4) & 0xf),         \
                                "i" ((((n) >> 0) & 0xf) | 16))

#else
#error "Unsupported compiler"
#endif

#define MAGIC(n) do {                                            \
        __MAGIC_CASSERT((n) >= 0 && (n) < (1 << 13));            \
        MAGIC_INSTRUCTION(n);                                    \
} while (0)

#else /* !__powerpc64__ && !SIM_NEW_RLWIMI_MAGIC */

#if defined __DCC__                     /* Diab compiler */

asm volatile void MAGIC_INSTRUCTION(int n)
{
%con n  /* n is a constant      */
!       /* no scratch registers */
        rlwimi  (n >> 10) & 31, (n >> 10) & 31, 0, (n >> 5) & 31, n & 31
}

#elif defined __GNUC__

#define MAGIC_INSTRUCTION(n)                                    \
       __asm__ __volatile__ ("rlwimi %0,%0,0,%1,%2"             \
                             :: "i" (((n) >> 10) & 0x1f),       \
                                "i" (((n) >>  5) & 0x1f),       \
                                "i" (((n) >>  0) & 0x1f))

#else
#error "Unsupported compiler"
#endif

#define MAGIC(n) do {                                            \
        __MAGIC_CASSERT((n) >= 0 && (n) < (1 << 15));            \
        MAGIC_INSTRUCTION(n);                                    \
} while (0)

#endif /* !__powerpc64__ && !SIM_NEW_RLWIMI_MAGIC */

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined (__aarch64__)
#ifdef __GNUC__
#define MAGIC(n) do {                                            \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 31);                  \
        __asm__ __volatile__ ("orr x" #n ", x" #n ", x" #n);     \
} while (0)
#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined(__arm__)

#ifdef __GNUC__

#ifdef __thumb__
/* Use .w suffix to force an error for Thumb-1 assemblers, instead of silently
   converting it to an orrs instruction with side-effects */
#define MAGIC(n) do {                                            \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 12);                  \
        __asm__ __volatile__ ("orr.w r" #n ", r" #n ", r" #n);   \
} while (0)
#else
#define MAGIC(n) do {                                            \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 14);                  \
        __asm__ __volatile__ ("orr r" #n ", r" #n ", r" #n);     \
} while (0)
#endif

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined(__mips__)

#ifdef __GNUC__

#define MAGIC(n) do {                                   \
	__MAGIC_CASSERT((n) >= 0 && (n) <= 0xffff);     \
        /* GAS refuses to do 'li $zero,n' */            \
        __asm__ __volatile__ (".word 0x24000000+" #n);	\
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __H8300__ || defined __H8300S__

#ifdef __GNUC__

#define MAGIC(n) do {                                   \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 0xff);       \
        __asm__ __volatile__(".byte 0x41\n" /* brn */   \
                             ".byte " #n);  /* disp */  \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __SPU__

#ifdef __GNUC__

#define MAGIC(n) do {                                     \
	__MAGIC_CASSERT((n) >= 0 && (n) < 128);           \
        __asm__ __volatile__ ("or " #n ", " #n ", " #n);  \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined(_SH1) || defined(_SH4) || defined(_SH4A)

#ifdef __GNUC__

#define MAGIC(n) do {                                \
        __MAGIC_CASSERT((n) >= 0 && (n) < 16);       \
        __asm__ __volatile__ ("mov r" #n ", r" #n);  \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __m68k__

#ifdef __GNUC__

#define MAGIC(n) do {                                                         \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 0x3ffff);                          \
        __asm__ __volatile__ (                                                \
           ".word 0x50c8 + ((" #n ") >> 15), ((" #n ") & 0x7fff) << 1");      \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __arc__

#ifdef __GNUC__

#define MAGIC(n) do {                                           \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 0x3e);               \
        __asm__ __volatile__ ("mov 0, %0" :: "L" (n + 1));      \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __nios2_arch__

#ifdef __GNUC__

#define MAGIC(n) do {                                      \
        __MAGIC_CASSERT((n) >= 0 && (n) <= 0x1f);          \
        __asm__ __volatile__ ("or r" #n ", r" #n ", r" #n);   \
} while (0)

#else
#error "Unsupported compiler"
#endif

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __XTENSA__

#define MAGIC(n)                                                              \
        do {                                                                  \
                __asm__ __volatile__ (                                        \
                        "mov.n a2,%0\n"                                       \
                        ".byte 0x00,0x51,0x00" : : "a" (n) : "%a2","memory"); \
        } while (0)

#define MAGIC_BREAKPOINT MAGIC(0)

#elif defined __riscv

#define MAGIC(n) do {                                               \
                __MAGIC_CASSERT((n) >= 0 && (n) <= 0x3f);           \
                __asm__ __volatile__ ("srai zero, zero," #n );      \
        } while (0)

#define MAGIC_BREAKPOINT MAGIC(0)

#else
#error "Unsupported architecture"
#endif

#endif
