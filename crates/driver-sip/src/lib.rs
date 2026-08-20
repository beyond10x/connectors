#![forbid(unsafe_code)]

//! Development-gated sipx implementation of the neutral telephony session.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bytes::Bytes;
use domain::voice::{
    AudioFrame, ChannelSignal, ContextTrust, MediaDescriptor, ParticipantContext, TelephonySession,
    TerminationReason, VoiceError, VoiceRef,
};
use service::{AdmittedSipPlan, CredentialSet, SipSignalingTransport};
use sipx_call::{CallEvent, Codecs, DialOptions, EndCause, Served};
use sipx_media::{Interrupt, MediaSession, Playback};
use sipx_sip::{Host, Uri};
use sipx_transport::{Config, Target, TransportKind};
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const SAMPLES_PER_FRAME: usize = 160;
const SIGNAL_CAPACITY: usize = 16;
static SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Closed establishment failures. Credential values and call audio are absent from every error.
#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    /// Credentials must be exactly username then password.
    #[error("SIP credentials are incomplete")]
    Credentials,
    /// An application-admitted SIP URI is invalid.
    #[error("application-admitted SIP identity or target URI is invalid")]
    InvalidUri,
    /// sipx could not bind the admitted endpoint.
    #[error("sipx signaling bind failed: {0}")]
    Bind(String),
    /// The actual ephemeral bind escaped the admitted aperture.
    #[error("sipx signaling listener escaped the admitted aperture")]
    BindAperture,
    /// Call establishment failed.
    #[error("sipx call establishment failed: {0}")]
    Establish(String),
    /// The owning runtime cancelled establishment and sipx withdrew the invitation.
    #[error("sipx call establishment was cancelled")]
    Cancelled,
    /// Contact or route-set state learned from SIP escaped the admitted aperture.
    #[error("SIP-learned dialog target escaped the admitted aperture")]
    LearnedSignalingTarget,
    /// SDP-selected RTP target escaped the admitted aperture.
    #[error("SDP-learned media target escaped the admitted aperture")]
    LearnedMediaTarget,
    /// The call did not negotiate the exact first-profile media shape.
    #[error("SIP media negotiation differs from the neutral voice profile ({rate_hz} Hz, {samples_per_packet} samples)")]
    MediaMismatch {
        /// Decoded sample rate.
        rate_hz: u32,
        /// Decoded samples in one negotiated packet.
        samples_per_packet: usize,
    },
    /// A neutral descriptor could not be built.
    #[error(transparent)]
    Voice(#[from] VoiceError),
}

