# Closed Box Harnessing

- [Closed Box Harnessing](#closed-box-harnessing)
  - [Disabling Compiled-in/Magic Harnesses](#disabling-compiled-inmagic-harnesses)
  - [Triggering Manual Stops/Solutions](#triggering-manual-stopssolutions)

## Disabling Compiled-in/Magic Harnesses

Magic start and stop behavior can be disabled, which allows harnessing target software
without compiled-in harness code. However, implementation becomes highly target-specific
and the magic harness approach is highly preferred.

The same code as before, with no harness:


```c
#include "tsffs.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    function_under_test(buf);

    return 0;
}
```

Can be harnessed in a closed-box fashion by creating a script-branch to wait until the
simulation reaches a specified address, timeout, HAP, or any other condition. First, we
can disable magic harnesses (this is not strictly necessary unless any magic harnesses
actually exist in the target software, but it is good practice).

```python
@tsffs.start_on_harness = False
@tsffs.stop_on_harness = False
```

Once compiled-in harnesses are disabled, the fuzzing loop can be started manually. There
are two APIs available. Both receive a `cpu` object as their first argument, which
should be a processor instance, for example on the QSP platform
`qsp.mb.cpu0.core[0][0]`. This is the processor whose associated memory will be written
with new testcases, and whose address space will be used for virtual address
translation.

Both testcases also take a `virt: bool` as their final argument, to specify whether the
addresses passed are virtual addresses or physical addresses. If `False`, the addresses
will *not* be translated, which can allow circumventing issues where the address is
accessible, but the page table does not contain an identity mapping for it. A physical
address that is identity mapped may be passed with either a `True` or `False` value of
`virt`.

The first API takes two memory addresses, and is equivalent to the [compiled in
`HARNESS_START`](compiled-in.md#using-provided-headers) macro. When called, the fuzzer
will save the passed-in addresses (which may be virtual or physical), read the
pointer-sized integer at `*size_address` as the maximum size of testcases. take a
snapshot, and begin the fuzzing loop. Each iteration, a testcase will be written to the
testcase pointer, and the size of the testcase will be written to the provided size
pointer. This API is called like:

```python
@tsffs.iface.fuzz.start(cpu, testcase_address, size_address, True)
```

The second API takes one memory address and a maximum size. Testcases will be written
to the provided testcase address, and will be truncated to the provided maximum size.
This API is called like:

```python
@tsffs.iface.fuzz.start_with_maximum_size(cpu, testcase_address, maximum_size, True)
```

## Triggering Manual Stops/Solutions

During manual or harnessed fuzzer execution, a normal stop or solution can be specified
at any time using the API. This allows arbitrary conditions for stopping execution of
a test case, or treating an execution as a solution, by programming via the SIMICS
script or SIMICS Python script.

During execution, the fuzzer can be signaled to stop the current testcase execution with
a normal exit (i.e. *not* a solution), and reset to the initial snapshot with a new
testcase with:

```python
@tsffs.iface.fuzz.stop()
```

Likewise, the fuzzer can be signaled to stop the current testcase execution with a
solution. The fuzzer will save the input for this execution to the solutions directory
(see [that section](../config/common-options.md#set-corpus-and-solutions-directory)).
The `solution` method takes an ID and message that will be saved along with this
solution for later use. Any id and message can be provided, it is entirely up to the
user:

```python
@tsffs.iface.fuzz.solution(1, "A descriptive message about why this is a solution condition")
```
