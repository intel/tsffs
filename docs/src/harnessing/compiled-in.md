# Compiled-In Harnessing

- [Compiled-In Harnessing](#compiled-in-harnessing)
  - [Using Provided Headers](#using-provided-headers)
  - [Multiple Harnesses in One Binary](#multiple-harnesses-in-one-binary)
  - [Alternative Start Harnesses](#alternative-start-harnesses)
  - [Troubleshooting](#troubleshooting)
    - [Compile Errors About Temporaries](#compile-errors-about-temporaries)

## Using Provided Headers

The TSFFS project provides harnessing headers for each supported combination of
architecture and build toolchain. These headers can be found in the `harness` directory
in the repository. There is also a monolithic header `tsffs.h` which conditionally
compiles to whichever architecture is in use and can be used on any supported
architecture and platform.

Each header provides the macros `HARNESS_START` and `HARNESS_STOP`.

`HARNESS_START(testcase_ptr, size_ptr)` takes two arguments, a buffer for the fuzzer to
write testcases into each fuzzing iteration and a pointer to a pointer-sized variable,
which the fuzzer will write the size of each testcase to each fuzzing iteration.  The
variable pointed to by `size_pointer` should be initially equal to the maximum size of a
testcase, typically the size of the buffer passed as the first argument.

`HARNESS_STOP` takes no arguments.

For example, the following code will invoke the start and stop harnesses correctly:

```c
#include "tsffs.h"

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
    int retval = function_under_test(buffer, size);

    if (retval == SOMETHING_IMPOSSIBLE_HAPPENED) {
        /// Some exceptional condition occurred -- note, don't use this for normal "bad" return
        /// values, use it for instances where something that you are fuzzing for happened.
        HARNESS_ASSERT();
    }
    
    // Stop normally
    HARNESS_STOP();
    return 0;
}
```

By default, TSFFS is enabled to use these harnesses, so no explicit configuration is
necessary. However, the defaults are equivalent to the configuration:

```python
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
@tsffs.magic_start_index = 0
@tsffs.magic_stop_indices = [0]
@tsffs.magic_assert_indices = [0]
```

This sets TSFFS to start the fuzzing loop on a *magic*
harness with magic number `1` (used by `HARNESS_START`)
and index `0` (the default) and stop execution and
restore to the initial snapshot on *magic* harnesses
with magic number `2` (used by `HARNESS_STOP`) and
index `0` (the default).

## Multiple Harnesses in One Binary

If multiple fuzzing campaigns will be run on the same target software, it is
sometimes advantageous to compile multiple harnesses into the same target
software ahead of time, and choose which to enable at runtime.Each provided
header also provides two lower-level macros for this purpose.

* `HARNESS_START_INDEX(index, testcase_ptr, size_ptr)`
* `HARNESS_STOP(index)`

These macros are used in the same way as `HARNESS_START` and `HARNESS_STOP`,
with the additional first argument. The default value of `index` is 0, but
TSFFS can be configured to treat a different index as the trigger to start or
stop the fuzzing loop.

```c
#include "tsffs.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    HARNESS_START(buf, &size);

    if (size < 3) {
        // Stop early if there is not enough data
        HARNESS_STOP();
    }

    char * result = function_under_test(buf);

    // Stop normally on success
    HARNESS_STOP();

    HARNESS_START_INDEX(1, result, &size);

    second_function_under_test(result);

    HARNESS_STOP();

    return 0;
}
```

And configuration settings like:


```python
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
@tsffs.magic_start_index = 1
```

With this runtime configuration, the first harness will be ignored, and only the second
set of harness calls will be used.

## Alternative Start Harnesses

Several additional variants of the start harness are provided to allow
different target software to be used with as little modification as possible.

* `HARNESS_START_WITH_MAXIMUM_SIZE(void *buffer, size_t max_size)` takes a
  pointer to a buffer like `HARNESS_START` but takes a size instead of a
  pointer to a size as the second argument. Use this harness when the target
  software does not need to read the actual buffer size.
* `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(void *buffer, void *size_ptr, size_t max_size)`
  takes a pointer to both a buffer and size like `HARNESS_START`, and takes a
  size as the third argument. Use this harness when the target software does
  not initially have `*size_ptr` set to the maximum size, but still needs to
  read the actual buffer size.

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
