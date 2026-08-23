//! S-049: a session signal carries authority.
//!
//! `SessionSignal` sends effect-bearing input into a live call — a keypress cannot be undone —
//! so the hosted route must refuse it unless a Grant admits the session's own operation. These
//! tests drive the seam over the hosted HTTP surface with a backend that owns one live session.

use std::sync::atomic::{AtomicUsize, Ordering};

use connector_state::{MemoryState, StateStore};
use domain::{Grant, GrantSet};
use protocol::operation::{
    ChannelSignal, SessionRequest, SessionSignalRequest, SessionState, SessionStatus,
};

use super::*;

/// The one live session this backend owns, created by an effect-bearing dial.
const LIVE_SESSION: &str = "execution:live";
const SESSION_OPERATION: &str = "test/dial";
const SESSION_CONNECTION: &str = "connection:test";

/// A backend holding one established session, counting the signals that reach it.
#[derive(Default)]
struct SessionBackend {
    signals_dispatched: AtomicUsize,
}

impl SessionBackend {
    fn status(&self) -> SessionStatus {
        SessionStatus {
            execution_ref: LIVE_SESSION.to_owned(),
            operation_ref: SESSION_OPERATION.to_owned(),
            connection_ref: SESSION_CONNECTION.to_owned(),
            state: SessionState::Established,
            termination: None,
            connector_audit_ref: "audit:test".to_owned(),
        }
    }
}

#[async_trait]
impl ConnectorBackend for SessionBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::SessionStatus(SessionRequest { execution_ref })
                if execution_ref == LIVE_SESSION =>
            {
                Ok(OperationResult::SessionStatus(self.status()))
            }
            OperationRequest::SessionStatus(_) => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "no such session",
                false,
            )),
            OperationRequest::Describe(request) => {
                Ok(OperationResult::Describe(OperationDescription {
                    operation_ref: request.operation_ref,
                    title: "test dial".to_owned(),
                    description: "test dial".to_owned(),
                    input_schema: serde_json::json!({"type": "object"}),
                    output_schema: serde_json::json!({"type": "object"}),
                    effect: EffectClass::Mutating,
                    approval: ApprovalPosture::Required,
                    connections: vec![protocol::operation::ConnectionSummary {
                        connection_ref: SESSION_CONNECTION.to_owned(),
                        label: "test".to_owned(),
                        provider: "test".to_owned(),
                        audiences: Vec::new(),
                        purpose: None,
                    }],
                    description_ref: "description:test".to_owned(),
                }))
            }
            OperationRequest::SessionSignal(SessionSignalRequest { execution_ref, .. })
                if execution_ref == LIVE_SESSION =>
            {
                self.signals_dispatched.fetch_add(1, Ordering::SeqCst);
                Ok(OperationResult::SessionSignal(self.status()))
            }
            OperationRequest::SessionSignal(_) => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "no such session",
                false,
            )),
            _ => unreachable!("signal tests send only session and describe requests"),
        }
    }
}

fn app(store: &Arc<MemoryState>, backend: &Arc<SessionBackend>) -> Router {
    let bound: Arc<dyn connector_state::StateStore> = store.clone();
    router(
        Arc::new(Verifier),
        backend.clone(),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::bound(bound, "https://identity.example.test"),
    )
}

fn seed_session_grant(store: &MemoryState) {
    GrantSet {
        revision: 1,
        grants: vec![Grant {
            grant: "grant:test".to_owned(),
            provider: "test".to_owned(),
            connection: SESSION_CONNECTION.to_owned(),
            selector: None,
            allow: BTreeSet::from([SESSION_OPERATION.to_owned()]),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }],
    }
    .write(store, "tenant-dev")
    .expect("seed grants");
}

fn signal_envelope(execution_ref: &str) -> RequestEnvelope {
    let mut request = envelope("tenant-dev");
    request.request = OperationRequest::SessionSignal(SessionSignalRequest {
        execution_ref: execution_ref.to_owned(),
        signal: ChannelSignal::Dtmf {
            digits: "1".to_owned(),
        },
    });
    request
}

