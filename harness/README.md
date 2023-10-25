# Harness Collection for TSFFS

This directory contains a selection of provided (and tested) harness header files. All
harnesses are tested automatically at project test time for correct passing of the
testcase and size addresses.

All headers define the following:

* `MAGIC_START` - The value used by default to signal the fuzzer to start
* `MAGIC_STOP` - The value used by default to signal the fuzzer to stop
* `HARNESS_START(uint8_t **addr_ptr, size_t * size_ptr)` - The macro used to signal the
  fuzzer to start fuzzing, writing each testcase to the buffer pointed to by `addr_ptr`
  and writing the size of each testcase to `size_ptr`, where `*size_ptr` is initially
  equal to the maximum testcase size (i.e. the size of `*addr_ptr`).
* `HARNESS_STOP()` - The macro used to signal the fuzzer to stop the current execution,
  restore the snapshot taken at the location of `HARNESS_START`, and start another
  execution with a new testcase.

Some architectures or programming environments require an assembly file in addition to
the provided header file. Notably, MSVC does not support intrinsics when compiling
edk2-based UEFI code, and does not support inline assembly, so assembly files are
necessary.

| Architecture | Compiler | Programming Environment | Header/Support File(s)                             |
| ------------ | -------- | ----------------------- | -------------------------------------------------- |
| x86_64       | gcc      | Generic (non-edk2)      | [tsffs-gcc-x86_64.h](tsffs-gcc-x86_64.h)           |
| x86          | gcc      | Generic (non-edk2)      | [tsffs-gcc-x86.h](tsffs-gcc-x86.h)                 |
| x86_64       | gcc      | edk2                    | [tsffs-gcc-x86_64-edk2.h](tsffs-gcc-x86_64-edk2.h) |
| x86          | gcc      | edk2                    | [tsffs-gcc-x86-edk2.h](tsffs-gcc-x86-edk2.h)       |