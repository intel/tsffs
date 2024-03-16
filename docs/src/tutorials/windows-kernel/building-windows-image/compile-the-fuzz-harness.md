# Compile the Fuzz Harness

That's all we need to test the driver from user-space. We can now compile the harness by
entering the Build Environment for VS Community (not the EWDK, because it lacks
SanitizerCoverage and LibFuzzer):

```powershell
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1' -Arch amd64
cl /fsanitize=fuzzer /fsanitize-coverage=edge /fsanitize-coverage=trace-cmp /fsanitize-coverage=trace-div fuzzer.c
```
