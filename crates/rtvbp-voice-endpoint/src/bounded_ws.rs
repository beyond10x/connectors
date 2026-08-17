//! Bounded RTVBP-compatible semantic transport over an already established WebSocket.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use rtvbp::{
    ControlChannel, KeepalivePolicy, MediaChannel, MediaFormat, MediaFrame, Received, Transport,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, Mutex as AsyncMutex, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::WebSocketStream;

use crate::{AUDIO_CHANNEL, MAX_CONTROL_FRAME_BYTES};

/// All finite queues and deadlines for one transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds {
    /// Incoming complete control envelopes.
    pub incoming_control: usize,
    /// Outgoing complete control envelopes, including keepalive.
    pub outgoing_control: usize,
    /// Incoming application-to-call audio; the first dropped frame terminates as media overload.
    pub incoming_media: usize,
    /// Outgoing audio frames; saturation fails instead of silently dropping speech.
    pub outgoing_media: usize,
    /// Exact maximum control envelope bytes.
    pub control_frame_bytes: usize,
    /// Bounded close handshake deadline.
    pub close_deadline: Duration,
}

impl Bounds {
    /// Exact first binding limits.
    #[must_use]
    pub fn voice_v1() -> Self {
        Self {
            incoming_control: 16,
            outgoing_control: 16,
            incoming_media: 50,
            outgoing_media: 50,
            control_frame_bytes: MAX_CONTROL_FRAME_BYTES,
            close_deadline: Duration::from_secs(5),
        }
    }

