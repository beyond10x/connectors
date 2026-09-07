async fn endpoint_fixture(
    inventory: Arc<dyn StateStore>,
) -> (
    tempfile::TempDir,
    MonitoringBackend,
    Arc<MemoryStore>,
    Arc<FakeExecutor>,
) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let owner = owner();
    let credential_ref = grafana_credential_ref(&owner).unwrap();
    let credentials = Arc::new(MemoryStore::new());
    let executor = Arc::new(FakeExecutor::default());
    let mut backend = MonitoringBackend::with_executor(
        owner,
        policy(),
        root.path(),
        credentials.clone(),
        credential_ref.clone(),
        executor.clone(),
    );
    Arc::get_mut(&mut backend.inner).unwrap().inventory_state = Some(inventory);
    lock(&backend.inner.state).parent = Some(ParentConnection {
        connection_ref: "connection:grafana:endpoints".into(),
        label: "Endpoint fixture".into(),
    });
    credentials
        .put(
            &credential_ref,
            &Secret::new("SENTINEL-ENDPOINT-SOURCE-TOKEN"),
        )
        .await
        .unwrap();
    backend.inner.refresh_endpoint_inventory().await.unwrap();
    (root, backend, credentials, executor)
}

#[tokio::test]
async fn endpoints_discover_unknown_datasources_and_lazily_invoke_through_grafana() {
    use protocol::endpoint::{self, EndpointRequest, EndpointResult, EndpointState};
    let (_root, backend, _, executor) =
        endpoint_fixture(Arc::new(connector_state::MemoryState::new())).await;
    let EndpointResult::List { endpoints, .. } = backend
        .handle_endpoint(
            &owner(),
            EndpointRequest::List(endpoint::ListRequest {
                source_ref: None,
                query: String::new(),
                limit: 100,
                cursor: None,
            }),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(endpoints.len(), 3);
    assert!(endpoints
        .iter()
        .any(|endpoint| endpoint.state == EndpointState::UnknownProvider));
    assert!(!serde_json::to_string(&endpoints)
        .unwrap()
        .contains("SENTINEL"));
    let endpoint = endpoints
        .iter()
        .find(|endpoint| endpoint.provider.as_deref() == Some("prometheus"))
        .unwrap();
    let connection = backend
        .resolve_endpoint(&owner(), &endpoint.endpoint_ref, PROMETHEUS_QUERY_RANGE)
        .await
        .unwrap();
    let description = backend.inner.describe_connection(&connection).unwrap();
    assert!(matches!(
        description.summary.route,
        ConnectionRoute::ViaConnection {
            route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
            ..
        }
    ));
    let OperationResult::Describe(description) = backend
        .handle(
            &owner(),
            OperationRequest::Describe(DescribeRequest {
                operation_ref: PROMETHEUS_QUERY_RANGE.into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let resolved = backend
        .resolve_endpoint(&owner(), &endpoint.endpoint_ref, PROMETHEUS_QUERY_RANGE)
        .await
        .unwrap();
    assert_eq!(connection, resolved);
    backend.handle(&owner(), OperationRequest::Invoke(InvokeRequest { operation_ref: PROMETHEUS_QUERY_RANGE.into(), connection_ref: resolved,
        description_ref: description.description_ref, input: serde_json::json!({"query":"up","start":"2026-01-01T00:00:00Z","end":"2026-01-01T00:01:00Z","step":"15s"}), approval_evidence_ref: None,
    })).await.unwrap();
    assert!(lock(&executor.requests)
        .last()
        .unwrap()
        .url
        .contains("/api/datasources/proxy/uid/prom-main/"));
}

#[tokio::test]
async fn endpoint_inventory_restores_identity_but_rechecks_current_target_grants() {
    use protocol::endpoint::{self, EndpointRequest, EndpointResult};
    let inventory: Arc<dyn StateStore> = Arc::new(connector_state::MemoryState::new());
    let (root, backend, credentials, executor) = endpoint_fixture(inventory.clone()).await;
    let EndpointResult::List { endpoints, .. } = backend
        .handle_endpoint(
            &owner(),
            EndpointRequest::List(endpoint::ListRequest {
                source_ref: None,
                query: String::new(),
                limit: 100,
                cursor: None,
            }),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let endpoint = endpoints
        .into_iter()
        .find(|endpoint| endpoint.provider.as_deref() == Some("prometheus"))
        .unwrap();
    backend
        .resolve_endpoint(&owner(), &endpoint.endpoint_ref, PROMETHEUS_QUERY_RANGE)
        .await
        .unwrap();
    let mut changed_policy = policy();
    changed_policy.target_grants.remove("prometheus");
    let mut restored = MonitoringBackend::with_executor(
        owner(),
        changed_policy,
        root.path(),
        credentials,
        grafana_credential_ref(&owner()).unwrap(),
        executor.clone(),
    );
    Arc::get_mut(&mut restored.inner).unwrap().inventory_state = Some(inventory);
    restored.inner.restore_inventory().unwrap();
    assert_eq!(restored.connection_count(), 1);
    let result = restored
        .handle_endpoint(
            &owner(),
            EndpointRequest::Show(endpoint::ShowRequest {
                endpoint_ref: endpoint.endpoint_ref.clone(),
            }),
        )
        .await
        .unwrap();
    let EndpointResult::Show {
        endpoint: restored_endpoint,
    } = result
    else {
        panic!()
    };
    assert_eq!(restored_endpoint.endpoint_ref, endpoint.endpoint_ref);
    assert_eq!(restored_endpoint.state, endpoint::EndpointState::Denied);
    assert!(restored
        .resolve_endpoint(&owner(), &endpoint.endpoint_ref, PROMETHEUS_QUERY_RANGE)
        .await
        .is_err());
    assert!(!lock(&executor.requests)
        .iter()
        .any(|request| request.url.contains("/proxy/")));
}
#[tokio::test]
async fn readiness_checks_only_the_mandatory_credential_store() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let owner = owner();
    let credential_ref = grafana_credential_ref(&owner).unwrap();
    let executor = Arc::new(FakeExecutor::default());
    let backend = MonitoringBackend::with_executor(
        owner,
        policy(),
        root.path(),
        Arc::new(FailingStore),
        credential_ref,
        Arc::clone(&executor),
    );

    assert_eq!(backend.ready().await, Err(BackendReadinessError));
    assert!(lock(&executor.requests).is_empty());
}
