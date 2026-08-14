#![forbid(unsafe_code)]

//! One supervised runtime leaf joining admitted SIP telephony to authenticated RTVBP.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use domain::voice::{TerminationReason, VoiceSessionDescriptor};
use protocol::sip::{SipDialEstablished, SipDialState};
use rtvbp::{KeepalivePolicy, Transport as _};
use rtvbp_voice_endpoint::bounded_ws::Bounds;
use rtvbp_voice_endpoint::connect::connect_authenticated;
use rtvbp_voice_endpoint::{BindingError, ControlOutcome, VoiceEndpoint, PROFILE};
use server::authority::{AuthorityIssuer, IssueRequest, ProofKey};
use server::{AdmittedVoicePlan, CredentialSet, VoiceApplicationRoute, VOICE_APPLICATION_PROFILE};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{oneshot, Notify};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

const SESSION_ID_BYTES: usize = 16;
const MAX_SESSION_ID_BYTES: usize = 128;

/// An already established TLS application stream.
pub trait ApplicationStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> ApplicationStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// Redaction-safe failure from a deployment-owned runtime dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct DependencyError {
    code: &'static str,
}

impl DependencyError {
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }

    #[must_use]
    pub const fn code(self) -> &'static str {
        self.code
    }
}

/// Resolve operation-scoped SIP credentials after admission; values remain inside the runtime.
#[async_trait]
pub trait CredentialSource: Send + Sync {
    async fn resolve(&self, admitted: &AdmittedVoicePlan)
        -> Result<CredentialSet, DependencyError>;
}

/// Establish DNS/TCP/proxy/TLS according to deployment policy for the exact admitted route.
#[async_trait]
pub trait ApplicationConnector: Send + Sync {
    async fn connect(
        &self,
        route: &VoiceApplicationRoute,
    ) -> Result<Box<dyn ApplicationStream>, DependencyError>;
}

/// Wall clock used for short-lived authority and proof timestamps.
pub trait Clock: Send + Sync {
    fn now_epoch_seconds(&self) -> Result<u64, DependencyError>;
}

/// Production wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_seconds(&self) -> Result<u64, DependencyError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| DependencyError::new("system_clock_before_epoch"))
    }
}

/// Fresh proof key and one-shot identifiers generated before network establishment.
pub struct SessionMaterial {
    pub proof_key: ProofKey,
    pub authority_id: String,
    pub proof_id: String,
}

/// Supplies per-session cryptographic material; deterministic implementations are test-only.
pub trait SessionMaterialSource: Send + Sync {
    fn generate(&self) -> Result<SessionMaterial, DependencyError>;
}

/// Operating-system randomness for production session material.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsSessionMaterial;

impl SessionMaterialSource for OsSessionMaterial {
    fn generate(&self) -> Result<SessionMaterial, DependencyError> {
        let mut proof_key = [0_u8; 32];
        let mut authority = [0_u8; SESSION_ID_BYTES];
        let mut proof = [0_u8; SESSION_ID_BYTES];
        getrandom::fill(&mut proof_key)
            .and_then(|()| getrandom::fill(&mut authority))
            .and_then(|()| getrandom::fill(&mut proof))
            .map_err(|_| DependencyError::new("operating_system_randomness_unavailable"))?;
        let key = ProofKey::from_bytes(&proof_key);
        proof_key.fill(0);
        Ok(SessionMaterial {
            proof_key: key,
            authority_id: opaque_id("authority", &authority),
            proof_id: opaque_id("proof", &proof),
        })
    }
}

/// Lifecycle source that won the single serialized terminal transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSource {
    External,
    Application,
    Telephony,
    Lease,
    Media,
    Transport,
    Protocol,
}

/// One payload-free observation for audit/metrics integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceObservation<'a> {
    Established {
        descriptor: &'a VoiceSessionDescriptor,
    },
    Terminated {
        descriptor: Option<&'a VoiceSessionDescriptor>,
        reason: TerminationReason,
        source: TerminalSource,
    },
}

