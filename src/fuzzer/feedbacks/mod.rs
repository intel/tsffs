// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use libafl::{
    events::EventFirer,
    feedbacks::{Feedback, HasObserverName, IsNovel, MapFeedback, MapFeedbackMetadata, Reducer},
    inputs::HasTargetBytes,
    observers::UsesObserver,
    prelude::{ExitKind, MapObserver, Observer, ObserversTuple, UsesInput},
    state::{HasCorpus, HasNamedMetadata, State},
};
use libafl_bolts::{AsIter, AsSlice, Named};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::{mpsc::Sender, OnceLock},
};

use super::messages::FuzzerMessage;

#[derive(Clone, Debug)]
pub(crate) struct ReportingMapFeedback<N, O, R, S, T> {
    /// The base map
    base: MapFeedback<N, O, R, S, T>,
    /// A sender to send textual reports to
    sender: OnceLock<Sender<FuzzerMessage>>,
}

impl<N, O, R, S, T> UsesObserver<S> for ReportingMapFeedback<N, O, R, S, T>
where
    S: UsesInput,
    O: Observer<S>,
{
    type Observer = O;
}

impl<N, O, R, S, T> Feedback<S> for ReportingMapFeedback<N, O, R, S, T>
where
    N: IsNovel<T>,
    O: MapObserver<Entry = T> + for<'it> AsIter<'it, Item = T>,
    R: Reducer<T>,
    S: State + HasNamedMetadata + HasCorpus,
    S::Input: HasTargetBytes,
    T: Default + Copy + Serialize + for<'de> Deserialize<'de> + PartialEq + Debug + 'static,
{
    fn is_interesting<EM, OT>(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        input: &<S>::Input,
        observers: &OT,
        exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error>
    where
        EM: EventFirer<State = S>,
        OT: ObserversTuple<S>,
    {
        let is_interesting = self
            .base
            .is_interesting(state, manager, input, observers, exit_kind)?;

        if is_interesting {
            let observer = observers
                .match_name::<O>(self.observer_name())
                .ok_or_else(|| {
                    libafl::Error::unknown("Failed to get observer from observers tuple")
                })?;

            let map_state = state
                .named_metadata_map_mut()
                .get_mut::<MapFeedbackMetadata<T>>(self.name())
                .ok_or_else(|| libafl::Error::unknown("Failed to get metadata"))?;

            let len = observer.len();

            if map_state.history_map.len() < len {
                map_state.history_map.resize(len, observer.initial());
            }

            let history_map = map_state.history_map.as_slice();

            let initial = observer.initial();

            let mut interesting_indices = vec![];

            for (i, item) in observer
                .as_iter()
                .copied()
                .enumerate()
                .filter(|(_, item)| *item != initial)
            {
                let existing = unsafe { *history_map.get_unchecked(i) };
                let reduced = R::reduce(existing, item);
                if N::is_novel(existing, reduced) {
                    interesting_indices.push(i);
                }
            }

            self.sender
                .get_mut()
                .and_then(|sender| {
                    sender
                        .send(FuzzerMessage::Interesting {
                            indices: interesting_indices,
                            input: input.target_bytes().as_slice().to_vec(),
                        })
                        .ok()
                })
                .ok_or_else(|| libafl::Error::unknown("Failed to send report"))?;
        }

        if *exit_kind == ExitKind::Crash {
            let observer = observers
                .match_name::<O>(self.observer_name())
                .ok_or_else(|| {
                    libafl::Error::unknown("Failed to get observer from observers tuple")
                })?;

            let map_state = state
                .named_metadata_map_mut()
                .get_mut::<MapFeedbackMetadata<T>>(self.name())
                .ok_or_else(|| libafl::Error::unknown("Failed to get metadata"))?;

            let len = observer.len();

            if map_state.history_map.len() < len {
                map_state.history_map.resize(len, observer.initial());
            }

            let history_map = map_state.history_map.as_slice();

            let initial = observer.initial();

            let mut indices = vec![];

            for (i, item) in observer
                .as_iter()
                .copied()
                .enumerate()
                .filter(|(_, item)| *item != initial)
            {
                let existing = unsafe { *history_map.get_unchecked(i) };
                let reduced = R::reduce(existing, item);
                if N::is_novel(existing, reduced) {
                    indices.push(i);
                }
            }

            self.sender
                .get_mut()
                .and_then(|sender| {
                    sender
                        .send(FuzzerMessage::Crash {
                            indices,
                            input: input.target_bytes().as_slice().to_vec(),
                        })
                        .ok()
                })
                .ok_or_else(|| libafl::Error::unknown("Failed to send report"))?;
        }

        if *exit_kind == ExitKind::Timeout {
            let observer = observers
                .match_name::<O>(self.observer_name())
                .ok_or_else(|| {
                    libafl::Error::unknown("Failed to get observer from observers tuple")
                })?;

            let map_state = state
                .named_metadata_map_mut()
                .get_mut::<MapFeedbackMetadata<T>>(self.name())
                .ok_or_else(|| libafl::Error::unknown("Failed to get metadata"))?;

            let len = observer.len();

            if map_state.history_map.len() < len {
                map_state.history_map.resize(len, observer.initial());
            }

            let history_map = map_state.history_map.as_slice();

            let initial = observer.initial();

            let mut indices = vec![];

            for (i, item) in observer
                .as_iter()
                .copied()
                .enumerate()
                .filter(|(_, item)| *item != initial)
            {
                let existing = unsafe { *history_map.get_unchecked(i) };
                let reduced = R::reduce(existing, item);
                if N::is_novel(existing, reduced) {
                    indices.push(i);
                }
            }

            self.sender
                .get_mut()
                .and_then(|sender| {
                    sender
                        .send(FuzzerMessage::Timeout {
                            indices,
                            input: input.target_bytes().as_slice().to_vec(),
                        })
                        .ok()
                })
                .ok_or_else(|| libafl::Error::unknown("Failed to send report"))?;
        }

        Ok(is_interesting)
    }

    fn init_state(&mut self, state: &mut S) -> Result<(), libafl::Error> {
        self.base.init_state(state)
    }

    fn append_metadata<EM, OT>(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        observers: &OT,
        testcase: &mut libafl::prelude::Testcase<<S>::Input>,
    ) -> Result<(), libafl::Error>
    where
        OT: ObserversTuple<S>,
        EM: EventFirer<State = S>,
    {
        self.base
            .append_metadata(state, manager, observers, testcase)
    }

    fn discard_metadata(&mut self, state: &mut S, input: &<S>::Input) -> Result<(), libafl::Error> {
        self.base.discard_metadata(state, input)
    }
}

impl<N, O, R, S, T> Named for ReportingMapFeedback<N, O, R, S, T> {
    #[inline]
    fn name(&self) -> &str {
        self.base.name()
    }
}

impl<N, O, R, S, T> HasObserverName for ReportingMapFeedback<N, O, R, S, T>
where
    T: PartialEq + Default + Copy + 'static + Serialize + DeserializeOwned + Debug,
    R: Reducer<T>,
    N: IsNovel<T>,
    O: MapObserver<Entry = T>,
    for<'it> O: AsIter<'it, Item = T>,
    S: HasNamedMetadata,
{
    #[inline]
    fn observer_name(&self) -> &str {
        self.base.observer_name()
    }
}

impl<N, O, R, S, T> ReportingMapFeedback<N, O, R, S, T>
where
    T: PartialEq + Default + Copy + 'static + Serialize + DeserializeOwned + Debug,
    R: Reducer<T>,
    O: MapObserver<Entry = T>,
    for<'it> O: AsIter<'it, Item = T>,
    N: IsNovel<T>,
    S: UsesInput + HasNamedMetadata,
{
    #[must_use]
    pub fn new(base: MapFeedback<N, O, R, S, T>, sender: Sender<FuzzerMessage>) -> Self {
        let sender = {
            let lock = OnceLock::new();
            // NOTE: This is ok because initializing a just-created lock is infallible
            lock.set(sender).expect("Failed to set sender");
            lock
        };
        Self { base, sender }
    }
}
