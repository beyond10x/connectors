use super::*;
use connectors_sdk::Secret;
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(all(unix, any(target_os = "linux", test)))]
use std::os::fd::FromRawFd;
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
#[cfg(unix)]
use std::{
    os::{
        fd::AsRawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Command, Stdio},
};

/// How a read decodes the result a child wrote.
type Decode = fn(&[u8]) -> Result<serde_json::Value>;

/// The step at which a start of an instance's program failed
/// (`connectors.cli.StartFailure`, ess/domains/cli.yaml;
/// story:program-start-failure-reason). It is data beside the start's
/// failure code, which it never changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartFailure {
    /// The configured executable's path was not admitted.
    Admission,
    /// The executable's content is not the pinned one: its digest or its
    /// executable format differs.
    Digest,
    /// The owner's private copy of the executable could not be made or
    /// launched: the sealed snapshot on Linux, the owner-private copy on
    /// macOS (`artifact::launch_path`), the deny-write copy on Windows.
    Copy,
    /// The process, or the channel or standard-error pipe it starts with,
    /// could not be created.
    Spawn,
    /// The program exited before its handshake finished, with its exit code
    /// or ended by a signal.
    ExitedBeforeHandshake {
        code: Option<i64>,
        signal: Option<i32>,
    },
    /// The handshake did not finish by its deadline.
    HandshakeDeadline,
    /// The program answered the handshake with anything but the readiness
    /// its configuration expects (`readiness_mismatch`), or broke its channel
    /// while still running.
    HandshakeMismatch,
    /// The program answered the handshake by refusing the private protocol its
    /// entry selects (`unsupported_private_protocol`), naming those it speaks
    /// (story:adapter-private-protocol-refusal-named). It is answered as a
    /// failure of its own, not an observation: `instances status` never prints
    /// it as `start_failure`.
    UnsupportedPrivateProtocol { supported: SupportedProtocols },
}
/// The private protocols a refusing program said it speaks, among those this
/// host knows (a version it does not know is not kept).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupportedProtocols(u8);
impl SupportedProtocols {
    fn bit(protocol: PrivateProtocol) -> u8 {
        match protocol {
            PrivateProtocol::V1 => 1,
            PrivateProtocol::V2 => 2,
            PrivateProtocol::V3 => 4,
        }
    }
    fn named(versions: &[String]) -> Self {
        Self(
            versions
                .iter()
                .filter_map(|version| PrivateProtocol::parse(version))
                .fold(0, |bits, protocol| bits | Self::bit(protocol)),
        )
    }
    /// The protocols, lowest first.
    pub fn protocols(self) -> Vec<PrivateProtocol> {
        [
            PrivateProtocol::V1,
            PrivateProtocol::V2,
            PrivateProtocol::V3,
        ]
        .into_iter()
        .filter(|protocol| self.0 & Self::bit(*protocol) != 0)
        .collect()
    }
}
impl StartFailure {
    /// The protocols a program that refused its entry's said it speaks.
    pub fn supported_private_protocols(self) -> Vec<PrivateProtocol> {
        match self {
            Self::UnsupportedPrivateProtocol { supported } => supported.protocols(),
            _ => Vec::new(),
        }
    }
    /// The value `instances status` prints as `start_failure`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admission => "admission",
            Self::Digest => "digest",
            Self::Copy => "copy",
            Self::Spawn => "spawn",
            Self::ExitedBeforeHandshake { .. } => "exited_before_handshake",
            Self::HandshakeDeadline => "handshake_deadline",
            Self::HandshakeMismatch => "handshake_mismatch",
            Self::UnsupportedPrivateProtocol { .. } => "unsupported_private_protocol",
        }
    }
}

/// A failed [`Child::start`]: the failure code the caller answers, as
/// before, the step that failed and the start log the attempt kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartError {
    pub failure: Failure,
    pub step: StartFailure,
    pub log: Option<PathBuf>,
}

/// The directory under the state directory holding one start log per instance.
pub const START_LOGS: &str = "start-logs";
/// The most bytes of a program's standard error a start log keeps.
pub const START_LOG_LIMIT: usize = 64 * 1024;

/// The file name of `instance`'s start log: the id with every byte other than
/// an ASCII letter, digit, `-` or `_` written as `%XX`, so distinct ids never
/// share a name and no name is `.`, `..` or holds a separator; an id whose
/// name would be empty or longer than 200 bytes is named by its SHA-256.
pub fn start_log_name(instance: &str) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    let mut name = String::new();
    for byte in instance.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            name.push(char::from(byte));
        } else {
            let _ = write!(name, "%{byte:02X}");
        }
    }
    if name.is_empty() || name.len() > 200 {
        name = format!(
            "sha256-{}",
            hex::encode(sha2::Sha256::digest(instance.as_bytes()))
        );
    }
    name + ".log"
}

/// The owner's end of the pipe a starting program writes its standard error
/// to: a thread that reads it while the program starts, keeping the first
/// [`START_LOG_LIMIT`] bytes and dropping the rest, so a program that writes
/// more than the pipe holds before its handshake is never held up by it.
/// Windows starts a program with no standard handles and keeps none.
#[cfg(unix)]
struct Stderr {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    reader: Option<std::thread::JoinHandle<Vec<u8>>>,
}
#[cfg(windows)]
type Stderr = std::convert::Infallible;

#[cfg(unix)]
impl Stderr {
    /// Ends the reading: what was written before this call is taken, the
    /// owner's end is closed, and the kept bytes are returned.
    fn finish(mut self) -> Vec<u8> {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        self.reader
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default()
    }
}
#[cfg(unix)]
impl Drop for Stderr {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Reads the owner's non-blocking end of a start's standard-error pipe until
/// the program closes its end or `stop` is set, keeping the first
/// [`START_LOG_LIMIT`] bytes. Once `stop` is set one last pass takes what is
/// already in the pipe, bounded so that a writer which never stops cannot hold
/// the start.
#[cfg(unix)]
fn drain(mut reader: std::io::PipeReader, stop: &std::sync::atomic::AtomicBool) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let stopping = stop.load(std::sync::atomic::Ordering::Acquire);
        let mut taken = 0usize;
        loop {
            if stopping && taken >= 4 * START_LOG_LIMIT {
                return kept;
            }
            match std::io::Read::read(&mut reader, &mut buffer) {
                Ok(0) => return kept,
                Ok(count) => {
                    taken += count;
                    let take = count.min(START_LOG_LIMIT - kept.len());
                    kept.extend_from_slice(&buffer[..take]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                // Nothing more is buffered, or the pipe failed.
                Err(_) => break,
            }
        }
        if stopping {
            return kept;
        }
        let mut ready = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one live pollfd on a descriptor this function owns; a
        // timeout so `stop` is seen while the program writes nothing.
        unsafe { libc::poll(&mut ready, 1, 20) };
    }
}

/// Ends the reading of `stderr`, closing the owner's end, and, given the
/// state directory, publishes the first [`START_LOG_LIMIT`] bytes the program
/// wrote as `instance`'s start log: an owner-only file, replacing the one
/// before, never written through a link
/// ([`crate::local::filesystem::publish_replace`]). A log that cannot be
/// kept leaves the start's outcome as it is.
#[cfg(unix)]
fn keep_start_log(stderr: Option<Stderr>, state: Option<&Path>, instance: &str) -> Option<PathBuf> {
    use crate::local::filesystem;
    let kept = stderr.map(Stderr::finish).unwrap_or_default();
    let state = state?;
    filesystem::directory(state, false, true).ok()?;
    let directory = state.join(START_LOGS);
    let held = filesystem::directory(&directory, true, true).ok()?;
    let name = start_log_name(instance);
    filesystem::publish_replace(&held, std::ffi::OsStr::new(&name), &kept).ok()?;
    Some(directory.join(name))
}
#[cfg(windows)]
fn keep_start_log(
    _stderr: Option<Stderr>,
    _state: Option<&Path>,
    _instance: &str,
) -> Option<PathBuf> {
    None
}

/// The pipe a starting program's standard error goes to, with the thread
/// that reads the owner's end ([`Stderr`]). That end does not block, so the
/// thread sees a stop while the program writes nothing.
#[cfg(unix)]
fn stderr_pipe() -> Result<(Stderr, std::io::PipeWriter)> {
    let (reader, writer) = std::io::pipe().map_err(|_| Failure::Unavailable)?;
    let fd = reader.as_raw_fd();
    // SAFETY: fcntl on a live descriptor this function owns, with scalar arguments.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: as above; only the status flags of the owner's end change.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(Failure::Unavailable);
    }
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = std::sync::Arc::clone(&stop);
    let reader = std::thread::Builder::new()
        .name("connectors-start-log".into())
        .spawn(move || drain(reader, &seen))
        .map_err(|_| Failure::Unavailable)?;
    Ok((
        Stderr {
            stop,
            reader: Some(reader),
        },
        writer,
    ))
}

