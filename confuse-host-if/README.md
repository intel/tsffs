# Confuse Host Interface

This interface is to be used by host-side applications (running next to Simics) to control Simics and to exchange data with it.
It is split into a low-level interface (suffixed `ll`) to control Simics and a data input/output interface (suffixed `dio`) to exchange data with the SW under test.

## The low level interface

Has three functions:

`int confuse_init(const char* simics_prj, const char* config, simics_handle* simics)`

Start Simics using the provided Simics project with the provided config (can be any legal Simics script, i.e., YML, Python or Simics script). The path to the Simics project should best be absolute (relative has never been tested), the path to the config is relative to the project. On a succsessful return (return value 0), the handle `simics` will refer to the created session and can be used for other calls.

Expectations on the used Simics config:
- Automatically runs up to a point from where a fuzzing loop shall start
- Create an in-memory snapshot at that point
- Creates an instance of the `confuse_ll` Simics module (see `simics/modules/confuse-ll` in this repo).
- Creates an instance of the `confuse_dio` Simics module (see `simics/modules/confuse-dio` in this repo).
- Informs the created instances of `confuse_ll` and `confuse_dio` about the PID of the process using the low level interface which it can read out of the file `<SimicsProject>/_if_data_.tmp` created by `confuse_init`
- Informs the created instance of `afl-branch-tracer` about the name of the AFL shared memory which it can read out of the file `<SimicsProject>/_if_data_.tmp` created by `confuse_init`
- Defines stop conditions (using the instance of `confuse_dio`) for the test that ensure the Simics session will not run endlessly when continuing from the created snapshot.

The call to `confuse_init` will only return after all of the above is done.

`int confuse_reset(const simics_handle simics)`

Reset Simics to the snapshot created during `confuse_init`. Nothing else.

`int confuse_run(const simics_handle simics);`

Run Simics forward until Simics stops. As mentioned above, the Simics session must be configured in a way to avoid endless runs.

## The data input/output interface

This interface has only a single function `unsigned char* confuse_create_dio_shared_mem(unsigned long long size)`. Calling this function will create a shared memory, mmap it and return a pointer to it, ready to use.

The mmap will go away when the process terminates, hence we do not need to worry about cleaning this up.
However, the shared mem will persist. The idea is that the Simics side unlinks the shm
as soon as it has it mmapped as well. This will ensure that the shm is deallocated as soon
as both processes that have it mmapped die. So the only chance for a stale (and persisting)
shm is when the Simics side fails to start or crashes before unlinking the shm.
So in nominal execution, shm should be cleaned up at the end. For now, we recommend to check /dev/shm every now and then and potentially clean it up, in case there are some left overs.

The data format in the shared mem is not yet fully defined. Right now, we only support data movement between shared memory and the magic pipe. In the current state, the contract between host side interface and Simics module is that the buffer starts with a `size_t` value defining the amount of the following bytes. These bytes will then be moved from the shared mem into the magic pipe or the other way. The format is the same for input and output.

## Building

Just invoke `make`in this directory. Tested on Ubuntu 22.04 with gcc 11.3.
