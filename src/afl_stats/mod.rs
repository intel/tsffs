// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! AFL++-compatible campaign statistics.
//!
//! A session owns one *Instance* directory, `<output_directory>/<instance_name>/`, holding the
//! files AFL++ tooling (`afl-whatsup`, `afl-plot`) reads. This module has no Simics dependency:
//! the Simics side feeds it a [`StatsSample`] on a wall-clock cadence and the [`InstanceWriter`]
//! turns that into files. See `docs/specs/afl-stats.md`.

pub(crate) mod derive;
mod session;
pub(crate) mod snapshot;

use derive::coverage_pct;
use snapshot::FuzzerSnapshot;

use anyhow::{anyhow, bail, Result};
use std::{
    fs::{create_dir_all, rename, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Name of the stats file inside an Instance
pub(crate) const FUZZER_STATS_NAME: &str = "fuzzer_stats";
/// Name of the lock file inside an Instance
const LOCK_NAME: &str = ".lock";
/// Name of the time series file inside an Instance
pub(crate) const PLOT_DATA_NAME: &str = "plot_data";
/// Name of the file that makes `afl-whatsup` discover an Instance
pub(crate) const FUZZER_SETUP_NAME: &str = "fuzzer_setup";

/// The `plot_data` header, with the 15 columns `afl-plot` selects by position. This is the
/// header AFL++ itself writes; column 7 (`map_size`) holds a coverage percentage there too.
/// Extra attempts at rotating `plot_data` if the first rename fails
const ROTATE_RETRIES: u32 = 5;
const ROTATE_RETRY_DELAY: Duration = Duration::from_millis(200);

const PLOT_DATA_HEADER: &str = "# relative_time, cycles_done, cur_item, corpus_count, \
pending_total, pending_favs, map_size, saved_crashes, saved_hangs, max_depth, execs_per_sec, \
total_execs, edges_found, total_crashes, servers_count";

/// Check that an Instance name is a single safe path component: letters, digits, `.`, `_` and
/// `-`, and not made only of dots (which would be `.` or `..`).
pub(crate) fn validate_instance_name(name: &str) -> Result<()> {
    let allowed = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));

    if !allowed || name.chars().all(|c| c == '.') {
        bail!(
            "Invalid instance_name {name:?}: it must be non-empty, contain only letters, digits, \
            '.', '_' and '-', and not consist only of dots"
        );
    }

    Ok(())
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_seconds_or_zero(time: Option<SystemTime>) -> u64 {
    time.map(unix_seconds).unwrap_or(0)
}

/// Settings that are fixed for the lifetime of a session
#[derive(Debug, Clone)]
pub(crate) struct InstanceConfig {
    /// The output directory containing all Instances
    pub output_directory: PathBuf,
    /// The Instance name, used as the directory name and as `afl_banner`
    pub instance_name: String,
    /// When the session started
    pub start_time: SystemTime,
    /// Process id of the Simics process
    pub fuzzer_pid: u32,
    /// The Simics command line
    pub command_line: String,
    /// The configured per-iteration timeout, in milliseconds
    pub exec_timeout_ms: u64,
    /// Configuration attribute names and values, recorded once in `fuzzer_setup`
    pub setup_attributes: Vec<(String, String)>,
}

/// The live counters known to the Simics side at one stats tick
#[derive(Debug, Clone)]
pub(crate) struct StatsSample {
    /// The wall-clock time of this tick
    pub now: SystemTime,
    /// Total executions so far
    pub execs_done: u64,
    /// Executions that ended in a crash (a solution that is not a timeout)
    pub crashes: u64,
    /// Executions that ended in a timeout
    pub hangs: u64,
    /// When the last interesting input (new coverage) was found
    pub last_find: Option<SystemTime>,
    /// When the last crash happened
    pub last_crash: Option<SystemTime>,
    /// When the last timeout happened
    pub last_hang: Option<SystemTime>,
    /// The value of `execs_done` at the moment of the last crash
    pub execs_at_last_crash: u64,
    /// Entries in the corpus. Live: bumped between fuzzer snapshots, then overwritten by them.
    pub corpus_count: u64,
    /// Edges found. Live: bumped between fuzzer snapshots, then overwritten by them.
    pub edges_found: u64,
    /// The size of the coverage map
    pub total_edges: u64,
    /// The latest values derived from the fuzzer `State`, if the fuzzer has published any yet
    pub fuzzer: Option<FuzzerSnapshot>,
    /// Distinct program counters reached by taken branches. Only tracked when coverage
    /// reporting is enabled, so `None` otherwise. These are not Edges.
    pub branch_targets: Option<u64>,
}

/// What the caller should tell the user after a write attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteNotice {
    /// Nothing to report
    Quiet,
    /// First failure after a success: log a warning
    Failed(String),
    /// A repeated failure: log at debug level only
    StillFailing(String),
    /// First success after a failure: log an informational recovery message
    Recovered,
}

