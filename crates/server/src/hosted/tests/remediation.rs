//! Proposed hosted auth deciding cases. Fixture backend only; no production OAuth adapter.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use connector_state::{MemoryState, StateStore};
use domain::{ApprovalRecord, Grant, GrantSet, APPROVAL_AUDIT_STATE_KEY};
use protocol::operation::{v3, versions};
use service::{
    CredentialReadiness, RemediationAuthority, RemediationError, RemediationMetadata,
    RemediationRequest, RemediationResult, RemediationRoute, RemediationTarget,
};

use super::*;

const ISSUER: &str = "https://identity.example.test";
const READ: &str = "slack-conversations-history";
const WRITE: &str = "slack-chat-post-message";
const CONNECTION: &str = "connection:test";
const INTEGRATION: &str = "slack";
const PURPOSE: &str = "slack.bot_token";
const PRIVATE: &str = "https://private.example.test/SYNTHETIC_PRIVATE_INSTRUCTION";

struct RemediationBackend {
    need: CredentialReadiness,
    ready: AtomicBool,
    poison_metadata: bool,
    readiness_calls: AtomicUsize,
    dispatches: AtomicUsize,
    bound_calls: AtomicUsize,
}

impl RemediationBackend {
    fn new(need: CredentialReadiness, poison_metadata: bool) -> Self {
        Self {
            need,
            ready: AtomicBool::new(false),
            poison_metadata,
            readiness_calls: AtomicUsize::new(0),
            dispatches: AtomicUsize::new(0),
            bound_calls: AtomicUsize::new(0),
        }
    }

    fn description(operation_ref: &str) -> Result<OperationDescription, OperationError> {
        if !matches!(operation_ref, READ | WRITE) {
            return Err(OperationError::new(
                OperationErrorCode::NotFound,
                "unknown fixture operation",
                false,
            ));
        }
        let operation = catalog::operation(catalog::OperationKey::id(operation_ref)).unwrap();
        Ok(OperationDescription {
            rate_advice: None,
            operation_ref: operation_ref.into(),
            title: "Hosted remediation fixture".into(),
            description: operation.description.into(),
            input_schema: serde_json::from_str(operation.input_schema).unwrap(),
            output_schema: serde_json::json!({"type":"object"}),
            effect: if operation_ref == READ {
                EffectClass::ReadOnly
            } else {
                EffectClass::Mutating
            },
            approval: if operation_ref == READ {
                ApprovalPosture::NotRequired
            } else {
                ApprovalPosture::Required
            },
            connections: vec![protocol::operation::ConnectionSummary {
                connection_ref: CONNECTION.into(),
                label: "Fixture Connection".into(),
                provider: operation.provider.into(),
                audiences: Vec::new(),
                purpose: Some(PURPOSE.into()),
            }],
            description_ref: "description:test".into(),
        })
    }
}

#[async_trait]
impl ConnectorBackend for RemediationBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => {
                matches!(request.operation_ref.as_str(), READ | WRITE)
            }
            OperationRequest::Invoke(request) => {
                matches!(request.operation_ref.as_str(), READ | WRITE)
            }
            _ => false,
        }
    }

    fn owns_remediation(&self, route: RemediationRoute<'_>) -> bool {
        matches!(route, RemediationRoute::Target(target)
            if matches!(target.operation_ref, READ | WRITE) && target.connection_ref == CONNECTION)
    }

    fn remediation_metadata<'a>(
        &'a self,
        context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> Result<RemediationMetadata<'a>, RemediationError> {
        assert_eq!(context.subject(), "person:test");
        assert_eq!(context.agent_revision(), None);
        if !self.owns_remediation(RemediationRoute::Target(target)) {
            return Err(RemediationError::Refused);
        }
        Ok(RemediationMetadata {
            operation: catalog::reader::operation(target.operation_ref).unwrap(),
            connection: domain::ConnectionAuthority::new(
                CONNECTION,
                domain::InitiationPolicy::platform_only(),
            )
            .unwrap(),
            integration_ref: if self.poison_metadata {
                PRIVATE
            } else {
                INTEGRATION
            }
            .into(),
            auth_profile: PURPOSE.into(),
            catalog_generation: catalog::reader::embedded().digest().into(),
        })
    }

    async fn credential_readiness(
        &self,
        _context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> CredentialReadiness {
        assert!(self.owns_remediation(RemediationRoute::Target(target)));
        self.readiness_calls.fetch_add(1, Ordering::SeqCst);
        if self.ready.load(Ordering::SeqCst) {
            CredentialReadiness::Ready
        } else {
            self.need
        }
    }

    async fn handle_remediation(
        &self,
        _context: &PrincipalContext,
        _request: RemediationRequest,
        _authority: Arc<dyn RemediationAuthority>,
    ) -> Result<RemediationResult, RemediationError> {
        self.bound_calls.fetch_add(1, Ordering::SeqCst);
        panic!("an operation preflight must not start, poll or acknowledge a session")
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        assert_eq!(context.subject(), "person:test");
        match request {
            OperationRequest::Describe(request) => {
                Self::description(&request.operation_ref).map(OperationResult::Describe)
            }
            OperationRequest::Invoke(request) => {
                self.dispatches.fetch_add(1, Ordering::SeqCst);
                assert!(
                    self.ready.load(Ordering::SeqCst),
                    "credential need crossed dispatch"
                );
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output: serde_json::json!({"fixture_only":true}),
                    connector_audit_ref: "audit:fixture".into(),
                    execution_ref: None,
                }))
            }
            _ => panic!("these fixtures send only describe and invoke"),
        }
    }
}

