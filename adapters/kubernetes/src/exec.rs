//! `pods.exec`, a mutation: one command in one named container, through the
//! pod's exec subresource upgraded to a WebSocket with the
//! `v5.channel.k8s.io` or `v4.channel.k8s.io` subprotocol. The typed model is
//! `PodExecRequest`, `ExecChannel`, `PodExecResult` and `PodExecOutcome` in
//! the adapter's ESS `mutations` domain; the normative rules are
//! `contracts/mutations/v1alpha1/semantics.md`, `pods.exec`.
//!
//! Authority stays with the host, exactly as for every implemented mutation:
//! it admits the connection, spends the approval, records the attempt and
//! hands the composition one consuming write capability. This module only
//! prepares the request (no I/O) and settles the outcome from what the
//! stream delivered. It never retries and never settles an unknown outcome.
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::WriteOutcome;
use serde::Deserialize;
use serde_json::{Value, json};

/// The subprotocols offered, in preference order. Both frame every binary
/// message with a leading channel byte and end with a v1 `Status` on channel
/// 3, success included; older subprotocols report errors as text and are not
/// offered.
pub const PROTOCOLS: [&str; 2] = ["v5.channel.k8s.io", "v4.channel.k8s.io"];
pub const DEFAULT_TIMEOUT_SECONDS: u32 = 30;
pub const DEFAULT_MAX_OUTPUT_BYTES: u32 = 65_536;
/// The most stream bytes one attempt consumes, retained or discarded. Past it
/// the status can no longer be waited for, so the outcome is unknown.
pub const MAX_CONSUMED_BYTES: usize = 16 * 1024 * 1024;
/// The largest status document accepted on channel 3.
pub const MAX_STATUS_BYTES: usize = 64 * 1024;
const MAX_ARGUMENT_BYTES: usize = 4096;
const MAX_COMMAND_BYTES: usize = 16_384;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub namespace: String,
    pub pod: String,
    pub container: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u32>,
    #[serde(default)]
    pub max_output_bytes: Option<u32>,
}

/// A prepared, not yet dispatched, exec. It carries no credential and grants
/// nothing: the composition pairs it with the host's one-use capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    pub instance: String,
    pub namespace: String,
    pub pod: String,
    pub container: String,
    pub command: Vec<String>,
    pub timeout_seconds: u32,
    pub max_output_bytes: u32,
}

/// Bounds of the input that do not need the configuration. Names follow the
/// same DNS-1123 rules as every other read of this adapter, so none can add a
/// path segment or query term.
pub(crate) fn admit(instance: &str, input: Input) -> Result<Prepared> {
    let timeout_seconds = input.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    let max_output_bytes = input.max_output_bytes.unwrap_or(DEFAULT_MAX_OUTPUT_BYTES);
    if !(1..=60).contains(&timeout_seconds) || !(1..=1_048_576).contains(&max_output_bytes) {
        return Err(Error::invalid(
            "timeout_seconds or max_output_bytes is outside its bound",
        ));
    }
    if input.command.is_empty()
        || input.command.len() > 64
        || input
            .command
            .iter()
            .any(|argument| argument.is_empty() || argument.len() > MAX_ARGUMENT_BYTES)
        || input.command.iter().map(String::len).sum::<usize>() > MAX_COMMAND_BYTES
    {
        return Err(Error::invalid(
            "command must be a nonempty argument vector within its bounds",
        ));
    }
    if !crate::valid_name(&input.pod) {
        return Err(Error::invalid("pod is not a Kubernetes object name"));
    }
    if !crate::valid_label(&input.container) {
        return Err(Error::invalid(
            "container is not a Kubernetes container name",
        ));
    }
    Ok(Prepared {
        instance: instance.to_owned(),
        namespace: input.namespace,
        pod: input.pod,
        container: input.container,
        command: input.command,
        timeout_seconds,
        max_output_bytes,
    })
}

impl Prepared {
    pub fn segments(&self) -> [&str; 7] {
        [
            "api",
            "v1",
            "namespaces",
            self.namespace.as_str(),
            "pods",
            self.pod.as_str(),
            "exec",
        ]
    }
    /// One `command` term per argument, in order: the argument vector the
    /// process receives. stdin and tty are off, so only channels 1 to 3 open.
    pub fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = vec![("container", self.container.clone())];
        query.extend(
            self.command
                .iter()
                .map(|argument| ("command", argument.clone())),
        );
        query.extend([
            ("stdin", "false".to_owned()),
            ("stdout", "true".to_owned()),
            ("stderr", "true".to_owned()),
            ("tty", "false".to_owned()),
        ]);
        query
    }
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(u64::from(self.timeout_seconds))
    }
}

