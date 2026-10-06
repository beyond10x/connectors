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
    /// The owner's executable digest; `None` for an owner that predates the
    /// build handshake, which is by definition another build.
    owner_build: Option<String>,
}
/// SHA-256 of this process's own executable image, measured once. Reading
/// `/proc/self/exe` measures the image actually running, even after the file it
/// was started from has been replaced on disk.
fn own_build() -> Result<&'static str> {
    static BUILD: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    BUILD
        .get_or_init(|| {
            std::fs::read("/proc/self/exe")
                .ok()
                .map(|image| hex::encode(Sha256::digest(image)))
        })
        .as_deref()
        .ok_or_else(|| Code::Unavailable.into())
}
pub struct WriteClient(Client);
const WRITE_VERSION: &str = "connectors-owner/2";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteRequest {
    format: String,
    adapter: String,
    connection: String,
    operation: String,
    schema: String,
    revision: String,
    deadline_monotonic_ns: u64,
    idempotency_key: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WriteReply {
    Done,
    Failed { error: Error },
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
/// A request without a well-formed reply may have been carried out: its
/// outcome is unknown, whatever failed while sending or while reading and
/// decoding the reply. An interruption stays one (C03 "or interruption as
/// appropriate"). An owner's own `Failed` reply is definite and never passes here.
fn lost(error: Error) -> Error {
    if error.code == Code::Interrupted {
        error
    } else {
        Code::OutcomeUnknown.into()
    }
}
impl Capture {
    pub fn complete(mut self, secret: &Secret) -> Result<Value> {
        let answer = (|| {
            crate::local::protected::cancellation()?;
            channel::write(
                &mut self.client.stream,
                &Request::Complete,
                Some(secret),
                &[],
                until(self.expires_at_ms)?,
            )?;
            self.client.answer(until(self.expires_at_ms)?)
        })();
        answer
            .unwrap_or_else(|error| Err(lost(error)))
            .map_err(|mut error: Error| {
                error.acquisition = Some(self.acquisition.clone());
                error
            })
    }
}
impl Client {
    pub fn connect(paths: &Paths, start: bool) -> Result<Self> {
        Self::connect_version(
            paths,
            start,
            VERSION,
            Instant::now() + Duration::from_secs(10),
        )
    }
    fn connect_version(
        paths: &Paths,
        start: bool,
        version: &str,
        deadline: Instant,
    ) -> Result<Self> {
        let authority = Metadata::inspect(&paths.state)?.authority()?.to_string();
        let directory = fs::directory(&paths.state, false, true)?;
        let socket = socket_path(&directory);
        loop {
            crate::local::protected::cancellation()?;
            match connect_identified(&socket) {
                Ok((stream, identity)) => {
                    match Self::greet_running(&socket, stream, paths, &authority, version, deadline)
                    {
                        // The owner closed the stream and its socket is no longer
                        // at the path: it retired (owner.md). Start over as if
                        // none was running; a newer owner may already listen.
                        Err(error)
                            if start
                                && error.code == Code::Unavailable
                                && socket_identity(&socket) != Some(identity)
                                && Instant::now() < deadline => {}
                        result => return result,
                    }
                }
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
                    return Self::greet_running(
                        &socket, stream, paths, &authority, version, deadline,
                    );
                }
                let (stream, mut process) = spawn(paths, &lock, deadline)?;
                let result = Self::greet(
                    stream,
                    paths,
                    &authority,
                    version,
                    Some(own_build()?),
                    deadline,
                );
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
    /// Greets an owner that was already running. An owner built before the
    /// build handshake closes the stream on the unknown `build` field without
    /// any reply. A stream can also close without a reply for a reason that has
    /// nothing to do with the build, so a silent close is never taken as proof:
    /// the CLI greets without `build` and then asks for the build on that same
    /// established stream. An owner that answers knows the handshake, and the
    /// greeting with `build` is repeated; only an owner that accepts the
    /// greeting and then closes on the unknown request is identified as one
    /// from before the handshake. A reply of any kind, including a refusal,
    /// keeps its own code. No work request is sent during identification.
    fn greet_running(
        socket: &std::path::Path,
        mut stream: UnixStream,
        paths: &Paths,
        authority: &str,
        version: &str,
        deadline: Instant,
    ) -> Result<Self> {
        let build = own_build()?;
        // Starts at the 20 ms startup-poll pause and doubles, so one transient
        // close costs 20 ms and a persistently odd owner is not flooded.
        let mut pause = Duration::from_millis(20);
        loop {
            let silent = match Self::hello(stream, paths, authority, version, Some(build), deadline)
            {
                Ok(answer) => return answer,
                Err(silent) => silent,
            };
            let probe = Self::greet(
                connect_socket(socket)?,
                paths,
                authority,
                VERSION,
                None,
                deadline,
            )?;
            match probe.probe_build(deadline) {
                Ok(()) => {}
                Err(None) => {
                    return Self::greet(
                        connect_socket(socket)?,
                        paths,
                        authority,
                        VERSION,
                        None,
                        deadline,
                    );
                }
                Err(Some(error)) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(silent);
            }
            std::thread::sleep(pause.min(deadline.saturating_duration_since(Instant::now())));
            pause = (pause * 2).min(Duration::from_secs(1));
            stream = connect_socket(socket)?;
        }
    }
    /// `Ok(())` when the owner answers with its build; `Err(None)` when it
    /// closes the established stream without any reply.
    fn probe_build(mut self, deadline: Instant) -> std::result::Result<(), Option<Error>> {
        let silent = |error: Error| {
            if error.code == Code::Unavailable {
                None
            } else {
                Some(error)
            }
        };
        channel::write(&mut self.stream, &Request::Build, None, &[], deadline)
            .map_err(|error| silent(error.into()))?;
        let frame =
            read_reply(&mut self.stream, deadline, runtime::RESULT_LIMIT).map_err(silent)?;
        match frame.control {
            Reply::Success
                if connectors_core::read_json::<Value>(&frame.document)
                    .ok()
                    .and_then(|value| value.get("build").and_then(Value::as_str).map(str::len))
                    == Some(64) =>
            {
                Ok(())
            }
            Reply::Failed { error } => Err(Some(error)),
            _ => Err(Some(Code::ReadinessMismatch.into())),
        }
    }
    fn greet(
        stream: UnixStream,
        paths: &Paths,
        authority: &str,
        selected_version: &str,
        build: Option<&str>,
        deadline: Instant,
    ) -> Result<Self> {
        Self::hello(stream, paths, authority, selected_version, build, deadline).unwrap_or_else(Err)
    }
    /// The outer `Err` means the stream closed before any reply arrived; the
    /// inner result is the owner's answer.
    fn hello(
        mut stream: UnixStream,
        paths: &Paths,
        authority: &str,
        selected_version: &str,
        build: Option<&str>,
        deadline: Instant,
    ) -> std::result::Result<Result<Self>, Error> {
        let closed = |error: Error| {
            if error.code == Code::Unavailable {
                Err(error)
            } else {
                Ok(Err(error))
            }
        };
        if let Err(error) = channel::peer(&stream) {
            return Ok(Err(error.into()));
        }
        let challenge = uuid::Uuid::new_v4().to_string();
        if let Err(error) = channel::write(
            &mut stream,
            &Request::Hello {
                version: selected_version.into(),
                challenge: challenge.clone(),
                configuration: paths.config.clone(),
                authority: authority.into(),
                build: build.map(str::to_owned),
            },
            None,
            &[],
            deadline,
        ) {
            return closed(error.into());
        }
        let reply = match read_reply(&mut stream, deadline, 0) {
            Ok(reply) => reply,
            Err(error) => return closed(error),
        };
        Ok(match reply.control {
            Reply::Hello {
                version,
                challenge: returned,
                host_incarnation,
                authority: returned_authority,
                build: owner_build,
            } if version == selected_version
                && returned == challenge
                && returned_authority == authority
                && owner_build.is_some() == build.is_some()
                && uuid::Uuid::parse_str(&host_incarnation).is_ok() =>
            {
                Ok(Self {
                    stream,
                    host_incarnation,
                    owner_build,
                })
            }
            Reply::Failed { error } => Err(error),
            _ => Err(Code::ReadinessMismatch.into()),
        })
    }
    pub fn begin(
        mut self,
        adapter: &str,
        profile: Option<String>,
        connection: Option<String>,
        expected_revision: Option<String>,
    ) -> Result<Capture> {
        self.same_build()?;
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
        self.same_build()?;
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
        self.same_build()?;
        crate::local::protected::cancellation()?;
        let deadline = until(deadline_ms)?;
        let answer = (|| {
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
                deadline,
            )?;
            self.answer(deadline)
        })();
        answer.unwrap_or_else(|error| Err(lost(error)))
    }
    /// Ask the owner for an admitted consumer launch. Its reply carries the
    /// consumer's pinned argv; the captured image and the sealed credential
    /// follow as descriptors, which this process hands to the consumer unread.
    pub fn launch(
        mut self,
        adapter: &str,
        connection: &str,
        consumer: &str,
        deadline_ms: u64,
    ) -> Result<runtime::launch::Consumer> {
        self.same_build()?;
        crate::local::protected::cancellation()?;
        let deadline = until(deadline_ms)?;
        channel::write(
            &mut self.stream,
            &Request::Launch {
                adapter: adapter.into(),
                connection: connection.into(),
                consumer: consumer.into(),
                deadline_ms,
            },
            None,
            &[],
            deadline,
        )?;
        let (args, pass_env) = match read_reply(&mut self.stream, deadline, 0)?.control {
            Reply::Launch { args, pass_env } => (args, pass_env),
            Reply::Failed { error } => return Err(error),
            _ => return Err(Code::Unavailable.into()),
        };
        let mut files =
            runtime::launch::receive(&self.stream, runtime::launch::DESCRIPTORS, deadline)?;
        let credential = files.pop().ok_or(Code::Unavailable)?;
        let executable = files.pop().ok_or(Code::Unavailable)?;
        // The owner's final answer follows the release of its read use.
        self.value(deadline)?;
        Ok(runtime::launch::Consumer::new(
            executable, credential, args, pass_env,
        ))
    }
    pub fn status(mut self, adapter: &str) -> Result<Value> {
        self.same_build()?;
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
        self.same_build()?;
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
    /// Refuses work on an owner running a different executable than this CLI.
    /// Stopping or replacing that owner is left to the user; `shutdown` stays
    /// available across builds so the old owner can still be stopped.
    fn same_build(&self) -> Result<()> {
        if self.owner_build.as_deref() == Some(own_build()?) {
            Ok(())
        } else {
            Err(Code::OwnerBuildMismatch.into())
        }
    }
    fn simple(&mut self, request: Request, span: Duration) -> Result<Value> {
        let deadline = Instant::now() + span;
        channel::write(&mut self.stream, &request, None, &[], deadline)?;
        self.value(deadline)
    }
    fn value(&mut self, deadline: Instant) -> Result<Value> {
        self.answer(deadline).unwrap_or_else(Err)
    }
    /// The outer `Err` means no usable reply arrived: the stream failed or
    /// closed, or the frame was not a well-formed answer. The inner result is
    /// the owner's own answer, a success or its definite `Failed` reply.
    fn answer(&mut self, deadline: Instant) -> std::result::Result<Result<Value>, Error> {
        let frame = read_reply(&mut self.stream, deadline, runtime::RESULT_LIMIT)?;
        match frame.control {
            Reply::Success => {
                channel::depth(&frame.document)?;
                connectors_core::read_json(&frame.document)
                    .map(Ok)
                    .map_err(|_| Code::Unavailable.into())
            }
            Reply::Failed { error } if frame.document.is_empty() => Ok(Err(error)),
            _ => Err(Code::Unavailable.into()),
        }
    }
}
impl WriteClient {
    pub fn connect(paths: &Paths, start: bool, deadline: mutation::Deadline) -> Result<Self> {
        Client::connect_version(
            paths,
            start,
            WRITE_VERSION,
            deadline
                .until()?
                .min(Instant::now() + Duration::from_secs(10)),
        )
        .and_then(|client| {
            client.same_build()?;
            Ok(Self(client))
        })
    }
    pub fn invoke(
        mut self,
        adapter: &str,
        request: &approval_issuance::Request<'_>,
        key: Option<&str>,
        proof: Option<&Secret>,
        deadline: mutation::Deadline,
    ) -> Result<mutation::Delivery> {
        let until = deadline.until()?;
        crate::local::protected::cancellation()?;
        let control = WriteRequest {
            format: WRITE_VERSION.into(),
            adapter: adapter.into(),
            connection: request.connection.into(),
            operation: request.operation.into(),
            schema: request.schema.into(),
            revision: request.revision.into(),
            deadline_monotonic_ns: deadline.ticks(),
            idempotency_key: key.map(str::to_owned),
        };
        let response = (|| {
            channel::write(
                &mut self.0.stream,
                &control,
                proof,
                request.input.as_bytes(),
                until,
            )?;
            channel::read_with_cancel::<WriteReply>(
                &mut self.0.stream,
                until,
                false,
                runtime::RESULT_LIMIT,
                Some(&|| {
                    crate::local::protected::cancellation()
                        .map_err(|_| runtime::Failure::Interrupted)
                }),
            )
        })()
        .map_err(|_| Error::from(Code::OutcomeUnknown))?;
        match response.control {
            WriteReply::Done => {
                channel::depth(&response.document).map_err(|_| Code::OutcomeUnknown)?;
                let value: mutation::Delivery = connectors_core::read_json(&response.document)
                    .map_err(|_| Code::OutcomeUnknown)?;
                value.validate()?;
                Ok(value)
            }
            WriteReply::Failed { mut error } if response.document.is_empty() => {
                if matches!(
                    error.code,
                    Code::Timeout | Code::Interrupted | Code::Unavailable
                ) {
                    error.code = Code::OutcomeUnknown;
                }
                Err(error)
            }
            _ => Err(Code::OutcomeUnknown.into()),
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
    connect_identified(path).map(|(stream, _)| stream)
}
/// Also returns the identity of the socket file connected to, so that a caller
/// can tell whether the owner behind it has since removed it.
fn connect_identified(path: &std::path::Path) -> Result<(UnixStream, (u64, u64))> {
    let info = std::fs::symlink_metadata(path).map_err(|_| Code::Unavailable)?;
    if !info.file_type().is_socket() || info.uid() != fs::uid() || info.mode() & 0o077 != 0 {
        return Err(Code::InvalidConfiguration.into());
    }
    let stream = UnixStream::connect(path).map_err(|_| Code::Unavailable)?;
    channel::peer(&stream)?;
    Ok((stream, (info.dev(), info.ino())))
}
fn socket_identity(path: &std::path::Path) -> Option<(u64, u64)> {
    std::fs::symlink_metadata(path)
        .ok()
        .map(|info| (info.dev(), info.ino()))
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
        sha256: hash,
        args: Vec::new(),
    };
    let executable = selection.capture(deadline)?;
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
    build: &'static str,
    pool: Arc<supervisor::Pool>,
    shutdown: AtomicBool,
    clients: AtomicUsize,
}
pub fn serve(paths: Paths) -> Result<()> {
    // Duplicate before opening anything: a direct invocation with missing fds
    // must not mistake newly opened configuration files for inherited authority.
    let (startup, lifetime) = inherited()?;
    let build = own_build()?;
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
    let mut listener = listen(&socket)?;
    let paths = Arc::new(paths);
    let incarnation = uuid::Uuid::new_v4().to_string();
    let owner = Arc::new(Owner {
        paths: paths.clone(),
        authority,
        incarnation: incarnation.clone(),
        build,
        pool: Arc::new(supervisor::Pool::new(paths, incarnation)),
        shutdown: AtomicBool::new(false),
        clients: AtomicUsize::new(0),
    });
    // From the first request onward, keep lifetime authority through kernel
    // process exit, including unwinding or any early return with live threads.
    // CLOEXEC prevents native children from inheriting it.
    std::mem::forget(lifetime);
    let recovery = maintenance::Background::start(owner.paths.clone(), owner.pool.clone())?;
    handle(owner.clone(), startup)?;
    owner.pool.automatic(&config);
    let (mut result, retired) = accept(&owner, &recovery, &mut listener, &socket, idle_bound());
    drop(listener);
    if let Err(error) = recovery.stop() {
        result = Err(error);
    }
    if retired {
        // A recovery pass that ended while retiring may have queued work on an
        // instance worker. Let it run before the supervisor stops; it is bounded.
        let until = Instant::now() + Duration::from_secs(5);
        while !owner.pool.quiet().unwrap_or(true) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    if let Err(error) = owner.pool.shutdown() {
        result = Err(error);
    }
    if !retired && std::fs::remove_file(socket).is_err() {
        result = Err(Code::Unavailable.into());
    }
    result
}
/// Serves connections until shutdown, a listener failure or idle retirement;
/// the flag reports retirement, after which the socket is already gone.
fn accept(
    owner: &Arc<Owner>,
    recovery: &maintenance::Background,
    listener: &mut UnixListener,
    socket: &std::path::Path,
    bound: Duration,
) -> (Result<()>, bool) {
    let mut result = Ok(());
    let mut active_at = Instant::now();
    let mut settled = owner.pool.settled();
    let mut retired = false;
    while !owner.shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                active_at = Instant::now();
                let _ = handle(owner.clone(), stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                let progress = owner.pool.settled();
                if working(owner) || progress != settled {
                    settled = progress;
                    active_at = Instant::now();
                } else if active_at.elapsed() >= bound && quiet(owner, recovery) {
                    match retire(owner, recovery, listener, socket) {
                        Retirement::Exit => {
                            retired = true;
                            break;
                        }
                        Retirement::Serve => {
                            active_at = Instant::now();
                            continue;
                        }
                        // Recovery began during the grace: exit once it ends.
                        Retirement::Retry => continue,
                        Retirement::Failed(error) => {
                            result = Err(error);
                            retired = true;
                            break;
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(_) => {
                result = Err(Code::Unavailable.into());
                break;
            }
        }
    }
    (result, retired)
}
/// contracts/cli/v1alpha1/owner.md: an owner with no client and no child work
/// for this long exits.
const IDLE_EXIT: Duration = Duration::from_secs(600);
/// Test builds may shorten the bound through the environment; a release build
/// has no override. The CLI starts owners with a cleared environment.
#[cfg(debug_assertions)]
fn idle_bound() -> Duration {
    std::env::var("CONNECTORS_TEST_OWNER_IDLE_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(IDLE_EXIT)
}
#[cfg(not(debug_assertions))]
fn idle_bound() -> Duration {
    IDLE_EXIT
}
fn listen(socket: &std::path::Path) -> Result<UnixListener> {
    let listener = UnixListener::bind(socket).map_err(|_| Code::Unavailable)?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| Code::Unavailable)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| Code::Unavailable)?;
    Ok(listener)
}
/// Work that restarts the idle clock: a connected client (including the startup
/// channel) or a queued or running job. Recovery restarts it only by settling an
/// attempt; one that settles nothing, such as a Prepared attempt without a
/// qualified clock, would otherwise keep an owner alive for ever. The attempt
/// stays pending in the store and the next owner resumes it.
fn working(owner: &Owner) -> bool {
    owner.clients.load(Ordering::SeqCst) > 0 || owner.pool.working().unwrap_or(true)
}
/// Nothing at all in flight, recovery included: the owner never exits while a
/// recovery pass is running.
fn quiet(owner: &Owner, recovery: &maintenance::Background) -> bool {
    owner.clients.load(Ordering::SeqCst) == 0
        && !recovery.busy()
        && owner.pool.quiet().unwrap_or(false)
}
enum Retirement {
    Exit,
    Serve,
    Retry,
    Failed(Error),
}
/// Removes the socket so that no later connect reaches this owner, then serves
/// any connection that reached it first. A later CLI finds no socket, waits on
/// the lifetime lock this process holds until exit, and starts a new owner.
/// If anything arrived meanwhile, the owner serves it and listens again.
fn retire(
    owner: &Arc<Owner>,
    recovery: &maintenance::Background,
    listener: &mut UnixListener,
    socket: &std::path::Path,
) -> Retirement {
    if std::fs::remove_file(socket).is_err() {
        return Retirement::Serve;
    }
    // A connect that resolved the path just before removal is queued here.
    let grace = Instant::now() + Duration::from_millis(100);
    let mut arrived = false;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                arrived = true;
                let _ = handle(owner.clone(), stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= grace {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
    if !arrived && quiet(owner, recovery) {
        owner.shutdown.store(true, Ordering::SeqCst);
        return Retirement::Exit;
    }
    match listen(socket) {
        Ok(next) => {
            // Anything still queued on the unlinked listener is served first.
            while let Ok((stream, _)) = listener.accept() {
                let _ = handle(owner.clone(), stream);
            }
            *listener = next;
            if arrived {
                Retirement::Serve
            } else {
                Retirement::Retry
            }
        }
        Err(error) => {
            // Unreachable now: finish what was admitted, then exit.
            while owner.clients.load(Ordering::SeqCst) > 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
            owner.shutdown.store(true, Ordering::SeqCst);
            Retirement::Failed(error)
        }
    }
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
        // Answer instead of closing silently: a stream closed without any reply
        // sends the CLI into build identification. Every earlier build reads
        // this reply as a greeting failure with its existing capacity code.
        let _ = channel::write(
            &mut stream,
            &Reply::Failed {
                error: Code::Capacity.into(),
            },
            None,
            &[],
            Instant::now() + Duration::from_millis(500),
        );
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
        build,
    } = hello.control
    else {
        return Err(Code::InvalidInput.into());
    };
    if (version != VERSION && version != WRITE_VERSION)
        || uuid::Uuid::parse_str(&challenge).is_err()
        || configuration != owner.paths.config
        || authority != owner.authority
    {
        return Err(Code::ReadinessMismatch.into());
    }
    channel::write(
        stream,
        &Reply::Hello {
            version: version.clone(),
            challenge,
            host_incarnation: owner.incarnation.clone(),
            authority: owner.authority.clone(),
            // The caller compares and refuses; answering a caller that did not
            // ask keeps the reply readable by builds before the handshake.
            build: build.map(|_| owner.build.to_owned()),
        },
        None,
        &[],
        until,
    )?;
    if version == WRITE_VERSION {
        return exchange_write(owner, stream);
    }
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
fn exchange_write(owner: &Owner, stream: &mut UnixStream) -> Result<()> {
    let frame = channel::read::<WriteRequest>(
        stream,
        Instant::now() + Duration::from_secs(10),
        true,
        approval_issuance::TARGET_LIMIT,
    )?;
    let until = mutation::Deadline::from_ticks(frame.control.deadline_monotonic_ns)?.until()?;
    let result = write_action(owner, frame.control, frame.secret, frame.document, until);
    let (reply, document) = match result {
        Ok(mut value) => {
            value.validate()?;
            let mut document = serde_json::to_vec(&value).map_err(|_| Code::Unavailable)?;
            if document.len() > runtime::RESULT_LIMIT {
                value.result = None;
                value.error = Some(Code::Capacity.into());
                value.mutation.cause = Some(mutation::Cause {
                    code: connectors_core::ErrorCode::Capacity,
                    stage: mutation::Stage::Response,
                });
                document = serde_json::to_vec(&value).map_err(|_| Code::Unavailable)?;
            }
            (WriteReply::Done, document)
        }
        Err(error) => (WriteReply::Failed { error }, Vec::new()),
    };
    // A terminal response may report a known effect even if metadata work used
    // the last budget. This bounded delivery grace grants no native dispatch.
    channel::write(
        stream,
        &reply,
        None,
        &document,
        Instant::now() + Duration::from_secs(5),
    )?;
    Ok(())
}
fn write_action(
    owner: &Owner,
    request: WriteRequest,
    proof: Secret,
    document: Vec<u8>,
    until: Instant,
) -> Result<mutation::Delivery> {
    if owner.shutdown.load(Ordering::SeqCst) {
        return Err(Code::Unavailable.into());
    }
    if request.format != WRITE_VERSION || proof.0.len() > 20 * 1024 {
        return Err(Code::InvalidInput.into());
    }
    let document = String::from_utf8(document).map_err(|_| Code::InvalidInput)?;
    let target = approval_issuance::Request {
        connection: &request.connection,
        operation: &request.operation,
        schema: &request.schema,
        revision: &request.revision,
        input: &document,
    };
    let original = mutation::observe_original(
        &owner.paths,
        &request.adapter,
        &target,
        request.idempotency_key.as_deref(),
        until,
    )?;
    let (_, adapter) = selected(&owner.paths, &request.adapter)?;
    let deadline = connectors_sdk::now_ms()
        + until.saturating_duration_since(Instant::now()).as_millis() as u64;
    if let Some(original) = original {
        if !original.pending || !owner.pool.idle(&adapter)? {
            return Ok(original.delivery);
        }
        return match owner.pool.run(
            &request.adapter,
            &adapter,
            supervisor::Task::ObserveWrite {
                connection: request.connection,
                operation: request.operation,
                schema: request.schema,
                revision: request.revision,
                document,
                key: request.idempotency_key.ok_or(Code::OutcomeUnknown)?,
                until,
            },
            deadline,
        )? {
            supervisor::Output::Write(value) => Ok(value),
            _ => Err(Code::OutcomeUnknown.into()),
        };
    }
    match owner.pool.run(
        &request.adapter,
        &adapter,
        supervisor::Task::Write {
            connection: request.connection,
            operation: request.operation,
            schema: request.schema,
            revision: request.revision,
            document,
            key: request.idempotency_key,
            proof: if proof.0.is_empty() {
                None
            } else {
                Some(proof)
            },
            until,
        },
        deadline,
    )? {
        supervisor::Output::Write(value) => Ok(value),
        _ => Err(Code::OutcomeUnknown.into()),
    }
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
    if let Request::Build = request {
        if !document.is_empty() {
            return Err(Code::InvalidInput.into());
        }
        return Ok(json!({ "build": owner.build }));
    }
    let alias = match &request {
        Request::Begin { adapter, .. }
        | Request::Revalidate { adapter, .. }
        | Request::Invoke { adapter, .. }
        | Request::Status { adapter }
        | Request::Launch { adapter, .. }
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
            // Admission decides existence, then grant, then revision.
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
        Request::Launch {
            connection,
            consumer,
            deadline_ms,
            ..
        } if document.is_empty() => {
            if !(1..=30_000).contains(&deadline_ms.saturating_sub(connectors_sdk::now_ms())) {
                return Err(Code::Timeout.into());
            }
            launch(
                owner,
                stream,
                &alias,
                &config,
                &connection,
                &consumer,
                deadline_ms,
            )
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

/// A consumer launch, in the owner, which alone reads the credential. Admission
/// is repeated, the consumer image is captured before the credential is read,
/// and the configuration is re-read after it. The read use is held only across
/// the delivery: the sealed copy is the consumer's from then on.
fn launch(
    owner: &Owner,
    stream: &mut UnixStream,
    alias: &str,
    config: &Config,
    connection: &str,
    consumer: &str,
    deadline_ms: u64,
) -> Result<Value> {
    let admitted = admit_launch(&owner.paths, alias, connection, consumer)?;
    let deadline = super::until(deadline_ms)?;
    let executable = admitted.consumer.executable.capture(deadline)?;
    let registry = registry::Registry::with_system_clock(&owner.paths.state);
    let captured = registry
        .capture_read(
            &admitted.binding,
            connection,
            &Default::default(),
            connectors_sdk::now_ms(),
            deadline_ms,
        )
        .map_err(launch_failure)?;
    let version = captured.version();
    let material =
        match custody::Store::open_at(version.scope(), config.secret_service_socket.as_deref())
            .and_then(|store| store.read(version))
        {
            Ok(material) => material,
            Err(error) => {
                if matches!(error, custody::Failure::Missing) {
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
    let checked: Result<()> = (|| {
        let (latest_config, latest) = selected(&owner.paths, alias)?;
        let binding = &admitted.binding;
        if latest_config.consumers.get(consumer) != Some(&admitted.consumer)
            || latest_config.secret_service_socket != config.secret_service_socket
            || latest.instance_id != binding.instance_id
            || latest.adapter_id != binding.adapter_id
            || latest.configuration_revision != binding.configuration_revision
        {
            return Err(Code::LifecycleConflict.into());
        }
        if !latest.permissions.profiles.contains(&binding.profile.id) {
            return Err(Code::Forbidden.into());
        }
        Ok(())
    })();
    if let Err(error) = checked {
        registry.cancel_read(captured, connectors_sdk::now_ms())?;
        return Err(error);
    }
    let dispatched = registry
        .dispatch_read(captured, connectors_sdk::now_ms())
        .map_err(launch_failure)?;
    let delivered: Result<()> = (|| {
        let credential = runtime::launch::sealed(&material.0)?;
        drop(material);
        channel::write(
            stream,
            &Reply::Launch {
                args: admitted.consumer.executable.args.clone(),
                pass_env: admitted.consumer.pass_env.clone(),
            },
            None,
            &[],
            deadline,
        )?;
        runtime::launch::send(stream, &[&executable, &credential], deadline)?;
        Ok(())
    })();
    registry.release_read(dispatched, connectors_sdk::now_ms())?;
    delivered?;
    Ok(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The greeting an owner built before the build handshake understands.
    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum LegacyRequest {
        Hello {
            version: String,
            challenge: String,
            #[allow(dead_code)]
            configuration: PathBuf,
            authority: String,
        },
        Status {
            #[allow(dead_code)]
            adapter: String,
        },
        Shutdown {
            #[allow(dead_code)]
            host_incarnation: String,
        },
    }
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum LegacyReply {
        Hello {
            version: String,
            challenge: String,
            host_incarnation: String,
            authority: String,
        },
        Success,
    }

    /// Serves `connections` exchanges the way an owner from an earlier build
    /// does: a greeting it cannot parse closes the stream without a reply.
    fn legacy_owner(
        listener: UnixListener,
        connections: usize,
        seen: Arc<Mutex<Vec<&'static str>>>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            for _ in 0..connections {
                let (mut stream, _) = listener.accept().unwrap();
                let until = Instant::now() + Duration::from_secs(5);
                let Ok(hello) = channel::read::<LegacyRequest>(&mut stream, until, false, 0) else {
                    seen.lock().unwrap().push("refused_greeting");
                    continue;
                };
                let LegacyRequest::Hello {
                    version,
                    challenge,
                    authority,
                    ..
                } = hello.control
                else {
                    panic!("first frame must be a greeting");
                };
                seen.lock().unwrap().push("greeting");
                channel::write(
                    &mut stream,
                    &LegacyReply::Hello {
                        version,
                        challenge,
                        host_incarnation: uuid::Uuid::new_v4().to_string(),
                        authority,
                    },
                    None,
                    &[],
                    until,
                )
                .unwrap();
                let Ok(frame) =
                    channel::read::<LegacyRequest>(&mut stream, until, false, runtime::INPUT_LIMIT)
                else {
                    continue;
                };
                seen.lock().unwrap().push(match frame.control {
                    LegacyRequest::Status { .. } => "status",
                    LegacyRequest::Shutdown { .. } => "shutdown",
                    LegacyRequest::Hello { .. } => "hello",
                });
                channel::write(&mut stream, &LegacyReply::Success, None, b"{}", until).unwrap();
            }
        })
    }

    #[test]
    fn an_owner_from_an_earlier_build_is_refused_by_name_and_left_running() {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        let directory = fs::directory(&paths.state, false, true).unwrap();
        let socket = socket_path(&directory);
        let listener = UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let owner = legacy_owner(listener, 6, seen.clone());

        let refused = Client::connect(&paths, false)
            .and_then(|client| client.status("forge"))
            .map_err(|error| serde_json::to_value(error.code).unwrap());
        assert_eq!(refused, Err(json!("owner_build_mismatch")));

        // The remedy stays reachable: the earlier owner can still be asked to stop.
        let client = Client::connect(&paths, false).unwrap();
        let host = client.host_incarnation.clone();
        client.shutdown(&host).unwrap();
        owner.join().unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            [
                // status: silent close, greeting then the build probe it cannot
                // parse (closed, unrecorded), greeting that is refused client-side
                "refused_greeting",
                "greeting",
                "greeting",
                // shutdown stays reachable across builds
                "refused_greeting",
                "greeting",
                "greeting",
                "shutdown"
            ],
            "no work request may reach an owner from another build"
        );
    }

    /// story:owner-idle-exit: a CLI whose connection a retiring owner closes
    /// after removing its socket treats that owner as gone. It waits on the
    /// lifetime lock and greets the next owner instead of reporting a failure.
    /// The test holds the lock throughout, so this CLI never spawns one itself.
    #[test]
    fn a_connection_closed_by_a_retiring_owner_reaches_the_next_owner() {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        let directory = fs::directory(&paths.state, false, true).unwrap();
        let held = lock(&directory).unwrap();
        // SAFETY: a live descriptor owned by `held`.
        assert_eq!(
            unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let socket = socket_path(&directory);
        let retiring = UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        let cli = {
            let paths = Paths {
                config: paths.config.clone(),
                state: paths.state.clone(),
            };
            std::thread::spawn(move || {
                Client::connect_version(
                    &paths,
                    true,
                    VERSION,
                    Instant::now() + Duration::from_secs(10),
                )
                .map(|client| client.host_incarnation)
                .map_err(|error| error.code)
            })
        };
        // The retiring owner: the CLI's connection reached it before removal.
        let (stream, _) = retiring.accept().unwrap();
        std::fs::remove_file(&socket).unwrap();
        drop(retiring);
        drop(stream);
        // The next owner starts only after the retiring one has exited.
        std::thread::sleep(Duration::from_millis(300));

        let next = UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        next.set_nonblocking(true).unwrap();
        // A current-build owner: it answers a greeting with or without `build`
        // and the build probe, until a CLI greets it with `build`.
        let owner = std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(10);
            let host = uuid::Uuid::new_v4().to_string();
            let build = own_build().unwrap().to_owned();
            while Instant::now() < until {
                let Ok((mut stream, _)) = next.accept() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                let Ok(hello) = channel::read::<Request>(&mut stream, until, false, 0) else {
                    continue;
                };
                let Request::Hello {
                    version,
                    challenge,
                    authority,
                    build: asked,
                    ..
                } = hello.control
                else {
                    continue;
                };
                let named = asked.is_some();
                let reply = Reply::Hello {
                    version,
                    challenge,
                    host_incarnation: host.clone(),
                    authority,
                    build: asked.map(|_| build.clone()),
                };
                if channel::write(&mut stream, &reply, None, &[], until).is_err() {
                    continue;
                }
                if named {
                    return Some(host);
                }
                if let Ok(frame) = channel::read::<Request>(&mut stream, until, false, 0)
                    && matches!(frame.control, Request::Build)
                {
                    let document = serde_json::to_vec(&json!({ "build": build })).unwrap();
                    let _ = channel::write(&mut stream, &Reply::Success, None, &document, until);
                }
            }
            None
        });
        let greeted = cli
            .join()
            .unwrap()
            .map_err(|code| serde_json::to_value(code).unwrap());
        assert!(greeted.is_ok(), "the CLI reported {greeted:?}");
        drop(held);
        let host = owner.join().unwrap();
        assert_eq!(greeted, Ok(host.expect("the next owner was never greeted")));
    }
}

/// story:owner-idle-exit with the real accept loop, recovery thread and pool,
/// and an idle bound spanning several 5 s recovery sweeps.
#[cfg(test)]
mod idle_sweep_tests {
    use super::*;
    use crate::local::{mutations as ledger, registry};

    struct FixedClock;
    impl ledger::Clock for FixedClock {
        fn now(&self) -> ledger::Result<ledger::ClockInterval> {
            let now = connectors_sdk::now_ms() as i64;
            Ok(ledger::ClockInterval {
                lower_unix_ms: now,
                upper_unix_ms: now + 2000,
            })
        }
    }

    /// A Prepared attempt, as an owner that stopped before dispatch leaves one.
    fn pending_attempt(state: &std::path::Path) -> ledger::AttemptRef {
        let mut metadata = Metadata::update_mutations(state).unwrap();
        metadata
            .connection
            .execute_batch(
                "INSERT INTO registry_instances VALUES ('instance','adapter','config',0);",
            )
            .unwrap();
        let binding = registry::fixture_binding("instance");
        metadata
            .connection
            .execute(
                "INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1',?1)",
                [serde_json::to_string(&binding.profile).unwrap()],
            )
            .unwrap();
        metadata
            .connection
            .execute(
                "INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms)
                 VALUES ('connection','instance','profile',?1,'scope','revision','fence','live',1,1)",
                [serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        metadata.persist().unwrap();
        drop(metadata);
        let store = ledger::Store::new(state, FixedClock, ledger::Limits::default()).unwrap();
        let candidate = ledger::Candidate {
            namespace: ledger::Namespace {
                receiver_instance: "instance".into(),
                tenant: None,
                realm: None,
                caller: "caller".into(),
                executor: None,
                origin: ledger::Origin::Direct,
            },
            fingerprint: ledger::Fingerprint {
                operation: ledger::OperationRef {
                    instance: "instance".into(),
                    adapter: "adapter".into(),
                    operation: "operation".into(),
                },
                connection_ref: "connection".into(),
                connection_revision: "revision".into(),
                contract_ref: "operations/v1alpha1".into(),
                profile: "mutation".into(),
                descriptor_revision: "descriptor".into(),
                configuration_revision: "config".into(),
                canonicalization_version: "adapter-v1-canonical-json".into(),
                input_digest: "a".repeat(64),
                route: None,
            },
            caller_key: Some("key".into()),
            request_id: "request".into(),
            approval: ledger::Approval::NotRequired,
        };
        match store.prepare(&candidate).unwrap() {
            ledger::Preparation::Prepared(prepared) => prepared.reference(),
            ledger::Preparation::Existing(_) => panic!("unexpected existing attempt"),
        }
    }

    /// Runs the owner's accept loop with no client and returns how long it took
    /// to retire, or `None` if it was still serving `bound` + 10 s later.
    fn idle_exit(state: &std::path::Path, bound: Duration) -> Option<Duration> {
        let paths = Arc::new(Paths {
            // No configuration: recovery has no qualified clock.
            config: state.join("absent/config.toml"),
            state: state.to_owned(),
        });
        let incarnation = uuid::Uuid::new_v4().to_string();
        let owner = Arc::new(Owner {
            paths: paths.clone(),
            authority: Metadata::inspect(state)
                .unwrap()
                .authority()
                .unwrap()
                .to_string(),
            incarnation: incarnation.clone(),
            build: "test",
            pool: Arc::new(supervisor::Pool::new(paths.clone(), incarnation)),
            shutdown: AtomicBool::new(false),
            clients: AtomicUsize::new(0),
        });
        let recovery = maintenance::Background::start(paths, owner.pool.clone()).unwrap();
        let directory = fs::directory(state, false, true).unwrap();
        let socket = socket_path(&directory);
        let mut listener = listen(&socket).unwrap();
        let (done, finished) = std::sync::mpsc::channel();
        let serving = owner.clone();
        let started = Instant::now();
        let loop_thread = std::thread::spawn(move || {
            // Keeps the descriptor that `socket` names open while the loop uses it.
            let _directory = &directory;
            let (result, retired) = accept(&serving, &recovery, &mut listener, &socket, bound);
            let _ = done.send(started.elapsed());
            recovery.stop().unwrap();
            (result, retired)
        });
        let exit = finished.recv_timeout(bound + Duration::from_secs(10)).ok();
        owner.shutdown.store(true, Ordering::SeqCst);
        let (result, retired) = loop_thread.join().unwrap();
        owner.pool.shutdown().unwrap();
        if exit.is_some() {
            assert!(result.is_ok() && retired, "the owner retired cleanly");
            assert!(!state.join("owner.sock").exists());
        }
        exit
    }

    fn state_dir() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
        drop(Metadata::initialize(&state).unwrap());
        (root, state)
    }

    /// Nothing pending: the 5 s sweeps inside a 15 s bound are not work.
    #[test]
    fn an_idle_owner_exits_at_a_bound_spanning_several_empty_recovery_sweeps() {
        let (_root, state) = state_dir();
        let bound = Duration::from_secs(15);
        let exit = idle_exit(&state, bound)
            .expect("an owner with nothing pending outlived its 15 s bound by 10 s");
        assert!(exit >= bound, "the owner exited before its bound: {exit:?}");
    }

    /// A Prepared attempt with no qualified clock cannot settle. Recovery that
    /// settles nothing does not keep the owner alive, and the attempt stays
    /// pending for the next owner.
    #[test]
    fn an_unsettleable_pending_attempt_does_not_keep_an_owner_alive() {
        let (_root, state) = state_dir();
        let reference = pending_attempt(&state);
        let bound = Duration::from_secs(15);
        let exit = idle_exit(&state, bound)
            .expect("an owner whose recovery settles nothing outlived its 15 s bound by 10 s");
        assert!(exit >= bound, "the owner exited before its bound: {exit:?}");
        let store = ledger::Store::new(&state, FixedClock, ledger::Limits::default()).unwrap();
        assert_eq!(
            store.observe(reference).unwrap().state,
            ledger::State::Prepared,
            "the attempt is left for the next owner"
        );
    }
}