fn application(backend: Arc<RemediationBackend>, store: Option<&Arc<MemoryState>>) -> Router {
    let authority = match store {
        Some(store) => HostedAuthority::bound(store.clone(), ISSUER),
        None => HostedAuthority::unbound(),
    };
    router(
        Arc::new(Verifier),
        backend,
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        authority,
    )
}

fn grant(store: &MemoryState) {
    GrantSet {
        revision: 1,
        grants: vec![Grant {
            grant: "grant:fixture-slack".into(),
            provider: "slack".into(),
            connection: CONNECTION.into(),
            selector: None,
            allow: BTreeSet::from([READ.into(), WRITE.into()]),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }],
    }
    .write(store, "tenant-dev")
    .unwrap();
}

fn request(operation: &str, approval: Option<&str>) -> RequestEnvelope {
    // The existing helper serializes the common request vocabulary without running a v2 reader.
    let mut frame = invocation_envelope(operation, approval);
    frame.protocol = v3::CONTRACT.into();
    if let OperationRequest::Invoke(invoke) = &mut frame.request {
        invoke.input = if operation == WRITE {
            serde_json::json!({"channel":"fixture-channel","text":"fixture message"})
        } else {
            serde_json::json!({"channel":"fixture-channel"})
        };
    }
    let bytes = serde_json::to_vec(&frame).unwrap();
    assert_eq!(
        versions::decode_request(&bytes).unwrap().0,
        versions::Version::V0Alpha3
    );
    frame
}

async fn bytes(response: Response) -> Vec<u8> {
    to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
        .await
        .unwrap()
        .to_vec()
}

fn decoded(bytes: &[u8]) -> v3::ResponseEnvelope {
    let (version, reply) = versions::decode_response(bytes).unwrap();
    assert_eq!(version, versions::Version::V0Alpha3);
    assert_eq!(reply.request_id, "request-1");
    reply
}

fn audit(store: &MemoryState) -> Option<Vec<u8>> {
    store
        .read(APPROVAL_AUDIT_STATE_KEY, 8 * 1024 * 1024)
        .unwrap()
}