/// A program that exited before its handshake finished, with its status.
#[cfg(unix)]
fn ended(status: std::process::ExitStatus) -> StartFailure {
    use std::os::unix::process::ExitStatusExt;
    StartFailure::ExitedBeforeHandshake {
        code: status.code().map(i64::from),
        signal: status.signal(),
    }
}
#[cfg(windows)]
fn ended(code: u32) -> StartFailure {
    StartFailure::ExitedBeforeHandshake {
        code: Some(i64::from(code)),
        signal: None,
    }
}

/// Owned process, never reconstructed from a PID or readiness file. It is not
/// Send: its spawning thread must remain alive for the Linux parent-death lease.
/// On macOS the process is an [`artifact::ExactChild`]: signalled only under
/// the lock that reaps it, with no pidfd (`child_reap_lock`). On Windows it is
/// an [`artifact::Process`], owned through its process handle, in the owner's
/// kill-on-close job; its image copy is removed once it has exited.
pub struct Child {
    #[cfg(target_os = "linux")]
    process: std::process::Child,
    #[cfg(target_os = "linux")]
    pidfd: File,
    #[cfg(target_os = "macos")]
    process: artifact::ExactChild,
    #[cfg(windows)]
    process: artifact::Process,
    /// Dropped after the process has been ended and waited for.
    #[cfg(windows)]
    _image: artifact::Launch,
    channel: channel::Stream,
    bootstrap: Bootstrap,
    incarnation: String,
    protocol: PrivateProtocol,
    live: bool,
    _parent_thread: PhantomData<Rc<()>>,
}
impl Child {
    pub fn spawn(config: &super::super::config::Adapter) -> Result<Self> {
        Self::spawn_until(config, Instant::now() + Duration::from_secs(10))
    }
    /// [`Child::spawn`] with a start budget the caller chooses, past the
    /// ten seconds `spawn` and `start` cap a start at. For a test in another
    /// crate, where `cfg(test)` of this one is not visible, that runs the
    /// adapter on a loaded host.
    #[doc(hidden)]
    pub fn spawn_within(config: &super::super::config::Adapter, budget: Duration) -> Result<Self> {
        Self::start_capped(config, Instant::now() + budget, budget, None)
            .map_err(|error| error.failure)
    }
    pub(crate) fn spawn_until(
        config: &super::super::config::Adapter,
        deadline: Instant,
    ) -> Result<Self> {
        Self::start(config, deadline, None).map_err(|error| error.failure)
    }
    /// [`Child::spawn_until`], naming the step a failed start failed at
    /// ([`StartFailure`]). Given the state directory, the program's standard
    /// error up to the end of its handshake is kept, at most
    /// [`START_LOG_LIMIT`] bytes, as the instance's start log under
    /// [`START_LOGS`], replaced on each start; the error names its path. The
    /// owner closes its end of that pipe before this returns, so nothing the
    /// program writes once it can hold a credential is kept. Without a state
    /// directory standard error goes nowhere, as it always did.
    pub(crate) fn start(
        config: &super::super::config::Adapter,
        deadline: Instant,
        state: Option<&Path>,
    ) -> std::result::Result<Self, StartError> {
        Self::start_capped(config, deadline, Duration::from_secs(10), state)
    }
    fn start_capped(
        config: &super::super::config::Adapter,
        deadline: Instant,
        cap: Duration,
        state: Option<&Path>,
    ) -> std::result::Result<Self, StartError> {
        let mut stderr = None;
        let started = Self::launch(config, deadline, cap, &mut stderr);
        // A failed start's process has been ended and reaped by now: the log
        // holds what it wrote, and nothing more can arrive.
        let log = keep_start_log(stderr, state, &config.instance_id);
        started.map_err(|(failure, step)| StartError { failure, step, log })
    }
    fn launch(
        config: &super::super::config::Adapter,
        deadline: Instant,
        cap: Duration,
        stderr: &mut Option<Stderr>,
    ) -> std::result::Result<Self, (Failure, StartFailure)> {
        let spawn = |failure| (failure, StartFailure::Spawn);
        let until = deadline.min(Instant::now() + cap);
        let executable = config.executable.capture_explained(until)?;
        #[cfg(windows)]
        let _ = stderr;
        #[cfg(windows)]
        let (process, image, channel) = spawn_windows(config, executable).map_err(spawn)?;
        #[cfg(unix)]
        let (channel, child_channel) =
            UnixStream::pair().map_err(|_| spawn(Failure::Unavailable))?;
        #[cfg(unix)]
        channel::peer(&channel).map_err(spawn)?;
        #[cfg(unix)]
        let child_fd = child_channel.as_raw_fd();
        #[cfg(unix)]
        let (reader, writer) = stderr_pipe().map_err(spawn)?;
        #[cfg(unix)]
        {
            *stderr = Some(reader);
        }
        // SAFETY: getpid has no preconditions.
        #[cfg(unix)]
        let parent = unsafe { libc::getpid() };
        #[cfg(target_os = "linux")]
        let mut command = Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
        // macOS executes the owner-private copy by its path (`owner_private_copy`).
        #[cfg(target_os = "macos")]
        let launch =
            artifact::launch_path(&executable).map_err(|failure| (failure, StartFailure::Copy))?;
        #[cfg(target_os = "macos")]
        let mut command = Command::new(launch.path());
        // Standard error goes to the owner's pipe until the handshake ends
        // (`Child::start`); the SDK's `serve` then points it at /dev/null.
        #[cfg(unix)]
        command
            .args(&config.executable.args)
            .args(["--connectors-private-fd", "3"])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(writer);
        // SAFETY: the hook uses only async-signal-safe syscalls, captured scalar
        // values and fixed errors. Both source descriptors remain held by spawn.
        #[cfg(unix)]
        unsafe {
            command.pre_exec(move || {
                #[cfg(target_os = "linux")]
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                // macOS has no parent-death signal (`adapter_channel_close`):
                // the adapter exits when its channel on descriptor 3 closes.
                #[cfg(target_os = "macos")]
                if libc::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                if libc::dup2(child_fd, 3) < 0 || libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        #[cfg(target_os = "linux")]
        let mut process = command.spawn().map_err(|_| spawn(Failure::Unavailable))?;
        #[cfg(target_os = "macos")]
        let process =
            artifact::ExactChild::spawn(&mut command).map_err(|_| spawn(Failure::Unavailable))?;
        // The command holds the pipe's write end: only the child keeps one.
        #[cfg(unix)]
        drop(command);
        #[cfg(unix)]
        drop(child_channel);
        #[cfg(unix)]
        drop(executable);
        #[cfg(target_os = "macos")]
        drop(launch);
        // SAFETY: pidfd_open takes the just-spawned child's PID and zero flags.
        #[cfg(target_os = "linux")]
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, process.id(), 0) } as i32;
        #[cfg(target_os = "linux")]
        if raw < 0 {
            let _ = process.kill();
            let _ = process.wait();
            return Err(spawn(Failure::Unavailable));
        }
        // SAFETY: the successful syscall returned a new owned descriptor.
        #[cfg(target_os = "linux")]
        let pidfd = unsafe { File::from_raw_fd(raw) };
        let incarnation = uuid::Uuid::new_v4().to_string();
        let nonce = uuid::Uuid::new_v4().to_string();
        // Build an owned guard before any fallible channel work.
        let mut child = Self {
            process,
            #[cfg(target_os = "linux")]
            pidfd,
            #[cfg(windows)]
            _image: image,
            channel,
            bootstrap: Bootstrap {
                bindings: None,
                credential_revision: None,
                executable: None,
                instance: String::new(),
                adapter: String::new(),
                protocol: String::new(),
                configuration_revision: String::new(),
                provider_authority: String::new(),
                descriptor: String::new(),
                profiles: Vec::new(),
                requirements: Vec::new(),
            },
            incarnation: incarnation.clone(),
            protocol: config.private_protocol(),
            live: true,
            _parent_thread: PhantomData,
        };
        let response = channel::write(
            &mut child.channel,
            &Request::Hello {
                version: config.private_protocol().as_str().into(),
                nonce: nonce.clone(),
                child_incarnation: incarnation.clone(),
            },
            None,
            &[],
            until,
        )
        .and_then(|_| channel::read::<Reply>(&mut child.channel, until, false, 0));
        let response = match response {
            Ok(response) => response,
            Err(failure) => return Err((failure, child.interrupted(failure))),
        };
        let mismatch = (Failure::ReadinessMismatch, StartFailure::HandshakeMismatch);
        // The program refuses the protocol this entry selects, by name.
        if let Reply::Refused { version, supported } = &response.control {
            if version != config.private_protocol().as_str() {
                return Err(mismatch);
            }
            return Err((
                Failure::UnsupportedPrivateProtocol,
                StartFailure::UnsupportedPrivateProtocol {
                    supported: SupportedProtocols::named(supported),
                },
            ));
        }
        let Reply::Ready {
            version,
            nonce: returned,
            child_incarnation,
            bootstrap,
        } = response.control
        else {
            return Err(mismatch);
        };
        bootstrap
            .validate_for(config.private_protocol())
            .map_err(|_| mismatch)?;
        if version != config.private_protocol().as_str()
            || returned != nonce
            || child_incarnation != incarnation
            || bootstrap.instance != config.instance_id
            || bootstrap.adapter != config.adapter_id
            || bootstrap.protocol != config.protocol
            || bootstrap.configuration_revision != config.configuration_revision
            // A credential-side digest the entry states is the implementation's,
            // as its configuration revision is (D12).
            || config
                .credential_revision
                .as_ref()
                .is_some_and(|stated| bootstrap.credential_revision.as_ref() != Some(stated))
        {
            return Err(mismatch);
        }
        let mut bootstrap = bootstrap;
        bootstrap.configure(config);
        match child.process.try_wait() {
            Ok(None) => {}
            Ok(Some(status)) => return Err((Failure::Unavailable, ended(status))),
            Err(_) => return Err(spawn(Failure::Unavailable)),
        }
        child.bootstrap = bootstrap;
        Ok(child)
    }
    /// The step a handshake that failed with `failure` failed at: its
    /// deadline, the program's exit or a broken handshake. A program that
    /// exits closes its channel a moment before its exit can be reaped, so
    /// the exit is awaited briefly.
    fn interrupted(&mut self, failure: Failure) -> StartFailure {
        if failure == Failure::Timeout {
            return StartFailure::HandshakeDeadline;
        }
        let until = Instant::now() + Duration::from_millis(500);
        loop {
            match self.process.try_wait() {
                Ok(Some(status)) => return ended(status),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
                _ => return StartFailure::HandshakeMismatch,
            }
        }
    }
    pub fn bootstrap(&self) -> &Bootstrap {
        &self.bootstrap
    }
    pub fn incarnation(&self) -> &str {
        &self.incarnation
    }
    /// Duplicate this owned kernel handle for the owner's independent stop path.
    #[cfg(target_os = "linux")]
    pub(crate) fn termination_handle(&self) -> Result<File> {
        self.pidfd.try_clone().map_err(|_| Failure::Unavailable)
    }
    /// Share this child's reap lock with the owner's independent stop path.
    #[cfg(target_os = "macos")]
    pub(crate) fn termination_handle(&self) -> Result<artifact::ExactChild> {
        self.process.try_clone().map_err(|_| Failure::Unavailable)
    }
    /// Duplicate the process handle for the owner's independent stop path.
    #[cfg(windows)]
    pub(crate) fn termination_handle(&self) -> Result<std::os::windows::io::OwnedHandle> {
        self.process
            .try_clone_handle()
            .map_err(|_| Failure::Unavailable)
    }
    pub fn running(&mut self) -> Result<bool> {
        if self.live
            && self
                .process
                .try_wait()
                .map_err(|_| Failure::Unavailable)?
                .is_some()
        {
            self.live = false;
        }
        Ok(self.live)
    }
    pub fn validate(
        &mut self,
        profile: &str,
        secret: &Secret,
        deadline_ms: u64,
    ) -> Result<Baseline> {
        self.validate_explained(profile, secret, deadline_ms)
            .map_err(|refusal| refusal.failure)
    }
    /// [`Child::validate`] with the child's reason beside `insufficient_scope`,
    /// admitted again here against `secret`: the answers that refused each
    /// alternative binding (private-adapter.md, "Alternative bindings"). A
    /// reason beside any other code, and a baseline naming a binding the
    /// bootstrap does not declare, are protocol violations.
    pub fn validate_explained(
        &mut self,
        profile: &str,
        secret: &Secret,
        deadline_ms: u64,
    ) -> std::result::Result<Baseline, Refusal> {
        self.bootstrap.profile(profile)?;
        let id = uuid::Uuid::new_v4().to_string();
        let frame = self.exchange(
            Request::Validate {
                request_id: id.clone(),
                profile: profile.to_owned(),
                deadline_ms,
            },
            Some(secret),
            &[],
            deadline_ms,
        )?;
        match frame.control {
            Reply::Validated {
                request_id,
                baseline,
            } if request_id == id
                && frame.document.is_empty()
                && self.bootstrap.declares_binding(baseline.binding.as_deref()) =>
            {
                Ok(baseline)
            }
            // A failed validation carries a reason only beside
            // `insufficient_scope`; one beside any other code is a protocol
            // violation, like any other malformed reply.
            Reply::Failed {
                request_id,
                code,
                reason,
                retry_after_seconds: None,
            } if request_id == id
                && frame.document.is_empty()
                && (reason.is_none() || code == Failure::InsufficientScope) =>
            {
                Err(Refusal {
                    failure: code,
                    reason: super::admitted(reason, secret),
                    retry_after_seconds: None,
                })
            }
            _ => {
                self.terminate();
                Err(Failure::Protocol.into())
            }
        }
    }
    /// A transport operation only. The caller must already have admitted current
    /// policy, scopes, exact material and final dispatch through the registry.
    pub fn invoke(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> Result<Vec<u8>> {
        self.invoke_explained(operation, revision, partition, secret, input, deadline_ms)
            .map_err(|refusal| refusal.failure)
    }
    /// [`Child::invoke`] with the upstream's reason for a failure, admitted
    /// again here against `secret` (see [`super::admitted`]), and the delay
    /// the provider named beside a rate-limit failure.
    pub fn invoke_explained(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> std::result::Result<Vec<u8>, Refusal> {
        self.invoke_bound(
            None,
            operation,
            revision,
            partition,
            secret,
            input,
            deadline_ms,
        )
    }
    /// [`Child::invoke_explained`] under `binding`, the binding the read's
    /// evidence selected: the requirement, revision and output schema are
    /// that binding's (`Bootstrap::bound`), and the request names it.
    #[allow(clippy::too_many_arguments)]
    pub fn invoke_bound(
        &mut self,
        binding: Option<&str>,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> std::result::Result<Vec<u8>, Refusal> {
        self.read(
            binding,
            (operation, revision, partition),
            secret,
            (input, deadline_ms),
            |bytes| connectors_core::read_json(bytes).map_err(|_| Failure::Protocol),
        )
    }
    /// [`Child::invoke_bound`] for a governed read: the result keeps every
    /// admitted JSON number exactly as the child wrote it and a duplicate
    /// member is refused (`connectors_core::json`). Authorization remains the
    /// owner's, exactly as for `invoke`.
    #[allow(clippy::too_many_arguments)]
    pub fn invoke_lossless(
        &mut self,
        binding: Option<&str>,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> std::result::Result<Vec<u8>, Refusal> {
        self.read(
            binding,
            (operation, revision, partition),
            secret,
            (input, deadline_ms),
            |bytes| connectors_core::json::decode(bytes, 64).map_err(|_| Failure::Protocol),
        )
    }
    fn read(
        &mut self,
        binding: Option<&str>,
        (operation, revision, partition): (&str, &str, &str),
        secret: &Secret,
        (input, deadline_ms): (&[u8], u64),
        decode: Decode,
    ) -> std::result::Result<Vec<u8>, Refusal> {
        let mut reason = None;
        let mut delay = None;
        self.dispatch(
            binding,
            (operation, revision, partition),
            secret,
            input,
            (deadline_ms, decode),
            &mut reason,
            &mut delay,
        )
        .map_err(|failure| Refusal {
            failure,
            reason: super::admitted(reason, secret),
            retry_after_seconds: delay,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn dispatch(
        &mut self,
        binding: Option<&str>,
        (operation, revision, partition): (&str, &str, &str),
        secret: &Secret,
        input: &[u8],
        (deadline_ms, decode): (u64, Decode),
        reason: &mut Option<String>,
        delay: &mut Option<u64>,
    ) -> Result<Vec<u8>> {
        // A bounded-read child answers only `invoke_bounded`.
        if self.protocol == PrivateProtocol::V3 {
            return Err(Failure::Unsupported);
        }
        if input.len() > INPUT_LIMIT || !connectors_core::valid_id(partition) {
            return Err(Failure::InvalidInput);
        }
        let bound = self.bootstrap.bound(binding)?;
        let requirement = bound
            .requirements
            .iter()
            .find(|r| r.operation == operation)
            .ok_or(Failure::NotFound)?;
        if requirement.effect != Effect::Read {
            return Err(Failure::Unsupported);
        }
        let descriptor = bound.descriptor()?;
        if descriptor.revision != revision {
            return Err(Failure::StaleDescription);
        }
        channel::depth(input)?;
        let id = uuid::Uuid::new_v4().to_string();
        let frame = self.exchange(
            Request::Invoke {
                request_id: id.clone(),
                operation: operation.into(),
                revision: revision.into(),
                partition: partition.into(),
                deadline_ms,
                binding: binding.map(str::to_owned),
            },
            Some(secret),
            input,
            deadline_ms,
        )?;
        match frame.control {
            Reply::Success { request_id } if request_id == id => {
                let checked = (|| {
                    channel::depth(&frame.document)?;
                    let value = decode(&frame.document)?;
                    let operation = descriptor
                        .operation(operation)
                        .map_err(Failure::from_service)?;
                    connectors_sdk::validate(&operation.output_schema, &value)
                        .map_err(|_| Failure::Protocol)
                })();
                if let Err(error) = checked {
                    self.terminate();
                    return Err(error);
                }
                Ok(frame.document)
            }
            // A delay belongs beside the provider's rate limit only; one beside
            // any other code is a malformed reply.
            Reply::Failed {
                request_id,
                code,
                reason: carried,
                retry_after_seconds,
            } if request_id == id
                && frame.document.is_empty()
                && (retry_after_seconds.is_none() || code == Failure::ProviderRateLimited) =>
            {
                *reason = carried;
                *delay = super::admitted_delay(retry_after_seconds);
                Err(code)
            }
            _ => {
                self.terminate();
                Err(Failure::Protocol)
            }
        }
    }
    /// Explicit read transport only. Caller supplies current admission and the
    /// original budget established before admission; no new interval starts here.
    pub fn invoke_bounded(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        budget: ReadBudget,
    ) -> Result<Vec<u8>> {
        self.invoke_bounded_until_cancelled(
            operation, revision, partition, secret, input, budget, None,
        )
    }
    /// [`Child::invoke_bounded`] that stops waiting for the answer once `cancel`
    /// fails, checked at least every 250 ms: the child is then ended, so its
    /// provider call and credential read end with it
    /// (story:owner-cancels-a-bounded-read).
    #[allow(clippy::too_many_arguments)]
    pub fn invoke_bounded_until_cancelled(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        budget: ReadBudget,
        cancel: Option<&dyn Fn() -> Result<()>>,
    ) -> Result<Vec<u8>> {
        if self.protocol != PrivateProtocol::V3 {
            return Err(Failure::Unsupported);
        }
        if !self.live {
            return Err(Failure::Unavailable);
        }
        let until = budget.until()?;
        if input.len() > budget.input_bytes() || !connectors_core::valid_id(partition) {
            return Err(Failure::InvalidInput);
        }
        let descriptor = self.bootstrap.descriptor()?;
        if descriptor.revision != revision {
            return Err(Failure::StaleDescription);
        }
        let requirement = self
            .bootstrap
            .requirements
            .iter()
            .find(|r| r.operation == operation)
            .ok_or(Failure::NotFound)?;
        if requirement.effect != Effect::Read {
            return Err(Failure::Unsupported);
        }
        let declaration = descriptor
            .operation(operation)
            .map_err(Failure::from_service)?;
        let value = connectors_core::json::decode(input, 64).map_err(|_| Failure::InvalidInput)?;
        connectors_sdk::validate(&declaration.input_schema, &value)
            .map_err(Failure::from_service)?;
        let id = uuid::Uuid::new_v4().to_string();
        let result = channel::write(
            &mut self.channel,
            &bounded::ReadRequest::InvokeBounded {
                request_id: id.clone(),
                operation: operation.into(),
                revision: revision.into(),
                partition: partition.into(),
                budget,
            },
            Some(secret),
            input,
            until,
        )
        .and_then(|_| {
            channel::read_with_cancel::<Reply>(
                &mut self.channel,
                until,
                false,
                budget.result_bytes(),
                cancel,
            )
        })
        .and_then(|frame| match frame.control {
            Reply::Success { request_id } if request_id == id => {
                let value = connectors_core::json::decode(&frame.document, 64)
                    .map_err(|_| Failure::Protocol)?;
                connectors_sdk::validate(&declaration.output_schema, &value)
                    .map_err(|_| Failure::Protocol)?;
                budget.until()?;
                Ok(frame.document)
            }
            Reply::Failed {
                request_id,
                code,
                reason: None,
                retry_after_seconds: None,
            } if request_id == id && frame.document.is_empty() => Err(code),
            _ => Err(Failure::Protocol),
        });
        if matches!(
            result,
            Err(Failure::Timeout
                | Failure::ProviderTimeout
                | Failure::Interrupted
                | Failure::Protocol
                | Failure::Unavailable)
        ) {
            self.terminate();
        }
        result
    }
    /// Prepare on this exact child without writing. The returned borrow occupies
    /// it through commit/cancel and keeps the original monotonic deadline. The
    /// host coordinator must admit current policy and credentials before entry.
    pub fn prepare_write(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> Result<PreparedInvocation<'_>> {
        self.prepare_write_until(
            operation,
            revision,
            partition,
            secret,
            input,
            (deadline_ms, writes::budget(deadline_ms)?),
        )
    }

    /// Compose a narrower original owner budget with the private wire deadline.
    /// Clock changes or time consumed before this call cannot extend that budget.
    pub fn prepare_write_until(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline: (u64, Instant),
    ) -> Result<PreparedInvocation<'_>> {
        let (deadline_ms, original_until) = deadline;
        if self.protocol != PrivateProtocol::V2 {
            return Err(Failure::Unsupported);
        }
        if !self.live {
            return Err(Failure::Unavailable);
        }
        let until = writes::budget(deadline_ms)?.min(original_until);
        if Instant::now() >= until {
            return Err(Failure::Timeout);
        }
        if input.len() > INPUT_LIMIT || secret.0.is_empty() || secret.0.len() > SECRET_LIMIT {
            return Err(Failure::InvalidInput);
        }
        let descriptor = self.bootstrap.descriptor()?;
        let declaration = writes::write_declaration(
            &self.bootstrap,
            &descriptor,
            operation,
            revision,
            partition,
        )?;
        channel::depth(input)?;
        let value = connectors_core::read_json(input).map_err(|_| Failure::InvalidInput)?;
        connectors_sdk::validate_write_value(&declaration.input_schema, &value)
            .map_err(Failure::from_service)?;
        let output_schema = declaration.output_schema.clone();
        let id = uuid::Uuid::new_v4().to_string();
        let request = writes::WriteRequest::Prepare {
            id: id.clone(),
            operation: operation.into(),
            revision: revision.into(),
            partition: partition.into(),
            deadline_ms,
        };
        let response = channel::write(&mut self.channel, &request, Some(secret), input, until)
            .and_then(|_| channel::read::<writes::WriteReply>(&mut self.channel, until, false, 0));
        match response {
            Ok(channel::Frame {
                control:
                    writes::WriteReply::PreparedWrite {
                        id: returned,
                        preparation_id,
                    },
                ..
            }) if returned == id && writes::canonical_id(&preparation_id) => {
                Ok(PreparedInvocation {
                    child: self,
                    id,
                    preparation_id,
                    until,
                    deadline_ms,
                    output_schema,
                    finished: false,
                })
            }
            Ok(channel::Frame {
                control: writes::WriteReply::Failed { request_id, code },
                ..
            }) if request_id == id => {
                if matches!(code, Failure::Timeout | Failure::Interrupted) {
                    self.terminate();
                }
                Err(code)
            }
            response => {
                self.terminate();
                Err(response.err().unwrap_or(Failure::Protocol))
            }
        }
    }
    fn exchange(
        &mut self,
        request: Request,
        secret: Option<&Secret>,
        document: &[u8],
        deadline_ms: u64,
    ) -> Result<channel::Frame<Reply>> {
        if !self.live {
            return Err(Failure::Unavailable);
        }
        let duration = deadline_ms
            .checked_sub(connectors_sdk::now_ms())
            .filter(|ms| (1..=120_000).contains(ms))
            .ok_or(Failure::Timeout)?;
        let until = Instant::now() + Duration::from_millis(duration);
        let result = channel::write(&mut self.channel, &request, secret, document, until)
            .and_then(|_| channel::read(&mut self.channel, until, false, RESULT_LIMIT));
        if result.is_err()
            || matches!(
                &result,
                Ok(channel::Frame {
                    control: Reply::Failed {
                        code: Failure::Timeout | Failure::Interrupted,
                        ..
                    },
                    ..
                })
            )
        {
            self.terminate();
        }
        result
    }
    pub fn stop(&mut self, expected: &str) -> Result<()> {
        if expected != self.incarnation {
            return Err(Failure::IncarnationMismatch);
        }
        if !self.live {
            return Ok(());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let until = Instant::now() + Duration::from_secs(5);
        let response = channel::write(
            &mut self.channel,
            &Request::Stop {
                request_id: id.clone(),
            },
            None,
            &[],
            until,
        )
        .and_then(|_| channel::read::<Reply>(&mut self.channel, until, false, 0));
        let acknowledged = matches!(response,Ok(channel::Frame {control:Reply::Stopped {request_id},..}) if request_id==id);
        self.terminate();
        if acknowledged {
            Ok(())
        } else {
            Err(Failure::Unavailable)
        }
    }
    /// Kills and reaps under the child's reap lock (`child_reap_lock`).
    #[cfg(target_os = "macos")]
    fn terminate(&mut self) {
        if self.live {
            let _ = self.process.kill();
            let _ = self.process.wait();
            self.live = false;
        }
    }
    #[cfg(target_os = "linux")]
    fn terminate(&mut self) {
        if self.live {
            // SAFETY: pidfd is an owned handle to this child, never a numeric PID
            // restored from metadata. No siginfo is supplied.
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pidfd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
            let _ = self.process.wait();
            self.live = false;
        }
    }
    /// Terminates through the process handle and waits on it
    /// (`exact_process_signal`): there is no signal to send first.
    #[cfg(windows)]
    fn terminate(&mut self) {
        if self.live {
            let _ = self.process.kill();
            let _ = self.process.wait();
            self.live = false;
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Starts an adapter on Windows (private-adapter.md, "Windows"): from the
/// captured copy's path, whose deny-write handle is held until the child
/// exists (`deny_write_copy`), inside the owner's kill-on-close job, which
/// permits no breakaway (`parent_death_kill`), with the copy's directory as
/// its current directory. The owner's pipe pair was checked to be connected to
/// this process alone; the adapter inherits its client end as the only handle
/// in its handle list, its value given as `--connectors-private-fd`, with no
/// standard handles and an environment of `SystemRoot` alone, which Windows
/// needs to start its network stack and which carries no credential.
#[cfg(windows)]
fn spawn_windows(
    config: &super::super::config::Adapter,
    executable: artifact::Image,
) -> Result<(artifact::Process, artifact::Launch, channel::Stream)> {
    use std::os::windows::io::AsRawHandle;
    // The copy's custody, from before any fallible step: every early return
    // removes it.
    let mut launch = artifact::Launch::from(executable);
    let (channel, client) = channel::windows::pair()?;
    let client = artifact::inheritable(client.as_raw_handle(), None)?;
    let value = client.as_raw_handle() as usize;
    let mut arguments: Vec<std::ffi::OsString> = config
        .executable
        .args
        .iter()
        .map(std::ffi::OsString::from)
        .collect();
    arguments.push("--connectors-private-fd".into());
    arguments.push(value.to_string().into());
    let environment: Vec<_> = std::env::var_os("SystemRoot")
        .map(|root| ("SystemRoot".into(), root))
        .into_iter()
        .collect();
    let process = artifact::create(
        &artifact::Creation {
            application: launch.path(),
            arguments: &arguments,
            environment: &environment,
            standard: [std::ptr::null_mut(); 3],
            inherited: &[client.as_raw_handle()],
            flags: windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
        },
        artifact::Job::adapters()?,
    )
    .map_err(|_| Failure::Unavailable)?;
    drop(client);
    // Held until the child was created from the copy; released, not removed.
    launch.created();
    Ok((process, launch, channel))
}

/// Live process-only preparation. Dropping it kills/reaps the exact child so
/// unacknowledged cancellation can never leave a reusable pending write.
/// ```compile_fail
/// use connectors_host::local::runtime::PreparedInvocation;
/// fn reuse(prepared: PreparedInvocation<'_>) {
///     prepared.commit();
///     prepared.commit();
/// }
/// ```
#[must_use]
pub struct PreparedInvocation<'a> {
    child: &'a mut Child,
    id: String,
    preparation_id: String,
    until: Instant,
    deadline_ms: u64,
    output_schema: serde_json::Value,
    finished: bool,
}
impl PreparedInvocation<'_> {
    pub fn remaining(&self) -> Result<()> {
        writes::remaining(self.until, self.deadline_ms)
    }
    pub fn cancel(mut self) -> Result<()> {
        self.remaining()?;
        let response = channel::write(
            &mut self.child.channel,
            &writes::WriteRequest::Cancel {
                id: self.id.clone(),
                preparation_id: self.preparation_id.clone(),
            },
            None,
            &[],
            self.until,
        )
        .and_then(|_| {
            channel::read::<writes::WriteReply>(&mut self.child.channel, self.until, false, 0)
        });
        match response {
            Ok(channel::Frame {
                control: writes::WriteReply::CancelledWrite { id, preparation_id },
                ..
            }) if id == self.id && preparation_id == self.preparation_id => {
                self.finished = true;
                Ok(())
            }
            response => Err(response.err().unwrap_or(Failure::Protocol)),
        }
    }
    /// Transport commit only: the trusted coordinator must already hold definite
    /// approval, audit, attempt, credential-dispatch and gate acknowledgements.
    /// Every failure from here is possible-write uncertainty, including expiry
    /// before the send. The caller must retain/quarantine its attempt, not retry.
    pub fn commit(self) -> WriteResult {
        self.finish(false).0
    }
    /// [`PreparedInvocation::commit`] for a write whose answer carries a
    /// credential it created (`connectors.mutations.CredentialCustody`): the
    /// bytes the child sent in the protected slot beside an applied success.
    /// An applied success without them, or bytes beside anything else, is a
    /// protocol fault that keeps the reported effect, drops the bytes and
    /// terminates the child; so are bytes on a commit that expects none.
    pub fn commit_holding(self) -> (WriteResult, Option<Secret>) {
        self.finish(true)
    }
    fn finish(mut self, holding: bool) -> (WriteResult, Option<Secret>) {
        let unknown = |code| (WriteResult::unknown(code), None);
        if let Err(code) = self.remaining() {
            return unknown(code);
        }
        let response = channel::write(
            &mut self.child.channel,
            &writes::WriteRequest::Commit {
                id: self.id.clone(),
                preparation_id: self.preparation_id.clone(),
            },
            None,
            &[],
            self.until,
        )
        .and_then(|_| {
            // The protected slot is read whether or not bytes are expected, so a
            // child that sends them anyway keeps the effect it reports.
            channel::read::<writes::WriteReply>(
                &mut self.child.channel,
                self.until,
                true,
                RESULT_LIMIT,
            )
        });
        let frame = match response {
            Ok(frame) => frame,
            // The child may exhaust the captured wall deadline and close before
            // our socket's monotonic timer fires. Recheck the original bounds;
            // an early transport loss stays unavailable, and neither grants retry.
            Err(Failure::Unavailable) => {
                return unknown(self.remaining().err().unwrap_or(Failure::Unavailable));
            }
            Err(code) => return unknown(code),
        };
        let writes::WriteReply::WriteResult { id, effect } = frame.control else {
            return unknown(Failure::Protocol);
        };
        if id != self.id || channel::depth(&frame.document).is_err() {
            return unknown(Failure::Protocol);
        }
        let document: writes::ResultDocument = match connectors_core::read_json(&frame.document) {
            Ok(value) => value,
            Err(_) => return unknown(Failure::Protocol),
        };
        // Retain definite effect knowledge; discard an invalid safe payload or
        // misplaced bytes and terminate the peer that sent them.
        let faulted = || {
            (
                WriteResult {
                    effect,
                    result: Err(Failure::Protocol),
                },
                None,
            )
        };
        let secret = Some(frame.secret).filter(|secret| !secret.0.is_empty());
        let result = match document {
            writes::ResultDocument::Failure { code } if secret.is_none() => Err(code),
            writes::ResultDocument::Success { value } if effect == WriteEffect::Applied => {
                if connectors_sdk::validate_write_value(&self.output_schema, &value).is_err()
                    || secret.is_some() != holding
                {
                    return faulted();
                }
                Ok(value)
            }
            writes::ResultDocument::Failure { .. } => return faulted(),
            _ => return unknown(Failure::Protocol),
        };
        self.finished = true;
        (WriteResult { effect, result }, secret)
    }
}
impl Drop for PreparedInvocation<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.child.terminate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::config::{Adapter, Executable, Restart, Startup};

    pub(super) fn bootstrap() -> Bootstrap {
        Bootstrap { bindings: None,
credential_revision: None,
executable: None,
            instance: "fixture".into(), adapter: "fixture".into(), protocol: "v1alpha1".into(),
            configuration_revision: "fixture-config".into(), provider_authority: "fixture-authority".into(),
            descriptor: serde_json::json!({
                "version":"v1alpha1", "instance":"fixture", "adapter":"fixture", "revision":"fixture-descriptor",
                "configuration_schema":{"type":"object"},
                "operations":[{"id":"read", "description":"Fictional read", "contract":"fixture/1", "profile":"fixture",
                    "input_schema":{"type":"object"}, "output_schema":{"type":"object","required":["value"],"properties":{"value":{"type":"boolean"}},"additionalProperties":false}}]
            }).to_string(),
            profiles: vec![Profile {
                id:"fixture".into(),revision:"fixture-profile".into(),purpose:registry::Purpose::DelegatedUser,
                subject:registry::Subject::User,scheme:"http_bearer".into(),capability:"http-bearer".into(),
                minimum_scopes:BTreeSet::new(),evidence_lifetime_ms:60_000,
                fields:vec![EntryField{name:"token".into(),label:"Fictional credential".into(),max_bytes:100}],
                acquisition:None,
            }],
            requirements:vec![Requirement{operation:"read".into(),custody:false,profile:"fixture".into(),scopes:BTreeSet::new(),effect:Effect::Read}],
        }
    }

    // The ordinary test invocation does nothing. Child::spawn re-enters just
    // this test through libtest's filter syntax, with the real private fd and
    // process-ownership path. No shell, helper executable or provider is needed.
    #[test]
    fn protocol_fixture() {
        let args: Vec<_> = std::env::args().collect();
        if !args.iter().any(|a| a == "--connectors-private-fd") {
            return;
        }
        let mode = args
            .iter()
            .find(|a| a.starts_with("fixture-mode-"))
            .unwrap();
        // Written to the descriptor itself: libtest captures `eprintln!`.
        let stderr = |line: &[u8]| {
            let _ = std::io::Write::write_all(&mut std::io::stderr(), line);
        };
        // A program that refuses to start: its own error line, exit 3.
        if mode == "fixture-mode-stderr-exit" {
            stderr(b"fixture: refused to start\n");
            std::process::exit(3);
        }
        // A program serving through the host library that speaks only
        // `connectors-private/1`: asked for another, it refuses by the
        // library's own reply (story:adapter-private-protocol-refusal-named).
        #[cfg(unix)]
        if mode == "fixture-mode-refuse-private-protocol" {
            struct V1Only;
            #[async_trait::async_trait]
            impl crate::local::runtime::Adapter for V1Only {
                fn bootstrap(&self) -> Bootstrap {
                    bootstrap()
                }
                async fn validate(&self, _: &str, _: Secret) -> Result<Baseline> {
                    Err(Failure::Unsupported)
                }
                async fn invoke(
                    &self,
                    _: &str,
                    _: &str,
                    _: Secret,
                    _: serde_json::Value,
                ) -> Result<serde_json::Value> {
                    Err(Failure::Unsupported)
                }
            }
            let _ = crate::local::runtime::serve(3, V1Only);
            std::process::exit(1);
        }
        // Four times the start log, more than a pipe holds, before anything
        // else: exits 3, or with `-start` goes on to its handshake.
        if mode.starts_with("fixture-mode-stderr-flood") {
            stderr(&vec![b'x'; 4 * START_LOG_LIMIT]);
            if mode == "fixture-mode-stderr-flood-exit" {
                std::process::exit(3);
            }
        }
        #[cfg(unix)]
        // SAFETY: only the spawned fixture path owns inherited descriptor three.
        let mut stream = unsafe { UnixStream::from_raw_fd(3) };
        #[cfg(windows)]
        let mut stream = private_channel();
        channel::peer(&stream).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        let hello = channel::read::<Request>(&mut stream, until, false, 0).unwrap();
        let Request::Hello {
            version,
            nonce,
            child_incarnation,
        } = hello.control
        else {
            panic!("expected hello")
        };
        let mut selected = bootstrap();
        if matches!(
            mode.as_str(),
            "fixture-mode-lossless" | "fixture-mode-duplicate"
        ) {
            let mut descriptor = selected.descriptor().unwrap();
            descriptor.operations[0].output_schema = serde_json::json!({"type":"object"});
            selected.descriptor = serde_json::to_string(&descriptor).unwrap();
        }
        if mode == "fixture-mode-stderr-after-ready" {
            stderr(b"fixture: before ready\n");
        }
        channel::write(
            &mut stream,
            &Reply::Ready {
                version,
                nonce,
                child_incarnation,
                bootstrap: selected,
            },
            None,
            &[],
            until,
        )
        .unwrap();
        let request = channel::read::<Request>(&mut stream, until, true, INPUT_LIMIT).unwrap();
        // A line written once the program holds a credential; it must never
        // reach the start log.
        if mode == "fixture-mode-stderr-after-ready" {
            stderr(b"fixture: after ready\n");
            let Request::Validate { request_id, .. } = request.control else {
                panic!("expected validation")
            };
            channel::write(
                &mut stream,
                &Reply::Failed {
                    request_id,
                    code: Failure::InvalidCredential,
                    reason: None,
                    retry_after_seconds: None,
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        if mode == "fixture-mode-validate-retry-after" {
            let Request::Validate { request_id, .. } = request.control else {
                panic!("expected validation")
            };
            channel::write(
                &mut stream,
                &Reply::Failed {
                    request_id,
                    code: Failure::ProviderRateLimited,
                    reason: None,
                    retry_after_seconds: Some(7),
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        // A validation refused under every binding, naming each answer, and
        // one answered under a binding the bootstrap does not declare.
        if mode == "fixture-mode-validate-scope-reason" {
            let Request::Validate { request_id, .. } = request.control else {
                panic!("expected validation")
            };
            channel::write(
                &mut stream,
                &Reply::Failed {
                    request_id,
                    code: Failure::InsufficientScope,
                    reason: Some(
                        "binding v2 answered 401 (scope does not match); binding v1 answered 403 (fixture forbidden)"
                            .into(),
                    ),
                    retry_after_seconds: None,
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        if mode == "fixture-mode-validate-undeclared-binding" {
            let Request::Validate { request_id, .. } = request.control else {
                panic!("expected validation")
            };
            let now = connectors_sdk::now_ms();
            channel::write(
                &mut stream,
                &Reply::Validated {
                    request_id,
                    baseline: Baseline {
                        identity: registry::ExternalIdentity {
                            kind: "account".into(),
                            subject: "fixture-subject".into(),
                        },
                        granted_scopes: None,
                        credential_expires_at_ms: None,
                        collected_at_ms: now,
                        valid_until_ms: now + 60_000,
                        binding: Some("v1".into()),
                    },
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        if mode == "fixture-mode-validate-reason" {
            let Request::Validate { request_id, .. } = request.control else {
                panic!("expected validation")
            };
            channel::write(
                &mut stream,
                &Reply::Failed {
                    request_id,
                    code: Failure::InvalidCredential,
                    reason: Some("Unauthorized; scope does not match".into()),
                    retry_after_seconds: None,
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        let Request::Invoke { request_id, .. } = request.control else {
            panic!("expected invocation")
        };
        // A failed invoke naming a delay: beside a rate limit it is the
        // provider's, beside any other code it is a malformed reply.
        let delayed = match mode.as_str() {
            "fixture-mode-retry-after" => Some(Failure::ProviderRateLimited),
            "fixture-mode-retry-after-unauthorized" => Some(Failure::InvalidCredential),
            _ => None,
        };
        if let Some(code) = delayed {
            channel::write(
                &mut stream,
                &Reply::Failed {
                    request_id,
                    code,
                    reason: None,
                    retry_after_seconds: Some(7),
                },
                None,
                &[],
                until,
            )
            .unwrap();
            std::thread::sleep(Duration::from_secs(30));
            return;
        }
        let (id, body) = match mode.as_str() {
            "fixture-mode-json" => (request_id, b"{\"value\":}".as_slice()),
            "fixture-mode-schema" => (request_id, br#"{"value":"wrong-type"}"#.as_slice()),
            "fixture-mode-identity" => ("another-request".into(), br#"{"value":true}"#.as_slice()),
            "fixture-mode-lossless" => {
                assert_eq!(request.document, br#"{"n":1844674407370955161701}"#);
                (
                    request_id,
                    br#"{"n":1844674407370955161701,"huge":1e400,"$serde_json::private::Number":"literal"}"#
                        .as_slice(),
                )
            }
            "fixture-mode-duplicate" => (request_id, br#"{"n":1,"n":2}"#.as_slice()),
            _ => panic!("unknown fixture mode"),
        };
        channel::write(
            &mut stream,
            &Reply::Success { request_id: id },
            None,
            body,
            until,
        )
        .unwrap();
        // The parent must kill and reap this exact child after the bad reply.
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn governed_child_transport_preserves_numbers_and_refuses_duplicate_keys() {
        for mode in ["lossless", "duplicate"] {
            let mut child = fixture_child(&format!("fixture-mode-{mode}"));
            let result = child
                .invoke_lossless(
                    None,
                    "read",
                    "fixture-descriptor",
                    "partition",
                    &Secret(b"fictional".to_vec()),
                    br#"{"n":1844674407370955161701}"#,
                    connectors_sdk::now_ms() + 2000,
                )
                .map_err(|refusal| refusal.failure);
            if mode == "lossless" {
                assert_eq!(
                    result.unwrap(),
                    br#"{"n":1844674407370955161701,"huge":1e400,"$serde_json::private::Number":"literal"}"#
                );
                assert!(child.live);
            } else {
                assert_eq!(result, Err(Failure::Protocol));
                assert!(!child.live);
                assert!(child.process.try_wait().unwrap().is_some());
            }
        }
    }

    #[test]
    fn malformed_replies_terminate_and_reap_only_the_owned_child() {
        let (path, hash) = crate::local::test_executable::path_and_sha256();
        for mode in ["json", "schema", "identity"] {
            let config = Adapter {
                credential_revision: None,
                private_protocol: None,
                permissions: Default::default(),
                instance_id: "fixture".into(),
                adapter_id: "fixture".into(),
                configuration_revision: "fixture-config".into(),
                protocol: "v1alpha1".into(),
                startup: Startup::OnDemand,
                restart: Restart::Never,
                executable: Executable {
                    path: path.clone(),
                    sha256: hash.clone(),
                    args: vec![
                        "--exact".into(),
                        "local::runtime::process::tests::protocol_fixture".into(),
                        "--".into(),
                        format!("fixture-mode-{mode}"),
                    ],
                },
            };
            let mut child = Child::spawn(&config).unwrap();
            assert!(child.process.try_wait().unwrap().is_none());
            assert_eq!(
                child.invoke(
                    "read",
                    "fixture-descriptor",
                    "partition",
                    &Secret(b"fictional".to_vec()),
                    b"{}",
                    connectors_sdk::now_ms() + 2000
                ),
                Err(Failure::Protocol)
            );
            assert!(!child.live);
            assert!(child.process.try_wait().unwrap().is_some());
        }
    }

    /// story:service-failure-carries-upstream-reason: only a failed `invoke`
    /// may carry a reason. A failed validation that carries one is refused
    /// like any malformed reply, and the child is terminated.
    #[test]
    fn a_reason_on_a_failed_validation_is_a_protocol_violation() {
        let (path, sha256) = crate::local::test_executable::path_and_sha256();
        let config = Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture".into(),
            adapter_id: "fixture".into(),
            configuration_revision: "fixture-config".into(),
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256,
                path,
                args: vec![
                    "--exact".into(),
                    "local::runtime::process::tests::protocol_fixture".into(),
                    "--".into(),
                    "fixture-mode-validate-reason".into(),
                ],
            },
        };
        let mut child = Child::spawn(&config).unwrap();
        assert_eq!(
            child
                .validate(
                    "fixture",
                    &Secret(b"fictional".to_vec()),
                    connectors_sdk::now_ms() + 2000
                )
                .err(),
            Some(Failure::Protocol)
        );
        assert!(!child.live);
        assert!(child.process.try_wait().unwrap().is_some());
    }

    /// story:confluence-token-kind-binding: a validation refused under every
    /// alternative binding carries the reason naming each answer beside
    /// `insufficient_scope`, and the child keeps running; a baseline naming a
    /// binding the bootstrap does not declare is a protocol violation, and the
    /// child is terminated.
    #[test]
    fn a_scope_refusal_keeps_its_reason_and_an_undeclared_binding_is_refused() {
        let mut child = fixture_child("fixture-mode-validate-scope-reason");
        let refusal = child
            .validate_explained(
                "fixture",
                &Secret(b"fictional".to_vec()),
                connectors_sdk::now_ms() + 2000,
            )
            .err()
            .unwrap();
        assert_eq!(refusal.failure, Failure::InsufficientScope);
        assert_eq!(
            refusal.reason.as_deref(),
            Some(
                "binding v2 answered 401 (scope does not match); binding v1 answered 403 (fixture forbidden)"
            )
        );
        assert!(child.live);

        let mut child = fixture_child("fixture-mode-validate-undeclared-binding");
        assert_eq!(
            child
                .validate(
                    "fixture",
                    &Secret(b"fictional".to_vec()),
                    connectors_sdk::now_ms() + 2000
                )
                .err(),
            Some(Failure::Protocol)
        );
        assert!(!child.live);
        assert!(child.process.try_wait().unwrap().is_some());
    }

    /// story:confluence-token-kind-binding: a bootstrap's bindings are read
    /// only descriptions of the same instance under ids and revisions of
    /// their own, a description names its binding by its revision, and a
    /// bootstrap without bindings answers as before.
    #[test]
    fn bindings_are_distinct_read_only_descriptions_named_by_revision() {
        let first: serde_json::Value = serde_json::from_str(&bootstrap().descriptor).unwrap();
        let mut other = first.clone();
        other["revision"] = serde_json::json!("fixture-descriptor-v1");
        let alternative = |id: &str, descriptor: &serde_json::Value, effect| AlternativeBinding {
            id: id.into(),
            descriptor: descriptor.to_string(),
            requirements: vec![Requirement {
                effect,
                ..bootstrap().requirements[0].clone()
            }],
        };
        let with = |alternatives| Bootstrap {
            bindings: Some(Bindings {
                id: "v2".into(),
                alternatives,
            }),
            ..bootstrap()
        };
        let bound = with(vec![alternative("v1", &other, Effect::Read)]);
        bound.validate().unwrap();
        let described = |revision| bound.described_binding(revision);
        assert_eq!(
            described("fixture-descriptor").unwrap().as_deref(),
            Some("v2")
        );
        assert_eq!(
            described("fixture-descriptor-v1").unwrap().as_deref(),
            Some("v1")
        );
        assert_eq!(
            described("fixture-other").err(),
            Some(Failure::StaleDescription)
        );
        let view = bound.bound(Some("v1")).unwrap();
        assert_eq!(view.descriptor().unwrap().revision, "fixture-descriptor-v1");
        assert!(view.bindings.is_none());
        assert_eq!(
            bound.bound(Some("v3")).err(),
            Some(Failure::StaleDescription)
        );
        assert_eq!(bound.effective_binding(None).as_deref(), Some("v2"));
        assert_eq!(bound.effective_binding(Some("v1")).as_deref(), Some("v1"));
        assert!(bound.declares_binding(Some("v1")));
        assert!(!bound.declares_binding(None) && !bound.declares_binding(Some("v3")));

        let plain = bootstrap();
        assert_eq!(plain.described_binding("fixture-descriptor").unwrap(), None);
        assert!(plain.declares_binding(None) && !plain.declares_binding(Some("v2")));
        assert_eq!(plain.effective_binding(None), None);
        assert_eq!(
            plain.bound(Some("v2")).err(),
            Some(Failure::StaleDescription)
        );

        for refused in [
            with(vec![]),
            with(vec![alternative("v2", &other, Effect::Read)]),
            with(vec![alternative("v1", &first, Effect::Read)]),
            with(vec![alternative("v1", &other, Effect::Write)]),
            with(vec![alternative("not an id", &other, Effect::Read)]),
        ] {
            assert_eq!(refused.validate().err(), Some(Failure::Protocol));
        }
    }

    fn fixture_config(mode: &str) -> Adapter {
        let (path, sha256) = crate::local::test_executable::path_and_sha256();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture".into(),
            adapter_id: "fixture".into(),
            configuration_revision: "fixture-config".into(),
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256,
                path,
                args: vec![
                    "--exact".into(),
                    "local::runtime::process::tests::protocol_fixture".into(),
                    "--".into(),
                    mode.into(),
                ],
            },
        }
    }

    fn fixture_child(mode: &str) -> Child {
        Child::spawn(&fixture_config(mode)).unwrap()
    }

    /// A state directory as the host keeps one: owner-only, which the start
    /// log's directory walk requires (`filesystem::directory`).
    fn private_state() -> tempfile::TempDir {
        let state = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(
            state.path(),
            std::os::unix::fs::PermissionsExt::from_mode(0o700),
        )
        .unwrap();
        state
    }

    /// A start of `config` with the state directory `state`, which fails.
    fn failed_start(config: &Adapter, state: &Path) -> StartError {
        let Err(error) = Child::start(
            config,
            Instant::now() + Duration::from_secs(10),
            Some(state),
        ) else {
            panic!("the start succeeded")
        };
        error
    }

    /// story:program-start-failure-reason: a program that prints a line to
    /// standard error and exits 3 before its handshake fails its start as
    /// `exited_before_handshake` with exit code 3, under the unchanged
    /// `unavailable`, and its line is in the instance's owner-only start log
    /// under the state directory. Unix only: on Windows the program starts
    /// with no standard handles and no start log is kept.
    #[cfg(unix)]
    #[test]
    fn a_program_that_exits_before_its_handshake_names_its_exit_and_keeps_its_line() {
        use std::os::unix::fs::PermissionsExt;
        let state = private_state();
        let error = failed_start(&fixture_config("fixture-mode-stderr-exit"), state.path());
        assert_eq!(
            error.step,
            StartFailure::ExitedBeforeHandshake {
                code: Some(3),
                signal: None
            }
        );
        assert_eq!(error.step.as_str(), "exited_before_handshake");
        assert_eq!(error.failure, Failure::Unavailable);
        let log = error.log.expect("a start log");
        assert_eq!(log, state.path().join(START_LOGS).join("fixture.log"));
        let kept = std::fs::read_to_string(&log).unwrap();
        assert_eq!(kept, "fixture: refused to start\n");
        let metadata = std::fs::symlink_metadata(&log).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        // The next start replaces the log.
        let again = failed_start(&fixture_config("fixture-mode-stderr-exit"), state.path());
        assert_eq!(again.log.as_ref(), Some(&log));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), kept);
    }

    /// story:adapter-private-protocol-refusal-named: a program that speaks
    /// only `connectors-private/1`, asked for `connectors-private/2`, fails its
    /// start as `unsupported_private_protocol`, not `unavailable`, and names
    /// the protocols it speaks.
    #[cfg(unix)]
    #[test]
    fn a_program_refusing_the_selected_private_protocol_is_named() {
        let state = private_state();
        let mut config = fixture_config("fixture-mode-refuse-private-protocol");
        config.private_protocol = Some(PrivateProtocol::V2);
        let error = failed_start(&config, state.path());
        assert_eq!(error.step.as_str(), "unsupported_private_protocol");
        assert_eq!(
            serde_json::to_value(error.failure).unwrap(),
            "unsupported_private_protocol"
        );
        assert_eq!(
            error.step.supported_private_protocols(),
            [PrivateProtocol::V1]
        );
    }

    /// story:program-start-failure-reason: an executable whose content is
    /// not the pinned one fails as `digest`, and one whose path is not
    /// admitted as `admission`, each under its unchanged code and with no
    /// program started: the start log they replace is empty.
    #[test]
    fn a_digest_mismatch_names_digest_and_a_refused_path_names_admission() {
        let state = private_state();
        let mut config = fixture_config("fixture-mode-json");
        config.executable.sha256 = "0".repeat(64);
        let error = failed_start(&config, state.path());
        assert_eq!(error.step, StartFailure::Digest);
        assert_eq!(error.step.as_str(), "digest");
        assert_eq!(error.failure, Failure::InvalidConfiguration);
        #[cfg(unix)]
        assert_eq!(std::fs::read(error.log.expect("a start log")).unwrap(), b"");

        let mut config = fixture_config("fixture-mode-json");
        config.executable.path = state.path().join("not-installed");
        let error = failed_start(&config, state.path());
        assert_eq!(error.step, StartFailure::Admission);
        assert_eq!(error.failure, Failure::InvalidConfiguration);
    }

    /// story:program-start-failure-reason: after a successful start the start
    /// log holds what the program wrote before its handshake ended and
    /// nothing it wrote once it could hold a credential: the owner's end of
    /// the pipe is closed when the start returns.
    #[cfg(unix)]
    #[test]
    fn a_started_program_keeps_nothing_it_wrote_after_its_handshake() {
        let state = private_state();
        let Ok(mut child) = Child::start(
            &fixture_config("fixture-mode-stderr-after-ready"),
            Instant::now() + Duration::from_secs(10),
            Some(state.path()),
        ) else {
            panic!("the program did not start")
        };
        let log = state.path().join(START_LOGS).join("fixture.log");
        assert_eq!(
            std::fs::read_to_string(&log).unwrap(),
            "fixture: before ready\n"
        );
        // The fixture writes its line on receiving the validation, then answers.
        assert_eq!(
            child
                .validate(
                    "fixture",
                    &Secret(b"fictional".to_vec()),
                    connectors_sdk::now_ms() + 2000
                )
                .err(),
            Some(Failure::InvalidCredential)
        );
        assert!(child.running().unwrap());
        assert_eq!(
            std::fs::read_to_string(&log).unwrap(),
            "fixture: before ready\n"
        );
    }

    /// story:program-start-failure-reason: a program that writes more than a
    /// pipe holds before its handshake is not held up by it. Exiting, it
    /// fails as `exited_before_handshake`, not `handshake_deadline`; going on,
    /// it starts. Either way its start log keeps the first
    /// [`START_LOG_LIMIT`] bytes.
    #[cfg(unix)]
    #[test]
    fn a_program_that_floods_standard_error_is_not_held_up_and_its_log_is_bounded() {
        let state = private_state();
        let begun = Instant::now();
        let error = failed_start(
            &fixture_config("fixture-mode-stderr-flood-exit"),
            state.path(),
        );
        assert_eq!(
            error.step,
            StartFailure::ExitedBeforeHandshake {
                code: Some(3),
                signal: None
            }
        );
        assert!(begun.elapsed() < Duration::from_secs(8));
        let log = error.log.expect("a start log");
        assert_eq!(std::fs::read(&log).unwrap(), vec![b'x'; START_LOG_LIMIT]);

        let Ok(child) = Child::start(
            &fixture_config("fixture-mode-stderr-flood-start"),
            Instant::now() + Duration::from_secs(10),
            Some(state.path()),
        ) else {
            panic!("the program did not start")
        };
        drop(child);
        assert_eq!(std::fs::read(&log).unwrap(), vec![b'x'; START_LOG_LIMIT]);
    }

    /// Start log names are distinct for distinct ids and never a path.
    #[test]
    fn start_log_names_are_escaped_and_bounded() {
        assert_eq!(start_log_name("forge-local_1"), "forge-local_1.log");
        assert_eq!(start_log_name("a.b"), "a%2Eb.log");
        assert_eq!(start_log_name("a/.."), "a%2F%2E%2E.log");
        assert_ne!(start_log_name("a.b"), start_log_name("a_b"));
        let long = start_log_name(&"x".repeat(201));
        assert!(
            long.starts_with("sha256-") && long.len() == 7 + 64 + 4,
            "{long}"
        );
        assert!(start_log_name("").starts_with("sha256-"));
    }

    /// story:catalog-honours-retry-after: a failed `invoke` answered as the
    /// provider's rate limit carries the delay it named to the host; a delay
    /// beside any other code, or on a failed validation, is a malformed reply,
    /// refused as a protocol failure, and the child is terminated.
    #[test]
    fn a_named_delay_crosses_only_beside_a_rate_limited_invoke() {
        let invoke = |child: &mut Child| {
            child.invoke_explained(
                "read",
                "fixture-descriptor",
                "partition",
                &Secret(b"fictional".to_vec()),
                b"{}",
                connectors_sdk::now_ms() + 2000,
            )
        };
        let mut limited = fixture_child("fixture-mode-retry-after");
        let refusal = invoke(&mut limited).unwrap_err();
        assert_eq!(refusal.failure, Failure::ProviderRateLimited);
        assert_eq!(refusal.retry_after_seconds, Some(7));
        assert!(limited.live);
        limited.terminate();

        let mut misplaced = fixture_child("fixture-mode-retry-after-unauthorized");
        let refusal = invoke(&mut misplaced).unwrap_err();
        assert_eq!(refusal.failure, Failure::Protocol);
        assert_eq!(refusal.retry_after_seconds, None);
        assert!(!misplaced.live);
        assert!(misplaced.process.try_wait().unwrap().is_some());

        let mut validation = fixture_child("fixture-mode-validate-retry-after");
        assert_eq!(
            validation
                .validate(
                    "fixture",
                    &Secret(b"fictional".to_vec()),
                    connectors_sdk::now_ms() + 2000
                )
                .err(),
            Some(Failure::Protocol)
        );
        assert!(!validation.live);
        assert!(validation.process.try_wait().unwrap().is_some());
    }

    /// `parent_death_kill`: `adapter_channel_close`, the form macOS gives. The
    /// owner holds the only end of the adapter's channel, so closing it, as
    /// the owner's death does, ends the adapter with no signal sent.
    #[test]
    fn an_adapter_exits_when_the_owner_end_of_its_channel_closes() {
        let mut child = fixture_child("fixture-mode-json");
        assert!(child.process.try_wait().unwrap().is_none());
        #[cfg(unix)]
        drop(std::mem::replace(
            &mut child.channel,
            UnixStream::pair().unwrap().0,
        ));
        #[cfg(windows)]
        drop(std::mem::replace(
            &mut child.channel,
            channel::windows::pair().unwrap().0,
        ));
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.process.try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "the adapter outlived its channel"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// `exact_process_signal`: `child_reap_lock`, through the owner's stop
    /// handle: it reports the child exited and signals nothing once the
    /// child that owns the process has reaped it.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_stop_handle_signals_nothing_after_its_child_is_reaped() {
        let child = fixture_child("fixture-mode-json");
        let handle = child.termination_handle().unwrap();
        assert!(!handle.exited().unwrap());
        drop(child);
        assert!(handle.exited().unwrap());
        assert!(!handle.signal(libc::SIGKILL).unwrap());
    }

    /// The value of `--connectors-private-fd`: on Windows the inherited
    /// client end's handle value.
    #[cfg(windows)]
    pub(super) fn private_fd() -> i32 {
        let args: Vec<_> = std::env::args().collect();
        let index = args
            .iter()
            .position(|a| a == "--connectors-private-fd")
            .unwrap();
        args[index + 1].parse().unwrap()
    }

    /// The fixture's end of its channel on Windows: the inherited client end,
    /// taken over as `serve` takes it.
    #[cfg(windows)]
    pub(super) fn private_channel() -> channel::Stream {
        let handle = channel::windows::adopt(private_fd() as usize).unwrap();
        channel::Stream::from_owned(handle).unwrap()
    }

    /// `exact_process_signal` on Windows: the owner's stop handle is a
    /// duplicate of the child's process handle. It ends exactly that child,
    /// reports its exit, and is no error once the child has gone.
    #[cfg(windows)]
    #[test]
    fn a_stop_handle_terminates_exactly_its_child() {
        let mut child = fixture_child("fixture-mode-json");
        let other = fixture_child("fixture-mode-json");
        let handle = child.termination_handle().unwrap();
        assert!(!artifact::exited(&handle).unwrap());
        artifact::terminate(&handle).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.process.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "the child outlived its stop");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(artifact::exited(&handle).unwrap());
        assert!(!child.running().unwrap());
        drop(child);
        artifact::terminate(&handle).unwrap();
        let mut other = other;
        assert!(other.running().unwrap());
    }

    /// `parent_death_kill` on Windows: every adapter is in this process's
    /// kill-on-close job before it runs, and its image copy is removed once
    /// it has exited.
    #[cfg(windows)]
    #[test]
    fn an_adapter_runs_in_the_owner_job_and_its_copy_goes_with_it() {
        use std::os::windows::io::AsRawHandle;
        let child = fixture_child("fixture-mode-json");
        let copy = child._image.path().to_path_buf();
        assert!(copy.exists());
        let mut member = 0;
        // SAFETY: a live process handle, the null job (any job of this
        // process's) and one writable flag.
        let ok = unsafe {
            windows_sys::Win32::System::JobObjects::IsProcessInJob(
                child.process.try_clone_handle().unwrap().as_raw_handle(),
                artifact::Job::adapters().unwrap().raw(),
                &mut member,
            )
        };
        assert_ne!(ok, 0);
        assert_ne!(member, 0, "the adapter is in the owner's job");
        drop(child);
        assert!(!copy.exists(), "the copy outlived its adapter");
    }
}

#[cfg(test)]
mod write_tests;
