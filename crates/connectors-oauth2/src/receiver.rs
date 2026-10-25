//! The redirect receivers: a loopback listener on `127.0.0.1`, and the pasted `code#state` line.
use crate::{
    Authorization, AuthorizationCode, Cancellation, Failure, RedirectKind, Secret,
    ServerDeclaration, declaration::loopback,
};
use std::{
    io::{ErrorKind, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// The largest request head a connection may send, headers included: a browser sends every
/// cookie it holds for the loopback host with the redirect.
const HEAD_LIMIT: usize = 64 * 1024;
/// How long an accepted connection may take to complete its request head.
const IDLE: Duration = Duration::from_secs(2);
/// The most connections read at once; more are closed on accept.
const PENDING_LIMIT: usize = 64;
const POLL: Duration = Duration::from_millis(20);

/// The loopback redirect receiver of `loopback_any_port` and `loopback_fixed_port`. It listens
/// on `127.0.0.1` only. The first connection to complete a request head for the redirect path
/// decides the flow; a head for another path is answered 404 and decides nothing; a head
/// completed after the decision is answered 400. A connection that sends no complete head
/// within 2 s, or a head over 64 KiB, is dropped unanswered; at most 64 connections are read at
/// once.
#[derive(Debug)]
pub struct LoopbackReceiver {
    listener: TcpListener,
    redirect_uri: String,
    path: String,
}

impl LoopbackReceiver {
    /// Binds the listener the declaration's redirect kind names: for `loopback_any_port`, a
    /// port the system chooses, with the redirect URI the declaration's template (its host,
    /// `127.0.0.1` or `localhost`, and its path) at that port, or `http://127.0.0.1:<port>/`
    /// without one; for `loopback_fixed_port`, the port and path of the declared redirect URI.
    /// `pasted_code` and a redirect URI that is not plain loopback HTTP are
    /// [`Failure::InvalidDeclaration`]; a port that cannot be bound is
    /// [`Failure::Unavailable`].
    pub fn bind(declaration: &ServerDeclaration) -> Result<Self, Failure> {
        let invalid = Failure::InvalidDeclaration;
        let (port, mut uri) = match declaration.redirect_kind {
            RedirectKind::PastedCode => return Err(invalid),
            RedirectKind::LoopbackAnyPort => match declaration.redirect_uri.as_deref() {
                Some(template) => (0, loopback(template).ok_or(invalid)?),
                None => (0, loopback("http://127.0.0.1/").ok_or(invalid)?),
            },
            RedirectKind::LoopbackFixedPort => {
                let uri = declaration
                    .redirect_uri
                    .as_deref()
                    .and_then(loopback)
                    .ok_or(invalid)?;
                (uri.port().ok_or(invalid)?, uri)
            }
        };
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|_| Failure::Unavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| Failure::Unavailable)?;
        let bound = listener
            .local_addr()
            .map_err(|_| Failure::Unavailable)?
            .port();
        uri.set_port(Some(bound)).map_err(|()| invalid)?;
        Ok(Self {
            listener,
            path: uri.path().to_owned(),
            redirect_uri: uri.into(),
        })
    }

    /// The redirect URI to send in the authorization and in the exchange.
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// The address the listener is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .unwrap_or_else(|_| SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
    }

    /// Waits up to `timeout` for the redirect that decides `authorization`. A redirect with the
    /// flow's `state` and a code is answered 200 and returns the code; one whose `state` is
    /// absent or wrong is [`Failure::StateMismatch`], one with `error=access_denied`
    /// [`Failure::AccessDenied`], another error [`Failure::Rejected`], one without a code
    /// [`Failure::InvalidResponse`], each answered 400. Requests completed alongside or after
    /// the deciding one are answered 400 and change nothing: a connection accepted before the
    /// decision whose head completes after it is answered by a thread that ends once each such
    /// connection is answered, closed or past the 2 s and 64 KiB limits. Cancellation and the
    /// timeout are read every 20 ms; a timeout past what the clock can hold waits for the
    /// redirect or the cancellation. The listener closes when this returns.
    pub fn receive(
        self,
        authorization: &Authorization,
        timeout: Duration,
        cancellation: &Cancellation,
    ) -> Result<AuthorizationCode, Failure> {
        let deadline = Instant::now().checked_add(timeout);
        let mut loopback = Loopback {
            listener: self.listener,
            pending: Vec::new(),
        };
        loop {
            if cancellation.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Err(Failure::Timeout);
            }
            let mut decided = None;
            for (stream, line) in loopback.poll()? {
                if decided.is_some() {
                    answer(stream, Answer::Refused);
                } else if request_path(&line) != Some(self.path.as_str()) {
                    answer(stream, Answer::NotFound);
                } else {
                    let outcome = redirect(&line, authorization);
                    answer(
                        stream,
                        if outcome.is_ok() {
                            Answer::Completed
                        } else {
                            Answer::Refused
                        },
                    );
                    decided = Some(outcome);
                }
            }
            match decided {
                Some(outcome) => {
                    loopback.drain();
                    return outcome;
                }
                None => std::thread::sleep(POLL),
            }
        }
    }
}

