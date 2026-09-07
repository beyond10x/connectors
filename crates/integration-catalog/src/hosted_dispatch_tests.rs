// Exercise separate public requests; a private delegate shared by a test hides lease loss.
async fn describe_hosted(
    backend: &HostedCatalogBackend,
    owner: &PrincipalContext,
) -> Result<protocol::operation::OperationDescription, OperationError> {
    let result = backend
        .handle(
            owner,
            OperationRequest::Describe(protocol::operation::DescribeRequest {
                operation_ref: "grafana-datasources-list".to_owned(),
            }),
        )
        .await?;
    let OperationResult::Describe(description) = result else {
        panic!("description response")
    };
    Ok(description)
}
async fn invoke_hosted(
    backend: &HostedCatalogBackend,
    owner: &PrincipalContext,
    connection: &str,
    description: &str,
    input: serde_json::Value,
) -> Result<protocol::operation::InvocationResult, OperationError> {
    let result = backend
        .handle(
            owner,
            OperationRequest::Invoke(protocol::operation::InvokeRequest {
                operation_ref: "grafana-datasources-list".to_owned(),
                connection_ref: connection.to_owned(),
                description_ref: description.to_owned(),
                input,
                approval_evidence_ref: None,
            }),
        )
        .await?;
    let OperationResult::Invoke(result) = result else {
        panic!("invocation response")
    };
    Ok(result)
}

