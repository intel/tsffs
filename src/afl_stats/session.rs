// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Simics-side glue: owns the live counters and drives the [`InstanceWriter`] on a wall-clock
//! cadence.

use super::{InstanceConfig, InstanceWriter, StatsSample, WriteNotice};
use crate::Tsffs;
use anyhow::Result;
use simics::{debug, info, warn, AsConfObject};
use std::time::{Duration, SystemTime};

impl Tsffs {
    /// Create this session's Instance and write the first stats. Called once when the session
    /// starts. Configuration mistakes (such as an invalid `instance_name`) fail the session.
    pub fn start_afl_stats(&mut self) -> Result<()> {
        if !self.afl_stats || self.afl_writer.is_some() {
            return Ok(());
        }

        let start_time = self
            .start_time
            .get()
            .copied()
            .unwrap_or_else(SystemTime::now);

        self.afl_writer = Some(InstanceWriter::open(InstanceConfig {
            output_directory: self.output_directory.clone(),
            instance_name: self.instance_name.clone(),
            start_time,
            fuzzer_pid: std::process::id(),
            command_line: std::env::args().collect::<Vec<_>>().join(" "),
            exec_timeout_ms: (self.timeout * 1000.0) as u64,
        })?);

        self.afl_stats_tick(true);

        Ok(())
    }

    /// Write AFL statistics if `afl_stats_interval` seconds have passed since the last write,
    /// or unconditionally when `force` is set. Never fails: problems are reported as log
    /// messages so that monitoring cannot stop fuzzing.
    pub fn afl_stats_tick(&mut self, force: bool) {
        if self.afl_writer.is_none() {
            return;
        }

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
        };

        let Some(notice) = self.afl_writer.as_mut().map(|w| w.write(&sample)) else {
            return;
        };

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
