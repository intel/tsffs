# Optimizing for Fuzzing

There are a few techniques that can be used to optimize the fuzzer for performance while
fuzzing.

## Reduce Output

The most effective (and, helpfully, often the easiest) way to improve performance of the
fuzzer is to eliminate as much output as possible from the target software. You can use
the preprocessor definition `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION` to do this:

Before:

```c
log_info("Some info about what's happening");
log_debug("Some even more info about what's happening, the value is %d", some_value);
```

After:

```c
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
    log_info("Some info about what's happening");
    log_debug("Some even more info about what's happening, the value is %d", some_value);
#endif
```

This will reduce the logging output, which is important in SIMICS as it reduces the running
of the console output model, which is much slower than the CPU model.

## Run as little as possible

In general, the harnesses for fuzzing should be placed as close around the code you
actually wish to fuzz as possible. For example, if you only want to fuzz a specific function,
like `YourSpecialDecoder`, place your harnesses immediately around the function call you
want to fuzz:

```c
HARNESS_START(buf, buf_size_ptr);
int retval = YourSpecialDecoder(certbuf, certbuf_size_ptr);

if (!retval) {
    /// An error occurred
    HARNESS_ASSERT();
} else {
    HARNESS_STOP();
}
```