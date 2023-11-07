# TSFFS User's Guide

This guide walks through general configuration and usage of TSFFS for fuzzing. For
tutorials on various use cases, see the [tutorials](./tutorials).

- [TSFFS User's Guide](#tsffs-users-guide)
  - [Adding TSFFS to Projects](#adding-tsffs-to-projects)
    - [Adding to New Projects](#adding-to-new-projects)
    - [Adding to Existing Projects](#adding-to-existing-projects)
  - [Loading \& Initializing TSFFS](#loading--initializing-tsffs)
    - [Loading the Module](#loading-the-module)
    - [Initializing the Fuzzer](#initializing-the-fuzzer)
  - [Configuring the Fuzzer](#configuring-the-fuzzer)
    - [Start/Stop Configuration](#startstop-configuration)
      - [Magic Start/Stop](#magic-startstop)
      - [Manual Start/Stop](#manual-startstop)
      - [Manual Solutions](#manual-solutions)
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
    - [Reproducing Solutions](#reproducing-solutions)

## Adding TSFFS to Projects

Before TSFFS can be used, it must be added to (sometimes mentioned as *associated with*
in the SIMICS documentation) a project.

### Adding to New Projects

For new projects created with `ispm`, simply add the TSFFS package to the command line
when creating the project. For example, if you would create your project with the SIMICS
Base (1000) (always required), QSP-X86 (2096), and QSP-CPU (8112) packages with:

```sh
ispm projects /tmp/example-project-path/ --create \
    1000-latest 2096-latest 8112-latest
```

You can add the TSFFS package to the project by instead creating the project with:

```sh
ispm projects /tmp/example-project-path/ --create \
    1000-latest 2096-latest 8112-latest 31337-latest
```

### Adding to Existing Projects

Often, you will already have a project set up that you are using for simulation and
debugging of a particular platform and would prefer to add `tsffs` to it instead of
creating a new project. In this case, append a line to your project's `.package-list`
file containing either a relative path from the SIMICS base installation to the TSFFS
package like:

```txt
../simics-tsffs-6.0.0/
```

Or an absolute path to the directory where the TSFFS package was installed:

```txt
/path/to/simics-tsffs-6.0.0/
```

In either case, run the `project-setup` command in your project afterward:

```sh
bin/project-setup
```

## Loading & Initializing TSFFS

Before TSFFS can be used, the module must be loaded, an instance of the fuzzer must be
created and instantiated, and the fuzzer must be configured for your target.

### Loading the Module

The TSFFS module can be loaded by running (in a SIMICS script):

```simics
load-module tsffs
```

Or, in a Python script:

```python
SIM_load_module("tsffs")
```

### Initializing the Fuzzer

"The Fuzzer" is an instance of the `tsffs` class, declared in the `tsffs` module. The
`tsffs` class can only be instantiated once in a given simulation.

You can get the `tsffs` class by running (in a Python script -- this can be done in a
SIMICS script by prefixing this line with the `@` prefix):

```python
tsffs_cls = SIM_get_class("tsffs")
```

Once we have the `tsffs_cls` an instance can be created with:

```python
tsffs = SIM_create_object(tsffs_cls, "tsffs", [])
```

The fuzzer instance is now created and ready to configure and use.

## Configuring the Fuzzer

The fuzzer is configured through its singular interface, simply called
`tsffs`. This interface is used for both configuration and control of the
fuzzer.

### Start/Stop Configuration

The fuzzer can be configured to start the fuzzing loop one of two ways. First, code
compiled into the binary via the use of the [harness headers](../harness/) which provide
the `HARNESS_START` and `HARNESS_STOP` macros (or by manually implementing these macros)
can trigger a *magic instruction* defined by SIMICS to signal the fuzzer to start (or stop).
Second, the fuzzer can be started and stopped "manually" by calling interface functions,
which allows grey and black box fuzzing scenarios where modification of the source
code is impossible or infeasible.

#### Magic Start/Stop

Code using the provided harnesses typically looks something like:

```c
#include "tsffs-gcc-x86_64.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    HARNESS_START(buf, &size);

    if (size < 3) {
        // Stop early if there is not enough data
        HARNESS_STOP();
    }

    function_under_test(buf);

    // Stop normally on success
    HARNESS_STOP();

    return 0;
}
```

By default, TSFFS is enabled to use these harnesses, so no explicit configuration is
necessary. However, the defaults are equivalent to the configuration:

```python
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_start_magic_number(1)
tsffs.iface.tsffs.set_start_magic_number(2)
```

This sets TSFFS to start the fuzzing loop on a *magic* harness with magic number `1`
and stop execution and restore to the initial snapshot on *magic* harnesses with magic
number `2`.

If multiple fuzzing campaigns will be run on the same target software, it is sometimes
advantageous to compile multiple harnesses into the same target software ahead of time,
and choose which to enable at runtime. This can be accomplished with target software
like:

```c
#include "tsffs-gcc-x86_64.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    HARNESS_START(buf, &size);

    if (size < 3) {
        // Stop early if there is not enough data
        HARNESS_STOP();
    }

    char * result = function_under_test(buf);

    // Stop normally on success
    HARNESS_STOP();

    __arch_harness_start(3, result, &size);

    second_function_under_test(result);

    __arch_harness_stop(4);

    return 0;
}
```

And configuration settings like:


```python
tsffs.iface.tsffs.set_start_on_harness(True)
tsffs.iface.tsffs.set_stop_on_harness(True)
tsffs.iface.tsffs.set_start_magic_number(3)
tsffs.iface.tsffs.set_start_magic_number(4)
```

With this runtime configuration, the first (default) harness will be ignored, and only
the second set of harness calls will be used. Only up to 5 harnesses (with magic numbers
1-11) can be used. This is a limitation of the instructions SIMICS understands as
*magic*, some of which only support an immediate `0<=n<=12` (with magic numbers 0 and 12
*being
reserved by SIMICS).

#### Manual Start/Stop

Magic start and stop behavior can be disabled, which allows harnessing target software
without compiled-in harness code. However, implementation becomes highly target-specific
and the magic harness approach is highly preferred.

The same code as before, with no harness:


```c
#include "tsffs-gcc-x86_64.h"

int main() {
    char buf[20];
    size_t size = sizeof(buf);

    function_under_test(buf);

    return 0;
}
```

Can be harnessed in a black-box fashion by creating a script-branch to wait until the
simulation reaches a specified address, timeout, HAP, or any other condition. First, we
can disable magic harnesses (this is not strictly necessary unless any magic harnesses
actually exist in the target software, but it is good practice).

```python
tsffs.iface.tsffs.set_start_on_harness(False)
tsffs.iface.tsffs.set_stop_on_harness(False)
```

#### Manual Solutions

During manual or harnessed fuzzer execution, a normal stop or solution can be specified
at any time using the API. This allows arbitrary conditions for stopping execution of
a test case, or treating an execution as a solution, by programming via the SIMICS
script or SIMICS Python script.

During execution, the fuzzer can be signaled to stop the current testcase execution with
a normal exit (i.e. *not* a solution), and reset to the initial snapshot with a new
testcase with:

```python
tsffs.iface.tsffs.stop()
```

Likewise, the fuzzer can be signaled to stop the current testcase execution with a
solution. The fuzzer will save the input for this execution to the solutions directory
(see [that section](#set-corpus-and-solutions-directory)). The `solution` method takes
an ID and message that will be saved along with this solution for later use.

```python
tsffs.iface.tsffs.solution(0, "Solution because of [example]")
```

### Solution Configuration

#### Setting the Timeout

To set the number of seconds in virtual time until an iteration is considered *timed out*,
use the following (for example, to set the timeout to 3 seconds):

```python
tsffs.iface.tsffs.set_timeout(3.0)
```

Note that this timeout is in virtual time, not real time. This means that whether the
simulation runs faster or slower than real time, the timeout will be accurate to the
target software's execution speed.

#### Setting Exception Solutions

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

#### Setting Breakpoint Solutions

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

### Fuzzer Settings

#### Using Snapshots

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

#### Using CMPLog

Comparison logging greatly improves the efficiency of the fuzzer by making each
iteration more likely to progress through sometimes difficult-to-solve checks. It logs
values that are compared against during execution and uses them to mutate the input.

Comparison logging is enabled by default. It can be disabled with:

```python
tsffs.iface.tsffs.set_cmplog_enabled(False)
```

#### Set Corpus and Solutions Directory

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

#### Enable Random Corpus Generation

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

#### Set an Iteration Limit

The fuzzer can be set to execute only a specific number of iterations before exiting.
This is useful for CI fuzzing or for testing. The limit can be set with:

```python
tsffs.iface.tsffs.set_iterations(1000)
```

#### Adding Tokens From Target Software

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

#### Setting an Architecture Hint

Some SIMICS models may not report the correct architecture for their CPU cores. When not
correct, setting an architecture hint can be useful to override the detected
architecture for the core. this is mostly useful for architectures that report `x86-64`
but are actually `i386`, and for architectures that are actually `x86-64` but are
running `i386` code in backward-compatibility mode.

An architecture hint can be set with:

```python
tsffs.iface.tsffs.add_architecture_hint(qsp.mb.cpu0.core[0][0], "i386")
```

#### Adding a Trace Processor

By default, only the processor core that either executes the start harness or is passed
to the [manual start API](#manual-startstop) is traced during execution. When fuzzing
code running on multiple cores, the additional cores can be added with:

```python
tsffs.iface.tsffs.add_trace_processor(qsp.mb.cpu0.core[0][1])
```

### Reproducing Solutions

Once a solution is found, the fuzzer can be run in *repro* mode which will:

* Save a bookmark when the testcase is written
* Write only one testcase, the bytes from the specified file
* Stop without resetting to the initial snapshot

Repro mode can be run after stopping execution, or before executing the fuzzing loop.

```python
tsffs.iface.tsffs.repro("%simics%/solutions/TESTCASE")
```