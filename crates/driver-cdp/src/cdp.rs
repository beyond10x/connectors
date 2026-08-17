//! The wire half of this driver: a minimal blocking Chrome `DevTools` Protocol client.
//!
//! This is the *external system* the `cdp_v1` word names. The protocol is JSON request/response
//! with interleaved events over one `WebSocket`, which is a few hundred lines of correlation logic.
//! A vendored browser-automation SDK would bring an async runtime and a large dependency surface
//! for that, against a repository whose discipline is pinned documented surfaces, so the transport
//! is written directly on the `WebSocket` client this crate depends on.
//!
//! Nothing here knows what a page, an element, or a profile is. That meaning belongs to
//! [`crate::chromium`], and the neutral shape of it to [`crate::page`].

use std::io::Read as _;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

/// How long one read may block before the deadline is re-checked.
const READ_SLICE: Duration = Duration::from_millis(250);

/// A bound on one protocol message, so a hostile or broken peer cannot exhaust memory.
const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

/// How long to wait between polls while a launched browser is still coming up.
const POLL_SLICE: Duration = Duration::from_millis(50);

/// Every way the `DevTools` transport refuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CdpError {
    /// The endpoint could not be reached, or does not speak this protocol.
    #[error("devtools endpoint could not be reached: {0}")]
    Transport(String),
    /// The peer closed before answering.
    #[error("devtools connection closed before a response arrived")]
    Closed,
    /// The peer answered with its own refusal. An unanswered command is never a success.
    #[error("devtools call `{method}` failed: {message}")]
    Refused {
        /// The command that was refused.
        method: String,
        /// The peer's own message.
        message: String,
    },
    /// The command exceeded its bound.
    #[error("devtools call `{method}` exceeded its {timeout_ms} ms bound")]
    TimedOut {
        /// The command that ran out of time.
        method: String,
        /// The bound it exceeded.
        timeout_ms: u64,
    },
    /// The peer sent something this client cannot interpret.
    #[error("devtools sent a message this client cannot interpret: {0}")]
    Protocol(String),
}

/// One decoded protocol message.
enum Incoming {
    Response {
        id: u64,
        outcome: Result<Value, String>,
    },
    Event {
        method: String,
        params: Value,
    },
}

/// A blocking `DevTools` client bound to exactly one target.
pub struct CdpClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl CdpClient {
    /// Connect to one exact `ws://` debugger URL.
    ///
    /// No `Origin` header is sent, so the browser's cross-origin debugging restriction does not
    /// apply and `--remote-allow-origins` is never needed.
    ///
    /// # Errors
    ///
    /// Returns [`CdpError::Transport`] when the endpoint cannot be reached or does not speak
    /// `WebSocket`.
    pub fn connect(url: &str) -> Result<Self, CdpError> {
        let (socket, _response) =
            tungstenite::connect(url).map_err(|error| CdpError::Transport(error.to_string()))?;
        let client = Self { socket, next_id: 0 };
        client.set_read_timeout(Some(READ_SLICE))?;
        Ok(client)
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), CdpError> {
        let MaybeTlsStream::Plain(stream) = self.socket.get_ref() else {
            return Ok(());
        };
        stream
            .set_read_timeout(timeout)
            .map_err(|error| CdpError::Transport(error.to_string()))
    }

    /// Issue one command and return its result, discarding events that arrive meanwhile.
    ///
    /// # Errors
    ///
    /// Returns the peer's own refusal, a timeout, or a transport failure. It never treats an
    /// unanswered command as success.
    pub fn call(
        &mut self,
        method: &str,
        params: &Value,
        budget: Duration,
    ) -> Result<Value, CdpError> {
        let id = self.send(method, params)?;
        let deadline = Instant::now() + budget;
        loop {
            match self.receive(method, budget, deadline)? {
                Incoming::Response {
                    id: answered,
                    outcome,
                } if answered == id => {
                    return outcome.map_err(|message| CdpError::Refused {
                        method: method.to_owned(),
                        message,
                    });
                }
                Incoming::Response { .. } | Incoming::Event { .. } => {}
            }
        }
    }

