# Spec: UEFI source coverage by replaying the corpus under Simics coverage

<!-- markdownlint-disable MD013 -->

## Problem Statement

A TSFFS user fuzzing UEFI firmware cannot get a source-level coverage report. The previous attempt (the first version of intel/tsffs#389) added a DWARF parser, a reader for the Simics UEFI tracker's internal `maps` attribute and per-instruction attribution inside TSFFS. Validation on Simics 6.0.185 and 7.113.0 showed that it files inlined lines under the wrong source file (the fuzzed file can be absent from the report), never sees SMM modules, and slows a full-firmware campaign from 53 to 9 executions per second.

Simics already ships source coverage (`collect-coverage`) that reads the same debug files correctly, covers SMM modules, and works with TSFFS's snapshot restores on both Simics versions. Collecting it during a campaign has two costs: the campaign runs about 2.6 times slower, and a crash of TSFFS or Simics loses everything collected so far.

## Solution

Keep coverage out of the fuzzing run. The campaign fuzzes at full speed and saves its corpus to disk as it does today. A separate Simics run then replays the saved inputs one by one while Simics collects coverage, and writes the report.

No new TSFFS feature is needed for the replay: a script loops over the saved inputs and calls the existing `tsffs.iface.fuzz.repro(file)` for each. TSFFS stays unaware of coverage.

A crash during the campaign loses no coverage, because the report is rebuilt from the corpus on demand: during the campaign, after it, or on another machine.

## Approaches considered

1. **TSFFS parses DWARF and attributes coverage itself** (first version of #389). Rejected: wrong-file attribution, no SMM modules, a large parser to maintain, and it depends on an undocumented tracker attribute.
2. **TSFFS drives `collect-coverage` during the campaign.** Rejected: slows fuzzing, loses data on a crash, and needs a way to tell a long campaign when to write the report.
3. **A dedicated corpus replay mode in TSFFS.** Rejected: a script loop over the existing `repro` does the same job, as checked below.
4. **A script loop over `repro` plus a coverage recipe** (chosen). Fuzzing speed is unaffected, nothing is lost on a crash, and TSFFS changes by one small fix.

## What was checked

On Simics 6.0.185 and 7.113.0, with the EDK2 test application and the UEFI firmware tracker:

1. A loop calling `repro(file)` then resuming, over six inputs (two normal paths, a page fault, a timeout, an invalid opcode, a normal path again), ran every input and reported the right stop reason for each.
2. `collect-coverage` started with `-running` after TSFFS had taken its snapshot survived the snapshot restores, and the LCOV output credited the lines of every path to `HelloWorld.c`.
3. No debugger errors were logged on Simics 7.

That run had fuzzed a few iterations before the loop. Without them the loop stops working after the first input: `repro` only restores the snapshot and writes the next input when at least one fuzzing iteration has completed.

## The TSFFS fix

`repro` can be called again after a `repro` run in a session that never fuzzed. It restores the snapshot and writes the new input whenever the snapshot exists, not only after a fuzzing iteration.

With `repro_auto_continue = False`, which already exists, each call leaves the simulation stopped with the input prepared, so a script decides when to resume.

## UEFI source coverage recipe

Documented, with the loop shipped as an example script:

1. Enable the Simics UEFI firmware tracker and set `repro_auto_continue = False`.
2. Call `repro` on the first input and run. TSFFS stops at the harness with its snapshot taken and the input prepared.
3. Start `collect-coverage` on the firmware context with `-running`, and add a path map when the firmware build tree is not at its original path. Starting here keeps the boot out of the report.
4. Loop over the inputs: `repro(file)`, resume until the input has run.
5. Write the report with `lcov-output` and `html-report`.

## Testing

1. A CI test on Simics 6.0.185 and on the latest Simics that runs the recipe over a small fixed set of inputs, including one that crashes and one that times out, with no fuzzing before the loop. It checks that every input ran and that the LCOV output credits the expected lines to `HelloWorld.c`.
2. The EDK2 test application's build script exports the application's debug files, which the test needs.

## Documentation

A new page for source coverage of UEFI targets with the recipe, the path map, and the Windows-host limitation for firmware built with Visual Studio. The `repro` documentation mentions that it can be called repeatedly.

## Out of scope

1. The two `save_symbolic_coverage` fixes from the first version of #389. They concern the existing Windows coverage and go in their own pull request.
2. TSFFS failing to load on Simics 7.113.0 because of the Python version it links against.
3. The EDK2 platform tutorial firmware that does not boot at its pinned revisions.
4. Coverage collected during a live campaign.

## Left to the implementer

1. Whether a resumed `repro` still needs two resumes per input. Today the restored snapshot re-executes the harness start, so TSFFS stops once more before the input runs.
2. Whether the fuzzer thread is still started in a `repro`-only session. Today it is, which requires a non-empty corpus directory or `generate_random_corpus`.
