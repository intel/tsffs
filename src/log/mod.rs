// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Logging

use crate::{fuzzer::messages::FuzzerMessage, Tsffs};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::Serialize;
use simics::{info, AsConfObject};
use std::{fs::OpenOptions, io::Write, time::SystemTime};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LogMessageEdge {
    pub pc: u64,
    pub afl_idx: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LogMessageInteresting {
    pub indices: Vec<usize>,
    pub input: Vec<u8>,
    pub edges: Vec<LogMessageEdge>,
}

pub(crate) type LogMessageSolution = LogMessageInteresting;
pub(crate) type LogMessageTimeout = LogMessageInteresting;

#[derive(Clone, Debug, Serialize)]
pub(crate) enum LogMessage {
    Startup {
        timestamp: String,
    },
    Message {
        timestamp: String,
        message: String,
    },
    Interesting {
        timestamp: String,
        message: LogMessageInteresting,
    },
    Solution {
        timestamp: String,
        message: LogMessageSolution,
    },
    Timeout {
        timestamp: String,
        message: LogMessageTimeout,
    },
    Heartbeat {
        iterations: usize,
        solutions: usize,
        timeouts: usize,
        edges: usize,
        timestamp: String,
    },
}

impl LogMessage {
    pub(crate) fn startup() -> Self {
        Self::Startup {
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub(crate) fn message(message: String) -> Self {
        Self::Message {
            timestamp: Utc::now().to_rfc3339(),
            message,
        }
    }

    pub(crate) fn interesting(
        indices: Vec<usize>,
        input: Vec<u8>,
        edges: Vec<LogMessageEdge>,
    ) -> Self {
        Self::Interesting {
            timestamp: Utc::now().to_rfc3339(),
            message: LogMessageInteresting {
                indices,
                input,
                edges,
            },
        }
    }

    pub(crate) fn heartbeat(
        iterations: usize,
        solutions: usize,
        timeouts: usize,
        edges: usize,
    ) -> Self {
        Self::Heartbeat {
            iterations,
            solutions,
            timeouts,
            edges,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

impl Tsffs {
    pub fn log_messages(&mut self) -> Result<()> {
        let messages = self
            .fuzzer_messages
            .get()
            .map(|m| m.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();

        messages.iter().try_for_each(|m| {
            match m {
                FuzzerMessage::String(s) => {
                    info!(self.as_conf_object(), "Fuzzer message: {s}");
                    self.log(LogMessage::message(s.clone()))?;
                }
                FuzzerMessage::Interesting { indices, input } => {
                    info!(
                        self.as_conf_object(),
                        "Interesting input for AFL indices {indices:?} with input {input:?}"
                    );

                    if !self.edges_seen_since_last.is_empty() {
                        let mut edges = self
                            .edges_seen_since_last
                            .iter()
                            .map(|(p, a)| LogMessageEdge {
                                pc: *p,
                                afl_idx: *a,
                            })
                            .collect::<Vec<_>>();

                        edges.sort_by(|e1, e2| e1.pc.cmp(&e2.pc));

                        info!(
                            self.as_conf_object(),
                            "{} Interesting edges seen since last report ({} edges total)",
                            self.edges_seen_since_last.len(),
                            self.edges_seen.len(),
                        );

                        self.log(LogMessage::interesting(
                            indices.clone(),
                            input.clone(),
                            edges,
                        ))?;

                        self.edges_seen_since_last.clear();
                    }

                    if self.save_interesting_execution_traces {
                        self.save_execution_trace()?;
                    }
                }
                FuzzerMessage::Crash { indices, input } => {
                    info!(
                        self.as_conf_object(),
                        "Solution input for AFL indices {indices:?} with input {input:?}"
                    );

                    if !self.edges_seen_since_last.is_empty() {
                        let mut edges = self
                            .edges_seen_since_last
                            .iter()
                            .map(|(p, a)| LogMessageEdge {
                                pc: *p,
                                afl_idx: *a,
                            })
                            .collect::<Vec<_>>();

                        edges.sort_by(|e1, e2| e1.pc.cmp(&e2.pc));

                        self.log(LogMessage::Solution {
                            timestamp: Utc::now().to_rfc3339(),
                            message: LogMessageSolution {
                                indices: indices.clone(),
                                input: input.clone(),
                                edges,
                            },
                        })?;
                    }

                    if self.save_solution_execution_traces {
                        self.save_execution_trace()?;
                    }
                }
                FuzzerMessage::Timeout { indices, input } => {
                    info!(
                        self.as_conf_object(),
                        "Timeout input for AFL indices {indices:?} with input {input:?}"
                    );

                    if !self.edges_seen_since_last.is_empty() {
                        let mut edges = self
                            .edges_seen_since_last
                            .iter()
                            .map(|(p, a)| LogMessageEdge {
                                pc: *p,
                                afl_idx: *a,
                            })
                            .collect::<Vec<_>>();

                        edges.sort_by(|e1, e2| e1.pc.cmp(&e2.pc));

                        self.log(LogMessage::Timeout {
                            timestamp: Utc::now().to_rfc3339(),
                            message: LogMessageTimeout {
                                indices: indices.clone(),
                                input: input.clone(),
                                edges,
                            },
                        })?;
                    }

                    if self.save_timeout_execution_traces {
                        self.save_execution_trace()?;
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        })?;

        if self.heartbeat {
            let last = self.last_heartbeat_time.get_or_insert_with(SystemTime::now);

            if last.elapsed()?.as_secs() >= self.heartbeat_interval {
                self.log(LogMessage::heartbeat(
                    self.iterations,
                    self.solutions,
                    self.timeouts,
                    self.edges_seen.len(),
                ))?;

                // Set the last heartbeat time
                self.last_heartbeat_time = Some(SystemTime::now());
            }
        }

        Ok(())
    }

    pub fn log<I>(&mut self, item: I) -> Result<()>
    where
        I: Serialize,
    {
        if !self.log_to_file {
            return Ok(());
        }

        let item = serde_json::to_string(&item).expect("Failed to serialize item") + "\n";

        let log = if let Some(log) = self.log.get_mut() {
            log
        } else {
            self.log
                .set(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.log_path)?,
                )
                .map_err(|_| anyhow!("Log already set"))?;
            self.log.get_mut().ok_or_else(|| anyhow!("Log not set"))?
        };

        log.write_all(item.as_bytes())?;

        Ok(())
    }
}
