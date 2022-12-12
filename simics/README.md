## Generic Simics scripts and modules

This directory holds scripts that are independent of the SW under test and the used Fuzzer and should be reusable for other tests and campaigns.

## The confuse_ll module

This is the Simics-side end of the `confuse` low level interface (see also `confuse-host-if` in this repo).
When instantiated, it will react to SIGUSR1 and SIGUSR2. On SIGUSR1 it will make Simics run forward, on SIGUSR2 it will restore snapshot with ID 0 (the expectation is that there is exactly one snapshot).

In addition to this, it can inform the host side by sending SIGUSR2 to it. This can happen automatically as soon as Simics stops or manually by explicitly triggering that. For this, there are two pseudo attributes in the device:

- `send_usr2` : Writing a PID to this attribute will send SIGUSR2 to the given PID.
- `arm_auto_send_usr2`: Writing a PID to this attribute, will make the device send SIGUSR2 to the PID whenever Simics stops.

## The confuse_dio module

This is the Simics-side end of the `confuse` DIO interface (see also `confuse-host-if` in this repo).
When instantiated, it will do nothing, really. As soon as you assign its attribute `if_pid` it will attempt to find a sharded memory named `/dev/shm/confuse-dio-shm-<16digitPID>` and mmap it for use.

In addition, it will listen to the magic pipe. The expectation is that the target SW uses the pipe twice per test run. First, at the very beginning, where the target SW does not write anything into the pipe but wants to get input data and secondly, when the test is done and the target SW writes out results and does not expect any data back from the pipe.

So when the module sees 0 bytes coming out of the pipe in the reader callback, it knows that in the writer callback it needs to read from the shared mem and write the new inputs for the target SW into the pipe. When it sees >0 bytes coming out of the pipe in the reader callback, it will read test results data from the pipe and write it into the shared memory and do not write anything into the pipe in the following write callback.


## Simics scripts in targets/qsp-x86-fuzzing

Technically, there is no need to have these scripts in their own sub directory in `targets` but this way it is easier to setup a project (see `simple-example` in this repo).
The scripts are actually not Fuzzing specific, they just start certain applications automatically and if said application is a test harness (or more precisily the Simics-side part of a test harness that it interacting with a host-side Fuzzer) then you can use it for fuzzing.

This is what we currently have in there:

- `run-uefi-app.simics` : Instantiate a QSP and (once the simulation runs forward) automatically go into the UEFI shell, transfer a specified EFI app into the system and start it. The EFI app must be provided in parameter `uefi_app` to the script.


## MagicPipeLib in target-sw

This is a UEFI port of the magic pipe library such that EFI applications can use the magic pipe to transfer data directly to an endpoint running on the host (within Simics).