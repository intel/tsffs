# Spec: AFL++-compatible campaign statistics

## Problem Statement

A user running a TSFFS campaign has no standard way to watch it. Progress is only visible as a prose status line on the Simics console and as proprietary JSON records in `log.json`, which no tool outside TSFFS understands. Users who already monitor fuzzers with `afl-whatsup` and `afl-plot`, or who track campaigns in their own dashboards, cannot point that tooling at TSFFS. Campaigns run for days, often several in parallel on one host or a CI farm, so this blind spot costs real triage time: operators cannot tell at a glance whether an Instance is alive, still finding Edges, stable, or stuck.

The first implementation attempt also showed what not to do: a LibAFL `Monitor` only sees aggregated event counters, so most AFL fields were zero-filled or hard-coded, coverage counted Branch targets instead of Edges, and an I/O error while writing stats could abort the whole Simics session.

## Solution

Every TSFFS session maintains an Instance directory, `<output_directory>/<instance_name>/`, containing AFL++-format `fuzzer_stats`, `plot_data`, and `fuzzer_setup` files that stay current while the campaign runs. The values mean what AFL++ users expect them to mean, so `afl-whatsup` summarises a TSFFS campaign and `afl-plot` charts it without modification, and any script that parses AFL stat files works too.

Writing stats can never stop fuzzing. Parallel sessions never overwrite each other by default, and restarting into an existing Instance never mixes two sessions into one graph.

This is the first stage of retiring `log.json` in favour of community-standard outputs (see ADR 0001): heartbeat and prose stats records stop being written to `log.json`, while its remaining records stay until their replacements land.

## User Stories

