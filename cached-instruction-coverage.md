# Selective instruction instrumentation: x86 specification

Status: implemented for x86-64 and i386 behind the opt-in `selective_instrumentation`
attribute (default off); validation results and limitations in Appendix C.
Baseline: `dc70e107765f7ce7629892db03a7dd9b9684dbb3`.

## 1. Goal and scope

Reduce the cost of x86-64 fuzzing feedback by classifying instructions when they enter
Simics' internal instruction cache and installing execution callbacks only on branches
and configured comparisons. Do not fetch instruction bytes or disassemble instructions
on the selective execution hot path.

Retain full-instruction instrumentation as a supported mode for execution traces and
symbolic/source coverage. It is not a legacy mode: these features need information
that branch/compare-only instrumentation cannot provide.

Deliver and validate the PoC on Simics 7 first, then move to Simics 6. Other
architectures retain their current instrumentation. This change does not alter fuzzing
feedback semantics, expand the supported instruction set, or introduce new
parallel-execution safety guarantees.

## 2. Mode selection

Selective mode is opt-in: it is used only when `selective_instrumentation` is set to
`True` (default `False`). Choose one instrumentation mode per processor when it is
instrumented:

| Processor and configuration | Mode | Execution subscriptions |
| --- | --- | --- |
| `selective_instrumentation = False` (default), any architecture | Full-instruction (unchanged) | Existing global before/after callbacks |
| Opt-in, x86-64 or i386, no full-instruction features requested | Selective | Cached branch-after callbacks and, when configured, compare-before callbacks |
| Opt-in, x86-64 or i386, any full-instruction feature requested | Full-instruction, with a warning | Existing global before/after callbacks |
| Opt-in, other architectures | Full-instruction, with a warning | Existing global before/after callbacks |

Full-instruction features are:

- `symbolic_coverage`, including its symbolic/source coverage use cases.
- Any execution-trace saving option: `save_all_execution_traces`,
  `save_interesting_execution_traces`, `save_solution_execution_traces`, or
  `save_timeout_execution_traces`.

Ordinary branch `coverage_reporting` does not select full-instruction mode.
`execution_trace_pc_only` controls trace contents, not whether tracing is requested.
Comparison logging alone does not require full-instruction mode.

Never install selective and global feedback subscriptions simultaneously on the same
processor: that would double-count feedback. Repeated registration of an already
monitored processor must not duplicate subscriptions.

### Visibility

When selective mode is requested but configuration or the CPU model forces
full-instruction mode, emit a warning identifying the CPU, the reason, and the
performance cost. With the default (`selective_instrumentation = False`) emit nothing,
as before. For example:

```text
CPU 0: full-instruction instrumentation enabled for all execution traces;
callbacks on every instruction may substantially reduce fuzzing throughput
```

This warning makes accidental expensive configuration visible. Report selective mode
at informational level. Do not describe full-instruction mode as a legacy fallback.

### Configuration lifetime

Processors are instrumented when fuzzing starts (the start harness or manual start API
enables coverage), not when they are added: a processor registered earlier with
`add_trace_processor` is subscribed at that point, so campaign `cmplog` and
full-instruction feature options may be set any time before the fuzzing loop starts.
Processors added after start are instrumented immediately. All feedback callbacks are
gated on `coverage_enabled`, so deferring the subscription is not observable. Once a
processor is instrumented its mode is fixed; runtime mode switching and callback
reconfiguration are out of scope.

This restriction does not apply to `coverage_enabled` or per-testcase `cmplog_enabled`.
Those remain execution-time gates and may change without reclassifying instructions.

## 3. Selective instrumentation contract

### Processor setup

Require `cpu_instrumentation_subscribe.register_cached_instruction_cb` and
`cpu_cached_instruction.register_instruction_after_cb`. Also require the cached
before-instruction method when campaign cmplog is enabled.

