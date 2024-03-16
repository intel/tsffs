# Clone and Build HEVD

We will use [HackSys Extreme Vulnerable Driver
(HEVD)](https://github.com/hacksysteam/HackSysExtremeVulnerableDriver) as our windows
driver target.

We'll clone HEVD into our home directory and enter the EWDK build environment.

```powershell
cd ~
git clone https://github.com/novafacing/HackSysExtremeVulnerableDriver -b windows-training
cd HackSysExtremeVulnerableDriver/Driver
W:\LaunchBuildEnv.cmd
```

Now, we can go ahead and build the driver:

```cmd
cmake -S . -B build -DKITS_ROOT="W:\Program Files\Windows Kits\10"
cmake --build build --config Release
```

And exit our build environment:

```cmd
exit
```

Back in PowerShell, check to make sure there is a release directory:

```powershell
ls build/HEVD/Windows/
```

You should see:


```txt

    Directory: C:\Users\user\HackSysExtremeVulnerableDriver\Driver\build\HEVD\Windows


Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
d-----        12/20/2023   7:16 PM                CMakeFiles
d-----        12/20/2023   7:16 PM                HEVD.dir
d-----        12/20/2023   7:17 PM                Release
-a----        12/20/2023   7:16 PM           1073 cmake_install.cmake
-a----        12/20/2023   7:17 PM           2275 hevd.cat
-a----        12/20/2023   7:17 PM           1456 HEVD.inf
-a----        12/20/2023   7:17 PM          32216 HEVD.sys
-a----        12/20/2023   7:16 PM          45308 HEVD.vcxproj
-a----        12/20/2023   7:16 PM           4117 HEVD.vcxproj.filters
```

If so, we're in business!