/// The server half of an upgraded exec stream, as the composition's write
/// capability delivers it: whole binary WebSocket messages, ping/pong and
/// fragmentation already handled below this port.
#[async_trait::async_trait]
pub trait ExecStream: Send {
    /// The subprotocol the server selected.
    fn protocol(&self) -> &str;
    /// The next binary message. `Ok(None)` is a clean close; an error is a
    /// lost stream.
    async fn next_message(&mut self) -> Result<Option<Vec<u8>>>;
}

/// What became of the upgrade request.
pub enum Upgrade {
    /// Refused before anything was sent: no attempt reached the provider.
    NotSent(Error),
    /// The API server answered the upgrade request with this status and no
    /// stream, so no process was started.
    Answered(u16),
    /// The request was sent and nothing more is known.
    Lost(Error),
    /// The server switched protocols; the command may be running.
    Accepted(Box<dyn ExecStream>),
}

/// Keeps the first `limit` bytes of one channel and counts the rest.
struct Retained {
    bytes: Vec<u8>,
    limit: usize,
    truncated: bool,
}
impl Retained {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            truncated: false,
        }
    }
    fn push(&mut self, data: &[u8]) {
        let keep = data.len().min(self.limit - self.bytes.len());
        self.bytes.extend_from_slice(&data[..keep]);
        self.truncated |= keep < data.len();
    }
    /// The retained bytes as text. At a cut, an incomplete trailing scalar is
    /// withheld rather than replaced, wherever earlier invalid bytes are; any
    /// other invalid byte is replaced.
    fn text(&self) -> String {
        let end = if self.truncated {
            incomplete_tail(&self.bytes)
        } else {
            self.bytes.len()
        };
        String::from_utf8_lossy(&self.bytes[..end]).into_owned()
    }
}

/// Where an incomplete trailing UTF-8 scalar starts, or the length when the
/// bytes do not end inside one. A scalar is at most four bytes, so only the
/// last three can begin an incomplete one; a start qualifies only when the
/// bytes from it are a valid prefix of some scalar.
fn incomplete_tail(bytes: &[u8]) -> usize {
    (bytes.len().saturating_sub(3)..bytes.len())
        .rev()
        .find(|&start| {
            matches!(std::str::from_utf8(&bytes[start..]),
                Err(error) if error.valid_up_to() == 0 && error.error_len().is_none())
        })
        .unwrap_or(bytes.len())
}

fn unknown(code: ErrorCode, message: &str) -> WriteOutcome<Value> {
    WriteOutcome::Unknown(Error::new(code, message))
}

