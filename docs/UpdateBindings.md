# Updating Bindings

Unless you are a developer of the fuzzer, you should't need to go through this
procedure. You are welcome to, however, if you beat the maintainers on the draw for new
SIMICS version releases.

To update bindings, first run `update-bindings.rs` according to the
[README](../simics-api-sys/scripts/README.md).

Next, add and/or update the version for SIMICS base (package 1000), QSP, and QSP-X86
in all locations they occur. `rg` or `grep` is helpful.