/// Establish one admitted outbound loopback call and return only the neutral port.
pub async fn establish_outbound(
    admitted: &AdmittedSipPlan,
    credentials: &CredentialSet,
    cancelled: CancellationToken,
) -> Result<Arc<dyn TelephonySession>, DriverError> {
    let route = admitted.route();
    let mut config = Config::new(route.signaling_bind);
    config.sent_by = route.sent_by.clone();
    let (endpoint, mut incoming) = tokio::select! {
        biased;
        () = cancelled.cancelled() => return Err(DriverError::Cancelled),
        result = sipx_transport::bind(config) => {
            result.map_err(|error| DriverError::Bind(error.to_string()))?
        }
    };
    if !admitted.admits_signaling(endpoint.local_addr()) {
        endpoint.shutdown().await;
        return Err(DriverError::BindAperture);
    }

    let to = Uri::parse(Bytes::copy_from_slice(route.to_uri.as_bytes()))
        .map_err(|_| DriverError::InvalidUri)?;
    let mut options = DialOptions::new(&route.from_uri, route.media_advertised)
        .with_media_bind_address(route.media_bind)
        .with_codecs(Codecs::G711);
    options.timeout = Some(route.dial_timeout);
    match credentials.values() {
        [] => {}
        [username, password] => {
            options = options.with_credentials(sipx_call::Credentials::new(
                username.expose_secret(),
                password.expose_secret(),
            ));
        }
        _ => {
            endpoint.shutdown().await;
            return Err(DriverError::Credentials);
        }
    }
    let transport = match route.signaling_transport {
        SipSignalingTransport::Udp => TransportKind::Udp,
        SipSignalingTransport::Tcp => TransportKind::Tcp,
    };
    // Always already resolved: `admit_sip_dial` turns a named trunk into an address *before*
    // admission, so that address could be aperture-checked. An unresolved target reaching here
    // would mean admission was bypassed, which is a refusal rather than a lookup to perform.
    let Some(signaling_target) = route.target.address() else {
        endpoint.shutdown().await;
        return Err(DriverError::InvalidUri);
    };
    let target = Target::new(signaling_target, transport);
    if cancelled.is_cancelled() {
        endpoint.shutdown().await;
        return Err(DriverError::Cancelled);
    }
    let mut call = match sipx_call::dial_until(
        &endpoint,
        target,
        &to,
        &options,
        cancelled.clone().cancelled_owned(),
    )
    .await
    {
        Ok(call) => call,
        Err(error) => {
            endpoint.shutdown().await;
            return if cancelled.is_cancelled() {
                Err(DriverError::Cancelled)
            } else {
                Err(DriverError::Establish(error.to_string()))
            };
        }
    };

    if cancelled.is_cancelled() {
        let _ = call.hang_up().await;
        endpoint.shutdown().await;
        return Err(DriverError::Cancelled);
    }

    if !admitted.admits_media(call.peer_media_address()) {
        let _ = call.hang_up().await;
        endpoint.shutdown().await;
        return Err(DriverError::LearnedMediaTarget);
    }
    if !dialog_target_is_admitted(admitted, &call) {
        let _ = call.hang_up().await;
        endpoint.shutdown().await;
        return Err(DriverError::LearnedSignalingTarget);
    }
    let rate_hz = call.media().audio_rate();
    let samples_per_packet = call.media().samples_per_packet();
    if rate_hz != 8_000 || samples_per_packet != SAMPLES_PER_FRAME {
        let _ = call.hang_up().await;
        endpoint.shutdown().await;
        return Err(DriverError::MediaMismatch {
            rate_hz,
            samples_per_packet,
        });
    }

    let session_number = SESSION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let descriptor = VoiceSessionDescriptor::new(session_number)?;
    let media = call.media_handle();
    let events = call.events().ok_or_else(|| {
        DriverError::Establish("call event stream was already claimed".to_owned())
    })?;
    let (signals_tx, signals_rx) = mpsc::channel(SIGNAL_CAPACITY);
    let (terminate_tx, mut terminate_rx) = mpsc::channel(1);
    let shared = Arc::new(Shared {
        terminal: Mutex::new(None),
        terminal_ready: Notify::new(),
        requested_terminal: Mutex::new(None),
        signal_loss: AtomicU64::new(0),
    });
    let session = Arc::new(SipTelephonySession {
        descriptor: descriptor.0,
        media,
        input_sequence: AtomicU64::new(1),
        output_sequence: AtomicU64::new(0),
        signals: tokio::sync::Mutex::new(signals_rx),
        active_playback: Mutex::new(None),
        terminate: terminate_tx,
        shared: Arc::clone(&shared),
        owner: tokio::sync::Mutex::new(None),
    });

    let endpoint_owner = endpoint.clone();
    let owner = tokio::spawn(async move {
        let event_shared = Arc::clone(&shared);
        let work = move |_media: Arc<MediaSession>,
                         stopped: tokio_util::sync::CancellationToken| {
            observe_events(events, signals_tx, event_shared, stopped)
        };
        let interrupted = async move {
            let _ = terminate_rx.recv().await;
        };
        let outcome = sipx_call::serve_until(&mut call, &mut incoming, work, interrupted).await;
        let reason = match outcome {
            Ok(Served::Remote { cause, .. }) => remote_reason(cause),
            Ok(Served::Interrupted { .. }) => {
                lock(&shared.requested_terminal).unwrap_or(TerminationReason::Completed)
            }
            Ok(Served::Local { .. }) => TerminationReason::Completed,
            Ok(_) => TerminationReason::ProtocolError,
            Err(_) => TerminationReason::TransportLost,
        };
        shared.finish(reason);
        endpoint_owner.shutdown().await;
    });
    *session.owner.lock().await = Some(owner);
    Ok(session as Arc<dyn TelephonySession>)
}

struct VoiceSessionDescriptor(domain::voice::VoiceSessionDescriptor);