Check individual required methods, not just interface presence. If selective mode cannot
be supported, fall back to full-instruction mode with a warning naming the CPU and the
missing capability, so CPU models that worked before keep working. Full-instruction mode
does not require the cached-instruction API.

Register one cache-fill callback for the processor. If that subscription fails, use
full-instruction mode instead; never install both. A failure to attach callbacks to one
cache entry is reported once and skipped rather than aborting the simulation.

### Cache-fill callback

Fetch the instruction bytes and classify them using the existing x86-64 or i386
instruction classification rules. Preserve the current branch categories, including
calls, returns, and supported interrupt/system-control transfers, and the existing
comparison categories, including `TEST`.

For each cache entry:

- Attach an after-execution callback if the instruction is a branch.
- Attach a before-execution callback if it is a comparison and campaign `cmplog` is
  true.
- Attach no execution callback for other instructions.

Classification must not emit coverage, update edge-chaining state, or log comparisons.
The cache-fill callback is not an execution event.

Attach callbacks independently of current `coverage_enabled` and `cmplog_enabled`.
When campaign cmplog is false, do not install comparison callbacks or retain comparison
metadata. Instructions that cannot be decoded retain the existing skip behavior;
do not reuse a previous instruction's classification.

### Execution callbacks

Branch-after callbacks check `coverage_enabled`, then read the current program counter
and record the destination. Preserve the existing coverage map indexing, wrapping hit
counts, `prev_loc` updates and per-testcase reset, and branch reporting.

Compare-before callbacks check `coverage_enabled`, campaign `cmplog`, and per-testcase
`cmplog_enabled`. When enabled, read the current PC and evaluate saved operand
expressions against current registers and memory. Preserve existing operand
interpretation, comparison types, map indexing, and handling of unsupported operand
evaluation.

Both callbacks record feedback on every eligible execution, including the first
execution after classification, repeated loop iterations, and subsequent testcases.
Neither callback fetches instruction bytes or disassembles them.

## 4. Metadata ownership and cache lifecycle

Comparison metadata owns the decoded operand expressions and comparison types. It must
not borrow decoder scratch state or depend on the most recently decoded instruction.
Transfer ownership only to a successfully installed callback; release it on registration
failure or through Simics' free callback when the cache entry is discarded.

Do not cache logical PCs, register values, effective memory addresses, or operand
values. Cached instructions are associated with physical locations, and logical
mapping and machine state can change between executions.

Install per-entry callbacks only inside the cache-fill callback, while its handle is
valid. A cache eviction or flush may cause another classification; reconstruction must
not leak metadata, reuse stale metadata, or create duplicate execution feedback.

Correctness must not depend on snapshot restore either retaining or flushing the cache.
Verify instructions already cached before subscription, as well as instructions cached
after subscription but before coverage activation. If the CPU requires explicit
reinstrumentation at subscription time, use the supported SDK mechanism; do not rely
on an eventual incidental flush.

## 5. Full-instruction mode contract

Preserve the existing global before/after execution path for trace saving and
symbolic/source coverage, alongside ordinary branch feedback and comparison logging.
No cached feedback subscription is installed for that processor.

All trace-saving options must receive complete instruction traces, including
non-branch/non-comparison instructions. Symbolic/source coverage must retain its
instruction-level observations. The mode-selection warning must identify why the
more expensive path was selected.

## 6. Validation and acceptance

The PoC is not validated merely because it compiles or completes a fuzzing run.