/// Writes the files of one Instance. Writing never returns an error: failures are summarized
/// as a [`WriteNotice`] so a monitoring side feature cannot stop a campaign.
#[derive(Debug)]
pub(crate) struct InstanceWriter {
    config: InstanceConfig,
    dir: PathBuf,
    /// Time and execution count at the previous write, for `execs_ps_last_min`
    previous: (SystemTime, u64),
    failing: bool,
    /// Exclusive lock on `<instance>/.lock`, held for the lifetime of the session. The OS
    /// releases it when this is dropped or the process exits, so a crashed session never blocks
    /// the next one.
    _lock: File,
}

impl InstanceWriter {
    /// Create the Instance directory for a session and lock it. Fails on an invalid Instance
    /// name, or if another live session already holds the Instance.
    pub(crate) fn open(config: InstanceConfig) -> Result<Self> {
        validate_instance_name(&config.instance_name)?;

        let dir = config.output_directory.join(&config.instance_name);
        create_dir_all(&dir)
            .map_err(|e| anyhow!("Failed to create instance directory {}: {e}", dir.display()))?;

        // The lock is taken before anything else in the Instance changes
        let lock = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(dir.join(LOCK_NAME))
            .map_err(|e| anyhow!("Failed to open lock in {}: {e}", dir.display()))?;

        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => bail!(
                "Instance {:?} ({}) is in use by another running session. Parallel sessions \
                need distinct instance_name values.",
                config.instance_name,
                dir.display()
            ),
            Err(TryLockError::Error(e)) => {
                bail!("Failed to lock instance {}: {e}", dir.display())
            }
        }

        // Only now that this session owns the Instance is it safe to touch its files
        rotate_plot_data(&dir, config.start_time)?;

        let writer = Self {
            previous: (config.start_time, 0),
            config,
            dir,
            failing: false,
            _lock: lock,
        };

        writer.write_fuzzer_setup()?;
        writer.start_plot_data()?;

