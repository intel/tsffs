# Analyzing Results

Once a solution is found, the fuzzer can be run in *repro* mode which will:

* Save a bookmark when the testcase is written
* Write only one testcase, the bytes from the specified file
* Stop without resetting to the initial snapshot

Repro mode can be run after stopping execution, or before executing the fuzzing loop.

```python
tsffs.iface.fuzz.repro("%simics%/solutions/TESTCASE")
```