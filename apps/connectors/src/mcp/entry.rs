//! Production stdio composition. Only complete MCP frames cross stdout.
use super::*;
use clap::{CommandFactory, FromArgMatches, Parser};
use connectors_mcp::{
    framing::LineDecoder,
    protocol::{self, Route},
    results,
};
use serde_json::Value;
use std::{
    collections::VecDeque,
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(name = "connectors", disable_help_subcommand = true)]
struct Selectors {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long = "state-dir", global = true)]
    state: Option<PathBuf>,
}
pub(crate) fn selected(args: &[OsString]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--config" | "--state-dir" | "--output") => {
                rest.next();
            }
            Some(a)
                if a.starts_with("--config=")
                    || a.starts_with("--state-dir=")
                    || a.starts_with("--output=") => {}
            Some("server") => return true,
            _ => return false,
        }
    }
    false
}
pub(crate) fn run(args: Vec<OsString>) -> i32 {
    let result = (|| {
        let matches = match Selectors::command()
            .subcommand_required(true)
            .subcommand(connectors_mcp::launch::command())
            .try_get_matches_from(args)
        {
            Ok(matches) => matches,
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
                io::stdout()
                    .write_all(error.to_string().as_bytes())
                    .map_err(|_| ())?;
                return Ok(());
            }
            Err(_) => return Err(()),
        };
        let selectors = Selectors::from_arg_matches(&matches).map_err(|_| ())?;
        connectors_mcp::launch::input(matches.subcommand_matches("server").ok_or(())?)
            .map_err(|_| ())?;
        let paths = Paths::resolve(selectors.config.as_deref(), selectors.state.as_deref())
            .map_err(|_| ())?;
        let signals = connectors_host::local::protected::Signals::install().map_err(|_| ())?;
        serve(paths, &signals).map_err(|_| ())
    })();
    if result.is_ok() {
        0
    } else {
        let _ = writeln!(io::stderr(), "mcp_unavailable");
        1
    }
}

