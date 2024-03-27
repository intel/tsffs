#ifndef TSFFS_H
#define TSFFS_H

/// Define common with LibFuzzer and other fuzzers to allow code that is
/// fuzzing-specific to be left in the codebase. See
/// https://llvm.org/docs/LibFuzzer.html#id35 for more information
#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
#define FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION (1)
#endif  // FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION

#ifdef __cplusplus

#include <cstdint>

/// HARNESS_START
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_INDEX` macro to specify different indices, then enable them
/// at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The size of the buffer pointed to by `size_ptr` will be saved as the
///   maximum testcase size. Each fuzzing iteration, the actual size of the
///   current testcase will be written to `*size_ptr`.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START(buffer, &size);
/// ```
extern "C" void HARNESS_START(void *buffer, void *size_ptr);

/// HARNESS_START_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The size of the buffer pointed to by `size_ptr` will be saved as the
///   maximum testcase size. Each fuzzing iteration, the actual size of the
///   current testcase will be written to `*size_ptr`.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_INDEX(0x0001U, buffer, &size);
/// ```
extern "C" void HARNESS_START_INDEX(size_t start_index, void *buffer, void *size_ptr);

/// HARNESS_START_WITH_MAXIMUM_SIZE
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_INDEX` macro to specify different indices,
/// then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE(buffer, 1024);
/// ```
extern "C" void HARNESS_START_WITH_MAXIMUM_SIZE(void *buffer, size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(0x0001U, buffer, 1024);
/// ```
extern "C" void HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(size_t start_index, void *buffer,
                                           size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX` macro to specify different
/// indices, then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The address `size_ptr` will be saved. Each fuzzing iteration, the actual
/// size of the current testcase will be written to `*size_ptr`.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, &size, 1024);
/// ```
extern "C" void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(void *buffer, void *size_ptr,
                                             size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The address `size_ptr` will be saved. Each fuzzing iteration, the actual
/// size of the current testcase will be written to `*size_ptr`.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(0x0001U, buffer, &size, 1024);
/// ```
extern "C" void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(size_t start_index,
                                                   void *buffer, void *size_ptr,
                                                   size_t max_size);

/// HARNESS_STOP
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The
/// default index of 0 will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use the
/// `HARNESS_STOP_INDEX` macro to specify different indices, then enable them at
/// runtime by configuring the fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_STOP();
/// ```
extern "C" void HARNESS_STOP(void);

/// HARNESS_STOP_INDEX
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The index
/// specified by `stop_index` will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// # Arguments
///
/// - `stop_index`: The index to use for this stop harness
///
/// # Example
///
/// ```
/// HARNESS_STOP_INDEX(0x0001U);
/// ```
extern "C" void HARNESS_STOP_INDEX(size_t stop_index);

/// HARNESS_ASSERT
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The default index of 0 will be
/// used. If you need to differentiate between multiple assertion harnesses
/// compiled into the same binary, you can use the `HARNESS_ASSERT_INDEX` macro
/// to specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_ASSERT();
/// ```
extern "C" void HARNESS_ASSERT(void);

/// HARNESS_ASSERT_INDEX
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The index specified by
/// `assert_index` will be used. If you need to differentiate between multiple
/// assertion harnesses compiled into the same binary, you can use this macro to
/// specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Arguments
///
/// - `assert_index`: The index to use for this assertion harness
///
/// # Example
///
/// ```
/// HARNESS_ASSERT_INDEX(0x0001U);
/// ```
extern "C" void HARNESS_ASSERT_INDEX(size_t assert_index);

#else // __cplusplus

#include <stddef.h>

/// HARNESS_START
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_INDEX` macro to specify different indices, then enable them
/// at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The size of the buffer pointed to by `size_ptr` will be saved as the
///   maximum testcase size. Each fuzzing iteration, the actual size of the
///   current testcase will be written to `*size_ptr`.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START(buffer, &size);
/// ```
void HARNESS_START(void *buffer, void *size_ptr);

/// HARNESS_START_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The size of the buffer pointed to by `size_ptr` will be saved as the
///   maximum testcase size. Each fuzzing iteration, the actual size of the
///   current testcase will be written to `*size_ptr`.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_INDEX(0x0001U, buffer, &size);
/// ```
void HARNESS_START_INDEX(size_t start_index, void *buffer, void *size_ptr);

/// HARNESS_START_WITH_MAXIMUM_SIZE
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_INDEX` macro to specify different indices,
/// then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE(buffer, 1024);
/// ```
void HARNESS_START_WITH_MAXIMUM_SIZE(void *buffer, size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(0x0001U, buffer, 1024);
/// ```
void HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(size_t start_index, void *buffer,
                                           size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The default "index" of 0 will be used. If you need multiple start
/// harnesses compiled into the same binary, you can use the
/// `HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX` macro to specify different
/// indices, then enable them at runtime by configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The address `size_ptr` will be saved. Each fuzzing iteration, the actual
/// size of the current testcase will be written to `*size_ptr`.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(buffer, &size, 1024);
/// ```
void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(void *buffer, void *size_ptr,
                                             size_t max_size);

/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX
///
/// Signal the fuzzer to start the fuzzing loop at the point this macro is
/// called. The index specified by `start_index` will be used. If you need
/// multiple start harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// When this macro is called:
///
/// - A snapshot will be taken and saved
/// - The buffer pointed to by `buffer` will be saved and used as the testcase
///   buffer. Each
///   fuzzing iteration, a new test case will be written to this buffer.
/// - The address `size_ptr` will be saved. Each fuzzing iteration, the actual
/// size of the current testcase will be written to `*size_ptr`.
/// - The `max_size` value will be saved as the maximum testcase size. Fuzzing
///   test cases will be truncated to this size before being written to the
///   buffer.
///
/// # Arguments
///
/// - `start_index`: The index to use for this start harness
/// - `buffer`: The pointer to the testcase buffer
/// - `size_ptr`: The pointer to the size of the testcase buffer
/// - `max_size`: The maximum size of the testcase buffer
///
/// # Example
///
/// ```
/// unsigned char buffer[1024];
/// size_t size;
/// HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(0x0001U, buffer, &size, 1024);
/// ```
void HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(size_t start_index,
                                                   void *buffer, void *size_ptr,
                                                   size_t max_size);

/// HARNESS_STOP
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The
/// default index of 0 will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use the
/// `HARNESS_STOP_INDEX` macro to specify different indices, then enable them at
/// runtime by configuring the fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_STOP();
/// ```
void HARNESS_STOP(void);

/// HARNESS_STOP_INDEX
///
/// Signal the fuzzer to stop and reset to the beginning of the fuzzing loop
/// with a "normal" stop status, indicating no solution has occurred. The index
/// specified by `stop_index` will be used. If you need to differentiate between
/// multiple stop harnesses compiled into the same binary, you can use this
/// macro to specify different indices, then enable them at runtime by
/// configuring the fuzzer.
///
/// # Arguments
///
/// - `stop_index`: The index to use for this stop harness
///
/// # Example
///
/// ```
/// HARNESS_STOP_INDEX(0x0001U);
/// ```
void HARNESS_STOP_INDEX(size_t stop_index);

/// HARNESS_ASSERT
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The default index of 0 will be
/// used. If you need to differentiate between multiple assertion harnesses
/// compiled into the same binary, you can use the `HARNESS_ASSERT_INDEX` macro
/// to specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Example
///
/// ```
/// HARNESS_ASSERT();
/// ```
void HARNESS_ASSERT(void);

/// HARNESS_ASSERT_INDEX
///
/// Signal the fuzzer that a custom assertion has occurred, and the fuzzer
/// should stop the current fuzzing iteration and reset to the beginning of the
/// fuzzing loop with a "solution" stop status. The index specified by
/// `assert_index` will be used. If you need to differentiate between multiple
/// assertion harnesses compiled into the same binary, you can use this macro to
/// specify different indices, then enable them at runtime by configuring the
/// fuzzer.
///
/// # Arguments
///
/// - `assert_index`: The index to use for this assertion harness
///
/// # Example
///
/// ```
/// HARNESS_ASSERT_INDEX(0x0001U);
/// ```
void HARNESS_ASSERT_INDEX(size_t assert_index);
#endif // __cplusplus

#endif  // TSFFS_H
