# Common Options

TSFFS provides a set of common options that are usable no matter what type of harnessing
is desired.

- [Common Options](#common-options)
  - [Solution Configuration](#solution-configuration)
    - [Setting the Timeout](#setting-the-timeout)
    - [Setting Exception Solutions](#setting-exception-solutions)
    - [Setting Breakpoint Solutions](#setting-breakpoint-solutions)
  - [Fuzzer Settings](#fuzzer-settings)
    - [Using Snapshots](#using-snapshots)
    - [Using CMPLog](#using-cmplog)
    - [Set Corpus and Solutions Directory](#set-corpus-and-solutions-directory)
    - [Enable Random Corpus Generation](#enable-random-corpus-generation)
    - [Set an Iteration Limit](#set-an-iteration-limit)
    - [Adding Tokens From Target Software](#adding-tokens-from-target-software)
    - [Setting an Architecture Hint](#setting-an-architecture-hint)
    - [Adding a Trace Processor](#adding-a-trace-processor)

## Solution Configuration

TSFFS can be configured to treat various events as
[solutions](../README.md#terminology).

### Setting the Timeout

To set the number of seconds in virtual time until an iteration is considered *timed out*,
use the following (for example, to set the timeout to 3 seconds):

```python
tsffs.iface.tsffs.set_timeout(3.0)
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
tsffs.iface.tsffs.add_exception_solution(13)
```

An already-added exception can be removed from the tracked set that are considered
solutions with:

```python
tsffs.iface.tsffs.remove_exception_solution(13)
```

In addition, if *all* exceptions should be considered as solutions, use:

```python
tsffs.iface.tsffs.set_all_exceptions_are_solutions(True)
```

Note that this is typically not useful, all exceptions including innocuous exceptions
like timer interrupts will cause solutions. It is mainly useful for embedded models
running short code paths like when fuzzing interrupt handlers themselves, where any
exception occurring is truly an error.

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
tsffs.iface.tsffs.add_breakpoint_solution(bp_number)
tsffs.iface.tsffs.remove_breakpoint_solution(bp_number)
```

If not specifying a breakpoint number, breakpoints can be set as solutions with:

```python
tsffs.iface.tsffs.set_all_breakpoints_are_solutions(True)
```

This is useful when testing code that is not allowed to write, read, or execute specific
code. For example, userspace code should typically not execute code from its stack or
heap.

## Fuzzer Settings

### Using Snapshots

SIMICS 6.0.175 introduced an experimental snapshots feature that is not dependent on
reverse execution micro-checkpoints. In some cases, this snapshot method is faster and
in some cases resolves issues with model incompatibility with micro-checkpoints. This
feature is not enabled by default.

To use reverse-execution micro-checkpoints instead, use:

```python
tsffs.iface.tsffs.set_use_snapshots(False)
```

Micro-checkpoints cannot be used by when compiling the module against versions of SIMICS
which do not support them, and a runtime panic will occur when attempting to take a
snapshot if enabled on an older version of SIMICS.

### Using CMPLog

Comparison logging greatly improves the efficiency of the fuzzer by making each
iteration more likely to progress through sometimes difficult-to-solve checks. It logs
values that are compared against during execution and uses them to mutate the input.

Comparison logging is enabled by default. It can be disabled with:

```python
tsffs.iface.tsffs.set_cmplog_enabled(False)
```

### Set Corpus and Solutions Directory

By default, the corpus will be taken from (and written to) the directory "%simics%/corpus".

Initial test cases should be placed in this directory.

The directory test cases are taken from and written to can be changed with:

```python
tsffs.iface.tsffs.set_corpus_directory("%simics%/other_corpus_directory")
```

Likewise, the directory solutions are saved to can be changed with:


```python
tsffs.iface.tsffs.set_solutions_directory("%simics%/other_solutions_directory")
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
tsffs.iface.tsffs.set_generate_random_corpus(True)
```

### Set an Iteration Limit

The fuzzer can be set to execute only a specific number of iterations before exiting.
This is useful for CI fuzzing or for testing. The limit can be set with:

```python
tsffs.iface.tsffs.set_iterations(1000)
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
tsffs.iface.tsffs.tokenize_executable("%simics%/test.efi")
```

Tokens from source files are extracted in a best-effort language-independent way.
Multiple source files can be added.

```python
tsffs.iface.tsffs.tokenize_executable("/home/user/source/test.c")
tsffs.iface.tsffs.tokenize_executable("/home/user/source/test_lib.c")
tsffs.iface.tsffs.tokenize_executable("/home/user/source/test.h")
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
tsffs.iface.tsffs.add_token_file("%simics%/token-file.txt")
```

### Setting an Architecture Hint

Some SIMICS models may not report the correct architecture for their CPU cores. When not
correct, setting an architecture hint can be useful to override the detected
architecture for the core. this is mostly useful for architectures that report `x86-64`
but are actually `i386`, and for architectures that are actually `x86-64` but are
running `i386` code in backward-compatibility mode.

An architecture hint can be set with:

```python
tsffs.iface.tsffs.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")
```

### Adding a Trace Processor

By default, only the processor core that either executes the start harness or is passed
to the [manual start API](../harnessing/closed-box.md) is traced during execution. When fuzzing
code running on multiple cores, the additional cores can be added with:

```python
tsffs.iface.tsffs.add_trace_processor(qsp.mb.cpu0.core[0][1])
```