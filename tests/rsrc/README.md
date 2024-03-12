# Tests

Examples that should be tested should be placed here. Each example should have a
`build.sh` script that generates or places all files required to run the test into the
test's directory. The `build.sh` script may accept a `SIMICS_BASE` environment
variable to locate simics-provided scripts.

For example, [minimal-riscv-64/build.sh](minimal-riscv-64/build.sh) produces:

- `fw_jump.elf`
- `Image`
- `rootfs.ext2`
- `test`
- `test-mod`
- `test-mod-userspace`
- `test-mod.ko`

All of which are used by the tests that use this example. The files should be output in
the same directory structure they should be placed into the SIMICS project set up for
testing.

Test scripts should be named following the pattern `test*.simics` and placed in the test
directory.

