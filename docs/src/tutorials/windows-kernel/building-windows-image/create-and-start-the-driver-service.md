# Create and Start the Driver Service

We'll create a service for the driver and start it.

```powershell
sc.exe create HEVD type= kernel binPath= C:\Users\user\HackSysExtremeVulnerableDriver\Driver\build\HEVD\Windows\HEVD.sys
sc.exe start HEVD
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
        PID                : 0
        FLAGS              :
```

This means our vulnerable driver is now running.
