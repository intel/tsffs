# Reproducing Runs

It is unlikely you'll find any bugs with this harness (if you do, report them to edk2!),
but we can still test the
["repro" functionality](../../fuzzing/analyzing-results.md)
which allows you to replay an execution of a testcase from an input file.

## Listing and Examining Testcases

After pressing Ctrl+C during execution, list the corpus files (tip: `!` in front of a
line in the SIMICS console lets you run shell commands):

```txt
simics> !ls corpus
0
1
2
3
4385dc33f608888d
5b7dc5642294ccb9
```

You will probably have several files. Let's examine testcase `4385dc33f608888d`:

```txt
simics> !hexdump -C corpus/4385dc33f608888d | head -n 2
00000000  30 82 04 e8 30 82 04 53  a0 03 02 01 02 02 1d 58  |0...0..S.......X|
00000010  74 4e e3 aa f9 7e e8 ff  2f 67 53 31 6e 62 3d 1e  |tN...~../gS1nb=.|
```

We can tell the fuzzer that we want to run with this specific input by using:

```txt
simics> @tsffs.iface.fuzz.repro("%simics%/corpus/4385dc33f608888d")
```

> **Note:** You can change the testcase you are examining by choosing a different one
> with `tsffs.iface.fuzz.repro`, but you cannot resume fuzzing after entering repro
> mode due to inconsistencies with the simulated system clock.

## Inspecting State with SIMICS Reverse Debugging

By default, the simulation runs the testcase through to completion and saves a bookmark
at the point the harness was triggered. You can then replay the execution by running:

```txt
simics> reverse-to start
```

From here, you can examine memory and registers (with `x`), single step execution (`si`)
and more! Check out the SIMICS documentation and explore all the deep debugging
capabilities that SIMICS offers. When you're done exploring, run `c` to continue.

## Debugging Live with a GDB Stub

Reverse debugging is great for inspecting state after a testcase has run. If you instead
want to step through execution as it happens (setting breakpoints, watching registers
change, using a familiar GDB workflow), you can attach a GDB stub before the testcase
executes.

**Step 1: Enable repro mode without auto continue.**

Add the following to your SIMICS script:

```python
@tsffs.iface.fuzz.repro("%simics%/corpus/4385dc33f608888d")
@tsffs.repro_auto_continue = False
```

With this set, when the harness start (`HARNESS_START`) is hit, TSFFS will write the
testcase into the target's buffer and pause the simulation there, waiting for you to
resume manually.

```c
  if (!Input) {
    return EFI_OUT_OF_RESOURCES;
  }

  HARNESS_START(Input, &InputSize);  // <-- Simulation is paused here

  Print(L"Input: %p Size: %d\n", Input, InputSize);
```

**Step 2: Start the GDB stub.**

SIMICS exposes a GDB remote stub via the `new-gdb-remote` command. Start it by
appending it to your invocation on the command line so it runs after your script
finishes loading:

```sh
./simics run.simics -e new-gdb-remote
```

Or add it at the end of your script directly, after the `run` line:

```txt
new-gdb-remote
```

SIMICS output:

```txt
[tsffs info] Stopped for repro. Restore to start bookmark with 'reverse-to start'
No CPU is specified; using current processor.
[gdb0 info] Attached to CPU: qsp.mb.cpu0.core[0][0]
Warning: This can expose the target system on the host local network.
[gdb0 info] Awaiting GDB connections on port 9123.
[gdb0 info] Connect from GDB using: "target remote localhost:9123"
```

**Step 3: Connect from your GDB client.**

From a separate terminal, connect to the stub. You now have full GDB control over the
simulated target. Set breakpoints, step through instructions, inspect memory and
registers. The simulation will only advance when you tell it to.

