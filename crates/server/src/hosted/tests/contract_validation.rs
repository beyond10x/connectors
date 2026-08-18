use super::*;

struct MalformedBackend;

#[async_trait]
impl ConnectorBackend for MalformedBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        _request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        Ok(OperationResult::Search {
            operations: vec![protocol::operation::OperationSummary {
                operation_ref: "colab.rooms.create".to_owned(),
                title: "Create a conversation room".to_owned(),
                effect: EffectClass::Mutating,
                approval: ApprovalPosture::NotRequired,
                connections: Vec::new(),
            }],
        })
    }
}

#[tokio::test]
async fn hosted_route_refuses_a_malformed_backend_contract() {
    let app = router(
        Arc::new(Verifier),
        Arc::new(MalformedBackend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
    );
    let response = app
        .oneshot(operation_http_request(&envelope("tenant-dev")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
        .await
        .unwrap();
    let response: ResponseEnvelope = serde_json::from_slice(&body).unwrap();
    assert_eq!(response.status, protocol::operation::ResponseStatus::Error);
    assert_eq!(response.error.unwrap().code, OperationErrorCode::Protocol);
}