/// The code of a pasted `code#state` line for `authorization`. Surrounding whitespace is
/// ignored; a line without the flow's `state` after its last `#` is
/// [`Failure::StateMismatch`], and an empty code or one with whitespace
/// [`Failure::InvalidResponse`].
pub fn pasted_code(
    authorization: &Authorization,
    line: &str,
) -> Result<AuthorizationCode, Failure> {
    let line = line.trim();
    let Some((code, state)) = line.rsplit_once('#') else {
        return Err(Failure::StateMismatch);
    };
    if state.is_empty() || !authorization.state_matches(state) {
        return Err(Failure::StateMismatch);
    }
    if code.is_empty() || !code.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(Failure::InvalidResponse);
    }
    Ok(AuthorizationCode(Secret::new(code.to_owned())))
}

/// A connection that has not yet completed its request head. Only the request line is kept;
/// the headers after it are counted and scanned for the end of the head, never stored.
struct Pending {
    stream: TcpStream,
    line: Zeroizing<Vec<u8>>,
    /// Bytes of the head read so far, request line included.
    seen: usize,
    /// Whether the request line's CRLF has been read.
    line_done: bool,
    /// How much of `\r\n\r\n` the head currently ends with.
    matched: usize,
    accepted: Instant,
}

impl Pending {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            line: Zeroizing::new(Vec::with_capacity(1024)),
            seen: 0,
            line_done: false,
            matched: 0,
            accepted: Instant::now(),
        }
    }

    /// Takes `bytes` of the head; answers whether the head is now complete.
    fn take(&mut self, bytes: &[u8]) -> bool {
        const END: &[u8; 4] = b"\r\n\r\n";
        for &byte in bytes {
            self.seen += 1;
            self.matched = if byte == END[self.matched] {
                self.matched + 1
            } else if byte == b'\r' {
                1
            } else {
                0
            };
            if !self.line_done {
                if self.matched == 2 {
                    // The CR before this LF was pushed; it is not the line's.
                    self.line.pop();
                    self.line_done = true;
                } else {
                    push(&mut self.line, byte);
                }
            }
            if self.matched == END.len() {
                return true;
            }
        }
        false
    }
}

/// Appends without leaving an unzeroized copy: a full buffer moves to a larger one and the old
/// one is zeroized as it drops.
fn push(buffer: &mut Zeroizing<Vec<u8>>, byte: u8) {
    if buffer.len() == buffer.capacity() {
        let mut larger = Zeroizing::new(Vec::with_capacity(buffer.capacity().max(512) * 2));
        larger.extend_from_slice(buffer);
        *buffer = larger;
    }
    buffer.push(byte);
}

/// A connection whose request head is complete, with its request line.
type Complete = (TcpStream, Zeroizing<Vec<u8>>);

/// The nonblocking listener and the connections it is reading, so no connection can hold up
/// another.
struct Loopback {
    listener: TcpListener,
    pending: Vec<Pending>,
}

