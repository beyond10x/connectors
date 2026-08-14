use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use domain::{
    AdmittedOperation, Capability, Implementation, Interaction, OperationFacts, Placement,
    ProtocolPlan, SipPlan, ZeroIoPlan,
};
use futures_util::{SinkExt as _, StreamExt as _};
use protocol::voice::{Acknowledged, Close, Ready, Terminated};
use rtvbp::{ControlFrame, Envelope as _, FrameKind};
use rtvbp_voice_endpoint::{CLOSE_METHOD, INITIALIZE_METHOD, PROFILE, TERMINATED_EVENT};
use server::authority::{
    AuthorityIssuer, AuthorityRedeemer, ExpectedAuthority, InMemoryReplayStore, NoRevocations,
    ProofKey, RedemptionRequest, AUTHORIZATION_SCHEME, DPOP_HEADER,
};
use server::{
    admit_voice_plan, AdmittedVoicePlan, CredentialSet, SipDeploymentRoute, SocketAperture,
    VoiceApplicationRoute,
};
use sipx_call::Codecs;
use sipx_transport::Config;
use tokio::io::DuplexStream;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::{header, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use voice_runtime::{
    ApplicationConnector, ApplicationStream, CredentialSource, DependencyError, NoopObserver,
    RuntimeConfig, SessionMaterial, SessionMaterialSource, SystemClock, TerminalSource,
    VoiceRuntime, VoiceSessionControl,
};

const ENDPOINT: &str = "wss://application.example/voice";
const SAMPLES_PER_FRAME: usize = 160;

struct EmptyCredentials;

#[async_trait]
impl CredentialSource for EmptyCredentials {
    async fn resolve(
        &self,
        _admitted: &AdmittedVoicePlan,
    ) -> Result<CredentialSet, DependencyError> {
        Ok(CredentialSet::default())
    }
}

struct OneStream(Mutex<Option<DuplexStream>>);

#[async_trait]
impl ApplicationConnector for OneStream {
    async fn connect(
        &self,
        route: &VoiceApplicationRoute,
    ) -> Result<Box<dyn ApplicationStream>, DependencyError> {
        assert_eq!(route.endpoint, ENDPOINT);
        self.0
            .lock()
            .unwrap()
            .take()
            .map(|stream| Box::new(stream) as Box<dyn ApplicationStream>)
            .ok_or_else(|| DependencyError::new("test_stream_already_claimed"))
    }
}

struct FixedMaterial;

impl SessionMaterialSource for FixedMaterial {
    fn generate(&self) -> Result<SessionMaterial, DependencyError> {
        Ok(SessionMaterial {
            proof_key: ProofKey::from_bytes(&[4; 32]),
            authority_id: "authority-runtime-1".to_owned(),
            proof_id: "proof-runtime-1".to_owned(),
        })
    }
}

fn loopback() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

fn plan() -> ZeroIoPlan {
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
            "org-1",
            "principal-1",
            "grant-1",
            "connection-1",
        ),
        ProtocolPlan::SipV1(SipPlan {
            connection: "connection-1".to_owned(),
        }),
    )
}

fn application_route() -> VoiceApplicationRoute {
    VoiceApplicationRoute {
        actor: "connectors-voice".to_owned(),
        audience: "application-voice".to_owned(),
        deployment: "application-1".to_owned(),
        resource: "channel-1".to_owned(),
        endpoint: ENDPOINT.to_owned(),
        authority_lifetime: Duration::from_secs(30),
        session_lease: Duration::from_secs(60),
    }
}

fn expected_authority() -> ExpectedAuthority {
    ExpectedAuthority {
        issuer: "https://connectors.example".to_owned(),
        audience: "application-voice".to_owned(),
        subject: "principal-1".to_owned(),
        actor: "connectors-voice".to_owned(),
        organization: "org-1".to_owned(),
        deployment: "application-1".to_owned(),
        connection: "connection-1".to_owned(),
        grant: "grant-1".to_owned(),
        resource: "channel-1".to_owned(),
        operation: "loopback-call-establish".to_owned(),
        channel_kind: "voice".to_owned(),
        protocol: PROFILE.to_owned(),
        endpoint: ENDPOINT.to_owned(),
    }
}