#[tokio::test]
async fn auth_v3_need_precedes_real_approval_redemption_and_dispatch() {
    for (need, expected) in [
        (
            CredentialReadiness::MissingCredential,
            v3::AuthenticationNeed::AuthorizeConfigured,
        ),
        (
            CredentialReadiness::CredentialDegraded,
            v3::AuthenticationNeed::ReauthorizeExisting,
        ),
    ] {
        let store = Arc::new(MemoryState::new());
        grant(&store);
        let frame = request(WRITE, Some("approval:fixture-once"));
        let OperationRequest::Invoke(invoke) = &frame.request else {
            unreachable!()
        };
        issue_approval(
            &*store,
            &ApprovalRecord {
                reference: "approval:fixture-once".into(),
                issuer: ISSUER.into(),
                subject: "person:test".into(),
                operation: WRITE.into(),
                connection: CONNECTION.into(),
                input_digest: canonical_input_digest(&invoke.input),
                expires_at_seconds: u64::MAX,
            },
        )
        .unwrap();
        let before = audit(&store);
        let backend = Arc::new(RemediationBackend::new(need, false));
        let app = application(backend.clone(), Some(&store));
        let response = app
            .clone()
            .oneshot(operation_http_request(&frame))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let reply = decoded(&bytes(response).await);
        assert!(reply.response.is_none());
        let error = reply.error.unwrap();
        assert_eq!(error.code, v3::OperationErrorCode::AuthenticationRequired);
        assert!(!error.retriable);
        assert_eq!(error.retry_after_seconds, None);
        let authentication = error.authentication.unwrap();
        assert_eq!(authentication.need, expected);
        assert_eq!(
            authentication.attempt,
            v3::AuthenticationAttemptState::NotAttempted
        );
        assert_eq!(
            authentication.next_action,
            v3::AuthenticationNextAction::StartTrustedRemediation
        );
        assert_eq!(authentication.operation_ref, WRITE);
        assert_eq!(authentication.connection_ref, CONNECTION);
        assert_eq!(authentication.integration_ref, INTEGRATION);
        assert_eq!(authentication.auth_profile, PURPOSE);
        assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
        assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            audit(&store),
            before,
            "auth need must not reach ApprovalGate redemption"
        );

        // A separate explicit request is a positive control that the real approval is unspent.
        backend.ready.store(true, Ordering::SeqCst);
        let success = app
            .clone()
            .oneshot(operation_http_request(&frame))
            .await
            .unwrap();
        assert_eq!(success.status(), StatusCode::OK);
        assert!(matches!(
            decoded(&bytes(success).await).response,
            Some(OperationResult::Invoke(_))
        ));
        assert_eq!(backend.dispatches.load(Ordering::SeqCst), 1);
        let replay = app.oneshot(operation_http_request(&frame)).await.unwrap();
        assert_eq!(replay.status(), StatusCode::FORBIDDEN);
        assert_eq!(backend.dispatches.load(Ordering::SeqCst), 1);
        assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
        let journal = String::from_utf8(audit(&store).unwrap()).unwrap();
        assert!(journal.contains("\"kind\":\"attempted\""));
        assert!(journal.contains("\"kind\":\"completed\""));
        assert!(journal.contains("\"kind\":\"replayed\""));
    }
}

#[tokio::test]
async fn auth_v3_requires_a_real_grant_and_keeps_unknown_targets_opaque() {
    let backend = Arc::new(RemediationBackend::new(
        CredentialReadiness::MissingCredential,
        false,
    ));
    let outage = application(backend.clone(), None)
        .oneshot(operation_http_request(&request(READ, None)))
        .await
        .unwrap();
    assert_eq!(outage.status(), StatusCode::SERVICE_UNAVAILABLE);
    let error = decoded(&bytes(outage).await).error.unwrap();
    assert_eq!(error.code, v3::OperationErrorCode::Unavailable);
    assert!(error.authentication.is_none());
    assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 0);

    let store = Arc::new(MemoryState::new()); // Bound, healthy, with no admitting grants.
    let app = application(backend.clone(), Some(&store));
    let unadmitted = request(READ, None);
    let unknown_operation = request("auth-unconfigured-operation", None);
    let mut unknown_connection = request(READ, None);
    if let OperationRequest::Invoke(invoke) = &mut unknown_connection.request {
        invoke.connection_ref = "connection:unknown".into();
    }
    let mut bodies = Vec::new();
    for frame in [unadmitted, unknown_operation, unknown_connection] {
        let response = app
            .clone()
            .oneshot(operation_http_request(&frame))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = bytes(response).await;
        let error = decoded(&body).error.unwrap();
        assert_eq!(error.code, v3::OperationErrorCode::NotGranted);
        assert!(error.authentication.is_none());
        bodies.push(body);
    }
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(bodies[0], bodies[2]);
    assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
    assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
    assert_eq!(audit(&store), None);
}

#[tokio::test]
async fn auth_v3_does_not_publish_a_structurally_valid_private_backend_reference() {
    let store = Arc::new(MemoryState::new());
    grant(&store);
    // Source-level validity is checked with the actual committed DTO, not a string heuristic.
    v3::AuthenticationRequired {
        operation_ref: READ.into(),
        connection_ref: CONNECTION.into(),
        integration_ref: PRIVATE.into(),
        auth_profile: PURPOSE.into(),
        need: v3::AuthenticationNeed::AuthorizeConfigured,
        attempt: v3::AuthenticationAttemptState::NotAttempted,
        next_action: v3::AuthenticationNextAction::StartTrustedRemediation,
    }
    .validate()
    .unwrap();
    let backend = Arc::new(RemediationBackend::new(
        CredentialReadiness::MissingCredential,
        true,
    ));
    let response = application(backend.clone(), Some(&store))
        .oneshot(operation_http_request(&request(READ, None)))
        .await
        .unwrap();
    assert!(matches!(
        response.status(),
        StatusCode::FORBIDDEN | StatusCode::SERVICE_UNAVAILABLE
    ));
    let body = bytes(response).await;
    assert!(!String::from_utf8_lossy(&body).contains(PRIVATE));
    let error = decoded(&body).error.unwrap();
    assert!(error.authentication.is_none());
    assert!(!error.retriable);
    assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
    assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
    assert_eq!(audit(&store), None);
}

