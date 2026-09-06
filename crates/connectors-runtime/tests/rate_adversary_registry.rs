use async_trait::async_trait;
use connectors_runtime::BackendRegistry;
use protocol::operation::*;
use serde_json::json;
use service::{ConnectorBackend, PrincipalContext};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

struct Owner {
    connection: &'static str,
    rate: Mutex<u32>,
    invokes: AtomicUsize,
}
impl Owner {
    fn description(&self) -> OperationDescription {
        OperationDescription {
            operation_ref: "fixture.read".into(),
            title: "Read".into(),
            description: "Read".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections: vec![ConnectionSummary {
                connection_ref: self.connection.into(),
                label: self.connection.into(),
                provider: "fixture".into(),
                audiences: vec![],
                purpose: None,
            }],
            description_ref: "unchanged-backend-lease".into(),
            rate_advice: Some(OperationRateAdvice {
                fixed: Some(FixedRateLimit {
                    requests: *self.rate.lock().unwrap(),
                    per_seconds: 60,
                    bucket: None,
                }),
                alternatives: vec![],
            }),
        }
    }
}
#[async_trait]
impl ConnectorBackend for Owner {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(value) => value.operation_ref == "fixture.read",
            OperationRequest::Invoke(value) => {
                value.operation_ref == "fixture.read" && value.connection_ref == self.connection
            }
            _ => false,
        }
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Describe(_) => Ok(OperationResult::Describe(self.description())),
            OperationRequest::Invoke(request) => {
                self.invokes.fetch_add(1, Ordering::SeqCst);
                assert_eq!(request.description_ref, "unchanged-backend-lease");
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output: json!({}),
                    connector_audit_ref: "audit:fixture".into(),
                    execution_ref: None,
                }))
            }
            _ => unreachable!(),
        }
    }
}
#[tokio::test]
async fn rate_adversary_registry_checks_advice_through_describe_and_invoke() {
    let context = PrincipalContext::local(&OwnerContext {
        tenant_id: "local".into(),
        agent_id: "agent".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot".into(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap();
    let a = Arc::new(Owner {
        connection: "connection:a",
        rate: Mutex::new(50),
        invokes: AtomicUsize::new(0),
    });
    let b = Arc::new(Owner {
        connection: "connection:b",
        rate: Mutex::new(50),
        invokes: AtomicUsize::new(0),
    });
    let registry = BackendRegistry::new(vec![a.clone(), b.clone()]);
    let reversed = BackendRegistry::new(vec![b.clone(), a.clone()]);
    let describe = OperationRequest::Describe(DescribeRequest {
        operation_ref: "fixture.read".into(),
    });
    let OperationResult::Describe(before) =
        registry.handle(&context, describe.clone()).await.unwrap()
    else {
        panic!()
    };
    let OperationResult::Describe(other) =
        reversed.handle(&context, describe.clone()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(before, other);
    let invoke = |lease: String| {
        OperationRequest::Invoke(InvokeRequest {
            operation_ref: "fixture.read".into(),
            connection_ref: "connection:a".into(),
            description_ref: lease,
            input: json!({}),
            approval_evidence_ref: None,
        })
    };
    *b.rate.lock().unwrap() = 1;
    assert_eq!(
        registry
            .handle(&context, describe.clone())
            .await
            .unwrap_err()
            .code,
        OperationErrorCode::Protocol
    );
    assert_eq!(
        registry
            .handle(&context, invoke(before.description_ref.clone()))
            .await
            .unwrap_err()
            .code,
        OperationErrorCode::Protocol
    );
    *a.rate.lock().unwrap() = 1;
    assert_eq!(
        registry
            .handle(&context, invoke(before.description_ref))
            .await
            .unwrap_err()
            .code,
        OperationErrorCode::StaleAuthority
    );
    assert_eq!(a.invokes.load(Ordering::SeqCst), 0);
    let OperationResult::Describe(fresh) = registry.handle(&context, describe).await.unwrap()
    else {
        panic!()
    };
    registry
        .handle(&context, invoke(fresh.description_ref))
        .await
        .unwrap();
    assert_eq!(a.invokes.load(Ordering::SeqCst), 1);
    assert_eq!(b.invokes.load(Ordering::SeqCst), 0);
}