    fn validate(&self) -> Result<(), rtvbp::Error> {
        if self.incoming_control == 0
            || self.outgoing_control == 0
            || self.incoming_media == 0
            || self.outgoing_media == 0
            || self.control_frame_bytes == 0
            || self.close_deadline.is_zero()
        {
            Err(rtvbp::Error::Configuration(
                "bounded WebSocket limits must all be positive".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

/// Measurable queue evidence for tests and runtime observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueEvidence {
    /// Current incoming controls.
    pub incoming_control: usize,
    /// Current incoming media frames.
    pub incoming_media: usize,
    /// Remaining outgoing control capacity.
    pub outgoing_control_remaining: usize,
    /// Remaining outgoing media capacity.
    pub outgoing_media_remaining: usize,
    /// Monotonic media frames dropped by this transport owner.
    pub incoming_media_loss: u64,
}

#[derive(Debug, Clone)]
enum Terminal {
    Orderly,
    Failed(String),
}

impl Terminal {
    fn result(self) -> Result<(), rtvbp::Error> {
        match self {
            Self::Orderly => Ok(()),
            Self::Failed(message) => Err(rtvbp::Error::Transport(message)),
        }
    }

    fn error(self) -> rtvbp::Error {
        match self {
            Self::Orderly => rtvbp::Error::Closed,
            Self::Failed(message) => rtvbp::Error::Transport(message),
        }
    }
}

#[derive(Debug)]
struct InboxState<T> {
    items: VecDeque<T>,
    terminal: Option<Terminal>,
}

#[derive(Debug)]
struct Inbox<T> {
    capacity: usize,
    state: Mutex<InboxState<T>>,
    ready: Notify,
}

impl<T> Inbox<T> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new(InboxState {
                items: VecDeque::with_capacity(capacity),
                terminal: None,
            }),
            ready: Notify::new(),
        }
    }

    fn push_strict(&self, item: T) -> Result<(), rtvbp::Error> {
        let mut state = lock(&self.state);
        if let Some(terminal) = state.terminal.clone() {
            return Err(terminal.error());
        }
        if state.items.len() == self.capacity {
            return Err(rtvbp::Error::Transport(
                "bounded incoming control queue overloaded".to_owned(),
            ));
        }
        state.items.push_back(item);
        drop(state);
        self.ready.notify_one();
        Ok(())
    }

    fn push_drop_oldest(&self, item: T) -> Result<bool, rtvbp::Error> {
        let mut state = lock(&self.state);
        if let Some(terminal) = state.terminal.clone() {
            return Err(terminal.error());
        }
        let dropped = if state.items.len() == self.capacity {
            state.items.pop_front();
            true
        } else {
            false
        };
        state.items.push_back(item);
        drop(state);
        self.ready.notify_one();
        Ok(dropped)
    }

    async fn pop(&self) -> Result<T, rtvbp::Error> {
        loop {
            let notified = self.ready.notified();
            {
                let mut state = lock(&self.state);
                if let Some(item) = state.items.pop_front() {
                    return Ok(item);
                }
                if let Some(terminal) = state.terminal.clone() {
                    return Err(terminal.error());
                }
            }
            notified.await;
        }
    }

    fn close(&self, terminal: Terminal) {
        let mut state = lock(&self.state);
        if state.terminal.is_none() {
            state.terminal = Some(terminal);
        }
        drop(state);
        self.ready.notify_waiters();
    }

    fn len(&self) -> usize {
        lock(&self.state).items.len()
    }
}

/// A finite RTVBP `Transport` that uses the classic text-control/binary-audio WebSocket mapping.
pub struct BoundedWsTransport {
    bounds: Bounds,
    format: MediaFormat,
    control: Arc<WsControl>,
    media: Arc<WsMedia>,
    outgoing_control: mpsc::Sender<Message>,
    outgoing_media: mpsc::Sender<Message>,
    terminal: Mutex<Option<Terminal>>,
    done: Arc<Notify>,
    closing: AtomicBool,
    close_requested: Arc<Notify>,
    media_claimed: AtomicBool,
    incoming_media_loss: AtomicU64,
    media_loss_ready: Notify,
    pongs: Arc<Inbox<Vec<u8>>>,
    keepalive_claimed: AtomicBool,
    ping_serial: AtomicU64,
    pumps: AsyncMutex<Option<PumpHandles>>,
}

struct PumpHandles {
    writer: JoinHandle<()>,
    reader: JoinHandle<()>,
}

impl BoundedWsTransport {
    /// Start bounded read/write pumps over an upgraded stream.
    pub fn start<S>(
        stream: WebSocketStream<S>,
        bounds: Bounds,
        format: MediaFormat,
    ) -> Result<Arc<Self>, rtvbp::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        bounds.validate()?;
        format.frame_bytes()?;
        let (control_tx, control_rx) = mpsc::channel(bounds.outgoing_control);
        let (media_tx, media_rx) = mpsc::channel(bounds.outgoing_media);
        let transport = Arc::new_cyclic(|weak| Self {
            bounds: bounds.clone(),
            format: format.clone(),
            control: Arc::new(WsControl {
                transport: weak.clone(),
                incoming: Arc::new(Inbox::new(bounds.incoming_control)),
            }),
            media: Arc::new(WsMedia {
                transport: weak.clone(),
                incoming: Arc::new(Inbox::new(bounds.incoming_media)),
                format,
                closed: AtomicBool::new(false),
            }),
            outgoing_control: control_tx,
            outgoing_media: media_tx,
            terminal: Mutex::new(None),
            done: Arc::new(Notify::new()),
            closing: AtomicBool::new(false),
            close_requested: Arc::new(Notify::new()),
            media_claimed: AtomicBool::new(false),
            incoming_media_loss: AtomicU64::new(0),
            media_loss_ready: Notify::new(),
            pongs: Arc::new(Inbox::new(1)),
            keepalive_claimed: AtomicBool::new(false),
            ping_serial: AtomicU64::new(0),
            pumps: AsyncMutex::new(None),
        });
        let (writer, reader) = stream.split();
        let writer = tokio::spawn(write_pump(
            Arc::downgrade(&transport),
            control_rx,
            media_rx,
            writer,
        ));
        let reader = tokio::spawn(read_pump(Arc::downgrade(&transport), reader));
        match transport.pumps.try_lock() {
            Ok(mut pumps) => *pumps = Some(PumpHandles { writer, reader }),
            Err(_) => {
                writer.abort();
                reader.abort();
                return Err(rtvbp::Error::Configuration(
                    "WebSocket pump ownership was unavailable during startup".to_owned(),
                ));
            }
        }
        Ok(transport)
    }

