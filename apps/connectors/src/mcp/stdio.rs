//! The stdio serve loop: nonblocking standard descriptors polled with a short
//! finite wait, and a data lease on the boot clock. Linux only: the lease
//! clock is `CLOCK_BOOTTIME` (`session.rs`).
use super::*;
use connectors_mcp::{
    framing::LineDecoder,
    protocol::{self, Route},
    results,
};
use serde_json::Value;
use std::{
    collections::VecDeque,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

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
/// Owner calls a session holds at once: the request slot's and those of
/// cancelled calls still running out at the owner, which has no cancellation of
/// its own. A cancel frees the slot (server semantics § 3.4); this bounds what
/// repeated cancels can leave running.
const OWNER_CALLS: usize = 2;
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
pub(super) fn serve(
    paths: Paths,
    signals: &connectors_host::local::protected::Signals,
) -> owner::Result<()> {
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
        None,
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
    // Cancelled calls whose owner work is still running: no reply is awaited
    // from them, they hold no request slot, and teardown joins them.
    let mut cancelled_work = Vec::<Work>::new();
    // Set at the first terminal: no request reaches the owner after it
    // (sessions § 4.1).
    let admission_closed = Arc::new(AtomicBool::new(false));
    let mut frame_until = None::<Instant>;
    let mut closing = false;
    // Standard input reached end-of-file: nothing more is read, and the
    // replies to what was already read are still written under the live lease.
    let mut hangup = false;
    let result = (|| -> owner::Result<()> {
        loop {
            if signals.interrupted() {
                return Err(Code::Interrupted.into());
            }
            session.maintain(&paths)?;
            let (ended, running) = std::mem::take(&mut cancelled_work)
                .into_iter()
                .partition::<Vec<_>, _>(|job| job.thread.is_finished());
            cancelled_work = running;
            for job in ended {
                job.thread.join().map_err(|_| Code::Unavailable)?;
            }
            if let Some(job) = &work {
                match job.reply.try_recv() {
                    Ok(reply) => {
                        let job = work.take().ok_or(Code::Unavailable)?;
                        job.thread.join().map_err(|_| Code::Unavailable)?;
                        if Instant::now() >= job.until {
                            return Err(Code::Timeout.into());
                        }
                        enqueue(&mut queue, Some(&job.id), reply, job.until, response_limit)?;
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
            // A cancelled request is answered with nothing, so it holds no
            // reply back after a hangup; teardown still joins its worker.
            if hangup && queue.is_empty() && work.is_none() {
                return Ok(());
            }
            if frame_until.is_some_and(|until| Instant::now() >= until) {
                return Err(Code::Timeout.into());
            }
            let reading = !closing && !hangup;
            let mut polls = [
                libc::pollfd {
                    fd: if reading { 0 } else { -1 },
                    events: libc::POLLIN,
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
            if !reading || polls[0].revents & (libc::POLLIN | libc::POLLHUP) == 0 {
                continue;
            }
            session.permit("stdin")?;
            let mut bytes = [0u8; 4096];
            // SAFETY: bytes is a live writable buffer for this nonblocking read.
            let read = unsafe { libc::read(0, bytes.as_mut_ptr().cast(), bytes.len()) };
            if read == 0 {
                // The graceful shutdown signal (server semantics § 3.5). A
                // partial frame is discarded; complete ones were already read.
                // Local admission stops here: a request read but not yet at
                // the owner is refused, one at the owner keeps its reply.
                decoder.finish();
                frame_until = None;
                admission_closed.store(true, Ordering::SeqCst);
                session.hang_up()?;
                hangup = true;
                continue;
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
                            // The cancelled call frees its slot at once; its
                            // owner work runs out apart (server semantics § 3.4).
                            if work.as_ref().is_some_and(|job| &job.id == cancelled) {
                                let job = work.take().ok_or(Code::Unavailable)?;
                                job.cancelled.store(true, Ordering::SeqCst);
                                cancelled_work.push(job);
                            }
                            // A reply not yet begun is withdrawn. One already
                            // being written finishes, so stdout never holds a
                            // cut-off frame: that race is expected, not an
                            // error, and the session stays `Ready` (§ 3.4).
                            queue.retain(|q| q.id.as_ref() != Some(cancelled) || q.offset > 0);
                        }
                    }
                    Ok(Route::Reply { result, close }) => {
                        enqueue(&mut queue, id, Ok(result), until, response_limit)?;
                        closing = close;
                    }
                    Ok(Route::Work { primary }) => {
                        if work.is_some()
                            || cancelled_work.len() >= OWNER_CALLS
                            || queue.iter().any(|q| q.id.as_ref() == id)
                        {
                            enqueue(
                                &mut queue,
                                id,
                                Err(super::projection::refusal("capacity")),
                                until,
                                response_limit,
                            )?;
                            continue;
                        }
                        // The request's own budget bounds it and its reply,
                        // not the lease left at dispatch: renewal keeps a long
                        // request alive, and an unrenewed lease still ends
                        // the session (server semantics § 3.3).
                        let id = id.cloned().ok_or(Code::InvalidInput)?;
                        let cancelled = Arc::new(AtomicBool::new(false));
                        let halt = super::projection::Halt {
                            cancelled: cancelled.clone(),
                            admission_closed: admission_closed.clone(),
                        };
                        let paths = paths.clone();
                        let host = host.clone();
                        let (sender, reply) = mpsc::sync_channel(1);
                        let thread = thread::spawn(move || {
                            let result = super::projection::execute(
                                &paths, &host, &value, primary, until, &halt,
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
    admission_closed.store(true, Ordering::SeqCst);
    let until = session.begin_close(result.is_ok());
    let mut released = true;
    for job in work.into_iter().chain(cancelled_work) {
        job.cancelled.store(true, Ordering::SeqCst);
        while !job.thread.is_finished() && Instant::now() < until {
            thread::sleep(Duration::from_millis(5));
        }
        released &= job.thread.is_finished() && job.thread.join().is_ok();
    }
    session.finish_teardown(released);
    if released {
        result
    } else {
        Err(Code::Unavailable.into())
    }
}
