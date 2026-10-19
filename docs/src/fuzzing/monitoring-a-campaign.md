# Monitoring a Campaign

A TSFFS session writes AFL++-format statistics into its *Instance* directory,
`<output_directory>/<instance_name>/` (by default `output/default/` in the Simics
project; see [AFL++ Statistics and the Instance Directory](../config/common-options.md#afl-statistics-and-the-instance-directory)).
Because the files and their keys mean what AFL++ users expect, the standard AFL++
monitoring tools work on a TSFFS campaign, and so does anything that can read a text
file or a CSV.

The Instance holds:

| File           | Contents                                                                |
| -------------- | ----------------------------------------------------------------------- |
| `fuzzer_stats` | The current state of the campaign, rewritten every `afl_stats_interval` |
| `plot_data`    | One row per write: the campaign's history, for charts                   |
| `fuzzer_setup` | The session's configuration and command line, written once              |

The Simics console also prints a one-line status summary at every write, so an
interactive session keeps reporting progress even when no new testcase is found.

## Checking a Campaign with `afl-whatsup`

`afl-whatsup` summarises every Instance found under an output directory, which makes it
the quickest way to check a fleet of parallel campaigns:

```sh
afl-whatsup -d output
```

Always pass `-d` for TSFFS. Without it, `afl-whatsup` decides whether an Instance is
alive with `kill -0 <fuzzer_pid>`. That check cannot see a native Windows Simics process
from WSL or Git Bash, and it cannot see a process running on another host, so those
Instances are reported as dead and skipped. `-d` includes dead and remote Instances in
the summary. Fields that depend on liveness, such as the time since the last find, read
as "none seen yet" for Instances that `afl-whatsup` believes are dead; use
`last_find` in `fuzzer_stats` directly if you need it.

## Charting a Campaign with `afl-plot`

`afl-plot` renders `plot_data` into graphs of corpus growth, pending items, crashes and
hangs, execution speed and Edges:

```sh
afl-plot output/default plots
```

`afl-plot` is a bash script that drives `gnuplot`, so on Windows it needs both:

- **bash**, through WSL or Git Bash, and
- **gnuplot**, installed in whichever of those you use (`sudo apt install gnuplot` in
  WSL).

It needs at least a header and two data rows, so let a campaign run for a few
`afl_stats_interval`s before plotting.

## Loading `plot_data` in pandas or a Spreadsheet

`plot_data` is a comma-separated file whose first line is a `#`-prefixed header. In
pandas:

```python
import pandas as pd

plot = pd.read_csv("output/default/plot_data", skipinitialspace=True)
plot = plot.rename(columns=lambda name: name.lstrip("# "))

# map_size is a percentage with a trailing '%', as in AFL++
plot["map_size"] = plot["map_size"].str.rstrip("%").astype(float)

plot.plot(x="relative_time", y=["corpus_count", "edges_found"], secondary_y="edges_found")
```

In a spreadsheet, open the file as comma-separated text and delete the leading `#` and
space from the first header cell. Every row has exactly 15 columns.

A session that restarts into an existing Instance moves the old `plot_data` to
`plot_data.<unix_timestamp>` and starts a new one, so each file is one session and you
can load them side by side.

## Telling a Stuck Campaign from a Quiet One

TSFFS writes statistics each time the simulation stops, once `afl_stats_interval`
seconds of wall-clock time have passed. A healthy campaign stops the simulation at the
end of every iteration, so `last_update` in `fuzzer_stats` keeps advancing even while
nothing new is being found.

**If `last_update` is stale** (much older than `afl_stats_interval`), the simulation has
not stopped for that long: an iteration is stuck, for example a target that never
reaches the stop harness while the virtual-time timeout is too long or disabled. A
campaign that is alive but has plateaued shows a fresh `last_update` with a growing
`time_wo_finds` and `cycles_wo_finds`.

Other keys worth watching:

- `edges_found` and `bitmap_cvg` show whether coverage is still growing, and whether the
  coverage map is approaching saturation.
- `stability` below 100% means some Edges are not deterministic, for example paths that
  depend on timers or interrupts the snapshot does not pin down.
- `pending_total` and `pending_favs` show how much of the corpus has not been fuzzed yet.
- `saved_crashes` and `saved_hangs` count crashes and timeouts separately.