| Scenario | Required evidence |
| --- | --- |
| Instruction selection | Branches receive after callbacks; configured comparisons receive before callbacks; unrelated instructions receive neither in selective mode |
| Repeated execution | Already-cached branches contribute hit counts on every execution, and edge chaining follows execution order, not decode order |
| Feedback equivalence | For the same executed instruction sequence, selective feedback matches the baseline's destinations, coverage-map values, comparison PCs, types, and values |
| Late activation | Instructions cached before coverage activation produce feedback after activation; instructions cached before subscription are not missed |
| Cmplog disabled | Campaign `cmplog = false` installs no comparison callbacks |
| Cmplog phase changes | With campaign cmplog enabled, an ordinary execution followed by a comparison-logging execution observes current operands without requiring a flush |
| Metadata independence | Later decodes do not change earlier instructions' comparison metadata; changing runtime registers/memory changes evaluated operands |
| Restore and cache invalidation | Repeated snapshot restores and cache invalidation preserve feedback and release/rebuild per-entry metadata safely |
| Missing API | Missing required interfaces or methods select full-instruction mode with a warning, never both modes |
| Full-instruction features | Trace/source-coverage configuration selects the global path, emits the warning, preserves full-instruction observations, and does not double-count feedback |

Run focused tests and the existing relevant build/lint checks. First establish this
behavior on Simics 7; only then validate the Simics 6 path.

### Performance comparison

Measure the unmodified baseline and PoC using the same x86-64 target, CPU model,
Simics version, build profile, corpus, and iteration/snapshot settings. Measure with
campaign cmplog both disabled and enabled. Disable trace/source-coverage features
for selective-mode measurements.

Use the separately built unmodified revision as the baseline; enabling full traces
on the PoC is not a fair substitute. Separate target boot/setup from fuzzing throughput,
repeat measurements, and record the workload, settings, execution count, elapsed time,
and exec/s. Report observed improvement or regression; there is no predetermined
speedup threshold.

### Remaining empirical questions

The SDK inspection below does not establish whether initial subscription reinstruments
already-cached instructions or whether snapshot restore flushes the cache on every CPU
model. Qualify runtime behavior on each supported model rather than generalize from
interface presence alone. The target-specific checks and measurements are recorded
below.

## Appendix A. Original investigation

The following investigation describes the unmodified baseline at `dc70e10`; source
line references in this appendix refer to that revision. The specification above
supersedes its tentative implementation suggestions.

### Summary

At the baseline revision, TSFFS collects coverage with the **global** mode of
`cpu_instrumentation_subscribe`. For every executed instruction it runs a callback,
fetches the instruction bytes and disassembles them, then throws away everything that is
not a branch (or a compare, for cmplog). The Simics docs describe this mode as one that
"will become very slow".

The v1 C `afl-branch-tracer` (Robert Guenzel) used the **selective / cached
instruction** mode correctly. The Rust port did adopt `register_cached_instruction_cb`,
but it recorded coverage *inside the decode callback* instead of attaching per-execution
callbacks through `cpu_cached_instruction`. That only gives "first time cached"
coverage. It was shipped as an optional `Once` mode and then removed in the 0.2.0
rewrite. As far as the git history shows, the selective approach was never rejected on
its merits: it was ported incorrectly, and that version was removed.

Labels: **[fact]** means verified in code or git history, **[inference]** means
interpretation.

### A1. Baseline hooking

**[fact]** When a processor is added, `add_processor` (`src/lib.rs:716-726`) registers
two global callbacks with `connection = NULL` and no filter:

```rust
let mut cpu_interface: CpuInstrumentationSubscribeInterface = get_interface(cpu)?;
cpu_interface.register_instruction_after_cb(null_mut(), Some(on_instruction_after), self as *mut Self as *mut _)?;
cpu_interface.register_instruction_before_cb(null_mut(), Some(on_instruction_before), self as *mut Self as *mut _)?;
```

C callback type (`cpu_instruction_cb_t`): `fn(obj: *mut conf_object_t, cpu: *mut
conf_object_t, handle: *mut instruction_handle_t, user_data: *mut lang_void)`. The
`#[ffi(arg(rest), arg(self))]` wrappers in `src/tracer/mod.rs` turn `user_data` back
into `&mut Tsffs`.

