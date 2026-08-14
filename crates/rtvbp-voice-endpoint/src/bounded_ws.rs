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
use tokio::sync::{mpsc, Notify};
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
    /// Incoming audio frames; saturation drops oldest and increments owned loss.
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
    done: Notify,
    closing: AtomicBool,
    close_requested: Notify,
    media_claimed: AtomicBool,
    incoming_media_loss: AtomicU64,
    pongs: Arc<Inbox<Vec<u8>>>,
    keepalive_claimed: AtomicBool,
    ping_serial: AtomicU64,
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
            done: Notify::new(),
            closing: AtomicBool::new(false),
            close_requested: Notify::new(),
            media_claimed: AtomicBool::new(false),
            incoming_media_loss: AtomicU64::new(0),
            pongs: Arc::new(Inbox::new(1)),
            keepalive_claimed: AtomicBool::new(false),
            ping_serial: AtomicU64::new(0),
        });
        let (writer, reader) = stream.split();
        tokio::spawn(write_pump(
            Arc::clone(&transport),
            control_rx,
            media_rx,
            writer,
        ));
        tokio::spawn(read_pump(Arc::clone(&transport), reader));
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

    /// Return and reset loss newly observed by this receiver.
    pub fn take_incoming_media_loss(&self) -> u64 {
        self.incoming_media_loss.swap(0, Ordering::AcqRel)
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
        if let Some(terminal) = lock(&self.terminal).clone() {
            return terminal.result();
        }
        if !self.closing.swap(true, Ordering::AcqRel) {
            self.close_requested.notify_waiters();
        }
        tokio::time::timeout(self.bounds.close_deadline, self.wait_closed())
            .await
            .map_err(|_| rtvbp::Error::Timeout)?
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

async fn write_pump<S>(
    transport: Arc<BoundedWsTransport>,
    mut control: mpsc::Receiver<Message>,
    mut media: mpsc::Receiver<Message>,
    mut writer: S,
) where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    loop {
        if transport.closing.load(Ordering::Acquire) {
            let result = writer
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "Closed".into(),
                })))
                .await;
            let _ = writer.close().await;
            transport.finish(match result {
                Ok(()) => Terminal::Orderly,
                Err(error) => Terminal::Failed(error.to_string()),
            });
            return;
        }
        let message = tokio::select! {
            biased;
            () = transport.close_requested.notified() => continue,
            message = control.recv() => message,
            message = media.recv() => message,
        };
        let Some(message) = message else {
            transport.finish(Terminal::Orderly);
            return;
        };
        if let Err(error) = writer.send(message).await {
            transport.finish(Terminal::Failed(error.to_string()));
            return;
        }
    }
}

async fn read_pump<S>(transport: Arc<BoundedWsTransport>, mut reader: S)
where
    S: Stream<Item = Result<Message, WebSocketError>> + Unpin,
{
    while let Some(message) = reader.next().await {
        let result = match message {
            Ok(Message::Text(text)) => {
                if text.len() > transport.bounds.control_frame_bytes {
                    Err(rtvbp::Error::Transport(
                        "control frame exceeds the configured bound".to_owned(),
                    ))
                } else {
                    transport.control.incoming.push_strict(Received {
                        data: text.as_bytes().to_vec(),
                        received_at: SystemTime::now(),
                    })
                }
            }
            Ok(Message::Binary(data)) => match transport.format.frame_bytes() {
                Ok(expected) if data.len() == expected => transport
                    .media
                    .incoming
                    .push_drop_oldest(MediaFrame::untimed(data.to_vec()))
                    .map(|dropped| {
                        if dropped {
                            transport.incoming_media_loss.fetch_add(1, Ordering::AcqRel);
                        }
                    }),
                Ok(_) => Err(rtvbp::Error::InvalidMediaFormat(
                    "media frame does not match negotiated fixed width".to_owned(),
                )),
                Err(error) => Err(error),
            },
            Ok(Message::Pong(data)) => transport.pongs.push_drop_oldest(data.to_vec()).map(drop),
            Ok(Message::Ping(_) | Message::Frame(_)) => Ok(()),
            Ok(Message::Close(_)) => {
                transport.finish(Terminal::Orderly);
                return;
            }
            Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => {
                transport.finish(Terminal::Orderly);
                return;
            }
            Err(error) => Err(rtvbp::Error::Transport(error.to_string())),
        };
        if let Err(error) = result {
            transport.finish(Terminal::Failed(error.to_string()));
            return;
        }
    }
    transport.finish(Terminal::Orderly);
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
