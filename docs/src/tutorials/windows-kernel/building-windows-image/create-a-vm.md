# Create a VM

Run VirtualBox. You will be greeted with this window (if this is your first
time using VirtualBox, the list of VMs will be empty, for example below you
will not see an entry for "Windows 10"):

![VM List](images/2024-03-16-11-55-02.png)

Click "New" to create a new Virtual Machine. You should see the dialog below.

![New VM Dialog](images/2024-03-16-11-55-58.png)

Enter a name, select the ISO image we downloaded, and be sure to check "Skip
unattended installation". Then, click "Next".

![New VM Dialog 2](images/2024-03-16-11-56-39.png)

At least 4GB of RAM and 1 CPU is recommended, but add more if you have
resources available. Be sure to *check* "Enable EFI (special OSes Only)". Then,
click "Next".

![New VM Dialog 3](images/2024-03-16-11-57-17.png)

At least 64GB of disk space is recommended to ensure enough space for all
required development tools, including Visual Studio and the Windows Driver Kit.

![New VM Dialog 4](images/2024-03-16-11-57-48.png)

Ensure the settings look correct, then select "Finish".

![Finish Settings](images/2024-03-16-11-58-06.png)

Click "Settings" in the Windows 11 image tab.

![Settings](images/2024-03-16-12-10-29.png)

In the "System" tab and "Motherboard" sub-tab, ensure the following settings.
Uncheck "Floppy" from "Boot Order", ensure "TPM" is set to "v2.0", ensure
"Enable I/O APIC" and "Enable EFI" are checked, and ensure "Enable Secure Boot"
is *unchecked*.

![System Tab](images/2024-03-16-12-11-56.png)

In the "System" tab and "Processor" sub-tab, ensure the following settings.
Ensure "Enable PAE/NX" is *checked* and that "Enable Nested VT-x/AMD-V" is
*unchecked*.

![System Processor Tab](images/2024-03-16-12-17-25.png)

In the "Display" tab, ensure "Graphics Controller" is set to "VBoxSVGA" and
"Extended Features: Enable 3D Acceleration" is *checked*.

![Display](images/2024-03-16-12-13-14.png)

Click "OK" to close the settings Window, then click "Start" in the Windows 11
image tab to start the virtual machine.

![Start VM](images/2024-03-16-12-14-15.png)

A window that says "Press any key to boot from CD or DVD....." will appear.
Click inside the Virtual Machine window and press "Enter". The VirtualBox EFI
boot screen should appear, followed by the Windows Setup dialog. We're ready to
install Windows.

![](images/2024-03-16-12-14-29.png)
