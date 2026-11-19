# Tests

Targets used by the integration tests in `tests/*.rs` are placed here. Each target has a
`build.sh` script that generates or places all files required to run the tests into the
target's directory.

For example, [riscv-64/build.sh](riscv-64/build.sh) produces:

- `fw_jump.elf`
- `Image`
- `rootfs.ext2`
- `test`
- `test-mod`
- `test-mod-userspace`
- `test-mod.ko`

All of which are used by the tests that use this target. The files should be output in
the same directory structure they should be placed into the SIMICS project set up for
testing.

A test copies a target's directory into its SIMICS project with the `directories` option
of `TestEnvSpec`, and drives SIMICS with an inline script (see `tests/x86_64_magic_latest.rs`).
