use super::*;
use connectors_sdk::Secret;
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(all(unix, any(target_os = "linux", test)))]
use std::os::fd::FromRawFd;
use std::{
    marker::PhantomData,
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
    pub(crate) fn spawn_until(
        config: &super::super::config::Adapter,
        deadline: Instant,
    ) -> Result<Self> {
        let until = deadline.min(Instant::now() + Duration::from_secs(10));
        let executable = config.executable.capture(until)?;
        #[cfg(windows)]
        let (process, image, channel) = spawn_windows(config, executable)?;
        #[cfg(unix)]
        let (channel, child_channel) = UnixStream::pair().map_err(|_| Failure::Unavailable)?;
        #[cfg(unix)]
        channel::peer(&channel)?;
        #[cfg(unix)]
        let child_fd = child_channel.as_raw_fd();
        // SAFETY: getpid has no preconditions.
        #[cfg(unix)]
        let parent = unsafe { libc::getpid() };
        #[cfg(target_os = "linux")]
        let mut command = Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
        // macOS executes the owner-private copy by its path (`owner_private_copy`).
        #[cfg(target_os = "macos")]
        let launch = artifact::launch_path(&executable)?;
        #[cfg(target_os = "macos")]
        let mut command = Command::new(launch.path());
        #[cfg(unix)]
        command
            .args(&config.executable.args)
            .args(["--connectors-private-fd", "3"])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
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
        let mut process = command.spawn().map_err(|_| Failure::Unavailable)?;
        #[cfg(target_os = "macos")]
        let process =
            artifact::ExactChild::spawn(&mut command).map_err(|_| Failure::Unavailable)?;
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
            return Err(Failure::Unavailable);
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
        channel::write(
            &mut child.channel,
            &Request::Hello {
                version: config.private_protocol().as_str().into(),
                nonce: nonce.clone(),
                child_incarnation: incarnation.clone(),
            },
            None,
            &[],
            until,
        )?;
        let response = channel::read::<Reply>(&mut child.channel, until, false, 0)?;
        let Reply::Ready {
            version,
            nonce: returned,
            child_incarnation,
            bootstrap,
        } = response.control
        else {
            return Err(Failure::ReadinessMismatch);
        };
        bootstrap
            .validate_for(config.private_protocol())
            .map_err(|_| Failure::ReadinessMismatch)?;
        if version != config.private_protocol().as_str()
            || returned != nonce
            || child_incarnation != incarnation
            || bootstrap.instance != config.instance_id
            || bootstrap.adapter != config.adapter_id
            || bootstrap.protocol != config.protocol
            || bootstrap.configuration_revision != config.configuration_revision
        {
            return Err(Failure::ReadinessMismatch);
        }
        if child
            .process
            .try_wait()
            .map_err(|_| Failure::Unavailable)?
            .is_some()
        {
            return Err(Failure::Unavailable);
        }
        child.bootstrap = bootstrap;
        Ok(child)
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
            } if request_id == id && frame.document.is_empty() => Ok(baseline),
            // Only a failed `invoke` may carry a reason; one on a validation
            // reply is a protocol violation, like any other malformed reply.
            Reply::Failed {
                request_id,
                code,
                reason: None,
                retry_after_seconds: None,
            } if request_id == id && frame.document.is_empty() => Err(code),
            _ => {
                self.terminate();
                Err(Failure::Protocol)
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
        let mut reason = None;
        let mut delay = None;
        self.dispatch(
            operation,
            revision,
            partition,
            secret,
            input,
            deadline_ms,
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
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
        reason: &mut Option<String>,
        delay: &mut Option<u64>,
    ) -> Result<Vec<u8>> {
        if input.len() > INPUT_LIMIT || !connectors_core::valid_id(partition) {
            return Err(Failure::InvalidInput);
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
        if self.bootstrap.descriptor()?.revision != revision {
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
            },
            Some(secret),
            input,
            deadline_ms,
        )?;
        match frame.control {
            Reply::Success { request_id } if request_id == id => {
                let checked = (|| {
                    channel::depth(&frame.document)?;
                    let value = connectors_core::read_json(&frame.document)
                        .map_err(|_| Failure::Protocol)?;
                    let descriptor = self.bootstrap.descriptor()?;
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
        Bootstrap {
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
        channel::write(
            &mut stream,
            &Reply::Ready {
                version,
                nonce,
                child_incarnation,
                bootstrap: bootstrap(),
            },
            None,
            &[],
            until,
        )
        .unwrap();
        let request = channel::read::<Request>(&mut stream, until, true, INPUT_LIMIT).unwrap();
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
    fn malformed_replies_terminate_and_reap_only_the_owned_child() {
        let (path, hash) = crate::local::test_executable::path_and_sha256();
        for mode in ["json", "schema", "identity"] {
            let config = Adapter {
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

    fn fixture_child(mode: &str) -> Child {
        let (path, sha256) = crate::local::test_executable::path_and_sha256();
        Child::spawn(&Adapter {
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
        })
        .unwrap()
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
