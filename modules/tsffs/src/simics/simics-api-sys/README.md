# SIMICS API SYS

This crate provides low-level (direct) bindings to the SIMICS API generated with
`bindgen`. This crate shouldn't be used directly, instead you should use the
`simics-api` crate.

## Using This Crate

This crate's bindings are versioned by the Simics Base package version they were
generated from. This means that without any feature flags, this crate does nothing.

To use it in your crate, you should write, for example:

```toml
simics-api-sys = { version = "0.1.0", features = ["6.0.169"] }
```

## Updating Bindings

When a new Simics Base version is released, or you want to add older bindings, you
should edit the `SIMICS_BASE_VERSIONS` list in
[update-bindings.rs](./scripts/update-bindings.rs), then run `cargo make
update-bindings` to update the bindings. If you don't have `cargo make`, you can install
it with `cargo install cargo-make`

## Updating ISPM

When a new ISPM version is released, you should be able to continue to use this crate's
`update-bindings` automatically, but if a newer Simics Base version requires a newer
version of ISPM for some reason, you can update the ISPM tarball in the [script
resource](./scripts/resource/) directory, then update the `ispm_file` argument in the
`update-bindings.rs` script `main`.
