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
struct RateAdversaryAdmissionBackend(std::sync::atomic::AtomicUsize);
#[async_trait]
impl ConnectorBackend for RateAdversaryAdmissionBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        if matches!(request, OperationRequest::Invoke(_)) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(OperationError::rate_limited(
                "definite refusal",
                Some(u64::MAX),
            ))
        } else {
            ApprovalBackend.handle(context, request).await
        }
    }
}

#[tokio::test]
async fn rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version() {
    for version in [
        protocol::operation::CONTRACT,
        protocol::operation::legacy::CONTRACT,
    ] {
        for case in [
            "unbound",
            "empty-grant",
            "missing-approval",
            "admitted",
            "tenant",
            "unknown-version",
        ] {
            let store = Arc::new(connector_state::MemoryState::new());
            if case != "empty-grant" {
                domain::GrantSet {
                    revision: 1,
                    grants: vec![domain::Grant {
                        grant: "grant:todo".into(),
                        provider: "provider:todo".into(),
                        connection: "connection:todo".into(),
                        selector: None,
                        allow: BTreeSet::from([APPROVAL_OPERATION.to_owned()]),
                        deny: BTreeSet::new(),
                        inbound_events: BTreeSet::new(),
                    }],
                }
                .write(&*store, "tenant-dev")
                .unwrap();
            }
            if case == "admitted" {
                issue_approval(
                    &*store,
                    &domain::ApprovalRecord {
                        reference: "approval:rate".into(),
                        issuer: "https://identity.example.test".into(),
                        subject: "person:test".into(),
                        operation: APPROVAL_OPERATION.into(),
                        connection: "connection:todo".into(),
                        input_digest: canonical_input_digest(&serde_json::json!({})),
                        expires_at_seconds: u64::MAX,
                    },
                )
                .unwrap();
            }
            let backend = Arc::new(RateAdversaryAdmissionBackend(
                std::sync::atomic::AtomicUsize::new(0),
            ));
            let authority = if case == "unbound" {
                HostedAuthority::unbound()
            } else {
                HostedAuthority::bound(store, "https://identity.example.test")
            };
            let app = router(
                Arc::new(Verifier),
                backend.clone(),
                HostedAdmissionPolicy::new(["operator".into()])
                    .with_generated_service_operations([APPROVAL_OPERATION.into()]),
                authority,
            );
            let mut request = invocation_envelope(
                APPROVAL_OPERATION,
                if case == "admitted" {
                    Some("approval:rate")
                } else {
                    None
                },
            );
            request.protocol = if case == "unknown-version" {
                "b10x.connector-operation.v9".into()
            } else {
                version.into()
            };
            if case == "tenant" {
                request.context.tenant_id = "other-tenant".into();
            }
            let OperationRequest::Invoke(invoke) = &mut request.request else {
                panic!()
            };
            invoke.connection_ref = "connection:todo".into();
            invoke.description_ref = "description:todo-create-list".into();
            let response = app.oneshot(operation_http_request(&request)).await.unwrap();
            let expected = match case {
                "admitted" => StatusCode::OK,
                "unbound" => StatusCode::SERVICE_UNAVAILABLE,
                "unknown-version" => StatusCode::BAD_REQUEST,
                _ => StatusCode::FORBIDDEN,
            };
            assert_eq!(response.status(), expected, "{case}");
            let bytes = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            if case != "unknown-version" {
                assert_eq!(value["protocol"], version, "{case}");
            }
            assert_eq!(value["request_id"], "request-1");
            assert_eq!(
                backend.0.load(std::sync::atomic::Ordering::SeqCst),
                usize::from(case == "admitted")
            );
            let expected_delay = if case == "admitted" && version == protocol::operation::CONTRACT {
                Some(u64::MAX)
            } else {
                None
            };
            assert_eq!(
                value["error"]
                    .get("retry_after_seconds")
                    .and_then(serde_json::Value::as_u64),
                expected_delay
            );
            if case == "admitted" {
                assert_eq!(
                    value["error"]["code"],
                    if version == protocol::operation::CONTRACT {
                        "rate_limited"
                    } else {
                        "unavailable"
                    }
                );
                assert_eq!(value["error"]["retriable"], true);
            }
        }
    }
}

