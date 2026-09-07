//! The real hosted boundary must authorize before endpoint management or target resolution.

use super::*;
use protocol::{
    endpoint,
    operation::{self, v4},
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct EndpointBackend {
    management: AtomicUsize,
    resolutions: AtomicUsize,
    invocations: AtomicUsize,
    target_descriptions: AtomicUsize,
}

#[async_trait]
impl ConnectorBackend for EndpointBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle_endpoint(
        &self,
        _: &PrincipalContext,
        request: endpoint::EndpointRequest,
    ) -> Result<endpoint::EndpointResult, endpoint::EndpointError> {
        self.management.fetch_add(1, Ordering::SeqCst);
        match request {
            endpoint::EndpointRequest::Refresh(_) => Ok(endpoint::EndpointResult::Refresh {
                endpoints: 0,
                warnings: Vec::new(),
            }),
            endpoint::EndpointRequest::List(_) => Ok(endpoint::EndpointResult::List {
                endpoints: Vec::new(),
                next_cursor: None,
                warnings: Vec::new(),
            }),
            _ => Err(endpoint::EndpointError::new(
                endpoint::EndpointErrorCode::NotFound,
                "fixture endpoint unavailable",
                false,
            )),
        }
    }
    async fn resolve_endpoint(
        &self,
        _: &PrincipalContext,
        endpoint: &str,
        operation: &str,
    ) -> Result<String, OperationError> {
        self.resolutions.fetch_add(1, Ordering::SeqCst);
        if endpoint != "endpoint:one" || operation != "fixture.read" {
            return Err(OperationError::new(
                protocol::operation::OperationErrorCode::NotFound,
                "fixture endpoint unavailable",
                false,
            ));
        }
        Ok("connection:resolved".into())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Describe(_) => Err(OperationError::new(
                protocol::operation::OperationErrorCode::Protocol,
                "the fixture has incompatible global contributors",
                false,
            )),
            OperationRequest::Invoke(request) => {
                assert_eq!(request.connection_ref, "connection:resolved");
                self.invocations.fetch_add(1, Ordering::SeqCst);
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output: serde_json::json!({"ok":true}),
                    connector_audit_ref: "audit:fixture".into(),
                    execution_ref: None,
                }))
            }
            _ => unreachable!(),
        }
    }
    async fn describe_target(
        &self,
        _: &PrincipalContext,
        operation_ref: &str,
        connection_ref: &str,
    ) -> Result<OperationDescription, OperationError> {
        assert_eq!(connection_ref, "connection:resolved");
        self.target_descriptions.fetch_add(1, Ordering::SeqCst);
        Ok(OperationDescription {
            operation_ref: operation_ref.into(),
            title: "Fixture".into(),
            description: "Read fixture".into(),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections: vec![protocol::operation::ConnectionSummary {
                connection_ref: "connection:resolved".into(),
                label: "Fixture".into(),
                provider: "fixture".into(),
                audiences: Vec::new(),
                purpose: None,
            }],
            description_ref: "description:fixture".into(),
            rate_advice: None,
        })
    }
}

fn context() -> OwnerContext {
    OwnerContext {
        tenant_id: "tenant-dev".into(),
        agent_id: "agent".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot".into(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}
fn app(backend: Arc<EndpointBackend>, operator: bool) -> Router {
    router(
        Arc::new(Verifier),
        backend,
        HostedAdmissionPolicy::new([if operator {
            "operator"
        } else {
            "another-group"
        }
        .into()]),
        HostedAuthority::unbound(),
    )
}
fn http(path: &str, value: &impl serde::Serialize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("authorization", "Bearer access")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(value).unwrap()))
        .unwrap()
}
fn request(request: v4::OperationRequest) -> v4::RequestEnvelope {
    v4::RequestEnvelope {
        protocol: v4::CONTRACT.into(),
        request_id: "request:endpoint".into(),
        context: context(),
        request,
    }
}