    /// Issue one command and then wait for a named event.
    ///
    /// Navigation is the reason this exists: `Page.navigate` returns as soon as the request is
    /// dispatched, so a snapshot taken straight afterwards would describe the previous document.
    ///
    /// # Errors
    ///
    /// Returns the same refusals as [`Self::call`], plus a timeout when the event never arrives.
    pub fn call_awaiting_event(
        &mut self,
        method: &str,
        params: &Value,
        event: &str,
        budget: Duration,
    ) -> Result<Value, CdpError> {
        let id = self.send(method, params)?;
        let deadline = Instant::now() + budget;
        let mut result = None;
        loop {
            match self.receive(method, budget, deadline)? {
                Incoming::Response {
                    id: answered,
                    outcome,
                } if answered == id => {
                    let value = outcome.map_err(|message| CdpError::Refused {
                        method: method.to_owned(),
                        message,
                    })?;
                    result = Some(value);
                }
                Incoming::Event {
                    method: observed,
                    params,
                } if observed == event => {
                    // A command that answered supplies the result; otherwise the event's own
                    // parameters are the only thing observed, and returning them beats inventing
                    // an empty success.
                    return Ok(result.unwrap_or(params));
                }
                Incoming::Response { .. } | Incoming::Event { .. } => {}
            }
        }
    }

    fn send(&mut self, method: &str, params: &Value) -> Result<u64, CdpError> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        let frame = json!({"id": id, "method": method, "params": params});
        let text =
            serde_json::to_string(&frame).map_err(|error| CdpError::Protocol(error.to_string()))?;
        self.socket
            .send(Message::Text(text.into()))
            .map_err(|error| CdpError::Transport(error.to_string()))?;
        Ok(id)
    }

    fn receive(
        &mut self,
        method: &str,
        budget: Duration,
        deadline: Instant,
    ) -> Result<Incoming, CdpError> {
        loop {
            if Instant::now() >= deadline {
                return Err(CdpError::TimedOut {
                    method: method.to_owned(),
                    timeout_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
                });
            }
            let message = match self.socket.read() {
                Ok(message) => message,
                Err(tungstenite::Error::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    return Err(CdpError::Closed);
                }
                Err(error) => return Err(CdpError::Transport(error.to_string())),
            };
            let text = match message {
                Message::Text(text) => text.to_string(),
                Message::Binary(bytes) => {
                    if bytes.len() > MAX_MESSAGE_BYTES {
                        return Err(CdpError::Protocol("message exceeds its bound".to_owned()));
                    }
                    String::from_utf8(bytes.to_vec())
                        .map_err(|_| CdpError::Protocol("message is not UTF-8".to_owned()))?
                }
                Message::Close(_) => return Err(CdpError::Closed),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            };
            if text.len() > MAX_MESSAGE_BYTES {
                return Err(CdpError::Protocol("message exceeds its bound".to_owned()));
            }
            return decode(&text);
        }
    }
}

fn decode(text: &str) -> Result<Incoming, CdpError> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| CdpError::Protocol(error.to_string()))?;
    if let Some(id) = value.get("id").and_then(Value::as_u64) {
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unspecified devtools error");
            return Ok(Incoming::Response {
                id,
                outcome: Err(message.to_owned()),
            });
        }
        return Ok(Incoming::Response {
            id,
            outcome: Ok(value.get("result").cloned().unwrap_or_else(|| json!({}))),
        });
    }
    let method = value
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| CdpError::Protocol("message is neither a response nor an event".to_owned()))?
        .to_owned();
    Ok(Incoming::Event {
        method,
        params: value.get("params").cloned().unwrap_or_else(|| json!({})),
    })
}