        Ok(writer)
    }

    /// Record the session configuration once, in the layout AFL++ uses for `fuzzer_setup`
    fn write_fuzzer_setup(&self) -> Result<()> {
        let mut contents = String::from("# environment variables:\n");
        for (name, value) in &self.config.setup_attributes {
            contents.push_str(&format!("{name}={value}\n"));
        }
        contents.push_str(&format!("# command line:\n{}\n", self.config.command_line));

        write_atomically(&self.dir, FUZZER_SETUP_NAME, contents.as_bytes())
    }

    /// Start a fresh `plot_data` containing only the header
    fn start_plot_data(&self) -> Result<()> {
        let mut file = File::create(self.dir.join(PLOT_DATA_NAME))?;
        writeln!(file, "{PLOT_DATA_HEADER}")?;
        Ok(())
    }

    /// The Instance directory
    #[cfg(test)]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write all files for one stats tick. Never fails; see [`WriteNotice`].
    pub(crate) fn write(&mut self, sample: &StatsSample) -> WriteNotice {
        let derived = self.derive(sample);
        let result = self
            .write_fuzzer_stats(sample, &derived)
            .and_then(|()| self.append_plot_data(sample, &derived));

        // Advance the baseline even on failure so the next speed is over the right interval
        self.previous = (sample.now, sample.execs_done);

        match (result, self.failing) {
            (Ok(()), false) => WriteNotice::Quiet,
            (Ok(()), true) => {
                self.failing = false;
                WriteNotice::Recovered
            }
            (Err(e), false) => {
                self.failing = true;
                WriteNotice::Failed(e.to_string())
            }
            (Err(e), true) => WriteNotice::StillFailing(e.to_string()),
        }
    }

    fn write_fuzzer_stats(&self, sample: &StatsSample, derived: &Derived) -> Result<()> {
        let contents = self.render_fuzzer_stats(sample, derived);
        write_atomically(&self.dir, FUZZER_STATS_NAME, contents.as_bytes())
    }

    /// Append one row to `plot_data`. The file is opened per row so a reader holding it open
    /// never blocks the session.
    fn append_plot_data(&self, sample: &StatsSample, derived: &Derived) -> Result<()> {
        let fuzzer = sample.fuzzer.as_ref().cloned().unwrap_or_default();
        let path = self.dir.join(PLOT_DATA_NAME);
        // If the file was deleted or truncated externally, restore the header so AFL++ tooling
        // can still parse it
        let needs_header = std::fs::metadata(&path).map_or(true, |m| m.len() == 0);
        let mut row = String::new();
        if needs_header {
            row.push_str(PLOT_DATA_HEADER);
            row.push('\n');
        }
        // `execs_per_sec` is the recent rate (`execs_ps_last_min`), as in AFL++, which writes
        // its sliding-window rate in this column; the whole-run average is only in `fuzzer_stats`
        row.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {:.2}%, {}, {}, {}, {:.2}, {}, {}, {}, {}\n",
            derived.run_time.as_secs(),
            fuzzer.cycles_done,
            fuzzer.cur_item,
            sample.corpus_count,
            fuzzer.pending_total,
            fuzzer.pending_favs,
            derived.bitmap_cvg,
            sample.crashes,
            sample.hangs,
            fuzzer.max_depth,
            derived.execs_ps_last_min,
            sample.execs_done,
            sample.edges_found,
            sample.crashes,
            // Sanitizer binaries, which TSFFS does not use
            0,
        ));

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(row.as_bytes())?;

        Ok(())
    }

    /// The figures that both `fuzzer_stats` and `plot_data` report
    fn derive(&self, sample: &StatsSample) -> Derived {
        let start = self.config.start_time;
        let run_time = sample.now.duration_since(start).unwrap_or(Duration::ZERO);
        let since_previous = sample
            .now
            .duration_since(self.previous.0)
            .unwrap_or(Duration::ZERO);

        Derived {
            run_time,
            execs_per_sec: rate(sample.execs_done, run_time),
            execs_ps_last_min: rate(
                sample.execs_done.saturating_sub(self.previous.1),
                since_previous,
            ),
            bitmap_cvg: coverage_pct(sample.edges_found as usize, sample.total_edges as usize),
        }
    }

    fn render_fuzzer_stats(&self, sample: &StatsSample, derived: &Derived) -> String {
        let start = self.config.start_time;
        let Derived {
            run_time,
            execs_per_sec,
            execs_ps_last_min,
            bitmap_cvg,
        } = *derived;

        // AFL++ measures time without finds from the start of the campaign until a find happens
        let time_wo_finds = sample
            .now
            .duration_since(sample.last_find.unwrap_or(start))
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let execs_since_crash = sample.execs_done.saturating_sub(sample.execs_at_last_crash);

        let fuzzer = sample.fuzzer.as_ref();

        // Key order follows AFL++. A key whose value TSFFS cannot know is `None` and omitted.
        let lines: Vec<(&str, Option<String>)> = vec![
            ("start_time", Some(unix_seconds(start).to_string())),
            ("last_update", Some(unix_seconds(sample.now).to_string())),
            ("run_time", Some(run_time.as_secs().to_string())),
            ("fuzzer_pid", Some(self.config.fuzzer_pid.to_string())),
            ("cycles_done", fuzzer.map(|f| f.cycles_done.to_string())),
            (
                "cycles_wo_finds",
                fuzzer.map(|f| f.cycles_wo_finds.to_string()),
            ),
            ("time_wo_finds", Some(time_wo_finds.to_string())),
            ("execs_done", Some(sample.execs_done.to_string())),
            ("execs_per_sec", Some(format!("{execs_per_sec:.2}"))),
            ("execs_ps_last_min", Some(format!("{execs_ps_last_min:.2}"))),
            ("corpus_count", Some(sample.corpus_count.to_string())),
            (
                "corpus_favored",
                fuzzer.map(|f| f.corpus_favored.to_string()),
            ),
            ("corpus_found", fuzzer.map(|f| f.corpus_found.to_string())),
            ("max_depth", fuzzer.map(|f| f.max_depth.to_string())),
            ("cur_item", fuzzer.map(|f| f.cur_item.to_string())),
            ("pending_favs", fuzzer.map(|f| f.pending_favs.to_string())),
            ("pending_total", fuzzer.map(|f| f.pending_total.to_string())),
            ("stability", fuzzer.map(|f| format!("{:.2}%", f.stability))),
            ("bitmap_cvg", Some(format!("{bitmap_cvg:.2}%"))),
            ("saved_crashes", Some(sample.crashes.to_string())),
            ("saved_hangs", Some(sample.hangs.to_string())),
            (
                "last_find",
                Some(unix_seconds_or_zero(sample.last_find).to_string()),
            ),
            (
                "last_crash",
                Some(unix_seconds_or_zero(sample.last_crash).to_string()),
            ),
            (
                "last_hang",
                Some(unix_seconds_or_zero(sample.last_hang).to_string()),
            ),
            ("execs_since_crash", Some(execs_since_crash.to_string())),
            (
                "exec_timeout",
                Some(self.config.exec_timeout_ms.to_string()),
            ),
            ("edges_found", Some(sample.edges_found.to_string())),
            ("total_edges", Some(sample.total_edges.to_string())),
            (
                "var_byte_count",
                fuzzer.map(|f| f.var_byte_count.to_string()),
            ),
            ("afl_banner", Some(self.config.instance_name.clone())),
            (
                "afl_version",
                Some(format!("tsffs-{}", env!("CARGO_PKG_VERSION"))),
            ),
            ("target_mode", Some("tsffs".to_string())),
            ("command_line", Some(self.config.command_line.clone())),
            (
                "tsffs_branch_targets",
                sample.branch_targets.map(|b| b.to_string()),
            ),
        ];

        lines
            .into_iter()
            .filter_map(|(key, value)| value.map(|v| format!("{key:<18}: {v}\n")))
            .collect()
    }
}