1. As a fuzzing operator, I want each TSFFS session to write an AFL++ `fuzzer_stats` file, so that I can inspect a campaign's state with the same tools I use for AFL++.
2. As a fuzzing operator, I want `afl-whatsup` to summarise all TSFFS Instances under one output directory, so that I can check a fleet of parallel campaigns with one command.
3. As a fuzzing operator, I want `afl-plot` to render a TSFFS Instance's `plot_data`, so that I can see coverage and corpus growth over time.
4. As a fuzzing operator, I want `edges_found` to count Edges (occupied coverage-map slots), so that the number is comparable with what AFL++ reports.
5. As a fuzzing operator, I want `bitmap_cvg` to be the share of the coverage map occupied by Edges, so that I can tell when coverage plateaus or the map nears saturation.
6. As a fuzzing operator, I want `bitmap_cvg` never to exceed 100%, so that I can trust it.
7. As a fuzzing operator, I want Edge counts reported even when coverage reporting is disabled, so that AFL stats do not depend on an unrelated setting.
8. As a fuzzing operator, I want `stability` computed as AFL++ does (unstable Edges relative to filled Edges), so that genuine non-determinism in the target is visible rather than hidden near 100%.
9. As a fuzzing operator, I want `stability` to reflect calibration results instead of a hard-coded value, so that I can detect timer- or interrupt-dependent paths the snapshot does not pin down.
10. As a fuzzing operator, I want `cycles_done` to report completed passes over the corpus, so that I know how thoroughly the corpus has been explored.
11. As a fuzzing operator, I want `pending_total` and `pending_favs` to report corpus entries not yet fuzzed, so that I can judge whether a campaign still has work queued.
12. As a fuzzing operator, I want `corpus_count` to be current at every write, so that corpus growth is not reported late.
13. As a fuzzing operator, I want `cur_item` and `max_depth` to reflect the fuzzer's real position and lineage depth, so that the stats are not misleading placeholders.
14. As a fuzzing operator, I want `saved_crashes` and `saved_hangs` reported separately, so that I can tell crashes from timeouts without opening the solutions directory.
15. As a fuzzing operator, I want `last_find`, `last_crash`, and `last_hang` to carry real timestamps, so that `afl-whatsup` can tell me how long since the last progress.
16. As a fuzzing operator, I want `execs_done` and `execs_per_sec` to be live at every write, so that throughput is never minutes stale.
17. As a fuzzing operator, I want `execs_ps_last_min` computed over the actual time since the previous write, so that slow iterations do not inflate the recent speed.
18. As a fuzzing operator, I want `execs_since_crash` to count executions since the most recent crash, so that it means what AFL++ users expect.
19. As a fuzzing operator, I want `exec_timeout` to reflect the configured TSFFS timeout, so that the file describes the real campaign settings.
20. As a fuzzing operator, I want stats written on a wall-clock cadence while the simulation runs, so that the files stay fresh regardless of how long one fuzzing round takes.
21. As a fuzzing operator, I want stats written whether or not heartbeat logging is enabled, so that turning off heartbeats does not silently disable AFL stats.
22. As a fuzzing operator, I want a final stats write when the session shuts down, so that the last numbers on disk match the end of the campaign.
23. As a fuzzing operator, I want AFL stats enabled by default, so that every campaign is observable without extra configuration.
24. As a fuzzing operator, I want to disable AFL stats with one attribute, so that I can turn them off on hosts where writing extra files is unwanted.
25. As a fuzzing operator, I want to choose the output directory, so that I can put Instances on fast or shared storage.
26. As a fuzzing operator running sessions in parallel, I want each session's Instance to get a unique default name, so that parallel sessions never overwrite each other's stats.
27. As a CI maintainer, I want to set an explicit Instance name, so that each CI job's stats land in a predictable place.
28. As a CI maintainer, I want an Instance name containing path separators, `..`, or shell metacharacters to be rejected at startup with a clear error, so that stats never escape the output directory and `afl-whatsup` parsing never breaks.
29. As a CI maintainer reusing an Instance name, I want the previous session's `plot_data` preserved under a timestamped name when a new session starts, so that two sessions are never mixed into one graph and no history is lost.
30. As a CI maintainer, I want `fuzzer_stats` replaced atomically, so that a reader polling it never sees an empty or half-written file.
31. As a Windows user, I want transient file-sharing locks (another reader, antivirus) to be retried and tolerated, so that stats keep flowing on Windows hosts.
32. As a fuzzing operator, I want a stats write failure (full disk, read-only share, lock) to log a warning and let fuzzing continue, so that a multi-day campaign is never lost to a monitoring side feature.
33. As a fuzzing operator, I want each distinct kind of write failure reported once and its recovery reported once, so that a persistent problem does not flood the Simics log.
34. As a fuzzing operator, I want stats writing to resume automatically when the failure cause goes away, so that a brief lock does not disable stats for the rest of the campaign.
35. As a fuzzing operator, I want `fuzzer_setup` written once per session, so that `afl-whatsup` can discover the Instance without needless rewrites.
36. As an interactive Simics user, I want the fuzzer's progress line to keep appearing on the Simics console, so that I still see live feedback without opening any file.
37. As a TSFFS user, I want `log.json` to stop accumulating heartbeat and prose stats records, so that stats have a single, standard home.
38. As a TSFFS user, I want `log.json` to keep its interesting, solution, and timeout records for now, so that I lose no data until the AFL output layout replaces them.
39. As a TSFFS user, I want the documentation to describe the new attributes, the Instance layout, the default naming, and restart rotation, so that I can configure stats without reading source.
40. As a TSFFS user, I want the `log.json` documentation to state that it is deprecated and what replaces each record type, so that I can migrate before it is removed.
41. As a Windows user, I want a "Monitoring a campaign" guide that explains `afl-whatsup -d` (the PID liveness check does not work for Windows or remote Instances), `afl-plot` prerequisites (bash and gnuplot via WSL or Git Bash), and loading `plot_data` in pandas or a spreadsheet, so that I can monitor campaigns without AFL++ installed natively.
42. As a fuzzing researcher, I want `plot_data` to use the AFL++ 15-column layout, so that existing plotting scripts and notebooks work unmodified.
43. As a TSFFS maintainer, I want fuzzer-state-derived stats computed inside the fuzzer and live counters kept on the Simics side, so that each value comes from the component that actually knows it.
44. As a TSFFS maintainer, I want the stats snapshot to keep the coverage history map reachable, so that the follow-on raw bitmap dump needs no new plumbing.
45. As a TSFFS maintainer, I want the domain glossary to distinguish Edge from Branch target, so that future contributors do not conflate the two again.

