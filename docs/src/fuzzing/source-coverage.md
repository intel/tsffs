# Source Coverage for UEFI Targets

After a campaign on UEFI firmware, you usually want to know which source lines the
fuzzer reached and which it never touched. TSFFS does not compute that itself. SIMICS
already ships source-level code coverage (the `collect-coverage` command), so the report
is produced by running the inputs the campaign saved once more, in a separate SIMICS
session, while SIMICS collects coverage.

An *input* on this page is one file saved by the fuzzer, in the corpus or in the
solutions directory. The rest of this documentation calls it a *testcase*, the name it
has once TSFFS runs it.

This page gives the full recipe. It relies on three things:

- The *corpus*: the directory of inputs the fuzzer kept because each one reached code
  no earlier input had reached (`%simics%/corpus` unless you
  [changed it](../config/common-options.md#set-corpus-and-solutions-directory)).
  Running every input of the corpus again reaches the code the campaign reached.
- [Repro mode](analyzing-results.md), which runs one saved input from the snapshot TSFFS
  takes at the harness. Here it is called in a loop, once per input.
- The SIMICS *UEFI firmware tracker*, which tells SIMICS which firmware module is loaded
  at which address, so that executed instructions can be matched to the debug
  information of the right module.

## Why Coverage Is a Separate Pass

The coverage that guides the fuzzer is a fixed-size map, as in AFL. Each branch the
target takes is hashed, together with the branch taken before it, into one slot of the
map, and an occupied slot is what TSFFS counts as an
[Edge](../config/common-options.md#afl-statistics-and-the-instance-directory). The map
is cheap to maintain, but it is not a list of the jumps that were taken: several
branches can share a slot, and nothing in it names a source file or a line. Source
coverage needs the firmware's debug information and costs more at run time.

Collecting source coverage while fuzzing has two costs:

- The campaign slows down. With `collect-coverage` active during fuzzing, the campaign
  we measured ran about 2.6 times slower.
- The coverage data lives in the SIMICS process. If SIMICS or TSFFS crashes during a
  long campaign, everything collected so far is lost.

Replaying the corpus afterwards avoids both. The campaign runs at full speed and only
writes its corpus, as it always does. The report is rebuilt from the corpus whenever you
want it: after the campaign, while it is still running, or on another machine.

Each run of the replay script needs report directories that do not exist yet. The two
report commands refuse an existing output directory, and they only run at the end, so a
second run fails after the whole replay with
`Error: '<...>/coverage-lcov' is a directory`. Before each run, remove the two report
directories of the previous one (`coverage-lcov` and `coverage-html` in the script
below) or choose new ones.

## What You Need

- **The inputs to replay.** The corpus directory of the campaign, and the solutions
  directory if you also want the lines only reached by crashing or hanging inputs (see
  [Choosing the Inputs to Replay](#choosing-the-inputs-to-replay)).
- **Firmware built with debug information**, and its build tree. For each module, the
  tracker needs the module's `DEBUG` directory from the EDK2 `Build` tree. For a GCC
  build that directory holds the `.efi`, `.dll` and `.debug` files of the module. The
  source files are needed too, to render the report.
- **The UEFI firmware tracker** (the `uefi-fw-tracker` SIMICS module). The script below
  enables it. Its options are described in the [SIMICS documentation of the
  tracker](https://intel.github.io/tsffs/simics/analyzer-user-guide/uefi-fw-trk.html).
- **The replay helper**, `examples/source-coverage/replay.py` in the TSFFS repository
  ([view it on
  GitHub](https://github.com/intel/tsffs/blob/main/examples/source-coverage/replay.py)),
  copied into your SIMICS project. It defines the two functions the script calls.
- **Your fuzzing script.** The replay session must load the target, boot it and reach
  the harness exactly as the campaign does.

## The Replay Script

Start from a copy of your fuzzing script and make the four additions marked below. The
example is for a UEFI application, `test.efi`, started from the UEFI shell, with a
compiled-in harness. It was built in a container under `/edk2/HelloWorld`, and its build
tree was copied to `debug-info` in the SIMICS project.

```simics
load-module tsffs
init-tsffs

# The settings of the fuzzing script, unchanged
@tsffs.log_level = 2
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
@tsffs.timeout = 3.0
@tsffs.exceptions = [14, 6]

# Addition 1: load the helper and prepare the first input, before the target runs
run-script "%simics%/replay.py"
@tsffs_replay_arm("%simics%/corpus")

load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

# Addition 2: enable the UEFI firmware tracker
load-module uefi-fw-tracker
new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -load
qsp.software.enable-tracker

# The boot sequence of the fuzzing script, unchanged
script-branch {
    bp.time.wait-for seconds = 15
    qsp.serconsole.con.input "\n"
    bp.time.wait-for seconds = .5
    qsp.serconsole.con.input "FS0:\n"
    bp.time.wait-for seconds = .5
    local $manager = (start-agent-manager)
    qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test.efi") + "\n")
    bp.time.wait-for seconds = .5
    qsp.serconsole.con.input "test.efi\n"
}

# Boots the target. Ends with the simulation stopped at the harness.
run

# Addition 3: start coverage, replay every input, stop coverage
$cc = (collect-coverage context-query = "'UEFI Firmware'" -running)
$cc.add-path-map "/edk2/HelloWorld" (lookup-file "%simics%/debug-info")
@tsffs_replay("%simics%/corpus")
$cc.stop

# Addition 4: write the report
$cc.add-functions
$cc.add-source-info
@simenv.lcov = SIM_lookup_file("%simics%") + "/coverage-lcov"
@simenv.html = SIM_lookup_file("%simics%") + "/coverage-html"
$cc.lcov-output $lcov
$cc.html-report $html
```

Run it like a fuzzing script:

```sh
./simics --no-win --batch-mode coverage.simics
```

The helper prints one line per input, then a last line with the number of inputs:

```txt
TSFFS_REPLAY <i>/<n> <file name> <stop reason>
TSFFS_REPLAY done <n>
```

The stop reason is the value of `tsffs.last_stop_reason` when the input ended. It
contains `StopNormal` for an input that reached the stop harness, `Exception` and the
exception number for an input that raised one of `tsffs.exceptions`, and `Timeout` for
an input that ran longer than `tsffs.timeout`.

## What the Script Does

**The settings stay those of the campaign.** An input must end in the replay where it
ended while fuzzing. Keep the harness settings, `tsffs.timeout`, `tsffs.exceptions` and
any breakpoints of the fuzzing script, so that a crashing input stops at its exception
and a hanging input stops at the timeout instead of running on.

**`tsffs_replay_arm(directory)`** puts TSFFS in repro mode with the first input of the
directory, and sets `tsffs.repro_auto_continue = False`. Call it after `init-tsffs` and
before the target runs. With that setting, the first `run` ends when the target reaches
the harness: TSFFS takes its snapshot, writes the input into the target's buffer, logs
that the testcase is prepared and that it waits for an external resume, and leaves the
simulation stopped there.

**Coverage starts at that stop.** The boot is then not part of the report, and the
snapshot TSFFS restores for every input already exists. The context query
`'UEFI Firmware'` is the name the tracker gives to the firmware in the SIMICS debugger
(`list-debug-contexts` shows it). `-running` makes SIMICS collect on that firmware
context, which already exists at this point, instead of waiting for a new one to be
created. `collect-coverage` returns the coverage object the following commands are
called on (`$cc` here).

**`tsffs_replay(directory)`** runs every input, one after the other. For each one it
calls `tsffs.iface.fuzz.repro(file)`, which restores the snapshot and writes the input,
then resumes the simulation once. That one resume ends when the input ends: at the stop
harness, at an exception, or at the timeout. Restoring the snapshot rewinds the target,
not the coverage SIMICS has collected, so the coverage of all the inputs adds up.

**The inputs** are the regular files directly inside the directory, sorted by name.
Hidden files are skipped (the fuzzer keeps `.lafl_lock` and `.metadata` files next to
the corpus entries), and so are subdirectories. If the directory is missing or holds no
input, both functions raise a `ValueError` naming the directory.

A corpus written by a campaign holds every input twice, with identical bytes: once as
`<hash>` and once as `id_<n>_<hash>`. The replay runs both files. The coverage is the
same as with one of them, and the replay takes twice as long.

**The report commands** stop the collection, add function and source line information
to the collected data, and write it in two formats: LCOV in `coverage-lcov` and HTML in
`coverage-html`, both in the SIMICS project. Neither directory may exist before the
run, as explained in
[Why Coverage Is a Separate Pass](#why-coverage-is-a-separate-pass).

> **Note:** TSFFS starts its fuzzer when the harness is first reached, even in a session
> that only replays, and the fuzzer does not start without at least one input. If
> `tsffs.corpus_directory` (by default `%simics%/corpus`) is missing or empty, SIMICS
> exits at the harness with `No interesting cases found from inputs!`. Replaying the
> corpus from its usual place satisfies this. If you replay another directory in a
> project that has no corpus, point `tsffs.corpus_directory` at a directory with at
> least one input, or set `tsffs.generate_random_corpus = True`. See [Set Corpus and
> Solutions Directory](../config/common-options.md#set-corpus-and-solutions-directory)
> and [Enable Random Corpus
> Generation](../config/common-options.md#enable-random-corpus-generation).

## Mapping Build Paths

Debug information records paths as they were on the machine that built the firmware:
the path of each module's debug file, and the paths of the source files. SIMICS looks
for those paths on the machine that runs the replay.

No path map is needed when the build tree is still at the path it was built at, on the
same machine. Add one when that is not the case, for example when:

- the firmware was built in a container or on a build server, and the tree was copied
  out afterwards,
- the tree was moved, or the replay runs on another machine.

A path map replaces the start of the recorded paths by the place where the tree is now:

```simics
$cc.add-path-map "/edk2/HelloWorld" (lookup-file "%simics%/debug-info")
```

The first argument is the path at build time, the second one the path on this machine.
SIMICS uses the map both to find the debug files of the modules and to find the source
files, so the copy must keep the layout of the original tree below the mapped directory.
In the example, the files needed from `/edk2/HelloWorld` in the container are:

```txt
HelloWorld.c
Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld/HelloWorld/DEBUG/HelloWorld.efi
Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld/HelloWorld/DEBUG/HelloWorld.dll
Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld/HelloWorld/DEBUG/HelloWorld.debug
```

and `debug-info` holds these four files at the same relative paths. For a whole
firmware, map the root of the EDK2 workspace the same way. Several path maps can be
added, one command each, when sources and build output were moved separately.

Add the path maps before the report commands. If the report misses a source file you
expect, the path map is the first thing to check.

## Reading the Report

**HTML.** Open `coverage-html/index.html` in a browser. The first page lists the
directories of the source files SIMICS knows from the debug information, with their
coverage. The files are one level down, on the page of their directory. To have the
files listed on the first page instead, write the report with
`$cc.html-report $html -summary-per-file`.

The page of a file shows its source: executed lines in green, lines that could have been
executed but were not in red, and lines without code (comments, lines the compiler
optimized away, the continuation lines of a statement) in grey. A line counts as
executed as soon as one instruction compiled from it was executed. The report also links
to a disassembly view with the coverage of each function.

**LCOV.** `coverage-lcov` is a directory that holds the same data in the LCOV tracefile
format, a text format that other tools read: `genhtml` from the LCOV project, editor
plugins, and the coverage services of CI systems. There is one tracefile per module that
has debug information, named `<symbol file>_<load address>`: `HelloWorld.efi_dd99d000`
for the application above, `BoardBdsHookDxe.debug_dee9e000` for a module of a full
firmware. A module without debug information gets no tracefile.

In a tracefile, a record starts with `SF:` and the path of a source file, and each
`DA:<line>,<count>` entry below it is one executable line. A count of zero means the
line was not executed. An `FNDA:<count>,<function>` entry counts the entries into a
function. The function that contains the harness has a count of zero there
(`FNDA:0,UefiMain` for the application above) although its lines are executed, because
it was entered before coverage started.

When reading either report, keep in mind:

- The report covers what ran between the harness start and the end of each input. Code
  that only runs during boot is reported as not executed.
- Firmware is usually built with optimizations, so several source lines can share the
  same instructions, and some lines have no instruction of their own.
- Only modules with debug information appear in the report.

## Choosing the Inputs to Replay

`tsffs_replay` can be called several times before `$cc.stop`, and the coverage keeps
adding up. To include the inputs that crashed or timed out, replay the solutions
directory after the corpus:

```simics
@tsffs_replay("%simics%/corpus")
@tsffs_replay("%simics%/solutions")
$cc.stop
```

Add the `solutions` line only when the campaign found solutions. If that directory is
missing or holds no input, `tsffs_replay` raises a `ValueError` naming it and the script
ends there, before the report commands run.

Each input that times out runs for `tsffs.timeout` seconds of simulated time, so a
solutions directory full of hangs takes a while.

To get a report while the campaign is still running, copy the corpus directory and
replay the copy in a separate SIMICS project, so that the replay session never works in
the directory the campaign is writing to.

## Limits

- **Firmware built with Visual Studio needs a Windows host.** Its debug information is
  in PDB files, which SIMICS only reads on Windows, where it needs `DBGHELP.DLL`.
- **System Management Mode (SMM) code** is only covered when the tracker tracks SMM
  modules, which needs both its SMM tracking and its execution tracking (the
  `smm_tracking` and `exec_tracking` parameters of the tracker). See the [SIMICS tracker
  docs](https://intel.github.io/tsffs/simics/analyzer-user-guide/uefi-fw-trk.html).
- **SIMICS versions.** The recipe was run on the EDK2 test application of the TSFFS test
  suite with SIMICS 6.0.185, 7.70.0 and 7.113.0, and on a full debug firmware with
  SIMICS 7.113.0 only.
- **One core.** The recipe is tested on a target with a single processor core.
- **Harness type.** The replay loop is tested with compiled-in start harnesses, which
  pass a buffer to TSFFS (`HARNESS_START`), and with
  [`start_without_buffer`](../harnessing/manual.md) called from a callback on the magic
  instruction of the start harness. With `start_without_buffer`, TSFFS has no buffer to
  write the input to: the restored snapshot reaches the start again, where your script
  must call `start_without_buffer` again, as it does while fuzzing, and the call returns
  the input given to the latest `repro`. Calling `repro` repeatedly is not tested with
  the other [manual start methods](../harnessing/closed-box.md) of the `fuzz` interface
  (`start_with_buffer_ptr_size_ptr`, `start_with_buffer_ptr_size_value` and
  `start_with_buffer_ptr_size_ptr_value`).
- **No fuzzing in the same session.** As in any repro session, fuzzing cannot be resumed
  after the replay. Use a separate SIMICS session for the report.
