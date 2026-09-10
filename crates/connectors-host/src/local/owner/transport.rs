use super::super::{filesystem as fs, metadata::Metadata};
use super::*;
use connectors_sdk::Secret;
use runtime::channel;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            fs::{FileTypeExt, MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
            process::CommandExt,
        },
    },
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub struct Client {
    stream: UnixStream,
    pub host_incarnation: String,
}
pub struct Capture {
    client: Client,
    pub acquisition: String,
    pub expires_at_ms: u64,
    pub profile: runtime::Profile,
}
fn read_reply(
    stream: &mut UnixStream,
    deadline: Instant,
    limit: usize,
) -> Result<channel::Frame<Reply>> {
    channel::read_with_cancel(
        stream,
        deadline,
        false,
        limit,
        Some(&|| {
            crate::local::protected::cancellation().map_err(|_| runtime::Failure::Interrupted)
        }),
    )
    .map_err(Error::from)
}
impl Capture {
    pub fn complete(mut self, secret: &Secret) -> Result<Value> {
        let result = (|| {
            crate::local::protected::cancellation()?;
            channel::write(
                &mut self.client.stream,
                &Request::Complete,
                Some(secret),
                &[],
                until(self.expires_at_ms)?,
            )?;
            self.client.value(until(self.expires_at_ms)?)
        })();
        result.map_err(|mut error: Error| {
            if matches!(error.code, Code::Unavailable | Code::Timeout) {
                error.code = Code::OutcomeUnknown;
            }
            error.acquisition = Some(self.acquisition.clone());
            error
        })
    }
}
impl Client {
    pub fn connect(paths: &Paths, start: bool) -> Result<Self> {
        let authority = Metadata::inspect(&paths.state)?.authority()?.to_string();
        let directory = fs::directory(&paths.state, false, true)?;
        let socket = socket_path(&directory);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            crate::local::protected::cancellation()?;
            match connect_socket(&socket) {
                Ok(stream) => return Self::greet(stream, paths, &authority, deadline),
                Err(error) if !start => return Err(error),
                Err(error) if error.code != Code::Unavailable => return Err(error),
                Err(_) => {}
            }
            if !start {
                return Err(Code::Unavailable.into());
            }
            let lock = lock(&directory)?;
            // SAFETY: the held owner-only file is the one admitted lifetime lock.
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                if let Ok(stream) = connect_socket(&socket) {
                    return Self::greet(stream, paths, &authority, deadline);
                }
                let (stream, mut process) = spawn(paths, &lock, deadline)?;
                let result = Self::greet(stream, paths, &authority, deadline);
                if result.is_err() {
                    let _ = process.kill();
                    let _ = process.wait();
                }
                return result;
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::WouldBlock {
                return Err(Code::Unavailable.into());
            }
            if Instant::now() >= deadline {
                return Err(Code::Timeout.into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn greet(
        mut stream: UnixStream,
        paths: &Paths,
        authority: &str,
        deadline: Instant,
    ) -> Result<Self> {
        channel::peer(&stream)?;
        let challenge = uuid::Uuid::new_v4().to_string();
        channel::write(
            &mut stream,
            &Request::Hello {
                version: VERSION.into(),
                challenge: challenge.clone(),
                configuration: paths.config.clone(),
                authority: authority.into(),
            },
            None,
            &[],
            deadline,
        )?;
        let reply = read_reply(&mut stream, deadline, 0)?;
        match reply.control {
            Reply::Hello {
                version,
                challenge: returned,
                host_incarnation,
                authority: returned_authority,
            } if version == VERSION
                && returned == challenge
                && returned_authority == authority
                && uuid::Uuid::parse_str(&host_incarnation).is_ok() =>
            {
                Ok(Self {
                    stream,
                    host_incarnation,
                })
            }
            Reply::Failed { error } => Err(error),
            _ => Err(Code::ReadinessMismatch.into()),
        }
    }
    pub fn begin(
        mut self,
        adapter: &str,
        profile: Option<String>,
        connection: Option<String>,
        expected_revision: Option<String>,
    ) -> Result<Capture> {
        let deadline = Instant::now() + Duration::from_secs(40);
        channel::write(
            &mut self.stream,
            &Request::Begin {
                adapter: adapter.into(),
                profile,
                connection,
                expected_revision,
            },
            None,
            &[],
            deadline,
        )?;
        let frame = read_reply(&mut self.stream, deadline, 0)?;
        match frame.control {
            Reply::Capture {
                acquisition,
                expires_at_ms,
                profile,
            } if connectors_core::valid_id(&acquisition)
                && expires_at_ms > connectors_sdk::now_ms()
                && expires_at_ms <= connectors_sdk::now_ms() + 300_000 =>
            {
                Ok(Capture {
                    client: self,
                    acquisition,
                    expires_at_ms,
                    profile,
                })
            }
            Reply::Failed { error } => Err(error),
            _ => Err(Code::Unavailable.into()),
        }
    }
    pub fn invoke(
        mut self,
        adapter: &str,
        call: &Value,
        document: &[u8],
        deadline_ms: u64,
    ) -> Result<Value> {
        crate::local::protected::cancellation()?;
        let request = Request::Invoke {
            adapter: adapter.into(),
            connection: input(call, "connection")?,
            operation: input(call, "operation")?,
            schema: input(call, "schema")?,
            revision: input(call, "revision")?,
            deadline_ms,
        };
        channel::write(
            &mut self.stream,
            &request,
            None,
            document,
            until(deadline_ms)?,
        )?;
        self.value(until(deadline_ms)?)
    }
    pub fn revalidate(
        mut self,
        adapter: &str,
        connection: &str,
        revision: &str,
        deadline_ms: u64,
    ) -> Result<Value> {
        crate::local::protected::cancellation()?;
        channel::write(
            &mut self.stream,
            &Request::Revalidate {
                adapter: adapter.into(),
                connection: connection.into(),
                expected_revision: revision.into(),
                deadline_ms,
            },
            None,
            &[],
            until(deadline_ms)?,
        )?;
        self.value(until(deadline_ms)?).map_err(|mut error| {
            if matches!(error.code, Code::Unavailable | Code::Timeout) {
                error.code = Code::OutcomeUnknown;
            }
            error
        })
    }
    pub fn status(mut self, adapter: &str) -> Result<Value> {
        self.simple(
            Request::Status {
                adapter: adapter.into(),
            },
            Duration::from_secs(2),
        )
    }
    pub fn stop(
        mut self,
        adapter: &str,
        configuration_revision: &str,
        host: &str,
        child: &str,
    ) -> Result<Value> {
        self.simple(
            Request::Stop {
                adapter: adapter.into(),
                configuration_revision: configuration_revision.into(),
                host_incarnation: host.into(),
                child_incarnation: child.into(),
            },
            Duration::from_secs(6),
        )
    }
    /// Private owner lifecycle control, also used by disposable acceptance owners.
    /// The caller must retain the exact incarnation observed on this connection.
    pub fn shutdown(mut self, expected: &str) -> Result<()> {
        self.simple(
            Request::Shutdown {
                host_incarnation: expected.into(),
            },
            Duration::from_secs(5),
        )
        .map(|_| ())
    }
    fn simple(&mut self, request: Request, span: Duration) -> Result<Value> {
        let deadline = Instant::now() + span;
        channel::write(&mut self.stream, &request, None, &[], deadline)?;
        self.value(deadline)
    }
    fn value(&mut self, deadline: Instant) -> Result<Value> {
        let frame = read_reply(&mut self.stream, deadline, runtime::RESULT_LIMIT)?;
        match frame.control {
            Reply::Success => {
                channel::depth(&frame.document)?;
                connectors_core::read_json(&frame.document).map_err(|_| Code::Unavailable.into())
            }
            Reply::Failed { error } if frame.document.is_empty() => Err(error),
            _ => Err(Code::Unavailable.into()),
        }
    }
}
fn socket_path(directory: &File) -> PathBuf {
    PathBuf::from(format!(
        "/proc/self/fd/{}/owner.sock",
        directory.as_raw_fd()
    ))
}
fn connect_socket(path: &std::path::Path) -> Result<UnixStream> {
    let info = std::fs::symlink_metadata(path).map_err(|_| Code::Unavailable)?;
    if !info.file_type().is_socket() || info.uid() != fs::uid() || info.mode() & 0o077 != 0 {
        return Err(Code::InvalidConfiguration.into());
    }
    let stream = UnixStream::connect(path).map_err(|_| Code::Unavailable)?;
    channel::peer(&stream)?;
    Ok(stream)
}
fn lock(directory: &File) -> Result<File> {
    match fs::publish_new(directory, std::ffi::OsStr::new("owner.lock"), &[]) {
        Ok(()) | Err(crate::local::Failure::ConfigurationExists) => {}
        Err(e) => return Err(e.into()),
    }
    Ok(fs::private_file_at(
        directory,
        std::ffi::OsStr::new("owner.lock"),
    )?)
}
fn spawn(
    paths: &Paths,
    lock: &File,
    deadline: Instant,
) -> Result<(UnixStream, std::process::Child)> {
    let binary = std::env::current_exe().map_err(|_| Code::Unavailable)?;
    let hash = hex::encode(Sha256::digest(
        std::fs::read(&binary).map_err(|_| Code::Unavailable)?,
    ));
    let selection = crate::local::config::Executable {
        path: binary,
        sha256: hash.clone(),
        args: Vec::new(),
    };
    let executable = runtime::artifact::capture(selection.open()?, &hash, deadline)?;
    let (parent, child) = UnixStream::pair().map_err(|_| Code::Unavailable)?;
    // Put source fds beyond the destination slots before fork.
    let duplicate = |fd| -> Result<File> {
        // SAFETY: fcntl duplicates a live caller-owned descriptor.
        let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
        if raw < 0 {
            return Err(Code::Unavailable.into());
        }
        // SAFETY: successful duplication transfers ownership.
        Ok(unsafe { File::from_raw_fd(raw) })
    };
    let private = duplicate(child.as_raw_fd())?;
    let lifetime = duplicate(lock.as_raw_fd())?;
    let (private_fd, lock_fd) = (private.as_raw_fd(), lifetime.as_raw_fd());
    let mut command = Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
    command
        .args([
            std::ffi::OsStr::new("__connectors-owner"),
            paths.config.as_os_str(),
            paths.state.as_os_str(),
        ])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the hook uses only async-signal-safe syscalls and scalar fds. This
    // process intentionally outlives the CLI and starts a separate session.
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() < 0
                || libc::dup2(private_fd, 3) < 0
                || libc::dup2(lock_fd, 4) < 0
                || libc::fcntl(3, libc::F_SETFD, 0) < 0
                || libc::fcntl(4, libc::F_SETFD, 0) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let process = command.spawn().map_err(|_| Code::Unavailable)?;
    Ok((parent, process))
}

struct Owner {
    paths: Arc<Paths>,
    authority: String,
    incarnation: String,
    pool: supervisor::Pool,
    shutdown: AtomicBool,
    clients: AtomicUsize,
}
pub fn serve(paths: Paths) -> Result<()> {
    // Duplicate before opening anything: a direct invocation with missing fds
    // must not mistake newly opened configuration files for inherited authority.
    let (startup, lifetime) = inherited()?;
    let config = Config::load(&paths.config)?;
    let directory = fs::directory(&paths.state, false, true)?;
    let metadata = Metadata::update(&paths.state, true)?;
    let authority = metadata.authority()?.to_string();
    drop(metadata);
    fs::check_private_file(&lifetime)?;
    let expected = fs::private_file_at(&directory, std::ffi::OsStr::new("owner.lock"))?;
    let (a, b) = (
        lifetime.metadata().map_err(|_| Code::Unavailable)?,
        expected.metadata().map_err(|_| Code::Unavailable)?,
    );
    if a.dev() != b.dev() || a.ino() != b.ino() {
        return Err(Code::InvalidConfiguration.into());
    }
    // SAFETY: the inherited file is verified against the exact owner lock inode.
    if unsafe { libc::flock(lifetime.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(Code::Unavailable.into());
    }
    let socket = socket_path(&directory);
    if let Ok(metadata) = std::fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket()
            || metadata.uid() != fs::uid()
            || UnixStream::connect(&socket).is_ok()
        {
            return Err(Code::InvalidConfiguration.into());
        }
        std::fs::remove_file(&socket).map_err(|_| Code::Unavailable)?;
    }
    let listener = UnixListener::bind(&socket).map_err(|_| Code::Unavailable)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| Code::Unavailable)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| Code::Unavailable)?;
    let paths = Arc::new(paths);
    let incarnation = uuid::Uuid::new_v4().to_string();
    let owner = Arc::new(Owner {
        paths: paths.clone(),
        authority,
        incarnation: incarnation.clone(),
        pool: supervisor::Pool::new(paths, incarnation),
        shutdown: AtomicBool::new(false),
        clients: AtomicUsize::new(0),
    });
    handle(owner.clone(), startup)?;
    owner.pool.automatic(&config);
    let mut result = Ok(());
    while !owner.shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle(owner.clone(), stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(_) => {
                result = Err(Code::Unavailable.into());
                break;
            }
        }
    }
    drop(listener);
    if let Err(error) = owner.pool.shutdown() {
        result = Err(error);
    }
    if std::fs::remove_file(socket).is_err() {
        result = Err(Code::Unavailable.into());
    }
    // This entrypoint returns directly to process::exit. Keep the owner lock
    // until kernel process exit, including any in-flight request publication.
    // CLOEXEC prevents adapter children from inheriting that lifetime authority.
    std::mem::forget(lifetime);
    result
}
fn inherited() -> Result<(UnixStream, File)> {
    let duplicate = |fd| -> Result<File> {
        // SAFETY: fcntl rejects a missing descriptor; it never transfers the
        // caller's original descriptor into Rust ownership.
        let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
        if raw < 0 {
            return Err(Code::InvalidConfiguration.into());
        }
        // SAFETY: successful duplication returns a new owned descriptor.
        Ok(unsafe { File::from_raw_fd(raw) })
    };
    let channel = duplicate(3)?;
    let lifetime = duplicate(4)?;
    let descriptor: std::os::fd::OwnedFd = channel.into();
    let stream = UnixStream::from(descriptor);
    channel::peer(&stream)?;
    fs::check_private_file(&lifetime)?;
    // SAFETY: both originals have now been checked and duplicated with CLOEXEC.
    // Closing them is required before any adapter spawn or unrelated file open.
    unsafe {
        libc::close(3);
        libc::close(4);
    }
    Ok((stream, lifetime))
}
fn handle(owner: Arc<Owner>, mut stream: UnixStream) -> Result<()> {
    channel::peer(&stream)?;
    if owner.clients.fetch_add(1, Ordering::SeqCst) >= 32 {
        owner.clients.fetch_sub(1, Ordering::SeqCst);
        return Err(Code::Capacity.into());
    }
    let tracked = owner.clone();
    if std::thread::Builder::new()
        .name("connectors-local-request".into())
        .spawn(move || {
            let _ = exchange(&owner, &mut stream);
            owner.clients.fetch_sub(1, Ordering::SeqCst);
        })
        .is_err()
    {
        tracked.clients.fetch_sub(1, Ordering::SeqCst);
        return Err(Code::Unavailable.into());
    }
    Ok(())
}
fn exchange(owner: &Owner, stream: &mut UnixStream) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(10);
    let hello = channel::read::<Request>(stream, until, false, 0)?;
    let Request::Hello {
        version,
        challenge,
        configuration,
        authority,
    } = hello.control
    else {
        return Err(Code::InvalidInput.into());
    };
    if version != VERSION
        || uuid::Uuid::parse_str(&challenge).is_err()
        || configuration != owner.paths.config
        || authority != owner.authority
    {
        return Err(Code::ReadinessMismatch.into());
    }
    channel::write(
        stream,
        &Reply::Hello {
            version: VERSION.into(),
            challenge,
            host_incarnation: owner.incarnation.clone(),
            authority: owner.authority.clone(),
        },
        None,
        &[],
        until,
    )?;
    let frame = channel::read::<Request>(
        stream,
        Instant::now() + Duration::from_secs(10),
        false,
        runtime::INPUT_LIMIT,
    )?;
    let shutdown = matches!(frame.control, Request::Shutdown { .. });
    let result = action(owner, stream, frame.control, frame.document);
    let shutdown = shutdown && result.is_ok();
    let (reply, document) = match result {
        Ok(value) => (
            Reply::Success,
            serde_json::to_vec(&value).map_err(|_| Code::Unavailable)?,
        ),
        Err(error) => (Reply::Failed { error }, Vec::new()),
    };
    let written = channel::write(
        stream,
        &reply,
        None,
        &document,
        Instant::now() + Duration::from_secs(5),
    );
    if shutdown {
        owner.shutdown.store(true, Ordering::SeqCst);
    }
    written?;
    Ok(())
}
fn action(
    owner: &Owner,
    stream: &mut UnixStream,
    request: Request,
    document: Vec<u8>,
) -> Result<Value> {
    use supervisor::{Output, Task};
    if owner.shutdown.load(Ordering::SeqCst) {
        return Err(Code::Unavailable.into());
    }
    if let Request::Shutdown { host_incarnation } = request {
        if host_incarnation != owner.incarnation || !document.is_empty() {
            return Err(Code::IncarnationMismatch.into());
        }
        owner.pool.shutdown()?;
        return Ok(json!({}));
    }
    let alias = match &request {
        Request::Begin { adapter, .. }
        | Request::Revalidate { adapter, .. }
        | Request::Invoke { adapter, .. }
        | Request::Status { adapter }
        | Request::Stop { adapter, .. } => adapter.clone(),
        _ => return Err(Code::InvalidInput.into()),
    };
    let (config, adapter) = selected(&owner.paths, &alias)?;
    match request {
        Request::Revalidate {
            connection,
            expected_revision,
            deadline_ms,
            ..
        } => {
            if !document.is_empty() || deadline_ms > connectors_sdk::now_ms() + 30_000 {
                return Err(Code::InvalidInput.into());
            }
            super::until(deadline_ms)?;
            let profile =
                admit_revalidation(&owner.paths, &alias, &connection, &expected_revision)?;
            let Output::Value(value) = owner.pool.run(
                &alias,
                &adapter,
                Task::Revalidate {
                    connection,
                    revision: expected_revision,
                    profile,
                },
                deadline_ms,
            )?
            else {
                return Err(Code::Unavailable.into());
            };
            Ok(value)
        }
        Request::Begin {
            profile,
            connection,
            expected_revision,
            ..
        } => {
            if !document.is_empty() {
                return Err(Code::InvalidInput.into());
            }
            let registry = registry::Registry::with_system_clock(&owner.paths.state);
            let profile = match (&connection, &expected_revision, profile) {
                (None, None, Some(profile)) => profile,
                (Some(reference), Some(revision), None) => {
                    let old = registry.describe(
                        &adapter.instance_id,
                        &adapter.adapter_id,
                        &adapter.configuration_revision,
                        reference,
                        connectors_sdk::now_ms(),
                        false,
                    )?;
                    if old.revision != *revision {
                        return Err(Code::LifecycleConflict.into());
                    }
                    if matches!(old.state, registry::State::Revoked) {
                        return Err(Code::Revoked.into());
                    }
                    old.profile
                }
                _ => return Err(Code::InvalidInput.into()),
            };
            if !adapter.permissions.profiles.contains(&profile) {
                return Err(Code::Forbidden.into());
            }
            if !custody::available_at(config.secret_service_socket.as_deref()) {
                return Err(Code::CustodyUnavailable.into());
            }
            let Output::Bootstrap(bootstrap, capture_epoch) = owner.pool.run(
                &alias,
                &adapter,
                Task::Ensure { resume: true },
                connectors_sdk::now_ms() + 30_000,
            )?
            else {
                return Err(Code::Unavailable.into());
            };
            // Startup is not authority: repeat the preflight before admitting
            // protected entry, and keep the originally selected custody binding.
            let (latest, _) = selected(&owner.paths, &alias)?;
            if latest.secret_service_socket != config.secret_service_socket {
                return Err(Code::LifecycleConflict.into());
            }
            admit_capture(
                &owner.paths,
                &alias,
                if connection.is_none() {
                    Some(profile.as_str())
                } else {
                    None
                },
                connection.as_deref(),
                expected_revision.as_deref(),
            )?;
            let binding = bootstrap.binding(&profile)?;
            let now = connectors_sdk::now_ms();
            let acquired = match connection {
                Some(reference) => registry.begin_repair(
                    &binding,
                    &reference,
                    expected_revision.as_deref().ok_or(Code::InvalidInput)?,
                    now,
                )?,
                None => registry.begin(&binding, now)?,
            };
            let reference = acquired.reference().to_owned();
            let expiry = now + 300_000;
            let result: Result<Secret> = (|| {
                channel::write(
                    stream,
                    &Reply::Capture {
                        acquisition: reference.clone(),
                        expires_at_ms: expiry,
                        profile: bootstrap.profile(&profile)?.clone(),
                    },
                    None,
                    &[],
                    super::until(expiry)?,
                )?;
                let frame = channel::read::<Request>(stream, super::until(expiry)?, true, 0)?;
                if !matches!(frame.control, Request::Complete) || frame.secret.0.is_empty() {
                    return Err(Code::InvalidInput.into());
                }
                let (_, current) = selected(&owner.paths, &alias)?;
                if current.selection() != adapter.selection() {
                    return Err(Code::LifecycleConflict.into());
                }
                if !current.permissions.profiles.contains(&profile) {
                    return Err(Code::Forbidden.into());
                }
                Ok(frame.secret)
            })();
            let claim = registry.consume(acquired, connectors_sdk::now_ms())?;
            let result = (|| {
                let secret = result?;
                let Output::Baseline(baseline) = owner.pool.run(
                    &alias,
                    &adapter,
                    Task::Validate {
                        profile: profile.clone(),
                        secret: Secret(secret.0.clone()),
                        capture_epoch,
                    },
                    expiry.min(connectors_sdk::now_ms() + 30_000),
                )?
                else {
                    return Err(Code::Unavailable.into());
                };
                let (latest, current) = selected(&owner.paths, &alias)?;
                if current.selection() != adapter.selection()
                    || latest.secret_service_socket != config.secret_service_socket
                {
                    return Err(Code::LifecycleConflict.into());
                }
                if !current.permissions.profiles.contains(&profile) {
                    return Err(Code::Forbidden.into());
                }
                let prepared = registry.prepare(
                    &claim,
                    baseline.into_registry(),
                    secret.0.len(),
                    connectors_sdk::now_ms(),
                )?;
                let store = custody::Store::open_at(
                    prepared.version().scope(),
                    config.secret_service_socket.as_deref(),
                )
                .map_err(|_| Code::CustodyUnavailable)?;
                let connection = registry.store_and_publish(
                    prepared,
                    &secret,
                    &store,
                    connectors_sdk::now_ms,
                )?;
                let description = registry.describe(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    &adapter.configuration_revision,
                    &connection,
                    connectors_sdk::now_ms(),
                    true,
                )?;
                Ok(json!({"connection":connection_value(&alias,description)}))
            })();
            result.map_err(|mut error: Error| {
                if error.code != Code::OutcomeUnknown
                    && let Err(failure) = registry.fail(&claim, connectors_sdk::now_ms())
                {
                    error = failure.into();
                }
                error.acquisition = Some(reference);
                error
            })
        }
        Request::Invoke {
            connection,
            operation,
            schema,
            revision,
            deadline_ms,
            ..
        } => {
            if !(1..=120_000).contains(&deadline_ms.saturating_sub(connectors_sdk::now_ms())) {
                return Err(Code::Timeout.into());
            }
            if !adapter.permissions.operations.contains(&operation) {
                return Err(Code::Forbidden.into());
            }
            admit_invoke(
                &owner.paths,
                &alias,
                &json!({"connection":connection,"operation":operation,"schema":schema,"revision":revision}),
                &document,
            )?;
            match owner.pool.run(
                &alias,
                &adapter,
                Task::Invoke {
                    connection,
                    operation,
                    schema,
                    revision,
                    document,
                },
                deadline_ms,
            )? {
                Output::Value(value) => Ok(value),
                _ => Err(Code::Unavailable.into()),
            }
        }
        Request::Status { .. } if document.is_empty() => owner.pool.status(&alias, &adapter),
        Request::Stop {
            configuration_revision,
            host_incarnation,
            child_incarnation,
            ..
        } if document.is_empty() => {
            if host_incarnation != owner.incarnation {
                return Err(Code::IncarnationMismatch.into());
            }
            owner.pool.stop(
                &alias,
                &adapter,
                &configuration_revision,
                &child_incarnation,
            )
        }
        _ => Err(Code::InvalidInput.into()),
    }
}
