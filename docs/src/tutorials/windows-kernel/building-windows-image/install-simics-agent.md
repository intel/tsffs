# Install the Simics Agent

You should already have Simics installed on your machine. In the Simics base directory
(e.g. `simics-6.0.185`), unzip `targets/common/images/simics_agent_binaries.zip`.

From the unzipped files, copy `simics_agent_x86_win64.exe` to the guest machine:

```sh
scp -P 2222 simics_agent_x86_win64.exe "user@localhost:C:\\Users\\user\\"
```

Next, on the guest machine, set the agent to run at logon:

```powershell
schtasks /create /sc onlogon /tn "Simics Agent" /tr "C:\Users\user\simics_agent_x86_win64.exe"
```

Now, set the `user` account to automatically log in at boot:

```powershell
reg add "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" /v AutoAdminLogon /t REG_SZ /d 1 /f
reg add "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" /v DefaultUserName /t REG_SZ /d "user" /f
reg add "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" /v DefaultPassword /t REG_SZ /d "password" /f
```

Restart the machine with:

```powershell
shutdown /r /f /t 0
```

And reconnect with:

```sh
ssh -P 2222 user@localhost
```

Ensure the agent is running:

```powershell
ps | findstr simics
```

You should see output like:

```powershell
     41       4      508       1408       0.02   4132   1 simics_agent_x86_win64
```