| Callback | Work on **every** executed instruction | Kept when |
| --- | --- | --- |
| `on_instruction_after` (`src/tracer/mod.rs:325`) | `trace_pc` (`src/arch/x86_64.rs:143`): `get_instruction_bytes`, then disassemble, then `last_was_call \|\| last_was_control_flow \|\| last_was_ret` | It is a branch. The edge is the PC *after* execution (the destination), fed into the AFL map: `idx = pc ^ prev_loc`, `prev_loc = pc >> 1` |
| `on_instruction_before` (`src/tracer/mod.rs:369`) | `trace_cmp`: fetch and disassemble again (only when cmplog is enabled) | It is a compare. Operands go into the AFL++ cmplog map |

Consequences:

- Up to two callbacks and two disassemblies per executed instruction, with no caching.
  Hot loops are re-decoded on every iteration.
- `coverage_enabled` is checked *inside* the callbacks. A CPU added before the harness
  start (for example by a non-matching magic instruction) still pays the callback cost
  on every instruction, just not the disassembly.

### A2. What the Simics docs describe

Source: *Model Builder User's Guide*, section 42.4.1 "CPU Instrumentation API" (Simics
7).

| Mode | API | Cost |
| --- | --- | --- |
| Global | `cpu_instrumentation_subscribe.register_instruction_before_cb()`, then filter via `cpu_instruction_query.get_instruction_bytes()` | Callback on every instruction, "will become very slow". **This is what TSFFS does.** |
| Selective (cached) | `cpu_instrumentation_subscribe.register_cached_instruction_cb()` is called when an instruction enters the model's internal instruction cache. Decode once, then attach a callback with `cpu_cached_instruction.register_instruction_{before,after}_cb()` for the interesting ones | Analysis once per cache fill. Only the chosen instructions get per-execution callbacks. A cache flush causes re-decoding |
| Counter | `cpu_cached_instruction.add_counter()` | Lowest overhead, but it only counts. It cannot do AFL edge chaining (which needs `prev_loc`), so it is not usable for coverage |

Note: the cached-instruction decode callback is **not** limited to branches. It fires
for **every** instruction placed in the cache. The tool does the filtering. The speedup
comes from classifying once, and from leaving uninteresting instructions with no
per-execution callback at all.

### A3. History: how the cached approach got lost

| Commit | Date / author | What happened |
| --- | --- | --- |
| `93f07c2` | 2022-12-16, Robert Guenzel | **[fact]** v1 C `simics/modules/afl-branch-tracer/afl-branch-tracer.c`. Registers `cached_instruction_cb`, disassembles once, and for branch mnemonics calls `ci_iface->register_instruction_after_cb(cpu, ci_handle, branch_cb, ...)`. Correct selective instrumentation. |
| `327c747` | 2023-02-17, Rowan Hart | Rust rewrite begins. |
| `a4bbb90` | 2023-05-01, Rowan Hart, "Add cached instruction callback" | **[fact]** Adds `register_cached_instruction_cb`, but the decode callback calls `log_pc` directly. **No per-instruction callback is attached through `cpu_cached_instruction`.** Coverage is recorded when an instruction is *cached*, not when it is executed. |
| `c547480` | 2023-05-02, Rowan Hart, "Add tracing modes" | **[fact]** `TraceMode::Once` (cached callback, coverage at cache fill) and `TraceMode::HitCount` (global `register_instruction_before_cb`, the default). |
| `b913368` | 2023-11-27, Rowan Hart, "0.2.0 Release Candidate (#30)" | **[fact]** Large rewrite with an empty commit message. Only the global before/after callbacks survive. `CoverageMode { HitCount, Once }` still exists in `src/tracer/mod.rs:198` but nothing reads it any more (dead code). |

#### Side by side

v1 C (`93f07c2`) classifies at decode time and acts at execution time:

```c
static void cached_instruction_cb(conf_object_t *obj, conf_object_t *cpu,
        cached_instruction_handle_t *ci_handle, instruction_handle_t *iq_handle, void *user_data)
{
    cpu_bytes_t b = tracer->iq_iface->get_instruction_bytes(cpu, iq_handle);
    /* ... disassemble, match against branch_infos[] ... */
    tracer->ci_iface->register_instruction_after_cb(cpu, ci_handle, branch_infos[i].cb, user_data, NULL);
}
```