## Implementation Decisions

- **Two producers, one writer.** A new LibAFL stage, `TsffsAflStatsStage`, runs last in the fuzzer's stage list and computes every value that requires the fuzzer `State`: `cycles_done` (scheduler metadata queue cycles), `pending_total` / `pending_favs` (scheduled count and favored metadata), `corpus_count`, Edge count and `bitmap_cvg` (map feedback history map), `stability`, `cur_item`, and `max_depth`. It publishes these into a shared snapshot. The Simics side owns the live counters it already maintains (executions, crashes, timeouts, and the times of the last interesting, crash, and timeout events), merges them with the latest snapshot, and writes the Instance files.
- **Snapshot transport** is shared state guarded by a lock, not a channel message: only the latest snapshot matters, and the writer reads it on its own cadence. Lock hold times are limited to copying the snapshot.
- **Write cadence** is wall-clock driven on the Simics side, using the existing heartbeat interval value, checked on each simulation stop. It is independent of the heartbeat logging flag. A final write happens at shutdown.
- **Why a stage and not a Monitor.** A LibAFL `Monitor` only sees events (aggregated execution counts, corpus size, objective count, user stats), never `State`, so it cannot provide cycles, pending, Edges, or stability. A stage alone runs only between fuzzing rounds, which can be minutes apart under Simics, so live counters stay on the Simics side. The `StatsMonitor` approach and its snapshot message are dropped.
- **Monitor.** The event manager keeps LibAFL's `SimpleMonitor`, whose prose status line is logged to the Simics console only, not to `log.json`.
- **Edge semantics.** `edges_found` and `bitmap_cvg` count Edges: occupied slots in the map feedback's history map. Branch targets (the tracer's distinct program counters) are not used in AFL files; `log.json` heartbeat Branch target counts are unaffected because heartbeat records are no longer written.
- **Stability formula.** `(filled - unstable) / filled`, where `filled` is the number of occupied history-map slots and `unstable` comes from calibration's unstable-entries metadata. This deliberately differs from LibAFL's own `stability` user stat, which divides by the whole map size and therefore stays near 100%.
- **Crash/hang split** uses the Simics-side crash and timeout counters, since LibAFL stores both in one solutions corpus.
- **Configuration attributes** on the `tsffs` object: `afl_stats` (bool, default true), `output_directory` (default: an `output` directory in the Simics project), `instance_name` (default `{hostname}-{pid}`). An `instance_name` containing path separators, `..`, or shell metacharacters is rejected at startup with an error explaining the allowed characters.
- **Instance directory contents.** `fuzzer_stats` (AFL++ key/value format, rewritten on each write via write-to-temporary then rename), `plot_data` (AFL++ 15-column format with header, appended each write), and `fuzzer_setup` (written once per session). `fuzzer_pid` is the Simics process id.
- **Restart rotation.** At session start, an existing `plot_data` in the Instance is renamed to `plot_data.<unix_timestamp>` and a fresh file with a header is started. A stale `fuzzer_stats` is simply replaced.
- **Error policy.** Stats write errors never propagate out of the simulation-stop handling. The first occurrence of each I/O error kind logs a warning; repeats log at debug level; the first success after a failure logs an informational recovery message. On Windows, a failed rename is retried once after a short delay before counting as a failure.
- **`log.json` changes.** Heartbeat and prose message records are no longer written. Startup, interesting, solution, and timeout records are unchanged. `log_to_file` and `log_path` keep working and are documented as deprecated; there is no runtime deprecation warning. The heartbeat flag and interval keep their meaning for any remaining heartbeat behaviour and the interval also drives the stats cadence.
- **Documentation.** The configuration chapter documents the three attributes, the Instance layout, default naming, and rotation; the `log.json` section gains a deprecation notice pointing to ADR 0001's replacement plan; a new "Monitoring a campaign" page covers `afl-whatsup -d`, `afl-plot` prerequisites, and loading `plot_data` with pandas or a spreadsheet.
- **Format target.** Field names and file layouts track AFL++ 4.x/5.x as consumed by `afl-whatsup` and `afl-plot`. The LibAFL revision TSFFS pins is not upgraded for this work; its upstream stats stage at that revision only fires a JSON user stat and writes no files.

