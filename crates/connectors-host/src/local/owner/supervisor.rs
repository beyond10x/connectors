use super::*;
use connectors_sdk::Secret;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex, Weak,
        atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
};

pub(super) enum Task {
    Ensure {
        resume: bool,
    },
    Validate {
        profile: String,
        secret: Secret,
        capture_epoch: u64,
    },
    Invoke {
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        document: Vec<u8>,
    },
    Write {
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        document: String,
        key: Option<String>,
        proof: Option<Secret>,
        until: Instant,
    },
    ObserveWrite {
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        document: String,
        key: String,
        until: Instant,
    },
    Revalidate {
        connection: String,
        revision: String,
        profile: String,
    },
}
/// The explicit resume actions of contracts/cli/v1alpha1/semantics.md:282-285:
/// connect/repair (an explicit `Ensure`), revalidate and invoke clear a stop's
/// durable suppression. The automatic sweep, credential validation and a write
/// observation never do.
fn resumes_suppression(task: &Task) -> bool {
    matches!(
        task,
        Task::Ensure { resume: true }
            | Task::Invoke { .. }
            | Task::Write { .. }
            | Task::Revalidate { .. }
    )
}
/// Whether a job that finds the adapter process of an earlier adapter entry
/// running stops it and launches the configured one, as it would launch one
/// when none runs: every job that launches the adapter (story
/// revision-change-keeps-a-recovery-path). A credential validation keeps the
/// refusal: it is fenced to the process its `Ensure` captured, and its connect
/// starts again with an `Ensure` that replaces the process. A write
/// observation never reaches a process (it is answered before any launch).
/// Exhaustive, so a new task is classified here before it compiles.
fn replaces_a_stale_child(task: &Task) -> bool {
    match task {
        Task::Ensure { .. }
        | Task::Invoke { .. }
        | Task::Write { .. }
        | Task::Revalidate { .. } => true,
        Task::Validate { .. } | Task::ObserveWrite { .. } => false,
    }
}
/// contracts/cli/v1alpha1/semantics.md §6 "Revalidation retries": at most three
/// validations, 200 ms before the second and 1 s before the third.
const REVALIDATION_PAUSES: [Duration; 2] = [Duration::from_millis(200), Duration::from_secs(1)];
/// What must remain before the deadline after a pause and an attempt as long as
/// the last one.
const REVALIDATION_MARGIN: Duration = Duration::from_millis(500);
/// The least a read whose current evidence would lapse early keeps of that
/// evidence for itself. It keeps half of what is left: its recollection ends
/// halfway to the evidence's expiry, so that a recollection that fails or
/// hangs leaves the read time to relaunch its adapter and be served on the
/// evidence it was admitted on (contracts/cli/v1alpha1/semantics.md). Evidence
/// with no more than twice this left is too short for that, and is recollected
/// within the read's own deadline, never skipped.
const EARLY_READ_MARGIN_MS: u64 = 2_000;

/// The transient class of a failed validation, which a revalidation sends
/// again; `None` for every other failure, a credential (401), permission (403),
/// scope, identity or protocol refusal among them, which is never sent again.
fn transient(failure: runtime::Failure) -> Option<Cause> {
    use runtime::Failure as F;
    match failure {
        F::Unavailable => Some(Cause::ProviderUnavailable),
        F::Timeout | F::ProviderTimeout => Some(Cause::ProviderTimeout),
        F::ProviderRateLimited => Some(Cause::ProviderRateLimited),
        F::ProviderInternal => Some(Cause::ProviderInternal),
        F::ProviderCapacity => Some(Cause::ProviderCapacity),
        _ => None,
    }
}

/// A revalidation's validation, sent again while it fails transiently, the
/// adapter process still runs (`attempt` answers its result and whether the
/// process runs after it), the owner is not `stopping`, and the next pause,
/// an attempt as long as the last one and the margin end before
/// `deadline_ms`. A pooled keep-alive connection the provider closed while
/// the adapter was idle fails one request; the next opens a new connection.
///
/// Answers the last result, the number of attempts sent, and the cause class
/// of a transient final failure.
fn revalidate_with_retries<T>(
    deadline_ms: u64,
    mut attempt: impl FnMut() -> (runtime::Result<T>, bool),
    mut stopping: impl FnMut() -> bool,
    mut pause: impl FnMut(Duration),
) -> (runtime::Result<T>, u32, Option<Cause>) {
    let mut attempts = 0u32;
    loop {
        let started = Instant::now();
        let (result, alive) = attempt();
        attempts += 1;
        let Err(failure) = result else {
            return (result, attempts, None);
        };
        let Some(cause) = transient(failure) else {
            return (result, attempts, None);
        };
        if !alive {
            // The owner itself ends a child whose request passed its deadline.
            let cause = if cause == Cause::ProviderTimeout {
                cause
            } else {
                Cause::AdapterExited
            };
            return (result, attempts, Some(cause));
        }
        if stopping() {
            return (result, attempts, Some(Cause::OwnerStopping));
        }
        let Some(wait) = REVALIDATION_PAUSES.get(attempts as usize - 1).copied() else {
            return (result, attempts, Some(cause));
        };
        let needed = wait + started.elapsed() + REVALIDATION_MARGIN;
        let remaining = deadline_ms.saturating_sub(connectors_sdk::now_ms());
        if needed.as_millis() >= u128::from(remaining) {
            return (result, attempts, Some(cause));
        }
        pause(wait);
        if stopping() {
            return (result, attempts, Some(Cause::OwnerStopping));
        }
    }
}