    /// Queue-depth and loss evidence without exposing payloads.
    #[must_use]
    pub fn evidence(&self) -> QueueEvidence {
        QueueEvidence {
            incoming_control: self.control.incoming.len(),
            incoming_media: self.media.incoming.len(),
            outgoing_control_remaining: self.outgoing_control.capacity(),
            outgoing_media_remaining: self.outgoing_media.capacity(),
            incoming_media_loss: self.incoming_media_loss.load(Ordering::Acquire),
        }
    }

    /// Wait for the first incoming media loss, reported directly by the owning read pump.
    pub async fn wait_incoming_media_loss(&self) -> u64 {
        loop {
            let notified = self.media_loss_ready.notified();
            let loss = self.incoming_media_loss.load(Ordering::Acquire);
            if loss > 0 {
                return loss;
            }
            notified.await;
        }
    }

    fn finish(&self, terminal: Terminal) {
        let first = {
            let mut state = lock(&self.terminal);
            if state.is_none() {
                *state = Some(terminal.clone());
                true
            } else {
                false
            }
        };
        if first {
            self.closing.store(true, Ordering::Release);
            self.control.incoming.close(terminal.clone());
            self.media.incoming.close(terminal.clone());
            self.pongs.close(terminal);
            self.close_requested.notify_waiters();
            self.done.notify_waiters();
        }
    }

    fn closed_error(&self) -> rtvbp::Error {
        lock(&self.terminal)
            .clone()
            .unwrap_or(Terminal::Orderly)
            .error()
    }

    async fn wait_closed(&self) -> Result<(), rtvbp::Error> {
        loop {
            let notified = self.done.notified();
            if let Some(terminal) = lock(&self.terminal).clone() {
                return terminal.result();
            }
            notified.await;
        }
    }

    async fn stop_and_join_pumps(&self, force_abort: bool) {
        let mut pumps = self.pumps.lock().await;
        let Some(handles) = pumps.as_mut() else {
            return;
        };
        if force_abort {
            handles.writer.abort();
            handles.reader.abort();
        }
        let joined = async {
            let _ = (&mut handles.writer).await;
            let _ = (&mut handles.reader).await;
        };
        if tokio::time::timeout(self.bounds.close_deadline, joined)
            .await
            .is_err()
        {
            handles.writer.abort();
            handles.reader.abort();
            let _ = tokio::time::timeout(self.bounds.close_deadline, async {
                let _ = (&mut handles.writer).await;
                let _ = (&mut handles.reader).await;
            })
            .await;
        }
        *pumps = None;
    }

    fn claim_media(&self) -> Result<Arc<dyn MediaChannel>, rtvbp::Error> {
        if self.closing.load(Ordering::Acquire) {
            return Err(self.closed_error());
        }
        if self
            .media_claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(rtvbp::Error::MediaAlreadyOpen);
        }
        Ok(Arc::clone(&self.media) as Arc<dyn MediaChannel>)
    }

    fn try_control(&self, message: Message) -> Result<(), rtvbp::Error> {
        if self.closing.load(Ordering::Acquire) {
            return Err(self.closed_error());
        }
        self.outgoing_control
            .try_send(message)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    rtvbp::Error::Transport("bounded outgoing control queue overloaded".to_owned())
                }
                mpsc::error::TrySendError::Closed(_) => self.closed_error(),
            })
    }

    fn try_media(&self, message: Message) -> Result<(), rtvbp::Error> {
        if self.closing.load(Ordering::Acquire) {
            return Err(self.closed_error());
        }
        self.outgoing_media
            .try_send(message)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    rtvbp::Error::Transport("bounded outgoing media queue overloaded".to_owned())
                }
                mpsc::error::TrySendError::Closed(_) => self.closed_error(),
            })
    }
}

struct WsControl {
    transport: Weak<BoundedWsTransport>,
    incoming: Arc<Inbox<Received>>,
}

#[async_trait]
impl ControlChannel for WsControl {
    async fn send(&self, data: Vec<u8>) -> Result<(), rtvbp::Error> {
        let transport = self.transport.upgrade().ok_or(rtvbp::Error::Closed)?;
        if data.len() > transport.bounds.control_frame_bytes {
            return Err(rtvbp::Error::Transport(
                "control frame exceeds the configured bound".to_owned(),
            ));
        }
        let text = String::from_utf8(data)
            .map_err(|_| rtvbp::Error::Transport("control frame is not UTF-8".to_owned()))?;
        transport.try_control(Message::Text(text.into()))
    }

