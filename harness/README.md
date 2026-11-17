# Harness Collection for TSFFS

This directory contains a selection of provided (and tested) harness header files. All
harnesses are tested automatically at project test time for correct passing of the
testcase and size addresses.

All headers define the following:

* `HARNESS_START(buffer, size_ptr)` - The macro used to signal the fuzzer to start
  fuzzing, writing each testcase to `buffer` and writing the size of each testcase to
  `*size_ptr`, where `*size_ptr` is initially equal to the maximum testcase size (i.e.
  the size of `buffer`).
* `HARNESS_START_WITH_MAXIMUM_SIZE(buffer, max_size)` - Like `HARNESS_START`, but takes
  the maximum testcase size by value. The size of each testcase is not written back.
* `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, size_ptr, max_size)` - Like
  `HARNESS_START`, but takes the maximum testcase size by value, so `*size_ptr` does not
  need to be initialized. The size of each testcase is written to `*size_ptr`.
* `HARNESS_STOP()` - The macro used to signal the fuzzer to stop the current execution,
  restore the snapshot taken at the location of `HARNESS_START`, and start another
  execution with a new testcase, without saving the input (no error or solution
  occurred).
* `HARNESS_ASSERT()` - The macro used to signal the fuzzer to stop the current
  execution, restore the snapshot taken at the location of `HARNESS_START`, and start
  another execution with a new testcase, while saving the input (an error or solution
  occurred).

Each macro has an `_INDEX` variant (for example `HARNESS_START_INDEX(index, buffer,
size_ptr)` and `HARNESS_STOP_INDEX(index)`) which takes an index as its first argument,
so that several harnesses can be compiled into one binary and selected at runtime with
the `magic_start_index`, `magic_stop_indices` and `magic_assert_indices` attributes.

Some architectures or programming environments require an assembly file in addition to
the provided header file. Notably, MSVC does not support intrinsics when compiling
edk2-based UEFI code, and does not support inline assembly, so assembly files are
necessary.