use super::*;
use connectors_sdk::Secret;
use serde_json::Value;
#[cfg(unix)]
use std::os::{fd::FromRawFd, unix::net::UnixStream};
use std::time::{Duration, Instant};

/// Implemented by trusted executable composition, not the provider business
/// library. Its methods receive only this command's protected material.
#[async_trait::async_trait]
pub trait Adapter: Send + Sync {
    fn bootstrap(&self) -> Bootstrap;
    /// Explicit opt-in with the separately revised full private projection.
    /// Existing read-only compositions refuse v2 before receiving credentials.
    fn bootstrap_v2(&self) -> Result<Bootstrap> {
        Err(Failure::Unsupported)
    }
    async fn prepare_write(
        &self,
        _operation: &str,
        _partition: &str,
        _document: Secret,
        _input: Value,
    ) -> Result<Box<dyn PreparedWrite>> {
        Err(Failure::Unsupported)
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline>;
    /// [`Adapter::validate`] with the reason beside `insufficient_scope` when
    /// every alternative binding was refused (private-adapter.md,
    /// "Alternative bindings"); the server answers through this one and sends
    /// a reason beside no other code. By default no reason is given.
    async fn validate_explained(
        &self,
        profile: &str,
        document: Secret,
    ) -> std::result::Result<Baseline, Refusal> {
        self.validate(profile, document)
            .await
            .map_err(Refusal::from)
    }
    async fn invoke(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value>;
    /// [`Adapter::invoke`] with the upstream's admitted reason and named delay
    /// for a refusal (see [`Refusal::from_provider`]); the server answers
    /// through this one. `deadline` is the instant the invocation's reply is
    /// due: an adapter that waits before sending again never waits past it.
    /// By default no reason or delay is given and the deadline is not read.
    async fn invoke_explained(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
        _deadline: Instant,
    ) -> std::result::Result<Value, Refusal> {
        self.invoke(operation, partition, document, input)
            .await
            .map_err(Refusal::from)
    }
    /// [`Adapter::invoke_explained`] under `binding`, one the bootstrap
    /// declares (the server has checked it, with the operation, revision and
    /// input, against that binding's description). By default only an
    /// unbound read is served.
    async fn invoke_bound(
        &self,
        binding: Option<&str>,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
        deadline: Instant,
    ) -> std::result::Result<Value, Refusal> {
        if binding.is_some() {
            return Err(Failure::Unsupported.into());
        }
        self.invoke_explained(operation, partition, document, input, deadline)
            .await
    }
}

/// Private inherited descriptor three is the only accepted entry. No user path,
/// stdio fallback or process-wide credential environment is consulted.
#[cfg(unix)]
pub fn serve(fd: i32, adapter: impl Adapter) -> Result<()> {
    // Before any protected byte arrives: a created credential passes through.
    crate::local::core_dumps::forbid().map_err(|_| Failure::Unavailable)?;
    if fd != 3 {
        return Err(Failure::InvalidInput);
    }
    // Duplicate rather than claim ownership of an arbitrary caller descriptor.
    // SAFETY: fcntl validates descriptor three and returns a new owned descriptor.
    let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 4) };
    if raw < 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: the descriptor will be checked as a connected Unix socket before use.
    let channel = unsafe { UnixStream::from_raw_fd(raw) };
    let peer = channel::peer(&channel)?;
    #[cfg(target_os = "linux")]
    let pid = Some(peer.pid);
    // A peer that has closed has no pid on macOS, and is refused here.
    #[cfg(target_os = "macos")]
    let pid = peer.pid;
    // SAFETY: getppid has no preconditions. Socketpair creator must be this parent.
    if pid != Some(unsafe { libc::getppid() }) {
        return Err(Failure::Unavailable);
    }
    // SAFETY: descriptor three was inherited for this binding and now duplicated.
    unsafe {
        libc::close(fd);
    }
    run(channel, adapter)
}
/// On Windows the private entry is the inherited client end of the owner's
/// pipe, whose handle value `fd` is (private-adapter.md, "Windows"). It is
/// taken over as a handle that is not inheritable and the inherited one is
/// closed; the pipe's server must be this process's parent and run as the
/// current user (`client_token_query`). The adapter is never told a pipe name.
#[cfg(windows)]
pub fn serve(fd: i32, adapter: impl Adapter) -> Result<()> {
    // Before any protected byte arrives: a created credential passes through.
    crate::local::core_dumps::forbid().map_err(|_| Failure::Unavailable)?;
    let handle = usize::try_from(fd).map_err(|_| Failure::InvalidInput)?;
    let channel = channel::Stream::from_owned(channel::windows::adopt(handle)?)?;
    let peer = channel::peer(&channel)?;
    if Some(peer.pid) != channel::windows::parent_process_id() {
        return Err(Failure::Unavailable);
    }
    run(channel, adapter)
}
fn run(mut channel: channel::Stream, adapter: impl Adapter) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(10);
    let hello = channel::read::<Request>(&mut channel, until, false, 0)?;
    let Request::Hello {
        version,
        nonce,
        child_incarnation,
    } = hello.control
    else {
        return Err(Failure::Protocol);
    };
    let protocol = match version.as_str() {
        VERSION => PrivateProtocol::V1,
        WRITE_VERSION => PrivateProtocol::V2,
        _ => return Err(Failure::Protocol),
    };
    if uuid::Uuid::parse_str(&nonce).is_err()
        || uuid::Uuid::parse_str(&child_incarnation).is_err()
        || (protocol == PrivateProtocol::V2
            && (!writes::canonical_id(&nonce) || !writes::canonical_id(&child_incarnation)))
    {
        return Err(Failure::Protocol);
    }
    let bootstrap = match protocol {
        PrivateProtocol::V1 => adapter.bootstrap(),
        PrivateProtocol::V2 => adapter.bootstrap_v2()?,
    };
    bootstrap.validate_for(protocol)?;
    channel::write(
        &mut channel,
        &Reply::Ready {
            version,
            nonce,
            child_incarnation,
            bootstrap: bootstrap.clone(),
        },
        None,
        &[],
        until,
    )?;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Failure::Unavailable)?;
    loop {
        // Idle waits are not provider deadlines. A kernel parent-death signal or
        // channel EOF terminates the child; each request gets its original budget.
        channel::wait_readable(&channel)?;
        let until = Instant::now() + Duration::from_secs(10);
        let frame = match protocol {
            PrivateProtocol::V1 => {
                channel::read::<Request>(&mut channel, until, true, INPUT_LIMIT)?
            }
            PrivateProtocol::V2 => {
                let frame =
                    channel::read::<writes::RequestV2>(&mut channel, until, true, INPUT_LIMIT)?;
                match frame.control {
                    writes::RequestV2::Legacy(control) => channel::Frame {
                        control,
                        secret: frame.secret,
                        document: frame.document,
                    },
                    writes::RequestV2::Write(control) => {
                        writes::serve_write(
                            &mut channel,
                            &executor,
                            &adapter,
                            &bootstrap,
                            channel::Frame {
                                control,
                                secret: frame.secret,
                                document: frame.document,
                            },
                        )?;
                        continue;
                    }
                }
            }
        };
        let (id, deadline, operation) = match frame.control {
            Request::Validate {
                request_id,
                profile,
                deadline_ms,
            } => (request_id, deadline_ms, Action::Validate(profile)),
            Request::Invoke {
                request_id,
                operation,
                revision,
                partition,
                deadline_ms,
                binding,
            } => (
                request_id,
                deadline_ms,
                Action::Invoke {
                    operation,
                    revision,
                    partition,
                    binding,
                },
            ),
            Request::Stop { request_id }
                if connectors_core::valid_id(&request_id)
                    && frame.secret.0.is_empty()
                    && frame.document.is_empty() =>
            {
                channel::write(
                    &mut channel,
                    &Reply::Stopped { request_id },
                    None,
                    &[],
                    Instant::now() + Duration::from_secs(5),
                )?;
                return Ok(());
            }
            _ => return Err(Failure::Protocol),
        };
        if !connectors_core::valid_id(&id) || frame.secret.0.is_empty() {
            return Err(Failure::Protocol);
        }
        let budget = deadline
            .checked_sub(connectors_sdk::now_ms())
            .filter(|ms| (1..=120_000).contains(ms))
            .ok_or(Failure::Timeout)?;
        let until = Instant::now() + Duration::from_millis(budget);
        let execution_budget = if matches!(operation, Action::Validate(_)) {
            budget.min(30_000)
        } else {
            budget
        };
        let mut reason = None;
        let mut retry_after_seconds = None;
        let reply = executor.block_on(async {
            let future = async {
                match operation {
                    Action::Validate(profile) => {
                        bootstrap.profile(&profile)?;
                        if !frame.document.is_empty() {
                            return Err(Failure::Protocol);
                        }
                        let baseline = adapter
                            .validate_explained(&profile, frame.secret)
                            .await
                            .map_err(|refusal| {
                                if refusal.failure == Failure::InsufficientScope {
                                    reason = refusal.reason;
                                }
                                refusal.failure
                            })?;
                        // The host refuses any other baseline: send none.
                        if !bootstrap.declares_binding(baseline.binding.as_deref()) {
                            return Err(Failure::Protocol);
                        }
                        Ok((
                            Reply::Validated {
                                request_id: id.clone(),
                                baseline,
                            },
                            Vec::new(),
                        ))
                    }
                    Action::Invoke {
                        operation,
                        revision,
                        partition,
                        binding,
                    } => {
                        if !connectors_core::valid_id(&partition) {
                            return Err(Failure::Protocol);
                        }
                        let bound = bootstrap.bound(binding.as_deref())?;
                        let descriptor = bound.descriptor()?;
                        if revision != descriptor.revision {
                            return Err(Failure::StaleDescription);
                        }
                        let requirement = bound
                            .requirements
                            .iter()
                            .find(|r| r.operation == operation)
                            .ok_or(Failure::NotFound)?;
                        if requirement.effect != Effect::Read {
                            return Err(Failure::Unsupported);
                        }
                        let declaration = descriptor
                            .operation(&operation)
                            .map_err(Failure::from_service)?;
                        channel::depth(&frame.document)?;
                        let input = connectors_core::read_json(&frame.document)
                            .map_err(|_| Failure::InvalidInput)?;
                        connectors_sdk::validate(&declaration.input_schema, &input)
                            .map_err(Failure::from_service)?;
                        let result = adapter
                            .invoke_bound(
                                binding.as_deref(),
                                &operation,
                                &partition,
                                frame.secret,
                                input,
                                until,
                            )
                            .await
                            .map_err(|refusal| {
                                reason = refusal.reason;
                                retry_after_seconds = refusal.retry_after_seconds;
                                refusal.failure
                            })?;
                        connectors_sdk::validate(&declaration.output_schema, &result)
                            .map_err(|_| Failure::Protocol)?;
                        let document =
                            serde_json::to_vec(&result).map_err(|_| Failure::Protocol)?;
                        if document.len() > RESULT_LIMIT {
                            return Err(Failure::Capacity);
                        }
                        Ok((
                            Reply::Success {
                                request_id: id.clone(),
                            },
                            document,
                        ))
                    }
                }
            };
            tokio::time::timeout(Duration::from_millis(execution_budget), future)
                .await
                .map_err(|_| Failure::Timeout)?
        });
        let (control, document) = match reply {
            Ok(value) => value,
            Err(code) => (
                Reply::Failed {
                    request_id: id,
                    code,
                    reason,
                    retry_after_seconds,
                },
                Vec::new(),
            ),
        };
        channel::write(&mut channel, &control, None, &document, until)?;
    }
}
enum Action {
    Validate(String),
    Invoke {
        operation: String,
        revision: String,
        partition: String,
        binding: Option<String>,
    },
}