Rust (`a4bbb90`) classifies *and acts* at decode time:

```rust
pub fn on_cached_instruction(&mut self, _obj, cpu,
        _cached_instruction_data: *mut CachedInstructionHandle,   // unused
        handle: *mut InstructionHandle) -> Result<()> {
    if let Ok(r) = processor.trace(handle) {
        if let Some(pc) = r.edge { self.log_pc(pc)?; }            // logged once per cache fill
    }
    ...
}
```

### A4. Why it was probably dropped

**[inference]** No commit message, comment or doc in the repo gives a reason. The likely
story:

1. With coverage recorded at cache-fill time, the edge map only shows *new* code
   entering the Simics instruction cache. That cache normally survives across fuzzing
   iterations, so a new input that reaches already-cached code produces **no coverage
   signal**. Feedback is close to useless after warm-up.
2. For the same reason, `prev_loc` chaining is meaningless in that mode: decode order is
   not execution order.
3. The mode name `Once` suggests the behaviour was read as inherent to cached callbacks,
   rather than as a missing `cpu_cached_instruction.register_instruction_*_cb` step.
4. With the cached path seemingly "only once", the global hit-count path became the only
   one worth keeping in 0.2.0.

Not checked: GitHub issues and PR discussions, and the pre-move sandbox repo referenced
in `d5363ce`.

### A5. API availability in our Rust stack

**[fact]** Checked against the generated bindings locally (`simics-api-sys`
`bindings-auto.rs`, `simics` `interfaces.rs`):

- `cpu_cached_instruction_interface` provides **both** `register_instruction_before_cb`
  and `register_instruction_after_cb`, with the signature `(cpu, ci_handle: *mut
  cached_instruction_handle_t, cb: cpu_instruction_cb_t, user_data, free_cb:
  cpu_callback_free_user_data_cb_t)`. The per-execution callback reuses the same
  `cpu_instruction_cb_t` type as today's global callbacks.
- `free_cb` is called "when the instruction is not being cached anymore". That fits
  per-instruction user data such as pre-decoded cmp operands.
- The safe `simics` crate generates `CpuCachedInstructionInterface` with
  `register_instruction_{before,after}_cb`, `register_{read,write}_{before,after}_cb`,
  `add_counter`, etc. No hand-written FFI is needed.

### A6. Original selective-path sketch

```text
add_processor(cpu):
    cpu_instrumentation_subscribe.register_cached_instruction_cb(cpu, NULL, on_decode, tsffs)

on_decode(cpu, ci_handle, iq_handle, tsffs):           // once per cache fill, for ANY instruction
    bytes = cpu_instruction_query.get_instruction_bytes(cpu, iq_handle)
    insn  = disassemble(bytes)
    if insn is call/jmp/jcc/ret:
        cpu_cached_instruction.register_instruction_after_cb(cpu, ci_handle, on_edge, tsffs, NULL)
    if cmplog and insn is cmp:
        cpu_cached_instruction.register_instruction_before_cb(cpu, ci_handle, on_cmp, decoded_operands, free_fn)

on_edge(...): if coverage_enabled: log_pc(pc_after)       // no decoding on the hot path
on_cmp(...):  if coverage_enabled && cmplog_enabled: evaluate saved operands, log_cmp
```

Points to watch:

- **Attach regardless of `coverage_enabled`.** Instructions cached before the harness
  start must still get their callbacks, otherwise they stay uninstrumented until the
  next flush. Check the flag in the per-execution callback.
- **cmplog toggling.** Same issue for `cmplog`/`cmplog_enabled`: if cmplog can be
  switched on at run time, the cmp callback must already be attached (and check the
  flag), or the cache must be flushed when it changes.
- **Snapshot restore** may flush the instruction cache. That only costs re-decoding and
  does not affect correctness.
- **CPU model support.** The original suggestion was a global fallback for missing APIs.
  The agreed specification instead requires a clear selective-mode setup failure.
