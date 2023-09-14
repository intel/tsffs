# Breakpoints

Using breakpoints is the main method of configuring additional user-defined fault
conditions.

For example, if your platform considers it an error for any code to write to a
particular memory address range, breakpoints can be used to allow the fuzzer to detect
that event and report it as a fault.

Along the way, if you'd like to skp the programming, you can find the full source for
this tutorial in the [breakpoints example](../examples/breakpoints/).

- [Breakpoints](#breakpoints)
  - [Install Rizin](#install-rizin)
  - [Update the Target Software](#update-the-target-software)
  - [Compile the Target Software](#compile-the-target-software)
  - [Set Breakpoints as Faults](#set-breakpoints-as-faults)
  - [Find Breakpoint Locations](#find-breakpoint-locations)
  - [Set Breakpoints](#set-breakpoints)
  - [Fuzz for Breakpoints](#fuzz-for-breakpoints)

## Install Rizin

Before we get started, you'll need to install the
[rizin](https://github.com/rizinorg/rizin) reverse engineering tool. We will use Rizin
to determine where we want to set our breakpoints.

Installation instructions for Rizin can be found on their [install
page](https://rizin.re/install/).

## Update the Target Software


To demonstrate how to use breakpoints, we'll first update our target software. Using
the same UEFI application target from the [Fuzzing A UEFI Target](./UEFISimpleTarget.md)
tutorial, we'll copy the code and add a couple new branches to the checks at the end
so that it looks like this:

```c
if (*(char *)buffer == 'a') {
  // Invalid opcode
  __asm__(".byte 0x06");
} else if (*(char *)buffer == 'b') {
  // Crash
  uint8_t *bad_ptr = (uint8_t *)0xffffffffffffffff;
  *bad_ptr = 0;
} else if (*(char *)buffer == 'c') {
  // Breakpoint-defined fault location (instruction BP)
  SystemTable->conOut->output_string(SystemTable->conOut,
                                      (int16_t *)L"Uh oh!\r\n");
} else if (*(char *)buffer == 'd') {
  for (size_t i = 0; i < sizeof(off_limits); i++) {
      off_limits[i] = 'X';
  }
}
```

We'll also need to declare our `off_limits` buffer at global scope:


```c
char off_limits[0x100] = {0x00};
```

## Compile the Target Software

We'll compile the software in almost the same way as we did in the [Fuzzing A UEFI
Target](./UEFISimpleTarget.md) tutorial, but we will add the `-g` flag. We will also
assume you have checked out the [harnessing](./Harnessing.md) documentation and are
using the include file to harness your code. Finally, this command assumes you are
compiling in the [examples/breakpoints/src/](../examples/breakpoints/src/) directory.

```sh
clang -target x86_64-pc-win32-coff -fno-stack-protector -fshort-wchar -mno-red-zone \
  -O0 -I../../../include/ -g -c target.c -o target.o
lld-link -filealign:16 -subsystem:efi_application -nodefaultlib -dll -entry:UefiMain \
  target.o -out:target.efi
```

## Set Breakpoints as Faults

To treat breakpoints as faults, we need to first tell the fuzzer that we want
breakpoints to be treated as faults. For various reasons, chiefly convenience,
breakpoints will *not* automatically be treated as faults and will instead be ignored.

To stop ignoring them, call `set_breakpoints_are_faults(True)` on the module interface:


```simics
@conf.tsffs_module.iface.set_breakpoints_are_faults(True)
```

## Find Breakpoint Locations

In practice, you may want to automate this process, possibly using DWARF or PDB debug
information. For the sake of once again dispelling any magic, we'll do it manually
first. To set breakpoints, we need to know *where* to set them. In particular, we want
to find the addresses of:

- Our start harness' magic instruction
- The instruction that calls the "Uh oh!" print-out
- The off-limits buffer we create

We'll use `rizin` to retrieve this information. Documenting the use of `rizin` is far
outside the scope of this document, but you can read the
[documentation](https://book.rizin.re) for more information.

First, we want to find the address of our start harness by seeking to the `entry0`
function, printing disassembly of the function, and grepping the output for `cpuid`.

```sh
$ rizin -Aqqc 'e asm.var=false; e asm.lines=false; sf entry0; pdf ~cpuid;' target.efi
0x180001082      cpuid
0x180001304      cpuid
```

We see two CPUID commands, the first of which is our *start* harness address and the
second of which is our *stop* harness. We'll use the first addres `0x180001082` later
as `TARGET_HARNESS_ADDRESS`.

Next, we want to find the address of the instruction that is printing out the "Uh oh!"
string.

```sh
$ rizin -Aqqc 'e asm.var=false; e asm.lines=false; sf entry0; pdf ~Uh oh!' \
    ../rsrc/target.efi
0x180001260      lea   rdx, data.180002010                             ; 0x180002010 ; u"Uh oh!\r\n"
```

This instruction is just loading the string's address in `rdx`, we need to disassemble
forward a few instructions to find the actual call:

```sh
$ rizin -Aqqc 'e asm.var=false; e asm.lines=false; sf entry0; pd 3 @ 0x180001260' \
    ../rsrc/target.efi
0x180001260      lea   rdx, data.180002010                             ; 0x180002010 ; u"Uh oh!\r\n"
0x180001267      call  rax
0x180001269      jmp   0x1800012c4
```

Now we found the instruction we want. We'll save the call address `0x180001267` for
later as `TARGET_INSTRUCTION_ADDRESS`.

Finally, we want to find the address in the data section our off-limits data lives. We
know we write an `'X'` character to it, so we'll first search for that:

```sh
$ rizin -Aqqc "e asm.var=false; e asm.lines=false; sf entry0; pdf ~'X'" \
    ../rsrc/target.efi
0x1800012a3      mov   byte [rax + rcx], 0x58                          ; 'X'
```

Then we can disassemble backward to find where `rax` points to:

```sh
$ rizin -Aqqc "e asm.var=false; e asm.lines=false; sf entry0; pd -3 @ 0x1800012a3" \
    ../rsrc/target.efi
0x180001291      jae   0x1800012ba
0x180001297      mov   rcx, qword [var_c8h]
0x18000129c      lea   rax, section..data                              ; 0x180003000
```

`rax` points to the data section at `0x180003000`. We'll save this final address
as `TARGET_DATA_ADDRESS`, and it has a size `0x100` which we know because we wrote the
code!

## Set Breakpoints

Now that we have set up breakpoints as faults, we can set the actual breakpoints. Using
the SIMICS script from the [Fuzzing A UEFI Target](./UEFISimpleTarget.md) tutorial as a
base, we'll add a new script-branch that will fire when a magic instruction occurs
with the `MAGIC_START` value (`1`). The start harness is a good place to "synchronize"
our static knowledge of the UEFI application binary we are running with the dynamic
state of the application.

In this case, we'll use the start harness to determine the value of the instruction
pointer (IP) when the magic instruction signifying our start harness is executed. Then,
we can use the difference between the dynamic IP value and the address in our
application binary to find the address of both the instruction (in the third branch) and
memory region (accessed in the fourth branch) to breakpoint both the instruction
execution and the access of off-limits memory.

We'll add a script branch to our fuzz script, using the addresses we found earlier.


```simics
script-branch "Set breakpoints" {
    local $TARGET_HARNESS_ADDRESS = 0x180001082
    local $TARGET_INSTRUCTION_ADDRESS = 0x180001267
    local $TARGET_DATA_ADDRESS = 0x180003000

    local $MAGIC_START = 1
    bp.magic.wait-for $MAGIC_START

    # Create a context for the CPU to set virtual address breakpoints. If you only need
    # physical address breakpoints, you can just use `break`
    local $ctx = (new-context)
    board.mb.cpu0.core[0][0].set-context $ctx

    # Get the value of the IP and the offset from the known address (in this case, it
    # should be 0)
    local $rip = (board.mb.cpu0.core[0][0].read-reg rip)
    local $offset = $rip - $TARGET_HARNESS_ADDRESS

    # Set a breakpoint on the bad instruction
    local $harness_break_addr = ($TARGET_INSTRUCTION_ADDRESS + $offset)
    echo "Setting instruction harness on:"
    print -x $harness_break_addr
    $ctx.break -x $harness_break_addr

    # Set a breakpoint on the data memory range
    local $data_break_addr = ($TARGET_DATA_ADDRESS + $offset)
    echo "Setting data harness on:"
    print -x $data_break_addr
    $ctx.break -r -w $data_break_addr 0x100
}
```

## Fuzz for Breakpoints

Now, if we run the fuzzer (assuming you are in the
[examples/breakpoints](../examples/breakpoints/) directory, replace `6.0.169` with your
installed SIMICS base version and `6.0.70` with your installed SIMICS QSP version):

```sh
cargo run --manifest-path ../../Cargo.toml --release \
    --features=6.0.169 -- \
    --project ./project --input ./input --solutions ./solutions --corpus ./corpus \
    --log-level INFO --trace-mode once --executor-timeout 60 --timeout 3 --cores 1 \
    --package 2096:6.0.70 \
    --file "./src/target.efi:%simics%/target.efi" \
    --file "./rsrc/fuzz.simics:%simics%/fuzz.simics" \
    --file "./rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff" \
    --command 'COMMAND:run-script "%simics%/fuzz.simics"'
```

We'll eventually see some output like:

```text
  2023-08-03T19:47:35.634199Z  INFO tsffs_module::module::components::detector: Got breakpoint
    at tsffs_module/src/module/components/detector/mod.rs:280

  2023-08-03T19:47:35.645715Z  INFO simics_fuzz::fuzzer: Target got a breakpoint #2
    at simics-fuzz/src/fuzzer/mod.rs:493 on main ThreadId(1)

  2023-08-03T19:47:35.653409Z  INFO simics_fuzz::fuzzer: [Objective   #1]  (GLOBAL) run time: 0h-0m-12s, clients: 2, corpus: 4, objectives: 1, executions: 11, exec/sec: 2.594
    at simics-fuzz/src/fuzzer/mod.rs:626 on main ThreadId(1)
```

Indicating that we've hit out breakpoint during execution and an objective has been
found.