async fn signal(app: Router, execution_ref: &str) -> Response {
    app.oneshot(operation_http_request(&signal_envelope(execution_ref)))
        .await
        .unwrap()
}

async fn body_bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
        .await
        .unwrap()
        .to_vec()
}

fn signal_journal(store: &MemoryState) -> String {
    let rows = store
        .read(SIGNAL_AUDIT_STATE_KEY, 8 * 1024 * 1024)
        .expect("read signal journal")
        .unwrap_or_default();
    String::from_utf8(rows).expect("signal journal is utf-8")
}

#[tokio::test]
async fn an_effect_bearing_session_signal_without_an_admitting_grant_refuses() {
    let store = Arc::new(MemoryState::new());
    let backend = Arc::new(SessionBackend::default());
    let response = signal(app(&store, &backend), LIVE_SESSION).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        backend.signals_dispatched.load(Ordering::SeqCst),
        0,
        "an ungranted keypress must never reach the backend"
    );
    let journal = signal_journal(&store);
    assert!(journal.contains("\"kind\":\"refused\""), "{journal}");
}

#[tokio::test]
async fn a_granted_session_signal_dispatches_behind_the_sessions_grant() {
    let store = Arc::new(MemoryState::new());
    let backend = Arc::new(SessionBackend::default());
    seed_session_grant(&store);
    let response = signal(app(&store, &backend), LIVE_SESSION).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(backend.signals_dispatched.load(Ordering::SeqCst), 1);
    // The admitted row is durable before dispatch, naming the session the proof covered.
    let journal = signal_journal(&store);
    assert!(journal.contains("\"kind\":\"admitted\""), "{journal}");
    assert!(
        journal.contains("\"execution_ref\":\"execution:live\""),
        "{journal}"
    );
}

#[tokio::test]
async fn an_unbound_grant_store_is_an_outage_for_session_signals() {
    let backend = Arc::new(SessionBackend::default());
    let app = router(
        Arc::new(Verifier),
        backend.clone(),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    let response = signal(app, LIVE_SESSION).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(backend.signals_dispatched.load(Ordering::SeqCst), 0);
}

/// The axis-free property across the acting family: a signal nobody grants, a session nobody
/// holds, and an invocation nobody grants all render the same refusal bytes, so a probing
/// caller learns neither which authority refused nor which execution refs exist.
#[tokio::test]
async fn a_signal_refusal_matches_the_invoke_refusal_bytes() {
    let store = Arc::new(MemoryState::new());
    let backend = Arc::new(SessionBackend::default());
    seed_session_grant(&store);
    let unknown_session = signal(app(&store, &backend), "execution:ghost").await;
    // A sprayed guess is refused with the same bytes but is not invisible: the refusal lands in
    // the journal, naming the guessed ref and nothing the deployment does not hold.
    let journal = signal_journal(&store);
    assert!(journal.contains("\"kind\":\"refused\""), "{journal}");
    assert!(
        journal.contains("\"execution_ref\":\"execution:ghost\""),
        "{journal}"
    );

    let ungranted = Arc::new(MemoryState::new());
    let refusals = [
        signal(app(&ungranted, &backend), LIVE_SESSION).await,
        app(&ungranted, &backend)
            .oneshot(operation_http_request(&invocation_envelope(
                SESSION_OPERATION,
                None,
            )))
            .await
            .unwrap(),
        unknown_session,
    ];
    let mut bodies = Vec::new();
    for refusal in refusals {
        assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
        bodies.push(body_bytes(refusal).await);
    }
    for body in &bodies[1..] {
        assert_eq!(
            body, &bodies[0],
            "every refusal in the acting family must be byte-identical"
        );
    }
    assert_eq!(backend.signals_dispatched.load(Ordering::SeqCst), 0);
}