```sh
(gdb) target remote :9123
Remote debugging using :9123
warning: No executable has been specified and target does not support
determining executable automatically.  Try using the "file" command.
0x00000000dd5b7c8c in ?? ()
(gdb) bt
#0  0x00000000dd5b7c8c in ?? ()
#1  0x0000000000000000 in ?? ()
(gdb) x/10i $rip
=> 0xdd5b7c8c:  cpuid
   0xdd5b7c8e:  incq   -0x128(%rbp)
   0xdd5b7c95:  jne    0xdd5b7743
   0xdd5b7c9b:  lea    0x2a6a4(%rip),%rcx        # 0xdd5e2346
   0xdd5b7ca2:  call   0xdd5b5af9
   0xdd5b7ca7:  mov    0x4f6a2(%rip),%rax        # 0xdd607350
   0xdd5b7cae:  mov    $0x1,%edx
   0xdd5b7cb3:  mov    -0x118(%rbp),%rcx
   0xdd5b7cba:  call   *0x30(%rax)
   0xdd5b7cbd:  test   %rax,%rax
(gdb)
```

## Source-Level Debugging

Once connected with a GDB stub, you can go further and load debug symbols so GDB shows
C source lines, function names, and local variables instead of raw addresses. This
requires two additional pieces of setup: OS awareness in the SIMICS script, and loading
the symbols at the correct address in GDB.

**Step 1: Add OS awareness to the script.**

Add the following block to your `run.simics` right after the `load-target` line:

```txt
new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -load
qsp.software.enable-tracker
```

This instructs SIMICS to track which EFI modules are loaded and at what addresses.

**Step 2: Find the load address of `Tutorial.efi`.**

After the simulation has started and the target has booted, run in the SIMICS console:

```txt
simics> qsp.software.tracker.list-modules max = 100
```

Look for `Tutorial.efi` in the output:

```txt
│   85│Tutorial.efi  │    0xdd548000│ 0xc3b80│
```

The `Loaded Address` column gives the base address of the module (`0xdd548000` in this
example).

**Step 3: Find the `.text` section RVA.**

On your host, inspect the debug file produced by the build:

```sh
$ objdump -h project/Tutorial.debug
project/Tutorial.debug:     file format elf64-x86-64

Sections:
Idx Name          Size      VMA               LMA               File off  Algn
  0 .text         0009a9d4  0000000000000240  0000000000000240  00000100  2**6
  1 .data         ...
```

The VMA of the `.text` section is `0x240`. Since EFI binaries are position-independent,
this equals the RVA to add to the load address.

**Step 4: Load symbols in GDB.**

In your GDB session, load the debug file using the load address from step 2 and the
`.text` VMA from step 3:

```sh
(gdb) add-symbol-file project/Tutorial.debug 0xdd548000+0x240
```

GDB will now resolve addresses to source lines and function names. You can set
breakpoints by function or file location, step through C source, and inspect named
variables.

**Step 5: Map source paths.**

The debug symbols reference the paths as they existed inside the Docker build container
(rooted at `/edk2`). The build script can copy those sources to your host under
`edk2-uefi/edk2/` when run with `COPY_SOURCES=1`:

```sh
COPY_SOURCES=1 ./build.sh
```

Then tell GDB how to map the container paths to your local copy:

```sh
(gdb) set substitute-path /edk2 /absolute/path/to/examples/tutorials/edk2-uefi/edk2
```

GDB will now find source files automatically when stepping through code.

```sh
(gdb) bt
#0  0x00000000dd5b89c6 in UefiMain (SystemTable=<optimized out>, ImageHandle=<optimized out>) at /edk2/Tutorial/Tutorial.c:58
#1  ProcessModuleEntryPointList (SystemTable=<optimized out>, ImageHandle=<optimized out>) at /edk2/Tutorial/Build/CryptoPkg/All/DEBUG_GCC/X64/Tutorial/Tutorial/DEBUG/AutoGen.c:319
#2  _ModuleEntryPoint (ImageHandle=<optimized out>, SystemTable=<optimized out>) at /edk2/MdePkg/Library/UefiApplicationEntryPoint/ApplicationEntryPoint.c:58
#3  0x00000000df32e14c in ?? ()
#4  0x00000001df322a88 in ?? ()
#5  0x00000000df322a98 in ?? ()
#6  0x00000000df322a20 in ?? ()
#7  0x0000000000000000 in ?? ()
(gdb) list .
53    BOOLEAN Status = X509VerifyCert(Cert, CertSize, CACert, CACertSize);
54
55    if (Status) {
56      HARNESS_ASSERT();
57    } else {
58      HARNESS_STOP();
59    }
60
61    if (Input) {
62      FreePages(Input, EFI_SIZE_TO_PAGES(MaxInputSize));
(gdb) 
```
