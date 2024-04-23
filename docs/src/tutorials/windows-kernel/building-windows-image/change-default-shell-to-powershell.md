# Change Default Shell to PowerShell

This is a CMD command prompt. The remainder of the tutorials for Windows will
provide only PowerShell commands. To change the default shell for OpenSSH to
PowerShell, run:

```cmd
powershell.exe -Command "New-ItemProperty -Path 'HKLM:\SOFTWARE\OpenSSH' -Name DefaultShell -Value 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' -PropertyType String -Force"
```

Exiting the SSH session by running `exit`, then reconnecting with `ssh -p 2222
user@localhost` should log you into a PowerShell session by default:

```txt
Windows PowerShell
Copyright (C) Microsoft Corporation. All rights reserved.

Try the new cross-platform PowerShell https://aka.ms/pscore6

PS C:\Users\user>
```
