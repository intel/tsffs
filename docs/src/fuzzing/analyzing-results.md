# Analyzing Results

Once a solution is found, the fuzzer can be run in *repro* mode which will:

* Mark a restore point at the moment the testcase is written (a bookmark on SIMICS 6,
  the initial snapshot on SIMICS 7)
* Write only one testcase, the bytes from the specified file
* Stop without resetting to the initial snapshot

Repro mode can be run after stopping execution, or before executing the fuzzing loop.

```python
@tsffs.iface.fuzz.repro("%simics%/solutions/TESTCASE")
```

By default, repro mode automatically resumes simulation after preparing the
testcase. If you need to attach an external debugger (for example a GDB stub)
before execution begins, you can disable auto-continue so that when the harness
is triggered, TSFFS prepares the testcase but waits for you to resume manually:

```python
@tsffs.repro_auto_continue = False
```

See [Disable Auto-Continue in Repro Mode](../config/common-options.md#disable-auto-continue-in-repro-mode)
for more details.

## Reproducing Several Testcases in One Session

`repro` can be called again once a reproduced execution has stopped, as many times as
needed, to run another testcase from the same starting point. This also works in a
session that never fuzzed (one where `repro` was called before the target ran).

Each new call restores the initial snapshot and writes the new testcase:

```python
@tsffs.iface.fuzz.repro("%simics%/solutions/OTHER_TESTCASE")
```

With the default setting, TSFFS then resumes the simulation by itself, as it does for
the first testcase. With `repro_auto_continue = False`, the simulation stays stopped
with the testcase written, and one resume (`run` or `continue`) runs that testcase to
its end: the stop harness, a solution such as an exception, or the timeout. The restored
snapshot executes the start harness again, but that does not stop the simulation a
second time.

This makes it possible to script a loop over a directory of testcases: set
`repro_auto_continue = False`, then call `repro` and resume once for each file.
[Source Coverage for UEFI Targets](source-coverage.md) uses such a loop to replay a
whole corpus while SIMICS collects source coverage, with an example script for the loop.

Fuzzing cannot be resumed in a session that has entered repro mode.
