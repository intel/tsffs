# Choosing a Harnessing Method

As covered in the [harnessing](../harnessing/) section, there are three options for
harnessing a given target software:

- Open-box, or compiled-in harnesses using provided macros
- Closed-box harnessing that injects testcases into some target software memory
- Fully manual harnessing that returns the testcase to the harnessing script

The method that should be used depends on your target software and, more importantly,
your build system.

## Compiled-In/Open-Box Harnessing

If you control the build system and are able to modify the code, you
should almost always prefer the compiled-in harnesses. When you control the compilation,
some examples of when compiled-in harnesses should be used are:

- Your UEFI application has a function (or code flow) that takes external input
  - Uses files from the filesystem, SRAM, or other persistent storage
  - Takes input from the operating system
- Your Kernel module takes external input
  - Receives input from user-space via IOCTL or system call
  - Uses DMA or MMIO to take input from an external source
- Your user space application takes user input
  - From command line
  - From a file

## Closed-Box Harnessing

The closed-box harnessing methods covered in
[the closed-box section](../harnessing/closed-box.md) work in the same way as the
open-box harnessing approach. They should be used when the software takes input in the
same way as software that would be harnessed using the open-box approach, but whose
code or build system cannot be changed to add compiled-in harnessing.

## Fully Manual Harnessing

Fully manual harnessing should be used in cases where neither other approach is
possible or in extremely complex cases. For example, when significant code is required
to preprocess and send an input via an external interface, for harnessing code such as
a UEFI update mechanism. This approach (when used correctly) can save time that would
have been spent writing a harness in the target software, but you should take care that
in-target harnessing is not the best option.