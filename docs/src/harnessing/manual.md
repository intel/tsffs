# Manual Harnessing

If the target software does not provide opportunity for injecting testcases into memory,
for example when testing an application which takes input via a network or other
hardware interface, manual harnessing can be used. This interface simply provides a way
for users to obtain the fuzzing test case directly from the fuzzer and use it in any way
that is appropriate.

Harnessing in this way can be done using the api. Note that the API method still takes
a CPU object. When called, the initial snapshot is still captured in the same way as
with other [closed-box](closed-box.md) harnessing methods.

```python
@testcase = tsffs.iface.fuzz.start_without_buffer(cpu)
```