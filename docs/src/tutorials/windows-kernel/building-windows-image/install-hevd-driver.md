# Install HEVD Driver

With the HEVD driver installed, we will create a
service and set it to automatically run on system
start.

First, create the service:

```powershell
sc.exe create HEVD type= kernel start= auto binPath= C:\Users\user\HackSysExtremeVulnerableDriver\Driver\build\HEVD\Windows\HEVD.sys
```

The service will automatically start on reboot.

Reboot the guest with:

```powershell
shutdown /r /f /t 0
```

And reconnect via ssh:

```sh
ssh -p 2222 user@localhost
```

We will then check that the service is started with:

```powershell
sc.exe query HEVD
```

You should see:

```txt
SERVICE_NAME: HEVD
        TYPE               : 1  KERNEL_DRIVER
        STATE              : 4  RUNNING
                                (STOPPABLE, NOT_PAUSABLE, IGNORES_SHUTDOWN)
        WIN32_EXIT_CODE    : 0  (0x0)
        SERVICE_EXIT_CODE  : 0  (0x0)
        CHECKPOINT         : 0x0
        WAIT_HINT          : 0x0
```

The driver is installed and set to start automatically.

