# Installing the EWDK

We will use the [Enterprise Windows Driver
Kit](https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/using-the-enterprise-wdk)
(EWDK) throughout this tutorial to compile both user-space applications and Windows
Kernel modules.

We will use the EWDK because unfortunately all versions of Visual Studio (including
Visual Studio Community) are not possible to easily install on the command line, which
means more images which complicate a tutorial unnecessarily and make it harder to
maintain. If you are more comfortable with using Visual Studio, the remainder of the
tutorial will still be relevant and you can translate to the equivalent GUI
instructions.

## Download the EWDK

If the link below becomes outdated, you can obtain the EWDK ISO download by visiting the
[WDK and EWDK download
page](https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk#download-icon-enterprise-wdk-ewdk)
and downloading it. The page [Using the Enterprise
WDK](https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/using-the-enterprise-wdk)
also contains useful background.

You can download the latest version of the EWDK as of the time of writing (20 December
2023) by running (note the first line is required to obtain a [reasonable download
speed](https://stackoverflow.com/questions/28682642/powershell-why-is-using-invoke-webrequest-much-slower-than-a-browser-download)):

```powershell
$ProgressPreference = 'SilentlyContinue'
Invoke-WebRequest -Uri "https://software-static.download.prss.microsoft.com/dbazure/888969d5-f34g-4e03-ac9d-1f9786c66749/EWDK_ni_release_svc_prod1_22621_230929-1800.iso" -OutFile ~/Downloads/EWDK_ni_release_svc_prod1_22621_230929-1800.iso
```

This download is quite large (approximately 15GB). The command will finish when the
download is complete.

## Mount the EWDK Disk Image

To ensure paths throughout the tutorial work correctly, we will mount our
disk image to a specific drive letter (`W`).

```powershell
$diskImage = Mount-DiskImage -ImagePath C:\Users\user\Downloads\EWDK_ni_release_svc_prod1_22621_230929-1800.iso -NoDriveLetter
$volumeInfo = $diskImage | Get-Volume
mountvol W: $volumeInfo.UniqueId
```

Note that after a reboot or sleep, you may need to run this command again
to re-mount the disk image.

## Test the Build Environment

We can now launch the build environment by running:

```powershell
W:\LaunchBuildEnv.cmd
```

Test that the build environment works as expected:

```cmd
cl
```

You should see the output:

```txt
Microsoft (R) C/C++ Optimizing Compiler Version 19.31.31107 for x86
Copyright (C) Microsoft Corporation.  All rights reserved.

usage: cl [ option... ] filename... [ /link linkoption... ]
```

Make sure to exit the `cmd` environment after using it and return to PowerShell:

```cmd
exit
```
