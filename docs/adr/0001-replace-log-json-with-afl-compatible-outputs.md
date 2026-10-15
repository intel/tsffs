# Replace log.json with AFL-compatible outputs, in stages

TSFFS's `log.json` is a proprietary JSON-lines log (startup, prose monitor stats, heartbeats, and interesting/solution/timeout records with input bytes and new edges). No tool outside TSFFS reads it, so users cannot point standard fuzzing tooling at a live campaign. We are retiring it in favour of formats the fuzzing community already uses, and accept breaking `log.json` consumers to do so.

Each record type gets a standard replacement, and `log.json` stays alive until all of them exist so no release drops data with no replacement:

1. AFL++ `fuzzer_stats` / `plot_data` replace heartbeat and prose stats records; those records stop being written and `log_to_file` / `log_path` are deprecated.
2. An AFL output layout (`queue/`, `crashes/`, `hangs/`) replaces interesting, solution, and timeout records.
3. Per-testcase drcov replaces the per-record edge lists; `log.json` is removed once this lands.

## Considered options

- **Keep `log.json` and add AFL files alongside it**: no breakage, but two parallel sources of truth and a format nobody else reads, forever.
- **Remove `log.json` in one step**: smallest total diff, but leaves a release where crash/hang classification and per-testcase edges have no replacement.
