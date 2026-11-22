//! The serve loop's standard descriptors, one bounded step at a time. Unix polls
//! nonblocking descriptors; Windows has no nonblocking anonymous pipe, so a reader
//! thread and a writer thread stand between the loop and the handles. Either way
//! the loop never blocks past its short wait, and a write is reported only once
//! it has been made.
use std::io;

/// Whether the session's standard input has been closed by its peer, asked from
/// a worker thread at admission without waiting for the serve loop's own read.
#[derive(Clone)]
pub(super) struct Hangup {
    #[cfg(unix)]
    fd: i32,
    #[cfg(windows)]
    ended: threaded::Ended,
}

#[cfg(unix)]
pub(super) use unix::Console;
#[cfg(windows)]
pub(super) use windows::Console;

#[cfg(unix)]
mod unix {
    use super::*;

    struct Nonblocking {
        fd: i32,
        flags: i32,
    }
    impl Nonblocking {
        fn new(fd: i32) -> io::Result<Self> {
            // SAFETY: fcntl validates this process's standard descriptor.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
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

    pub(in crate::mcp) struct Console {
        _input: Nonblocking,
        _output: Nonblocking,
    }
    impl Console {
        pub fn open() -> io::Result<Self> {
            Ok(Self {
                _input: Nonblocking::new(0)?,
                _output: Nonblocking::new(1)?,
            })
        }
        pub fn hangup(&self) -> Hangup {
            Hangup { fd: 0 }
        }
        /// Waits up to 10 ms for standard input to be readable (when `reading`)
        /// or standard output to be writable (when `writing`). Whether input
        /// has something to read, or has ended.
        pub fn wait(&mut self, reading: bool, writing: bool) -> io::Result<bool> {
            let mut polls = [
                libc::pollfd {
                    fd: if reading { 0 } else { -1 },
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: 1,
                    events: if writing { libc::POLLOUT } else { 0 },
                    revents: 0,
                },
            ];
            // SAFETY: polls names a live two-entry array, with a short finite wait.
            if unsafe { libc::poll(polls.as_mut_ptr(), 2, 10) } < 0 {
                return match io::Error::last_os_error().kind() {
                    io::ErrorKind::Interrupted => Ok(false),
                    kind => Err(kind.into()),
                };
            }
            if polls
                .iter()
                .any(|p| p.revents & (libc::POLLERR | libc::POLLNVAL) != 0)
            {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            Ok(reading && polls[0].revents & (libc::POLLIN | libc::POLLHUP) != 0)
        }
        /// `Ok(0)` is the end of input; `WouldBlock` and `Interrupted` mean nothing yet.
        pub fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            // SAFETY: bytes is a live writable buffer for this nonblocking read.
            let read = unsafe { libc::read(0, bytes.as_mut_ptr().cast(), bytes.len()) };
            usize::try_from(read).map_err(|_| io::Error::last_os_error())
        }
        /// Bytes written from the front of `bytes`; `WouldBlock` and
        /// `Interrupted` mean none yet.
        pub fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            // SAFETY: bytes is live for the bounded, nonblocking write.
            let written = unsafe { libc::write(1, bytes.as_ptr().cast(), bytes.len().min(4096)) };
            match usize::try_from(written) {
                Ok(0) => Err(io::ErrorKind::WriteZero.into()),
                Ok(written) => Ok(written),
                Err(_) => Err(io::Error::last_os_error()),
            }
        }
        /// Whether a write has begun that a later call finishes. A unix write
        /// is whole or none at all, so never.
        pub fn write_begun(&self) -> bool {
            false
        }
    }
    impl Hangup {
        #[cfg(test)]
        pub fn of(fd: i32) -> Self {
            Self { fd }
        }
        /// Whether the peer has closed input, buffered input or not. A poll
        /// that fails answers no, which leaves the end of file to the loop.
        pub fn hung_up(&self) -> bool {
            let mut poll = libc::pollfd {
                fd: self.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: poll names one live entry and does not wait.
            let ready = unsafe { libc::poll(&mut poll, 1, 0) };
            ready > 0 && poll.revents & libc::POLLHUP != 0
        }
    }
}

#[cfg(any(windows, test))]
mod threaded {
    use std::{
        io::{self, Read, Write},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::Duration,
    };

