#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use connector_secrets::{MemoryStore, StoreError};
    use monitoring_model::PROMETHEUS_QUERY_RANGE;
    use protocol::connection::ConnectSessionState;
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
    use tokio::net::UnixStream;

    #[derive(Default)]
    struct FakeExecutor {
        requests: Mutex<Vec<Request>>,
    }

    #[derive(Default)]
    struct BlockingExecutor {
        calls: AtomicUsize,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }

    struct FailingStore;

    #[async_trait]
    impl SecretStore for FailingStore {
        async fn ready(&self) -> Result<(), StoreError> {
            Err(StoreError::Unreachable {
                path: "test".to_owned(),
                reason: "injected failure".to_owned(),
            })
        }

        async fn get(&self, _reference: &CredentialRef) -> Result<Secret, StoreError> {
            Err(StoreError::NotFound {
                path: "test".to_owned(),
            })
        }

        async fn put(
            &self,
            _reference: &CredentialRef,
            _secret: &Secret,
        ) -> Result<(), StoreError> {
            Err(StoreError::Unreachable {
                path: "test".to_owned(),
                reason: "injected failure".to_owned(),
            })
        }

        async fn delete(&self, _reference: &CredentialRef) -> Result<(), StoreError> {
            Ok(())
        }
    }

    #[async_trait]
    impl HttpExecutor for FakeExecutor {
        async fn execute(&self, request: Request) -> Result<Value, MonitoringError> {
            let output = if request.url.contains("/api/datasources")
                && !request.url.contains("/proxy/")
            {
                serde_json::json!([
                    {"id":1,"uid":"prom-main","name":"Metrics","type":"prometheus"},
                    {"id":2,"uid":"loki-main","name":"Logs","type":"loki"},
                    {"id":3,"uid":"private-main","name":"Private plugin","type":"vendor-private"}
                ])
            } else {
                serde_json::json!({"status":"success","data":{"result":[]}})
            };
            lock(&self.requests).push(request);
            Ok(output)
        }
    }

    #[async_trait]
    impl HttpExecutor for BlockingExecutor {
        async fn execute(&self, _request: Request) -> Result<Value, MonitoringError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                self.entered.notify_one();
                self.release.notified().await;
            }
            Ok(serde_json::json!([]))
        }
    }

    fn owner() -> PrincipalContext {
        PrincipalContext::local(&protocol::operation::OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        })
        .unwrap()
    }

    fn policy() -> GrafanaIntegrationConfig {
        toml::from_str(
            r#"
origin = "https://grafana.example"
grant_ref = "grant:grafana"
initiation = "b10x"
connect_session_ttl_seconds = 300

[target_grants]
prometheus = "grant:prometheus"
loki = "grant:loki"
alertmanager = "grant:alertmanager"
"#,
        )
        .unwrap()
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

    #[tokio::test]
    async fn discovery_materialization_and_query_stay_on_the_grafana_route() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let executor = Arc::new(FakeExecutor::default());
        let owner = owner();
        let credential_ref = grafana_credential_ref(&owner).unwrap();
        let store = Arc::new(MemoryStore::new());
        let credential_store: Arc<dyn SecretStore> = store.clone();
        let backend = MonitoringBackend::with_executor(
            owner.clone(),
            policy(),
            root.path(),
            credential_store,
            credential_ref.clone(),
            Arc::clone(&executor),
        );
        let parent = ParentConnection {
            connection_ref: "connection:grafana:test".to_owned(),
            label: "Infrastructure Grafana".to_owned(),
        };
        let token = Secret::new("SENTINEL-NOT-A-REAL-SECRET");
        let output = backend
            .inner
            .execute_direct(
                &owner,
                &parent,
                &token,
                operation_document(GRAFANA_DATASOURCES_LIST).unwrap(),
                serde_json::json!({}),
            )
            .await
            .unwrap();
        backend
            .inner
            .reconcile_observations(&parent.connection_ref, &output)
            .unwrap();
        lock(&backend.inner.state).parent = Some(parent);
        store.put(&credential_ref, &token).await.unwrap();

        let observations = backend.inner.search_observations("", 64);
        assert_eq!(observations.len(), 3);
        assert!(observations.iter().any(|observation| {
            observation.observed_type == "vendor-private"
                && observation.state == DiscoveryObservationState::Unsupported
        }));
        let prometheus = observations
            .iter()
            .find(|observation| observation.observed_type == "prometheus")
            .unwrap();
        let child = backend
            .inner
            .materialize(&prometheus.observation_ref)
            .unwrap();
        assert!(matches!(
            child.summary.route,
            ConnectionRoute::ViaConnection {
                route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
                ..
            }
        ));

        let described = backend
            .handle(
                &owner,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: PROMETHEUS_QUERY_RANGE.to_owned(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::Describe(description) = described else {
            panic!("expected description")
        };
        let child_ref = child.summary.connection_ref.clone();
        backend
            .handle(
                &owner,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: PROMETHEUS_QUERY_RANGE.to_owned(),
                    connection_ref: child_ref.clone(),
                    description_ref: description.description_ref,
                    input: serde_json::json!({
                        "query":"up",
                        "start":"now-5m",
                        "end":"now",
                        "step":"30s"
                    }),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap();

        let requests = lock(&executor.requests);
        let query = requests.last().unwrap();
        assert!(query.url.starts_with(
            "https://grafana.example/api/datasources/proxy/uid/prom-main/api/v1/query_range?"
        ));
        assert!(!query.url.contains("mediated-target.invalid"));
        assert!(!format!("{query:?}").contains("SENTINEL"));

        let observation_ref = prometheus.observation_ref.clone();
        lock(&backend.inner.state)
            .observations
            .get_mut(&observation_ref)
            .unwrap()
            .target_provider = Some("loki".to_owned());
        assert!(backend
            .inner
            .connections_for_operation(PROMETHEUS_QUERY_RANGE)
            .is_empty());
        assert_eq!(
            backend
                .inner
                .describe_connection(&child_ref)
                .unwrap()
                .summary
                .state,
            ConnectionState::Degraded
        );
        assert!(backend.inner.materialize(&observation_ref).is_err());
    }

    #[tokio::test]
    async fn connect_session_uses_shared_transport_and_publishes_only_after_secret_custody() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let owner = owner();
        let credential_ref = grafana_credential_ref(&owner).unwrap();
        let store = Arc::new(MemoryStore::new());
        let credential_store: Arc<dyn SecretStore> = store.clone();
        let backend = MonitoringBackend::with_executor(
            owner.clone(),
            policy(),
            root.path(),
            credential_store,
            credential_ref.clone(),
            Arc::new(FakeExecutor::default()),
        );

        let created = backend
            .handle_connection(
                &owner,
                ConnectionRequest::ConnectSessionCreate(
                    protocol::connection::ConnectSessionCreateRequest {
                        integration_ref: GRAFANA.to_owned(),
                        label: "Infrastructure Grafana".to_owned(),
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionCreate(created) = created else {
            panic!("expected Connect Session creation")
        };
        let endpoint = PathBuf::from(created.completion_endpoint.clone().unwrap());
        let mut stream = UnixStream::connect(&endpoint).await.unwrap();
        stream
            .write_all(b"SENTINEL-NOT-A-REAL-SECRET\n")
            .await
            .unwrap();
        let mut response = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut response)
            .await
            .unwrap();

        assert_eq!(response, "{\"accepted\":true}\n");
        assert!(!endpoint.exists());
        assert_eq!(
            store
                .get(&credential_ref)
                .await
                .unwrap()
                .expose_secret(),
            "SENTINEL-NOT-A-REAL-SECRET"
        );
        let status = backend
            .handle_connection(
                &owner,
                ConnectionRequest::ConnectSessionStatus(
                    protocol::connection::ConnectSessionStatusRequest {
                        connect_session_ref: created.connect_session_ref,
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionStatus(status) = status else {
            panic!("expected Connect Session status")
        };
        assert_eq!(status.state, ConnectSessionState::Completed);
        assert_eq!(backend.connection_count(), 1);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn concurrent_completions_publish_exactly_one_parent_connection() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let owner = owner();
        let executor = Arc::new(BlockingExecutor::default());
        let backend = MonitoringBackend::with_executor(
            owner.clone(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            grafana_credential_ref(&owner).unwrap(),
            Arc::clone(&executor),
        );
        for (session, label) in [("connect-session:one", "One"), ("connect-session:two", "Two")]
        {
            lock(&backend.inner.sessions)
                .reserve(
                    session.to_owned(),
                    label.to_owned(),
                    1,
                    format!("/tmp/{label}.sock"),
                )
                .unwrap();
        }

        let first_inner = Arc::clone(&backend.inner);
        let first = tokio::spawn(async move {
            first_inner
                .complete_connection("connect-session:one", &Secret::new("first"))
                .await
        });
        executor.entered.notified().await;
        let second_inner = Arc::clone(&backend.inner);
        let second = tokio::spawn(async move {
            second_inner
                .complete_connection("connect-session:two", &Secret::new("second"))
                .await
        });
        assert!(tokio::time::timeout(
            Duration::from_millis(25),
            executor.entered.notified()
        )
        .await
        .is_err());
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        executor.release.notify_one();

        assert!(first.await.unwrap().is_ok());
        assert!(second.await.unwrap().is_err());
        assert_eq!(backend.connection_count(), 1);
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn failed_credential_custody_rolls_back_discovery_and_parent_state() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let owner = owner();
        let backend = MonitoringBackend::with_executor(
            owner.clone(),
            policy(),
            root.path(),
            Arc::new(FailingStore),
            grafana_credential_ref(&owner).unwrap(),
            Arc::new(FakeExecutor::default()),
        );
        lock(&backend.inner.sessions)
            .reserve(
                "connect-session:failed".to_owned(),
                "Failed".to_owned(),
                1,
                "/tmp/failed.sock".to_owned(),
            )
            .unwrap();

        assert!(backend
            .inner
            .complete_connection("connect-session:failed", &Secret::new("secret"))
            .await
            .is_err());
        let state = lock(&backend.inner.state);
        assert!(state.parent.is_none());
        assert!(state.observations.is_empty());
        assert!(state.children.is_empty());
    }

    #[tokio::test]
    async fn standalone_adapter_refuses_unowned_requests_without_fallthrough() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let owner = owner();
        let credential_ref = grafana_credential_ref(&owner).unwrap();
        let credential_store: Arc<dyn SecretStore> = Arc::new(MemoryStore::new());
        let backend = MonitoringBackend::with_executor(
            owner.clone(),
            policy(),
            root.path(),
            credential_store,
            credential_ref,
            Arc::new(FakeExecutor::default()),
        );

        assert_eq!(
            backend.capabilities(),
            BackendCapabilities {
                operations: true,
                connections: true,
                events: false,
                datasources: false,
            }
        );
        let operation_error = backend
            .handle(
                &owner,
                OperationRequest::SessionStatus(protocol::operation::SessionRequest {
                    execution_ref: "execution:elsewhere".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(operation_error.code, OperationErrorCode::NotFound);
        let connection_error = backend
            .handle_connection(
                &owner,
                ConnectionRequest::CandidateSearch(
                    protocol::connection::CandidateSearchRequest {
                        integration_ref: "elsewhere".to_owned(),
                        query: String::new(),
                        limit: 1,
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(connection_error.code, ConnectionErrorCode::NotFound);
    }
}
