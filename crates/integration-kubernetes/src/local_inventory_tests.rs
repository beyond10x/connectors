#[cfg(test)]
mod inventory_tests {
    use super::*;
    use protocol::operation::{OwnerContext, SearchRequest};
    use serde_json::{json, Value};
    use std::sync::Arc;

    const NAMESPACES: &str = "kubernetes.namespace.list";
    const WORKLOADS: &str = "kubernetes.workload.list";

    fn fixture_client(responses: Vec<(u16, Value)>) -> (Client, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let responses = Arc::new(Mutex::new(std::collections::VecDeque::from(responses)));
        let service = tower::service_fn(move |request: http::Request<kube::client::Body>| {
            recorded.lock().unwrap().push(request.uri().to_string());
            assert_eq!(request.method(), http::Method::GET);
            let (status, value) = responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected cluster request");
            async move {
                Ok::<_, std::io::Error>(
                    http::Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .body(kube::client::Body::from(
                            serde_json::to_vec(&value).unwrap(),
                        ))
                        .unwrap(),
                )
            }
        });
        (Client::new(service, "default"), requests)
    }

    fn backend(clients: Vec<(&str, Client)>, namespaces: &[&str]) -> KubernetesLocalBackend {
        let owner = PrincipalContext::local(&OwnerContext {
            tenant_id: "tenant-inventory".into(),
            agent_id: "agent-inventory".into(),
            agent_revision: 1,
            authority_snapshot_id: "inventory-authority-1".into(),
            authority_snapshot_sha256: "a".repeat(64),
        })
        .unwrap();
        let mut state = KubernetesState::default();
        for (reference, client) in clients {
            state.clients.insert(reference.to_owned(), client);
            state.connections.insert(
                reference.to_owned(),
                ConnectionDescription {
                    summary: ConnectionSummary {
                        connection_ref: reference.to_owned(),
                        integration_ref: KUBERNETES.to_owned(),
                        label: reference.to_owned(),
                        state: ConnectionState::Authorized,
                        initiation: vec![ConnectionInitiator::Platform],
                        route: ConnectionRoute::Direct,
                        scope: None,
                        actor: None,
                        auth_profile: None,
                    },
                    channels: Vec::new(),
                },
            );
        }
        KubernetesLocalBackend {
            owner,
            policy: KubernetesIntegrationConfig {
                grant_ref: "grant:kubernetes:inventory".into(),
                initiation: InitiationConfig::Platform,
                namespaces: namespaces.iter().map(|value| (*value).to_owned()).collect(),
                target_grants: BTreeMap::new(),
                allow_exec_auth: false,
                resource_limit: 256,
            },
            candidates: BTreeMap::new(),
            state: Mutex::new(state),
            activation: tokio::sync::Mutex::new(()),
            workloads: WorkloadSurface::default(),
        }
    }

    fn request(
        backend: &KubernetesLocalBackend,
        operation: &str,
        connection: &str,
        input: Value,
    ) -> OperationRequest {
        OperationRequest::Invoke(InvokeRequest {
            operation_ref: operation.into(),
            connection_ref: connection.into(),
            description_ref: backend.operation_description_ref(&backend.owner, operation),
            input,
            approval_evidence_ref: None,
        })
    }

    async fn invoke(
        backend: &KubernetesLocalBackend,
        operation: &str,
        connection: &str,
        input: Value,
    ) -> Result<Value, OperationError> {
        match backend
            .handle(
                &backend.owner,
                request(backend, operation, connection, input),
            )
            .await?
        {
            OperationResult::Invoke(result) => Ok(result.output),
            _ => panic!("expected invocation result"),
        }
    }

    fn deployment(name: &str, namespace: &str, image: &str, replicas: i32) -> Value {
        json!({
            "metadata": {"name": name, "namespace": namespace},
            "spec": {"replicas": replicas, "template": {"spec": {"containers": [
                {"name": "app", "image": image, "env": [{"name": "PRIVATE", "value": "must-not-escape"}]}
            ]}}},
            "status": {"readyReplicas": replicas}
        })
    }

    fn page(items: Vec<Value>, cursor: &str) -> Value {
        json!({"metadata": {"continue": cursor}, "items": items})
    }