/// The owner's failure for a revalidation's validation, naming the cause class
/// and the attempts only for a transient one.
fn retried(failure: runtime::Failure, attempts: u32, cause: Option<Cause>) -> Error {
    let mut error = Error::from(failure);
    if cause.is_some() {
        error.cause = cause;
        error.attempts = Some(attempts);
    }
    error
}
pub(super) enum Output {
    Bootstrap(runtime::Bootstrap, u64),
    Baseline(runtime::Baseline),
    Value(Value),
    Write(mutation::Delivery),
}
struct Job {
    alias: String,
    adapter: Adapter,
    task: Task,
    deadline: u64,
    reply: Reply,
    epoch: u64,
    queued: Option<JobQueued>,
    dispatch: Dispatch,
}
/// Whether a read job's worker dispatched it, decided once between the worker
/// and the pool waiting on it: the worker marks it immediately before it
/// records the read use dispatched, and a waiter whose deadline passed first
/// marks it abandoned, after which the worker dispatches nothing. A timeout
/// after dispatch is the provider's (`stage = dispatch`); one before stays
/// the host's own admission (contracts/cli/v1alpha1/semantics.md).
#[derive(Clone, Default)]
struct Dispatch(Arc<AtomicU8>);
impl Dispatch {
    const PENDING: u8 = 0;
    const DISPATCHED: u8 = 1;
    const ABANDONED: u8 = 2;
    /// The worker, before it dispatches: refused once the waiter gave up.
    fn begin(&self) -> Result<()> {
        self.0
            .compare_exchange(
                Self::PENDING,
                Self::DISPATCHED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map(|_| ())
            .map_err(|_| Code::Timeout.into())
    }
    /// The waiter, once its deadline passed: whether the worker had already
    /// dispatched. Either way the worker dispatches nothing after this.
    fn abandon(&self) -> bool {
        self.0
            .compare_exchange(
                Self::PENDING,
                Self::ABANDONED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err_and(|state| state == Self::DISPATCHED)
    }
}
/// Counts a sent job until the worker takes it up, after which the worker's
/// `busy` covers it, or until it is dropped unsent, so an owner never reads a
/// queued job as idle.
struct JobQueued(Arc<AtomicUsize>);
impl JobQueued {
    fn new(count: &Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self(count.clone())
    }
}
impl Drop for JobQueued {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
struct Worker {
    sender: mpsc::SyncSender<Work>,
    control: Arc<Mutex<lifecycle::Control>>,
    thread: std::thread::JoinHandle<()>,
    busy: Arc<AtomicBool>,
    /// Set with `busy` while the worker applies recovery rather than a job.
    recovering: Arc<AtomicBool>,
    recovery_pending: Arc<AtomicBool>,
    /// Jobs sent and not yet taken up; `busy` covers a job once it is.
    queued: Arc<AtomicUsize>,
}
enum Work {
    Invoke(Box<Job>),
    Recover(maintenance::Batch, RecoveryQueued),
}
struct RecoveryQueued(Arc<AtomicBool>);
impl Drop for RecoveryQueued {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
/// The supervisor constructs this between invocations, or while holding its
/// creation lock with no live worker for the instance. Neither state admits a
/// simultaneous native exchange; the lifetime lock excludes another owner.
pub(super) struct Quiescent<'a> {
    instance: &'a str,
    _child: &'a mut Option<runtime::Child>,
}
impl Quiescent<'_> {
    pub(super) fn instance(&self) -> &str {
        self.instance
    }
}
#[derive(Default)]
struct Launches {
    active: Mutex<usize>,
    wake: Condvar,
}
struct Permit<'a>(&'a Launches);
impl Launches {
    fn acquire(&self, deadline: u64) -> Result<Permit<'_>> {
        let mut active = self.active.lock().map_err(|_| Code::Unavailable)?;
        while *active >= 4 {
            let duration = until(deadline)?.saturating_duration_since(Instant::now());
            let (guard, timeout) = self
                .wake
                .wait_timeout(active, duration)
                .map_err(|_| Code::Unavailable)?;
            active = guard;
            if timeout.timed_out() {
                return Err(Code::Timeout.into());
            }
        }
        *active += 1;
        Ok(Permit(self))
    }
}
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            *active -= 1;
            self.0.wake.notify_one();
        }
    }
}
pub(super) struct Pool {
    workers: Mutex<BTreeMap<String, Worker>>,
    paths: Arc<Paths>,
    launches: Arc<Launches>,
    incarnation: String,
    stopped: Arc<AtomicBool>,
    shutdown_lock: Mutex<()>,
    /// Attempts recovery has settled; the owner's idle clock counts only these.
    settled: Arc<AtomicU64>,
    /// This pool, as the handle a worker's job reaches another instance's
    /// worker through: a write that created a credential holds it on its
    /// custody target's worker. Weak, so a worker never keeps its pool alive;
    /// empty for a pool not built by [`Pool::shared`].
    this: Weak<Pool>,
    /// The revalidations queued or running, explicit ones and reads'
    /// recollections alike: a revalidation of the same key joins the one in
    /// flight instead of queueing behind it (contracts/cli/v1alpha1/owner.md).
    revalidations: Flights,
}
/// The revalidations in flight, by instance, connection and semantic
/// revision. Shared with the workers, whose jobs keep their entries listed.
type Flights = Arc<Mutex<BTreeMap<JoinKey, Arc<Joined>>>>;
/// A revalidation's instance, connection and expected semantic revision.
type JoinKey = (String, String, String);
/// What a revalidation in flight answers the revalidations that joined it.
#[derive(Clone)]
enum Answer {
    /// Its own answer: the connection summary, or its refusal.
    Joined(Result<Value>),
    /// A read's recollection that ended with no answer a revalidation would
    /// give: changed mounted material, which a read never re-pins and an
    /// explicit revalidation does, or a recollection that ended before it
    /// answered. Each joiner runs a revalidation of its own.
    Rerun,
}
/// The answer of one revalidation in flight. The first one published is the
/// one every joiner reads.
#[derive(Default)]
struct Joined {
    answer: Mutex<Option<Answer>>,
    wake: Condvar,
}
impl Joined {
    fn publish(&self, answer: Answer) {
        if let Ok(mut slot) = self.answer.lock()
            && slot.is_none()
        {
            *slot = Some(answer);
        }
        self.wake.notify_all();
    }
    fn answered(&self) -> Option<Answer> {
        self.answer.lock().ok().and_then(|slot| slot.clone())
    }
    /// The joined revalidation's answer, or `outcome_unknown` once this
    /// waiter's own deadline passes first, as for an unanswered revalidation.
    fn wait(&self, deadline: u64) -> Result<Answer> {
        let mut slot = self.answer.lock().map_err(|_| Code::Unavailable)?;
        loop {
            if let Some(answer) = slot.as_ref() {
                return Ok(answer.clone());
            }
            let left = until(deadline)
                .map_err(|_| Error::from(Code::OutcomeUnknown))?
                .saturating_duration_since(Instant::now());
            slot = self
                .wake
                .wait_timeout(slot, left)
                .map_err(|_| Code::Unavailable)?
                .0;
        }
    }
}
/// One revalidation in flight. It stays listed for as long as its work can
/// still run: an explicit revalidation's for as long as its worker holds its
/// job (not as long as its caller waits), a read's recollection until it
/// ends. Answered, it leaves the list first, so a revalidation arriving after
/// the answer runs its own; dropped unanswered, it leaves and answers
/// `fallback`.
struct Flight {
    flights: Flights,
    key: JoinKey,
    joined: Arc<Joined>,
    fallback: Answer,
}
/// What finding a key in flight makes of a revalidation.
enum Member {
    Leading(Flight),
    Joined(Arc<Joined>),
}
impl Member {
    /// A read's recollection answers the flight it leads, or the queued
    /// revalidation's flight it found listed under its key.
    fn answer(&self, answer: Answer) {
        match self {
            Member::Leading(flight) => flight.answer(answer),
            Member::Joined(joined) => joined.publish(answer),
        }
    }
}
impl Flight {
    /// Joins the revalidation listed under `key`, or lists a new one.
    fn lead_or_join(flights: &Flights, key: JoinKey, fallback: Answer) -> Result<Member> {
        let mut listed = flights.lock().map_err(|_| Code::Unavailable)?;
        if let Some(joined) = listed.get(&key) {
            return Ok(Member::Joined(joined.clone()));
        }
        let joined = Arc::new(Joined::default());
        listed.insert(key.clone(), joined.clone());
        Ok(Member::Leading(Flight {
            flights: flights.clone(),
            key,
            joined,
            fallback,
        }))
    }
    fn leave(&self) {
        if let Ok(mut listed) = self.flights.lock()
            && listed
                .get(&self.key)
                .is_some_and(|joined| Arc::ptr_eq(joined, &self.joined))
        {
            listed.remove(&self.key);
        }
    }
    /// Its work's answer: it leaves the list, then answers its joiners.
    fn answer(&self, answer: Answer) {
        self.leave();
        self.joined.publish(answer);
    }
}
impl Drop for Flight {
    fn drop(&mut self) {
        self.leave();
        self.joined.publish(self.fallback.clone());
    }
}
/// A job's reply channel. An explicit revalidation's job also holds its
/// flight, so that the flight stays listed while the worker holds the job,
/// and the reply answers its joiners as it answers its caller.
struct Reply {
    sender: mpsc::Sender<Result<Output>>,
    flight: Option<Arc<Flight>>,
}
impl Reply {
    /// Answers the job's flight, then its caller. A caller that stopped
    /// waiting has dropped its receiver; the answer then reaches only the
    /// flight's joiners.
    fn send(self, result: Result<Output>) {
        if let Some(flight) = &self.flight {
            flight.answer(Answer::Joined(match &result {
                Ok(Output::Value(value)) => Ok(value.clone()),
                Ok(_) => Err(Code::Unavailable.into()),
                Err(error) => Err(error.clone()),
            }));
        }
        let _ = self.sender.send(result);
    }
    /// The answer a read's recollection gave this job's flight before the
    /// worker reached the job: the job runs no probe of its own.
    fn answered(&self) -> Option<Result<Value>> {
        match self.flight.as_ref()?.joined.answered()? {
            Answer::Joined(answer) => Some(answer),
            Answer::Rerun => None,
        }
    }
}
impl Pool {
    /// A pool whose workers hold no handle to it: its custody step finds none.
    #[cfg(test)]
    pub fn new(paths: Arc<Paths>, incarnation: String) -> Self {
        Self::with_handle(paths, incarnation, Weak::new())
    }
    /// [`Pool::new`], shared, its workers holding a handle to it.
    pub fn shared(paths: Arc<Paths>, incarnation: String) -> Arc<Self> {
        Arc::new_cyclic(|this| Self::with_handle(paths, incarnation, this.clone()))
    }
    fn with_handle(paths: Arc<Paths>, incarnation: String, this: Weak<Pool>) -> Self {
        Self {
            workers: Mutex::new(BTreeMap::new()),
            paths,
            launches: Arc::new(Launches::default()),
            incarnation,
            stopped: Arc::new(AtomicBool::new(false)),
            shutdown_lock: Mutex::new(()),
            settled: Arc::new(AtomicU64::new(0)),
            this,
            revalidations: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    fn send(
        &self,
        alias: &str,
        adapter: &Adapter,
        task: Task,
        deadline: u64,
        flight: Option<Arc<Flight>>,
    ) -> Result<(mpsc::Receiver<Result<Output>>, Dispatch)> {
        until(deadline)?;
        let mut workers = self.workers.lock().map_err(|_| Code::Unavailable)?;
        if self.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        if !workers.contains_key(&adapter.instance_id) {
            if workers.len() >= 64 {
                return Err(Code::Capacity.into());
            }
            let (sender, receiver) = mpsc::sync_channel(16);
            let (paths, launches) = (self.paths.clone(), self.launches.clone());
            let control = Arc::new(Mutex::new(lifecycle::Control::default()));
            let shared = control.clone();
            let stopped = self.stopped.clone();
            let busy = Arc::new(AtomicBool::new(false));
            let recovering = Arc::new(AtomicBool::new(false));
            let activity = Activity {
                busy: busy.clone(),
                recovering: recovering.clone(),
                settled: self.settled.clone(),
            };
            let recovery_pending = Arc::new(AtomicBool::new(false));
            let instance = adapter.instance_id.clone();
            let pool = self.this.clone();
            let flights = self.revalidations.clone();
            let thread = std::thread::Builder::new()
                .name("connectors-adapter-owner".into())
                .spawn(move || {
                    worker(
                        paths, launches, shared, stopped, activity, instance, receiver, pool,
                        flights,
                    )
                })
                .map_err(|_| Code::Unavailable)?;
            workers.insert(
                adapter.instance_id.clone(),
                Worker {
                    sender,
                    control,
                    thread,
                    busy,
                    recovering,
                    recovery_pending,
                    queued: Arc::new(AtomicUsize::new(0)),
                },
            );
        }
        let worker = &workers[&adapter.instance_id];
        let mut control = worker.control.lock().map_err(|_| Code::Unavailable)?;
        if control.stopping
            && control
                .child
                .as_ref()
                .map(|c| lifecycle::exited(&c.handle))
                .transpose()?
                .unwrap_or(true)
        {
            control.stopping = false;
        }
        control.check(control.epoch)?;
        let (reply, receiver) = mpsc::channel();
        let dispatch = Dispatch::default();
        worker
            .sender
            .try_send(Work::Invoke(Box::new(Job {
                alias: alias.into(),
                adapter: adapter.clone(),
                task,
                deadline,
                reply: Reply {
                    sender: reply,
                    flight,
                },
                epoch: control.epoch,
                queued: Some(JobQueued::new(&worker.queued)),
                dispatch: dispatch.clone(),
            })))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => Code::Capacity,
                mpsc::TrySendError::Disconnected(_) => Code::Unavailable,
            })?;
        Ok((receiver, dispatch))
    }
    /// Runs a job on its instance's worker. A revalidation that finds one of
    /// the same connection at the same expected revision queued or running,
    /// an explicit one or a read's recollection, joins it: it sends no job,
    /// runs no identity probe and answers that revalidation's own answer
    /// (contracts/cli/v1alpha1/owner.md). A read's recollection that ends on
    /// changed mounted material, or without an answer, answers
    /// `Answer::Rerun`: each joiner then runs its own revalidation.
    pub fn run(&self, alias: &str, adapter: &Adapter, task: Task, deadline: u64) -> Result<Output> {
        let Task::Revalidate {
            connection,
            revision,
            ..
        } = &task
        else {
            return self.run_job(alias, adapter, task, deadline);
        };
        let key = (
            adapter.instance_id.clone(),
            connection.clone(),
            revision.clone(),
        );
        let flight = loop {
            let fallback = Answer::Joined(Err(Code::OutcomeUnknown.into()));
            match Flight::lead_or_join(&self.revalidations, key.clone(), fallback)? {
                Member::Leading(flight) => break Arc::new(flight),
                Member::Joined(joined) => match joined.wait(deadline)? {
                    Answer::Joined(answer) => return answer.map(Output::Value),
                    // A read's recollection that ended without new evidence.
                    Answer::Rerun => continue,
                },
            }
        };
        let joined = flight.joined.clone();
        if let Err(error) = self.send(alias, adapter, task, deadline, Some(flight.clone())) {
            joined.publish(Answer::Joined(Err(error.clone())));
            return Err(error);
        }
        // The job now holds the flight; it stays listed until the worker lets
        // the job go, whether or not this wait outlasts it.
        drop(flight);
        match joined.wait(deadline) {
            Ok(Answer::Joined(answer)) => answer.map(Output::Value),
            // The worker may still commit this revalidation after the wait
            // ends; its joiners answer the same `outcome_unknown`.
            Ok(Answer::Rerun) | Err(_) => {
                joined.publish(Answer::Joined(Err(Code::OutcomeUnknown.into())));
                Err(Code::OutcomeUnknown.into())
            }
        }
    }
    fn run_job(&self, alias: &str, adapter: &Adapter, task: Task, deadline: u64) -> Result<Output> {
        let (receiver, dispatch) = self.send(alias, adapter, task, deadline, None)?;
        receiver
            .recv_timeout(until(deadline)?.saturating_duration_since(Instant::now()))
            .map_err(|e| match e {
                // A read its worker already dispatched: the request may have
                // reached the provider, so its deadline is the provider's.
                mpsc::RecvTimeoutError::Timeout if dispatch.abandon() => Error {
                    origin: Origin::Provider,
                    ..Error::from(Code::Timeout)
                },
                mpsc::RecvTimeoutError::Timeout => Code::Timeout.into(),
                _ => Error::from(Code::Unavailable),
            })?
    }
    pub(super) fn recover(&self, batch: maintenance::Batch) -> Result<()> {
        let workers = self.workers.lock().map_err(|_| Code::Unavailable)?;
        if self.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        if let Some(worker) = workers.get(&batch.instance)
            && !worker.thread.is_finished()
        {
            if worker
                .recovery_pending
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return Ok(());
            }
            let queued = RecoveryQueued(worker.recovery_pending.clone());
            worker
                .sender
                .try_send(Work::Recover(batch, queued))
                .map_err(|error| match error {
                    mpsc::TrySendError::Full(_) => Code::Capacity,
                    mpsc::TrySendError::Disconnected(_) => Code::Unavailable,
                })?;
        } else {
            // Keep worker creation excluded until these exact fences finish.
            // A retained finished thread cannot dispatch or be replaced here.
            let instance = batch.instance.clone();
            let mut absent = None;
            let settled = maintenance::apply(
                &self.paths,
                batch,
                Quiescent {
                    instance: &instance,
                    _child: &mut absent,
                },
                &self.stopped,
            )?;
            self.settled.fetch_add(settled as u64, Ordering::SeqCst);
        }
        Ok(())
    }
    /// Latency hint only. Recovery ownership comes from executing on the one
    /// serialized instance worker, never from observing this flag.
    pub fn idle(&self, adapter: &Adapter) -> Result<bool> {
        Ok(self
            .workers
            .lock()
            .map_err(|_| Code::Unavailable)?
            .get(&adapter.instance_id)
            .is_none_or(|worker| !worker.busy.load(Ordering::SeqCst)))
    }
    /// A job is queued or running, or a child is being started: work that
    /// restarts the owner's idle clock. Recovery is not included; see `settled`.
    pub fn working(&self) -> Result<bool> {
        let workers = self.workers.lock().map_err(|_| Code::Unavailable)?;
        for worker in workers.values() {
            if (worker.busy.load(Ordering::SeqCst) && !worker.recovering.load(Ordering::SeqCst))
                || worker.queued.load(Ordering::SeqCst) > 0
                || worker
                    .control
                    .lock()
                    .map_err(|_| Code::Unavailable)?
                    .starting
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// How many attempts recovery has settled so far in this owner.
    pub fn settled(&self) -> u64 {
        self.settled.load(Ordering::SeqCst)
    }
    /// No instance has work in flight: a queued or running job (a child's
    /// request in flight included), a child being started, or queued recovery.
    /// A live adapter child with nothing in flight is not work. The owner's
    /// idle exit reads this; it grants no recovery or lifecycle authority.
    pub fn quiet(&self) -> Result<bool> {
        let workers = self.workers.lock().map_err(|_| Code::Unavailable)?;
        for worker in workers.values() {
            if worker.busy.load(Ordering::SeqCst)
                || worker.recovery_pending.load(Ordering::SeqCst)
                || worker.queued.load(Ordering::SeqCst) > 0
                || worker
                    .control
                    .lock()
                    .map_err(|_| Code::Unavailable)?
                    .starting
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn automatic(&self, config: &Config) {
        for (alias, adapter) in &config.adapters {
            if matches!(adapter.startup, super::super::config::Startup::Automatic) {
                let _ = self.send(
                    alias,
                    adapter,
                    Task::Ensure { resume: false },
                    connectors_sdk::now_ms() + 120_000,
                    None,
                );
            }
        }
    }
    fn control(&self, adapter: &Adapter) -> Result<Option<Arc<Mutex<lifecycle::Control>>>> {
        Ok(self
            .workers
            .lock()
            .map_err(|_| Code::Unavailable)?
            .get(&adapter.instance_id)
            .map(|w| w.control.clone()))
    }
    pub fn status(&self, alias: &str, adapter: &Adapter) -> Result<Value> {
        let suppressed =
            runtime::state::State::new(&self.paths.state).suppressed(&adapter.instance_id)?;
        if let Some(control) = self.control(adapter)? {
            control.lock().map_err(|_| Code::Unavailable)?.observe(
                alias,
                adapter,
                &self.incarnation,
                suppressed,
            )
        } else {
            lifecycle::Control::default().observe(alias, adapter, &self.incarnation, suppressed)
        }
    }
    pub fn stop(
        &self,
        alias: &str,
        adapter: &Adapter,
        revision: &str,
        incarnation: &str,
    ) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let control = self.control(adapter)?.ok_or(Code::IncarnationMismatch)?;
        let handle = {
            let mut control = control.lock().map_err(|_| Code::Unavailable)?;
            let owned = control.child.as_ref().ok_or(Code::IncarnationMismatch)?;
            if owned.incarnation != incarnation
                || owned.revision != revision
                || lifecycle::exited(&owned.handle)?
            {
                return Err(Code::IncarnationMismatch.into());
            }
            let handle = owned.handle.try_clone().map_err(|_| Code::Unavailable)?;
            let next = control.epoch.checked_add(1).ok_or(Code::Capacity)?;
            runtime::state::State::new(&self.paths.state).suppress(&adapter.instance_id, true)?;
            control.epoch = next;
            control.stopping = true;
            handle
        };
        lifecycle::terminate(&handle, deadline)?;
        // Suppression remains set even if a process is still terminating. A new
        // explicit admission can resume only after the exact old pidfd exits.
        let mut control = control.lock().map_err(|_| Code::Unavailable)?;
        control.stopping = !lifecycle::exited(&handle)?;
        control.observe(alias, adapter, &self.incarnation, true)
    }
    pub fn shutdown(&self) -> Result<()> {
        let _shutdown = self.shutdown_lock.lock().map_err(|_| Code::Unavailable)?;
        self.stopped.store(true, Ordering::SeqCst);
        let workers = std::mem::take(&mut *self.workers.lock().map_err(|_| Code::Unavailable)?);
        let mut threads = Vec::new();
        let mut result = Ok(());
        for (_, worker) in workers {
            if let Ok(control) = worker.control.lock() {
                if let Some(child) = &control.child
                    && let Err(error) = lifecycle::kill(&child.handle)
                {
                    result = Err(error);
                }
            } else {
                result = Err(Code::Unavailable.into());
            }
            drop(worker.sender);
            threads.push(worker.thread);
        }
        for thread in threads {
            if thread.join().is_err() {
                result = Err(Code::Unavailable.into());
            }
        }
        result
    }
}
// One argument per piece of state the worker thread shares with its pool.
#[allow(clippy::too_many_arguments)]
fn worker(
    paths: Arc<Paths>,
    launches: Arc<Launches>,
    control: Arc<Mutex<lifecycle::Control>>,
    stopped: Arc<AtomicBool>,
    activity: Activity,
    instance: String,
    receiver: mpsc::Receiver<Work>,
    pool: Weak<Pool>,
    flights: Flights,
) {
    let Activity {
        busy,
        recovering,
        settled,
    } = activity;
    let state = runtime::state::State::new(&paths.state);
    let mut child: Option<runtime::Child> = None;
    let mut selection = String::new();
    for work in receiver {
        // Marked before `busy`, so that recovery never reads as a job.
        recovering.store(matches!(work, Work::Recover(..)), Ordering::SeqCst);
        busy.store(true, Ordering::SeqCst);
        let mut job = match work {
            Work::Invoke(job) => *job,
            Work::Recover(batch, _queued) => {
                if !stopped.load(Ordering::SeqCst) {
                    let applied = maintenance::apply(
                        &paths,
                        batch,
                        Quiescent {
                            instance: &instance,
                            _child: &mut child,
                        },
                        &stopped,
                    );
                    settled.fetch_add(applied.unwrap_or(0) as u64, Ordering::SeqCst);
                }
                busy.store(false, Ordering::SeqCst);
                recovering.store(false, Ordering::SeqCst);
                continue;
            }
        };
        // `busy` now covers this job; it is no longer queued.
        drop(job.queued.take());
        let result = (|| {
            // A read's recollection of the same connection and revision ran
            // while this revalidation was queued, and this one joined it.
            if let Some(answer) = job.reply.answered() {
                return answer.map(Output::Value);
            }
            until(job.deadline)?;
            if stopped.load(Ordering::SeqCst) {
                return Err(Code::Unavailable.into());
            }
            let mut guard = control.lock().map_err(|_| Code::Unavailable)?;
            if let Some(active) = child.as_mut()
                && !active.running()?
            {
                child = None;
                guard.child = None;
                guard.failed = true;
                guard.stopping = false;
            }
            guard.check(job.epoch)?;
            let (config, current) = selected(&paths, &job.alias)?;
            if current.instance_id != job.adapter.instance_id {
                return Err(Code::LifecycleConflict.into());
            }
            if current.selection() != job.adapter.selection() {
                return Err(Code::LifecycleConflict.into());
            }
            if let Task::ObserveWrite {
                connection,
                operation,
                schema,
                revision,
                document,
                key,
                until,
            } = &job.task
            {
                // Earlier jobs have ended. No original native preparation or
                // gate winner can survive this worker's serialized boundary.
                // Clock acquisition must not hold lifecycle or policy locks.
                drop(guard);
                let request = approval_issuance::Request {
                    connection,
                    operation,
                    schema,
                    revision,
                    input: document,
                };
                return mutation::recover_observation(
                    &paths,
                    &job.alias,
                    &request,
                    key,
                    *until,
                    Quiescent {
                        instance: &current.instance_id,
                        _child: &mut child,
                    },
                )
                .map(Output::Write);
            }
            if let Task::Write {
                connection,
                operation,
                schema,
                revision,
                document,
                key,
                until,
                ..
            } = &job.task
            {
                let request = approval_issuance::Request {
                    connection,
                    operation,
                    schema,
                    revision,
                    input: document,
                };
                // Even a queued winner is observed before suppression is lifted
                // or this worker starts any adapter child.
                if let Some(value) =
                    mutation::observe(&paths, &job.alias, &request, key.as_deref(), *until)?
                {
                    return Ok(Output::Write(value));
                }
            }
            if let Task::Validate { capture_epoch, .. } = &job.task {
                if *capture_epoch != job.epoch {
                    return Err(Code::LifecycleConflict.into());
                }
                if child.is_none() {
                    return Err(Code::Unavailable.into());
                }
            }
            let resume = resumes_suppression(&job.task);
            match &job.task {
                Task::Validate { profile, .. } | Task::Revalidate { profile, .. }
                    if !current.permissions.profiles.contains(profile) =>
                {
                    return Err(Code::Forbidden.into());
                }
                Task::Invoke { operation, .. } | Task::Write { operation, .. }
                    if !current.permissions.operations.contains(operation) =>
                {
                    return Err(Code::Forbidden.into());
                }
                _ => {}
            }
            if !resume && state.suppressed(&current.instance_id)? {
                return Err(Code::Unavailable.into());
            }
            if child.is_some() && selection != current.selection() {
                // The running process was launched under an earlier adapter
                // entry and serves nothing the configured one is asked for, so
                // no retry could pass this point. A job that launches the
                // adapter when none runs stops it and launches the configured
                // one, as `adapters stop` and the next launch would. Earlier
                // jobs have ended; a new epoch fences any queued under the old
                // process, among them a credential validation captured by it.
                if !replaces_a_stale_child(&job.task) {
                    return Err(Code::LifecycleConflict.into());
                }
                let next = guard.epoch.checked_add(1).ok_or(Code::Capacity)?;
                guard.epoch = next;
                job.epoch = next;
                guard.child = None;
                // Dropping the child kills its exact process and reaps it.
                child = None;
            }
            if resume {
                state.suppress(&current.instance_id, false)?;
            }
            drop(guard);
            let launch = Launch {
                control: &control,
                launches: &launches,
                state: &state,
                stopped: &stopped,
                current: &current,
                epoch: job.epoch,
            };
            if child.is_none() {
                child = Some(launch.run(job.deadline)?);
                selection = current.selection();
            }
            until(job.deadline)?;
            let active = child.as_mut().ok_or(Code::Unavailable)?;
            match job.task {
                Task::ObserveWrite { .. } => Err(Code::OutcomeUnknown.into()),
                Task::Write {
                    connection,
                    operation,
                    schema,
                    revision,
                    document,
                    key,
                    proof,
                    until,
                } => {
                    let request = approval_issuance::Request {
                        connection: &connection,
                        operation: &operation,
                        schema: &schema,
                        revision: &revision,
                        input: &document,
                    };
                    mutation::execute(
                        &paths,
                        &job.alias,
                        mutation::Invocation {
                            request,
                            key: key.as_deref(),
                            proof,
                            until,
                        },
                        active,
                        mutation::Control {
                            lifecycle: &control,
                            epoch: job.epoch,
                            stopped: &stopped,
                            pool: &pool,
                        },
                    )
                    .map(Output::Write)
                }
                Task::Ensure { .. } => Ok(Output::Bootstrap(active.bootstrap().clone(), job.epoch)),
                Task::Validate {
                    profile, secret, ..
                } => Ok(Output::Baseline(active.validate(
                    &profile,
                    &secret,
                    job.deadline.min(connectors_sdk::now_ms() + 30_000),
                )?)),
                Task::Revalidate {
                    connection,
                    revision,
                    profile,
                } => {
                    let registry = registry::Registry::with_system_clock(&paths.state);
                    let binding = saved_binding(active.bootstrap(), &profile)?;
                    let captured = registry.capture_revalidation(
                        &binding,
                        &connection,
                        &revision,
                        connectors_sdk::now_ms(),
                        job.deadline,
                    )?;
                    revalidate(
                        &Scope {
                            paths: &paths,
                            alias: &job.alias,
                            config: &config,
                            current: &current,
                            control: &control,
                            stopped: &stopped,
                            epoch: job.epoch,
                            deadline: job.deadline,
                            profile: &profile,
                        },
                        active,
                        &registry,
                        &binding,
                        captured,
                        Admission::Explicit {
                            connection: &connection,
                            revision: &revision,
                        },
                    )?;
                    let observed = registry.describe(
                        &current.instance_id,
                        &current.adapter_id,
                        current.configured(),
                        &connection,
                        connectors_sdk::now_ms(),
                        true,
                    )?;
                    Ok(Output::Value(
                        json!({"connection":connection_value(&job.alias,observed)}),
                    ))
                }
                Task::Invoke {
                    connection,
                    operation,
                    schema: expected_schema,
                    revision,
                    document,
                } => {
                    let bootstrap = active.bootstrap().clone();
                    if bootstrap.descriptor()?.revision != revision
                        || schema(&bootstrap, &operation)? != expected_schema
                    {
                        return Err(Code::StaleDescription.into());
                    }
                    let requirement = bootstrap
                        .requirements
                        .iter()
                        .find(|r| r.operation == operation)
                        .ok_or(Code::NotFound)?;
                    if requirement.effect != runtime::Effect::Read {
                        return Err(Code::Unsupported.into());
                    }
                    if !current.permissions.profiles.contains(&requirement.profile) {
                        return Err(Code::Forbidden.into());
                    }
                    runtime::channel::depth(&document)?;
                    let value: Value =
                        connectors_core::read_json(&document).map_err(|_| Code::InvalidInput)?;
                    connectors_sdk::validate(
                        &bootstrap
                            .descriptor()?
                            .operation(&operation)
                            .map_err(|_| Code::NotFound)?
                            .input_schema,
                        &value,
                    )
                    .map_err(|_| Code::InvalidInput)?;
                    let registry = registry::Registry::with_system_clock(&paths.state);
                    let binding = bootstrap.binding(&requirement.profile)?;
                    let capture = || {
                        registry.capture_read(
                            &binding,
                            &connection,
                            &requirement.scopes,
                            connectors_sdk::now_ms(),
                            job.deadline,
                        )
                    };
                    // Evidence still current that would lapse before this read
                    // could use it: under half its own lifetime (not the
                    // profile's bound, which the provider's probe may undercut)
                    // and under the read's own remaining deadline left. The
                    // use, and so the provider deadline, never outlasts the
                    // evidence; fresh evidence admits its read at once.
                    let lapses_early = |captured: &registry::ReadUse| {
                        let now = connectors_sdk::now_ms();
                        let window = (captured.evidence_lifetime_ms() / 2)
                            .min(job.deadline.saturating_sub(now));
                        captured.expires_at_ms() < now.saturating_add(window)
                    };
                    let captured = match capture() {
                        Ok(captured) if !lapses_early(&captured) => captured,
                        // Only the provider executable changed since the
                        // connection was saved (D12): its stored credential is
                        // validated once under this executable, which the
                        // registry follows when the recorded identity answers
                        // and refuses, changing nothing, otherwise. No
                        // credential entry, no question. The evidence that
                        // validation records is fresh, so the read it admits
                        // recollects nothing.
                        Err(registry::Failure::ExecutableChanged) => {
                            super::follow_executable(
                                &paths,
                                &config.selection(),
                                &registry,
                                &binding,
                                &connection,
                                job.deadline,
                                || {
                                    let guard = control.lock().map_err(|_| Code::Unavailable)?;
                                    guard.check(job.epoch)?;
                                    if stopped.load(Ordering::SeqCst) {
                                        return Err(Code::Unavailable.into());
                                    }
                                    Ok(guard)
                                },
                                |material| {
                                    revalidate_with_retries(
                                        job.deadline,
                                        || {
                                            let result = active.validate(
                                                &requirement.profile,
                                                material,
                                                job.deadline,
                                            );
                                            (result, active.running().unwrap_or(false))
                                        },
                                        || stopped.load(Ordering::SeqCst),
                                        std::thread::sleep,
                                    )
                                },
                            )?;
                            capture()?
                        }
                        // Only the evidence lapsed, or is about to, for an
                        // intact credential: recollect it once from the
                        // retained custody version, as `connections
                        // revalidate` would, within this read's deadline, then
                        // capture the read against it. Still not current after
                        // that is the read's refusal.
                        //
                        // A recollection run only because current evidence
                        // would lapse early keeps the read use captured on it:
                        // a failure that records no credential refusal leaves
                        // that evidence current, and the read is served on it.
                        result @ (Ok(_) | Err(registry::Failure::EvidenceExpired)) => {
                            let early = result.ok();
                            let recollected = (|| -> Result<()> {
                                let now = connectors_sdk::now_ms();
                                // Early, it is bounded by the evidence the read
                                // was admitted on, less the half the read keeps
                                // for itself, never by the read's own deadline: a
                                // hanging probe then still leaves the read
                                // served on that evidence. Evidence too short
                                // to leave the recollection as long as the read
                                // keeps is too short to serve the read on
                                // after a failed one: the recollection then
                                // runs as for lapsed evidence, within the
                                // read's deadline, and is never skipped
                                // (contracts/cli/v1alpha1/semantics.md: the
                                // read "recollects it first").
                                let bound = match &early {
                                    Some(early)
                                        if early.expires_at_ms().saturating_sub(now)
                                            > 2 * EARLY_READ_MARGIN_MS =>
                                    {
                                        now + (early.expires_at_ms() - now) / 2
                                    }
                                    _ => job.deadline,
                                };
                                let expires = bound.min(now.saturating_add(30_000));
                                let recollection = registry.capture_recollection(
                                    &binding,
                                    &connection,
                                    now,
                                    expires,
                                )?;
                                // This is the same recollection a revalidation
                                // runs: a revalidation of this connection at
                                // this revision joins it, and one already
                                // queued behind this job takes its answer
                                // (contracts/cli/v1alpha1/owner.md).
                                let member = Flight::lead_or_join(
                                    &flights,
                                    (
                                        job.adapter.instance_id.clone(),
                                        connection.clone(),
                                        recollection.revision().to_owned(),
                                    ),
                                    Answer::Rerun,
                                )?;
                                let joiners = std::cell::Cell::new(None);
                                let revalidated = revalidate(
                                    &Scope {
                                        paths: &paths,
                                        alias: &job.alias,
                                        config: &config,
                                        current: &current,
                                        control: &control,
                                        stopped: &stopped,
                                        epoch: job.epoch,
                                        deadline: expires,
                                        profile: &requirement.profile,
                                    },
                                    active,
                                    &registry,
                                    &binding,
                                    recollection,
                                    Admission::ReadRecollection { joiners: &joiners },
                                );
                                // Its joiners answer what an explicit
                                // revalidation answers: its refusal, a
                                // transient one included, so that none runs a
                                // probe of its own. Only a refusal an explicit
                                // revalidation goes on past (changed mounted
                                // material, which it re-pins) answers them
                                // nothing, and each runs its own: a flight this
                                // recollection leads answers them that as it
                                // is dropped, and a queued revalidation whose
                                // flight it joined runs its own job.
                                if let Err(error) = revalidated {
                                    if let Some(answer @ Answer::Joined(_)) = joiners.take() {
                                        member.answer(answer);
                                    }
                                    return Err(error);
                                }
                                if let Ok(observed) = registry.describe(
                                    &current.instance_id,
                                    &current.adapter_id,
                                    current.configured(),
                                    &connection,
                                    connectors_sdk::now_ms(),
                                    true,
                                ) {
                                    member.answer(Answer::Joined(Ok(json!({
                                        "connection": connection_value(&job.alias, observed)
                                    }))));
                                }
                                Ok(())
                            })();
                            match (recollected, early) {
                                (Ok(()), early) => {
                                    // The recollection's publication moved
                                    // the fence past the early use.
                                    if let Some(early) = early {
                                        registry.cancel_read(early, connectors_sdk::now_ms())?;
                                    }
                                    capture()?
                                }
                                (Err(error), None) => return Err(error),
                                (Err(error), Some(early)) => {
                                    // A credential refusal cut the material
                                    // off: the connection admits no read.
                                    // Anything else left the evidence the
                                    // read was captured on current.
                                    let admitted = registry.admit_read(
                                        &binding,
                                        &connection,
                                        &requirement.scopes,
                                        connectors_sdk::now_ms(),
                                    );
                                    if admitted.is_err() {
                                        registry.cancel_read(early, connectors_sdk::now_ms())?;
                                        return Err(error);
                                    }
                                    // A probe that timed out ended the adapter
                                    // process; the read is served by a new one,
                                    // launched within that evidence.
                                    if !active.running()? {
                                        match launch.run(early.expires_at_ms()) {
                                            Ok(new) => *active = new,
                                            Err(error) => {
                                                registry
                                                    .cancel_read(early, connectors_sdk::now_ms())?;
                                                return Err(error);
                                            }
                                        }
                                    }
                                    early
                                }
                            }
                        }
                        other => other?,
                    };
                    let version = captured.version();
                    let selection = config.selection();
                    let material =
                        custody::Store::open_for(version.scope(), &selection, &paths.state)
                            .and_then(|store| store.read(version));
                    let material = match material {
                        Ok(value) => value,
                        Err(e) => {
                            if super::read_invalidates(&selection, e) {
                                registry.invalidate_read(
                                    &captured,
                                    registry::InvalidCredential::Missing,
                                    connectors_sdk::now_ms(),
                                )?;
                            }
                            registry.cancel_read(captured, connectors_sdk::now_ms())?;
                            return Err(super::read_error(&selection, e));
                        }
                    };
                    let partition = captured.partition().to_owned();
                    let deadline = captured.expires_at_ms();
                    let checked: Result<()> = (|| {
                        let (latest_config, latest) = selected(&paths, &job.alias)?;
                        if latest.selection() != current.selection()
                            || latest_config.selection() != config.selection()
                        {
                            return Err(Code::LifecycleConflict.into());
                        }
                        if !latest.permissions.operations.contains(&operation)
                            || !latest.permissions.profiles.contains(&requirement.profile)
                        {
                            return Err(Code::Forbidden.into());
                        }
                        Ok(())
                    })();
                    if let Err(error) = checked {
                        registry.cancel_read(captured, connectors_sdk::now_ms())?;
                        return Err(error);
                    }
                    let guard = control.lock().map_err(|_| Code::Unavailable)?;
                    if let Err(error) = guard.check(job.epoch).and_then(|_| {
                        if stopped.load(Ordering::SeqCst) {
                            Err(Code::Unavailable.into())
                        } else {
                            Ok(())
                        }
                    }) {
                        registry.cancel_read(captured, connectors_sdk::now_ms())?;
                        return Err(error);
                    }
                    // Refused once the pool stopped waiting and reported the
                    // host's own timeout: nothing is dispatched after that.
                    if let Err(error) = job.dispatch.begin() {
                        registry.cancel_read(captured, connectors_sdk::now_ms())?;
                        return Err(error);
                    }
                    let dispatched = registry.dispatch_read(captured, connectors_sdk::now_ms())?;
                    drop(guard);
                    let result = active.invoke_explained(
                        &operation, &revision, &partition, &material, &document, deadline,
                    );
                    // The provider refused the stored credential as unauthorized
                    // (a refresh token answered `invalid_grant`, a token or basic
                    // entry answered 401): the connection is Stale until a
                    // reconnect publishes (story:connection-stale-state, decision
                    // 15 of story:ess-connector-vocabulary-model). A refused
                    // cached access token is not one (decision 16): the catalog
                    // adapter drops it and sends the read once more with a fresh
                    // exchange, and reports `invalid_credential` only when that
                    // exchange, or its fresh token, is refused too.
                    match &result {
                        Err(refusal) if refusal.failure == runtime::Failure::InvalidCredential => {
                            registry.refuse_read(dispatched, connectors_sdk::now_ms())?
                        }
                        _ => registry.release_read(dispatched, connectors_sdk::now_ms())?,
                    }
                    read_answer(&job.alias, &operation, &revision, result?).map(Output::Value)
                }
            }
        })();
        if result.is_err()
            && let Ok(mut guard) = control.lock()
            && guard.starting
        {
            guard.starting = false;
            guard.failed = true;
        }
        busy.store(false, Ordering::SeqCst);
        job.reply.send(result);
    }
}

/// What a job launches its instance's adapter process under.
struct Launch<'a> {
    control: &'a Mutex<lifecycle::Control>,
    launches: &'a Launches,
    state: &'a runtime::state::State,
    stopped: &'a AtomicBool,
    current: &'a Adapter,
    epoch: u64,
}
impl Launch<'_> {
    /// Launches the configured adapter process by `deadline` and records it
    /// as the instance's child, under this job's epoch.
    fn run(&self, deadline: u64) -> Result<runtime::Child> {
        {
            let mut guard = self.control.lock().map_err(|_| Code::Unavailable)?;
            guard.check(self.epoch)?;
            guard.starting = true;
        }
        let _permit = self.launches.acquire(deadline)?;
        let new = runtime::Child::spawn_until(self.current, until(deadline)?)?;
        let mut guard = self.control.lock().map_err(|_| Code::Unavailable)?;
        guard.check(self.epoch)?;
        if self.stopped.load(Ordering::SeqCst) {
            return Err(Code::Unavailable.into());
        }
        self.state
            .remember(&self.current.selection(), new.bootstrap())?;
        guard.child = Some(lifecycle::Owned {
            handle: new.termination_handle()?,
            incarnation: new.incarnation().into(),
            revision: new.bootstrap().configuration_revision.clone(),
        });
        guard.failed = false;
        guard.starting = false;
        Ok(new)
    }
}