impl VoiceSessionDescriptor {
    fn new(sequence: u64) -> Result<Self, VoiceError> {
        Ok(Self(domain::voice::VoiceSessionDescriptor {
            call: VoiceRef::new(format!("sip-call-{sequence}"))?,
            session: VoiceRef::new(format!("sip-session-{sequence}"))?,
            channel: VoiceRef::new(format!("sip-channel-{sequence}"))?,
            participant: ParticipantContext {
                reference: VoiceRef::new(format!("sip-peer-{sequence}"))?,
                trust: ContextTrust::Untrusted,
                display: None,
            },
            media: MediaDescriptor::pcm_s16le_8khz_mono_20ms(),
        }))
    }
}

struct Shared {
    terminal: Mutex<Option<TerminationReason>>,
    terminal_ready: Notify,
    requested_terminal: Mutex<Option<TerminationReason>>,
    signal_loss: AtomicU64,
}

impl Shared {
    fn finish(&self, reason: TerminationReason) {
        let mut terminal = lock(&self.terminal);
        if terminal.is_none() {
            *terminal = Some(reason);
            drop(terminal);
            self.terminal_ready.notify_waiters();
        }
    }

    async fn wait_terminal(&self) -> TerminationReason {
        loop {
            let notified = self.terminal_ready.notified();
            if let Some(reason) = *lock(&self.terminal) {
                return reason;
            }
            notified.await;
        }
    }
}

struct SipTelephonySession {
    descriptor: domain::voice::VoiceSessionDescriptor,
    media: Arc<MediaSession>,
    input_sequence: AtomicU64,
    output_sequence: AtomicU64,
    signals: tokio::sync::Mutex<mpsc::Receiver<ChannelSignal>>,
    active_playback: Mutex<Option<Playback>>,
    terminate: mpsc::Sender<()>,
    shared: Arc<Shared>,
    owner: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

#[async_trait]
impl TelephonySession for SipTelephonySession {
    fn descriptor(&self) -> &domain::voice::VoiceSessionDescriptor {
        &self.descriptor
    }

    async fn read_input(&self) -> Result<Option<AudioFrame>, VoiceError> {
        let Some(samples) = self.media.recv().await else {
            return Ok(None);
        };
        if samples.len() != SAMPLES_PER_FRAME {
            return Err(VoiceError::Endpoint(
                "sipx emitted a non-profile PCM frame".to_owned(),
            ));
        }
        let bytes = samples
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        AudioFrame::new(
            self.input_sequence.fetch_add(1, Ordering::Relaxed),
            bytes,
            &self.descriptor.media,
        )
        .map(Some)
    }

    async fn write_output(&self, frame: AudioFrame) -> Result<(), VoiceError> {
        if self.shared.terminal.lock().map_err(lock_error)?.is_some() {
            return Err(VoiceError::Terminated);
        }
        let frame = AudioFrame::new(frame.sequence, frame.bytes, &self.descriptor.media)?;
        let prior = self.output_sequence.load(Ordering::Acquire);
        if frame.sequence <= prior {
            return Err(VoiceError::Endpoint(
                "non-monotonic telephony output sequence".to_owned(),
            ));
        }
        self.output_sequence
            .store(frame.sequence, Ordering::Release);
        let samples = frame
            .bytes
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect::<Vec<_>>();
        let playback = self.media.start_playback(samples, Interrupt::Never);
        *self.active_playback.lock().map_err(lock_error)? = Some(playback.clone());
        let completed = playback.play_out().await.completed();
        let mut active = self.active_playback.lock().map_err(lock_error)?;
        if active
            .as_ref()
            .is_some_and(|current| current.id() == playback.id())
        {
            *active = None;
        }
        if completed {
            Ok(())
        } else {
            Err(VoiceError::Endpoint("sipx playback ended early".to_owned()))
        }
    }

    async fn next_signal(&self) -> Result<Option<ChannelSignal>, VoiceError> {
        Ok(self.signals.lock().await.recv().await)
    }

    async fn wait_terminated(&self) -> Result<TerminationReason, VoiceError> {
        Ok(self.shared.wait_terminal().await)
    }

    async fn interrupt_output(&self) -> Result<(), VoiceError> {
        if let Some(playback) = self.active_playback.lock().map_err(lock_error)?.take() {
            playback.stop();
        }
        Ok(())
    }

