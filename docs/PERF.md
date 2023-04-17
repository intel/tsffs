# Performance!

Fuzzers should go fast, so let's make it go fast!

## Release Profile

Run your fuzzer with `cargo run --release --bin your-fuzzer` to get a free ~2x speedup.

## Lower Log Level

Logging is super expensive, set up your `Fuzzer` with:

```rust
Fuzzer::try_new(_, _, _, Level::Error)?;
```

to disable any non-error logging from the fuzzer. In addition, set the log level of your
driver program to the lowest level possible (logging in the fuzz harness especially
can slow things down significantly).