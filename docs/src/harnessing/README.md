# Harnessing

Harnessing target software to effectively use TSFFS to fuzz it is a target-dependent
subject, so examples of each supported harnessing method are provided here. The order of
each approach here is roughly equivalent to the preferred order harnessing should be
tried. If possible, the target software should be harnessed at the source-code level. If
not, try injecting testcases into its memory directly, and if this is still not possible
or not appropriate for your use case, the fully-manual approach can be used.

- [Using Compiled-In Harnesses](compiled-in.md)
- [Closed-Box Testcase Injection](closed-box.md)
- [Manual Testcase Injection](manual.md)