- **Edge semantics.** Keep "PC after the branch" (the destination) so corpora and
  coverage maps stay comparable.
- **Dead code.** `CoverageMode` (`src/tracer/mod.rs:198`) could be removed, or
  repurposed to select global vs. cached.

### A7. Historical questions

1. (for Robert) Did v1 run into any limitation of the cached path (CPU models,
   multi-core, cache flushes on restore) that would explain why it was not carried over?
2. (for Robert) Any recollection of the `Once` / `HitCount` discussion with Rowan?
3. Are there exec/s numbers for v1 versus the current global path? Nothing has been
   benchmarked yet. A before/after exec/s run on one target (for example the edk2-uefi
   tutorial) is the obvious first measurement.

### Investigation caveats

- Only the x86_64 `trace_pc` / `trace_cmp` were read in detail. The other architectures
  follow the same fetch-and-disassemble pattern.
- The expected speedup comes from the Simics documentation and has not been measured.
- `b913368` is a very large commit. It was grepped for the instrumentation
  registrations, not fully reviewed.

## Appendix B. SDK contract checked locally

Verified against Simics 7.106.0
`src/include/simics/model-iface/cpu-instrumentation.h`:

- Cached execution callbacks use the same `cpu_instruction_cb_t` signature as global
  callbacks (lines 288-312). Both before and after registration methods are available
  in `cpu_cached_instruction` (lines 501-513).
- Registration through `cpu_cached_instruction` must occur inside the cache-fill
  callback while its cached-instruction handle is valid (lines 367-375, 495-499).
- The after callback sees the branch destination when reading the program counter;
  querying the instruction address instead would return the branch source (lines
  428-434).
- Per-instruction callback data can hold decoded metadata and has a free callback
  invoked when the cached instruction is discarded (lines 315-337, 397-407).
  Owned comparison expressions can therefore follow the cache entry's lifetime.
- Cached metadata is associated with a physical instruction address; logical addresses
  can vary between executions (lines 424-426). Do not cache the logical PC, register
  values, effective memory addresses, or comparison operand values.
- Attached execution callbacks run on every execution, including the first execution
  immediately after classification. An instruction with no attached callbacks is
  not instrumented (lines 1083-1102).
- An interface can exist with individual registration methods unavailable (NULL);
  checking the interface name alone is insufficient (lines 928-931).
- Instrumentation callbacks may run concurrently under Simics multithreaded execution
  modes (lines 884-902). This PoC must not claim new parallel-execution safety for
  TSFFS's existing shared tracing state.

The installed `simics` 0.2.6 wrapper generator creates interface methods from the
bindings and returns `NoInterfaceMethod` for missing function pointers
(`build.rs:638-658`). No handwritten replacement for the SDK interface is required.

The inspected contract does not explicitly settle instrumentation of instructions
already cached when a new subscriber is installed, or whether snapshot restore
flushes the cache. Validate those lifecycle scenarios at runtime rather than assume
either cache behavior.

## Appendix C. PoC validation results

### Checks completed

- Five focused Rust tests pass: branch/compare selection, comparison metadata surviving
  later decodes, execution-time register/PC evaluation, wrapping branch hit counts and
  edge chaining/reporting, and disabled comparison callbacks avoiding CPU-state reads.
- The relevant Rust files pass formatting and Clippy checks.
- Existing `x86_64_magic_speedtest_latest` and `x86_64_magic_speedtest_6_0_185`
  integration tests pass, each completing 1,000 iterations with selective
  instrumentation and the default per-iteration snapshot restore.
- Simics 7 checkpoint-based runs exercised target instructions before subscription and
  after subscription but before coverage activation. Both produced the same first-input
  feedback as the unmodified baseline, with one selective subscription per processor.
- Those runs also observed ordinary and comparison-logging executions of the cached
  code. For input `AAAAAAAA`, both logged comparison operands `U32((65, 102))` during
  the comparison-logging phase.