    async fn recv(&self) -> Result<Received, rtvbp::Error> {
        self.incoming.pop().await
    }
}

struct WsMedia {
    transport: Weak<BoundedWsTransport>,
    incoming: Arc<Inbox<MediaFrame>>,
    format: MediaFormat,
    closed: AtomicBool,
}

#[async_trait]
impl MediaChannel for WsMedia {
    fn id(&self) -> &str {
        AUDIO_CHANNEL
    }

    fn format(&self) -> &MediaFormat {
        &self.format
    }

    async fn write_frame(&self, frame: MediaFrame) -> Result<(), rtvbp::Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(rtvbp::Error::Closed);
        }
        let transport = self.transport.upgrade().ok_or(rtvbp::Error::Closed)?;
        if frame.data.len() != transport.format.frame_bytes()? {
            return Err(rtvbp::Error::InvalidMediaFormat(
                "media frame does not match negotiated fixed width".to_owned(),
            ));
        }
        transport.try_media(Message::Binary(frame.data.into()))
    }

    async fn read_frame(&self) -> Result<MediaFrame, rtvbp::Error> {
        self.incoming.pop().await
    }

    async fn close(&self) -> Result<(), rtvbp::Error> {
        self.closed.store(true, Ordering::Release);
        Ok(())
    }
}

#[async_trait]
impl Transport for BoundedWsTransport {
    fn control(&self) -> Arc<dyn ControlChannel> {
        Arc::clone(&self.control) as Arc<dyn ControlChannel>
    }

    async fn accept_media(&self) -> Result<Arc<dyn MediaChannel>, rtvbp::Error> {
        self.claim_media()
    }

    async fn open_media(
        &self,
        id: &str,
        format: MediaFormat,
    ) -> Result<Arc<dyn MediaChannel>, rtvbp::Error> {
        if id != AUDIO_CHANNEL {
            return Err(rtvbp::Error::MediaUnsupported);
        }
        if format != self.format {
            return Err(rtvbp::Error::AudioFormatConflict);
        }
        self.claim_media()
    }

    async fn close(&self) -> Result<(), rtvbp::Error> {
        let existing_terminal = lock(&self.terminal).clone();
        let (result, force_abort) = if let Some(terminal) = existing_terminal {
            (terminal.result(), false)
        } else {
            if !self.closing.swap(true, Ordering::AcqRel) {
                self.close_requested.notify_waiters();
            }
            match tokio::time::timeout(self.bounds.close_deadline, self.wait_closed()).await {
                Ok(result) => (result, false),
                Err(_) => {
                    self.finish(Terminal::Failed("WebSocket close timed out".to_owned()));
                    (Err(rtvbp::Error::Timeout), true)
                }
            }
        };
        self.stop_and_join_pumps(force_abort).await;
        result
    }

    fn supports_keepalive(&self) -> bool {
        true
    }

    async fn monitor_keepalive(&self, policy: KeepalivePolicy) -> Result<(), rtvbp::Error> {
        policy.validate()?;
        if !policy.enabled() {
            return Ok(());
        }
        if self.keepalive_claimed.swap(true, Ordering::AcqRel) {
            return Err(rtvbp::Error::Configuration(
                "keepalive monitor already running".to_owned(),
            ));
        }
        let mut misses = 0usize;
        loop {
            tokio::select! {
                () = tokio::time::sleep(policy.interval) => {}
                closed = self.wait_closed() => return closed,
            }
            let serial = self.ping_serial.fetch_add(1, Ordering::Relaxed) + 1;
            let payload = format!("b10x:{serial}").into_bytes();
            self.try_control(Message::Ping(payload.clone().into()))?;
            let matched = tokio::time::timeout(policy.timeout, async {
                loop {
                    match self.pongs.pop().await {
                        Ok(pong) if pong == payload => return true,
                        Ok(_) => {}
                        Err(_) => return false,
                    }
                }
            })
            .await
            .unwrap_or(false);
            if matched {
                misses = 0;
            } else {
                misses += 1;
                if misses >= policy.max_misses {
                    self.finish(Terminal::Failed("keepalive timed out".to_owned()));
                    return Err(rtvbp::Error::KeepaliveTimeout);
                }
            }
        }
    }
}