## Testing Decisions

- **Good tests here assert on the files a user would read**, not on internal structures: the key set and values in `fuzzer_stats`, the header and column count of `plot_data`, the presence of rotated files, and the absence of removed `log.json` records.
- **Seam 1: the Instance writer (unit tests, no Simics).** Input: one merged stats record. Output: files in an Instance directory under a unique temporary directory per test. Cases: required `fuzzer_stats` keys and value formatting; the 15-column `plot_data` header and rows; the stability formula; `execs_ps_last_min` from actual elapsed time; rotation of an existing `plot_data` at session start; `fuzzer_setup` written only once; atomic replacement of `fuzzer_stats`; an injected I/O failure producing a warning instead of an error, and recovery on the next successful write; rejection of invalid Instance names. These replace the existing writer test, which used a fixed temporary path and failed on re-runs.
- **Seam 2: one end-to-end Simics integration test.** Modelled on the existing magic-harness integration tests (built on `simics_test::TestEnvSpec` with an iteration limit). It runs a short campaign with a fixed `instance_name` and asserts: `fuzzer_stats` parses; `edges_found > 0`; `corpus_count >= 1`; `execs_done` is consistent with the iteration limit; `plot_data` has a header and at least one row; `log.json` contains no heartbeat or message records. It then runs a second session with the same `instance_name` and asserts the first session's `plot_data` was rotated to a timestamped name. This is the only test that exercises `TsffsAflStatsStage` against a real LibAFL `State`.
- **No separate unit-test seam for the stage.** Constructing a LibAFL `State` with scheduler, calibration, and history-map metadata by hand would test internals; the integration test covers the stage's observable output.
- **Build verification** includes the Windows GNU target (`x86_64-pc-windows-gnu`), since the default MSVC target does not link.

## Out of Scope

- An AFL output layout (`queue/`, `crashes/`, `hangs/`) replacing `log.json`'s interesting, solution, and timeout records. This is the next stage of ADR 0001.
- Dumping the raw coverage bitmap (`map.afl` / `fuzz_bitmap`), including the choice of bitmap format and dump trigger.
- Per-testcase drcov output for Lighthouse/IDA, and the final removal of `log.json`.
- Upgrading the pinned LibAFL revision.
- A bundled plotting script, a JSON-lines stats stream, a Prometheus endpoint, or a Simics `status` command.
- Compatibility with tools that need a full AFL output layout or replay binaries (`afl-cmin`, `afl-tmin`, `afl-cov`, CASR, FuzzBench reporting).
- Resuming `plot_data` time series across sessions; TSFFS counters restart per session, so sessions are rotated instead.

## Further Notes

- This spec supersedes the first implementation on this branch, which used a custom LibAFL `Monitor`. Review of that version found that stats write errors could abort the Simics session, that coverage counted Branch targets against the map size (exceeding 100% on large targets and reading 0 with coverage reporting off), that the recent-speed figure divided by the configured interval rather than elapsed time, that stability and several timestamps were hard-coded, that writes were not atomic, and that parallel and restarted sessions corrupted each other's files.
- `afl-whatsup` checks liveness with `kill -0 <fuzzer_pid>`, which cannot see a native Windows Simics process from WSL or Git Bash, nor a process on another host. Instances then show as dead; `afl-whatsup -d` includes them. This is documented rather than worked around.
- Vocabulary follows `CONTEXT.md` (Edge, Branch target, Instance). The `log.json` retirement plan is ADR 0001.
