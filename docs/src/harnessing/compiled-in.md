# Compiled-In Harnessing

- [Compiled-In Harnessing](#compiled-in-harnessing)
  - [Using Provided Headers](#using-provided-headers)
  - [Multiple Harnesses in One Binary](#multiple-harnesses-in-one-binary)
  - [Troubleshooting](#troubleshooting)
    - [Compile Errors About Temporaries](#compile-errors-about-temporaries)

## Using Provided Headers

The TSFFS project provides harnessing headers for each supported combination of
architecture and build toolchain. These headers can be found in the `harness` directory
in the repository.

Each header provides the macros `HARNESS_START` and `HARNESS_STOP`.


`HARNESS_START(testcase_ptr, size_ptr)` takes two arguments, a buffer for the fuzzer to
write testcases into each fuzzing iteration and a pointer to a pointer-sized variable,
which the fuzzer will write the size of each testcase to each fuzzing iteration.  The
variable pointed to by `size_pointer` should be initially equal to the maximum size of a
testcase, typically the size of the buffer passed as the first argument.

`HARNESS_STOP` takes no arguments.

For example, the following code will invoke the start and stop harnesses correctly:

```c
#include "tsffs-gcc-x86_64.h"

int main() {
    char buffer[20];
    size_t size = sizeof(buffer);

    // Each fuzzer iteration, execution will start from here, with a different buffer content
    // and size=len(buffer).
    HARNESS_START(buffer, &size);

    // Check if we got enough data from the fuzzer -- this is not always necessary
    if (size < 3) {
        // Stop early if we didn't get enough bytes from the fuzzer
        HARNESS_STOP();
    }

    // Do something with buffer and size
    function_under_test(buffer, size);
    
    // Stop normally
    HARNESS_STOP();
    return 0;
}
```

By default, TSFFS is enabled to use these harnesses, so no explicit configuration is
necessary. However, the defaults are equivalent to the configuration:

```python
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_start_magic_number(1)
tsffs.iface.tsffs.set_start_magic_number(2)
```

This sets TSFFS to start the fuzzing loop on a *magic* harness with magic number `1`
(used by `HARNESS_START`) and stop execution and restore to the initial snapshot on
*magic* harnesses with magic number `2` (used by `HARNESS_STOP`).

## Multiple Harnesses in One Binary

If multiple fuzzing campaigns will be run on the same target software, it is sometimes
advantageous to compile multiple harnesses into the same target software ahead of time,
and choose which to enable at runtime.Each provided header also provides two lower-level
macros for this purpose.

* `__arch_harness_start(start, testcase_ptr, size_ptr)`
* `__arch_harness_stop(stop)`

These macros are used in the same way as `HARNESS_START` and `HARNESS_STOP`, with the
additional first argument. The default value of `start` is 1, and the default value of
`stop` is 2, but TSFFS can be configured to treat a different value as the trigger to
start or stop the fuzzing loop. Note that `start` and `stop` must be at least 1 and at
most 11, so it is possible to create a target software with up to 10 different harnesses
(by using magic values `1`, `3-11` as start values and `2` as the stop value). This is a
limitation of the instructions SIMICS understands as *magic*, some of which only support
an immediate `0<=n<=12` (with magic numbers 0 and 12 *being reserved by SIMICS).

```c
#include "tsffs-gcc-x86_64.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    __arch_harness_start(1, buf, &size);

    if (size < 3) {
        // Stop early if there is not enough data
        HARNESS_STOP();
    }

    char * result = function_under_test(buf);

    // Stop normally on success
    HARNESS_STOP();

    __arch_harness_start(3, result, &size);

    second_function_under_test(result);

    HARNESS_STOP();

    return 0;
}
```

And configuration settings like:


```python
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_start_magic_number(3)
tsffs.iface.tsffs.set_start_magic_number(4)
```

With this runtime configuration, the first harness will be ignored, and only the second
set of harness calls will be used.


## Troubleshooting

### Compile Errors About Temporaries

Some toolchains or compiler versions may reject the use of an `&` reference in the
`HARNESS_START` macro (like `HARNESS_START(buffer, &size)`). In this case, create a
temporary to hold the address and pass it instead, like:

```c
char buffer[100];
size_t size = sizeof(buffer);
size_t size_ptr = &size;
HARNESS_START(buffer, size_ptr);
```