/// What admitted a revalidation (`connectors.cli.LocalRevalidationCause`,
/// ess/domains/cli.yaml).
#[derive(Clone, Copy)]
enum Admission<'a> {
    /// `connections revalidate`: the only cause that re-pins changed mounted
    /// material or follows a configuration upgrade.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Explicit {
        connection: &'a str,
        revision: &'a str,
    },
    /// The one recollection a read runs, first in its own job, when only its
    /// connection's evidence lapsed (contracts/cli/v1alpha1/semantics.md).
    /// A failed one leaves in `joiners` what the revalidations that joined it
    /// answer: the refusal an explicit revalidation gives (owner.md), or
    /// [`Answer::Rerun`] where only an explicit one goes on, re-pinning changed
    /// mounted material.
    ReadRecollection {
        joiners: &'a std::cell::Cell<Option<Answer>>,
    },
}

/// The job a revalidation runs in, on the instance's serialized worker.
#[derive(Clone, Copy)]
struct Scope<'a> {
    paths: &'a Paths,
    alias: &'a str,
    config: &'a Config,
    current: &'a Adapter,
    control: &'a Mutex<lifecycle::Control>,
    stopped: &'a AtomicBool,
    epoch: u64,
    /// The validation's deadline, at most the captured use's expiry.
    deadline: u64,
    profile: &'a str,
}

