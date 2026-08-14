#![forbid(unsafe_code)]

//! Exact local RTVBP binding for the protocol-neutral voice contract.

/// Locally owned finite WebSocket transport; upstream RTVBP semantics remain unchanged.
pub mod bounded_ws;
/// Proof-bound connecting-side WebSocket upgrade.
pub mod connect;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use domain::voice::{AudioFrame, TelephonySession, TerminationReason, VoiceError};
use protocol::voice::{self, Acknowledged, Close, InterruptOutput, Ready};
use rtvbp::{
    ControlFrame, Envelope as _, FrameKind, MediaChannel, MediaFormat, Transport, WireError,
};
use server::authority::IssuedAuthority;

/// Exact WebSocket subprotocol/profile token. Headerless negotiation is never accepted.
pub const PROFILE: &str = "b10x.voice.v1";
/// Upstream envelope selected by the binding manifest.
pub const ENVELOPE: &str = "classic.v1";
/// One media channel in the first binding.
pub const AUDIO_CHANNEL: &str = "audio";
/// Maximum complete encoded control envelope.
pub const MAX_CONTROL_FRAME_BYTES: usize = 16_384;

pub const INITIALIZE_METHOD: &str = "voice.initialize";
pub const INTERRUPT_METHOD: &str = "voice.output.interrupt";
pub const CLOSE_METHOD: &str = "voice.session.close";
pub const SIGNAL_EVENT: &str = "voice.signal";
pub const MEDIA_LOSS_EVENT: &str = "voice.media.loss";
pub const TERMINATED_EVENT: &str = "voice.terminated";

const INVALID_STATE: i64 = -32_001;
const INVALID_PAYLOAD: i64 = -32_002;

/// Exact profile negotiation refusal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindingError {
    #[error("RTVBP profile must be explicitly negotiated")]
    ProfileRequired,
    #[error("RTVBP profile `{0}` is not accepted by the generic voice endpoint")]
    ProfileRefused(String),
    #[error("issued authority binds `{actual}`, expected `{PROFILE}`")]
    AuthorityProfileMismatch { actual: String },
    #[error("RTVBP control frame exceeds {MAX_CONTROL_FRAME_BYTES} bytes")]
    ControlFrameTooLarge,
    #[error("RTVBP control frame is invalid: {0}")]
    Control(String),
    #[error("RTVBP transport failed: {0}")]
    Transport(String),
    #[error("bounded RTVBP media queue overloaded")]
    MediaOverload,
    #[error(transparent)]
    Voice(#[from] VoiceError),
}

