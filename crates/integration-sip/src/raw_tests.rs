//! Real raw-SIP launcher coverage through the backend's public lifecycle operations.

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;
use std::time::Duration;

use connectors_config::PersonalVoiceConfig;
use domain::voice::TerminationReason;
use protocol::operation::{
    DescribeRequest, InvokeRequest, OperationErrorCode, OperationRequest, OperationResult,
    RequestedSessionTermination, SessionRequest, SessionState, SessionTerminateRequest,
    SessionTermination,
};
use service::{ConnectorBackend, CredentialSet, PrincipalContext};
use tokio::net::UdpSocket;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use voice_runtime::VoiceSessionControl;

use super::SipLauncher;
use crate::SipOperationBackend;

const DEADLINE: Duration = Duration::from_secs(5);

struct Fixture {
    backend: SipOperationBackend<SipLauncher>,
    principal: PrincipalContext,
    root: tempfile::TempDir,
    peer: JoinHandle<()>,
    remote_hangup: Option<oneshot::Sender<()>>,
}

impl Fixture {
    async fn new() -> Self {
        let signaling = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = signaling.local_addr().unwrap();
        let media = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let (remote_hangup, remote) = oneshot::channel();
        let peer = tokio::spawn(run_peer(signaling, media, remote));
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = config(address);
        let principal = config.principal_context().unwrap();
        let backend = SipOperationBackend::new(
            config,
            Arc::new(SipLauncher::with_device(
                CredentialSet::default(),
                Arc::new(driver_audio::NullAudioDevice::new()),
            )),
            root.path(),
        )
        .unwrap();
        Self {
            backend,
            principal,
            root,
            peer,
            remote_hangup: Some(remote_hangup),
        }
    }

