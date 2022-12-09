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
- Informs the created instance of `confuse_ll` about the PID of the process using the low level interface which it can read out of the file `<SimicsProject>/_if_data_.tmp`
- Defines stop conditions for the test that ensure the Simics session will not run endlessly when continuing from the created snapshot.

The call to `confuse_init` will only return after all of the above is done.

`int confuse_reset(const simics_handle simics)`

Reset Simics to the snapshot created during `confuse_init`. Nothing else.


`int confuse_run(const simics_handle simics);`

Run Simics forward until Simics stops. As mentioned above, the Simics session must be configured in a way to avoid endless runs.


## The data input/output interface

TBD

## Building

Just invoke `make`in this directory. Tested on Ubuntu 22.04 with gcc 11.3.