/// Require exact explicit profile negotiation.
pub fn negotiate_profile(offered: Option<&str>) -> Result<(), BindingError> {
    match offered {
        None => Err(BindingError::ProfileRequired),
        Some(PROFILE) => Ok(()),
        Some(other) => Err(BindingError::ProfileRefused(other.to_owned())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Created,
    Ready,
    Closing,
    Closed,
}

/// Observable result of one application-to-voice control request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOutcome {
    /// The session remains ready after an acknowledged control.
    Continue,
    /// The application requested the single terminal transition.
    Close(TerminationReason),
}

/// Voice-side endpoint over one brokered transport.
pub struct VoiceEndpoint<T: TelephonySession + ?Sized> {
    telephony: Arc<T>,
    transport: Arc<dyn Transport>,
    envelope: rtvbp::envelope::v1classic::Envelope,
    state: Mutex<State>,
    media: Mutex<Option<Arc<dyn MediaChannel>>>,
    request_sequence: AtomicU64,
    output_sequence: AtomicU64,
}

impl<T: TelephonySession + ?Sized> VoiceEndpoint<T> {
    /// Construct from the issued authority the voice side will present. The serving application,
    /// never this endpoint, redeems it during the WebSocket upgrade.
    pub fn new(
        telephony: Arc<T>,
        transport: Arc<dyn Transport>,
        offered_profile: Option<&str>,
        issued: &IssuedAuthority,
    ) -> Result<Self, BindingError> {
        negotiate_profile(offered_profile)?;
        if issued.claims().dl_protocol != PROFILE {
            return Err(BindingError::AuthorityProfileMismatch {
                actual: issued.claims().dl_protocol.clone(),
            });
        }
        Ok(Self {
            telephony,
            transport,
            envelope: rtvbp::envelope::v1classic::Envelope,
            state: Mutex::new(State::Created),
            media: Mutex::new(None),
            request_sequence: AtomicU64::new(1),
            output_sequence: AtomicU64::new(1),
        })
    }

    /// Initialize the application role exactly once, then open the one duplex media channel.
    pub async fn initialize_application(&self) -> Result<(), BindingError> {
        if *self.state.lock().map_err(lock_error)? != State::Created {
            return Err(BindingError::Control("invalid_state".to_owned()));
        }
        let request_id = format!(
            "voice-{}",
            self.request_sequence.fetch_add(1, Ordering::Relaxed)
        );
        let payload = serde_json::to_value(voice::Initialize::from(self.telephony.descriptor()))
            .map_err(control_error)?;
        self.send(ControlFrame::request(
            request_id.clone(),
            INITIALIZE_METHOD,
            Some(payload),
        ))
        .await?;
        let response = self.receive().await?;
        if response.kind != FrameKind::Response || response.correlation_id != request_id {
            return Err(BindingError::Control(
                "invalid initialize response".to_owned(),
            ));
        }
        if let Some(error) = response.error {
            return Err(BindingError::Control(error.to_string()));
        }
        let ready: Ready =
            serde_json::from_value(response.payload.unwrap_or_default()).map_err(control_error)?;
        if ready.contract != voice::CONTRACT {
            return Err(BindingError::Control("wrong voice contract".to_owned()));
        }

        let media = self
            .transport
            .open_media(AUDIO_CHANNEL, media_format())
            .await
            .map_err(transport_error)?;
        *self.media.lock().map_err(lock_error)? = Some(media);
        *self.state.lock().map_err(lock_error)? = State::Ready;
        Ok(())
    }

    /// Handle one bounded application-to-voice control request.
    pub async fn serve_control_once(&self) -> Result<ControlOutcome, BindingError> {
        let frame = self.receive().await?;
        if frame.kind != FrameKind::Request {
            return Err(BindingError::Control("expected request".to_owned()));
        }
        let (response, outcome) = match frame.method.as_str() {
            INTERRUPT_METHOD if self.is_ready()? => {
                let _: InterruptOutput = decode_payload(&frame)?;
                self.telephony.interrupt_output().await?;
                (
                    ControlFrame::response(
                        frame.id,
                        Some(serde_json::to_value(Acknowledged::default()).map_err(control_error)?),
                        None,
                    ),
                    ControlOutcome::Continue,
                )
            }
            CLOSE_METHOD if self.is_ready()? => {
                let close: Close = decode_payload(&frame)?;
                *self.state.lock().map_err(lock_error)? = State::Closing;
                (
                    ControlFrame::response(
                        frame.id,
                        Some(serde_json::to_value(Acknowledged::default()).map_err(control_error)?),
                        None,
                    ),
                    ControlOutcome::Close(close.reason),
                )
            }
            INTERRUPT_METHOD | CLOSE_METHOD => (
                error_response(frame.id, INVALID_STATE, "invalid_state"),
                ControlOutcome::Continue,
            ),
            _ => (
                error_response(frame.id, INVALID_PAYLOAD, "unknown_method"),
                ControlOutcome::Continue,
            ),
        };
        self.send(response).await?;
        Ok(outcome)
    }

    /// Forward one admitted telephony input frame to the application media channel.
    pub async fn forward_input_once(&self) -> Result<bool, BindingError> {
        let media = self.ready_media()?;
        let Some(frame) = self.telephony.read_input().await? else {
            return Ok(false);
        };
        let descriptor = &self.telephony.descriptor().media;
        let frame = AudioFrame::new(frame.sequence, frame.bytes, descriptor)?;
        media
            .write_frame(rtvbp::MediaFrame {
                data: frame.bytes,
                pts: Some(
                    descriptor
                        .packet_time()
                        .saturating_mul(u32::try_from(frame.sequence).unwrap_or(u32::MAX)),
                ),
            })
            .await
            .map_err(transport_error)?;
        Ok(true)
    }

    /// Forward one application output frame into the neutral telephony port.
    pub async fn forward_output_once(&self) -> Result<(), BindingError> {
        let media = self.ready_media()?;
        let frame = media.read_frame().await.map_err(transport_error)?;
        let sequence = self.output_sequence.fetch_add(1, Ordering::Relaxed);
        let frame = AudioFrame::new(sequence, frame.data, &self.telephony.descriptor().media)?;
        self.telephony.write_output(frame).await?;
        Ok(())
    }

    /// Forward one optional neutral telephony signal as an RTVBP binding event.
    pub async fn forward_signal_once(&self) -> Result<bool, BindingError> {
        if !self.is_ready()? {
            return Err(BindingError::Control("invalid_state".to_owned()));
        }
        let Some(signal) = self.telephony.next_signal().await? else {
            return Ok(false);
        };
        signal.validate()?;
        self.send_event(
            SIGNAL_EVENT,
            serde_json::to_value(voice::Signal { signal }).map_err(control_error)?,
        )
        .await?;
        Ok(true)
    }

    /// Close transport and telephony exactly once with the supplied neutral reason.
    pub async fn terminate(&self, reason: TerminationReason) -> Result<(), BindingError> {
        {
            let mut state = self.state.lock().map_err(lock_error)?;
            if *state == State::Closed {
                return Ok(());
            }
            *state = State::Closed;
        }
        let terminal_event = self
            .send_event(
                TERMINATED_EVENT,
                serde_json::to_value(voice::Terminated { reason }).map_err(control_error)?,
            )
            .await;
        let (telephony, transport) = tokio::join!(
            async {
                self.telephony
                    .terminate(reason)
                    .await
                    .map_err(BindingError::from)
            },
            async { self.transport.close().await.map_err(transport_error) },
        );
        terminal_event?;
        telephony?;
        transport
    }

    fn is_ready(&self) -> Result<bool, BindingError> {
        Ok(*self.state.lock().map_err(lock_error)? == State::Ready)
    }

    fn ready_media(&self) -> Result<Arc<dyn MediaChannel>, BindingError> {
        if !self.is_ready()? {
            return Err(BindingError::Control("invalid_state".to_owned()));
        }
        self.media
            .lock()
            .map_err(lock_error)?
            .clone()
            .ok_or_else(|| BindingError::Control("media is not open".to_owned()))
    }

    async fn send(&self, frame: ControlFrame) -> Result<(), BindingError> {
        let bytes = self.envelope.encode(&frame).map_err(control_error)?;
        if bytes.len() > MAX_CONTROL_FRAME_BYTES {
            return Err(BindingError::ControlFrameTooLarge);
        }
        self.transport
            .control()
            .send(bytes)
            .await
            .map_err(transport_error)
    }

    async fn send_event(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<(), BindingError> {
        let event_id = format!(
            "voice-{}",
            self.request_sequence.fetch_add(1, Ordering::Relaxed)
        );
        self.send(ControlFrame::event(event_id, event, Some(payload)))
            .await
    }

    async fn receive(&self) -> Result<ControlFrame, BindingError> {
        let received = self
            .transport
            .control()
            .recv()
            .await
            .map_err(transport_error)?;
        if received.data.len() > MAX_CONTROL_FRAME_BYTES {
            return Err(BindingError::ControlFrameTooLarge);
        }
        self.envelope.decode(&received.data).map_err(control_error)
    }
}

#[must_use]
pub fn media_format() -> MediaFormat {
    MediaFormat {
        encoding: "L16".to_owned(),
        sample_rate: 8_000,
        bit_depth: 16,
        channels: 1,
        ptime: Duration::from_millis(20),
    }
}

fn decode_payload<T: serde::de::DeserializeOwned>(frame: &ControlFrame) -> Result<T, BindingError> {
    serde_json::from_value(frame.payload.clone().unwrap_or_default()).map_err(control_error)
}

fn error_response(correlation_id: String, code: i64, message: &str) -> ControlFrame {
    ControlFrame::response(
        correlation_id,
        None,
        Some(WireError {
            code,
            message: message.to_owned(),
            data: None,
        }),
    )
}

fn control_error(error: impl std::fmt::Display) -> BindingError {
    BindingError::Control(error.to_string())
}

fn transport_error(error: rtvbp::Error) -> BindingError {
    match error {
        rtvbp::Error::Transport(message) if message.contains("media queue overloaded") => {
            BindingError::MediaOverload
        }
        other => BindingError::Transport(other.to_string()),
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> BindingError {
    BindingError::Control(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use domain::voice::{
        ChannelSignal, ContextTrust, MediaDescriptor, ParticipantContext, VoiceRef,
        VoiceSessionDescriptor,
    };
    use rtvbp::transport::memory::{Config, MemoryTransport};
    use server::authority::{
        AuthorityIssuer, AuthorityRedeemer, ExpectedAuthority, InMemoryReplayStore, IssueRequest,
        IssuedAuthority, NoRevocations, ProofKey, RedeemedAuthority,
    };
    use tokio::sync::Notify;

    use super::*;

    struct FakeTelephony {
        descriptor: VoiceSessionDescriptor,
        input: Mutex<VecDeque<AudioFrame>>,
        output: Mutex<Vec<AudioFrame>>,
        signals: Mutex<VecDeque<ChannelSignal>>,
        interrupts: AtomicU64,
        terminal: Mutex<Option<TerminationReason>>,
        terminal_ready: Notify,
    }

    #[async_trait::async_trait]
    impl TelephonySession for FakeTelephony {
        fn descriptor(&self) -> &VoiceSessionDescriptor {
            &self.descriptor
        }

        async fn read_input(&self) -> Result<Option<AudioFrame>, VoiceError> {
            Ok(self.input.lock().unwrap().pop_front())
        }

        async fn write_output(&self, frame: AudioFrame) -> Result<(), VoiceError> {
            self.output.lock().unwrap().push(frame);
            Ok(())
        }

        async fn next_signal(&self) -> Result<Option<ChannelSignal>, VoiceError> {
            Ok(self.signals.lock().unwrap().pop_front())
        }

        async fn wait_terminated(&self) -> Result<TerminationReason, VoiceError> {
            loop {
                let notified = self.terminal_ready.notified();
                if let Some(reason) = *self.terminal.lock().unwrap() {
                    return Ok(reason);
                }
                notified.await;
            }
        }

        async fn interrupt_output(&self) -> Result<(), VoiceError> {
            self.interrupts.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn terminate(&self, reason: TerminationReason) -> Result<(), VoiceError> {
            let mut terminal = self.terminal.lock().unwrap();
            if terminal.is_none() {
                *terminal = Some(reason);
                drop(terminal);
                self.terminal_ready.notify_waiters();
            }
            Ok(())
        }
    }

    fn telephony() -> Arc<FakeTelephony> {
        let media = MediaDescriptor::pcm_s16le_8khz_mono_20ms();
        Arc::new(FakeTelephony {
            descriptor: VoiceSessionDescriptor {
                call: VoiceRef::new("call-1").unwrap(),
                session: VoiceRef::new("session-1").unwrap(),
                channel: VoiceRef::new("channel-1").unwrap(),
                participant: ParticipantContext {
                    reference: VoiceRef::new("participant-1").unwrap(),
                    trust: ContextTrust::Untrusted,
                    display: Some("Synthetic caller".to_owned()),
                },
                media: media.clone(),
            },
            input: Mutex::new(VecDeque::from([
                AudioFrame::new(1, vec![1; 320], &media).unwrap()
            ])),
            output: Mutex::new(Vec::new()),
            signals: Mutex::new(VecDeque::from([ChannelSignal::Dtmf {
                digits: "5".to_owned(),
            }])),
            interrupts: AtomicU64::new(0),
            terminal: Mutex::new(None),
            terminal_ready: Notify::new(),
        })
    }

    fn authority_pair() -> (IssuedAuthority, RedeemedAuthority) {
        use ed25519_dalek::SigningKey;

        let issuer = AuthorityIssuer::new("issuer", "key-1", SigningKey::from_bytes(&[3; 32]));
        let proof_key = ProofKey::from_bytes(&[4; 32]);
        let endpoint = "wss://application.example/voice".to_owned();
        let expected = ExpectedAuthority {
            issuer: "issuer".to_owned(),
            audience: "application".to_owned(),
            subject: "principal".to_owned(),
            actor: "voice".to_owned(),
            organization: "org".to_owned(),
            deployment: "application".to_owned(),
            connection: "connection".to_owned(),
            grant: "grant".to_owned(),
            resource: "endpoint".to_owned(),
            operation: "call".to_owned(),
            channel_kind: "voice".to_owned(),
            protocol: PROFILE.to_owned(),
            endpoint: endpoint.clone(),
        };
        let authority = issuer
            .issue(IssueRequest {
                audience: expected.audience.clone(),
                subject: expected.subject.clone(),
                actor: expected.actor.clone(),
                organization: expected.organization.clone(),
                deployment: expected.deployment.clone(),
                connection: expected.connection.clone(),
                grant: expected.grant.clone(),
                resource: expected.resource.clone(),
                operation: expected.operation.clone(),
                channel_kind: expected.channel_kind.clone(),
                protocol: expected.protocol.clone(),
                endpoint: endpoint.clone(),
                proof_thumbprint: proof_key.thumbprint(),
                authority_id: "authority".to_owned(),
                issued_at: 100,
                not_before: 100,
                expires_at: 160,
                lease_expires_at: 1_000,
            })
            .unwrap();
        let proof = proof_key
            .proof("GET", &endpoint, &authority, 101, "proof")
            .unwrap();
        let replay = InMemoryReplayStore::default();
        let presentation = authority.presentation("GET", endpoint, proof);
        let redeemed = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        )
        .redeem(&presentation, &expected, 101)
        .unwrap();
        (authority, redeemed)
    }

    #[tokio::test]
    async fn exact_profile_runs_duplex_memory_conformance_over_fake_telephony() {
        let telephony = telephony();
        let (voice_transport, application_transport) =
            MemoryTransport::pair(Config { media: true });
        let (issued, redeemed) = authority_pair();
        let endpoint = VoiceEndpoint::new(
            Arc::clone(&telephony),
            voice_transport,
            Some(PROFILE),
            &issued,
        )
        .unwrap();

        let application = tokio::spawn(async move {
            assert_eq!(redeemed.claims().dl_protocol, PROFILE);
            let envelope = rtvbp::envelope::v1classic::Envelope;
            let request = application_transport.control().recv().await.unwrap();
            let request = envelope.decode(&request.data).unwrap();
            assert_eq!(request.method, INITIALIZE_METHOD);
            let initialized: voice::Initialize =
                serde_json::from_value(request.payload.unwrap()).unwrap();
            assert_eq!(initialized.media.frame_bytes, 320);
            let response = ControlFrame::response(
                request.id,
                Some(
                    serde_json::to_value(Ready {
                        contract: voice::CONTRACT.to_owned(),
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
            media
                .write_frame(rtvbp::MediaFrame::untimed(vec![2; 320]))
                .await
                .unwrap();
            let inbound = media.read_frame().await.unwrap();
            assert_eq!(inbound.data, vec![1; 320]);
        });

        endpoint.initialize_application().await.unwrap();
        assert!(endpoint.forward_input_once().await.unwrap());
        endpoint.forward_output_once().await.unwrap();
        application.await.unwrap();
        assert_eq!(telephony.output.lock().unwrap()[0].bytes, vec![2; 320]);
    }

    #[tokio::test]
    async fn signal_application_close_and_terminal_event_are_serialized() {
        let telephony = telephony();
        let (voice_transport, application_transport) =
            MemoryTransport::pair(Config { media: true });
        let (issued, _) = authority_pair();
        let endpoint = VoiceEndpoint::new(
            Arc::clone(&telephony),
            voice_transport,
            Some(PROFILE),
            &issued,
        )
        .unwrap();
        let application = async move {
            let envelope = rtvbp::envelope::v1classic::Envelope;
            let initialize = application_transport.control().recv().await.unwrap();
            let initialize = envelope.decode(&initialize.data).unwrap();
            application_transport
                .control()
                .send(
                    envelope
                        .encode(&ControlFrame::response(
                            initialize.id,
                            Some(
                                serde_json::to_value(Ready {
                                    contract: voice::CONTRACT.to_owned(),
                                })
                                .unwrap(),
                            ),
                            None,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            let _media = application_transport.accept_media().await.unwrap();
            let signal = application_transport.control().recv().await.unwrap();
            let signal = envelope.decode(&signal.data).unwrap();
            assert_eq!(signal.kind, FrameKind::Event);
            assert_eq!(signal.method, SIGNAL_EVENT);
            let signal: voice::Signal = serde_json::from_value(signal.payload.unwrap()).unwrap();
            assert_eq!(
                signal.signal,
                ChannelSignal::Dtmf {
                    digits: "5".to_owned()
                }
            );
            application_transport
                .control()
                .send(
                    envelope
                        .encode(&ControlFrame::request(
                            "close-1",
                            CLOSE_METHOD,
                            Some(
                                serde_json::to_value(Close {
                                    reason: TerminationReason::Completed,
                                })
                                .unwrap(),
                            ),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            let acknowledged = application_transport.control().recv().await.unwrap();
            let acknowledged = envelope.decode(&acknowledged.data).unwrap();
            assert_eq!(acknowledged.kind, FrameKind::Response);
            assert_eq!(acknowledged.correlation_id, "close-1");
            let terminal = application_transport.control().recv().await.unwrap();
            let terminal = envelope.decode(&terminal.data).unwrap();
            assert_eq!(terminal.kind, FrameKind::Event);
            assert_eq!(terminal.method, TERMINATED_EVENT);
            let terminal: voice::Terminated =
                serde_json::from_value(terminal.payload.unwrap()).unwrap();
            assert_eq!(terminal.reason, TerminationReason::Completed);
        };
        let voice = async {
            endpoint.initialize_application().await.unwrap();
            assert!(endpoint.forward_signal_once().await.unwrap());
            assert_eq!(
                endpoint.serve_control_once().await.unwrap(),
                ControlOutcome::Close(TerminationReason::Completed)
            );
            endpoint
                .terminate(TerminationReason::Completed)
                .await
                .unwrap();
        };
        tokio::join!(application, voice);
        assert_eq!(
            *telephony.terminal.lock().unwrap(),
            Some(TerminationReason::Completed)
        );
    }

    #[test]
    fn absent_and_product_profiles_refuse_by_name() {
        assert_eq!(negotiate_profile(None), Err(BindingError::ProfileRequired));
        assert_eq!(
            negotiate_profile(Some("babelforce.v1")),
            Err(BindingError::ProfileRefused("babelforce.v1".to_owned()))
        );
    }
}
