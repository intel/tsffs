// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Logging

use crate::{fuzzer::messages::FuzzerMessage, Tsffs};
use anyhow::{anyhow, Result};
use serde::Serialize;
use simics::{info, AsConfObject};
use std::{fs::OpenOptions, io::Write};

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

#[derive(Clone, Debug, Serialize)]
pub(crate) enum LogMessage {
    Message(String),
    Interesting(LogMessageInteresting),
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
                    self.log(LogMessage::Message(s.clone()))?;
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

                        self.log(LogMessage::Interesting(LogMessageInteresting {
                            indices: indices.clone(),
                            input: input.clone(),
                            edges,
                        }))?;

                        self.edges_seen_since_last.clear();
                    }

                    if self.save_interesting_execution_traces {
                        self.save_execution_trace()?;
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        })?;

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