    async fn invoke(&self) -> Result<OperationResult, protocol::operation::OperationError> {
        let description = self
            .backend
            .handle(
                &self.principal,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: "sip.dial".to_owned(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::Describe(description) = description else {
            panic!("expected a SIP description");
        };
        self.backend
            .handle(
                &self.principal,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: "sip.dial".to_owned(),
                    connection_ref: "connection-loopback".to_owned(),
                    description_ref: description.description_ref,
                    input: serde_json::json!({"target": "loopback"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
    }

    async fn established(&self) -> String {
        let OperationResult::Invoke(invocation) = self.invoke().await.unwrap() else {
            panic!("expected an established SIP session");
        };
        assert_eq!(invocation.output["state"], "established");
        invocation.execution_ref.unwrap()
    }

    async fn assert_terminal(&self, execution_ref: String, expected: SessionTermination) {
        let OperationResult::SessionStatus(status) = self
            .backend
            .handle(
                &self.principal,
                OperationRequest::SessionStatus(SessionRequest { execution_ref }),
            )
            .await
            .unwrap()
        else {
            panic!("expected session status");
        };
        assert_eq!(status.state, SessionState::Terminated);
        assert_eq!(status.termination, Some(expected));
    }

    async fn peer_finished(&mut self) {
        tokio::time::timeout(DEADLINE, &mut self.peer)
            .await
            .expect("the loopback dialog must close")
            .expect("the peer must observe a complete BYE exchange");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.peer.abort();
    }
}

fn config(address: SocketAddr) -> PersonalVoiceConfig {
    toml::from_str(&format!(
        r#"
[owner]
tenant_id = "tenant-1"
agent_id = "agent-1"
agent_revision = 7
authority_snapshot_id = "snapshot-7"
authority_snapshot_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[connection]
connection_ref = "connection-loopback"
label = "Loopback fixture"
grant_ref = "grant-sip-dial-1"
initiation = "platform"
[[sip.targets]]
alias = "loopback"
permission_subject = "loopback:127.0.0.1"
signaling_bind = "127.0.0.1:0"
sent_by = "127.0.0.1"
target = "{address}"
signaling_transport = "udp"
to_uri = "sip:callee@{address}"
from_uri = "sip:caller@127.0.0.1"
media_advertised = "127.0.0.1"
media_bind = "127.0.0.1"
dial_timeout_seconds = 5
network_mode = "loopback"
signaling_apertures = [{{ address = "127.0.0.1", first_port = 1, last_port = 65535 }}]
media_apertures = [{{ address = "127.0.0.1", first_port = 1, last_port = 65535 }}]
"#
    ))
    .unwrap()
}

fn header<'a>(request: &'a str, name: &str) -> &'a str {
    request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(field, _)| field.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
        .unwrap_or_else(|| panic!("missing {name} in fixture request"))
}

fn response(request: &str, address: SocketAddr, body: &str) -> String {
    let to = header(request, "To");
    let to = if to.contains(";tag=") {
        to.to_owned()
    } else {
        format!("{to};tag=loopback-peer")
    };
    format!(
        "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {to}\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:callee@{address}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{body}",
        header(request, "Via"),
        header(request, "From"),
        header(request, "Call-ID"),
        header(request, "CSeq"),
        body.len(),
    )
}

// A bounded wire fixture: answer one G.711 INVITE, acknowledge its BYE, or originate one after
// the test requests remote hangup. Production signaling still runs through the pinned driver.
async fn run_peer(signaling: UdpSocket, media: UdpSocket, mut remote: oneshot::Receiver<()>) {
    let address = signaling.local_addr().unwrap();
    let mut bytes = [0_u8; 8192];
    let (length, caller) = signaling.recv_from(&mut bytes).await.unwrap();
    let invitation = std::str::from_utf8(&bytes[..length]).unwrap().to_owned();
    assert!(invitation.starts_with("INVITE "));
    let sdp = format!(
        "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=loopback\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio {} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\na=ptime:20\r\na=sendrecv\r\n",
        media.local_addr().unwrap().port(),
    );
    let answered = response(&invitation, address, &sdp);
    signaling
        .send_to(answered.as_bytes(), caller)
        .await
        .unwrap();
    let mut acknowledged = false;
    let mut remote_sent = false;
    loop {
        tokio::select! {
            message = signaling.recv_from(&mut bytes) => {
                let (length, source) = message.unwrap();
                assert_eq!(source, caller);
                let request = std::str::from_utf8(&bytes[..length]).unwrap();
                if request.starts_with("ACK ") {
                    acknowledged = true;
                } else if request.starts_with("INVITE ") {
                    signaling.send_to(answered.as_bytes(), caller).await.unwrap();
                } else if request.starts_with("BYE ") {
                    assert!(acknowledged, "the dial must establish before teardown");
                    signaling.send_to(response(request, address, "").as_bytes(), caller).await.unwrap();
                    return;
                } else if remote_sent && request.starts_with("SIP/2.0 200 ") {
                    assert_eq!(header(request, "CSeq"), "1 BYE");
                    return;
                } else {
                    panic!("unexpected fixture message: {}", request.lines().next().unwrap());
                }
            }
            requested = &mut remote, if acknowledged && !remote_sent => {
                requested.expect("remote hangup requested explicitly");
                let bye = format!(
                    "BYE sip:caller@{caller} SIP/2.0\r\nVia: SIP/2.0/UDP {address};branch=z9hG4bK-loopback-bye\r\nFrom: {};tag=loopback-peer\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 1 BYE\r\nMax-Forwards: 70\r\nContent-Length: 0\r\n\r\n",
                    header(&invitation, "To"),
                    header(&invitation, "From"),
                    header(&invitation, "Call-ID"),
                );
                signaling.send_to(bye.as_bytes(), caller).await.unwrap();
                remote_sent = true;
            }
        }
    }
}

#[tokio::test]
async fn backend_termination_closes_raw_sip_and_preserves_the_reason() {
    let mut fixture = Fixture::new().await;
    let execution_ref = fixture.established().await;
    fixture
        .backend
        .handle(
            &fixture.principal,
            OperationRequest::SessionTerminate(SessionTerminateRequest {
                execution_ref: execution_ref.clone(),
                reason: RequestedSessionTermination::Cancelled,
            }),
        )
        .await
        .unwrap();
    fixture.peer_finished().await;
    tokio::time::timeout(DEADLINE, fixture.backend.shutdown())
        .await
        .unwrap();
    fixture
        .assert_terminal(execution_ref, SessionTermination::Cancelled)
        .await;
}

#[tokio::test]
async fn backend_shutdown_waits_for_raw_sip_teardown() {
    let mut fixture = Fixture::new().await;
    let execution_ref = fixture.established().await;
    tokio::time::timeout(DEADLINE, fixture.backend.shutdown())
        .await
        .expect("shutdown must terminate the raw call");
    fixture.peer_finished().await;
    fixture
        .assert_terminal(execution_ref, SessionTermination::Revoked)
        .await;
}

#[tokio::test]
async fn failed_establishment_audit_terminates_the_unregistered_raw_call() {
    let mut fixture = Fixture::new().await;
    std::fs::create_dir(fixture.root.path().join("connector-audit.jsonl")).unwrap();
    let error = fixture
        .invoke()
        .await
        .expect_err("the audit path is unwritable");
    assert_eq!(error.code, OperationErrorCode::Unavailable);
    fixture.peer_finished().await;
}

#[tokio::test]
async fn remote_hangup_is_preserved_through_backend_shutdown() {
    let mut fixture = Fixture::new().await;
    let execution_ref = fixture.established().await;
    fixture.remote_hangup.take().unwrap().send(()).unwrap();
    fixture.peer_finished().await;
    tokio::time::timeout(DEADLINE, fixture.backend.shutdown())
        .await
        .unwrap();
    fixture
        .assert_terminal(execution_ref, SessionTermination::RemoteEnded)
        .await;
}

#[tokio::test]
async fn termination_control_retains_the_first_request_for_late_waiters() {
    let control = VoiceSessionControl::new();
    assert!(control.terminate(TerminationReason::Cancelled));
    assert!(!control.terminate(TerminationReason::AuthorityRevoked));
    assert_eq!(
        tokio::time::timeout(DEADLINE, control.wait_termination())
            .await
            .unwrap(),
        TerminationReason::Cancelled,
    );
}