struct Nonblocking {
    fd: i32,
    flags: i32,
}
impl Nonblocking {
    fn new(fd: i32) -> io::Result<Self> {
        // SAFETY: fcntl validates this process's standard descriptor.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd, flags })
    }
}
impl Drop for Nonblocking {
    fn drop(&mut self) {
        // SAFETY: restore only the flags read from this descriptor.
        unsafe { libc::fcntl(self.fd, libc::F_SETFL, self.flags) };
    }
}
struct Output {
    id: Option<Value>,
    bytes: Vec<u8>,
    offset: usize,
    until: Instant,
}
struct Work {
    id: Value,
    cancelled: Arc<AtomicBool>,
    thread: JoinHandle<()>,
    reply: mpsc::Receiver<Result<Value, Value>>,
    until: Instant,
}
fn enqueue(
    queue: &mut VecDeque<Output>,
    id: Option<&Value>,
    reply: Result<Value, Value>,
    until: Instant,
    limit: usize,
) -> owner::Result<()> {
    let encoded = match reply {
        Ok(value) => results::encode_result(id.ok_or(Code::Unavailable)?, value, limit),
        Err(error) => results::encode_error(id, error, limit),
    };
    let bytes = encoded
        .or_else(|_| results::encode_error(id, super::projection::refusal("capacity"), limit))
        .map_err(|_| Code::Capacity)?;
    if queue
        .iter()
        .map(|q| q.bytes.len() - q.offset)
        .sum::<usize>()
        .saturating_add(bytes.len())
        > limit
    {
        return Err(Code::Capacity.into());
    }
    queue.push_back(Output {
        id: id.cloned(),
        bytes,
        offset: 0,
        until,
    });
    Ok(())
}
fn serve(paths: Paths, signals: &connectors_host::local::protected::Signals) -> owner::Result<()> {
    let start = Instant::now();
    let config = load(&paths)?;
    let limits =
        serde_json::to_value(&config.document().limits).map_err(|_| Code::InvalidConfiguration)?;
    let frame_limit = limits["frame_octets"]
        .as_u64()
        .ok_or(Code::InvalidConfiguration)? as usize;
    let response_limit = limits["response_octets"]
        .as_u64()
        .ok_or(Code::InvalidConfiguration)? as usize;
    let request_time = Duration::from_millis(
        limits["request_milliseconds"]
            .as_u64()
            .ok_or(Code::InvalidConfiguration)?,
    );
    let snapshot = super::projection::snapshot(
        &paths,
        None,
        start + request_time.min(Duration::from_secs(10)),
    )
    .map_err(|_| Code::Unavailable)?;
    let host = snapshot.host.clone();
    let mut session = super::session::Session::admit(&paths, &snapshot)?;
    let paths = Arc::new(paths);
    let _input = Nonblocking::new(0).map_err(|_| Code::Unavailable)?;
    let _output = Nonblocking::new(1).map_err(|_| Code::Unavailable)?;
    let mut decoder = LineDecoder::new(frame_limit).map_err(|_| Code::InvalidConfiguration)?;
    let mut protocol = protocol::Protocol::default();
    let mut queue = VecDeque::<Output>::new();
    let mut work = None::<Work>;
    let mut frame_until = None::<Instant>;
    let mut closing = false;
    let result = (|| -> owner::Result<()> {
        loop {
            if signals.interrupted() {
                return Err(Code::Interrupted.into());
            }
            session.maintain(&paths)?;
            if let Some(job) = &work {
                match job.reply.try_recv() {
                    Ok(reply) => {
                        let job = work.take().ok_or(Code::Unavailable)?;
                        job.thread.join().map_err(|_| Code::Unavailable)?;
                        if !job.cancelled.load(Ordering::SeqCst) {
                            if Instant::now() >= job.until {
                                return Err(Code::Timeout.into());
                            }
                            enqueue(&mut queue, Some(&job.id), reply, job.until, response_limit)?;
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => return Err(Code::Unavailable.into()),
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            if let Some(output) = queue.front_mut() {
                if Instant::now() >= output.until {
                    return Err(Code::Timeout.into());
                }
                session.permit("stdout")?;
                let bytes = &output.bytes[output.offset..];
                // SAFETY: bytes is live for the bounded, nonblocking write.
                let written =
                    unsafe { libc::write(1, bytes.as_ptr().cast(), bytes.len().min(4096)) };
                if written > 0 {
                    output.offset += written as usize;
                    if output.offset == output.bytes.len() {
                        queue.pop_front();
                    }
                } else if written == 0
                    || !matches!(
                        io::Error::last_os_error().kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    )
                {
                    return Err(Code::Unavailable.into());
                }
            }
            if closing && queue.is_empty() {
                return Ok(());
            }
            if frame_until.is_some_and(|until| Instant::now() >= until) {
                return Err(Code::Timeout.into());
            }
            let mut polls = [
                libc::pollfd {
                    fd: 0,
                    events: if closing { 0 } else { libc::POLLIN },
                    revents: 0,
                },
                libc::pollfd {
                    fd: 1,
                    events: if queue.is_empty() { 0 } else { libc::POLLOUT },
                    revents: 0,
                },
            ];
            // SAFETY: polls names a live two-entry array, with a short finite wait.
            if unsafe { libc::poll(polls.as_mut_ptr(), 2, 10) } < 0 {
                if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(Code::Unavailable.into());
            }
            if polls
                .iter()
                .any(|p| p.revents & (libc::POLLERR | libc::POLLNVAL) != 0)
            {
                return Err(Code::Unavailable.into());
            }
            if closing || polls[0].revents & (libc::POLLIN | libc::POLLHUP) == 0 {
                continue;
            }
            session.permit("stdin")?;
            let mut bytes = [0u8; 4096];
            // SAFETY: bytes is a live writable buffer for this nonblocking read.
            let read = unsafe { libc::read(0, bytes.as_mut_ptr().cast(), bytes.len()) };
            if read == 0 {
                decoder.finish();
                return Ok(());
            }
            if read < 0 {
                if matches!(
                    io::Error::last_os_error().kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) {
                    continue;
                }
                return Err(Code::Unavailable.into());
            }
            let mut pending = &bytes[..read as usize];
            while !pending.is_empty() {
                session.permit("stdin")?;
                let until = *frame_until.get_or_insert_with(|| Instant::now() + request_time);
                let (consumed, frame) = decoder.feed(pending);
                pending = &pending[consumed..];
                let Some(frame) = frame else {
                    continue;
                };
                frame_until = None;
                let value = frame
                    .ok()
                    .and_then(|bytes| connectors_mcp::json::decode(&bytes, 72).ok());
                let Some(value) = value else {
                    enqueue(
                        &mut queue,
                        None,
                        Err(protocol::error(-32700, "Parse error")),
                        until,
                        response_limit,
                    )?;
                    continue;
                };
                let id = protocol::request_id(&value);
                match protocol.prepare(&value) {
                    Err(error) => enqueue(&mut queue, id, Err(error), until, response_limit)?,
                    Ok(Route::Notification) => {
                        if value["method"] == "notifications/cancelled" {
                            let cancelled = &value["params"]["requestId"];
                            if let Some(job) = &work
                                && &job.id == cancelled
                            {
                                job.cancelled.store(true, Ordering::SeqCst);
                            }
                            if queue
                                .iter()
                                .any(|q| q.id.as_ref() == Some(cancelled) && q.offset > 0)
                            {
                                return Err(Code::Interrupted.into());
                            }
                            queue.retain(|q| q.id.as_ref() != Some(cancelled));
                        }
                    }
                    Ok(Route::Reply { result, close }) => {
                        enqueue(&mut queue, id, Ok(result), until, response_limit)?;
                        closing = close;
                    }
                    Ok(Route::Work { primary }) => {
                        if work.is_some() || queue.iter().any(|q| q.id.as_ref() == id) {
                            enqueue(
                                &mut queue,
                                id,
                                Err(super::projection::refusal("capacity")),
                                until,
                                response_limit,
                            )?;
                            continue;
                        }
                        let until = until.min(session.until()?);
                        let id = id.cloned().ok_or(Code::InvalidInput)?;
                        let cancelled = Arc::new(AtomicBool::new(false));
                        let flag = cancelled.clone();
                        let paths = paths.clone();
                        let host = host.clone();
                        let (sender, reply) = mpsc::sync_channel(1);
                        let thread = thread::spawn(move || {
                            let result = super::projection::execute(
                                &paths, &host, &value, primary, until, &flag,
                            );
                            let _ = sender.send(result);
                        });
                        work = Some(Work {
                            id,
                            cancelled,
                            thread,
                            reply,
                            until,
                        });
                    }
                }
                if closing {
                    break;
                }
            }
        }
    })();
    queue.clear();
    session.begin_close(result.is_ok());
    let released = if let Some(job) = work {
        job.cancelled.store(true, Ordering::SeqCst);
        let until = Instant::now() + Duration::from_secs(5);
        while !job.thread.is_finished() && Instant::now() < until {
            thread::sleep(Duration::from_millis(5));
        }
        job.thread.is_finished() && job.thread.join().is_ok()
    } else {
        true
    };
    session.finish_teardown(released);
    if released {
        result
    } else {
        Err(Code::Unavailable.into())
    }
}
