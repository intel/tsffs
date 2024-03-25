# Install the Code Signing Certificate

Windows does not permit loading drivers signed with
untrusted certificates, so we need to both import our
untrusted certificate and enable test signing. From the
`HackSysExtremeVulnerableDriver\Driver` directory, run
the following to enable test signing and reboot (which
is required after enabling test signing):

```powershell
certutil -importPFX HEVD\Windows\HEVD.pfx
bcdedit -set TESTSIGNING on
bcdedit -set loadoptions DISABLE_INTEGRITY_CHECKS
shutdown /r /f /t 0
```

Once the Virtual Machine reboots, you can reconnect with `ssh -p 2222 user@localhost`.
