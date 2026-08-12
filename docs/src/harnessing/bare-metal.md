# Bare-Metal and Non-x86 Compiled-In Harnessing

- [Bare-Metal and Non-x86 Compiled-In Harnessing](#bare-metal-and-non-x86-compiled-in-harnessing)
  - [Loading a Binary Directly Onto a CPU Model](#loading-a-binary-directly-onto-a-cpu-model)
  - [Providing a Minimal Runtime](#providing-a-minimal-runtime)
  - [Cross-Compiler Codegen Gotchas](#cross-compiler-codegen-gotchas)
  - [Checking Which ARM Exceptions to Configure](#checking-which-arm-exceptions-to-configure)

This page covers a specific but common case: harnessing a small piece of C
code (a single parser or decoder function, for example) that is normally
part of a much larger piece of software (an RTOS image, a full Linux
userspace daemon, a bootloader) which would be slow or inconvenient to boot
just to reach it. Rather than booting that larger system, this approach
compiles the function under test, plus a small custom entry point, into a
minimal freestanding binary that is loaded directly onto a CPU model and run
with no OS underneath it at all.

This is a variant of [compiled-in harnessing](compiled-in.md): the harness
macros and their behavior are unchanged, the difference is entirely in how
the harnessed binary gets onto the target and what runtime support it has
available.

## Loading a Binary Directly Onto a CPU Model

Instead of the CPU model going through its normal reset/firmware-load path,
the harnessed binary can be loaded directly into memory and the CPU's
program counter pointed at it. On many Simics CPU models, this looks like:

```simics
stop

# If the CPU model's own boot/reset sequencing left a pending exception
# queued (common for models that come out of a board-level reset
# component), step once to retire it before overriding PC below - otherwise
# the first instruction at the injected entry point can be swallowed by the
# stale exception instead of executing.
<cpu>.force-step-instruction 1

$entry = (<cpu>.load-binary "/path/to/harness.elf")
<cpu>.set-pc $entry

run
```

`load-binary` reads the entry point and segment load addresses from the
ELF header, so as long as the binary's linker script places it at a valid,
mapped address for the target (e.g. the base of on-chip RAM), no other setup
is required. This works for any CPU architecture Simics models, not just
the ones with dedicated compiled-in harness headers in `harness/`: write a
small assembly entry stub for the target's calling convention and reset
behavior, and the same technique applies.

## Providing a Minimal Runtime

A freestanding binary has no OS underneath it, so anything the harnessed
code depends on from libc must be supplied by hand. Before writing a custom
runtime, check what's actually needed: compiling the target function with
`-ffreestanding -fno-builtin` and inspecting undefined symbols in the
resulting object file (`nm` on Linux, or the equivalent for other
toolchains) is a fast way to find out. Parsers and decoders in particular
often only need a handful of the smallest libc functions:

* `memcpy` / `memcmp` / `memset`: trivial byte-loop implementations are
  sufficient; there's no need to reach for an optimized libc implementation
  in a fuzzing harness.
* `__assert_fail`: needed if the target code uses `assert()`. A minimal
  implementation can call `HARNESS_ASSERT()` and then loop forever (the
  fuzzer will restore a snapshot from `HARNESS_START` before the loop is
  ever actually reached at runtime).
* Any target-specific weak symbols the code calls for real hardware
  interaction (checksum validation, hardware-specific timing, etc.) that
  are irrelevant to the logic under test: override them with trivial
  stubs, the same way a normal unit test would mock them out.

A minimal entry point then looks like:

```c
#include "tsffs.h"

static uint8_t testcase[MAX_TESTCASE_SIZE];
static size_t testcase_size;

void _start(void) {
    for (;;) {
        testcase_size = sizeof(testcase);
        HARNESS_START(testcase, &testcase_size);

        function_under_test(testcase, testcase_size);

        HARNESS_STOP();
    }
}
```

linked with a linker script that places `.text`/`.data`/`.bss` at the
target's RAM base and reserves a small stack, and a short assembly stub
(`_reset`) that sets the stack pointer before branching to `_start`. The
CPU model's reset/entry conventions determine exactly what this stub needs
to do, but on most architectures it is only a few instructions.

## Cross-Compiler Codegen Gotchas

Freestanding code compiled at low optimization levels can still trigger
codegen a target CPU model doesn't support, in ways that have nothing to do
with the actual logic under test. A notable case: cross-compilers may
default to a hard-float ABI, which permits emitting vector/FPU instructions
(e.g. ARM NEON) for plain operations like zero-initializing a struct
(`struct foo x = {0};`), even in code that never touches floating-point
data. If the target CPU model doesn't implement those instructions, this
manifests as an Undefined Instruction exception on *every single* fuzzing
iteration, which looks identical to a genuinely broken harness. Check the
exact instruction address a crash actually happens at (a disassembly of the
harness binary makes this fast) before assuming a fuzzer, harness, or model
configuration problem.

For GCC ARM32 targets, disabling autovectorization is usually enough to
avoid this without otherwise changing codegen:

```sh
arm-linux-gnueabihf-gcc -ffreestanding -fno-builtin \
    -fno-tree-vectorize -fno-tree-slp-vectorize \
    ...
```

## Checking Which ARM Exceptions to Configure

`@tsffs.exceptions` takes CPU exception numbers, which are
architecture-specific. For ARM cores, query the exact numbers from the CPU
model itself rather than assuming values from another architecture or
another ARM core class:

```simics
simics> <cpu>.list-exceptions
```

The Data Abort, Prefetch Abort, and Undefined Instruction exceptions are
the rough ARM equivalents of x86's Page Fault and General Protection Fault,
and are a reasonable starting set for most memory-safety bugs:

```python
@tsffs.exceptions = [<data-abort>, <prefetch-abort>, <undefined-instruction>]
```
