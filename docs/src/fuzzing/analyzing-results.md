# Analyzing Results

Once a solution is found, the fuzzer can be run in *repro* mode which will:

* Save a bookmark when the testcase is written
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
