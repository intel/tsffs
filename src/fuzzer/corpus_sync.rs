// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! A LibAFL stage that imports the testcases added to the corpus directory while fuzzing, for
//! example by another fuzzer sharing the directory.
//!
//! LibAFL's `SyncFromDiskStage` cannot be pointed at the directory the on-disk corpus writes
//! to: it loads every recently modified file as an input, including the metadata the corpus
//! keeps in dotfiles next to each entry.

use libafl::{
    inputs::{Input, UsesInput},
    stages::Stage,
    state::UsesState,
    Error, Evaluator,
};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::read_dir,
    io,
    marker::PhantomData,
    path::{Path, PathBuf},
};

/// The testcases in `directory` that are not in `seen` yet, which are added to it: non-empty
/// regular files that are neither dotfiles nor the `.testcase` copies `keep_all_corpus` writes
/// of testcases that were just executed
fn unseen_testcases(directory: &Path, seen: &mut HashSet<OsString>) -> io::Result<Vec<PathBuf>> {
    let mut unseen = Vec::new();

    for entry in read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let lossy_name = name.to_string_lossy();

        if !lossy_name.starts_with('.')
            && !lossy_name.ends_with(".testcase")
            && entry.metadata().is_ok_and(|m| m.is_file() && m.len() > 0)
            && seen.insert(name.clone())
        {
            unseen.push(entry.path());
        }
    }

    Ok(unseen)
}

/// Evaluates each testcase that appears in the corpus directory, once
pub(crate) struct CorpusSyncStage<E, EM, Z> {
    directory: PathBuf,
    /// The names of the files already evaluated
    seen: HashSet<OsString>,
    phantom: PhantomData<(E, EM, Z)>,
}

impl<E, EM, Z> CorpusSyncStage<E, EM, Z> {
    /// The testcases already in `directory` are the initial corpus, which is loaded separately
    pub(crate) fn new(directory: PathBuf) -> io::Result<Self> {
        let mut seen = HashSet::new();
        unseen_testcases(&directory, &mut seen)?;

        Ok(Self {
            directory,
            seen,
            phantom: PhantomData,
        })
    }
}

impl<E, EM, Z> UsesState for CorpusSyncStage<E, EM, Z>
where
    E: UsesState,
{
    type State = E::State;
}

impl<E, EM, Z> Stage<E, EM, Z> for CorpusSyncStage<E, EM, Z>
where
    E: UsesState,
    EM: UsesState<State = E::State>,
    Z: Evaluator<E, EM, State = E::State>,
{
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut E::State,
        manager: &mut EM,
    ) -> Result<(), Error> {
        // shortcut: an entry the fuzzer itself adds to the corpus is evaluated once more here,
        // skip the names of the corpus entries if that execution ever matters
        for path in unseen_testcases(&self.directory, &mut self.seen)? {
            let input = <E::State as UsesInput>::Input::from_file(path)?;
            fuzzer.evaluate_input(state, executor, manager, input)?;
        }

        Ok(())
    }

    #[inline]
    fn restart_progress_should_run(&mut self, _state: &mut Self::State) -> Result<bool, Error> {
        Ok(true)
    }

    #[inline]
    fn clear_restart_progress(&mut self, _state: &mut Self::State) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all, write},
        process,
    };

    #[test]
    fn imports_each_testcase_once_and_skips_corpus_dotfiles() -> io::Result<()> {
        let directory = temp_dir().join(format!("tsffs-corpus-sync-test-{}", process::id()));
        let _ = remove_dir_all(&directory);
        create_dir_all(directory.join("subdirectory"))?;
        write(directory.join("0123456789abcdef"), b"entry")?;
        write(
            directory.join(".0123456789abcdef.metadata"),
            b"{\"metadata\":{}}",
        )?;
        write(directory.join(".0123456789abcdef.lafl_lock"), b"")?;
        write(directory.join("empty"), b"")?;
        write(directory.join("0123456789abcdef.testcase"), b"entry")?;

        let mut seen = HashSet::new();
        assert_eq!(
            unseen_testcases(&directory, &mut seen)?,
            [directory.join("0123456789abcdef")]
        );
        assert!(unseen_testcases(&directory, &mut seen)?.is_empty());

        write(directory.join("from-another-fuzzer"), b"new")?;
        assert_eq!(
            unseen_testcases(&directory, &mut seen)?,
            [directory.join("from-another-fuzzer")]
        );

        remove_dir_all(&directory)
    }
}
