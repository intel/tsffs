use core::{
    ffi::c_void,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use libafl::{
    events::{EventFirer, EventRestarter},
    executors::{hooks::ExecutorHooksTuple, HasObservers},
    fuzzer::HasObjective,
    inputs::UsesInput,
    observers::{ObserversTuple, UsesObservers},
    state::{HasCorpus, HasExecutions, HasSolutions, State, UsesState},
    Error,
};

/// The internal state of `GenericInProcessExecutor`.
pub(crate) struct GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    /// The observers, observing each run
    pub(super) observers: OT,
    // Crash and timeout hah
    pub(super) hooks: HT,
    phantom: PhantomData<S>,
}

impl<HT, OT, S> Debug for GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S> + Debug,
    S: State,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenericInProcessExecutorState")
            .field("observers", &self.observers)
            .finish_non_exhaustive()
    }
}

impl<HT, OT, S> UsesState for GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    type State = S;
}

impl<HT, OT, S> UsesObservers for GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    type Observers = OT;
}

impl<HT, OT, S> HasObservers for GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    #[inline]
    fn observers(&self) -> &OT {
        &self.observers
    }

    #[inline]
    fn observers_mut(&mut self) -> &mut OT {
        &mut self.observers
    }
}

impl<HT, OT, S> GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    /// This function marks the boundary between the fuzzer and the target
    #[inline]
    pub(crate) unsafe fn enter_target<EM, Z>(
        &mut self,
        _fuzzer: &mut Z,
        _state: &mut <Self as UsesState>::State,
        _mgr: &mut EM,
        _input: &<Self as UsesInput>::Input,
        _executor_ptr: *const c_void,
    ) {
    }

    /// This function marks the boundary between the fuzzer and the target
    #[inline]
    pub(crate) fn leave_target<EM, Z>(
        &mut self,
        _fuzzer: &mut Z,
        _state: &mut <Self as UsesState>::State,
        _mgr: &mut EM,
        _input: &<Self as UsesInput>::Input,
    ) {
    }
}

impl<HT, OT, S> GenericInProcessExecutorInner<HT, OT, S>
where
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: HasExecutions + HasSolutions + HasCorpus + State,
{
    /// Create a new in mem executor.
    /// Caution: crash and restart in one of them will lead to odd behavior if multiple are used,
    /// depending on different corpus or state.
    /// * `hooks` - the hooks run before and after the harness's execution
    /// * `harness_fn` - the harness, executing the function
    /// * `observers` - the observers observing the target during execution
    /// This may return an error on unix, if signal handler setup fails
    pub(crate) fn new<EM, OF, Z>(
        hooks: HT,
        observers: OT,
        _fuzzer: &mut Z,
        _event_mgr: &mut EM,
    ) -> Result<Self, Error>
    where
        EM: EventFirer<State = S> + EventRestarter,
        Z: HasObjective<Objective = OF, State = S>,
    {
        Ok(Self {
            observers,
            hooks,
            phantom: PhantomData,
        })
    }

    /// The inprocess handlers
    #[inline]
    pub(crate) fn hooks(&self) -> &HT {
        &self.hooks
    }

    /// The inprocess handlers (mutable)
    #[inline]
    pub(crate) fn hooks_mut(&mut self) -> &mut HT {
        &mut self.hooks
    }
}
