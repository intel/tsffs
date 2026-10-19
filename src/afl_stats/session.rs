// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simics-side glue: owns the live counters and drives the [`InstanceWriter`] on a wall-clock
//! cadence.

use super::{status_line, ConfigError, InstanceConfig, InstanceWriter, StatsSample, WriteNotice};
use crate::Tsffs;
use anyhow::Result;
use simics::{debug, info, warn, AsConfObject};
use std::time::{Duration, SystemTime};

impl Tsffs {
    /// Create this session's Instance and write the first stats. Called once when the session
    /// starts. Configuration mistakes (an invalid `instance_name`, an Instance in use) fail the
    /// session; environmental problems (unwritable directory, no file locking, a locked
    /// `plot_data`) only warn and run without statistics.
    pub fn start_afl_stats(&mut self) -> Result<()> {
        // A repro run must not lock, rotate or overwrite the campaign's Instance
        if !self.afl_stats || self.afl_writer.is_some() || self.repro_testcase.is_some() {
            return Ok(());
        }

        let start_time = self
            .start_time
            .get()
            .copied()
            .unwrap_or_else(SystemTime::now);

        let writer = InstanceWriter::open(InstanceConfig {
            output_directory: self.output_directory.clone(),
            instance_name: self.instance_name.clone(),
            start_time,
            fuzzer_pid: std::process::id(),
            command_line: std::env::args().collect::<Vec<_>>().join(" "),
            exec_timeout_ms: (self.timeout * 1000.0).round() as u64,
            setup_attributes: self.setup_attributes(),
        });

        self.afl_writer = match writer {
            Ok(writer) => Some(writer),
            Err(e) if e.downcast_ref::<ConfigError>().is_some() => return Err(e),
            Err(e) => {
                warn!(
                    self.as_conf_object(),
                    "AFL statistics disabled for this session: {e}"
                );
                return Ok(());
            }
        };

        // The first write waits a full interval, by which time the campaign has executions to
        // report; `afl-whatsup` divides by `execs_done`. A very short session still gets its
        // final write at shutdown.
        self.last_afl_stats_time = Some(SystemTime::now());

        Ok(())
    }

    /// The configuration recorded once in `fuzzer_setup`
    fn setup_attributes(&self) -> Vec<(String, String)> {
        let path = |p: &std::path::Path| p.display().to_string();

        vec![
            ("timeout", self.timeout.to_string()),
            ("start_on_harness", self.start_on_harness.to_string()),
            ("stop_on_harness", self.stop_on_harness.to_string()),
            ("iteration_limit", self.iteration_limit.to_string()),
            ("cmplog", self.cmplog.to_string()),
            ("coverage_reporting", self.coverage_reporting.to_string()),
            ("corpus_directory", path(&self.corpus_directory)),
            ("solutions_directory", path(&self.solutions_directory)),
            (
                "generate_random_corpus",
                self.generate_random_corpus.to_string(),
            ),
            ("keep_all_corpus", self.keep_all_corpus.to_string()),
            ("afl_stats_interval", self.afl_stats_interval.to_string()),
            ("output_directory", path(&self.output_directory)),
            ("instance_name", self.instance_name.clone()),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect()
    }

    /// Take the newest snapshot the fuzzer thread has published, if any. Its corpus and Edge
    /// counts are authoritative and overwrite the live counts bumped from `Interesting` messages.
    fn refresh_from_fuzzer_snapshot(&mut self) {
        if let Some((generation, snapshot)) =
            self.afl_snapshot.newer_than(self.afl_snapshot_generation)
        {
            self.afl_snapshot_generation = generation;
            self.afl_corpus_count = snapshot.corpus_count;
            self.afl_edges_found = snapshot.edges_found;
            self.afl_fuzzer_snapshot = Some(snapshot);
        }
    }

    /// Write AFL statistics if `afl_stats_interval` seconds have passed since the last write,
    /// or unconditionally when `force` is set. Never fails: problems are reported as log
    /// messages so that monitoring cannot stop fuzzing.
    pub fn afl_stats_tick(&mut self, force: bool) {
        if self.afl_writer.is_none() {
            return;
        }

        // Every stop, not only ticks, so the fuzzer's absolute counts never overwrite newer
        // per-stop bumps by more than one stop
        self.refresh_from_fuzzer_snapshot();

        let now = SystemTime::now();

        if !force
            && self.last_afl_stats_time.is_some_and(|last| {
                now.duration_since(last).unwrap_or(Duration::ZERO)
                    < Duration::from_secs(self.afl_stats_interval)
            })
        {
            return;
        }

        self.last_afl_stats_time = Some(now);

        let sample = StatsSample {
            now,
            execs_done: self.iterations as u64,
            crashes: self.solutions as u64,
            hangs: self.timeouts as u64,
            last_find: self.last_find_time,
            last_crash: self.last_crash_time,
            last_hang: self.last_hang_time,
            execs_at_last_crash: self.iterations_at_last_crash as u64,
            corpus_count: self.afl_corpus_count,
            edges_found: self.afl_edges_found,
            total_edges: Tsffs::COVERAGE_MAP_SIZE as u64,
            fuzzer: self.afl_fuzzer_snapshot.clone(),
            branch_targets: self
                .coverage_reporting
                .then_some(self.edges_seen.len() as u64),
        };

        let Some(notice) = self.afl_writer.as_mut().map(|w| w.write(&sample)) else {
            return;
        };

        // The console line does not depend on the files being writable
        let start_time = self.start_time.get().copied().unwrap_or(now);
        info!(
            self.as_conf_object(),
            "{}",
            status_line(start_time, &sample)
        );

        match notice {
            WriteNotice::Quiet => {}
            WriteNotice::Failed(e) => warn!(
                self.as_conf_object(),
                "Failed to write AFL stats (will retry at the next interval): {e}"
            ),
            WriteNotice::StillFailing(e) => {
                debug!(
                    self.as_conf_object(),
                    "Still failing to write AFL stats: {e}"
                )
            }
            WriteNotice::Recovered => {
                info!(self.as_conf_object(), "Writing AFL stats has recovered")
            }
        }
    }
}