/// Read the port and browser endpoint the browser actually bound.
///
/// **The launch flag is deliberately not trusted.** Distribution launchers such as `/usr/bin/brave`
/// are shell wrappers that `exec` the real binary with `"$@"` followed by the operator's own
/// `*-flags.conf` entries — user flags are appended *after* ours, so a user-supplied
/// `--remote-debugging-port` wins. `/usr/bin/google-chrome-stable` is the same pattern.
/// `DevToolsActivePort` records what the browser bound and is immune to flag ordering: line 1 is the
/// port, line 2 the browser endpoint path.
///
/// # Errors
///
/// Returns [`CdpError::Transport`] when the file does not appear within `budget` or does not carry
/// a non-zero port and a path.
pub fn read_active_port(
    user_data_dir: &std::path::Path,
    budget: Duration,
) -> Result<(u16, String), CdpError> {
    let path = user_data_dir.join("DevToolsActivePort");
    let deadline = Instant::now() + budget;
    loop {
        if let Ok(mut file) = std::fs::File::open(&path) {
            let mut body = String::new();
            if file.read_to_string(&mut body).is_ok() {
                let mut lines = body.lines();
                if let (Some(port), Some(endpoint)) = (lines.next(), lines.next()) {
                    // A zero port means the browser has written the file but has not bound yet.
                    match port.trim().parse::<u16>() {
                        Ok(port) if port != 0 => {
                            return Ok((port, endpoint.trim().to_owned()));
                        }
                        Ok(_) | Err(_) => {}
                    }
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(CdpError::Transport(format!(
                "`{}` did not report a debugging port",
                path.display()
            )));
        }
        std::thread::sleep(POLL_SLICE);
    }
}

/// Find the first attachable page target on a local debugging endpoint.
///
/// The client is built without any TLS backend on purpose: this driver's only HTTP call is
/// `http://127.0.0.1:<port>/json/list`, so a transport that cannot speak TLS at all is the exact
/// aperture, and an address that is not loopback plaintext is not reachable through it.
///
/// # Errors
///
/// Returns [`CdpError::Transport`] when the endpoint is unreachable or exposes no page target.
pub fn first_page_target(port: u16, budget: Duration) -> Result<String, CdpError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(budget)
        .build()
        .map_err(|error| CdpError::Transport(error.to_string()))?;
    let deadline = Instant::now() + budget;
    loop {
        let response = client
            .get(format!("http://127.0.0.1:{port}/json/list"))
            .send()
            .and_then(reqwest::blocking::Response::json::<Value>);
        if let Ok(Value::Array(targets)) = response {
            let page = targets
                .iter()
                .filter(|target| target.get("type").and_then(Value::as_str) == Some("page"))
                .find_map(|target| target.get("webSocketDebuggerUrl").and_then(Value::as_str));
            if let Some(url) = page {
                return Ok(url.to_owned());
            }
        }
        if Instant::now() >= deadline {
            return Err(CdpError::Transport(
                "the local devtools endpoint exposed no page target".to_owned(),
            ));
        }
        std::thread::sleep(POLL_SLICE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_decodes_to_its_correlated_result() {
        let incoming = decode(r#"{"id":7,"result":{"frameId":"F"}}"#).expect("decode");
        match incoming {
            Incoming::Response { id, outcome } => {
                assert_eq!(id, 7);
                assert_eq!(outcome.expect("result")["frameId"], json!("F"));
            }
            Incoming::Event { .. } => panic!("a response decoded as an event"),
        }
    }

    #[test]
    fn a_peer_refusal_is_carried_rather_than_treated_as_success() {
        let incoming =
            decode(r#"{"id":9,"error":{"code":-32000,"message":"no such node"}}"#).expect("decode");
        match incoming {
            Incoming::Response { id, outcome } => {
                assert_eq!(id, 9);
                assert_eq!(outcome.expect_err("refusal"), "no such node");
            }
            Incoming::Event { .. } => panic!("a refusal decoded as an event"),
        }
    }

    #[test]
    fn an_event_decodes_with_its_method_and_parameters() {
        let incoming = decode(r#"{"method":"Page.loadEventFired","params":{"timestamp":1.5}}"#)
            .expect("decode");
        match incoming {
            Incoming::Event { method, params } => {
                assert_eq!(method, "Page.loadEventFired");
                assert_eq!(params["timestamp"], json!(1.5));
            }
            Incoming::Response { .. } => panic!("an event decoded as a response"),
        }
    }

    #[test]
    fn an_uninterpretable_message_refuses_rather_than_being_ignored() {
        assert!(matches!(
            decode(r#"{"unexpected":true}"#),
            Err(CdpError::Protocol(_))
        ));
        assert!(matches!(decode("not json"), Err(CdpError::Protocol(_))));
    }

    #[test]
    fn an_absent_active_port_file_refuses_by_path() {
        let directory = tempfile::tempdir().expect("temp dir");
        let error = read_active_port(directory.path(), Duration::from_millis(120))
            .expect_err("absent port file");
        match error {
            CdpError::Transport(message) => assert!(message.contains("DevToolsActivePort")),
            other => panic!("unexpected refusal: {other}"),
        }
    }

    /// The launch flag is never the source of truth, so a file that does not name a real bound port
    /// must refuse rather than let an attach proceed against a port nobody verified.
    #[test]
    fn a_zero_or_malformed_active_port_is_not_accepted() {
        let directory = tempfile::tempdir().expect("temp dir");
        for body in ["0\n/devtools/browser/x\n", "not-a-port\n/x\n", "45123\n"] {
            std::fs::write(directory.path().join("DevToolsActivePort"), body).expect("write");
            assert!(
                read_active_port(directory.path(), Duration::from_millis(120)).is_err(),
                "accepted {body:?}"
            );
        }
    }

    #[test]
    fn a_written_active_port_is_read_back_with_its_endpoint() {
        let directory = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            directory.path().join("DevToolsActivePort"),
            "45123\n/devtools/browser/abc\n",
        )
        .expect("write");
        let (port, endpoint) =
            read_active_port(directory.path(), Duration::from_millis(120)).expect("port");
        assert_eq!(port, 45_123);
        assert_eq!(endpoint, "/devtools/browser/abc");
    }
}