fn connection_http(request: &protocol::connection_v2::RequestEnvelope) -> Request<Body> {
    Request::post("/connections")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer access")
        .body(Body::from(serde_json::to_vec(request).unwrap()))
        .unwrap()
}

#[tokio::test]
async fn auth_connection_v2_ordinary_requests_select_exact_identity_and_refuse_duplicates() {
    use protocol::connection_v2 as v2;
    let app = router(
        Arc::new(Verifier),
        Arc::new(Backend),
        HostedAdmissionPolicy::new(["operator".into()]),
        HostedAuthority::unbound(),
    );
    let request = v2::RequestEnvelope {
        protocol: v2::CONTRACT.into(),
        request_id: "connection:versions".into(),
        context: envelope("tenant-dev").context,
        request: v2::ConnectionRequest::Search(protocol::connection::SearchRequest {
            query: String::new(),
            limit: 10,
        }),
    };
    for version in [v2::Version::V0Alpha1, v2::Version::V0Alpha2] {
        let body = version.encode_request(request.clone()).unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::post("/connections")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let (selected, response) = v2::decode_response(&bytes(response).await).unwrap();
        assert_eq!(selected, version);
        assert_eq!(response.request_id, request.request_id);
        assert!(matches!(
            response.response,
            Some(v2::ConnectionResult::Search { .. })
        ));
    }
    let body = serde_json::to_string(&request).unwrap().replacen(
        '{',
        &format!("{{\"protocol\":\"{}\",", v2::CONTRACT),
        1,
    );
    let response = app
        .oneshot(
            Request::post("/connections")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn auth_connection_v2_hosted_start_has_real_grants_and_no_production_acquisition() {
    use protocol::connection_v2 as v2;
    let backend = Arc::new(RemediationBackend::new(
        CredentialReadiness::MissingCredential,
        false,
    ));
    let request = v2::RequestEnvelope {
        protocol: v2::CONTRACT.into(),
        request_id: "connection:bound".into(),
        context: envelope("tenant-dev").context,
        request: v2::ConnectionRequest::RemediationStart(v2::RemediationStartRequest {
            operation_ref: READ.into(),
            connection_ref: CONNECTION.into(),
            input: serde_json::json!({"channel":"C123"}),
        }),
    };
    let store = Arc::new(MemoryState::new());
    let app = application(backend.clone(), Some(&store));
    let mut unknown = request.clone();
    if let v2::ConnectionRequest::RemediationStart(value) = &mut unknown.request {
        value.connection_ref = "connection:unknown".into();
    }
    let mut refusals = Vec::new();
    for frame in [&request, &unknown] {
        let response = app.clone().oneshot(connection_http(frame)).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        refusals.push(bytes(response).await);
    }
    assert_eq!(refusals[0], refusals[1]);
    grant(&store);
    let response = app.oneshot(connection_http(&request)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let (_, response) = v2::decode_response(&bytes(response).await).unwrap();
    assert_eq!(
        response.error.unwrap().code,
        protocol::connection::ConnectionErrorCode::Unavailable
    );
    assert!(response.response.is_none());
    assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
    assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 0);
    assert!(audit(&store).is_none());
}

#[tokio::test]
async fn auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation() {
    let document: serde_json::Value =
        serde_json::from_str(crate::hosted::docs::document_json()).unwrap();
    let mut observations = Vec::new();
    for version in [
        versions::Version::V0Alpha1,
        versions::Version::V0Alpha2,
        versions::Version::V0Alpha3,
    ] {
        let store = Arc::new(MemoryState::new());
        grant(&store);
        let backend = Arc::new(RemediationBackend::new(
            CredentialReadiness::MissingCredential,
            false,
        ));
        let app = application(backend.clone(), Some(&store));
        let mut frame = request(READ, None);
        frame.protocol = protocol::operation::CONTRACT.into();
        let encoded = version.encode_request(frame).unwrap();
        let send = || {
            Request::post("/operations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::from(encoded.clone()))
                .unwrap()
        };
        let response = app.clone().oneshot(send()).await.unwrap();
        let status = response.status();
        let raw = bytes(response).await;
        let (selected, reply) = versions::decode_response(&raw).unwrap();
        assert_eq!(selected, version);
        assert_eq!(reply.request_id, "request-1");
        assert_eq!(
            reply.error.as_ref().unwrap().authentication.is_some(),
            version == versions::Version::V0Alpha3
        );
        assert!(!reply.error.as_ref().unwrap().retriable);
        let mut schema = document["paths"]["/operations"]["post"]["responses"][status.as_str()]
            ["content"]["application/json"]["schema"]
            .clone();
        schema["components"] = document["components"].clone();
        schema["$schema"] = serde_json::json!("https://json-schema.org/draft/2020-12/schema");
        let validator = jsonschema::validator_for(&schema).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        observations.push((
            format!("{version:?}"),
            status.as_u16(),
            validator.is_valid(&value),
        ));
        assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 1);
        GrantSet {
            revision: 2,
            grants: Vec::new(),
        }
        .write(&*store, "tenant-dev")
        .unwrap();
        let revoked = app.oneshot(send()).await.unwrap();
        assert_eq!(revoked.status(), StatusCode::FORBIDDEN);
        let (selected, refusal) = versions::decode_response(&bytes(revoked).await).unwrap();
        assert_eq!(selected, version);
        assert!(refusal.error.unwrap().authentication.is_none());
        assert_eq!(
            backend.readiness_calls.load(Ordering::SeqCst),
            1,
            "revocation precedes readiness"
        );
        assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
        assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
        assert!(audit(&store).is_none());
    }
    eprintln!("actual hosted selected response status/schema observations: {observations:?}");
    assert!(
        observations.iter().all(|(_, _, valid)| *valid),
        "served OpenAPI must admit the actual selected response at its actual HTTP status"
    );
}

#[tokio::test]
async fn auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts() {
    let document: serde_json::Value =
        serde_json::from_str(crate::hosted::docs::document_json()).unwrap();
    let mut observations = Vec::new();
    for version in [
        versions::Version::V0Alpha1,
        versions::Version::V0Alpha2,
        versions::Version::V0Alpha3,
    ] {
        for scenario in ["missing", "degraded", "outage", "stale"] {
            let store = Arc::new(MemoryState::new());
            grant(&store);
            let need = match scenario {
                "degraded" => CredentialReadiness::CredentialDegraded,
                "outage" => CredentialReadiness::DependencyUnavailable,
                _ => CredentialReadiness::MissingCredential,
            };
            let backend = Arc::new(RemediationBackend::new(need, false));
            let app = application(backend.clone(), Some(&store));
            let mut frame = request(READ, None);
            if scenario == "stale" {
                let OperationRequest::Invoke(invoke) = &mut frame.request else {
                    unreachable!()
                };
                invoke.description_ref = "description:retired".into();
            }
            frame.protocol = protocol::operation::CONTRACT.into();
            let encoded = version.encode_request(frame).unwrap();
            let response = app
                .oneshot(
                    Request::post("/operations")
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::AUTHORIZATION, "Bearer access")
                        .body(Body::from(encoded))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            let raw = bytes(response).await;
            let (selected, reply) = versions::decode_response(&raw).unwrap();
            assert_eq!(selected, version);
            let error = reply.error.unwrap();
            let auth = matches!(scenario, "missing" | "degraded")
                && version == versions::Version::V0Alpha3;
            assert_eq!(error.authentication.is_some(), auth);
            assert!(!error.retriable);
            assert_eq!(
                status,
                if scenario == "stale" || auth {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                }
            );
            assert_eq!(
                error.code,
                if scenario == "stale" {
                    v3::OperationErrorCode::StaleAuthority
                } else if auth {
                    v3::OperationErrorCode::AuthenticationRequired
                } else {
                    v3::OperationErrorCode::Unavailable
                }
            );
            let mut schema = document["paths"]["/operations"]["post"]["responses"][status.as_str()]
                ["content"]["application/json"]["schema"]
                .clone();
            schema["components"] = document["components"].clone();
            schema["$schema"] = serde_json::json!("https://json-schema.org/draft/2020-12/schema");
            let valid = jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&serde_json::from_slice::<serde_json::Value>(&raw).unwrap());
            observations.push((version, scenario, status.as_u16(), valid));
            assert_eq!(
                backend.readiness_calls.load(Ordering::SeqCst),
                usize::from(scenario != "stale")
            );
            assert_eq!(backend.dispatches.load(Ordering::SeqCst), 0);
            assert_eq!(backend.bound_calls.load(Ordering::SeqCst), 0);
            assert!(audit(&store).is_none());
        }
    }
    eprintln!("actual selected status/schema matrix, no dispatch: {observations:?}");
    assert!(observations.iter().all(|(_, _, _, valid)| *valid), "every selected response must satisfy the schema served for its actual HTTP status, including non-authentication conflicts");
}