impl Loopback {
    /// Reads what has arrived, accepts what is queued (closing what finds no free slot), and
    /// reads the new connections. Returns the request line of every connection whose head is
    /// complete, in the order the connections were accepted.
    fn poll(&mut self) -> Result<Vec<Complete>, Failure> {
        let mut complete = read(&mut self.pending);
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if self.pending.len() < PENDING_LIMIT && stream.set_nonblocking(true).is_ok() {
                        self.pending.push(Pending::new(stream));
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::Interrupted
                            | ErrorKind::ConnectionAborted
                            | ErrorKind::ConnectionReset
                    ) => {}
                Err(_) => return Err(Failure::Unavailable),
            }
        }
        complete.extend(read(&mut self.pending));
        Ok(complete)
    }

    /// Closes the listener, and answers 400 to each connection still reading its head once
    /// the head completes, on a thread of its own. The thread accepts nothing, and ends when the
    /// last such connection is answered, closes, or reaches the idle or head limit: at most 2 s
    /// after the decision.
    fn drain(self) {
        let Self {
            listener,
            mut pending,
        } = self;
        drop(listener);
        if pending.is_empty() {
            return;
        }
        // A thread that cannot be spawned leaves these connections unanswered, as before.
        let _ = std::thread::Builder::new().spawn(move || {
            loop {
                for (stream, _) in read(&mut pending) {
                    answer(stream, Answer::Refused);
                }
                if pending.is_empty() {
                    return;
                }
                std::thread::sleep(POLL);
            }
        });
    }
}

/// Reads what has arrived on each connection in `pending`. Returns those whose head is now
/// complete and keeps those still open, under the head limit and within the idle limit.
fn read(pending: &mut Vec<Pending>) -> Vec<Complete> {
    let mut complete = Vec::new();
    let mut chunk = Zeroizing::new([0_u8; 4096]);
    for mut connection in std::mem::take(pending) {
        let (mut open, mut done) = (true, false);
        while open && !done && connection.seen < HEAD_LIMIT {
            let room = (HEAD_LIMIT - connection.seen).min(chunk.len());
            match connection.stream.read(&mut chunk[..room]) {
                Ok(0) => open = false,
                Ok(read) => done = connection.take(&chunk[..read]),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => open = false,
            }
        }
        if done {
            complete.push((connection.stream, connection.line));
        } else if open && connection.seen < HEAD_LIMIT && connection.accepted.elapsed() < IDLE {
            pending.push(connection);
        }
    }
    complete
}

/// The path of a request line, without its query.
fn request_path(line: &[u8]) -> Option<&str> {
    let target = std::str::from_utf8(line).ok()?.split(' ').nth(1)?;
    Some(target.split_once('?').map_or(target, |(path, _)| path))
}

/// The code of a redirect request line on the receiver's path, for `authorization`.
fn redirect(line: &[u8], authorization: &Authorization) -> Result<AuthorizationCode, Failure> {
    let malformed = Failure::InvalidResponse;
    let line = std::str::from_utf8(line).map_err(|_| malformed)?;
    let mut words = line.split(' ');
    let (Some("GET"), Some(target), Some(version), None) =
        (words.next(), words.next(), words.next(), words.next())
    else {
        return Err(malformed);
    };
    if !version.starts_with("HTTP/1.") {
        return Err(malformed);
    }
    let query = target.split_once('?').map_or("", |(_, query)| query);
    let (mut code, mut state, mut error) = (None, None, None);
    for (name, value) in url::form_urlencoded::parse(query.as_bytes()) {
        let slot = match name.as_ref() {
            "code" => &mut code,
            "state" => &mut state,
            "error" => &mut error,
            _ => continue,
        };
        if slot.replace(Secret::new(value.into_owned())).is_some() {
            return Err(malformed);
        }
    }
    if !state.is_some_and(|state| authorization.state_matches(state.expose())) {
        return Err(Failure::StateMismatch);
    }
    if let Some(error) = error {
        return Err(if error.expose() == "access_denied" {
            Failure::AccessDenied
        } else {
            Failure::Rejected
        });
    }
    match code {
        Some(code) if !code.expose().is_empty() => Ok(AuthorizationCode(code)),
        _ => Err(malformed),
    }
}

enum Answer {
    Completed,
    Refused,
    NotFound,
}

/// Tells the browser how its request ended. The answer is not the flow's result.
fn answer(mut stream: TcpStream, answer: Answer) {
    let (status, body) = match answer {
        Answer::Completed => (
            "200 OK",
            "Authorization received. You can close this window and return to the terminal.",
        ),
        Answer::Refused => (
            "400 Bad Request",
            "Authorization was not completed. Return to the terminal.",
        ),
        Answer::NotFound => ("404 Not Found", "Not found."),
    };
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
}