/// Validates the captured custody version's bytes with the provider and
/// publishes the recollected baseline under the unchanged generation, or
/// records the provider's refusal (`connections revalidate`, owner.md). Under
/// the read-only binding, bytes no longer the pinned ones are re-pinned only
/// for an explicit revalidation; a read's recollection refuses them as its
/// read would.
fn revalidate(
    scope: &Scope<'_>,
    active: &mut runtime::Child,
    registry: &registry::Registry,
    binding: &registry::Binding,
    captured: registry::Revalidation,
    admission: Admission<'_>,
) -> Result<()> {
    // Only `connections revalidate` carries `cause` and `attempts`
    // (contracts/cli/v1alpha1/semantics.md, "Revalidation retries"); a read's
    // recollection answers the read's own failure without them, and its
    // joiners, revalidations themselves, the failure with them.
    let answer = |mut error: Error| -> Error {
        if let Admission::ReadRecollection { joiners } = admission {
            let joined = joiners
                .take()
                .unwrap_or_else(|| Answer::Joined(Err(error.clone())));
            joiners.set(Some(joined));
            error.cause = None;
            error.attempts = None;
        }
        error
    };
    revalidate_inner(scope, active, registry, binding, captured, admission).map_err(answer)
}

fn revalidate_inner(
    scope: &Scope<'_>,
    active: &mut runtime::Child,
    registry: &registry::Registry,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] binding: &registry::Binding,
    captured: registry::Revalidation,
    admission: Admission<'_>,
) -> Result<()> {
    let Scope {
        paths,
        alias,
        config,
        current,
        control,
        stopped,
        epoch,
        deadline,
        profile,
    } = *scope;
    let version = captured.version();
    let selection = config.selection();
    let read = custody::Store::open_for(version.scope(), &selection, &paths.state)
        .and_then(|store| store.read(version));
    let check = || -> Result<()> {
        let (latest_config, latest) = selected(paths, alias)?;
        if latest.selection() != current.selection()
            || latest_config.selection() != config.selection()
        {
            return Err(Code::LifecycleConflict.into());
        }
        if !latest.permissions.profiles.iter().any(|p| p == profile) {
            return Err(Code::Forbidden.into());
        }
        if stopped.load(Ordering::SeqCst) {
            return Err(Error {
                cause: Some(Cause::OwnerStopping),
                ..Error::from(Code::Unavailable)
            });
        }
        Ok(())
    };
    let material = match read {
        Ok(value) => value,
        // The mounted item's bytes are no longer the pinned ones: re-read it,
        // validate the new bytes with the provider and publish them as a new
        // version, without re-entry. The old bytes' version is retired.
        #[cfg(target_os = "linux")]
        Err(custody::Failure::Missing) if selection.read_only() => {
            registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
            let Admission::Explicit {
                connection,
                revision,
            } = admission
            else {
                // A read never re-pins changed material; each revalidation
                // that joined its recollection goes on to re-pin it.
                if let Admission::ReadRecollection { joiners } = admission {
                    joiners.set(Some(Answer::Rerun));
                }
                return Err(super::read_error(&selection, custody::Failure::Missing));
            };
            let acquired =
                registry.begin_repair(binding, connection, revision, connectors_sdk::now_ms())?;
            let claim = registry.consume(acquired, connectors_sdk::now_ms())?;
            let sent = std::cell::Cell::new(0u32);
            let published = super::publish_mounted(
                paths,
                &selection,
                registry,
                &claim,
                alias,
                profile,
                |secret| {
                    control
                        .lock()
                        .map_err(|_| Code::Unavailable)?
                        .check(epoch)
                        .and_then(|_| check())?;
                    let (result, attempts, cause) = revalidate_with_retries(
                        deadline,
                        || {
                            let result = active.validate(profile, &secret, deadline);
                            (result, active.running().unwrap_or(false))
                        },
                        || stopped.load(Ordering::SeqCst),
                        std::thread::sleep,
                    );
                    sent.set(attempts);
                    result
                        .map(runtime::Baseline::into_registry)
                        .map_err(|failure| retried(failure, attempts, cause))
                },
                || {
                    control
                        .lock()
                        .map_err(|_| Code::Unavailable)?
                        .check(epoch)
                        .and_then(|_| check())
                },
            );
            if let Err(error) = published {
                let mut error = error;
                if sent.get() > 0 && error.attempts.is_none() {
                    error.attempts = Some(sent.get());
                }
                if error.code != Code::OutcomeUnknown {
                    registry.fail(&claim, connectors_sdk::now_ms())?;
                }
                return Err(error);
            }
            return Ok(());
        }
        Err(error) => {
            if super::read_invalidates(&selection, error) {
                registry.missing_revalidation(captured, connectors_sdk::now_ms())?;
            } else {
                registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
            }
            return Err(match admission {
                Admission::Explicit { .. } => Code::CustodyUnavailable.into(),
                Admission::ReadRecollection { joiners } => {
                    joiners.set(Some(Answer::Joined(Err(Code::CustodyUnavailable.into()))));
                    super::read_error(&selection, error)
                }
            });
        }
    };
    let guard = control.lock().map_err(|_| Code::Unavailable)?;
    if let Err(error) = guard.check(epoch).and_then(|_| check()) {
        registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
        return Err(error);
    }
    // Only an explicit revalidation follows a configuration upgrade; a read's
    // recollection is captured against the connection's own binding.
    let upgrade = captured.upgrades();
    let dispatched = registry.dispatch_revalidation(captured, connectors_sdk::now_ms())?;
    drop(guard);
    let (result, attempts, cause) = revalidate_with_retries(
        deadline,
        || {
            let result = active.validate(profile, &material, deadline);
            (result, active.running().unwrap_or(false))
        },
        || stopped.load(Ordering::SeqCst),
        std::thread::sleep,
    );
    let guard = control.lock().map_err(|_| Code::Unavailable)?;
    if let Err(error) = guard.check(epoch).and_then(|_| check()) {
        registry.finish_revalidation(dispatched, Err(None), connectors_sdk::now_ms())?;
        let mut error = error;
        if attempts > 0 {
            error.attempts = Some(attempts);
        }
        return Err(error);
    }
    match result {
        Ok(baseline) => registry.finish_revalidation(
            dispatched,
            Ok(baseline.into_registry()),
            connectors_sdk::now_ms(),
        )?,
        Err(error) => {
            let reason = match error {
                runtime::Failure::InvalidCredential | runtime::Failure::IdentityMismatch => {
                    Some(registry::InvalidCredential::Invalid)
                }
                runtime::Failure::InsufficientScope => {
                    Some(registry::InvalidCredential::Insufficient)
                }
                _ => None,
            };
            registry.finish_revalidation(dispatched, Err(reason), connectors_sdk::now_ms())?;
            // Repair refuses the changed binding of an upgrade, so a
            // credential the new provider refuses needs a new connection.
            let mut error = retried(error, attempts, cause);
            error.reconnect = upgrade && reason.is_some();
            return Err(error);
        }
    }
    drop(guard);
    Ok(())
}

