# Compile the Fuzz Harness

That's all we need to test the driver from user-space. We can now compile the harness by
entering the Build Environment for VS Community (not the EWDK):

```powershell
Set-ExecutionPolicy Unrestricted
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1' -Arch amd64
ml64 /c /Cp /Cx /Zf tsffs-msvc-x86_64.asm
cl fuzzer.c tsffs-msvc-x86_64.obj
```
