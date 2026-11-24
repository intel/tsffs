# Spec: UEFI source coverage by replaying the corpus under Simics coverage

<!-- markdownlint-disable MD013 -->

## Problem Statement

A TSFFS user fuzzing UEFI firmware cannot get a source-level coverage report. The previous attempt (the first version of intel/tsffs#389) added a DWARF parser, a reader for the Simics UEFI tracker's internal `maps` attribute and per-instruction attribution inside TSFFS. Validation on Simics 6.0.185 and 7.113.0 showed that it files inlined lines under the wrong source file (the fuzzed file can be absent from the report), never sees SMM modules, and slows a full-firmware campaign from 53 to 9 executions per second.

Simics already ships source coverage (`collect-coverage`) that reads the same debug files correctly, covers SMM modules, and works with TSFFS's snapshot restores on both Simics versions. Collecting it during a campaign has two costs: the campaign runs about 2.6 times slower, and a crash of TSFFS or Simics loses everything collected so far.

## Solution

Keep coverage out of the fuzzing run. The campaign fuzzes at full speed and saves its corpus to disk as it does today. A separate Simics run then replays the saved inputs while Simics collects coverage, and writes the report.

The replay is one call: `tsffs.iface.fuzz.repro(directory)`. TSFFS runs every testcase of the directory from the initial snapshot and stops after the last one. TSFFS stays unaware of coverage.

A crash during the campaign loses no coverage, because the report is rebuilt from the corpus on demand: during the campaign, after it, or on another machine.

## Approaches considered

1. **TSFFS parses DWARF and attributes coverage itself** (first version of #389). Rejected: wrong-file attribution, no SMM modules, a large parser to maintain, and it depends on an undocumented tracker attribute.
2. **TSFFS drives `collect-coverage` during the campaign.** Rejected: slows fuzzing, loses data on a crash, and needs a way to tell a long campaign when to write the report.
3. **A script loop over `repro(file)`** (first version of this work). It needed a Python helper with two functions (one called before the target runs, one after), and `repro` grew a second copy of what the fuzzing loop does between two iterations, with a special case in `stop_simulation` so that the harness start reached again did not stop the simulation. Rejected for the interface it gave the user and for the duplicated mechanics.
4. **`repro` replays through the fuzzing loop** (chosen). Reproducing is the fuzzing loop with another source of testcases, so it reuses that loop and takes a directory. No helper, and less code than before.

## Design

### Interface

1. `repro(path)` takes a file or a directory. The testcases of a directory are the regular files directly inside it, sorted by name. Hidden files are skipped (the fuzzer keeps lock and metadata files next to the corpus entries), and so are subdirectories. A path without testcases is an error at the call.
2. TSFFS runs the testcases one after the other, each from the initial snapshot, and logs `Replayed <file>: <stop reason>` when each one ends. The simulation stays stopped after the last one, in the state that testcase ended in. With a single file this is what `repro` always did.
3. `repro` can be called again once the reproduced executions have stopped, in a session that fuzzed or in one that never did.
4. `repro_auto_continue` keeps its name and default. Set to `False`, it holds the first testcase of a `repro` call at the harness start: when the snapshot is taken, or when a later call has restored it. The following testcases of a directory run without a stop in between. The value at the time of the call counts, not the one the restored snapshot brings back.
5. A session that calls `repro` before the target runs does not start the fuzzer, so it needs no corpus.

### Mechanics

1. One piece of state replaces the three repro flags: the queue of testcases still to run. It is absent while fuzzing, and empty once the last testcase has ended, from when on TSFFS ignores stops.
2. `get_testcase` returns the front of the queue instead of asking the fuzzer.
3. `finish_iteration` handles the end of a replayed testcase: log it, then either stay stopped (queue empty) or restore the snapshot, write the next testcase and arm the timeout. Restoring, writing and arming are one function, shared with the fuzzing path and with `repro` called on an existing snapshot.
4. A harness start reached again (with `start_without_buffer`, the restored snapshot executes it for every testcase) resumes, as it does while fuzzing. Only the start that prepares the first testcase can wait for an external resume.
5. On Simics 6 the `start` bookmark is set when the snapshot is taken in a session that reproduces.

The replay always restores the snapshot between two testcases, whatever `snapshot_restore_interval` is: each saved input is reproducible by itself, and the order of the corpus is not the order of the campaign.

## UEFI source coverage recipe

1. Set `repro_auto_continue = False` and call `repro` on the corpus directory, before the target runs.
2. Enable the Simics UEFI firmware tracker and run. TSFFS stops at the harness with its snapshot taken and the first input prepared.
3. Start `collect-coverage` on the firmware context with `-running`, and add a path map when the firmware build tree is not at its original path. Starting here keeps the boot out of the report.
4. Run. Every input is executed, and the run ends after the last one.
5. Stop coverage and write the report with `lcov-output` and `html-report`.

To add the solutions, call `repro` on that directory and run once more before stopping coverage.

## Testing

Two tests on the latest Simics, both in sessions that never fuzzed. Simics 6.0.185 and 7.113.0 run the same scripts by hand.

1. The recipe on the EDK2 test application, with no corpus directory: the first run stops at the harness, the second executes six inputs (normal paths, a page fault, a timeout, an invalid opcode, an oversized input) in order and skips the hidden file and the subdirectory, the LCOV output credits the line of each path to `HelloWorld.c`, and the HTML report is written. Two more `repro` calls on single files then check that a prepared testcase does not report the stop of the previous one, and that `repro_auto_continue` set after the snapshot is the one that counts.
2. The directory replay with `start_without_buffer`.

The EDK2 test application's build script exports the application's debug files, which the first test needs.

Checked by hand, not in CI: `repro` on a directory after fuzzing to an iteration limit, `reverse-to start` on Simics 6 after a replay, and the recipe on a full debug firmware (Simics 7.113.0), whose report is identical to the one of the script loop on the same corpus.

## Documentation

A page for source coverage of UEFI targets with the recipe, the path map, how to read the report and the limits. The `repro` documentation describes directories and repeated calls.

## Out of scope

1. The two `save_symbolic_coverage` fixes from the first version of #389. They concern the existing Windows coverage and go in their own pull request.
2. TSFFS failing to load on Simics 7.113.0 because of the Python version it links against.
3. The EDK2 platform tutorial firmware that does not boot at its pinned revisions.
4. Coverage collected during a live campaign.
5. Passing the pause as an argument of `repro` instead of the `repro_auto_continue` attribute. It would remove the interaction with the snapshot, but it changes the signature of an interface method, which breaks every caller.

## Known limits

1. A missing or empty directory ends the Simics session at the `repro` call, as any error of a `fuzz` interface method does.
2. On Simics 7, with `collect-coverage` active, the debugger logs two `[tcf error]` lines between two inputs, as it does when coverage is collected during fuzzing. The report is not affected.
3. The replay is not tested with the buffer-based manual start methods (`start_with_buffer_ptr_size_ptr`, `start_with_buffer_ptr_size_value`, `start_with_buffer_ptr_size_ptr_value`).