impl Drop for BoundedWsTransport {
    fn drop(&mut self) {
        if let Some(handles) = self.pumps.get_mut().take() {
            handles.writer.abort();
            handles.reader.abort();
        }
    }
}

async fn write_pump<S>(
    transport: Weak<BoundedWsTransport>,
    mut control: mpsc::Receiver<Message>,
    mut media: mpsc::Receiver<Message>,
    mut writer: S,
) where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    loop {
        let Some(owner) = transport.upgrade() else {
            return;
        };
        let close_requested = Arc::clone(&owner.close_requested);
        let close_notification = close_requested.notified_owned();
        tokio::pin!(close_notification);
        close_notification.as_mut().enable();
        let closing = owner.closing.load(Ordering::Acquire);
        drop(owner);
        if closing {
            // Control acknowledgements and the single terminal event were accepted before close.
            // Flush that finite queue in order; queued media is deliberately abandoned once the
            // terminal transition wins.
            while let Ok(message) = control.try_recv() {
                if let Err(error) = writer.send(message).await {
                    if let Some(owner) = transport.upgrade() {
                        owner.finish(Terminal::Failed(error.to_string()));
                    }
                    return;
                }
            }
            let result = writer
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "Closed".into(),
                })))
                .await;
            let _ = writer.close().await;
            if let Some(owner) = transport.upgrade() {
                owner.finish(match result {
                    Ok(()) => Terminal::Orderly,
                    Err(error) => Terminal::Failed(error.to_string()),
                });
            }
            return;
        }
        let message = tokio::select! {
            biased;
            () = close_notification.as_mut() => continue,
            message = control.recv() => message,
            message = media.recv() => message,
        };
        let Some(message) = message else {
            if let Some(owner) = transport.upgrade() {
                owner.finish(Terminal::Orderly);
            }
            return;
        };
        if let Err(error) = writer.send(message).await {
            if let Some(owner) = transport.upgrade() {
                owner.finish(Terminal::Failed(error.to_string()));
            }
            return;
        }
    }
}

