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
    marker::PhantomData,
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
            // Slots no earlier input has occupied: the new Edges. The base feedback only
            // fills the history map later, in `append_metadata`, so it is still unchanged here.
            let mut new_edges = 0;

            for (i, item) in observer
                .as_iter()
                .copied()
                .enumerate()
                .filter(|(_, item)| *item != initial)
            {
                let existing = unsafe { *history_map.get_unchecked(i) };
                if existing == initial {
                    new_edges += 1;
                }
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
                            new_edges,
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

/// Reports solutions to the simulator thread. LibAFL does not evaluate the corpus feedback for
/// an execution the objective accepts, so [`ReportingMapFeedback`] never sees a solution. This
/// feedback accepts every execution: chain it after the feedbacks that decide what a solution is.
#[derive(Clone, Debug)]
pub(crate) struct SolutionReporter<O> {
    /// The name of the coverage map observer
    observer_name: String,
    /// The name of the coverage map feedback, whose history map holds the corpus coverage
    map_name: String,
    sender: Sender<FuzzerMessage>,
    phantom: PhantomData<O>,
}

impl<O> SolutionReporter<O>
where
    O: Named,
{
    #[must_use]
    pub fn new(observer: &O, map_name: &str, sender: Sender<FuzzerMessage>) -> Self {
        Self {
            observer_name: observer.name().to_string(),
            map_name: map_name.to_string(),
            sender,
            phantom: PhantomData,
        }
    }
}

impl<O> Named for SolutionReporter<O> {
    #[inline]
    fn name(&self) -> &str {
        "SolutionReporter"
    }
}

impl<O, S> Feedback<S> for SolutionReporter<O>
where
    O: MapObserver<Entry = u8> + for<'it> AsIter<'it, Item = u8>,
    S: State + HasNamedMetadata,
    S::Input: HasTargetBytes,
{
    fn is_interesting<EM, OT>(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        input: &<S>::Input,
        observers: &OT,
        exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error>
    where
        EM: EventFirer<State = S>,
        OT: ObserversTuple<S>,
    {
        let observer = observers
            .match_name::<O>(&self.observer_name)
            .ok_or_else(|| libafl::Error::unknown("Failed to get observer from observers tuple"))?;
        let initial = observer.initial();
        let history_map = state
            .named_metadata_map()
            .get::<MapFeedbackMetadata<u8>>(&self.map_name)
            .map(|metadata| metadata.history_map.as_slice())
            .unwrap_or_default();
        // The coverage map slots this execution fills that no corpus entry does
        let indices = observer
            .as_iter()
            .copied()
            .enumerate()
            .filter(|(i, item)| {
                *item != initial && history_map.get(*i).is_none_or(|seen| *seen == initial)
            })
            .map(|(i, _)| i)
            .collect();
        let input = input.target_bytes().as_slice().to_vec();

        self.sender
            .send(if *exit_kind == ExitKind::Timeout {
                FuzzerMessage::Timeout { indices, input }
            } else {
                FuzzerMessage::Crash { indices, input }
            })
            .map_err(|_| libafl::Error::unknown("Failed to send report"))?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libafl::{
        prelude::{BytesInput, InMemoryCorpus, NopEventManager, StdMapObserver},
        state::{HasNamedMetadata, StdState},
    };
    use libafl_bolts::{rands::StdRand, tuples::tuple_list};
    use std::sync::mpsc::channel;

    #[test]
    fn reports_crashes_and_timeouts_with_the_edges_only_they_reach() -> Result<(), libafl::Error> {
        let (sender, receiver) = channel();
        // The execution fills slots 1 and 3, the corpus already fills slot 1
        let observer = StdMapObserver::owned("edges", vec![0u8, 1, 0, 1]);
        let mut reporter = SolutionReporter::new(&observer, "history", sender);
        let mut state = StdState::new(
            StdRand::with_seed(0),
            InMemoryCorpus::<BytesInput>::new(),
            InMemoryCorpus::new(),
            &mut (),
            &mut (),
        )?;
        state.add_named_metadata(
            "history",
            MapFeedbackMetadata::<u8> {
                history_map: vec![0, 1, 0, 0],
                num_covered_map_indexes: 1,
            },
        );
        let observers = tuple_list!(observer);
        let mut manager = NopEventManager::new();
        let input = BytesInput::new(b"boom".to_vec());

        assert!(reporter.is_interesting(
            &mut state,
            &mut manager,
            &input,
            &observers,
            &ExitKind::Crash
        )?);
        assert!(matches!(
            receiver.try_recv(),
            Ok(FuzzerMessage::Crash { indices, input }) if indices == [3] && input == b"boom"
        ));

        assert!(reporter.is_interesting(
            &mut state,
            &mut manager,
            &input,
            &observers,
            &ExitKind::Timeout
        )?);
        assert!(matches!(
            receiver.try_recv(),
            Ok(FuzzerMessage::Timeout { indices, input }) if indices == [3] && input == b"boom"
        ));
        Ok(())
    }
}
