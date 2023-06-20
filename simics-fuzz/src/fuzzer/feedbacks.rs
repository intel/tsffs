use alloc::rc::Rc;
use core::{cell::RefCell, fmt::Debug};
use std::path::PathBuf;

use libafl::{
    alloc,
    bolts::tuples::Named,
    corpus::Testcase,
    events::EventFirer,
    executors::ExitKind,
    feedbacks::{Feedback, MinMapFeedback},
    impl_serdeany,
    inputs::{BytesInput, Input, UsesInput},
    observers::ObserversTuple,
    state::{HasClientPerfMonitor, HasMetadata},
    Error,
};
use serde::{Deserialize, Serialize};

use crate::fuzzer::observers::MappedEdgeMapObserver;

pub type ShrinkMapFeedback<O, S, T> = MinMapFeedback<MappedEdgeMapObserver<O, T>, S, usize>;
