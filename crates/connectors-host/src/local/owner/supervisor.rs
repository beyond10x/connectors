use super::*;
use connectors_sdk::Secret;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
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
        governed: bool,
        projection_revision: Option<String>,
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
pub(super) enum Output {
    Bootstrap(runtime::Bootstrap, u64),
    Baseline(runtime::Baseline),
    Value(Value),
    Document(Vec<u8>),
    Write(mutation::Delivery),
}
struct Job {
    alias: String,
    adapter: Adapter,
    task: Task,
    deadline: u64,
    reply: mpsc::Sender<Result<Output>>,
    epoch: u64,
    queued: Option<JobQueued>,
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
    read_policy: Arc<dyn ReadPolicy>,
}
impl Pool {
    #[cfg(test)]
    pub fn new(paths: Arc<Paths>, incarnation: String) -> Self {
        Self::with_read_policy(paths, incarnation, Arc::new(governed::UnboundReadPolicy))
    }
    pub fn with_read_policy(
        paths: Arc<Paths>,
        incarnation: String,
        read_policy: Arc<dyn ReadPolicy>,
    ) -> Self {
        Self {
            workers: Mutex::new(BTreeMap::new()),
            paths,
            launches: Arc::new(Launches::default()),
            incarnation,
            stopped: Arc::new(AtomicBool::new(false)),
            shutdown_lock: Mutex::new(()),
            settled: Arc::new(AtomicU64::new(0)),
            read_policy,
        }
    }
    fn send(
        &self,
        alias: &str,
        adapter: &Adapter,
        task: Task,
        deadline: u64,
    ) -> Result<mpsc::Receiver<Result<Output>>> {
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
            let read_policy = self.read_policy.clone();
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
            let thread = std::thread::Builder::new()
                .name("connectors-adapter-owner".into())
                .spawn(move || {
                    worker(
                        (paths, read_policy),
                        launches,
                        shared,
                        stopped,
                        activity,
                        instance,
                        receiver,
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
        worker
            .sender
            .try_send(Work::Invoke(Box::new(Job {
                alias: alias.into(),
                adapter: adapter.clone(),
                task,
                deadline,
                reply,
                epoch: control.epoch,
                queued: Some(JobQueued::new(&worker.queued)),
            })))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => Code::Capacity,
                mpsc::TrySendError::Disconnected(_) => Code::Unavailable,
            })?;
        Ok(receiver)
    }
    pub fn run(&self, alias: &str, adapter: &Adapter, task: Task, deadline: u64) -> Result<Output> {
        // The worker may still commit a revalidation after this wait ends.
        let committing = matches!(task, Task::Revalidate { .. });
        self.send(alias, adapter, task, deadline)?
            .recv_timeout(until(deadline)?.saturating_duration_since(Instant::now()))
            .map_err(|e| match e {
                _ if committing => Code::OutcomeUnknown,
                mpsc::RecvTimeoutError::Timeout => Code::Timeout,
                _ => Code::Unavailable,
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
                    && let Err(error) = lifecycle::signal(&child.handle, libc::SIGKILL)
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
fn worker(
    binding: (Arc<Paths>, Arc<dyn ReadPolicy>),
    launches: Arc<Launches>,
    control: Arc<Mutex<lifecycle::Control>>,
    stopped: Arc<AtomicBool>,
    activity: Activity,
    instance: String,
    receiver: mpsc::Receiver<Work>,
) {
    let (paths, read_policy) = binding;
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
            if let Task::Invoke {
                connection,
                operation,
                schema,
                revision,
                document,
                governed: true,
                projection_revision,
            } = &job.task
            {
                let input = std::str::from_utf8(document).map_err(|_| Code::InvalidInput)?;
                governed::resolve_selected(
                    &paths,
                    &job.alias,
                    &approval_issuance::Request {
                        connection,
                        operation,
                        schema,
                        revision,
                        input,
                    },
                    projection_revision
                        .as_deref()
                        .map(|r| (read_policy.as_ref(), r)),
                )?;
            }
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
                return Err(Code::LifecycleConflict.into());
            }
            if resume {
                state.suppress(&current.instance_id, false)?;
            }
            drop(guard);
            if child.is_none() {
                {
                    let mut guard = control.lock().map_err(|_| Code::Unavailable)?;
                    guard.check(job.epoch)?;
                    guard.starting = true;
                }
                let _permit = launches.acquire(job.deadline)?;
                let new = runtime::Child::spawn_until(&current, until(job.deadline)?)?;
                let mut guard = control.lock().map_err(|_| Code::Unavailable)?;
                guard.check(job.epoch)?;
                if stopped.load(Ordering::SeqCst) {
                    return Err(Code::Unavailable.into());
                }
                state.remember(&current.selection(), new.bootstrap())?;
                selection = current.selection();
                guard.child = Some(lifecycle::Owned {
                    handle: new.termination_handle()?,
                    incarnation: new.incarnation().into(),
                    revision: new.bootstrap().configuration_revision.clone(),
                });
                guard.failed = false;
                guard.starting = false;
                child = Some(new);
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
                    let binding = active.bootstrap().binding(&profile)?;
                    let captured = registry.capture_revalidation(
                        &binding,
                        &connection,
                        &revision,
                        connectors_sdk::now_ms(),
                        job.deadline,
                    )?;
                    let version = captured.version();
                    let material = custody::Store::open_at(
                        version.scope(),
                        config.secret_service_socket.as_deref(),
                    )
                    .and_then(|store| store.read(version));
                    let material = match material {
                        Ok(value) => value,
                        Err(error) => {
                            if matches!(error, custody::Failure::Missing) {
                                registry
                                    .missing_revalidation(captured, connectors_sdk::now_ms())?;
                            } else {
                                registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
                            }
                            return Err(Code::CustodyUnavailable.into());
                        }
                    };
                    let check = || -> Result<()> {
                        let (latest_config, latest) = selected(&paths, &job.alias)?;
                        if latest.selection() != current.selection()
                            || latest_config.secret_service_socket != config.secret_service_socket
                        {
                            return Err(Code::LifecycleConflict.into());
                        }
                        if !latest.permissions.profiles.contains(&profile) {
                            return Err(Code::Forbidden.into());
                        }
                        if stopped.load(Ordering::SeqCst) {
                            return Err(Code::Unavailable.into());
                        }
                        Ok(())
                    };
                    let guard = control.lock().map_err(|_| Code::Unavailable)?;
                    if let Err(error) = guard.check(job.epoch).and_then(|_| check()) {
                        registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
                        return Err(error);
                    }
                    let dispatched =
                        registry.dispatch_revalidation(captured, connectors_sdk::now_ms())?;
                    drop(guard);
                    let result = active.validate(&profile, &material, job.deadline);
                    let guard = control.lock().map_err(|_| Code::Unavailable)?;
                    if let Err(error) = guard.check(job.epoch).and_then(|_| check()) {
                        registry.finish_revalidation(
                            dispatched,
                            Err(None),
                            connectors_sdk::now_ms(),
                        )?;
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
                                runtime::Failure::InvalidCredential
                                | runtime::Failure::IdentityMismatch => {
                                    Some(registry::InvalidCredential::Invalid)
                                }
                                runtime::Failure::InsufficientScope => {
                                    Some(registry::InvalidCredential::Insufficient)
                                }
                                _ => None,
                            };
                            registry.finish_revalidation(
                                dispatched,
                                Err(reason),
                                connectors_sdk::now_ms(),
                            )?;
                            return Err(error.into());
                        }
                    }
                    drop(guard);
                    let observed = registry.describe(
                        &current.instance_id,
                        &current.adapter_id,
                        &current.configuration_revision,
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
                    governed,
                    projection_revision,
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
                    let value: Value = if governed {
                        connectors_core::json::decode(&document, 64)
                            .map_err(|_| Code::InvalidInput)?
                    } else {
                        connectors_core::read_json(&document).map_err(|_| Code::InvalidInput)?
                    };
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
                    let captured = registry
                        .capture_read(
                            &binding,
                            &connection,
                            &requirement.scopes,
                            connectors_sdk::now_ms(),
                            job.deadline,
                        )
                        .map_err(|error| {
                            if governed {
                                governed::registry_error(error)
                            } else {
                                error.into()
                            }
                        })?;
                    let version = captured.version();
                    let material = custody::Store::open_at(
                        version.scope(),
                        config.secret_service_socket.as_deref(),
                    )
                    .and_then(|store| store.read(version));
                    let material = match material {
                        Ok(value) => value,
                        Err(e) => {
                            if matches!(e, custody::Failure::Missing) {
                                registry.invalidate_read(
                                    &captured,
                                    registry::InvalidCredential::Missing,
                                    connectors_sdk::now_ms(),
                                )?;
                            }
                            registry.cancel_read(captured, connectors_sdk::now_ms())?;
                            return Err(Code::CustodyUnavailable.into());
                        }
                    };
                    let partition = captured.partition().to_owned();
                    let deadline = captured.expires_at_ms();
                    let checked: Result<()> = (|| {
                        let (latest_config, latest) = selected(&paths, &job.alias)?;
                        if latest.selection() != current.selection()
                            || latest_config.secret_service_socket != config.secret_service_socket
                        {
                            return Err(Code::LifecycleConflict.into());
                        }
                        if !latest.permissions.operations.contains(&operation) {
                            return Err(if governed {
                                Code::NotGranted
                            } else {
                                Code::Forbidden
                            }
                            .into());
                        }
                        if !latest.permissions.profiles.contains(&requirement.profile) {
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
                            if governed {
                                let input = std::str::from_utf8(&document)
                                    .map_err(|_| Code::InvalidInput)?;
                                governed::resolve_selected(
                                    &paths,
                                    &job.alias,
                                    &approval_issuance::Request {
                                        connection: &connection,
                                        operation: &operation,
                                        schema: &expected_schema,
                                        revision: &revision,
                                        input,
                                    },
                                    projection_revision
                                        .as_deref()
                                        .map(|r| (read_policy.as_ref(), r)),
                                )?;
                            }
                            Ok(())
                        }
                    }) {
                        registry.cancel_read(captured, connectors_sdk::now_ms())?;
                        return Err(error);
                    }
                    let dispatched = registry
                        .dispatch_read(captured, connectors_sdk::now_ms())
                        .map_err(|error| {
                            if governed {
                                governed::registry_error(error)
                            } else {
                                error.into()
                            }
                        })?;
                    drop(guard);
                    let result = if governed {
                        active.invoke_lossless(
                            &operation, &revision, &partition, &material, &document, deadline,
                        )
                    } else {
                        active.invoke(
                            &operation, &revision, &partition, &material, &document, deadline,
                        )
                    };
                    registry.release_read(dispatched, connectors_sdk::now_ms())?;
                    if governed {
                        return result.map(Output::Document).map_err(Error::from);
                    }
                    let body = String::from_utf8(result?).map_err(|_| Code::Unavailable)?;
                    Ok(Output::Value(
                        json!({"adapter":job.alias,"operation":operation,"revision":revision,"result":body}),
                    ))
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
        let _ = job.reply.send(result);
    }
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
                governed: false,
                projection_revision: None,
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
    use sha2::{Digest, Sha256};

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
        let path = std::env::current_exe().unwrap();
        let sha256 = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
        let adapter = Adapter {
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
}
