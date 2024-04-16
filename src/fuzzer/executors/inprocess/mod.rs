//! The [`InProcessExecutor`] is a libfuzzer-like executor, that will simply call a function.
//! It should usually be paired with extra error-handling, such as a restarting event manager, to be effective.
//!
//! Needs the `fork` feature flag.
#![allow(clippy::needless_pass_by_value)]

use core::{
    borrow::BorrowMut,
    ffi::c_void,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    ptr,
};

use libafl_bolts::tuples::tuple_list;

use libafl::{
    corpus::{Corpus, Testcase},
    events::{Event, EventFirer, EventRestarter},
    executors::{hooks::ExecutorHooksTuple, Executor, ExitKind, HasObservers},
    feedbacks::Feedback,
    fuzzer::HasObjective,
    inputs::UsesInput,
    observers::{ObserversTuple, UsesObservers},
    prelude::HasMetadata,
    state::{HasCorpus, HasExecutions, HasSolutions, State, UsesState},
    Error,
};

use self::inner::GenericInProcessExecutorInner;

/// The inner structure of `InProcessExecutor`.
pub(crate) mod inner;

/// The process executor simply calls a target function, as mutable reference to a closure.
pub(crate) type InProcessExecutor<'a, H, OT, S> = GenericInProcessExecutor<H, &'a mut H, (), OT, S>;

/// The inprocess executor that allows hooks
pub(crate) type HookableInProcessExecutor<'a, H, HT, OT, S> =
    GenericInProcessExecutor<H, &'a mut H, HT, OT, S>;
/// The process executor simply calls a target function, as boxed `FnMut` trait object
pub(crate) type OwnedInProcessExecutor<OT, S> = GenericInProcessExecutor<
    dyn FnMut(&<S as UsesInput>::Input) -> ExitKind,
    Box<dyn FnMut(&<S as UsesInput>::Input) -> ExitKind>,
    (),
    OT,
    S,
>;

/// The inmem executor simply calls a target function, then returns afterwards.
#[allow(dead_code)]
pub(crate) struct GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    harness_fn: HB,
    inner: GenericInProcessExecutorInner<HT, OT, S>,
    phantom: PhantomData<(*const H, HB)>,
}

impl<H, HB, HT, OT, S> Debug for GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S> + Debug,
    S: State,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenericInProcessExecutor")
            .field("inner", &self.inner)
            .field("harness_fn", &"<fn>")
            .finish_non_exhaustive()
    }
}

impl<H, HB, HT, OT, S> UsesState for GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    type State = S;
}

impl<H, HB, HT, OT, S> UsesObservers for GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    type Observers = OT;
}