async fn read_pump<S>(transport: Weak<BoundedWsTransport>, mut reader: S)
where
    S: Stream<Item = Result<Message, WebSocketError>> + Unpin,
{
    while let Some(message) = reader.next().await {
        let Some(owner) = transport.upgrade() else {
            return;
        };
        let result = match message {
            Ok(Message::Text(text)) => {
                if text.len() > owner.bounds.control_frame_bytes {
                    Err(rtvbp::Error::Transport(
                        "control frame exceeds the configured bound".to_owned(),
                    ))
                } else {
                    owner.control.incoming.push_strict(Received {
                        data: text.as_bytes().to_vec(),
                        received_at: SystemTime::now(),
                    })
                }
            }
            Ok(Message::Binary(data)) => match owner.format.frame_bytes() {
                Ok(expected) if data.len() == expected => match owner
                    .media
                    .incoming
                    .push_drop_oldest(MediaFrame::untimed(data.to_vec()))
                {
                    Ok(true) => {
                        owner.incoming_media_loss.fetch_add(1, Ordering::AcqRel);
                        owner.media_loss_ready.notify_waiters();
                        Err(rtvbp::Error::Transport(
                            "bounded incoming media queue overloaded".to_owned(),
                        ))
                    }
                    Ok(false) => Ok(()),
                    Err(error) => Err(error),
                },
                Ok(_) => Err(rtvbp::Error::InvalidMediaFormat(
                    "media frame does not match negotiated fixed width".to_owned(),
                )),
                Err(error) => Err(error),
            },
            Ok(Message::Pong(data)) => owner.pongs.push_drop_oldest(data.to_vec()).map(drop),
            Ok(Message::Ping(_) | Message::Frame(_)) => Ok(()),
            Ok(Message::Close(_)) => {
                owner.finish(Terminal::Orderly);
                return;
            }
            Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => {
                owner.finish(Terminal::Orderly);
                return;
            }
            Err(error) => Err(rtvbp::Error::Transport(error.to_string())),
        };
        if let Err(error) = result {
            owner.finish(Terminal::Failed(error.to_string()));
            return;
        }
    }
    if let Some(owner) = transport.upgrade() {
        owner.finish(Terminal::Orderly);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::pin::Pin;
    use std::sync::atomic::AtomicBool;
    use std::task::{Context, Poll};

    use futures_util::SinkExt as _;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio_tungstenite::tungstenite::protocol::Role;

    use super::*;

    struct StalledIo {
        dropped: Arc<AtomicBool>,
    }

    impl Drop for StalledIo {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::Release);
        }
    }

    impl AsyncRead for StalledIo {
        fn poll_read(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            _buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Pending
        }
    }

    impl AsyncWrite for StalledIo {
        fn poll_write(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            _buffer: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Pending
        }

        fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }

        fn poll_shutdown(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
    }

    fn short_bounds() -> Bounds {
        Bounds {
            incoming_control: 2,
            outgoing_control: 2,
            incoming_media: 1,
            outgoing_media: 2,
            control_frame_bytes: MAX_CONTROL_FRAME_BYTES,
            close_deadline: Duration::from_millis(25),
        }
    }

    #[tokio::test]
    async fn close_aborts_and_joins_pumps_when_async_write_never_progresses() {
        let dropped = Arc::new(AtomicBool::new(false));
        let stream = WebSocketStream::from_raw_socket(
            StalledIo {
                dropped: Arc::clone(&dropped),
            },
            Role::Client,
            None,
        )
        .await;
        let transport = BoundedWsTransport::start(stream, short_bounds(), crate::media_format())
            .expect("transport starts");

        assert!(matches!(
            transport.close().await,
            Err(rtvbp::Error::Timeout)
        ));
        assert!(
            dropped.load(Ordering::Acquire),
            "close returned while a stalled pump still owned the WebSocket stream"
        );
    }

    #[tokio::test]
    async fn cancelling_an_outer_close_timeout_does_not_detach_stalled_pumps() {
        let dropped = Arc::new(AtomicBool::new(false));
        let stream = WebSocketStream::from_raw_socket(
            StalledIo {
                dropped: Arc::clone(&dropped),
            },
            Role::Client,
            None,
        )
        .await;
        let mut bounds = short_bounds();
        bounds.close_deadline = Duration::from_secs(1);
        let transport = BoundedWsTransport::start(stream, bounds, crate::media_format())
            .expect("transport starts");

        assert!(
            tokio::time::timeout(Duration::from_millis(10), transport.close())
                .await
                .is_err()
        );
        drop(transport);
        tokio::time::timeout(Duration::from_secs(1), async {
            while !dropped.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dropping the transport aborts pumps retained after outer cancellation");
    }

    #[tokio::test]
    async fn an_earlier_media_overload_cannot_be_hidden_by_a_later_control_close() {
        let (voice_io, application_io) = tokio::io::duplex(8_192);
        let voice = WebSocketStream::from_raw_socket(voice_io, Role::Client, None).await;
        let mut application =
            WebSocketStream::from_raw_socket(application_io, Role::Server, None).await;
        let transport = BoundedWsTransport::start(voice, short_bounds(), crate::media_format())
            .expect("transport starts");
        let frame = vec![0_u8; crate::media_format().frame_bytes().unwrap()];

        application
            .feed(Message::Binary(frame.clone().into()))
            .await
            .unwrap();
        application
            .feed(Message::Binary(frame.into()))
            .await
            .unwrap();
        let _ = application.feed(Message::Text("later-close".into())).await;
        let _ = application.flush().await;

        let loss =
            tokio::time::timeout(Duration::from_secs(1), transport.wait_incoming_media_loss())
                .await
                .expect("media loss is signalled immediately");
        assert_eq!(loss, 1);
        assert_eq!(transport.evidence().incoming_control, 0);
        let control_error = transport.control().recv().await.unwrap_err();
        assert!(control_error.to_string().contains("media queue overloaded"));
        let close_error = transport.close().await.unwrap_err();
        assert!(close_error.to_string().contains("media queue overloaded"));
    }
}