    async fn terminate(&self, reason: TerminationReason) -> Result<(), VoiceError> {
        {
            let mut requested = self.shared.requested_terminal.lock().map_err(lock_error)?;
            requested.get_or_insert(reason);
        }
        match self.terminate.try_send(()) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => {
                if self.shared.terminal.lock().map_err(lock_error)?.is_none() {
                    return Err(VoiceError::Terminated);
                }
            }
        }
        let _winner = self.shared.wait_terminal().await;
        let owner = self.owner.lock().await.take();
        if let Some(owner) = owner {
            owner
                .await
                .map_err(|error| VoiceError::Endpoint(error.to_string()))?;
        }
        Ok(())
    }
}

impl Drop for SipTelephonySession {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.get_mut().take() {
            owner.abort();
        }
    }
}

async fn observe_events(
    mut events: sipx_call::CallEvents,
    signals: mpsc::Sender<ChannelSignal>,
    shared: Arc<Shared>,
    stopped: tokio_util::sync::CancellationToken,
) {
    loop {
        tokio::select! {
            () = stopped.cancelled() => return,
            event = events.recv() => match event {
                Some(CallEvent::Dtmf { digit, .. }) => {
                    let signal = ChannelSignal::Dtmf { digits: digit.to_string() };
                    if signals.try_send(signal).is_err() {
                        shared.signal_loss.fetch_add(1, Ordering::AcqRel);
                    }
                }
                Some(CallEvent::Ended(cause)) => shared.finish(remote_reason(cause)),
                Some(_) => {}
                None => return,
            }
        }
    }
}

fn dialog_target_is_admitted(admitted: &AdmittedSipPlan, call: &sipx_call::Call) -> bool {
    let (target, routes) = call.dialog.request_target();
    if !routes.is_empty() {
        return false;
    }
    let Some(Host::Ip(address)) = target.host() else {
        return false;
    };
    let port = target.port().unwrap_or(5060);
    admitted.admits_signaling((*address, port).into())
}