/// The answer to a read: the provider result as the JSON value the adapter
/// answered, never a string holding its JSON text
/// (`contracts/cli/v1alpha1/semantics.md` section 6). The child has already
/// held the document to the operation's output schema; one that is not a single
/// unambiguous JSON value here is the adapter's fault.
fn read_answer(alias: &str, operation: &str, revision: &str, document: Vec<u8>) -> Result<Value> {
    let result: Value = connectors_core::read_json(&document).map_err(|_| Code::Unavailable)?;
    Ok(json!({"adapter":alias,"operation":operation,"revision":revision,"result":result}))
}

/// A worker's markers, shared with the pool.
struct Activity {
    busy: Arc<AtomicBool>,
    recovering: Arc<AtomicBool>,
    settled: Arc<AtomicU64>,
}

#[cfg(test)]
mod maintenance_tests {
    use super::*;

    /// story:revision-change-keeps-a-recovery-path: every job that launches an
    /// adapter when none runs, a write among them, replaces the process of an
    /// earlier adapter entry instead of answering a conflict no retry passes;
    /// only a validation fenced to its captured process keeps the refusal.
    #[test]
    fn every_launching_job_replaces_the_process_of_an_earlier_entry() {
        let text = || String::from("x");
        let launching = [
            Task::Ensure { resume: true },
            Task::Ensure { resume: false },
            Task::Invoke {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: Vec::new(),
            },
            Task::Write {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: text(),
                key: None,
                proof: None,
                until: Instant::now(),
            },
            Task::Revalidate {
                connection: text(),
                revision: text(),
                profile: text(),
            },
        ];
        for task in &launching {
            assert!(replaces_a_stale_child(task));
        }
        let fenced = [
            Task::Validate {
                profile: text(),
                secret: Secret(Vec::new()),
                capture_epoch: 0,
            },
            Task::ObserveWrite {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: text(),
                key: text(),
                until: Instant::now(),
            },
        ];
        for task in &fenced {
            assert!(!replaces_a_stale_child(task));
        }
    }