impl<EM, H, HB, HT, OT, S, Z> Executor<EM, Z> for GenericInProcessExecutor<H, HB, HT, OT, S>
where
    EM: UsesState<State = S>,
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State + HasExecutions,
    Z: UsesState<State = S>,
{
    fn run_target(
        &mut self,
        fuzzer: &mut Z,
        state: &mut Self::State,
        mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, Error> {
        *state.executions_mut() += 1;
        unsafe {
            let executor_ptr = ptr::from_ref(self) as *const c_void;
            self.inner
                .enter_target(fuzzer, state, mgr, input, executor_ptr);
        }
        self.inner.hooks.pre_exec_all(state, input);

        let ret = (self.harness_fn.borrow_mut())(input);

        self.inner.hooks.post_exec_all(state, input);
        self.inner.leave_target(fuzzer, state, mgr, input);
        Ok(ret)
    }
}

impl<H, HB, HT, OT, S> HasObservers for GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State,
{
    #[inline]
    fn observers(&self) -> &OT {
        self.inner.observers()
    }

    #[inline]
    fn observers_mut(&mut self) -> &mut OT {
        self.inner.observers_mut()
    }
}

impl<'a, H, OT, S> InProcessExecutor<'a, H, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    OT: ObserversTuple<S>,
    S: HasExecutions + HasSolutions + HasCorpus + State,
{
    /// Create a new in mem executor.
    /// Caution: crash and restart in one of them will lead to odd behavior if multiple are used,
    /// depending on different corpus or state.
    /// * `user_hooks` - the hooks run before and after the harness's execution
    /// * `harness_fn` - the harness, executing the function
    /// * `observers` - the observers observing the target during execution
    /// This may return an error on unix, if signal handler setup fails
    pub(crate) fn new<EM, OF, Z>(
        harness_fn: &'a mut H,
        observers: OT,
        fuzzer: &mut Z,
        event_mgr: &mut EM,
    ) -> Result<Self, Error>
    where
        Self: Executor<EM, Z, State = S> + HasObservers,
        EM: EventFirer<State = S> + EventRestarter,
        OF: Feedback<S>,
        S: State,
        Z: HasObjective<Objective = OF, State = S>,
    {
        let inner = GenericInProcessExecutorInner::new::<EM, OF, Z>(
            tuple_list!(),
            observers,
            fuzzer,
            event_mgr,
        )?;

        Ok(Self {
            harness_fn,
            inner,
            phantom: PhantomData,
        })
    }
}

impl<H, HB, HT, OT, S> GenericInProcessExecutor<H, HB, HT, OT, S>
where
    H: FnMut(&S::Input) -> ExitKind + ?Sized,
    HB: BorrowMut<H>,
    HT: ExecutorHooksTuple<S>,
    OT: ObserversTuple<S>,
    S: State + HasExecutions + HasSolutions + HasCorpus,
{
    /// Create a new in mem executor.
    /// Caution: crash and restart in one of them will lead to odd behavior if multiple are used,
    /// depending on different corpus or state.
    /// * `user_hooks` - the hooks run before and after the harness's execution
    /// * `harness_fn` - the harness, executing the function
    /// * `observers` - the observers observing the target during execution
    /// This may return an error on unix, if signal handler setup fails
    pub(crate) fn generic<EM, OF, Z>(
        user_hooks: HT,
        harness_fn: HB,
        observers: OT,
        fuzzer: &mut Z,
        event_mgr: &mut EM,
    ) -> Result<Self, Error>
    where
        Self: Executor<EM, Z, State = S> + HasObservers,
        EM: EventFirer<State = S> + EventRestarter,
        OF: Feedback<S>,
        S: State,
        Z: HasObjective<Objective = OF, State = S>,
    {
        let inner = GenericInProcessExecutorInner::new::<EM, OF, Z>(
            user_hooks, observers, fuzzer, event_mgr,
        )?;

        Ok(Self {
            harness_fn,
            inner,
            phantom: PhantomData,
        })
    }

    /// Retrieve the harness function.
    #[inline]
    pub(crate) fn harness(&self) -> &H {
        self.harness_fn.borrow()
    }

    /// Retrieve the harness function for a mutable reference.
    #[inline]
    pub(crate) fn harness_mut(&mut self) -> &mut H {
        self.harness_fn.borrow_mut()
    }

    /// The inprocess handlers
    #[inline]
    pub(crate) fn hooks(&self) -> &HT {
        self.inner.hooks()
    }

    /// The inprocess handlers (mutable)
    #[inline]
    pub(crate) fn hooks_mut(&mut self) -> &mut HT {
        self.inner.hooks_mut()
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
/// Save state if it is an objective
pub(crate) fn run_observers_and_save_state<E, EM, OF, Z>(
    executor: &mut E,
    state: &mut E::State,
    input: &<E::State as UsesInput>::Input,
    fuzzer: &mut Z,
    event_mgr: &mut EM,
    exitkind: ExitKind,
) where
    E: HasObservers,
    EM: EventFirer<State = E::State> + EventRestarter<State = E::State>,
    OF: Feedback<E::State>,
    E::State: HasExecutions + HasSolutions + HasCorpus,
    Z: HasObjective<Objective = OF, State = E::State>,
{
    let observers = executor.observers_mut();

    observers
        .post_exec_all(state, input, &exitkind)
        .expect("Observers post_exec_all failed");

    let interesting = fuzzer
        .objective_mut()
        .is_interesting(state, event_mgr, input, observers, &exitkind)
        .expect("In run_observers_and_save_state objective failure.");

    if interesting {
        let executions = *state.executions();
        let mut new_testcase = Testcase::with_executions(input.clone(), executions);
        new_testcase.add_metadata(exitkind);
        new_testcase.set_parent_id_optional(*state.corpus().current());
        fuzzer
            .objective_mut()
            .append_metadata(state, event_mgr, observers, &mut new_testcase)
            .expect("Failed adding metadata");
        state
            .solutions_mut()
            .add(new_testcase)
            .expect("In run_observers_and_save_state solutions failure.");
        event_mgr
            .fire(
                state,
                Event::Objective {
                    objective_size: state.solutions().count(),
                    executions,
                    time: libafl_bolts::current_time(),
                },
            )
            .expect("Could not save state in run_observers_and_save_state");
    }

    // Serialize the state and wait safely for the broker to read pending messages
    event_mgr.on_restart(state).expect("Failed on restart");
}