    #[tokio::test]
    async fn inventory_discovery_publishes_both_reads_for_every_activated_connection() {
        let (alpha, _) = fixture_client(vec![]);
        let (beta, _) = fixture_client(vec![]);
        let backend = backend(
            vec![("connection:alpha", alpha), ("connection:beta", beta)],
            &["apps"],
        );
        let OperationResult::Search { operations } = backend
            .handle(
                &backend.owner,
                OperationRequest::Search(SearchRequest {
                    query: "kubernetes".into(),
                    limit: 25,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("search result")
        };
        for operation in [NAMESPACES, WORKLOADS] {
            let summary = operations
                .iter()
                .find(|summary| summary.operation_ref == operation)
                .expect("inventory operation is discoverable");
            assert_eq!(summary.effect, EffectClass::ReadOnly);
            assert_eq!(summary.approval, ApprovalPosture::NotRequired);
            assert_eq!(
                summary
                    .connections
                    .iter()
                    .map(|connection| connection.connection_ref.as_str())
                    .collect::<Vec<_>>(),
                ["connection:alpha", "connection:beta"]
            );
            let description = backend
                .operation_description(&backend.owner, operation)
                .unwrap();
            assert_eq!(description.connections, summary.connections);
            assert_eq!(description.output_schema["additionalProperties"], false);
            assert!(backend.owns_operation(&request(
                &backend,
                operation,
                "connection:beta",
                json!({})
            )));
        }
    }

    #[tokio::test]
    async fn inventory_namespaces_are_configured_admission_without_cluster_enumeration() {
        let (client, requests) = fixture_client(vec![]);
        let backend = backend(
            vec![("connection:alpha", client)],
            &["apps", "monitoring", "apps"],
        );
        assert_eq!(
            invoke(&backend, NAMESPACES, "connection:alpha", json!({}))
                .await
                .unwrap(),
            json!({
                "connection_ref": "connection:alpha", "namespaces": ["apps", "monitoring"]
            })
        );
        assert!(requests.lock().unwrap().is_empty());
        let (client, _) = fixture_client(vec![]);
        let empty = self::backend(vec![("connection:alpha", client)], &[]);
        assert_eq!(
            invoke(&empty, NAMESPACES, "connection:alpha", json!({}))
                .await
                .unwrap()["namespaces"],
            json!([])
        );
        assert_eq!(
            invoke(
                &empty,
                WORKLOADS,
                "connection:alpha",
                json!({"namespace": "apps"})
            )
            .await
            .unwrap_err()
            .code,
            OperationErrorCode::NotGranted
        );
    }

    #[tokio::test]
    async fn inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images() {
        let (alpha, alpha_requests) = fixture_client(vec![(
            200,
            page(vec![deployment("alpha", "apps", "example/alpha:v1", 0)], ""),
        )]);
        let (beta, beta_requests) = fixture_client(vec![(
            200,
            page(vec![deployment("beta", "apps", "example/beta:v2", 3)], ""),
        )]);
        let backend = backend(
            vec![("connection:alpha", alpha), ("connection:beta", beta)],
            &["apps"],
        );
        for (reference, name, image, replicas) in [
            ("connection:beta", "beta", "example/beta:v2", 3),
            ("connection:alpha", "alpha", "example/alpha:v1", 0),
        ] {
            let output = invoke(&backend, WORKLOADS, reference, json!({"namespace": "apps"}))
                .await
                .unwrap();
            assert_eq!(
                output,
                json!({"connection_ref": reference, "namespace": "apps", "deployments": [
                    {"name": name, "containers": [{"name": "app", "image": image}], "desired_replicas": replicas, "ready_replicas": replicas}
                ]})
            );
            assert!(!output.to_string().contains("must-not-escape"));
        }
        for requests in [alpha_requests, beta_requests] {
            assert_eq!(
                *requests.lock().unwrap(),
                ["/apis/apps/v1/namespaces/apps/deployments?limit=5"]
            );
        }
    }

    #[tokio::test]
    async fn inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io(
    ) {
        let (client, calls) = fixture_client(vec![]);
        let backend = backend(vec![("connection:alpha", client)], &["apps"]);
        for operation in [NAMESPACES, WORKLOADS] {
            assert_eq!(
                invoke(
                    &backend,
                    operation,
                    "connection:never-activated",
                    json!({"namespace": "apps"})
                )
                .await
                .unwrap_err()
                .code,
                OperationErrorCode::NotFound
            );
        }
        assert_eq!(
            invoke(
                &backend,
                WORKLOADS,
                "connection:alpha",
                json!({"namespace": "private"})
            )
            .await
            .unwrap_err()
            .code,
            OperationErrorCode::NotGranted
        );
        for input in [
            json!({}),
            json!({"namespace": "apps", "limit": 0}),
            json!({"namespace": "apps", "limit": 101}),
            json!({"namespace": "apps", "cursor": ""}),
            json!({"namespace": "apps", "extra": true}),
            json!({"namespace": "../secrets"}),
        ] {
            assert_eq!(
                invoke(&backend, WORKLOADS, "connection:alpha", input.clone())
                    .await
                    .unwrap_err()
                    .code,
                OperationErrorCode::InvalidInput,
                "{input}"
            );
        }
        assert_eq!(
            invoke(
                &backend,
                NAMESPACES,
                "connection:alpha",
                json!({"extra": true})
            )
            .await
            .unwrap_err()
            .code,
            OperationErrorCode::InvalidInput
        );
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn inventory_preserves_rbac_refusal() {
        let (client, calls) = fixture_client(vec![(
            403,
            json!({"kind": "Status", "apiVersion": "v1", "status": "Failure", "reason": "Forbidden", "message": "deployment list is forbidden", "code": 403}),
        )]);
        let backend = backend(vec![("connection:alpha", client)], &["apps"]);
        let error = invoke(
            &backend,
            WORKLOADS,
            "connection:alpha",
            json!({"namespace": "apps"}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::NotGranted);
        assert!(!error.retriable);
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace() {
        let first = deployment("first", "apps", "example/app:v1", 1);
        let responses = (0..8)
            .map(|_| (200, page(vec![first.clone()], "provider-secret-cursor")))
            .chain([(
                200,
                page(vec![deployment("last", "apps", "example/app:v2", 1)], ""),
            )])
            .collect();
        let (alpha, calls) = fixture_client(responses);
        let (beta, beta_calls) = fixture_client(vec![]);
        let backend = backend(
            vec![("connection:alpha", alpha), ("connection:beta", beta)],
            &["apps", "monitoring"],
        );
        let first = invoke(
            &backend,
            WORKLOADS,
            "connection:alpha",
            json!({"namespace": "apps", "limit": 100}),
        )
        .await
        .unwrap();
        assert_eq!(first["deployments"].as_array().unwrap().len(), 8);
        assert_eq!(calls.lock().unwrap().len(), 8);
        let cursor = first["next_cursor"]
            .as_str()
            .expect("short page carries continuation");
        assert!(!cursor.contains("provider-secret-cursor"));
        let second = invoke(
            &backend,
            WORKLOADS,
            "connection:alpha",
            json!({"namespace": "apps", "limit": 1, "cursor": cursor}),
        )
        .await
        .unwrap();
        assert_eq!(second["deployments"][0]["name"], "last");
        assert!(second.get("next_cursor").is_none());
        assert!(calls.lock().unwrap()[8].contains("limit=1&continue=provider-secret-cursor"));
        assert!(invoke(
            &backend,
            WORKLOADS,
            "connection:alpha",
            json!({"namespace": "apps", "cursor": cursor})
        )
        .await
        .is_err());
        assert!(invoke(
            &backend,
            WORKLOADS,
            "connection:beta",
            json!({"namespace": "apps", "cursor": cursor})
        )
        .await
        .is_err());
        assert!(invoke(
            &backend,
            WORKLOADS,
            "connection:alpha",
            json!({"namespace": "monitoring", "cursor": cursor})
        )
        .await
        .is_err());
        assert_eq!(calls.lock().unwrap().len(), 9);
        assert!(beta_calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn inventory_refuses_upstream_page_overrun_and_wrong_namespace() {
        for items in [
            vec![deployment("wrong", "private", "example/app:v1", 1)],
            vec![deployment("too-many", "apps", "example/app:v1", 1); 6],
        ] {
            let (client, calls) = fixture_client(vec![(200, page(items, ""))]);
            let backend = backend(vec![("connection:alpha", client)], &["apps"]);
            assert!(invoke(
                &backend,
                WORKLOADS,
                "connection:alpha",
                json!({"namespace": "apps"})
            )
            .await
            .is_err());
            assert_eq!(calls.lock().unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary() {
        for (connection, namespace) in [
            ("connection:beta", "apps"),
            ("connection:alpha", "monitoring"),
        ] {
            let (alpha, calls) = fixture_client(vec![(
                200,
                page(
                    vec![deployment("first", "apps", "example/app:v1", 1)],
                    "private-provider-cursor",
                ),
            )]);
            let (beta, beta_calls) = fixture_client(vec![]);
            let backend = backend(
                vec![("connection:alpha", alpha), ("connection:beta", beta)],
                &["apps", "monitoring"],
            );
            let first = invoke(
                &backend,
                WORKLOADS,
                "connection:alpha",
                json!({"namespace": "apps", "limit": 1}),
            )
            .await
            .unwrap();
            let error = invoke(
                &backend,
                WORKLOADS,
                connection,
                json!({"namespace": namespace, "cursor": first["next_cursor"]}),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code, OperationErrorCode::NotGranted);
            assert_eq!(calls.lock().unwrap().len(), 1);
            assert!(beta_calls.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn inventory_preserves_owner_and_description_lease_admission() {
        let (client, calls) = fixture_client(vec![]);
        let backend = backend(vec![("connection:alpha", client)], &["apps"]);
        for operation in [NAMESPACES, WORKLOADS] {
            let OperationRequest::Invoke(mut request) = request(
                &backend,
                operation,
                "connection:alpha",
                json!({"namespace": "apps"}),
            ) else {
                unreachable!()
            };
            request.description_ref = "description:stale".into();
            let error = backend
                .handle(&backend.owner, OperationRequest::Invoke(request))
                .await
                .unwrap_err();
            assert_eq!(error.code, OperationErrorCode::StaleAuthority);
            let other = self::backend(vec![], &[]);
            let context = PrincipalContext::local(&OwnerContext {
                tenant_id: "other-tenant".into(),
                agent_id: "agent-other".into(),
                agent_revision: 1,
                authority_snapshot_id: "different-authority".into(),
                authority_snapshot_sha256: "b".repeat(64),
            })
            .unwrap();
            let error = backend
                .handle(
                    &context,
                    self::request(&other, operation, "connection:alpha", json!({})),
                )
                .await
                .unwrap_err();
            assert_eq!(error.code, OperationErrorCode::StaleAuthority);
        }
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn inventory_refuses_oversized_provider_and_projected_values() {
        let large_image = "x".repeat(140_000);
        for responses in [
            vec![(
                200,
                page(
                    vec![deployment("oversized", "apps", &"x".repeat(300_000), 0)],
                    "",
                ),
            )],
            vec![
                (
                    200,
                    page(vec![deployment("first", "apps", &large_image, 0)], "more"),
                ),
                (
                    200,
                    page(vec![deployment("second", "apps", &large_image, 0)], ""),
                ),
            ],
        ] {
            let (client, _) = fixture_client(responses);
            let backend = backend(vec![("connection:alpha", client)], &["apps"]);
            assert_eq!(
                invoke(
                    &backend,
                    WORKLOADS,
                    "connection:alpha",
                    json!({"namespace": "apps"})
                )
                .await
                .unwrap_err()
                .code,
                OperationErrorCode::ResultTooLarge
            );
        }
    }

    #[tokio::test]
    async fn inventory_shared_reader_preserves_the_existing_compact_datasource_shape() {
        let (client, calls) = fixture_client(vec![(
            200,
            page(vec![deployment("app", "apps", "example/app:v1", 0)], ""),
        )]);
        let reader = KubeconfigReader::new(client);
        let result = reader.list_workloads("apps", 25, None).await.unwrap();
        assert_eq!(result.workloads.len(), 1);
        let value = serde_json::to_value(&result.workloads[0]).unwrap();
        assert_eq!(
            value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "desired_replicas",
                "name",
                "ready_replicas",
                "rollout_state"
            ]
        );
        assert_eq!(value["desired_replicas"], 0);
        assert!(result.next_cursor.is_none());
        assert_eq!(calls.lock().unwrap().len(), 1);
    }
}