    enum Input {
        Data(Vec<u8>),
        End,
        Failed(io::ErrorKind),
    }
    /// What the two threads tell the loop, in one order, on one channel, so the
    /// loop waits on both at once and never sleeps for either.
    enum Event {
        Input(Input),
        Written(io::Result<()>),
    }
    /// Whether the reader thread has seen the end of input.
    #[derive(Clone)]
    pub(super) struct Ended(Arc<AtomicBool>);
    impl Ended {
        pub fn hung_up(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
    }
    pub(super) struct Threaded {
        events: mpsc::Receiver<Event>,
        ready: Option<Input>,
        carry: Vec<u8>,
        ended: Ended,
        writer: mpsc::Sender<Vec<u8>>,
        finished: Option<io::Result<()>>,
        inflight: Option<usize>,
    }
    impl Threaded {
        pub fn new(
            mut input: impl Read + Send + 'static,
            mut output: impl Write + Send + 'static,
        ) -> io::Result<Self> {
            let ended = Ended(Arc::new(AtomicBool::new(false)));
            let (sender, events) = mpsc::sync_channel(8);
            let flag = ended.clone();
            let reads = sender.clone();
            thread::Builder::new().name("stdin".into()).spawn(move || {
                let mut bytes = [0u8; 4096];
                loop {
                    let message = match input.read(&mut bytes) {
                        Ok(0) => Input::End,
                        Ok(read) => Input::Data(bytes[..read].to_vec()),
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) => Input::Failed(error.kind()),
                    };
                    let last = !matches!(message, Input::Data(_));
                    if last {
                        flag.0.store(true, Ordering::SeqCst);
                    }
                    if reads.send(Event::Input(message)).is_err() || last {
                        break;
                    }
                }
            })?;
            let (writer, frames) = mpsc::channel::<Vec<u8>>();
            thread::Builder::new()
                .name("stdout".into())
                .spawn(move || {
                    for frame in frames {
                        let result = output.write_all(&frame).and_then(|()| output.flush());
                        if sender.send(Event::Written(result)).is_err() {
                            break;
                        }
                    }
                })?;
            Ok(Self {
                events,
                ready: None,
                carry: Vec::new(),
                ended,
                writer,
                finished: None,
                inflight: None,
            })
        }
        pub fn hangup(&self) -> Ended {
            self.ended.clone()
        }
        fn take(&mut self, event: Event, reading: bool) {
            match event {
                Event::Input(input) if reading => self.ready = Some(input),
                // Input nobody reads any more: the end was already flagged.
                Event::Input(_) => {}
                Event::Written(result) => self.finished = Some(result),
            }
        }
        /// Waits up to 10 ms for input or for the write in flight to finish,
        /// whichever comes first. Whether input has something to read, or has
        /// ended.
        pub fn wait(&mut self, reading: bool, _writing: bool) -> io::Result<bool> {
            if reading && (self.ready.is_some() || !self.carry.is_empty()) {
                return Ok(true);
            }
            if self.finished.is_none() {
                match self.events.recv_timeout(Duration::from_millis(10)) {
                    Ok(event) => self.take(event, reading),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        if reading {
                            self.ready = Some(Input::Failed(io::ErrorKind::BrokenPipe));
                        }
                        if self.inflight.is_some() {
                            self.finished = Some(Err(io::ErrorKind::BrokenPipe.into()));
                        }
                    }
                }
            }
            Ok(reading && (self.ready.is_some() || !self.carry.is_empty()))
        }
        /// `Ok(0)` is the end of input; `WouldBlock` means nothing yet.
        pub fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if self.carry.is_empty() {
                let message = loop {
                    if let Some(message) = self.ready.take() {
                        break message;
                    }
                    match self.events.try_recv() {
                        Ok(event) => self.take(event, true),
                        Err(mpsc::TryRecvError::Empty) => {
                            return Err(io::ErrorKind::WouldBlock.into());
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            return Err(io::ErrorKind::BrokenPipe.into());
                        }
                    }
                };
                match message {
                    Input::Data(data) => self.carry = data,
                    Input::End => return Ok(0),
                    Input::Failed(kind) => return Err(kind.into()),
                }
            }
            let count = self.carry.len().min(bytes.len());
            bytes[..count].copy_from_slice(&self.carry[..count]);
            self.carry.drain(..count);
            Ok(count)
        }
        /// Hands all of `bytes`, the frame's remainder, to the writer thread
        /// and reports it written only once the thread has finished it, so
        /// the loop never counts a frame it has not yet sent. `WouldBlock`
        /// while it is in flight; the next call is the same frame's, and
        /// `wait` returns as soon as the thread reports.
        pub fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.inflight.is_none() {
                self.writer
                    .send(bytes.to_vec())
                    .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
                self.inflight = Some(bytes.len());
            }
            let Some(result) = self.finished.take() else {
                return Err(io::ErrorKind::WouldBlock.into());
            };
            let count = self.inflight.take().unwrap_or_default();
            result.map(|()| count)
        }
        /// Whether the front frame is with the writer thread: it can no longer
        /// be withdrawn, as a frame already begun cannot.
        #[cfg_attr(not(windows), allow(dead_code))]
        pub fn write_begun(&self) -> bool {
            self.inflight.is_some()
        }
    }

    /// F2 of the adversary's review: a 4 MiB reply to a peer that reads it at
    /// once is written in well under a second, not 4 KiB per two loop passes.
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::Mutex;

        /// The peer: reads every byte as soon as it is offered, each write
        /// call costing what a pipe write to a reading peer costs.
        struct Sink(Arc<Mutex<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                thread::sleep(Duration::from_micros(100));
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        /// An input that never ends and never has a byte: the peer holds
        /// standard input open, as the serve loop's reading case.
        struct Silent(mpsc::Receiver<()>);
        impl Read for Silent {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                let _ = self.0.recv();
                Ok(0)
            }
        }

        fn drain(reading: bool) -> Duration {
            let written = Arc::new(Mutex::new(Vec::new()));
            let (hold, silent) = mpsc::channel();
            let mut console = Threaded::new(Silent(silent), Sink(written.clone())).unwrap();
            let frame = vec![b'x'; 4 << 20];
            let begun = std::time::Instant::now();
            let mut offset = 0;
            while offset < frame.len() {
                match console.write(&frame[offset..]) {
                    Ok(count) => offset += count,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        console.wait(reading, true).unwrap();
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            let took = begun.elapsed();
            assert_eq!(written.lock().unwrap().len(), frame.len());
            assert!(written.lock().unwrap().iter().all(|byte| *byte == b'x'));
            drop(hold);
            took
        }

        #[test]
        fn a_large_reply_is_written_without_waiting_per_chunk_while_input_is_open() {
            let took = drain(true);
            assert!(took < Duration::from_millis(1000), "took {took:?}");
        }

        #[test]
        fn a_large_reply_is_written_without_waiting_per_chunk_after_hangup() {
            let took = drain(false);
            assert!(took < Duration::from_millis(1000), "took {took:?}");
        }

        /// Input bytes arrive whole and in order, and the end of input is seen
        /// by the hangup probe before the loop reads it.
        #[test]
        fn input_arrives_in_order_and_its_end_is_visible_to_the_probe() {
            let mut console = Threaded::new(&b"abc\ndef\n"[..], Sink(Arc::default())).unwrap();
            let ended = console.hangup();
            let mut got = Vec::new();
            let mut bytes = [0u8; 3];
            let until = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                assert!(std::time::Instant::now() < until, "no end of input");
                if !console.wait(true, false).unwrap() {
                    continue;
                }
                match console.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(count) => got.extend_from_slice(&bytes[..count]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("{error}"),
                }
            }
            assert_eq!(got, b"abc\ndef\n");
            assert!(ended.hung_up());
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    pub(in crate::mcp) struct Console(threaded::Threaded);
    impl Console {
        pub fn open() -> io::Result<Self> {
            threaded::Threaded::new(io::stdin(), io::stdout()).map(Self)
        }
        pub fn hangup(&self) -> Hangup {
            Hangup {
                ended: self.0.hangup(),
            }
        }
        pub fn wait(&mut self, reading: bool, writing: bool) -> io::Result<bool> {
            self.0.wait(reading, writing)
        }
        pub fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.0.read(bytes)
        }
        pub fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.write(bytes)
        }
        pub fn write_begun(&self) -> bool {
            self.0.write_begun()
        }
    }
    impl Hangup {
        pub fn hung_up(&self) -> bool {
            self.ended.hung_up()
        }
    }
}
