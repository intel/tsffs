## Generic Simics scripts and modules

This directory holds scripts that are independent of the SW under test and the used Fuzzer and should be reusable for other tests and campaigns.

## The confuse_ll module

This is the Simics-side end of the `confuse_ll` interface (see also `confuse-host-if` in this repo).
When instantiated, it will react to SIGUSR1 and SIGUSR2. On SIGUSR1 it will make Simics run forward, on SIGUSR2 it will restore snapshot with ID 0 (the expectation is that there is exactly one snapshot).

In addition to this, it can inform the host side by sending SIGUSR2 to it. This can happen automatically as soon as Simics stops or manually by explicitly triggering that. For this, there are two pseudo attributes in the device:

- `send_usr2` : Writing a PID to this attribute will send SIGUSR2 to the given PID.
- `arm_auto_send_usr2`: Writing a PID to this attribute, will make the device send SIGUSR2 to the PID whenever Simics stops.

## Simics scripts in targets/qsp-x86-fuzzing

Technically, there is no need to have these scripts in their own sub directory in `targets` but this way it is easier to setup a project (see `simple-example` in this repo).
The scripts are actually not Fuzzing specific, they just start certain applications automatically and if said application is a test harness (or more precisily the Simics-side part of a test harness that it interacting with a host-side Fuzzer) then you can use it for fuzzing.

This is what we currently have in there:

- `run-uefi-app.simics` : Instantiate a QSP and (once the simulation runs forward) automatically go into the UEFI shell, transfer a specified EFI app into the system and start it. The EFI app must be provided in parameter `uefi_app` to the script.