#[tokio::test]
async fn endpoint_management_requires_operator_even_with_management_scope() {
    let backend = Arc::new(EndpointBackend::default());
    for request in [
        endpoint::EndpointRequest::Refresh(endpoint::RefreshRequest { source_ref: None }),
        endpoint::EndpointRequest::Bind(endpoint::BindRequest {
            endpoint_ref: "endpoint:one".into(),
            binding: endpoint::EndpointBinding {
                provider: "fixture".into(),
                base_path: None,
                credential: None,
                direct_address: Some("https://fixture.example".into()),
                database: None,
                tls: None,
                scheme: None,
            },
        }),
    ] {
        let response = app(backend.clone(), false)
            .oneshot(http(
                "/endpoints",
                &endpoint::RequestEnvelope {
                    protocol: endpoint::CONTRACT.into(),
                    request_id: "request:manage".into(),
                    context: context(),
                    request,
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(backend.management.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn endpoint_describe_and_invoke_enter_existing_connection_dispatch() {
    let backend = Arc::new(EndpointBackend::default());
    let response = app(backend.clone(), true)
        .oneshot(http(
            "/operations",
            &request(v4::OperationRequest::Describe(v4::DescribeRequest {
                operation_ref: "fixture.read".into(),
                connection_ref: None,
                endpoint_ref: Some("endpoint:one".into()),
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response: v4::ResponseEnvelope = serde_json::from_slice(
        &to_bytes(response.into_body(), operation::MAX_RESULT_BYTES)
            .await
            .unwrap(),
    )
    .unwrap();
    response.validate().unwrap();
    let Some(OperationResult::Describe(description)) = response.response else {
        panic!("expected target description")
    };
    assert_eq!(description.connections.len(), 1);
    let response = app(backend.clone(), true)
        .oneshot(http(
            "/operations",
            &request(v4::OperationRequest::Invoke(v4::InvokeRequest {
                operation_ref: "fixture.read".into(),
                connection_ref: None,
                endpoint_ref: Some("endpoint:one".into()),
                description_ref: description.description_ref,
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response: v4::ResponseEnvelope = serde_json::from_slice(
        &to_bytes(response.into_body(), operation::MAX_RESULT_BYTES)
            .await
            .unwrap(),
    )
    .unwrap();
    response.validate().unwrap();
    assert!(
        matches!(response.response, Some(OperationResult::Invoke(_))),
        "{response:?}"
    );
    assert_eq!(backend.resolutions.load(Ordering::SeqCst), 2);
    assert_eq!(backend.invocations.load(Ordering::SeqCst), 1);
    assert_eq!(backend.target_descriptions.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn denied_invocation_does_not_even_resolve_the_endpoint() {
    let backend = Arc::new(EndpointBackend::default());
    let response = app(backend.clone(), false)
        .oneshot(http(
            "/operations",
            &request(v4::OperationRequest::Invoke(v4::InvokeRequest {
                operation_ref: "fixture.read".into(),
                connection_ref: None,
                endpoint_ref: Some("endpoint:one".into()),
                description_ref: "description:fixture".into(),
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(backend.resolutions.load(Ordering::SeqCst), 0);
    assert_eq!(backend.invocations.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn endpoint_inventory_refuses_cross_tenant_before_backend_access() {
    let backend = Arc::new(EndpointBackend::default());
    let mut owner = context();
    owner.tenant_id = "other-tenant".into();
    let response = app(backend.clone(), true)
        .oneshot(http(
            "/endpoints",
            &endpoint::RequestEnvelope {
                protocol: endpoint::CONTRACT.into(),
                request_id: "request:inventory".into(),
                context: owner,
                request: endpoint::EndpointRequest::List(endpoint::ListRequest {
                    source_ref: None,
                    query: String::new(),
                    limit: 10,
                    cursor: None,
                }),
            },
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(backend.management.load(Ordering::SeqCst), 0);
}
