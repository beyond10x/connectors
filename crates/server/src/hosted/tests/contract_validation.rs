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
        HostedAuthority::unbound(),
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

#[tokio::test]
async fn rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals() {
    let app = router(
        Arc::new(Verifier),
        Arc::new(Backend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    for version in [
        "b10x.connector-operation.v0alpha2",
        "b10x.connector-operation.v0alpha1",
    ] {
        for (tenant, expected_status, expected_code) in [
            ("tenant-dev", StatusCode::OK, None),
            ("another-tenant", StatusCode::FORBIDDEN, Some("not_granted")),
        ] {
            let mut frame = serde_json::to_value(envelope(tenant)).unwrap();
            frame["protocol"] = serde_json::json!(version);
            let request = Request::post("/operations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::from(frame.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
                .await
                .unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(status, expected_status, "{body}");
            assert_eq!(
                body["protocol"], version,
                "every protocol response uses the requested identity"
            );
            assert_eq!(body["request_id"], "request-1");
            if let Some(code) = expected_code {
                assert_eq!(body["error"]["code"], code);
            }
        }
    }
}

struct RateRefusalBackend {
    calls: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl ConnectorBackend for RateRefusalBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        _: &service::PrincipalContext,
        _: protocol::operation::OperationRequest,
    ) -> Result<protocol::operation::OperationResult, OperationError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(OperationError::rate_limited(
            "provider refused this request",
            Some(30),
        ))
    }
}

#[tokio::test]
async fn rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend()
{
    let backend = Arc::new(RateRefusalBackend {
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let app = router(
        Arc::new(Verifier),
        backend.clone(),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    for version in [
        protocol::operation::CONTRACT,
        protocol::operation::legacy::CONTRACT,
    ] {
        for valid in [true, false] {
            let mut frame = serde_json::to_value(envelope("tenant-dev")).unwrap();
            frame["protocol"] = serde_json::json!(version);
            if !valid {
                frame["request"]["params"]["limit"] = serde_json::json!(0);
            }
            let request = Request::post("/operations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::from(frame.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                if valid {
                    StatusCode::OK
                } else {
                    StatusCode::BAD_REQUEST
                }
            );
            let bytes = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["protocol"], version);
            assert_eq!(value["request_id"], frame["request_id"]);
            if !valid {
                assert_eq!(value["error"]["code"], "invalid_input");
            } else {
                assert_eq!(value["error"]["retriable"], true);
                if version == protocol::operation::CONTRACT {
                    assert_eq!(value["error"]["code"], "rate_limited");
                    assert_eq!(value["error"]["retry_after_seconds"], 30);
                } else {
                    assert_eq!(value["error"]["code"], "unavailable");
                    assert!(value["error"].get("retry_after_seconds").is_none());
                }
            }
        }
    }
    assert_eq!(backend.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal() {
    let backend = Arc::new(RateRefusalBackend {
        calls: std::sync::atomic::AtomicUsize::new(0),
    });
    let app = router(
        Arc::new(Verifier),
        backend.clone(),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    for version in [
        protocol::operation::CONTRACT,
        protocol::operation::legacy::CONTRACT,
    ] {
        for invalid in [
            serde_json::json!(""),
            serde_json::json!("x".repeat(129)),
            serde_json::json!("bad\nreference"),
            serde_json::json!(17),
        ] {
            let mut frame = serde_json::to_value(envelope("tenant-dev")).unwrap();
            frame["protocol"] = serde_json::json!(version);
            frame["request_id"] = invalid;
            let response = app
                .clone()
                .oneshot(
                    Request::post("/operations")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(frame.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let bytes = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["protocol"], version);
            assert_eq!(value["request_id"], "invalid-request");
            assert_eq!(value["error"]["code"], "protocol");
        }
    }
    assert_eq!(backend.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
