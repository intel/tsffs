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
    - [Adding Tokens From Target Software](#adding-tokens-from-target-software)
    - [Setting an Architecture Hint](#setting-an-architecture-hint)
    - [Adding a Trace Processor](#adding-a-trace-processor)
    - [Disabling Coverage Reporting](#disabling-coverage-reporting)
    - [Enable Logging and Set Log path](#enable-logging-and-set-log-path)
    - [Keep All Corpus Entries](#keep-all-corpus-entries)
    - [Use Initial Buffer Contents As Corpus](#use-initial-buffer-contents-as-corpus)

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

### Enable Logging and Set Log path

By default, the fuzzer will log useful informational messages in JSON format to
a log in the project directory (`log.json`).

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
