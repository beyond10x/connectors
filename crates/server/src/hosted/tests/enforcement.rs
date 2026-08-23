//! S-046: the hosted route enforces Grant and approval authority instead of refusing.
//!
//! These tests drive the full chain over the hosted HTTP surface: identity verification,
//! receiver policy, re-description, Grant evaluation, approval verification with one-time
//! redemption, dispatch, and the terminal journal row.

use connector_state::{MemoryState, StateStore};
use domain::{ApprovalRecord, Grant, GrantSet, APPROVAL_AUDIT_STATE_KEY};

use super::*;

const ISSUER: &str = "https://identity.example.test";

/// A backend with one read-only operation and two effect-bearing ones, described with the
/// Connection each operation is invocable over.
struct EffectBackend;

fn described(operation_ref: String) -> OperationDescription {
    let (effect, approval) = match operation_ref.as_str() {
        "test/read" => (EffectClass::ReadOnly, ApprovalPosture::NotRequired),
        "test/effect" => (EffectClass::Mutating, ApprovalPosture::NotRequired),
        _ => (EffectClass::Mutating, ApprovalPosture::Required),
    };
    OperationDescription {
        operation_ref,
        title: "test".to_owned(),
        description: "test".to_owned(),
        input_schema: serde_json::json!({"type": "object"}),
        output_schema: serde_json::json!({"type": "object"}),
        effect,
        approval,
        connections: vec![protocol::operation::ConnectionSummary {
            connection_ref: "connection:test".to_owned(),
            label: "test".to_owned(),
            provider: "test".to_owned(),
            audiences: Vec::new(),
            purpose: None,
        }],
        description_ref: "description:test".to_owned(),
    }
}

#[async_trait]
impl ConnectorBackend for EffectBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Describe(request) => {
                Ok(OperationResult::Describe(described(request.operation_ref)))
            }
            OperationRequest::Invoke(request) => Ok(OperationResult::Invoke(InvocationResult {
                operation_ref: request.operation_ref,
                output: serde_json::json!({"ok": true}),
                connector_audit_ref: "audit:test".to_owned(),
                execution_ref: None,
            })),
            _ => unreachable!("enforcement tests send only describe and invoke"),
        }
    }
}

fn store() -> Arc<MemoryState> {
    Arc::new(MemoryState::new())
}

fn app(store: &Arc<MemoryState>) -> Router {
    let bound: Arc<dyn StateStore> = store.clone();
    router(
        Arc::new(Verifier),
        Arc::new(EffectBackend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::bound(bound, ISSUER),
    )
}

fn seed_grants(store: &MemoryState) {
    GrantSet {
        revision: 1,
        grants: vec![Grant {
            grant: "grant:test".to_owned(),
            provider: "test".to_owned(),
            connection: "connection:test".to_owned(),
            selector: None,
            allow: BTreeSet::from([
                "test/read".to_owned(),
                "test/effect".to_owned(),
                "test/restart".to_owned(),
            ]),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }],
    }
    .write(store, "tenant-dev")
    .expect("seed grants");
}

fn seed_approval(store: &MemoryState, reference: &str, input: &serde_json::Value) {
    issue_approval(
        store,
        &ApprovalRecord {
            reference: reference.to_owned(),
            issuer: ISSUER.to_owned(),
            subject: "person:test".to_owned(),
            operation: "test/restart".to_owned(),
            connection: "connection:test".to_owned(),
            input_digest: canonical_input_digest(input),
            expires_at_seconds: u64::MAX,
        },
    )
    .expect("seed approval record");
}

async fn invoke(app: Router, operation_ref: &str, approval: Option<&str>) -> Response {
    app.oneshot(operation_http_request(&invocation_envelope(
        operation_ref,
        approval,
    )))
    .await
    .unwrap()
}

async fn body_bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
        .await
        .unwrap()
        .to_vec()
}

fn journal(store: &MemoryState) -> String {
    let rows = store
        .read(APPROVAL_AUDIT_STATE_KEY, 8 * 1024 * 1024)
        .expect("read journal")
        .unwrap_or_default();
    String::from_utf8(rows).expect("journal is utf-8")
}