    #[test]
    fn a_live_worker_with_an_idle_hint_queues_one_recovery_instead_of_taking_ownership() {
        let root = tempfile::tempdir().unwrap();
        let paths = Arc::new(Paths {
            config: root.path().join("unread-config"),
            state: root.path().join("unopened-state"),
        });
        let pool = Pool::new(paths, uuid::Uuid::new_v4().to_string());
        let (sender, receiver) = mpsc::sync_channel(16);
        // An unresponsive live worker is deliberately not executing recovery.
        // Dropping release also ends it if any assertion panics.
        let (release, blocked) = mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            let _ = blocked.recv();
        });
        let queued = Arc::new(AtomicBool::new(false));
        pool.workers.lock().unwrap().insert(
            "instance".into(),
            Worker {
                sender,
                thread,
                control: Arc::new(Mutex::new(lifecycle::Control::default())),
                busy: Arc::new(AtomicBool::new(false)),
                recovering: Arc::new(AtomicBool::new(false)),
                recovery_pending: queued.clone(),
                queued: Arc::new(AtomicUsize::new(0)),
            },
        );
        let reference = crate::local::mutations::AttemptRef {
            authority: uuid::Uuid::new_v4(),
            attempt_id: uuid::Uuid::new_v4(),
        };
        let batch = || maintenance::Batch::without_clock("instance".into(), vec![reference]);
        pool.recover(batch()).unwrap();
        pool.recover(batch()).unwrap();
        let Work::Recover(observed, token) = receiver.try_recv().unwrap() else {
            panic!("wrong work kind");
        };
        assert_eq!(observed.references, vec![reference]);
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert!(queued.load(Ordering::SeqCst));
        assert!(!pool.quiet().unwrap(), "queued recovery is work");
        drop(token);
        assert!(!queued.load(Ordering::SeqCst));
        assert!(pool.quiet().unwrap());
        pool.recover(batch()).unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Work::Recover(_, _))));
        assert!(!root.path().join("unopened-state").exists());
        drop(release);
        pool.shutdown().unwrap();
    }
}

#[cfg(test)]
mod suppression_tests {
    use super::*;

    // contracts/cli/v1alpha1/semantics.md:281-288: only an explicit connect,
    // repair, revalidate or invoke resumes a stopped entry; the automatic sweep
    // and every other task leave the durable suppression in place.
    #[test]
    fn only_explicit_resume_actions_clear_stop_suppression() {
        let until = Instant::now();
        let text = String::new;
        let resuming = [
            Task::Ensure { resume: true },
            Task::Invoke {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: Vec::new(),
            },
            Task::Write {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: text(),
                key: None,
                proof: None,
                until,
            },
            Task::Revalidate {
                connection: text(),
                revision: text(),
                profile: text(),
            },
        ];
        for task in &resuming {
            assert!(resumes_suppression(task));
        }
        let passive = [
            Task::Ensure { resume: false },
            Task::Validate {
                profile: text(),
                secret: Secret(Vec::new()),
                capture_epoch: 0,
            },
            Task::ObserveWrite {
                connection: text(),
                operation: text(),
                schema: text(),
                revision: text(),
                document: text(),
                key: text(),
                until,
            },
        ];
        for task in &passive {
            assert!(!resumes_suppression(task));
        }
    }
}

#[cfg(test)]
mod idle_tests {
    use super::*;
    use crate::local::config::{Executable, Startup};