/// Figures shared by `fuzzer_stats` and `plot_data`
#[derive(Debug, Clone, Copy)]
struct Derived {
    run_time: Duration,
    execs_per_sec: f64,
    execs_ps_last_min: f64,
    bitmap_cvg: f64,
}

/// Write `name` in `dir` by writing a temporary file and renaming it over the target, so a
/// reader never sees an empty or partial file
fn write_atomically(dir: &Path, name: &str, contents: &[u8]) -> Result<()> {
    let temporary = dir.join(format!("{name}.tmp"));

    let mut file = File::create(&temporary)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    rename(&temporary, dir.join(name))?;

    Ok(())
}

/// Move a `plot_data` left by an earlier session to `plot_data.<unix_timestamp>`, so two
/// sessions are never mixed into one graph and no history is lost. The timestamp is this
/// session's start time; if that name is taken a counter is appended.
fn rotate_plot_data(dir: &Path, start_time: SystemTime) -> Result<()> {
    let current = dir.join(PLOT_DATA_NAME);
    if !current.exists() {
        return Ok(());
    }

    let stamp = unix_seconds(start_time);
    let mut target = dir.join(format!("{PLOT_DATA_NAME}.{stamp}"));
    let mut counter = 1;
    while target.exists() {
        target = dir.join(format!("{PLOT_DATA_NAME}.{stamp}.{counter}"));
        counter += 1;
    }

    // A monitoring tool holding the file open without delete sharing (notably on Windows) makes
    // the rename fail transiently, so retry briefly before refusing to start
    let mut result = rename(&current, &target);
    for _ in 0..ROTATE_RETRIES {
        if result.is_ok() {
            break;
        }
        std::thread::sleep(ROTATE_RETRY_DELAY);
        result = rename(&current, &target);
    }

    result.map_err(|e| {
        anyhow!(
            "Failed to rotate {} to {}: {e}",
            current.display(),
            target.display()
        )
    })
}

/// The one-line summary printed to the Simics console at every stats tick
pub(crate) fn status_line(start_time: SystemTime, sample: &StatsSample) -> String {
    let run_time = sample
        .now
        .duration_since(start_time)
        .unwrap_or(Duration::ZERO);
    let last_find = match sample.last_find {
        Some(t) => format!(
            "{}s ago",
            sample
                .now
                .duration_since(t)
                .unwrap_or(Duration::ZERO)
                .as_secs()
        ),
        None => "none yet".to_string(),
    };

    format!(
        "execs: {} ({:.2}/sec), edges: {}, corpus: {}, crashes: {}, hangs: {}, last find: {}",
        sample.execs_done,
        rate(sample.execs_done, run_time),
        sample.edges_found,
        sample.corpus_count,
        sample.crashes,
        sample.hangs,
        last_find
    )
}