#[tokio::test]
async fn a_mutation_with_no_admitting_grant_refuses() {
    let store = store();
    assert_eq!(
        invoke(app(&store), "test/effect", None).await.status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn an_unbound_grant_store_is_an_outage_for_effects_only() {
    let app = router(
        Arc::new(Verifier),
        Arc::new(EffectBackend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    let outage = app
        .clone()
        .oneshot(operation_http_request(&invocation_envelope(
            "test/effect",
            None,
        )))
        .await
        .unwrap();
    assert_eq!(outage.status(), StatusCode::SERVICE_UNAVAILABLE);
    let read = app
        .oneshot(operation_http_request(&invocation_envelope(
            "test/read",
            None,
        )))
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);
}

#[tokio::test]
async fn the_read_only_path_is_unchanged_for_callers_without_grants() {
    let store = store();
    assert_eq!(
        invoke(app(&store), "test/read", None).await.status(),
        StatusCode::OK
    );
    // And an admitting Grant serves the same read through the decision proof.
    seed_grants(&store);
    assert_eq!(
        invoke(app(&store), "test/read", None).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone() {
    let store = store();
    seed_grants(&store);
    assert_eq!(
        invoke(app(&store), "test/effect", None).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_granted_mutation_demanding_approval_refuses_without_one() {
    let store = store();
    seed_grants(&store);
    assert_eq!(
        invoke(app(&store), "test/restart", None).await.status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn a_granted_mutation_with_a_demanded_approval_dispatches_with_one() {
    let store = store();
    seed_grants(&store);
    seed_approval(&store, "approval:one", &serde_json::json!({}));
    let response = invoke(app(&store), "test/restart", Some("approval:one")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let journal = journal(&store);
    assert!(journal.contains("\"kind\":\"attempted\""), "{journal}");
    assert!(journal.contains("\"kind\":\"completed\""), "{journal}");
}

#[tokio::test]
async fn a_second_presentation_of_the_same_approval_refuses_and_journals_replay() {
    let store = store();
    seed_grants(&store);
    seed_approval(&store, "approval:once", &serde_json::json!({}));
    assert_eq!(
        invoke(app(&store), "test/restart", Some("approval:once"))
            .await
            .status(),
        StatusCode::OK
    );
    let replay = invoke(app(&store), "test/restart", Some("approval:once")).await;
    assert_eq!(replay.status(), StatusCode::FORBIDDEN);
    let journal = journal(&store);
    assert!(journal.contains("\"kind\":\"replayed\""), "{journal}");
}

/// The axis-free property at the route level: every enforcement refusal — no admitting grant,
/// demanded approval missing, unissued reference, mismatched record, replay, and evidence
/// nothing demands — renders as the same bytes, so a probing caller learns nothing about which
/// authority refused or whether a reference was already spent.
#[tokio::test]
async fn every_enforcement_refusal_renders_the_same_bytes() {
    let ungranted = store();
    let no_grant = invoke(app(&ungranted), "test/effect", None).await;

    let store = store();
    seed_grants(&store);
    seed_approval(
        &store,
        "approval:mismatched",
        &serde_json::json!({"other": 1}),
    );
    seed_approval(&store, "approval:spent", &serde_json::json!({}));
    assert_eq!(
        invoke(app(&store), "test/restart", Some("approval:spent"))
            .await
            .status(),
        StatusCode::OK
    );
    let refusals = [
        no_grant,
        invoke(app(&store), "test/restart", None).await,
        invoke(app(&store), "test/restart", Some("approval:unissued")).await,
        invoke(app(&store), "test/restart", Some("approval:mismatched")).await,
        invoke(app(&store), "test/restart", Some("approval:spent")).await,
        invoke(app(&store), "test/effect", Some("approval:undemanded")).await,
        invoke(app(&store), "test/read", Some("approval:undemanded")).await,
    ];
    let mut bodies = Vec::new();
    for refusal in refusals {
        assert_eq!(refusal.status(), StatusCode::FORBIDDEN);
        bodies.push(body_bytes(refusal).await);
    }
    for body in &bodies[1..] {
        assert_eq!(
            body, &bodies[0],
            "every enforcement refusal must be byte-identical"
        );
    }
}
