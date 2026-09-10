use super::*;
use connectors_sdk::Secret;
use serde_json::Value;
use std::{
    os::{fd::FromRawFd, unix::net::UnixStream},
    time::{Duration, Instant},
};

/// Implemented by trusted executable composition, not the provider business
/// library. Its methods receive only this command's protected material.
#[async_trait::async_trait]
pub trait Adapter: Send + Sync {
    fn bootstrap(&self) -> Bootstrap;
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline>;
    async fn invoke(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value>;
}

/// Private inherited descriptor three is the only accepted entry. No user path,
/// stdio fallback or process-wide credential environment is consulted.
pub fn serve(fd: i32, adapter: impl Adapter) -> Result<()> {
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
    let mut channel = unsafe { UnixStream::from_raw_fd(raw) };
    let peer = channel::peer(&channel)?;
    // SAFETY: getppid has no preconditions. Socketpair creator must be this parent.
    if peer.pid != unsafe { libc::getppid() } {
        return Err(Failure::Unavailable);
    }
    // SAFETY: descriptor three was inherited for this binding and now duplicated.
    unsafe {
        libc::close(fd);
    }
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
    if version != VERSION
        || uuid::Uuid::parse_str(&nonce).is_err()
        || uuid::Uuid::parse_str(&child_incarnation).is_err()
    {
        return Err(Failure::Protocol);
    }
    let bootstrap = adapter.bootstrap();
    bootstrap.validate()?;
    channel::write(
        &mut channel,
        &Reply::Ready {
            version: VERSION.into(),
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
        let frame = channel::read::<Request>(
            &mut channel,
            Instant::now() + Duration::from_secs(10),
            true,
            INPUT_LIMIT,
        )?;
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
            } => (
                request_id,
                deadline_ms,
                Action::Invoke {
                    operation,
                    revision,
                    partition,
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
        let reply = executor.block_on(async {
            let future = async {
                match operation {
                    Action::Validate(profile) => {
                        bootstrap.profile(&profile)?;
                        if !frame.document.is_empty() {
                            return Err(Failure::Protocol);
                        }
                        let baseline = adapter.validate(&profile, frame.secret).await?;
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
                    } => {
                        if !connectors_core::valid_id(&partition) {
                            return Err(Failure::Protocol);
                        }
                        let descriptor = bootstrap.descriptor()?;
                        if revision != descriptor.revision {
                            return Err(Failure::StaleDescription);
                        }
                        let requirement = bootstrap
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
                            .invoke(&operation, &partition, frame.secret, input)
                            .await?;
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
    },
}
