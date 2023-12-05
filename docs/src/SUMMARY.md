# Summary

[Introduction](README.md)

## User Guide

- [Setting Up](setup/README.md)
  - [Using Docker](setup/docker.md)
  - [For Linux](setup/linux.md)
  - [For Windows](setup/windows.md)
- [Configuration](config/README.md)
  - [Installing in Projects](config/installing-in-projects.md)
  - [Loading The TSFFS Module](config/loading-module.md)
  - [Common Options](config/common-options.md)
- [Harnessing Fuzzing Targets](harnessing/README.md)
  - [Using a Compiled-in Harness](harnessing/compiled-in.md)
  - [Using Closed-Box Testcase Injection](harnessing/closed-box.md)
  - [Using Testcase Data Manually](harnessing/manual.md)
- [Running A Fuzzing Campaign](fuzzing/README.md)
  - [Checking Target Software Compatibility](fuzzing/compatibility.md)
  - [Choosing A Harnessing Method](fuzzing/choose-harnessing-method.md)
  - [Running the Fuzzer](fuzzing/running-fuzzer.md)
  - [Optimizing For Fuzzing](fuzzing/optimizing-for-fuzzing.md)
  - [Analyzing Results](fuzzing/analyzing-results.md)

## Tutorials

- [Tutorials](tutorials/README.md)
  - [Fuzzing an x86_64 EDK2 UEFI Application](tutorials/edk2-uefi/README.md)
    - [Writing the Application](tutorials/edk2-uefi/writing-the-application.md)
    - [Building the Application](tutorials/edk2-uefi/building-the-application.md)
    - [Testing the Application](tutorials/edk2-uefi/testing-the-application.md)
    - [Configuring the Fuzzer](tutorials/edk2-uefi/configuring-the-fuzzer.md)
    - [Running the Fuzzer](tutorials/edk2-uefi/running-the-fuzzer.md)
    - [Reproducing Runs](tutorials/edk2-uefi/reproducing-runs.md)
    - [Optimizing For Speed](tutorials/edk2-uefi/optimizing-for-speed.md)
  - [Fuzzing a RISC-V Kernel Module](tutorials/kernel-module/README.md)
    - [Target Software Outline](tutorials/kernel-module/target-software-outline.md)
    - [Target Software Boilerplate](tutorials/kernel-module/target-software-boilerplate.md)
    - [Kernel Module Code](tutorials/kernel-module/kernel-module-code.md)
    - [Kernel Module Harnessing](tutorials/kernel-module/kernel-module-harnessing.md)
    - [Updating the Build Configuration](tutorials/kernel-module/build-configuration-updates.md)
    - [Running the Fuzzer](tutorials/kernel-module/running-the-fuzzer.md)

## Reference Guide

- [SIMICS and Crate Documentation](documentation/README.md)
- [Developer Documentation](developer/README.md)
  - [Build Internals](developer/build.md)
  - [Refreshing Build Environment](developer/refresh.md)
  - [Building Against A Specific SIMICS Version](developer/specific-simics-version.md)

