use super::*;
use connectors_sdk::Secret;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
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
    reply: mpsc::Sender<Result<Output>>,
    epoch: u64,
}
struct Worker {
    sender: mpsc::SyncSender<Work>,
    control: Arc<Mutex<lifecycle::Control>>,
    thread: std::thread::JoinHandle<()>,
    busy: Arc<AtomicBool>,
    recovery_pending: Arc<AtomicBool>,
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
}
impl Pool {
    pub fn new(paths: Arc<Paths>, incarnation: String) -> Self {
        Self {
            workers: Mutex::new(BTreeMap::new()),
            paths,
            launches: Arc::new(Launches::default()),
            incarnation,
            stopped: Arc::new(AtomicBool::new(false)),
            shutdown_lock: Mutex::new(()),
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
            let control = Arc::new(Mutex::new(lifecycle::Control::default()));
            let shared = control.clone();
            let stopped = self.stopped.clone();
            let busy = Arc::new(AtomicBool::new(false));
            let active = busy.clone();
            let recovery_pending = Arc::new(AtomicBool::new(false));
            let instance = adapter.instance_id.clone();
            let thread = std::thread::Builder::new()
                .name("connectors-adapter-owner".into())
                .spawn(move || worker(paths, launches, shared, stopped, active, instance, receiver))
                .map_err(|_| Code::Unavailable)?;
            workers.insert(
                adapter.instance_id.clone(),
                Worker {
                    sender,
                    control,
                    thread,
                    busy,
                    recovery_pending,
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
            })))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => Code::Capacity,
                mpsc::TrySendError::Disconnected(_) => Code::Unavailable,
            })?;
        Ok(receiver)
    }
    pub fn run(&self, alias: &str, adapter: &Adapter, task: Task, deadline: u64) -> Result<Output> {
        self.send(alias, adapter, task, deadline)?
            .recv_timeout(until(deadline)?.saturating_duration_since(Instant::now()))
            .map_err(|e| match e {
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
            maintenance::apply(
                &self.paths,
                batch,
                Quiescent {
                    instance: &instance,
                    _child: &mut absent,
                },
                &self.stopped,
            )?;
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
    paths: Arc<Paths>,
    launches: Arc<Launches>,
    control: Arc<Mutex<lifecycle::Control>>,
    stopped: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    instance: String,
    receiver: mpsc::Receiver<Work>,
) {
    let state = runtime::state::State::new(&paths.state);
    let mut child: Option<runtime::Child> = None;
    let mut selection = String::new();
    for work in receiver {
        busy.store(true, Ordering::SeqCst);
        let job = match work {
            Work::Invoke(job) => *job,
            Work::Recover(batch, _queued) => {
                if !stopped.load(Ordering::SeqCst) {
                    let _ = maintenance::apply(
                        &paths,
                        batch,
                        Quiescent {
                            instance: &instance,
                            _child: &mut child,
                        },
                        &stopped,
                    );
                }
                busy.store(false, Ordering::SeqCst);
                continue;
            }
        };
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
            let resume = matches!(
                job.task,
                Task::Ensure { resume: true }
                    | Task::Invoke { .. }
                    | Task::Write { .. }
                    | Task::Revalidate { .. }
            );
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
                    let captured = registry.capture_read(
                        &binding,
                        &connection,
                        &requirement.scopes,
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
                    let dispatched = registry.dispatch_read(captured, connectors_sdk::now_ms())?;
                    drop(guard);
                    let result = active.invoke(
                        &operation, &revision, &partition, &material, &document, deadline,
                    );
                    registry.release_read(dispatched, connectors_sdk::now_ms())?;
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
                recovery_pending: queued.clone(),
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
        drop(token);
        assert!(!queued.load(Ordering::SeqCst));
        pool.recover(batch()).unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Work::Recover(_, _))));
        assert!(!root.path().join("unopened-state").exists());
        drop(release);
        pool.shutdown().unwrap();
    }
}