struct FinalDescribeBackend(std::sync::atomic::AtomicUsize);
#[async_trait]
impl ConnectorBackend for FinalDescribeBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let OperationRequest::Describe(request) = request else {
            panic!("Describe only")
        };
        let source = if request.operation_ref.ends_with("valid")
            && !request.operation_ref.ends_with("invalid")
        {
            "https://%41.example.test:00065535/a%2fb?x=%23%40"
        } else {
            "https://docs.example.test:65536/private-SENTINEL"
        };
        Ok(OperationResult::Describe(
            protocol::operation::OperationDescription {
                operation_ref: request.operation_ref,
                title: "Fixture read".into(),
                description: "Fixture metadata".into(),
                input_schema: serde_json::json!({"type":"object","additionalProperties":false}),
                output_schema: serde_json::json!({"type":"object","properties":{"vendor":{"const":"retained"}}}),
                effect: EffectClass::ReadOnly,
                approval: ApprovalPosture::NotRequired,
                connections: vec![],
                description_ref: "fixture-description".into(),
                rate_advice: Some(protocol::operation::OperationRateAdvice {
                    fixed: None,
                    alternatives: vec![protocol::operation::ConditionalRateAdvice::new(
                        protocol::operation::ConditionalRateLimit {
                            applies_when: "Fixture category".into(),
                            rate: None,
                            source_url: source.into(),
                        },
                    )],
                }),
            },
        ))
    }
}

struct FinalCountingVerifier(std::sync::atomic::AtomicUsize);
#[async_trait]
impl IdentityVerifier for FinalCountingVerifier {
    async fn ready(&self) -> Result<(), IdentityVerificationError> {
        Ok(())
    }
    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Verifier.verify(credential, audience).await
    }
}
#[tokio::test]
async fn rate_final_hosted_describe_versions_reject_bad_advice_before_loss() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let backend = Arc::new(FinalDescribeBackend(AtomicUsize::new(0)));
    let verifier = Arc::new(FinalCountingVerifier(AtomicUsize::new(0)));
    let app = router(
        verifier.clone(),
        backend.clone(),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    );
    for version in [
        protocol::operation::CONTRACT,
        protocol::operation::legacy::CONTRACT,
    ] {
        for operation in ["fixture.valid", "fixture.invalid", ""] {
            let mut frame = serde_json::to_value(envelope("tenant-dev")).unwrap();
            frame["protocol"] = serde_json::json!(version);
            frame["request"] =
                serde_json::json!({"method":"describe","params":{"operation_ref":operation}});
            let before_verifier = verifier.0.load(Ordering::SeqCst);
            let before_backend = backend.0.load(Ordering::SeqCst);
            let request = Request::post("/operations")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::from(frame.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                if operation.is_empty() {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::OK
                }
            );
            let bytes = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
                .await
                .unwrap();
            let reply: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(reply["protocol"], version);
            assert_eq!(reply["request_id"], "request-1");
            let calls = usize::from(!operation.is_empty());
            assert_eq!(verifier.0.load(Ordering::SeqCst), before_verifier + calls);
            assert_eq!(backend.0.load(Ordering::SeqCst), before_backend + calls);
            if operation == "fixture.valid" {
                assert_eq!(reply["status"], "ok", "{reply}");
                let value = &reply["response"]["value"];
                assert_eq!(value["operation_ref"], operation);
                assert_eq!(
                    value["output_schema"]["properties"]["vendor"]["const"],
                    "retained"
                );
                if version == protocol::operation::CONTRACT {
                    assert_eq!(
                        value["rate_advice"]["alternatives"][0]["declaration"]["source_url"],
                        "https://%41.example.test:00065535/a%2fb?x=%23%40"
                    );
                } else {
                    assert!(value.get("rate_advice").is_none());
                }
            } else {
                assert_eq!(reply["status"], "error");
                assert_eq!(
                    reply["error"]["code"],
                    if operation.is_empty() {
                        "invalid_input"
                    } else {
                        "protocol"
                    }
                );
                assert!(!String::from_utf8_lossy(&bytes).contains("SENTINEL"));
            }
        }
    }
    assert_eq!(backend.0.load(Ordering::SeqCst), 4);
    assert_eq!(verifier.0.load(Ordering::SeqCst), 4);
}
