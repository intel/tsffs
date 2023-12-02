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
  execution with a new testcase, without saving the input (no error or solution
  occurred).
* `HARNESS_ASSERT()` - The macro used to signal the fuzzer to stop the current
  execution, restore the snapshot taken at the location of `HARNESS_START`, and start
  another execution with a new testcase, while saving the input (an error or solution
  occurred).

Some architectures or programming environments require an assembly file in addition to
the provided header file. Notably, MSVC does not support intrinsics when compiling
edk2-based UEFI code, and does not support inline assembly, so assembly files are
necessary.