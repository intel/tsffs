# Common Options

TSFFS provides a set of common options that are usable no matter what type of harnessing
is desired.

- [Common Options](#common-options)
  - [Solution Configuration](#solution-configuration)
    - [Setting the Timeout](#setting-the-timeout)
    - [Setting Exception Solutions](#setting-exception-solutions)
    - [Setting Breakpoint Solutions](#setting-breakpoint-solutions)
  - [Fuzzer Settings](#fuzzer-settings)
    - [Using CMPLog](#using-cmplog)
    - [Set Corpus and Solutions Directory](#set-corpus-and-solutions-directory)
    - [Enable and Set the Checkpoint Path](#enable-and-set-the-checkpoint-path)
    - [Enable Random Corpus Generation](#enable-random-corpus-generation)
    - [Set an Iteration Limit](#set-an-iteration-limit)
    - [Set Snapshot Restore Interval](#set-snapshot-restore-interval)
    - [Adding Tokens From Target Software](#adding-tokens-from-target-software)
    - [Setting an Architecture Hint](#setting-an-architecture-hint)
    - [Adding a Trace Processor](#adding-a-trace-processor)
    - [Disabling Coverage Reporting](#disabling-coverage-reporting)
    - [Enable Logging and Set Log path](#enable-logging-and-set-log-path)
    - [Keep All Corpus Entries](#keep-all-corpus-entries)
    - [Use Initial Buffer Contents As Corpus](#use-initial-buffer-contents-as-corpus)
    - [Disable Auto-Continue in Repro Mode](#disable-auto-continue-in-repro-mode)

## Solution Configuration

TSFFS can be configured to treat various events as
[solutions](../README.md#terminology).

### Setting the Timeout

To set the number of seconds in virtual time until an iteration is considered *timed out*,
use the following (for example, to set the timeout to 3 seconds):

```python
@tsffs.timeout = 3.0
```

Note that this timeout is in virtual time, not real time. This means that whether the
simulation runs faster or slower than real time, the timeout will be accurate to the
target software's execution speed.

### Setting Exception Solutions

The primary way TSFFS detects bugs is via CPU exceptions that are raised, but should not
be. For example, when fuzzing a user-space application on x86 a General Protection Fault
(GPF) (#13) tells the fuzzer that a crash has occurred, or when fuzzing a UEFI
application a Page Fault (#14) tells the fuzzer that a crash has occurred.

Each CPU model has different exceptions (e.g. RISC has different codes than x86), but
SIMICS represents all exceptions as an integer. An exception number can be added as a
tracked condition that will cause the fuzzer to consider an exception (in this example,
GPF #13) as a solution with:

```python
@tsffs.exceptions = [13]
```

An already-added exception can be removed from the tracked set that are considered
solutions with:

```python
@tsffs.exceptions.remove(13)
```

In addition, if *all* exceptions should be considered as solutions, use:

```python
@tsffs.all_exceptions_are_solutions = True
```

Note that this is typically not useful in practice. With all exceptions set as
solutions, all exceptions including innocuous exceptions like timer interrupts will
cause solutions. It is mainly useful for embedded models running short code paths like
when fuzzing interrupt handlers themselves, where any exception occurring is truly an
error.

### Setting Breakpoint Solutions

SIMICS provides several ways of setting breakpoints, for example below shows setting a
breakpoint when a CPU writes to a specific range of memory:

```simics
local $ctx = (new-context)
local $BREAK_BUFFER_ADDRESS = 0x400000
local $BREAK_BUFFER_SIZE = 0x100
qsp.mb.cpu0.core[0][0].set-context $ctx
local $bp_number = ($ctx.break -w $BREAK_BUFFER_ADDRESS $BREAK_BUFFER_SIZE)
```

Breakpoints have numbers, which you can add and remove from the set of breakpoints
the fuzzer treats as solutions with:

```python
@tsffs.breakpoints = [2]
@tsffs.breakpoints.remove()
```

Note that when setting a breakpoint via a Simics command, like:

```simics
local $bp_number = ($ctx.break -w $BREAK_BUFFER_ADDRESS $BREAK_BUFFER_SIZE)
```

The variable `bp_number` can be added to the set of solution breakpoints by accessing
the `simenv` variable, like:

```python
@tsffs.breakpoints += [simenv.bp_number]
```

If not specifying a breakpoint number, breakpoints can be set as solutions with:

```python
@tsffs.all_breakpoints_are_solutions = True
```

This is useful when testing code that is not allowed to write, read, or execute specific
code. For example, userspace code should typically not execute code from its stack or
heap.

## Fuzzer Settings

### Using CMPLog

Comparison logging greatly improves the efficiency of the fuzzer by making each
iteration more likely to progress through sometimes difficult-to-solve checks. It logs
values that are compared against during execution and uses them to mutate the input.

Comparison logging is enabled by default. It can be disabled with:

```python
@tsffs.cmplog = False
```

### Set Corpus and Solutions Directory

By default, the corpus will be taken from (and written to) the directory "%simics%/corpus".

Initial test cases should be placed in this directory.

The directory test cases are taken from and written to can be changed with:

```python
@tsffs.corpus_directory = SIM_lookup_file("%simics%/other_corpus_directory")
```

Note the directory must exist. Likewise, the directory solutions are saved to can be
changed with:


```python
@tsffs.solutions_directory = SIM_lookup_file("%simics%/other_solutions_directory")
```

### Enable and Set the Checkpoint Path

The fuzzer captures an on-disk checkpoint before starting fuzzing by default. On Simics
7 and higher, this increases the snapshot restore speed very significantly, so it should
only be disabled if required.

To disable this behavior, you can set:

```python
@tsffs.pre_snapshot_checkpoint = False
```

To set the path for the checkpoint, you can set:

```python
@tsffs.checkpoint_path = SIM_lookup_file("%simics%") + "/checkpoint.ckpt"
```

### Enable Random Corpus Generation

For testing, the fuzzer can generate an initial random corpus for you. This option
should *not* be used for real fuzzing campaigns, but can be useful for testing.

In real campaigns, a representative corpus of the input triggering both the error and
non-error paths of the software under test should be placed in the `%simics%/corpus`
directory (or the directory specified with [the
API](#set-corpus-and-solutions-directory)).

This can be enabled with:

```python
@tsffs.generate_random_corpus = True
```

The size of the initial random corpus can be set via (note, larger random corpuses are
generally not useful and a real corpus matching the expected data format should be used
instead!):

```python
@tsffs.initial_random_corpus_size = 64
```

### Set an Iteration Limit

The fuzzer can be set to execute only a specific number of iterations before exiting.
This is useful for CI fuzzing or for testing. The limit can be set with:

```python
@tsffs.iteration_limit = 1000
```

### Set Snapshot Restore Interval

By default, TSFFS restores the initial snapshot at every normal iteration boundary.
This can be changed to support semi-persistent or fully persistent execution:

```python
# Default behavior: restore every iteration
@tsffs.snapshot_restore_interval = 1

# Semi-persistent: restore every 100 iterations
@tsffs.snapshot_restore_interval = 100

# Fully persistent: never restore after startup
@tsffs.snapshot_restore_interval = 0
```

Values greater than 1 restore every N iterations, where N is the configured
value. This cadence is based on the global iteration count.
This option only accepts integer values (`0`, `1`, or `N > 1`).

Discovered solutions, including timeouts, exceptions, and
breakpoint-triggered solutions, always restore the initial snapshot before
the next iteration resumes if one exists. `snapshot_restore_interval`
controls only the restore behavior for normal iteration boundaries.

When using values other than `1`, the harness must use a loop so that execution
reaches the next `HARNESS_STOP` without relying on a snapshot restore. See
[Semi-Persistent and Fully Persistent Execution](../harnessing/compiled-in.md#semi-persistent-and-fully-persistent-execution).

### Adding Tokens From Target Software

The fuzzer has a mutator which will insert, remove, and mutate tokens in testcases. This
allows the fuzzer to much more easily pass checks against strings and other short
sequences. In many cases, especially for text based protocols, this is an extremely
large improvement to fuzzer performance, and it should always be used where possible.

The fuzzer provides methods for adding tokens from executable files, source files, and
dictionary files. Executable files can be PE/COFF (i.e. UEFI applications or Windows
applications) or ELF (i.e. unpacked kernel images or Linux applications).

To add tokens from an executable file:

```python
@tsffs.token_executables += [SIM_lookup_file("%simics%/test.efi")]
```

Tokens from source files are extracted in a best-effort language-independent way.
Multiple source files can be added.

```python
@tsffs.token_src_files += [
  "/home/user/source/test.c",
  "/home/user/source/test_lib.c",
  "/home/user/source/test.h"
]
```

Dictionary files are given in the same format as AFL and LibFuzzer:

```txt
token_x = "hello"
token_y = "foo\x41bar"
```

Token dictionaries can be created manually, or tokens can be extracted from source files
more accurately than the built-in executable tokenizer using some existing tools:

* [autodict-ql](https://github.com/AFLplusplus/AFLplusplus/tree/85c5b5218c6a7b2289f309fbd1625a5d0a602a00/utils/autodict_ql)
* [AFL++ dict2file pass](https://github.com/AFLplusplus/AFLplusplus/blob/stable/instrumentation/README.llvm.md#5-bonus-feature-dict2file-pass)

Once created, the tokens from these dictionaries can be added to the fuzzer with:

```python
@tsffs.token_files += [SIM_lookup_file("%simics%/token-file.txt")]
```

### Setting an Architecture Hint

Some SIMICS models may not report the correct architecture for their CPU cores. When not
correct, setting an architecture hint can be useful to override the detected
architecture for the core. this is mostly useful for architectures that report `x86-64`
but are actually `i386`, and for architectures that are actually `x86-64` but are
running `i386` code in backward-compatibility mode.

An architecture hint can be set with:

```python
@tsffs.iface.config.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")
```

### Adding a Trace Processor

By default, only the processor core that either executes the start harness or is passed
to the [manual start API](../harnessing/closed-box.md) is traced during execution. When fuzzing
code running on multiple cores, the additional cores can be added with:

```python
@tsffs.iface.config.add_trace_processor(qsp.mb.cpu0.core[0][1])
```

### Disabling Coverage Reporting

By default, the fuzzer will report new interesting control flow edges. This is
normally useful to check the fuzzer's progress and ensure it is finding new
paths. However in some cases, output may not be needed, so coverage reporting
can be disabled with:

```python
@tsffs.coverage_reporting = False
```

### AFL++ Statistics and the Instance Directory

Each fuzzing session keeps AFL++-format statistics current while the campaign runs.
They live in an *Instance* directory, `<output_directory>/<instance_name>/`. By
default that is `output/default/` in the Simics project, and it contains:

- `fuzzer_stats`: the current state of the campaign as `key : value` lines, replaced
  atomically on every write so a reader never sees a partial file.
- `plot_data`: a time series with one row appended on every write, in the AFL++
  15-column layout (`relative_time`, `cycles_done`, `cur_item`, `corpus_count`,
  `pending_total`, `pending_favs`, `map_size`, `saved_crashes`, `saved_hangs`,
  `max_depth`, `execs_per_sec`, `total_execs`, `edges_found`, `total_crashes`,
  `servers_count`). Existing AFL++ plotting scripts and notebooks work unmodified, and
  `afl-plot` renders it. `map_size` holds the coverage percentage, as in AFL++.
- `fuzzer_setup`: written once per session. It lists the session's configuration as
  `name=value` lines, followed by a `# command line:` line with the Simics command line.
  `afl-whatsup` discovers an Instance by finding this file.
- `.lock`: the Instance lock, described below.

`fuzzer_stats` is rewritten every `afl_stats_interval` seconds of wall-clock time
(checked each time the simulation stops, so a very long iteration delays the next
write) and once more when the session shuts down. Writing statistics can never stop
fuzzing: if a write fails, TSFFS logs a warning once, keeps fuzzing, and logs once more
when writing recovers. Keys TSFFS cannot know, such as `slowest_exec_ms`, are omitted
rather than reported as zero.

The following attributes control this behavior:

```python
# Turn statistics off entirely. No Instance directory is created.
@tsffs.afl_stats = False

# Write statistics every 30 seconds instead of every 10
@tsffs.afl_stats_interval = 30

# Put Instances somewhere other than %simics%/output
@tsffs.output_directory = SIM_lookup_file("%simics%") + "/my-output"

# Name the Instance (default: "default")
@tsffs.instance_name = "nightly-1"
```

An `instance_name` may contain only letters, digits, `.`, `_` and `-`, and may not
consist only of dots. Anything else fails the session at startup with an error naming
the allowed characters.

When a session starts on an Instance that already holds a `plot_data` from an earlier
session, that file is renamed to `plot_data.<unix_timestamp>` and a fresh `plot_data` is
started, so two sessions are never mixed into one graph and no history is lost. A stale
`fuzzer_stats` is simply replaced. Rotation only happens after the session holds the
Instance lock, so a refused second session can never rotate a live session's files.

The Instance is named `default` unless you name it, as AFL++ does, so repeated repro and
debugging sessions reuse one directory instead of littering the output directory.

At startup TSFFS takes an exclusive lock on `<instance>/.lock` and holds it for the
session. A second live session that targets the same Instance fails at startup with an
error saying the Instance is in use. **Sessions running in parallel need distinct
`instance_name` values.** The operating system releases the lock when the process exits,
so a session that crashed never blocks the next one and there is nothing to clean up by
hand.

`fuzzer_stats` reports **Edges**: occupied slots of the fuzzer's coverage map, which is
what `edges_found` means for AFL++. `bitmap_cvg` is the share of the map they occupy and
never exceeds 100%, and `total_edges` is the map size. These keys are reported whether or
not `coverage_reporting` is enabled. `stability` is computed as AFL++ does, the share of
occupied slots that calibration found stable, and reads 100% until calibration has run;
`var_byte_count` is the number of unstable slots. `corpus_count` and `edges_found` are
kept current between fuzzing rounds, which can be minutes apart, and are replaced by the
fuzzer's exact values after every round.

The queue keys describe how far the fuzzer has worked through its corpus: `cycles_done`
is the number of completed passes over the corpus, `cycles_wo_finds` the passes since the
corpus last grew, `pending_total` the entries not yet fuzzed, `pending_favs` the favored
entries not yet fuzzed, `cur_item` the index of the entry being fuzzed, and `max_depth`
the longest lineage of mutations. The corpus is scanned for these at most once per
`afl_stats_interval`, so very large corpora do not slow every round.

Each console status line (executions, executions per second, Edges, corpus size,
crashes, hangs, and time since the last find) is printed at every statistics write, so the Simics console keeps
reporting progress even when no new testcase or solution is found.

When `coverage_reporting` is enabled, `fuzzer_stats` also carries
`tsffs_branch_targets`: the number of distinct program counters reached by taken
branches. This is the firmware reachability measure that `log.json` heartbeats used to
report. It is a **Branch target** count, not an Edge count: several Branch targets can
hash into one Edge and one Branch target can contribute to several Edges, so the two are
not comparable. The key is absent when coverage reporting is disabled.

### Enable Logging and Set Log path

> **Deprecated.** `log.json`, and the `log_to_file`, `log_path`, `heartbeat` and
> `heartbeat_interval` attributes that configure it, are being replaced by
> AFL++-compatible outputs (see
> [ADR 0001](https://github.com/Wenzel/tsffs/blob/ecov-m1-stage/docs/adr/0001-replace-log-json-with-afl-compatible-outputs.md)).
> Heartbeat and message records are **no longer written** to `log.json`: use
> `fuzzer_stats` and `plot_data` in the [Instance directory](#afl-statistics-and-the-instance-directory)
> instead, with `afl_stats_interval` replacing `heartbeat_interval`. The `heartbeat`
> and `heartbeat_interval` attributes are still accepted so existing scripts keep
> working, but they no longer have any effect. Heartbeat Branch target counts moved to
> `tsffs_branch_targets` in `fuzzer_stats`. `log_to_file` and `log_path` still control
> the startup, interesting, solution and timeout records, which stay until their
> replacements land.

By default, the fuzzer will log startup, interesting input, solution and timeout
records in JSON format to a log in the project directory (`log.json`).

The path for this log can be set by setting:

```python
@tsffs.log_path = SIM_lookup_file("%simics%) + "/log.json"
```

You can also disable the logging completely with:

```python
@tsffs.log_to_file = False
```

### Keep All Corpus Entries

For debugging purposes, TSFFS can be set to keep *all* corpus entries, not just
corpus entries which cause interesting results. This generates a large number
of corpus files.

```python
@tsffs.keep_all_corpus = True
```

### Use Initial Buffer Contents As Corpus

When using compiled-in or manual harnessing, the initial contents of the
testcase
buffer can be used as a seed corpus entry. This can be enabled with:

```python
@tsffs.use_initial_as_corpus = True
```

### Disable Auto-Continue in Repro Mode

By default, when running in [repro mode](../fuzzing/analyzing-results.md), TSFFS
automatically resumes simulation after preparing the testcase. To instead pause
and let an external debugger (for example a GDB stub) control resume, set:

```python
@tsffs.repro_auto_continue = False
```

See [Debugging Live with a GDB Stub](../tutorials/edk2-uefi/reproducing-runs.md#debugging-live-with-a-gdb-stub)
for a full walkthrough.
