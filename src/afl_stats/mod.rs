// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! AFL++-compatible campaign statistics.
//!
//! A session owns one *Instance* directory, `<output_directory>/<instance_name>/`, holding the
//! files AFL++ tooling (`afl-whatsup`, `afl-plot`) reads. This module has no Simics dependency:
//! the Simics side feeds it a [`StatsSample`] on a wall-clock cadence and the [`InstanceWriter`]
//! turns that into files. See `docs/specs/afl-stats.md`.

mod session;

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

        Ok(Self {
            previous: (config.start_time, 0),
            config,
            dir,
            failing: false,
            _lock: lock,
        })
    }

    /// The Instance directory
    #[allow(dead_code)]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write all files for one stats tick. Never fails; see [`WriteNotice`].
    pub(crate) fn write(&mut self, sample: &StatsSample) -> WriteNotice {
        let result = self.write_fuzzer_stats(sample);

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

    fn write_fuzzer_stats(&self, sample: &StatsSample) -> Result<()> {
        let contents = self.render_fuzzer_stats(sample);
        let temporary = self.dir.join(format!("{FUZZER_STATS_NAME}.tmp"));
        let target = self.dir.join(FUZZER_STATS_NAME);

        let mut file = File::create(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);

        rename(&temporary, &target)?;

        Ok(())
    }

    fn render_fuzzer_stats(&self, sample: &StatsSample) -> String {
        let start = self.config.start_time;
        let run_time = sample.now.duration_since(start).unwrap_or(Duration::ZERO);
        let since_previous = sample
            .now
            .duration_since(self.previous.0)
            .unwrap_or(Duration::ZERO);

        let execs_per_sec = rate(sample.execs_done, run_time);
        let execs_ps_last_min = rate(
            sample.execs_done.saturating_sub(self.previous.1),
            since_previous,
        );

        // AFL++ measures time without finds from the start of the campaign until a find happens
        let time_wo_finds = sample
            .now
            .duration_since(sample.last_find.unwrap_or(start))
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let execs_since_crash = sample.execs_done.saturating_sub(sample.execs_at_last_crash);

        let lines: Vec<(&str, String)> = vec![
            ("start_time", unix_seconds(start).to_string()),
            ("last_update", unix_seconds(sample.now).to_string()),
            ("run_time", run_time.as_secs().to_string()),
            ("fuzzer_pid", self.config.fuzzer_pid.to_string()),
            ("time_wo_finds", time_wo_finds.to_string()),
            ("execs_done", sample.execs_done.to_string()),
            ("execs_per_sec", format!("{execs_per_sec:.2}")),
            ("execs_ps_last_min", format!("{execs_ps_last_min:.2}")),
            ("saved_crashes", sample.crashes.to_string()),
            ("saved_hangs", sample.hangs.to_string()),
            (
                "last_find",
                unix_seconds_or_zero(sample.last_find).to_string(),
            ),
            (
                "last_crash",
                unix_seconds_or_zero(sample.last_crash).to_string(),
            ),
            (
                "last_hang",
                unix_seconds_or_zero(sample.last_hang).to_string(),
            ),
            ("execs_since_crash", execs_since_crash.to_string()),
            ("exec_timeout", self.config.exec_timeout_ms.to_string()),
            ("afl_banner", self.config.instance_name.clone()),
            (
                "afl_version",
                format!("tsffs-{}", env!("CARGO_PKG_VERSION")),
            ),
            ("target_mode", "tsffs".to_string()),
            ("command_line", self.config.command_line.clone()),
        ];

        lines
            .into_iter()
            .map(|(key, value)| format!("{key:<18}: {value}\n"))
            .collect()
    }
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
        "execs: {} ({:.2}/sec), crashes: {}, hangs: {}, last find: {}",
        sample.execs_done,
        rate(sample.execs_done, run_time),
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

    #[test]
    fn status_line_summarizes_the_campaign() {
        let mut s = sample(100, 5000);
        s.crashes = 2;
        s.hangs = 1;
        s.last_find = Some(at(40));
        assert_eq!(
            status_line(at(0), &s),
            "execs: 5000 (50.00/sec), crashes: 2, hangs: 1, last find: 60s ago"
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