/// Settle one attempt. Applied means the status message was received and
/// carried an exit code; a non-zero exit is applied too, because the command
/// ran to completion. Refused means the command provably did not start.
/// Everything else after the request was sent is unknown.
pub async fn settle(prepared: Prepared, upgrade: Upgrade) -> WriteOutcome<Value> {
    let mut stream = match upgrade {
        Upgrade::NotSent(error) => return WriteOutcome::Refused(error),
        Upgrade::Answered(status) => {
            return match status {
                400 | 422 => {
                    WriteOutcome::Refused(Error::invalid("provider refused the exec request"))
                }
                401 => WriteOutcome::Refused(Error::new(
                    ErrorCode::Unauthorized,
                    "provider refused the exec request",
                )),
                403 => WriteOutcome::Refused(Error::new(
                    ErrorCode::Forbidden,
                    "provider refused the exec request",
                )),
                404 => WriteOutcome::Refused(Error::new(
                    ErrorCode::NotFound,
                    "pod or container not found",
                )),
                // A 5xx or anything else is not proof the command did not run.
                _ => unknown(
                    ErrorCode::Unavailable,
                    "exec request answered without a settled outcome",
                ),
            };
        }
        Upgrade::Lost(error) => return WriteOutcome::Unknown(error),
        Upgrade::Accepted(stream) => stream,
    };
    if !PROTOCOLS.contains(&stream.protocol()) {
        return unknown(
            ErrorCode::UpstreamProtocol,
            "exec stream selected a subprotocol that was not offered",
        );
    }
    let limit = prepared.max_output_bytes as usize;
    let mut stdout = Retained::new(limit);
    let mut stderr = Retained::new(limit);
    let mut status = Vec::new();
    let mut consumed = 0_usize;
    // How the stream ended when it did not close cleanly: the port's code
    // (`connectors.transport.UpgradedStreamEnd`) is kept for the unknown
    // outcome, so a deadline stays `timeout` and a host byte bound `capacity`.
    let ended = loop {
        let message = match stream.next_message().await {
            Ok(Some(message)) => message,
            Ok(None) => break None,
            Err(error) => break Some(error.code),
        };
        consumed = consumed.saturating_add(message.len());
        if consumed > MAX_CONSUMED_BYTES {
            return unknown(
                ErrorCode::Capacity,
                "exec output exceeded the consumable bound before its status",
            );
        }
        let Some((&channel, data)) = message.split_first() else {
            return unknown(
                ErrorCode::UpstreamProtocol,
                "exec stream sent an empty message",
            );
        };
        match channel {
            1 => stdout.push(data),
            2 => stderr.push(data),
            3 => {
                if status.len() + data.len() > MAX_STATUS_BYTES {
                    return unknown(ErrorCode::UpstreamProtocol, "exec status exceeds its bound");
                }
                status.extend_from_slice(data);
            }
            _ => {
                return unknown(
                    ErrorCode::UpstreamProtocol,
                    "exec stream used a channel this binding did not open",
                );
            }
        }
    };
    // A status is the only settlement. A close or a loss before one, or a
    // status that does not parse, leaves the command's completion unobserved.
    let exit_code = match exit_code(&status) {
        Some(code) => code,
        None => {
            let Some(code) = ended else {
                return unknown(
                    ErrorCode::UpstreamProtocol,
                    "exec stream closed without a status this binding can settle",
                );
            };
            return match code {
                ErrorCode::Timeout => unknown(
                    code,
                    "exec stream reached its deadline before the command's status",
                ),
                ErrorCode::Capacity => unknown(
                    code,
                    "exec stream reached a host byte bound before the command's status",
                ),
                ErrorCode::UpstreamProtocol => unknown(
                    code,
                    "exec stream carried an invalid message before the command's status",
                ),
                // A loss, or any code the port does not declare for a stream end.
                _ => unknown(
                    ErrorCode::Unavailable,
                    "exec stream was lost before the command's status",
                ),
            };
        }
    };
    WriteOutcome::Applied(Ok(json!({
        "namespace": prepared.namespace,
        "pod": prepared.pod,
        "container": prepared.container,
        "exit_code": exit_code,
        "stdout": stdout.text(),
        "stderr": stderr.text(),
        "stdout_truncated": stdout.truncated,
        "stderr_truncated": stderr.truncated,
        "provenance": connectors_sdk::provenance(
            &prepared.instance,
            format!("{}/pods/{}/exec", prepared.namespace, prepared.pod),
            None,
        ),
    })))
}

/// The exit code a v1 `Status` on channel 3 settles: 0 for `Success`, the
/// `ExitCode` cause of a `NonZeroExitCode` failure, and nothing for any other
/// document, which is a failure this binding cannot attribute to the command.
pub fn exit_code(status: &[u8]) -> Option<u8> {
    if status.is_empty() {
        return None;
    }
    let status: Value = serde_json::from_slice(status).ok()?;
    match status["status"].as_str()? {
        "Success" => Some(0),
        "Failure" if status["reason"] == "NonZeroExitCode" => {
            let causes = status["details"]["causes"].as_array()?;
            let mut codes = causes
                .iter()
                .filter(|cause| cause["reason"] == "ExitCode")
                .map(|cause| cause["message"].as_str());
            let code = codes.next()??;
            if codes.next().is_some()
                || code.is_empty()
                || code.len() > 3
                || !code.bytes().all(|b| b.is_ascii_digit())
                || (code.len() > 1 && code.starts_with('0'))
            {
                return None;
            }
            code.parse::<u8>().ok().filter(|code| *code != 0)
        }
        _ => None,
    }
}