- Enabling all execution traces selected full-instruction mode and emitted the expected
  warning, without a selective-mode subscription message. Saved traces contain ordinary
  `mov` instructions as well as comparisons and control-flow instructions.

The Simics 7 module was built against SDK 7.106.0; the existing "latest" integration
environment used runtime 7.117.0 and CPU class `x86QSP3`. The second integration test
used Simics 6.0.185.

### Performance

Method: each seed of a fixed corpus is replayed exactly once (no mutation), from a
checkpoint stopped at harness start, with the snapshot restored after every execution.
Builds: unmodified `main`, this branch with `selective_instrumentation` off, and this
branch with it on. Simics 7.117 on the QSP x86-64 board (`x86QSP3`), release builds,
runs interleaved and executed one at a time, median of 5 rounds. "Execution" is harness
start to stop (guest plus instrumentation); "total" also includes snapshot restore and
fuzzer overhead.

The three cmplog modes are: campaign cmplog off; campaign cmplog on with the testcase's
cmplog off (most executions in a real campaign); and both on (the cmplog stage).

UEFI driver handler, 616 seeds, ~40k guest instructions over ~3.2k distinct PCs per
execution:

| Cmplog | `main` exec | Flag off | Flag on | Exec speedup | Total speedup |
| --- | ---: | ---: | ---: | ---: | ---: |
| Off | 7.11 ms | 6.97 ms | 3.94 ms | 1.80x | 1.51x |
| On, testcase off | 7.01 ms | 7.17 ms | 4.21 ms | 1.66x | 1.40x |
| On, testcase on | 15.22 ms | 15.44 ms | 10.96 ms | 1.39x | 1.24x |

Password-check UEFI application (`tests/rsrc/x86_64-uefi/test.c`), 171 seeds: 153
executions run ~55 guest cycles and 18 crashing ones run ~1.43M cycles in the fault
path. Short executions are unchanged; total time improves 1.40x (cmplog off), 1.35x
(on, testcase off) and 1.32x (both on), all from the crashing executions.

With the flag off, both targets stay within 3% of `main`. On tiny straight-line
harnesses such as `test-fast.c`, snapshot restore is ~96% of wall time and the
instruction cache is flushed on every restore, so the gain is within noise.

### Validation limits

- Symbolic/source coverage selected full-instruction mode and emitted the warning.
  However, this fixture has no populated source-coverage records: output generation
  fails with `Node not found for path`. The unmodified baseline fails the same way.
  End-to-end source mapping still needs a suitable symbol-bearing target; this PoC
  does not fix that pre-existing empty-source-coverage failure.
- Superseded by the differential validation in the PR description: full coverage and
  AFL++ cmp maps were later compared execution by execution against the unmodified
  baseline (x86-64 on Simics 7 and 6, i386 on Simics 6) and were identical.
- No CPU model missing the cached API was available in these runs; missing-interface
  and missing-method handling is implemented but not exercised on such a model.
- Cache eviction cleanup uses the SDK free callback. No dedicated leak-checker run or
  parallel-execution qualification was performed.

### Commands and artifacts

The focused checks used:

```sh
SIMICS_BASE=/path/to/simics-7 cargo clippy --offline --lib --tests
LD_PRELOAD=/path/to/simics-python/lib/libpython3.10.so.1.0 \
  SIMICS_BASE=/path/to/simics-7 cargo test --offline --lib
SIMICS_BASE=/path/to/simics-7 SIMICS_TEST_LOCAL_PACKAGES_ONLY=1 \
  cargo test --offline --release --test x86_64_magic_speedtest_latest -- --nocapture
SIMICS_BASE=/path/to/simics-6.0.185 SIMICS_TEST_LOCAL_PACKAGES_ONLY=1 \
  cargo test --offline --release --test x86_64_magic_speedtest_6_0_185 -- --nocapture
```

The Python preload was required by the local standalone unit-test executable because
the Simics shared library otherwise could not resolve `PyExc_SystemError`.
