# Building a Simics-Compatible Windows Kernel Development VM

We will use VirtualBox to create a Windows Kernel Development Virtual machine
before converting the VirtualBox Virtual Disk Image (VDI) to the CRAFF format used by
Simics.

There are several advantages to creating the image this way:

- Speed: VirtualBox runs faster than Simics and is easier to work with interactively
- Compatibility: The image can be used for other purposes
- Iteration: Speed and compatibility allow iterating on the image contents more quickly

1. [Install VirtualBox](install-virtualbox.md)
2. [Download Windows](download-windows.md)
3. [Create a VM](create-a-vm.md)
4. [Install Windows](install-windows.md)
5. [Set Up SSH](set-up-ssh.md)
6. [Enable SSH Port Forwarding in VirtualBox](enable-ssh-port-forwarding-in-virtualbox.md)
7. [Change Default Shell to PowerShell](change-default-shell-to-powershell.md)
8. [Installing the EWDK](installing-the-ewdk.md)
9. [Installing Development Tools](installing-development-tools.md)
10. [Clone and Build HEVD](clone-and-build-hevd.md)
11. [Install the Code Signing Certificate](install-the-code-signing-certificate.md)
12. [Create and Start the Driver Service](create-and-start-the-driver-service.md)
13. [Create a Fuzz Harness](create-a-fuzz-harness.md)
14. [Compile the Fuzz Harness](compile-the-fuzz-harness.md)
15. [Run the Fuzz Harness](run-the-fuzz-harness.md)