fn remote_reason(cause: EndCause) -> TerminationReason {
    match cause {
        EndCause::RemoteBye | EndCause::RemoteCancel => TerminationReason::RemoteHangup,
        EndCause::Timeout => TerminationReason::TransportLost,
        _ => TerminationReason::Completed,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> VoiceError {
    VoiceError::Endpoint(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;

    use domain::{
        AdmittedOperation, Capability, ConnectionAuthority, Implementation, InitiationPolicy,
        Interaction, OperationFacts, Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
    };
    use protocol::voice::Ready;
    use rtvbp::transport::memory::Config as RtvbpConfig;
    use rtvbp::{ControlFrame, Envelope as _, Transport as _};
    use rtvbp_voice_endpoint::{VoiceEndpoint, INITIALIZE_METHOD, PROFILE};
    use service::authority::{AuthorityIssuer, IssueRequest, ProofKey};
    use service::{admit_sip_plan, SipDeploymentRoute, SipSignalingTarget, SocketAperture};
    use tokio::sync::oneshot;

    use super::*;

    fn loopback() -> IpAddr {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }

    fn zero_io_plan() -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "loopback-pbx".to_owned(),
                operation: "loopback-call-establish".to_owned(),
                service: "voice".to_owned(),
                interaction: Interaction::SessionEstablishment,
                placement: Placement::ConnectorsDeployment,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec!["loopback:127.0.0.1".to_owned()],
            },
            AdmittedOperation::from_grant_decision(
                "loopback-pbx",
                "loopback-call-establish",
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::b10x_only())
                    .unwrap(),
            ),
            ProtocolPlan::SipV1(SipPlan {
                connection: "connection-1".to_owned(),
            }),
        )
    }

    #[tokio::test]
    async fn sip_to_rtvbp_fake_application_runs_duplex_pcm_and_observed_teardown() {
        let (callee, mut callee_incoming) =
            sipx_transport::bind(Config::new(SocketAddr::new(loopback(), 0)))
                .await
                .unwrap();
        let all_loopback_ports = SocketAperture::new(loopback(), 1..=u16::MAX).unwrap();
        let admitted = admit_sip_plan(
            &zero_io_plan(),
            SipDeploymentRoute {
                connection: "connection-1".to_owned(),
                signaling_bind: SocketAddr::new(loopback(), 0),
                sent_by: "127.0.0.1".to_owned(),
                target: SipSignalingTarget::Address(callee.local_addr()),
                signaling_transport: SipSignalingTransport::Udp,
                to_uri: format!("sip:callee@{}", callee.local_addr()),
                from_uri: "sip:caller@127.0.0.1".to_owned(),
                media_advertised: loopback(),
                media_bind: loopback(),
                signaling_apertures: vec![all_loopback_ports.clone()],
                media_apertures: vec![all_loopback_ports],
                dial_timeout: Duration::from_secs(5),
                network_mode: service::SipNetworkMode::Loopback,
                accepts_dialed_number: false,
            },
        )
        .unwrap();
        let (ready_tx, ready_rx) = oneshot::channel();
        let callee_owner = callee.clone();
        let callee_task = tokio::spawn(async move {
            let invitation = callee_incoming.recv().await.unwrap();
            let mut call =
                sipx_call::answer_with(&callee_owner, &invitation, loopback(), Codecs::G711)
                    .await
                    .unwrap();
            let ack = callee_incoming.recv().await.unwrap();
            assert!(call.handle(&ack).await.unwrap());
            ready_tx.send(call.media_handle()).unwrap();
            sipx_call::serve(&mut call, &mut callee_incoming)
                .await
                .unwrap();
            callee_owner.shutdown().await;
        });

        let session = establish_outbound(
            &admitted,
            &CredentialSet::default(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let callee_media = ready_rx.await.unwrap();
        let (voice_transport, application_transport) =
            rtvbp::transport::memory::MemoryTransport::pair(RtvbpConfig { media: true });
        let issuer = AuthorityIssuer::new(
            "issuer",
            "key-1",
            ed25519_dalek::SigningKey::from_bytes(&[3; 32]),
        );
        let proof_key = ProofKey::from_bytes(&[4; 32]);
        let issued = issuer
            .issue(IssueRequest {
                audience: "application".to_owned(),
                subject: "principal".to_owned(),
                actor: "voice".to_owned(),
                organization: "org".to_owned(),
                deployment: "application".to_owned(),
                connection: "connection-1".to_owned(),
                grant: "grant-1".to_owned(),
                resource: "endpoint".to_owned(),
                operation: "loopback-call-establish".to_owned(),
                channel_kind: "voice".to_owned(),
                protocol: PROFILE.to_owned(),
                endpoint: "wss://application.example/voice".to_owned(),
                proof_thumbprint: proof_key.thumbprint(),
                authority_id: "authority-1".to_owned(),
                issued_at: 100,
                not_before: 100,
                expires_at: 160,
                lease_expires_at: 1_000,
            })
            .unwrap();
        let endpoint = VoiceEndpoint::new(
            Arc::clone(&session),
            voice_transport,
            Some(PROFILE),
            &issued,
        )
        .unwrap();
        let expected_input = vec![1_234_i16; SAMPLES_PER_FRAME];
        let application = tokio::spawn(async move {
            let envelope = rtvbp::envelope::v1classic::Envelope;
            let request = application_transport.control().recv().await.unwrap();
            let request = envelope.decode(&request.data).unwrap();
            assert_eq!(request.method, INITIALIZE_METHOD);
            let response = ControlFrame::response(
                request.id,
                Some(
                    serde_json::to_value(Ready {
                        contract: protocol::voice::CONTRACT.to_owned(),
                    })
                    .unwrap(),
                ),
                None,
            );
            application_transport
                .control()
                .send(envelope.encode(&response).unwrap())
                .await
                .unwrap();
            let media = application_transport.accept_media().await.unwrap();
            let inbound = media.read_frame().await.unwrap();
            let heard = inbound
                .data
                .chunks_exact(2)
                .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
                .collect::<Vec<_>>();
            assert!(heard
                .iter()
                .zip(&expected_input)
                .all(
                    |(actual, expected)| (i32::from(*actual) - i32::from(*expected)).abs() <= 128
                ));
            media
                .write_frame(rtvbp::MediaFrame::untimed(
                    [0x2e, 0x16].repeat(SAMPLES_PER_FRAME),
                ))
                .await
                .unwrap();
        });
        endpoint.initialize_application().await.unwrap();

        let source = vec![1_234_i16; SAMPLES_PER_FRAME];
        let send = tokio::spawn({
            let callee_media = Arc::clone(&callee_media);
            let source = source.clone();
            async move {
                assert!(callee_media.play(&source, SAMPLES_PER_FRAME).await);
            }
        });
        assert!(
            tokio::time::timeout(Duration::from_secs(5), endpoint.forward_input_once())
                .await
                .unwrap()
                .unwrap()
        );
        send.await.unwrap();

        let (written, recorded) = tokio::join!(
            endpoint.forward_output_once(),
            callee_media.record_at_least(SAMPLES_PER_FRAME, Duration::from_secs(5))
        );
        written.unwrap();
        assert_eq!(recorded.len(), SAMPLES_PER_FRAME);
        assert!(recorded
            .iter()
            .all(|actual| (i32::from(*actual) - 5_678).abs() <= 128));
        application.await.unwrap();

        tokio::time::timeout(
            Duration::from_secs(5),
            endpoint.terminate(TerminationReason::Completed),
        )
        .await
        .unwrap()
        .unwrap();
        callee_task.await.unwrap();
    }

    #[tokio::test]
    async fn cancellation_while_ringing_withdraws_the_invite_before_returning() {
        let (callee, mut callee_incoming) =
            sipx_transport::bind(Config::new(SocketAddr::new(loopback(), 0)))
                .await
                .unwrap();
        let all_loopback_ports = SocketAperture::new(loopback(), 1..=u16::MAX).unwrap();
        let admitted = admit_sip_plan(
            &zero_io_plan(),
            SipDeploymentRoute {
                connection: "connection-1".to_owned(),
                signaling_bind: SocketAddr::new(loopback(), 0),
                sent_by: "127.0.0.1".to_owned(),
                target: SipSignalingTarget::Address(callee.local_addr()),
                signaling_transport: SipSignalingTransport::Udp,
                to_uri: format!("sip:callee@{}", callee.local_addr()),
                from_uri: "sip:caller@127.0.0.1".to_owned(),
                media_advertised: loopback(),
                media_bind: loopback(),
                signaling_apertures: vec![all_loopback_ports.clone()],
                media_apertures: vec![all_loopback_ports],
                dial_timeout: Duration::from_secs(5),
                network_mode: service::SipNetworkMode::Loopback,
                accepts_dialed_number: false,
            },
        )
        .unwrap();
        let (ringing_tx, ringing_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let callee_owner = callee.clone();
        let callee_task = tokio::spawn(async move {
            let invitation = callee_incoming.recv().await.unwrap();
            assert_eq!(invitation.request.method, sipx_sip::Method::Invite);
            let ringing = sipx_sip::build::ResponseBuilder::to_request(
                &invitation.request,
                sipx_sip::StatusCode::new(180).unwrap(),
                "Ringing",
            )
            .unwrap()
            .build();
            callee_owner
                .respond(&invitation.key, ringing)
                .await
                .unwrap();
            ringing_tx.send(()).unwrap();

            let cancellation = callee_incoming.recv().await.unwrap();
            assert_eq!(cancellation.request.method, sipx_sip::Method::Cancel);
            let cancel_ok = sipx_sip::build::ResponseBuilder::to_request(
                &cancellation.request,
                sipx_sip::StatusCode::new(200).unwrap(),
                "OK",
            )
            .unwrap()
            .build();
            callee_owner
                .respond(&cancellation.key, cancel_ok)
                .await
                .unwrap();
            let withdrawn = sipx_sip::build::ResponseBuilder::to_request(
                &invitation.request,
                sipx_sip::StatusCode::new(487).unwrap(),
                "Request Terminated",
            )
            .unwrap()
            .build();
            callee_owner
                .respond(&invitation.key, withdrawn)
                .await
                .unwrap();
            cancel_tx.send(()).unwrap();
            callee_owner.shutdown().await;
        });
        let cancelled = CancellationToken::new();
        let cancellation = cancelled.clone();
        let cancel_when_ringing = async move {
            ringing_rx.await.unwrap();
            cancellation.cancel();
        };
        let credentials = CredentialSet::default();

        let (established, ()) = tokio::join!(
            establish_outbound(&admitted, &credentials, cancelled),
            cancel_when_ringing,
        );
        assert!(matches!(established, Err(DriverError::Cancelled)));
        tokio::time::timeout(Duration::from_secs(2), cancel_rx)
            .await
            .expect("the callee receives CANCEL before driver return")
            .unwrap();
        callee_task.await.unwrap();
    }

    #[test]
    fn transport_timeout_is_preserved_as_a_neutral_terminal_reason() {
        assert_eq!(
            remote_reason(EndCause::Timeout),
            TerminationReason::TransportLost
        );
    }
}
