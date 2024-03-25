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
10. [Install Simics Agent](install-simics-agent.md)
11. [Clone and Build HEVD](clone-and-build-hevd.md)
12. [Install the Code Signing Certificate](install-the-code-signing-certificate.md)
13. [Install HEVD Driver](install-hevd-driver.md)
14. [Create a Fuzz Harness](create-a-fuzz-harness.md)
15. [Compile the Fuzz Harness](compile-the-fuzz-harness.md)
16. [Convert the Image to CRAFF](convert-image.md)