/// Infallible observation boundary. Implementations enqueue bounded, payload-free facts.
pub trait VoiceObserver: Send + Sync {
    fn observe(&self, observation: VoiceObservation<'_>);
}

/// Observer used when a deployment has no extra metrics sink configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopObserver;

impl VoiceObserver for NoopObserver {
    fn observe(&self, _observation: VoiceObservation<'_>) {}
}

/// Redaction-safe refusal when a dial invocation ends before it can return an established handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("sip.dial ended before the voice session was established")]
pub struct DialEstablishmentError;

/// One-shot observer used by an operation server to return `sip.dial` as soon as both sides bind.
pub struct DialEstablishmentObserver {
    result: Mutex<Option<oneshot::Sender<Result<SipDialEstablished, DialEstablishmentError>>>>,
}

/// Awaitable half of [`dial_establishment_channel`].
pub struct DialEstablishmentWaiter {
    result: oneshot::Receiver<Result<SipDialEstablished, DialEstablishmentError>>,
}

/// Create the observer/waiter pair that bridges the supervised session to operation invocation.
#[must_use]
pub fn dial_establishment_channel() -> (DialEstablishmentObserver, DialEstablishmentWaiter) {
    let (sender, result) = oneshot::channel();
    (
        DialEstablishmentObserver {
            result: Mutex::new(Some(sender)),
        },
        DialEstablishmentWaiter { result },
    )
}

impl DialEstablishmentWaiter {
    /// Wait until the session is established or its supervisor reports a pre-establishment end.
    pub async fn wait(self) -> Result<SipDialEstablished, DialEstablishmentError> {
        self.result.await.unwrap_or(Err(DialEstablishmentError))
    }
}

impl VoiceObserver for DialEstablishmentObserver {
    fn observe(&self, observation: VoiceObservation<'_>) {
        let result = match observation {
            VoiceObservation::Established { descriptor } => Ok(SipDialEstablished {
                call: descriptor.call.as_str().to_owned(),
                session: descriptor.session.as_str().to_owned(),
                channel: descriptor.channel.as_str().to_owned(),
                state: SipDialState::Established,
            }),
            VoiceObservation::Terminated { .. } => Err(DialEstablishmentError),
        };
        if let Some(sender) = lock(&self.result).take() {
            let _ = sender.send(result);
        }
    }
}

/// Finite transport and liveness policy for one runtime generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub transport: Bounds,
    pub keepalive: KeepalivePolicy,
    pub shutdown_deadline: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            transport: Bounds::voice_v1(),
            keepalive: KeepalivePolicy {
                interval: Duration::from_secs(15),
                timeout: Duration::from_secs(5),
                max_misses: 3,
            },
            shutdown_deadline: Duration::from_secs(5),
        }
    }
}

/// Cloneable first-wins cancellation, drain, revocation, or shutdown input.
#[derive(Clone, Default)]
pub struct VoiceSessionControl {
    terminal: Arc<TerminalCell>,
}

#[derive(Default)]
struct TerminalCell {
    selection: Mutex<Option<TerminalSelection>>,
    ready: Notify,
    cancelled: CancellationToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalSelection {
    reason: TerminationReason,
    source: TerminalSource,
}

impl VoiceSessionControl {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the reason only if no terminal fact already won.
    pub fn terminate(&self, reason: TerminationReason) -> bool {
        let mut selected = lock(&self.terminal.selection);
        if selected.is_some() {
            return false;
        }
        *selected = Some(TerminalSelection {
            reason,
            source: TerminalSource::External,
        });
        drop(selected);
        self.terminal.cancelled.cancel();
        self.terminal.ready.notify_waiters();
        true
    }

    async fn wait(&self) -> TerminalSelection {
        loop {
            let notified = self.terminal.ready.notified();
            if let Some(selection) = *lock(&self.terminal.selection) {
                return selection;
            }
            notified.await;
        }
    }

    fn select(&self, reason: TerminationReason, source: TerminalSource) -> TerminalSelection {
        let mut selected = lock(&self.terminal.selection);
        if let Some(selected) = *selected {
            return selected;
        }
        let selected_value = TerminalSelection { reason, source };
        *selected = Some(selected_value);
        drop(selected);
        self.terminal.cancelled.cancel();
        self.terminal.ready.notify_waiters();
        selected_value
    }
}

/// The one terminal result returned after all owned transports have been asked to close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceSessionResult {
    pub descriptor: Option<VoiceSessionDescriptor>,
    pub reason: TerminationReason,
    pub source: TerminalSource,
    pub cleanup_error: bool,
}

