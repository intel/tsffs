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

See [Disable Auto-Continue in Repro
Mode](../config/common-options.md#disable-auto-continue-in-repro-mode) for more
details.

## Reproducing Several Testcases in One Session

`repro` can be called again once a reproduced execution has stopped, as many times as
needed, to run another testcase from the same starting point. This also works in a
session that never fuzzed (one where `repro` was called before the target ran).

Each new call restores the initial snapshot and writes the new testcase:

```python
@tsffs.iface.fuzz.repro("%simics%/solutions/OTHER_TESTCASE")
```

With the default setting (`repro_auto_continue = True`), TSFFS then resumes the
simulation by itself, as it does for the first testcase. That resume is queued: SIMICS
performs it once the command that called `repro` has returned. So:

- When the call is a line of its own in a SIMICS script, the testcase has run to its end
  before the next line executes.
- When the call is made from inside a Python function, the testcase has not run yet when
  the call returns. The simulation is still stopped, and the testcase only runs after
  the function has returned to SIMICS. A Python loop that calls `repro` and then reads
  the result would read it before the testcase has run.

With `repro_auto_continue = False`, nothing is queued. The simulation stays stopped with
the testcase written, and one resume (`run` or `continue`) runs that testcase to its
end: the stop harness, a solution such as an exception, or the timeout. The restored
snapshot executes the start harness again, but that does not stop the simulation a
second time.

This is the setting for a loop over a directory of testcases, in a SIMICS script or in
Python: set `repro_auto_continue = False`, then for each file call `repro` and resume
once (from Python, `cli.quiet_run_command("run")`). The example script of
[Source Coverage for UEFI Targets](source-coverage.md) does exactly that to replay a
whole corpus while SIMICS collects source coverage.

Fuzzing cannot be resumed in a session that has entered repro mode.
