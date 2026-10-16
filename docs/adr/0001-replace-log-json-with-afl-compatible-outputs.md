# Replace log.json with AFL-compatible outputs, in stages

TSFFS's `log.json` is a proprietary JSON-lines log (startup, prose monitor stats, heartbeats, and interesting/solution/timeout records with input bytes and new edges). No tool outside TSFFS reads it, so users cannot point standard fuzzing tooling at a live campaign. We are retiring it in favour of formats the fuzzing community already uses, and accept breaking `log.json` consumers to do so.

Each record type gets a standard replacement, and `log.json` stays alive until all of them exist so no release drops data with no replacement:

1. AFL++ `fuzzer_stats` / `plot_data` replace heartbeat and prose stats records; those records stop being written. Heartbeat Branch target counts move to a `tsffs_branch_targets` key in `fuzzer_stats`. `log_to_file`, `log_path`, `heartbeat`, and `heartbeat_interval` are deprecated; `afl_stats_interval` replaces the heartbeat cadence.
2. An AFL output layout (`queue/`, `crashes/`, `hangs/`) replaces interesting, solution, and timeout records. It lives in the stage-1 Instance directory (`<output_directory>/<instance_name>/`), and `corpus_directory` / `solutions_directory` move there with it, so all of a session's outputs share one root and later stages need no further relocation.
3. Per-testcase drcov replaces the per-record edge lists; `log.json` is removed once this lands.

## Considered options

- **Keep `log.json` and add AFL files alongside it**: no breakage, but two parallel sources of truth and a format nobody else reads, forever.
- **Remove `log.json` in one step**: smallest total diff, but leaves a release where crash/hang classification and per-testcase edges have no replacement.
- **Keep corpus and solutions in their own configurable roots in stage 2**: avoids moving existing directories, but splits one session's outputs across unrelated paths and leaves `afl-whatsup`-style tooling unable to find them from the Instance.
