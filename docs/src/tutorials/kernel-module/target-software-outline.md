# Target Software Outline

We are targeting RISC-V, so we will be using [buildroot](https://buildroot.org/) for
our toolchain and Linux build. We need to build the following:

* `fw_jump.elf`, `Image`, and `rootfs.ext2`, our firmware jump binary, linux kernel
  image, and root filesystem, respectively. These three files are expected by the
  public RISC-V platform model for SIMICS to boot Linux. Other approaches can be
  used but will require significantly more customization.
* `tutorial-mod.ko` our tutorial kernel module. We'll create a kernel module which
  provides a virtual device which can be controlled via IOCTL.
* `tutorial-mod-driver` a user-space application which will trigger the funcionality
  we want to fuzz in our kernel module. We'll discuss how to harness both by
  compiling the harness code into the kernel module *and* by compiling the harness code
  into the user-space driver application.

We'll use the
[br2-external](https://buildroot.org/downloads/manual/manual.html#outside-br-custom)
mechanism to keep our kernel module package separate from the buildroot tree.