    /// story:owner-idle-exit: a live adapter child with nothing in flight is
    /// not work, so an `Automatic` adapter cannot keep an owner alive; a job in
    /// flight is. The owner's retirement stops the idle child through the same
    /// `shutdown` a shutdown request uses.
    #[test]
    fn a_live_idle_child_leaves_the_pool_quiet_and_a_request_in_flight_does_not() {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        let (path, sha256) = crate::local::test_executable::path_and_sha256();
        let adapter = Adapter {
            credential_revision: None,
            instance_id: "fixture".into(),
            adapter_id: "fixture".into(),
            configuration_revision: "fixture-config".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Startup::Automatic,
            restart: Default::default(),
            permissions: Default::default(),
            executable: Executable {
                path,
                sha256,
                args: vec![
                    "--exact".into(),
                    "local::runtime::process::write_tests::fixture".into(),
                    "--".into(),
                    "write-mode=idle".into(),
                    format!("write-root={}", root.path().display()),
                ],
            },
        };
        config.adapters.insert("fixture".into(), adapter);
        std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
        let pool = Pool::new(Arc::new(paths), uuid::Uuid::new_v4().to_string());
        pool.automatic(&config);

        // The automatic start is work until its child is ready and the job ends.
        let until = Instant::now() + Duration::from_secs(20);
        let (handle, busy) = loop {
            let ready = {
                let workers = pool.workers.lock().unwrap();
                workers.get("fixture").and_then(|worker| {
                    let control = worker.control.lock().unwrap();
                    let child = control.child.as_ref()?;
                    (!worker.busy.load(Ordering::SeqCst) && !control.starting)
                        .then(|| (child.handle.try_clone().unwrap(), worker.busy.clone()))
                })
            };
            if let Some(ready) = ready {
                break ready;
            }
            assert!(
                Instant::now() < until,
                "the automatic child never became ready"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(!lifecycle::exited(&handle).unwrap(), "the child is live");
        assert!(pool.quiet().unwrap(), "a live idle child is not work");

        // `busy` is the worker's in-flight marker for a job's whole duration.
        busy.store(true, Ordering::SeqCst);
        assert!(!pool.quiet().unwrap(), "a request in flight is work");
        busy.store(false, Ordering::SeqCst);
        assert!(pool.quiet().unwrap());

        pool.shutdown().unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while !lifecycle::exited(&handle).unwrap() {
            assert!(Instant::now() < until, "the idle child outlived shutdown");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// story:failed-connect-reports-its-cause: a revalidation the pool stopped
/// waiting for may still be committed by its worker, so its outcome is
/// unknown; other tasks keep the definite wait failure.
#[cfg(test)]
mod revalidate_wait_tests {
    use super::*;
    use crate::local::config::{Executable, Startup};

    fn adapter() -> Adapter {
        Adapter {
            credential_revision: None,
            instance_id: "instance".into(),
            adapter_id: "adapter".into(),
            configuration_revision: "config".into(),
            protocol: "v1alpha1".into(),
            private_protocol: None,
            startup: Startup::OnDemand,
            restart: Default::default(),
            permissions: Default::default(),
            executable: Executable {
                path: "/not-launched".into(),
                sha256: "a".repeat(64),
                args: Vec::new(),
            },
        }
    }

    /// A pool whose one worker takes every job and either keeps it unanswered
    /// until shutdown or drops it at once.
    fn pool(drop_jobs: bool) -> Pool {
        let root = tempfile::tempdir().unwrap();
        let pool = Pool::new(
            Arc::new(Paths {
                config: root.path().join("unread-config"),
                state: root.path().join("unopened-state"),
            }),
            uuid::Uuid::new_v4().to_string(),
        );
        let (sender, receiver) = mpsc::sync_channel::<Work>(16);
        let thread = std::thread::spawn(move || {
            let mut held = Vec::new();
            while let Ok(work) = receiver.recv() {
                if !drop_jobs {
                    held.push(work);
                }
            }
        });
        pool.workers.lock().unwrap().insert(
            "instance".into(),
            Worker {
                sender,
                thread,
                control: Arc::new(Mutex::new(lifecycle::Control::default())),
                busy: Arc::new(AtomicBool::new(false)),
                recovering: Arc::new(AtomicBool::new(false)),
                recovery_pending: Arc::new(AtomicBool::new(false)),
                queued: Arc::new(AtomicUsize::new(0)),
            },
        );
        pool
    }

    fn revalidate() -> Task {
        Task::Revalidate {
            connection: "connection".into(),
            revision: "revision".into(),
            profile: "profile".into(),
        }
    }

    fn code(pool: &Pool, task: Task) -> Code {
        match pool.run("alias", &adapter(), task, connectors_sdk::now_ms() + 300) {
            Err(error) => error.code,
            Ok(_) => panic!("the stand-in worker never answers"),
        }
    }

    #[test]
    fn a_revalidation_the_pool_stopped_waiting_for_is_outcome_unknown() {
        for (drop_jobs, other) in [(false, Code::Timeout), (true, Code::Unavailable)] {
            let pool = pool(drop_jobs);
            assert_eq!(code(&pool, revalidate()), Code::OutcomeUnknown);
            assert_eq!(code(&pool, Task::Ensure { resume: false }), other);
            pool.shutdown().unwrap();
        }
    }

    /// A pool whose one worker answers each job after `delay` with the number
    /// of jobs it has taken up, counting them in the returned counter.
    fn answering(delay: Duration) -> (Pool, Arc<AtomicUsize>) {
        let pool = pool(true);
        let jobs = Arc::new(AtomicUsize::new(0));
        let taken = jobs.clone();
        let (sender, receiver) = mpsc::sync_channel::<Work>(16);
        let thread = std::thread::spawn(move || {
            while let Ok(work) = receiver.recv() {
                if let Work::Invoke(job) = work {
                    let n = taken.fetch_add(1, Ordering::SeqCst) + 1;
                    std::thread::sleep(delay);
                    job.reply.send(Ok(Output::Value(json!(n))));
                }
            }
        });
        let mut workers = pool.workers.lock().unwrap();
        let worker = workers.get_mut("instance").unwrap();
        worker.sender = sender;
        let _ = std::mem::replace(&mut worker.thread, thread);
        drop(workers);
        (pool, jobs)
    }

    fn task(revision: &str) -> Task {
        Task::Revalidate {
            connection: "connection".into(),
            revision: revision.into(),
            profile: "profile".into(),
        }
    }

    /// story:connection-lease-holds-unattended, contracts/cli/v1alpha1/owner.md:
    /// a revalidation arriving while one of the same connection and expected
    /// revision is in flight joins it and answers its answer, sending no job of
    /// its own; one at another revision, or arriving after the answer, runs its
    /// own.
    #[test]
    fn a_revalidation_in_flight_is_joined_not_queued_behind() {
        let (pool, jobs) = answering(Duration::from_millis(600));
        let run = |revision: &str| {
            pool.run(
                "alias",
                &adapter(),
                task(revision),
                connectors_sdk::now_ms() + 5_000,
            )
        };
        let value = |output: Result<Output>| match output {
            Ok(Output::Value(value)) => value,
            _ => panic!("the stand-in worker answers a value"),
        };
        let started = Instant::now();
        let [first, second, other] = std::thread::scope(|scope| {
            let first = scope.spawn(|| value(run("revision")));
            std::thread::sleep(Duration::from_millis(100));
            let second = scope.spawn(|| value(run("revision")));
            let other = scope.spawn(|| value(run("other")));
            [first, second, other].map(|t| t.join().unwrap())
        });
        assert_eq!(first, second, "the joiner answers the joined answer");
        assert_ne!(first, other, "another revision never joins");
        assert_eq!(jobs.load(Ordering::SeqCst), 2);
        // The joiner did not wait for a second probe behind the first.
        assert!(started.elapsed() < Duration::from_millis(1_800));
        assert!(pool.revalidations.lock().unwrap().is_empty());
        let later = value(run("revision"));
        assert_ne!(later, first, "a revalidation after the answer runs its own");
        assert_eq!(jobs.load(Ordering::SeqCst), 3);
        pool.shutdown().unwrap();
    }

    /// A joiner whose own deadline passes before the joined answer is
    /// `outcome_unknown`, as an unanswered revalidation is.
    #[test]
    fn a_joiner_past_its_own_deadline_is_outcome_unknown() {
        let (pool, _) = answering(Duration::from_millis(1_000));
        std::thread::scope(|scope| {
            let leader = scope.spawn(|| {
                pool.run(
                    "alias",
                    &adapter(),
                    task("revision"),
                    connectors_sdk::now_ms() + 5_000,
                )
            });
            std::thread::sleep(Duration::from_millis(100));
            match pool.run(
                "alias",
                &adapter(),
                task("revision"),
                connectors_sdk::now_ms() + 200,
            ) {
                Err(error) => assert_eq!(error.code, Code::OutcomeUnknown),
                Ok(_) => panic!("the joiner's deadline passes first"),
            }
            assert!(leader.join().unwrap().is_ok());
        });
        pool.shutdown().unwrap();
    }

    /// A revalidation whose own wait ended `outcome_unknown` stays in flight
    /// while its worker still holds its job: a revalidation arriving then
    /// joins it and answers the same `outcome_unknown` (owner.md), sending no
    /// second job. Once the worker lets the job go, the next one runs its own.
    #[test]
    fn a_revalidation_stays_in_flight_while_its_worker_holds_the_job() {
        let (pool, jobs) = answering(Duration::from_millis(1_000));
        let run = |ms: u64| {
            pool.run(
                "alias",
                &adapter(),
                task("revision"),
                connectors_sdk::now_ms() + ms,
            )
        };
        let unknown = |output: Result<Output>| match output {
            Err(error) => assert_eq!(error.code, Code::OutcomeUnknown),
            Ok(_) => panic!("the in-flight revalidation's wait ended outcome_unknown"),
        };
        unknown(run(300));
        // The leader stopped waiting; its job still runs on the worker.
        assert_eq!(pool.revalidations.lock().unwrap().len(), 1);
        let started = Instant::now();
        unknown(run(5_000));
        assert!(started.elapsed() < Duration::from_millis(300));
        std::thread::sleep(Duration::from_millis(1_000));
        assert_eq!(jobs.load(Ordering::SeqCst), 1, "a second job was sent");
        assert!(pool.revalidations.lock().unwrap().is_empty());
        assert!(
            run(5_000).is_ok(),
            "a revalidation after the job ran its own"
        );
        assert_eq!(jobs.load(Ordering::SeqCst), 2);
        pool.shutdown().unwrap();
    }

    fn key() -> JoinKey {
        ("instance".into(), "connection".into(), "revision".into())
    }

    /// A read's recollection that finds an explicit revalidation of its key
    /// queued behind its own job answers that revalidation's flight, so the
    /// queued job runs no probe of its own.
    #[test]
    fn a_reads_recollection_answers_a_queued_revalidation_of_its_key() {
        let flights: Flights = Arc::default();
        let fallback = || Answer::Joined(Err(Code::OutcomeUnknown.into()));
        let Member::Leading(flight) = Flight::lead_or_join(&flights, key(), fallback()).unwrap()
        else {
            panic!("nothing was in flight");
        };
        let (sender, _receiver) = mpsc::channel();
        let reply = Reply {
            sender,
            flight: Some(Arc::new(flight)),
        };
        assert!(reply.answered().is_none());
        let read = Flight::lead_or_join(&flights, key(), Answer::Rerun).unwrap();
        assert!(matches!(read, Member::Joined(_)));
        read.answer(Answer::Joined(Ok(json!("summary"))));
        drop(read);
        assert_eq!(reply.answered().unwrap().unwrap(), json!("summary"));
        // Listed until the worker lets the job go.
        assert_eq!(flights.lock().unwrap().len(), 1);
        drop(reply);
        assert!(flights.lock().unwrap().is_empty());
    }

    /// A read's recollection that ends without new evidence answers its
    /// joiners nothing of its own: each runs its own revalidation.
    #[test]
    fn a_reads_recollection_without_new_evidence_has_its_joiners_rerun() {
        let flights: Flights = Arc::default();
        let read = Flight::lead_or_join(&flights, key(), Answer::Rerun).unwrap();
        let Member::Joined(joined) = Flight::lead_or_join(&flights, key(), Answer::Rerun).unwrap()
        else {
            panic!("the read's recollection was in flight");
        };
        drop(read);
        assert!(flights.lock().unwrap().is_empty());
        assert!(matches!(
            joined.wait(connectors_sdk::now_ms() + 1_000).unwrap(),
            Answer::Rerun
        ));
    }
}

#[cfg(test)]
mod read_answer_tests {
    use super::*;

    /// story:owner-read-answers-json-value: the answer a read through the owner
    /// carries holds the provider result as the JSON value the adapter
    /// answered, never as a string holding its JSON text.
    #[test]
    fn a_read_answer_carries_the_provider_result_as_a_json_value() {
        let answer = read_answer(
            "forge",
            "item.read",
            "desc-1",
            br#"{"id":7,"body":{"name":"fixture"}}"#.to_vec(),
        )
        .unwrap();
        assert_eq!(
            answer,
            json!({"adapter":"forge","operation":"item.read","revision":"desc-1",
                   "result":{"id":7,"body":{"name":"fixture"}}})
        );
        assert!(answer["result"].is_object(), "{answer}");
        // An array and a scalar result stay what the adapter answered.
        let listed = read_answer("forge", "items.list", "desc-1", b"[1,\"two\"]".to_vec()).unwrap();
        assert_eq!(listed["result"], json!([1, "two"]));
        // A document that is not one unambiguous JSON value is the adapter's
        // fault, never a string handed on.
        for document in [&b"not json"[..], b"{\"a\":1,\"a\":2}", b"\xff"] {
            let refused = read_answer("forge", "item.read", "desc-1", document.to_vec());
            assert_eq!(
                refused.map_err(|error| error.code).unwrap_err(),
                Code::Unavailable
            );
        }
    }
}

/// story:invoke-timeout-after-dispatch-stage: a read whose deadline passes
/// after its worker dispatched it is the provider's timeout (`stage =
/// dispatch`); one whose deadline passes before dispatch stays the host's
/// own (`stage = admission`), and its worker can no longer dispatch it.
#[cfg(test)]
mod dispatch_wait_tests {
    use super::*;
    use crate::local::config::{Executable, Startup};

    fn adapter() -> Adapter {
        Adapter {
            credential_revision: None,
            instance_id: "instance".into(),
            adapter_id: "adapter".into(),
            configuration_revision: "config".into(),
            protocol: "v1alpha1".into(),
            private_protocol: None,
            startup: Startup::OnDemand,
            restart: Default::default(),
            permissions: Default::default(),
            executable: Executable {
                path: "/not-launched".into(),
                sha256: "a".repeat(64),
                args: Vec::new(),
            },
        }
    }

    fn read() -> Task {
        Task::Invoke {
            connection: "connection".into(),
            operation: "read".into(),
            schema: "schema".into(),
            revision: "revision".into(),
            document: b"{}".to_vec(),
        }
    }

    /// A pool whose one stand-in worker takes every job, dispatches it when
    /// `dispatch` is set, reports what its dispatch attempt answered, and
    /// keeps the job unanswered until shutdown.
    fn pool(dispatch: bool) -> (Pool, mpsc::Receiver<Dispatch>, tempfile::TempDir) {
        let root = tempfile::tempdir().unwrap();
        let pool = Pool::new(
            Arc::new(Paths {
                config: root.path().join("unread-config"),
                state: root.path().join("unopened-state"),
            }),
            uuid::Uuid::new_v4().to_string(),
        );
        let (sender, receiver) = mpsc::sync_channel::<Work>(16);
        let (taken, jobs) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut held = Vec::new();
            while let Ok(work) = receiver.recv() {
                if let Work::Invoke(job) = &work {
                    if dispatch {
                        job.dispatch.begin().unwrap();
                    }
                    let _ = taken.send(job.dispatch.clone());
                }
                held.push(work);
            }
        });
        pool.workers.lock().unwrap().insert(
            "instance".into(),
            Worker {
                sender,
                thread,
                control: Arc::new(Mutex::new(lifecycle::Control::default())),
                busy: Arc::new(AtomicBool::new(false)),
                recovering: Arc::new(AtomicBool::new(false)),
                recovery_pending: Arc::new(AtomicBool::new(false)),
                queued: Arc::new(AtomicUsize::new(0)),
            },
        );
        (pool, jobs, root)
    }

    fn timed_out(pool: &Pool) -> Error {
        match pool.run("alias", &adapter(), read(), connectors_sdk::now_ms() + 300) {
            Err(error) => error,
            Ok(_) => panic!("the stand-in worker never answers"),
        }
    }

    #[test]
    fn a_read_whose_deadline_passes_after_dispatch_is_the_providers_timeout() {
        let (pool, _jobs, _root) = pool(true);
        let error = timed_out(&pool);
        assert_eq!(error.code, Code::Timeout);
        assert_eq!(
            error.origin,
            Origin::Provider,
            "a dispatched read's timeout reported as the host's admission"
        );
        pool.shutdown().unwrap();
    }

    #[test]
    fn a_read_whose_deadline_passes_before_dispatch_stays_admission_and_never_dispatches() {
        let (pool, jobs, _root) = pool(false);
        let error = timed_out(&pool);
        assert_eq!(error.code, Code::Timeout);
        assert_eq!(error.origin, Origin::Host);
        // The waiter answered admission, so the worker may not dispatch it now.
        let dispatch = jobs.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            dispatch.begin().map_err(|error| error.code),
            Err(Code::Timeout),
            "a read the waiter reported as not dispatched was dispatched after all"
        );
        pool.shutdown().unwrap();
    }
}

/// story:revalidation-retries-a-transient-failure: the owner's bounded retry of
/// a revalidation's validation (contracts/cli/v1alpha1/semantics.md §6
/// "Revalidation retries").
#[cfg(test)]
mod revalidation_retry_tests {
    use super::*;
    use runtime::Failure as F;

    /// Runs the retry over scripted attempts, each answering its failure (or
    /// success) and whether the adapter still runs; answers the outcome and
    /// the pauses taken.
    fn run(
        deadline_ms: u64,
        script: &[(Option<F>, bool)],
        stopping: bool,
    ) -> ((runtime::Result<()>, u32, Option<Cause>), Vec<Duration>) {
        let mut next = script.iter();
        let mut pauses = Vec::new();
        let outcome = revalidate_with_retries(
            deadline_ms,
            || {
                let (failure, alive) = next.next().expect("an attempt past the script");
                (failure.map_or(Ok(()), Err), *alive)
            },
            || stopping,
            |wait| pauses.push(wait),
        );
        (outcome, pauses)
    }

    fn later() -> u64 {
        connectors_sdk::now_ms() + 30_000
    }

    #[test]
    fn a_stale_pooled_connection_fails_one_attempt_and_the_next_succeeds() {
        let (outcome, pauses) = run(
            later(),
            &[(Some(F::Unavailable), true), (None, true)],
            false,
        );
        assert_eq!(outcome, (Ok(()), 2, None));
        assert_eq!(pauses, [Duration::from_millis(200)]);
    }

    #[test]
    fn a_persistent_transient_failure_is_sent_three_times_and_names_its_class() {
        for (failure, cause) in [
            (F::Unavailable, Cause::ProviderUnavailable),
            (F::ProviderTimeout, Cause::ProviderTimeout),
            (F::ProviderRateLimited, Cause::ProviderRateLimited),
            (F::ProviderInternal, Cause::ProviderInternal),
            (F::ProviderCapacity, Cause::ProviderCapacity),
        ] {
            let (outcome, pauses) = run(later(), &[(Some(failure), true); 3], false);
            assert_eq!(outcome, (Err(failure), 3, Some(cause)), "{failure:?}");
            assert_eq!(
                pauses,
                [Duration::from_millis(200), Duration::from_secs(1)],
                "{failure:?}"
            );
        }
    }

    #[test]
    fn a_credential_permission_or_scope_refusal_is_never_sent_again() {
        for failure in [
            F::InvalidCredential,
            F::Forbidden,
            F::ProviderForbidden,
            F::InsufficientScope,
            F::IdentityMismatch,
            F::Protocol,
            F::InvalidConfiguration,
            F::Unsupported,
        ] {
            let (outcome, pauses) = run(later(), &[(Some(failure), true)], false);
            assert_eq!(outcome, (Err(failure), 1, None), "{failure:?}");
            assert!(pauses.is_empty(), "{failure:?}");
        }
    }

    #[test]
    fn no_attempt_is_sent_that_could_not_end_before_the_deadline() {
        // 200 ms pause + 500 ms margin fit in 1 s; the 1 s pause does not.
        let deadline = connectors_sdk::now_ms() + 1_000;
        let (outcome, pauses) = run(deadline, &[(Some(F::Unavailable), true); 2], false);
        assert_eq!(
            outcome,
            (Err(F::Unavailable), 2, Some(Cause::ProviderUnavailable))
        );
        assert_eq!(pauses, [Duration::from_millis(200)]);
        // Not even the 200 ms pause and the margin fit.
        let deadline = connectors_sdk::now_ms() + 600;
        let (outcome, pauses) = run(deadline, &[(Some(F::Unavailable), true)], false);
        assert_eq!(
            outcome,
            (Err(F::Unavailable), 1, Some(Cause::ProviderUnavailable))
        );
        assert!(pauses.is_empty());
    }

    #[test]
    fn an_exited_adapter_or_a_stopping_owner_ends_the_retries_by_name() {
        let (outcome, _) = run(later(), &[(Some(F::Unavailable), false)], false);
        assert_eq!(
            outcome,
            (Err(F::Unavailable), 1, Some(Cause::AdapterExited))
        );
        // The owner ends a child whose request timed out; the class stays the timeout.
        let (outcome, _) = run(later(), &[(Some(F::Timeout), false)], false);
        assert_eq!(outcome, (Err(F::Timeout), 1, Some(Cause::ProviderTimeout)));
        let (outcome, pauses) = run(later(), &[(Some(F::Unavailable), true)], true);
        assert_eq!(
            outcome,
            (Err(F::Unavailable), 1, Some(Cause::OwnerStopping))
        );
        assert!(pauses.is_empty());
    }

    #[test]
    fn only_a_transient_failure_names_its_cause_and_attempts() {
        let error = retried(F::Unavailable, 3, Some(Cause::ProviderUnavailable));
        assert_eq!(
            (error.code, error.cause, error.attempts),
            (Code::Unavailable, Some(Cause::ProviderUnavailable), Some(3))
        );
        let error = retried(F::InvalidCredential, 1, None);
        assert_eq!((error.cause, error.attempts), (None, None));
    }
}