#[tokio::test]
async fn public_describe_then_invoke_survives_scope_token_and_request_rotation() {
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let connection = connect(&backend, &owner).await;
    let description = describe_hosted(&backend, &owner).await.unwrap();
    let renewed = PrincipalContext::hosted(
        "tenant-test".to_owned(),
        "first".to_owned(),
        "first".to_owned(),
        Some("first@example.test".to_owned()),
        "snapshot:renewed".to_owned(),
        "b".repeat(64),
    )
    .unwrap()
    .with_hosted_provenance(
        "https://identity.example".to_owned(),
        "token:renewed".to_owned(),
        Some("deployment:test".to_owned()),
        "request:invoke".to_owned(),
        "trace:invoke".to_owned(),
    )
    .unwrap();
    invoke_hosted(
        &backend,
        &renewed,
        &connection,
        &description.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    invoke_hosted(
        &backend,
        &renewed,
        &connection,
        &description.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    assert_eq!(
        lock(&egress.urls).len(),
        3,
        "one verification and two actual operation exchanges"
    );
    assert_eq!(lock(&backend.inner.operation_delegates).len(), 1);
}

#[tokio::test]
async fn public_invocation_without_its_own_current_description_has_zero_egress() {
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let connection = connect(&backend, &owner).await;
    let description = describe_hosted(&backend, &owner).await.unwrap();
    let changed_groups = PrincipalContext::hosted_with_groups(
        "tenant-test".to_owned(),
        "first".to_owned(),
        "first".to_owned(),
        Some("first@example.test".to_owned()),
        "snapshot:new-groups".to_owned(),
        "c".repeat(64),
        BTreeSet::from(["readers".to_owned()]),
    )
    .unwrap();
    let foreign_tenant = PrincipalContext::hosted(
        "tenant-other".to_owned(),
        "first".to_owned(),
        "first".to_owned(),
        Some("first@example.test".to_owned()),
        "snapshot:other".to_owned(),
        "c".repeat(64),
    )
    .unwrap();
    for context in [
        principal("second"),
        foreign_tenant,
        changed_groups.clone(),
        owner
            .clone()
            .with_verified_realm(Some("realm:other".to_owned()))
            .unwrap(),
    ] {
        let error = invoke_hosted(
            &backend,
            &context,
            &connection,
            &description.description_ref,
            serde_json::json!({}),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.code,
            protocol::operation::OperationErrorCode::StaleAuthority
        );
    }
    assert_eq!(lock(&egress.urls).len(), 1);
    let fresh = describe_hosted(&backend, &changed_groups).await.unwrap();
    invoke_hosted(
        &backend,
        &changed_groups,
        &connection,
        &fresh.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    assert_eq!(lock(&egress.urls).len(), 2);
}

#[tokio::test]
async fn delegated_description_is_bound_to_the_exact_attempt_and_grant_revision() {
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let connection = connect(&backend, &owner).await;
    let delegated = |attempt: &str, delegation: &str, grant: &str, revision| {
        PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "first".to_owned(),
            "agent:worker".to_owned(),
            Some("first@example.test".to_owned()),
            "snapshot:delegated".to_owned(),
            "a".repeat(64),
        )
        .unwrap()
        .with_verified_execution(
            service::DelegatedExecution::after_verification(
                "agent:worker".to_owned(),
                attempt.to_owned(),
                delegation.to_owned(),
                grant.to_owned(),
                revision,
            )
            .unwrap(),
        )
        .unwrap()
    };
    let admitted = delegated("attempt:one", "delegation:one", "grant:one", 1);
    let description = describe_hosted(&backend, &admitted).await.unwrap();
    for changed in [
        owner,
        delegated("attempt:two", "delegation:one", "grant:one", 1),
        delegated("attempt:one", "delegation:two", "grant:one", 1),
        delegated("attempt:one", "delegation:one", "grant:two", 1),
        delegated("attempt:one", "delegation:one", "grant:one", 2),
    ] {
        let error = invoke_hosted(
            &backend,
            &changed,
            &connection,
            &description.description_ref,
            serde_json::json!({}),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.code,
            protocol::operation::OperationErrorCode::StaleAuthority
        );
    }
    assert_eq!(lock(&egress.urls).len(), 1);
    invoke_hosted(
        &backend,
        &admitted,
        &connection,
        &description.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    assert_eq!(lock(&egress.urls).len(), 2);
}

#[tokio::test]
async fn changing_or_removing_current_binding_cannot_reuse_a_retained_delegate() {
    let egress = Arc::new(RecordingEgress::default());
    let mut backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let connection = connect(&backend, &owner).await;
    let description = describe_hosted(&backend, &owner).await.unwrap();
    let saved = lock(&backend.inner.metadata).connections[0].clone();
    lock(&backend.inner.metadata).connections[0]
        .binding
        .endpoints
        .insert(
            "origin".to_owned(),
            "https://changed.monitoring.example".to_owned(),
        );
    assert!(invoke_hosted(
        &backend,
        &owner,
        &connection,
        &description.description_ref,
        serde_json::json!({})
    )
    .await
    .is_err());
    assert!(describe_hosted(&backend, &owner).await.is_err());
    lock(&backend.inner.metadata).connections[0] = saved;
    lock(&backend.inner.metadata).connections[0].label = "Renamed monitoring".to_owned();
    assert!(invoke_hosted(
        &backend,
        &owner,
        &connection,
        &description.description_ref,
        serde_json::json!({})
    )
    .await
    .is_err());
    let fresh = describe_hosted(&backend, &owner).await.unwrap();
    invoke_hosted(
        &backend,
        &owner,
        &connection,
        &fresh.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    Arc::get_mut(&mut backend.inner).unwrap().grant_ref = "grant:catalog-replaced".to_owned();
    assert!(invoke_hosted(
        &backend,
        &owner,
        &connection,
        &fresh.description_ref,
        serde_json::json!({})
    )
    .await
    .is_err());
    let fresh = describe_hosted(&backend, &owner).await.unwrap();
    invoke_hosted(
        &backend,
        &owner,
        &connection,
        &fresh.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    lock(&backend.inner.metadata).connections.clear();
    assert!(invoke_hosted(
        &backend,
        &owner,
        &connection,
        &fresh.description_ref,
        serde_json::json!({})
    )
    .await
    .is_err());
    assert_eq!(
        lock(&egress.urls).len(),
        3,
        "changed/removed bindings never reach the transport"
    );
}

#[tokio::test]
async fn delegate_retention_is_bounded_and_eviction_requires_a_fresh_description() {
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let connection = connect(&backend, &owner).await;
    let description = describe_hosted(&backend, &owner).await.unwrap();
    for index in 0..MAX_OPERATION_DELEGATES {
        backend
            .inner
            .delegate(&principal(&format!("other-{index}")))
            .unwrap();
    }
    assert_eq!(
        lock(&backend.inner.operation_delegates).len(),
        MAX_OPERATION_DELEGATES
    );
    let error = invoke_hosted(
        &backend,
        &owner,
        &connection,
        &description.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.code,
        protocol::operation::OperationErrorCode::StaleAuthority
    );
    let fresh = describe_hosted(&backend, &owner).await.unwrap();
    invoke_hosted(
        &backend,
        &owner,
        &connection,
        &fresh.description_ref,
        serde_json::json!({}),
    )
    .await
    .unwrap();
    assert_eq!(
        lock(&backend.inner.operation_delegates).len(),
        MAX_OPERATION_DELEGATES
    );
    assert_eq!(lock(&egress.urls).len(), 2);
}

#[tokio::test]
async fn public_pending_status_validates_without_replaying_creation_capability() {
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let created = backend
        .handle_connection(
            &owner,
            ConnectionRequest::ConnectSessionCreate(
                protocol::connection::ConnectSessionCreateRequest {
                    integration_ref: "grafana".to_owned(),
                    label: "Monitoring".to_owned(),
                    auth_profile: Some("grafana.service_account_token".to_owned()),
                },
            ),
        )
        .await
        .unwrap();
    protocol::connection::ResponseEnvelope::success("request:create", created.clone())
        .validate()
        .unwrap();
    let ConnectionResult::ConnectSessionCreate(created) = created else {
        panic!("created session")
    };
    assert!(created.browser_completion_url.is_some());
    for _ in 0..2 {
        let status = backend
            .handle_connection(
                &owner,
                ConnectionRequest::ConnectSessionStatus(
                    protocol::connection::ConnectSessionStatusRequest {
                        connect_session_ref: created.connect_session_ref.clone(),
                    },
                ),
            )
            .await
            .unwrap();
        let envelope =
            protocol::connection::ResponseEnvelope::success("request:status", status.clone());
        envelope.validate().unwrap();
        let ConnectionResult::ConnectSessionStatus(status) = status else {
            panic!("session status")
        };
        assert_eq!(status.state, ConnectSessionState::Pending);
        assert!(status.completion_endpoint.is_none());
        assert!(status.browser_completion_url.is_none());
        assert!(!serde_json::to_string(&envelope).unwrap().contains("token="));
    }
    assert!(lock(&egress.urls).is_empty());
}