/// Failure before a neutral telephony session exists.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("credential custody failed: {0}")]
    Credentials(DependencyError),
    #[error("session material failed: {0}")]
    Material(DependencyError),
    #[error("session clock failed: {0}")]
    Clock(DependencyError),
    #[error("session material contains an invalid opaque identifier")]
    InvalidMaterial,
    #[error("session authority or lease deadline is not representable")]
    InvalidDeadline,
    #[error("session authority could not be issued: {0}")]
    Authority(#[from] server::authority::AuthorityError),
    #[error("sipx telephony establishment failed: {0}")]
    Telephony(#[from] driver_sip::DriverError),
}

/// The only production object that imports both adapter closures.
pub struct VoiceRuntime<'a> {
    issuer: &'a AuthorityIssuer,
    credentials: &'a dyn CredentialSource,
    application: &'a dyn ApplicationConnector,
    clock: &'a dyn Clock,
    material: &'a dyn SessionMaterialSource,
    observer: &'a dyn VoiceObserver,
    config: RuntimeConfig,
}

impl<'a> VoiceRuntime<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer: &'a AuthorityIssuer,
        credentials: &'a dyn CredentialSource,
        application: &'a dyn ApplicationConnector,
        clock: &'a dyn Clock,
        material: &'a dyn SessionMaterialSource,
        observer: &'a dyn VoiceObserver,
        config: RuntimeConfig,
    ) -> Self {
        debug_assert_eq!(VOICE_APPLICATION_PROFILE, PROFILE);
        Self {
            issuer,
            credentials,
            application,
            clock,
            material,
            observer,
            config,
        }
    }

    /// Establish and supervise one admitted outbound call as one cancellation-safe owner task.
    pub async fn run_outbound(
        &self,
        admitted: &AdmittedVoicePlan,
        control: VoiceSessionControl,
    ) -> Result<VoiceSessionResult, RuntimeError> {
        let credentials = tokio::select! {
            selected = control.wait() => {
                return Ok(self.pre_session_terminal(selected));
            }
            credentials = self.credentials.resolve(admitted) => {
                credentials.map_err(RuntimeError::Credentials)?
            }
        };
        let material = self.material.generate().map_err(RuntimeError::Material)?;
        validate_material(&material)?;
        let SessionMaterial {
            proof_key,
            authority_id,
            proof_id,
        } = material;
        let now = self
            .clock
            .now_epoch_seconds()
            .map_err(RuntimeError::Clock)?;
        let route = admitted.application();
        let authority_seconds = route.authority_lifetime.as_secs();
        let lease_seconds = route.session_lease.as_secs();
        let session_started = Instant::now();
        let authority_deadline = session_started
            .checked_add(route.authority_lifetime)
            .ok_or(RuntimeError::InvalidDeadline)?;
        let lease_deadline = session_started
            .checked_add(route.session_lease)
            .ok_or(RuntimeError::InvalidDeadline)?;
        let authority = self.issuer.issue(IssueRequest {
            audience: route.audience.clone(),
            subject: admitted.sip().principal().to_owned(),
            actor: route.actor.clone(),
            organization: admitted.sip().organization().to_owned(),
            deployment: route.deployment.clone(),
            connection: admitted.sip().route().connection.clone(),
            grant: admitted.sip().grant().to_owned(),
            resource: route.resource.clone(),
            operation: admitted.sip().operation().to_owned(),
            channel_kind: "voice".to_owned(),
            protocol: PROFILE.to_owned(),
            endpoint: route.endpoint.clone(),
            proof_thumbprint: proof_key.thumbprint(),
            authority_id,
            issued_at: now,
            not_before: now,
            expires_at: now
                .checked_add(authority_seconds)
                .ok_or(RuntimeError::InvalidDeadline)?,
            lease_expires_at: now
                .checked_add(lease_seconds)
                .ok_or(RuntimeError::InvalidDeadline)?,
        })?;

        // sipx owns both the finite answer deadline and protocol-correct cancellation. When the
        // runtime terminal wins, `dial_until` withdraws an outstanding INVITE before returning.
        let telephony = match driver_sip::establish_outbound(
            admitted.sip(),
            &credentials,
            control.terminal.cancelled.clone(),
        )
        .await
        {
            Ok(telephony) => telephony,
            Err(driver_sip::DriverError::Cancelled) => {
                let selected = control.selected().unwrap_or_else(|| {
                    control.select(TerminationReason::Cancelled, TerminalSource::External)
                });
                return Ok(self.pre_session_terminal(selected));
            }
            Err(error) => return Err(RuntimeError::Telephony(error)),
        };
        let descriptor = telephony.descriptor().clone();
        if let Some(selected) = control.selected() {
            return Ok(self.finish_raw(telephony, descriptor, selected).await);
        }

        let stream = tokio::select! {
            selected = control.wait() => {
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
            () = tokio::time::sleep_until(authority_deadline) => {
                let selected = control.select(TerminationReason::ProtocolError, TerminalSource::Protocol);
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
            stream = self.application.connect(route) => match stream {
                Ok(stream) => stream,
                Err(_) => {
                    let selected = control.select(TerminationReason::TransportLost, TerminalSource::Transport);
                    return Ok(self.finish_raw(telephony, descriptor, selected).await);
                }
            }
        };
        let proof_now = match self.clock.now_epoch_seconds() {
            Ok(now) => now,
            Err(_) => {
                let selected =
                    control.select(TerminationReason::ProtocolError, TerminalSource::Protocol);
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
        };
        let transport = tokio::select! {
            selected = control.wait() => {
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
            () = tokio::time::sleep_until(authority_deadline) => {
                let selected = control.select(TerminationReason::ProtocolError, TerminalSource::Protocol);
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
            transport = connect_authenticated(
                stream,
                &authority,
                &proof_key,
                proof_now,
                proof_id,
                self.config.transport.clone(),
            ) => match transport {
                Ok(transport) => transport,
                Err(_) => {
                    let selected = control.select(TerminationReason::ProtocolError, TerminalSource::Protocol);
                    return Ok(self.finish_raw(telephony, descriptor, selected).await);
                }
            }
        };
        let endpoint = match VoiceEndpoint::new(
            Arc::clone(&telephony),
            Arc::clone(&transport) as Arc<dyn rtvbp::Transport>,
            Some(PROFILE),
            &authority,
        ) {
            Ok(endpoint) => Arc::new(endpoint),
            Err(_) => {
                let selected =
                    control.select(TerminationReason::ProtocolError, TerminalSource::Protocol);
                return Ok(self.finish_raw(telephony, descriptor, selected).await);
            }
        };
        let initialized = tokio::select! {
            selected = control.wait() => Err(selected),
            () = tokio::time::sleep_until(authority_deadline) => {
                Err(TerminalSelection { reason: TerminationReason::ProtocolError, source: TerminalSource::Protocol })
            }
            result = endpoint.initialize_application() => match result {
                Ok(()) => Ok(()),
                Err(error) => {
                    let (reason, source) = binding_terminal(&error);
                    Err(TerminalSelection { reason, source })
                },
            }
        };
        if let Err(candidate) = initialized {
            let selected = control.select(candidate.reason, candidate.source);
            let cleanup_error = self.terminate_endpoint(&endpoint, selected.reason).await;
            self.observer.observe(VoiceObservation::Terminated {
                descriptor: Some(&descriptor),
                reason: selected.reason,
                source: selected.source,
            });
            return Ok(VoiceSessionResult {
                descriptor: Some(descriptor),
                reason: selected.reason,
                source: selected.source,
                cleanup_error,
            });
        }
        self.observer.observe(VoiceObservation::Established {
            descriptor: &descriptor,
        });

        let input = input_pump(Arc::clone(&endpoint));
        let output = output_pump(Arc::clone(&endpoint));
        let signals = signal_pump(Arc::clone(&endpoint));
        let telephony_terminal = telephony_terminal_pump(Arc::clone(&telephony));
        let controls = control_pump(Arc::clone(&endpoint));
        let keepalive = keepalive_pump(Arc::clone(&transport), self.config.keepalive);
        let media_loss = media_loss_monitor(Arc::clone(&transport));
        tokio::pin!(
            input,
            output,
            signals,
            telephony_terminal,
            controls,
            keepalive,
            media_loss
        );
        let candidate = tokio::select! {
            selected = control.wait() => selected,
            terminal = &mut input => TerminalSelection { reason: terminal.0, source: terminal.1 },
            terminal = &mut output => TerminalSelection { reason: terminal.0, source: terminal.1 },
            terminal = &mut signals => TerminalSelection { reason: terminal.0, source: terminal.1 },
            terminal = &mut telephony_terminal => terminal,
            terminal = &mut controls => TerminalSelection { reason: terminal.0, source: terminal.1 },
            terminal = &mut keepalive => terminal,
            terminal = &mut media_loss => terminal,
            () = tokio::time::sleep_until(lease_deadline) => {
                TerminalSelection { reason: TerminationReason::LeaseExpired, source: TerminalSource::Lease }
            }
        };
        let selected = control.select(candidate.reason, candidate.source);
        let cleanup_error = self.terminate_endpoint(&endpoint, selected.reason).await;
        self.observer.observe(VoiceObservation::Terminated {
            descriptor: Some(&descriptor),
            reason: selected.reason,
            source: selected.source,
        });
        Ok(VoiceSessionResult {
            descriptor: Some(descriptor),
            reason: selected.reason,
            source: selected.source,
            cleanup_error,
        })
    }

    fn pre_session_terminal(&self, selected: TerminalSelection) -> VoiceSessionResult {
        self.observer.observe(VoiceObservation::Terminated {
            descriptor: None,
            reason: selected.reason,
            source: selected.source,
        });
        VoiceSessionResult {
            descriptor: None,
            reason: selected.reason,
            source: selected.source,
            cleanup_error: false,
        }
    }

    async fn finish_raw(
        &self,
        telephony: Arc<dyn domain::voice::TelephonySession>,
        descriptor: VoiceSessionDescriptor,
        selected: TerminalSelection,
    ) -> VoiceSessionResult {
        let cleanup_error = tokio::time::timeout(
            self.config.shutdown_deadline,
            telephony.terminate(selected.reason),
        )
        .await
        .map_or(true, |result| result.is_err());
        self.observer.observe(VoiceObservation::Terminated {
            descriptor: Some(&descriptor),
            reason: selected.reason,
            source: selected.source,
        });
        VoiceSessionResult {
            descriptor: Some(descriptor),
            reason: selected.reason,
            source: selected.source,
            cleanup_error,
        }
    }

    async fn terminate_endpoint(
        &self,
        endpoint: &VoiceEndpoint<dyn domain::voice::TelephonySession>,
        reason: TerminationReason,
    ) -> bool {
        tokio::time::timeout(self.config.shutdown_deadline, endpoint.terminate(reason))
            .await
            .map_or(true, |result| result.is_err())
    }
}

impl VoiceSessionControl {
    fn selected(&self) -> Option<TerminalSelection> {
        *lock(&self.terminal.selection)
    }
}

async fn input_pump(
    endpoint: Arc<VoiceEndpoint<dyn domain::voice::TelephonySession>>,
) -> (TerminationReason, TerminalSource) {
    loop {
        match endpoint.forward_input_once().await {
            Ok(true) => {}
            Ok(false) => return std::future::pending().await,
            Err(error) if waits_for_telephony_terminal(&error) => {
                return std::future::pending().await;
            }
            Err(error) => return binding_terminal(&error),
        }
    }
}

async fn output_pump(
    endpoint: Arc<VoiceEndpoint<dyn domain::voice::TelephonySession>>,
) -> (TerminationReason, TerminalSource) {
    loop {
        match endpoint.forward_output_once().await {
            Ok(()) => {}
            Err(error) if waits_for_telephony_terminal(&error) => {
                return std::future::pending().await;
            }
            Err(error) => return binding_terminal(&error),
        }
    }
}

async fn signal_pump(
    endpoint: Arc<VoiceEndpoint<dyn domain::voice::TelephonySession>>,
) -> (TerminationReason, TerminalSource) {
    loop {
        match endpoint.forward_signal_once().await {
            Ok(true) => {}
            Ok(false) => return std::future::pending().await,
            Err(error) if waits_for_telephony_terminal(&error) => {
                return std::future::pending().await;
            }
            Err(error) => return binding_terminal(&error),
        }
    }
}

async fn telephony_terminal_pump(
    telephony: Arc<dyn domain::voice::TelephonySession>,
) -> TerminalSelection {
    match telephony.wait_terminated().await {
        Ok(reason) => telephony_terminal(reason),
        Err(_) => TerminalSelection {
            reason: TerminationReason::TransportLost,
            source: TerminalSource::Telephony,
        },
    }
}

fn telephony_terminal(reason: TerminationReason) -> TerminalSelection {
    TerminalSelection {
        reason,
        source: TerminalSource::Telephony,
    }
}

async fn control_pump(
    endpoint: Arc<VoiceEndpoint<dyn domain::voice::TelephonySession>>,
) -> (TerminationReason, TerminalSource) {
    loop {
        match endpoint.serve_control_once().await {
            Ok(ControlOutcome::Continue) => {}
            Ok(ControlOutcome::Close(reason)) => return (reason, TerminalSource::Application),
            Err(error) if waits_for_telephony_terminal(&error) => {
                return std::future::pending().await;
            }
            Err(error) => return binding_terminal(&error),
        }
    }
}

async fn keepalive_pump(
    transport: Arc<rtvbp_voice_endpoint::bounded_ws::BoundedWsTransport>,
    policy: KeepalivePolicy,
) -> TerminalSelection {
    if !policy.enabled() {
        return std::future::pending::<TerminalSelection>().await;
    }
    let (reason, source) = match transport.monitor_keepalive(policy).await {
        Ok(()) => (TerminationReason::TransportLost, TerminalSource::Transport),
        Err(error) => transport_terminal(&error),
    };
    TerminalSelection { reason, source }
}

async fn media_loss_monitor(
    transport: Arc<rtvbp_voice_endpoint::bounded_ws::BoundedWsTransport>,
) -> TerminalSelection {
    let _loss = transport.wait_incoming_media_loss().await;
    TerminalSelection {
        reason: TerminationReason::MediaOverload,
        source: TerminalSource::Media,
    }
}

fn binding_terminal(error: &BindingError) -> (TerminationReason, TerminalSource) {
    match error {
        BindingError::Voice(_) => (TerminationReason::TransportLost, TerminalSource::Telephony),
        BindingError::Transport(_) => (TerminationReason::TransportLost, TerminalSource::Transport),
        BindingError::MediaOverload => (TerminationReason::MediaOverload, TerminalSource::Media),
        _ => (TerminationReason::ProtocolError, TerminalSource::Protocol),
    }
}

fn waits_for_telephony_terminal(error: &BindingError) -> bool {
    matches!(
        error,
        BindingError::Voice(domain::voice::VoiceError::Terminated)
    )
}

fn transport_terminal(error: &rtvbp::Error) -> (TerminationReason, TerminalSource) {
    match error {
        rtvbp::Error::Transport(message) if message.contains("media queue overloaded") => {
            (TerminationReason::MediaOverload, TerminalSource::Media)
        }
        _ => (TerminationReason::TransportLost, TerminalSource::Transport),
    }
}

fn validate_material(material: &SessionMaterial) -> Result<(), RuntimeError> {
    if material.authority_id.is_empty()
        || material.proof_id.is_empty()
        || material.authority_id.len() > MAX_SESSION_ID_BYTES
        || material.proof_id.len() > MAX_SESSION_ID_BYTES
        || !material.authority_id.is_ascii()
        || !material.proof_id.is_ascii()
    {
        return Err(RuntimeError::InvalidMaterial);
    }
    Ok(())
}

fn opaque_id(prefix: &str, bytes: &[u8]) -> String {
    let mut id = String::with_capacity(prefix.len() + 1 + bytes.len() * 2);
    id.push_str(prefix);
    id.push('-');
    for byte in bytes {
        let _ = write!(&mut id, "{byte:02x}");
    }
    id
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telephony_terminal_reason_is_not_reclassified_from_stream_eof() {
        assert_eq!(
            telephony_terminal(TerminationReason::TransportLost),
            TerminalSelection {
                reason: TerminationReason::TransportLost,
                source: TerminalSource::Telephony,
            }
        );
        assert!(waits_for_telephony_terminal(&BindingError::Voice(
            domain::voice::VoiceError::Terminated,
        )));
    }

    #[test]
    fn selecting_a_terminal_cancels_pre_session_establishment() {
        let control = VoiceSessionControl::new();
        assert!(!control.terminal.cancelled.is_cancelled());
        assert!(control.terminate(TerminationReason::AuthorityRevoked));
        assert!(control.terminal.cancelled.is_cancelled());
    }
}