#[tokio::test]
#[allow(clippy::result_large_err)]
async fn supervised_leaf_runs_real_sip_authenticated_rtvbp_and_one_terminal_result() {
    let (callee, mut callee_incoming) =
        sipx_transport::bind(Config::new(SocketAddr::new(loopback(), 0)))
            .await
            .unwrap();
    let aperture = SocketAperture::new(loopback(), 1..=u16::MAX).unwrap();
    let admitted = admit_voice_plan(
        &plan(),
        SipDeploymentRoute {
            connection: "connection-1".to_owned(),
            signaling_bind: SocketAddr::new(loopback(), 0),
            sent_by: "127.0.0.1".to_owned(),
            target: callee.local_addr(),
            to_uri: format!("sip:callee@{}", callee.local_addr()),
            from_uri: "sip:caller@127.0.0.1".to_owned(),
            media_advertised: loopback(),
            media_bind: loopback(),
            signaling_apertures: vec![aperture.clone()],
            media_apertures: vec![aperture],
            dial_timeout: Duration::from_secs(5),
            development_loopback_only: true,
        },
        application_route(),
    )
    .unwrap();

    let (media_tx, media_rx) = oneshot::channel();
    let callee_owner = callee.clone();
    let callee_task = tokio::spawn(async move {
        let invitation = callee_incoming.recv().await.unwrap();
        let mut call = sipx_call::answer_with(&callee_owner, &invitation, loopback(), Codecs::G711)
            .await
            .unwrap();
        let ack = callee_incoming.recv().await.unwrap();
        assert!(call.handle(&ack).await.unwrap());
        media_tx.send(call.media_handle()).unwrap();
        sipx_call::serve(&mut call, &mut callee_incoming)
            .await
            .unwrap();
        callee_owner.shutdown().await;
    });

    let issuer = AuthorityIssuer::new(
        "https://connectors.example",
        "key-1",
        ed25519_dalek::SigningKey::from_bytes(&[3; 32]),
    );
    let verifying_key = issuer.verifying_key();
    let (client_io, server_io) = tokio::io::duplex(65_536);
    let (output_seen_tx, output_seen_rx) = oneshot::channel();
    let connector = OneStream(Mutex::new(Some(client_io)));
    let application_task = tokio::spawn(async move {
        let replay = InMemoryReplayStore::default();
        let revocations = NoRevocations;
        let redeemer = AuthorityRedeemer::new(
            "https://connectors.example",
            "key-1",
            verifying_key,
            &replay,
            &revocations,
        );
        let expected = expected_authority();
        let mut socket = tokio_tungstenite::accept_hdr_async(
            server_io,
            |request: &Request, mut response: Response| {
                assert_eq!(request.uri().path(), "/voice");
                let authorization = request
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap();
                assert!(authorization.starts_with(&format!("{AUTHORIZATION_SCHEME} ")));
                let dpop = request
                    .headers()
                    .get(DPOP_HEADER)
                    .and_then(|value| value.to_str().ok())
                    .unwrap();
                let presentation =
                    RedemptionRequest::from_wire("GET", ENDPOINT, authorization, dpop).unwrap();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let redeemed = redeemer.redeem(&presentation, &expected, now).unwrap();
                assert_eq!(redeemed.claims().dl_protocol, PROFILE);
                response.headers_mut().insert(
                    header::SEC_WEBSOCKET_PROTOCOL,
                    HeaderValue::from_static(PROFILE),
                );
                Ok(response)
            },
        )
        .await
        .unwrap();

        let envelope = rtvbp::envelope::v1classic::Envelope;
        let initialize = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let initialize = envelope.decode(initialize.as_bytes()).unwrap();
        assert_eq!(initialize.kind, FrameKind::Request);
        assert_eq!(initialize.method, INITIALIZE_METHOD);
        let ready = ControlFrame::response(
            initialize.id,
            Some(
                serde_json::to_value(Ready {
                    contract: protocol::voice::CONTRACT.to_owned(),
                })
                .unwrap(),
            ),
            None,
        );
        socket
            .send(Message::Text(
                String::from_utf8(envelope.encode(&ready).unwrap())
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();

        let input = socket.next().await.unwrap().unwrap().into_data();
        assert_eq!(input.len(), 320);
        socket
            .send(Message::Binary(
                [0x2e, 0x16].repeat(SAMPLES_PER_FRAME).into(),
            ))
            .await
            .unwrap();
        output_seen_rx.await.unwrap();
        let close = ControlFrame::request(
            "close-1",
            CLOSE_METHOD,
            Some(
                serde_json::to_value(Close {
                    reason: domain::voice::TerminationReason::Completed,
                })
                .unwrap(),
            ),
        );
        socket
            .send(Message::Text(
                String::from_utf8(envelope.encode(&close).unwrap())
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();

        let acknowledged = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let acknowledged = envelope.decode(acknowledged.as_bytes()).unwrap();
        assert_eq!(acknowledged.kind, FrameKind::Response);
        assert_eq!(acknowledged.correlation_id, "close-1");
        let _: Acknowledged = serde_json::from_value(acknowledged.payload.unwrap()).unwrap();
        let terminal = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let terminal = envelope.decode(terminal.as_bytes()).unwrap();
        assert_eq!(terminal.kind, FrameKind::Event);
        assert_eq!(terminal.method, TERMINATED_EVENT);
        let terminal: Terminated = serde_json::from_value(terminal.payload.unwrap()).unwrap();
        assert_eq!(terminal.reason, domain::voice::TerminationReason::Completed);
        let closed = socket.next().await.unwrap().unwrap();
        assert!(closed.is_close());
    });

    let runtime = VoiceRuntime::new(
        &issuer,
        &EmptyCredentials,
        &connector,
        &SystemClock,
        &FixedMaterial,
        &NoopObserver,
        RuntimeConfig::default(),
    );
    let media_roundtrip = async move {
        let media = media_rx.await.unwrap();
        let source = vec![1_234_i16; SAMPLES_PER_FRAME];
        let played = media.play(&source, SAMPLES_PER_FRAME);
        let recorded = media.record_at_least(SAMPLES_PER_FRAME, Duration::from_secs(5));
        let (played, recorded) = tokio::join!(played, recorded);
        assert!(played);
        output_seen_tx.send(()).unwrap();
        recorded
    };
    let (result, recorded) = tokio::join!(
        runtime.run_outbound(&admitted, VoiceSessionControl::new()),
        media_roundtrip,
    );
    let result = result.unwrap();
    assert_eq!(result.reason, domain::voice::TerminationReason::Completed);
    assert_eq!(result.source, TerminalSource::Application);
    assert!(!result.cleanup_error);
    assert!(result.descriptor.is_some());
    assert_eq!(recorded.len(), SAMPLES_PER_FRAME);
    assert!(recorded
        .iter()
        .all(|actual| (i32::from(*actual) - 5_678).abs() <= 128));
    application_task.await.unwrap();
    callee_task.await.unwrap();
}

#[test]
fn first_terminal_request_wins_without_reclassification() {
    let control = VoiceSessionControl::new();
    assert!(control.terminate(domain::voice::TerminationReason::AuthorityRevoked));
    assert!(!control.terminate(domain::voice::TerminationReason::Cancelled));
}
