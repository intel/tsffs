# Installing Development Tools

We will install a couple of additional development tools.

## Set Up Winget

On most systems, WinGet should work correctly without
tweaking, however it is a notoriously buggy tool, and
in many cases issues may occur. You can view the most
up-to-date troubleshooting steps on
[GitHub](https://github.com/microsoft/winget-cli/tree/master/doc/troubleshooting).

In most cases, the best way to resolve an issue is to
simply install a new version of WinGet. The most up to
date `msixbundle` link can be found from the
[releases](https://github.com/microsoft/winget-cli/releases/latest).
For example:

```powershell
Invoke-WebRequest -Out C:\Users\user\Downloads\winget.msixbundle https://github.com/microsoft/winget-cli/releases/download/v1.7.10661/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle
Add-AppxPackage C:\Users\user\Downloads\winget.msixbundle
```

Then check the version matches with `winget --info`.

Once you have a working WinGet installation, update
your sources with:

```powershell
winget source update
```

You should see "Done" messages for all sources. If you
do not, refer to the troubleshooting steps, because the
next steps in this tutorial will not work correctly.

## Install Git

Install Git with:

```powershell
winget install --id Git.Git -e --source winget
```

Once the installation is complete (you should see some licensing and download
information on the command line), add it to the path with:

```powershell
$env:Path += ";C:\Program Files\Git\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\Git\bin", "Machine")
```

## Install Vim

Install Vim with:

```powershell
winget install --id vim.vim -e --source winget
```

And add it to the path with the following. Note that
the sub-directory `vim91` may change with newer
versions of vim -- make note of the major and minor
version displayed during the winget install (like
`Found Vim [vim.vim] Version 9.1.0104`) and subsitute
the major and minor version into the command below.

```powershell
$env:Path += ";C:\Program Files\Vim\vim91"
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\Vim\vim91", "Machine")
```

## Install CMake

Install CMake with:

```powershell
winget install --id Kitware.CMake -e --source winget
```

And add it to the path with:


```powershell
$env:Path += ";C:\Program Files\CMake\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\CMake\bin", "Machine")
```

## Install Visual Studio Community

We will use the EWDK to build the vulnerable driver,
but because we will be using LibFuzzer to fuzz the
driver from user-space, we also need to install Visual
Studio Community with the proper workloads to obtain
the LibFuzzer implementation.

```powershell
winget install Microsoft.VisualStudio.2022.Community --silent --override "--wait --quiet --addProductLang En-us --add Microsoft.VisualStudio.Workload.NativeDesktop --add Microsoft.VisualStudio.Component.VC.ASAN --add Microsoft.VisualStudio.Component.VC.ATL --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --add Microsoft.Component.VC.Runtime.UCRTSDK --add Microsoft.VisualStudio.Workload.CoreEditor"
```

The command will return once the installation is
complete, it may take a very long time (the same as the
graphical VS installer).

## Refresh PATH

The `$env:Path` environment variable changes will not
take effect until SSHD is restarted. Restart it with
(this will not end your current session):

```powershell
Restart-Service -Name sshd
```

Now, exit the sesion by typing `exit` and re-connect via SSH. Confirm the
environment variable changes took effect:

```powershell
git --version
vim --version
cmake --version
```

Both commands should succeed.