fn rate(count: u64, over: Duration) -> f64 {
    if over.is_zero() {
        0.0
    } else {
        count as f64 / over.as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        fs::{create_dir, read_to_string, remove_dir_all},
        sync::atomic::{AtomicU32, Ordering},
    };

    /// A unique temporary directory per test, removed on drop
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "tsffs-afl-stats-{}-{}-{label}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            create_dir(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.0);
        }
    }

    const START: u64 = 1_700_000_000;

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(START + seconds)
    }

    fn config(dir: &TempDir) -> InstanceConfig {
        InstanceConfig {
            output_directory: dir.0.clone(),
            instance_name: "default".to_string(),
            start_time: at(0),
            fuzzer_pid: 4242,
            command_line: "simics -e run".to_string(),
            exec_timeout_ms: 3000,
            setup_attributes: vec![
                ("timeout".to_string(), "3.0".to_string()),
                ("cmplog".to_string(), "true".to_string()),
            ],
        }
    }

    fn sample(seconds: u64, execs_done: u64) -> StatsSample {
        StatsSample {
            now: at(seconds),
            execs_done,
            crashes: 0,
            hangs: 0,
            last_find: None,
            last_crash: None,
            last_hang: None,
            execs_at_last_crash: 0,
            corpus_count: 0,
            edges_found: 0,
            total_edges: 131072,
            fuzzer: None,
            branch_targets: None,
        }
    }

    fn parse(contents: &str) -> HashMap<String, String> {
        contents
            .lines()
            .map(|l| {
                let (k, v) = l.split_once(": ").expect("key: value line");
                (k.trim_end().to_string(), v.to_string())
            })
            .collect()
    }

    fn read_stats(writer: &InstanceWriter) -> HashMap<String, String> {
        parse(&read_to_string(writer.dir().join(FUZZER_STATS_NAME)).expect("read fuzzer_stats"))
    }

    #[test]
    fn fuzzer_stats_has_expected_keys_and_formatting() {
        let dir = TempDir::new("keys");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        let mut s = sample(100, 5000);
        s.crashes = 2;
        s.hangs = 1;
        s.last_find = Some(at(40));
        s.last_crash = Some(at(90));
        s.execs_at_last_crash = 4000;
        assert_eq!(writer.write(&s), WriteNotice::Quiet);

        let raw = read_to_string(writer.dir().join(FUZZER_STATS_NAME)).expect("read");
        assert!(raw.starts_with("start_time        : 1700000000\n"), "{raw}");

        let stats = parse(&raw);
        let expected = [
            ("start_time", "1700000000"),
            ("last_update", "1700000100"),
            ("run_time", "100"),
            ("fuzzer_pid", "4242"),
            ("time_wo_finds", "60"),
            ("execs_done", "5000"),
            ("execs_per_sec", "50.00"),
            ("saved_crashes", "2"),
            ("saved_hangs", "1"),
            ("last_find", "1700000040"),
            ("last_crash", "1700000090"),
            ("last_hang", "0"),
            ("execs_since_crash", "1000"),
            ("exec_timeout", "3000"),
            ("afl_banner", "default"),
            ("target_mode", "tsffs"),
            ("command_line", "simics -e run"),
        ];
        for (key, value) in expected {
            assert_eq!(stats.get(key).map(String::as_str), Some(value), "{key}");
        }
        assert_eq!(
            stats.get("afl_version"),
            Some(&format!("tsffs-{}", env!("CARGO_PKG_VERSION")))
        );
    }

    fn fuzzer_snapshot() -> FuzzerSnapshot {
        FuzzerSnapshot {
            corpus_count: 12,
            corpus_favored: 5,
            corpus_found: 9,
            edges_found: 1310,
            var_byte_count: 131,
            stability: 90.0,
            cycles_done: 3,
            cycles_wo_finds: 2,
            pending_total: 4,
            pending_favs: 1,
            cur_item: 7,
            max_depth: 6,
        }
    }

    #[test]
    fn coverage_keys_report_edges_and_are_capped() {
        let dir = TempDir::new("coverage");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        let mut s = sample(10, 10);
        s.corpus_count = 12;
        s.edges_found = 1310;
        s.fuzzer = Some(fuzzer_snapshot());
        writer.write(&s);

        let stats = read_stats(&writer);
        let expected = [
            ("corpus_count", "12"),
            ("corpus_favored", "5"),
            ("corpus_found", "9"),
            ("edges_found", "1310"),
            ("total_edges", "131072"),
            ("bitmap_cvg", "1.00%"),
            ("stability", "90.00%"),
            ("var_byte_count", "131"),
            ("cycles_done", "3"),
            ("cycles_wo_finds", "2"),
            ("pending_total", "4"),
            ("pending_favs", "1"),
            ("cur_item", "7"),
            ("max_depth", "6"),
        ];
        for (key, value) in expected {
            assert_eq!(stats.get(key).map(String::as_str), Some(value), "{key}");
        }

        // More Edges than map slots can only be a counting error; never report above 100%
        s.edges_found = 500_000;
        writer.write(&s);
        assert_eq!(read_stats(&writer)["bitmap_cvg"], "100.00%");
    }

    #[test]
    fn branch_targets_are_reported_only_when_tracked() {
        let dir = TempDir::new("branch");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        let mut s = sample(10, 10);
        s.branch_targets = Some(42);
        writer.write(&s);
        let stats = read_stats(&writer);
        assert_eq!(stats["tsffs_branch_targets"], "42");
        // Branch targets never feed the Edge keys
        assert_eq!(stats["edges_found"], "0");

        s.branch_targets = None;
        writer.write(&s);
        assert!(!read_stats(&writer).contains_key("tsffs_branch_targets"));
    }

    #[test]
    fn state_derived_keys_are_omitted_until_the_fuzzer_has_published() {
        let dir = TempDir::new("nosnapshot");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");
        writer.write(&sample(10, 10));

        let stats = read_stats(&writer);
        for key in [
            "corpus_favored",
            "corpus_found",
            "stability",
            "var_byte_count",
            "cycles_done",
            "cycles_wo_finds",
            "pending_total",
            "pending_favs",
            "cur_item",
            "max_depth",
        ] {
            assert!(!stats.contains_key(key), "{key} must be omitted");
        }
        // The live counters are always real
        for key in ["corpus_count", "edges_found", "total_edges", "bitmap_cvg"] {
            assert!(stats.contains_key(key), "{key} must be present");
        }
    }

    #[test]
    fn keys_tsffs_cannot_know_are_omitted() {
        let dir = TempDir::new("omitted");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");
        writer.write(&sample(10, 10));

        let stats = read_stats(&writer);
        for key in [
            "slowest_exec_ms",
            "peak_rss_mb",
            "corpus_imported",
            "cpu_affinity",
        ] {
            assert!(!stats.contains_key(key), "{key} must be omitted");
        }
    }

    #[test]
    fn events_that_have_not_happened_are_zero() {
        let dir = TempDir::new("zero");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");
        writer.write(&sample(10, 10));

        let stats = read_stats(&writer);
        for key in ["last_find", "last_crash", "last_hang"] {
            assert_eq!(stats[key], "0", "{key}");
        }
        // Without a find, time without finds counts from the start of the campaign
        assert_eq!(stats["time_wo_finds"], "10");
        // Without a crash, every execution is since the last crash
        assert_eq!(stats["execs_since_crash"], "10");
    }

    #[test]
    fn execs_ps_last_min_uses_actual_elapsed_time() {
        let dir = TempDir::new("speed");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        writer.write(&sample(10, 1000));
        assert_eq!(read_stats(&writer)["execs_ps_last_min"], "100.00");

        // A slow iteration: 40 seconds pass for 400 more executions, not the nominal interval
        writer.write(&sample(50, 1400));
        let stats = read_stats(&writer);
        assert_eq!(stats["execs_ps_last_min"], "10.00");
        assert_eq!(stats["execs_per_sec"], "28.00");
    }

    #[test]
    fn fuzzer_stats_is_replaced_without_leaving_a_temporary_file() {
        let dir = TempDir::new("atomic");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        writer.write(&sample(10, 1));
        writer.write(&sample(20, 2));

        assert_eq!(read_stats(&writer)["execs_done"], "2");
        assert!(!writer.dir().join("fuzzer_stats.tmp").exists());
    }

    #[test]
    fn invalid_instance_names_are_rejected() {
        for name in [
            "", ".", "..", "...", "a/b", "a\\b", "a b", "a:b", "../x", "naïve",
        ] {
            let err = validate_instance_name(name).expect_err(name);
            assert!(err.to_string().contains("letters, digits"), "{name}: {err}");
        }
        for name in ["default", "ci-job_1.2", "a..b", ".hidden", "A9"] {
            validate_instance_name(name).expect(name);
        }

        let dir = TempDir::new("invalid");
        let mut cfg = config(&dir);
        cfg.instance_name = "../escape".to_string();
        assert!(InstanceWriter::open(cfg).is_err());
        assert!(!dir.0.join("..").join("escape").exists());
    }

    fn plot_rows(writer: &InstanceWriter) -> Vec<String> {
        read_to_string(writer.dir().join(PLOT_DATA_NAME))
            .expect("read plot_data")
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn plot_data_has_the_afl_header_and_fifteen_columns_per_row() {
        let dir = TempDir::new("plot");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        let mut s = sample(100, 5000);
        s.corpus_count = 12;
        s.edges_found = 1310;
        s.crashes = 2;
        s.hangs = 1;
        s.fuzzer = Some(fuzzer_snapshot());
        writer.write(&s);
        writer.write(&sample(200, 9000));

        let rows = plot_rows(&writer);
        assert_eq!(rows.len(), 3, "header and one row per write: {rows:?}");
        assert!(rows[0].starts_with("# relative_time, cycles_done, cur_item, corpus_count"));
        assert_eq!(rows[0].split(',').count(), 15);
        for row in &rows[1..] {
            assert_eq!(row.split(',').count(), 15, "{row}");
        }
        // relative_time, cycles_done, cur_item, corpus_count, pending_total, pending_favs,
        // map_size, saved_crashes, saved_hangs, max_depth, execs_per_sec, total_execs,
        // edges_found, total_crashes, servers_count
        assert_eq!(
            rows[1],
            "100, 3, 7, 12, 4, 1, 1.00%, 2, 1, 6, 50.00, 5000, 1310, 2, 0"
        );
    }

    #[test]
    fn deleted_plot_data_gets_its_header_back() {
        let dir = TempDir::new("plot-deleted");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");
        writer.write(&sample(100, 5000));
        std::fs::remove_file(writer.dir().join(PLOT_DATA_NAME)).expect("remove");
        writer.write(&sample(200, 9000));

        let rows = plot_rows(&writer);
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(rows[0].starts_with("# relative_time"));
    }

    #[test]
    fn fuzzer_setup_is_written_once_per_session() {
        let dir = TempDir::new("setup");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        let path = writer.dir().join(FUZZER_SETUP_NAME);
        let setup = read_to_string(&path).expect("read fuzzer_setup");
        assert_eq!(
            setup,
            "# environment variables:\ntimeout=3.0\ncmplog=true\n# command line:\nsimics -e run\n"
        );

        // Later ticks leave it alone
        std::fs::write(&path, "sentinel").expect("overwrite");
        writer.write(&sample(10, 10));
        writer.write(&sample(20, 20));
        assert_eq!(read_to_string(&path).expect("read"), "sentinel");
    }

    #[test]
    fn a_new_session_rotates_the_previous_plot_data() {
        let dir = TempDir::new("rotate");

        let mut first = InstanceWriter::open(config(&dir)).expect("first session");
        first.write(&sample(10, 10));
        first.write(&sample(20, 20));
        let old_rows = plot_rows(&first);
        let old_dir = first.dir().to_path_buf();
        drop(first);

        // A stale fuzzer_stats from the previous session is simply replaced
        let stale = read_to_string(old_dir.join(FUZZER_STATS_NAME)).expect("stale stats");

        let mut restart = config(&dir);
        restart.start_time = at(1000);
        let mut second = InstanceWriter::open(restart).expect("second session");

        let rotated = old_dir.join(format!("plot_data.{}", START + 1000));
        assert_eq!(
            read_to_string(&rotated)
                .expect("rotated plot_data")
                .lines()
                .collect::<Vec<_>>(),
            old_rows
        );
        assert_eq!(
            plot_rows(&second).len(),
            1,
            "fresh plot_data has only the header"
        );

        second.write(&sample(1010, 5));
        let fresh = read_to_string(old_dir.join(FUZZER_STATS_NAME)).expect("fresh stats");
        assert_ne!(fresh, stale);
        assert_eq!(parse(&fresh)["execs_done"], "5");
    }

    #[test]
    fn a_refused_session_does_not_rotate_the_live_sessions_plot_data() {
        let dir = TempDir::new("rotate-locked");
        let mut live = InstanceWriter::open(config(&dir)).expect("live session");
        live.write(&sample(10, 10));
        let before = plot_rows(&live);

        let mut intruder = config(&dir);
        intruder.start_time = at(500);
        InstanceWriter::open(intruder).expect_err("instance is in use");

        assert_eq!(plot_rows(&live), before);
        let leftovers = std::fs::read_dir(live.dir())
            .expect("list")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("plot_data."))
            .count();
        assert_eq!(leftovers, 0);
    }

    /// Writes a realistic Instance for pointing `afl-whatsup` and `afl-plot` at by hand:
    /// `TSFFS_SAMPLE_OUT=/some/dir cargo test --lib emit_sample_instance -- --ignored`
    #[test]
    #[ignore = "writes to TSFFS_SAMPLE_OUT for manual checks with AFL++ tooling"]
    fn emit_sample_instance() {
        let out = PathBuf::from(
            std::env::var("TSFFS_SAMPLE_OUT").expect("set TSFFS_SAMPLE_OUT to an output directory"),
        );
        let mut cfg = config(&TempDir::new("unused"));
        cfg.output_directory = out;
        cfg.start_time = SystemTime::now() - Duration::from_secs(600);

        let mut writer = InstanceWriter::open(cfg.clone()).expect("open");
        for step in 1..=6u64 {
            let mut s = sample(0, step * 3000);
            s.now = cfg.start_time + Duration::from_secs(step * 100);
            s.corpus_count = 4 + step;
            s.edges_found = 400 + step * 80;
            s.last_find = Some(s.now - Duration::from_secs(20));
            s.fuzzer = Some(FuzzerSnapshot {
                cycles_done: step / 2,
                cur_item: step,
                pending_total: 8 - step,
                pending_favs: 2,
                max_depth: step,
                stability: 98.5,
                ..fuzzer_snapshot()
            });
            writer.write(&s);
        }
    }

    #[test]
    fn status_line_summarizes_the_campaign() {
        let mut s = sample(100, 5000);
        s.crashes = 2;
        s.hangs = 1;
        s.last_find = Some(at(40));
        s.edges_found = 700;
        s.corpus_count = 12;
        assert_eq!(
            status_line(at(0), &s),
            "execs: 5000 (50.00/sec), edges: 700, corpus: 12, crashes: 2, hangs: 1, last find: 60s ago"
        );

        s.last_find = None;
        assert!(status_line(at(0), &s).ends_with("last find: none yet"));
    }

    #[test]
    fn second_session_on_a_locked_instance_is_refused() {
        let dir = TempDir::new("lock");
        let first = InstanceWriter::open(config(&dir)).expect("first session");

        let err = InstanceWriter::open(config(&dir)).expect_err("instance is in use");
        let message = err.to_string();
        assert!(message.contains("in use"), "{message}");
        assert!(message.contains("instance_name"), "{message}");

        // A differently named Instance is independent
        let mut other = config(&dir);
        other.instance_name = "other".to_string();
        InstanceWriter::open(other).expect("distinct instance");

        // Dropping the first session releases the lock
        drop(first);
        InstanceWriter::open(config(&dir)).expect("lock released");
    }

    #[test]
    fn write_failure_warns_once_and_recovery_is_reported() {
        let dir = TempDir::new("failure");
        let mut writer = InstanceWriter::open(config(&dir)).expect("open");

        // A directory in the place of fuzzer_stats makes the atomic replace fail
        create_dir(writer.dir().join(FUZZER_STATS_NAME)).expect("block target");

        assert!(matches!(
            writer.write(&sample(10, 1)),
            WriteNotice::Failed(_)
        ));
        assert!(matches!(
            writer.write(&sample(20, 2)),
            WriteNotice::StillFailing(_)
        ));
        assert!(matches!(
            writer.write(&sample(30, 3)),
            WriteNotice::StillFailing(_)
        ));

        remove_dir_all(writer.dir().join(FUZZER_STATS_NAME)).expect("unblock target");

        assert_eq!(writer.write(&sample(40, 4)), WriteNotice::Recovered);
        assert_eq!(writer.write(&sample(50, 5)), WriteNotice::Quiet);
        assert_eq!(read_stats(&writer)["execs_done"], "5");
    }
}
