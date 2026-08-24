#[cfg(test)]
mod tests {
    use super::*;
    use protocol::datasource::{Completeness, DatasourceRead, RecordView};

    struct Reader;

    #[async_trait]
    impl DeploymentReader for Reader {
        async fn read(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<DeploymentStatus, OperationError> {
            Ok(DeploymentStatus {
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                generation: 3,
                observed_generation: 3,
                desired_replicas: 2,
                ready_replicas: 2,
                available_replicas: 2,
                updated_replicas: 2,
                running: true,
                available_condition: true,
            })
        }

        async fn list_workloads(
            &self,
            _namespace: &str,
            _limit: u16,
            _cursor: Option<&str>,
        ) -> Result<WorkloadList, DatasourceError> {
            Ok(WorkloadList {
                workloads: vec![WorkloadCompact {
                    name: "backend".to_owned(),
                    desired_replicas: 2,
                    ready_replicas: 2,
                    rollout_state: "available".to_owned(),
                }],
                next_cursor: None,
            })
        }

        async fn workload_detail(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<WorkloadDetail, DatasourceError> {
            Ok(WorkloadDetail {
                workload: self
                    .list_workloads(namespace, 1, None)
                    .await?
                    .workloads
                    .remove(0),
                meta: WorkloadMeta {
                    namespace: namespace.to_owned(),
                    uid: "uid-backend".to_owned(),
                    resource_version: "42".to_owned(),
                    generation: 3,
                    observed_generation: 3,
                    updated_replicas: 2,
                    available_replicas: 2,
                    unavailable_replicas: 0,
                },
                pods: vec![PodSummary {
                    name: format!("{name}-abc"),
                    phase: "Running".to_owned(),
                    ready_containers: 1,
                    total_containers: 1,
                    restart_count: 0,
                    containers: vec![ContainerSummary {
                        name: "app".to_owned(),
                        image: "registry.example/backend:v1".to_owned(),
                        image_id: "docker-pullable://registry.example/backend@sha256:abc"
                            .to_owned(),
                        ready: true,
                        restart_count: 0,
                        state_reason: None,
                    }],
                }],
                warnings: vec![WarningSummary {
                    involved_kind: "Deployment".to_owned(),
                    involved_name: name.to_owned(),
                    reason: "BackOff".to_owned(),
                    count: 1,
                    first_observed_at: None,
                    last_observed_at: None,
                }],
                related_complete: true,
            })
        }

        async fn restart(
            &self,
            namespace: &str,
            name: &str,
            uid: &str,
            _resource_version: &str,
        ) -> Result<RestartAccepted, OperationError> {
            Ok(RestartAccepted {
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                uid: uid.to_owned(),
                resource_version: "43".to_owned(),
                patch_accepted: true,
            })
        }
    }

    /// One recorded `pod_logs` call: namespace, pod, container, tail lines, since seconds.
    type SeenLogRead = (String, String, Option<String>, u32, Option<u32>);

    /// A pod-log fake: records every call it sees and answers with a configured body, standing in
    /// for the in-cluster reader after it has already mapped the Kubernetes API response.
    struct LogReader {
        text: String,
        truncated: bool,
        refusal: Option<OperationError>,
        seen: std::sync::Mutex<Vec<SeenLogRead>>,
    }

    impl LogReader {
        fn returning(text: impl Into<String>) -> Self {
            Self {
                text: text.into(),
                truncated: false,
                refusal: None,
                seen: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn refusing(refusal: OperationError) -> Self {
            Self {
                refusal: Some(refusal),
                ..Self::returning(String::new())
            }
        }
    }

    #[async_trait]
    impl DeploymentReader for Arc<LogReader> {
        async fn read(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<DeploymentStatus, OperationError> {
            Err(unavailable("deployment status is not part of this test"))
        }

        async fn pod_logs(
            &self,
            namespace: &str,
            pod: &str,
            container: Option<&str>,
            tail_lines: u32,
            since_seconds: Option<u32>,
        ) -> Result<PodLogs, OperationError> {
            self.seen.lock().unwrap().push((
                namespace.to_owned(),
                pod.to_owned(),
                container.map(ToOwned::to_owned),
                tail_lines,
                since_seconds,
            ));
            if let Some(refusal) = &self.refusal {
                return Err(refusal.clone());
            }
            Ok(PodLogs {
                namespace: namespace.to_owned(),
                pod: pod.to_owned(),
                container: container.map(ToOwned::to_owned),
                tail_lines,
                text: self.text.clone(),
                truncated: self.truncated,
            })
        }
    }

    fn log_backend(reader: &Arc<LogReader>) -> KubernetesStatusBackend {
        KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Arc::clone(reader)),
        )
        .unwrap()
    }

    async fn logs_lease(backend: &KubernetesStatusBackend, context: &PrincipalContext) -> String {
        let OperationResult::Describe(description) = backend
            .handle(
                context,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: LOGS_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("pod log description expected");
        };
        description.description_ref
    }

    fn logs_invoke(description_ref: String, input: serde_json::Value) -> OperationRequest {
        OperationRequest::Invoke(InvokeRequest {
            operation_ref: LOGS_OPERATION.to_owned(),
            connection_ref: CONNECTION.to_owned(),
            description_ref,
            input,
            approval_evidence_ref: None,
        })
    }

    fn owner(tenant: &str) -> PrincipalContext {
        owner_with_groups(tenant, ["dev", "sre"])
    }

    pub(super) fn owner_with_groups<const N: usize>(tenant: &str, groups: [&str; N]) -> PrincipalContext {
        PrincipalContext::hosted_with_groups(
            tenant.to_owned(),
            "person:owner".to_owned(),
            "agent-dev".to_owned(),
            None,
            "snapshot-dev".to_owned(),
            "a".repeat(64),
            groups.into_iter().map(ToOwned::to_owned).collect(),
        )
        .unwrap()
    }

    pub(super) fn policy() -> Vec<KubernetesNamespaceAccessConfig> {
        vec![KubernetesNamespaceAccessConfig {
            namespace: "b10x".to_owned(),
            read_groups: vec!["dev".to_owned(), "sre".to_owned()],
            restart_groups: vec!["sre".to_owned()],
        }]
    }

    #[tokio::test]
    async fn search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let search = |query: &str| {
            OperationRequest::Search(protocol::operation::SearchRequest {
                query: query.to_owned(),
                limit: 16,
            })
        };
        let reader_principal = owner_with_groups("tenant-dev", ["dev"]);
        let OperationResult::Search { operations } = backend
            .handle(&reader_principal, search("logs"))
            .await
            .unwrap()
        else {
            panic!("search result expected");
        };
        assert!(
            operations
                .iter()
                .any(|operation| operation.operation_ref == "kubernetes.pod.logs"),
            "{operations:?}"
        );

        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let OperationResult::Search { operations } = backend
            .handle(&outsider, search("logs"))
            .await
            .unwrap()
        else {
            panic!("search result expected");
        };
        assert!(
            operations
                .iter()
                .all(|operation| operation.operation_ref != "kubernetes.pod.logs"),
            "{operations:?}"
        );
    }

    #[tokio::test]
    async fn pod_log_description_carries_schemas_and_a_lease_for_read_principals_only() {
        let reader = Arc::new(LogReader::returning("ready\n"));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let OperationResult::Describe(description) = backend
            .handle(
                &context,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: LOGS_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("pod log description expected");
        };
        assert_eq!(description.operation_ref, LOGS_OPERATION);
        assert_eq!(description.effect, EffectClass::ReadOnly);
        assert_eq!(description.approval, ApprovalPosture::NotRequired);
        assert_eq!(
            description.input_schema["required"],
            json!(["namespace", "pod"])
        );
        assert_eq!(
            description.input_schema["properties"]["tail_lines"]["maximum"],
            json!(1000)
        );
        assert_eq!(
            description.input_schema["properties"]["since_seconds"]["maximum"],
            json!(86400)
        );
        assert_eq!(
            description.output_schema["required"],
            json!(["namespace", "pod", "tail_lines", "text", "truncated"])
        );
        assert_eq!(
            description.description_ref,
            description_ref(&context, LOGS_OPERATION)
        );

        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let refused = backend
            .handle(
                &outsider,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: LOGS_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);
    }

    #[tokio::test]
    async fn pod_log_invoke_passes_the_input_through_and_defaults_tail_lines() {
        let reader = Arc::new(LogReader::returning("line-1\nline-2\n"));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = logs_lease(&backend, &context).await;
        let OperationResult::Invoke(result) = backend
            .handle(
                &context,
                logs_invoke(
                    lease.clone(),
                    json!({
                        "namespace": "b10x",
                        "pod": "backend-abc12",
                        "container": "app",
                        "tail_lines": 7,
                        "since_seconds": 3600
                    }),
                ),
            )
            .await
            .unwrap()
        else {
            panic!("pod log invocation expected");
        };
        assert_eq!(result.operation_ref, LOGS_OPERATION);
        assert_eq!(result.output["namespace"], "b10x");
        assert_eq!(result.output["pod"], "backend-abc12");
        assert_eq!(result.output["container"], "app");
        assert_eq!(result.output["tail_lines"], 7);
        assert_eq!(result.output["text"], "line-1\nline-2\n");
        assert_eq!(result.output["truncated"], false);

        backend
            .handle(
                &context,
                logs_invoke(
                    lease,
                    json!({"namespace": "b10x", "pod": "backend-abc12"}),
                ),
            )
            .await
            .unwrap();
        let seen = reader.seen.lock().unwrap();
        assert_eq!(
            *seen,
            vec![
                (
                    "b10x".to_owned(),
                    "backend-abc12".to_owned(),
                    Some("app".to_owned()),
                    7,
                    Some(3600),
                ),
                (
                    "b10x".to_owned(),
                    "backend-abc12".to_owned(),
                    None,
                    200,
                    None,
                ),
            ]
        );
    }

    #[tokio::test]
    async fn pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease() {
        let reader = Arc::new(LogReader::returning("ready\n"));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = logs_lease(&backend, &context).await;

        let refused = backend
            .handle(
                &context,
                logs_invoke(
                    lease,
                    json!({"namespace": "kube-system", "pod": "backend-abc12"}),
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);

        let stale = backend
            .handle(
                &context,
                logs_invoke(
                    "description:kubernetes:stale".to_owned(),
                    json!({"namespace": "b10x", "pod": "backend-abc12"}),
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(stale.code, OperationErrorCode::StaleAuthority);
        assert!(reader.seen.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn pod_log_invoke_enforces_the_input_caps_before_the_reader() {
        let reader = Arc::new(LogReader::returning("ready\n"));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = logs_lease(&backend, &context).await;
        for input in [
            json!({"namespace": "b10x", "pod": "backend-abc12", "tail_lines": 0}),
            json!({"namespace": "b10x", "pod": "backend-abc12", "tail_lines": 1001}),
            json!({"namespace": "b10x", "pod": "backend-abc12", "since_seconds": 0}),
            json!({"namespace": "b10x", "pod": "backend-abc12", "since_seconds": 86_401}),
            json!({"namespace": "b10x", "pod": "-not-a-label"}),
            json!({"namespace": "b10x", "pod": "backend-abc12", "container": "-bad"}),
            json!({"namespace": "b10x", "pod": "backend-abc12", "follow": true}),
        ] {
            let refused = backend
                .handle(&context, logs_invoke(lease.clone(), input.clone()))
                .await
                .unwrap_err();
            assert_eq!(refused.code, OperationErrorCode::InvalidInput, "{input}");
        }
        assert!(reader.seen.lock().unwrap().is_empty());
    }

    /// The in-cluster reader maps an upstream 401/403 to `not_granted` ("Kubernetes workload
    /// identity refused the log read"); the invoke path must surface that refusal untouched
    /// rather than reshaping it.
    #[tokio::test]
    async fn an_upstream_log_refusal_surfaces_as_not_granted() {
        let reader = Arc::new(LogReader::refusing(not_granted(
            "Kubernetes workload identity refused the log read",
        )));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = logs_lease(&backend, &context).await;
        let refused = backend
            .handle(
                &context,
                logs_invoke(
                    lease,
                    json!({"namespace": "b10x", "pod": "backend-abc12"}),
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);
        assert!(refused.message.contains("refused the log read"));
    }

    /// An oversized log body is front-trimmed to whole lines after serialization, marked
    /// truncated, and the resulting envelope validates — a log read never surfaces
    /// `result_too_large`.
    #[tokio::test]
    async fn an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope() {
        let newest = 2_999;
        let text = (0..=newest)
            .map(|index| format!("line-{index:06} {}\n", "x".repeat(88)))
            .collect::<String>();
        assert!(text.len() > protocol::operation::MAX_RESULT_BYTES);
        let reader = Arc::new(LogReader::returning(text));
        let backend = log_backend(&reader);
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = logs_lease(&backend, &context).await;
        let result = backend
            .handle(
                &context,
                logs_invoke(
                    lease,
                    json!({"namespace": "b10x", "pod": "backend-abc12"}),
                ),
            )
            .await
            .unwrap();
        let OperationResult::Invoke(invocation) = &result else {
            panic!("pod log invocation expected");
        };
        assert_eq!(invocation.output["truncated"], true);
        let kept = invocation.output["text"].as_str().unwrap();
        // Whole oldest lines went; the text still starts at a line boundary and ends with the
        // newest line.
        assert!(kept.starts_with("line-"), "{:?}", &kept[..32]);
        assert!(!kept.contains("line-000000"));
        assert!(kept.ends_with(&format!("line-{newest:06} {}\n", "x".repeat(88))));
        protocol::operation::ResponseEnvelope::success("request-logs", result)
            .validate()
            .expect("a trimmed log envelope must validate");
    }

    #[tokio::test]
    async fn read_only_status_is_description_bound_and_namespace_scoped() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner("tenant-dev");
        let OperationResult::Describe(description) = backend
            .handle(
                &context,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: STATUS_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("description result expected");
        };
        let OperationResult::Invoke(result) = backend
            .handle(
                &context,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: STATUS_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref,
                    input: json!({"namespace": "b10x", "name": "backend"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("invoke result expected");
        };
        assert_eq!(result.output["running"], true);

        let refused = backend
            .handle(
                &context,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: STATUS_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description_ref(&context, STATUS_OPERATION),
                    input: json!({"namespace": "kube-system", "name": "backend"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);
    }

    #[tokio::test]
    async fn hosted_connection_projection_is_value_free_and_tenant_bound() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let ConnectionResult::Search { connections } = backend
            .handle_connection(
                &owner("tenant-dev"),
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("Connection search result expected");
        };
        assert_eq!(connections, vec![control_connection()]);
        assert!(backend
            .handle_connection(
                &owner("tenant-other"),
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn datasource_projects_only_safe_workload_fields_for_granted_namespaces() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let DatasourceResult::Describe(description) = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("datasource description expected");
        };
        let binding = namespace_binding(CONNECTION, "b10x");
        let DatasourceResult::Read(page) = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Read(ReadRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                    binding_ref: binding.binding_ref,
                    description_ref: description.description_ref,
                    read: DatasourceRead::Get {
                        key: json!({"name": "backend"}),
                    },
                }),
            )
            .await
            .unwrap()
        else {
            panic!("datasource read expected");
        };
        let encoded = serde_json::to_string(&page).unwrap();
        assert!(encoded.contains("image_id"));
        assert!(encoded.contains("BackOff"));
        for forbidden in [
            "secret",
            "environment",
            "annotations",
            "labels",
            "event_messages",
        ] {
            assert!(!encoded.contains(forbidden));
        }
    }

    fn owner_on_token<const N: usize>(
        groups: [&str; N],
        token_id: &str,
        envelope_sha: char,
    ) -> PrincipalContext {
        // The hosted receiver fills the snapshot id from the access-token id and the snapshot sha
        // from the introspection envelope, so both differ on the next token while the admitted
        // authority is identical.
        PrincipalContext::hosted_with_groups(
            "tenant-dev".to_owned(),
            "person:owner".to_owned(),
            "person:owner".to_owned(),
            None,
            token_id.to_owned(),
            envelope_sha.to_string().repeat(64),
            groups.into_iter().map(ToOwned::to_owned).collect(),
        )
        .unwrap()
    }

    /// Datasource requests all travel on the cached `connectors.catalog.read` token, which lives
    /// at most five minutes. A describe and the read that follows can therefore arrive on two
    /// different tokens, and the description lease must not treat that as an authority change.
    #[tokio::test]
    async fn a_workload_read_survives_the_access_token_rotation_between_describe_and_read() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let describing = owner_on_token(["dev"], "token-catalog-1", 'c');
        let DatasourceResult::Describe(description) = backend
            .handle_datasource(
                &describing,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("datasource description expected");
        };
        let reading = owner_on_token(["dev"], "token-catalog-2", 'd');
        let DatasourceResult::Read(page) = backend
            .handle_datasource(
                &reading,
                DatasourceRequest::Read(ReadRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                    binding_ref: namespace_binding(CONNECTION, "b10x").binding_ref,
                    description_ref: description.description_ref,
                    read: DatasourceRead::List {
                        limit: 5,
                        cursor: None,
                    },
                }),
            )
            .await
            .unwrap()
        else {
            panic!("a read on the next access token must not be refused as stale");
        };
        assert_eq!(page.records.len(), 1);
    }

    /// Describe travels on `connectors.catalog.read` and invoke on `connectors.invoke`, and the
    /// client holds one cached access token per scope, so these two calls never share a token.
    /// A lease that noticed would refuse every hosted invocation of these operations.
    #[tokio::test]
    async fn a_status_invocation_survives_the_scope_change_between_describe_and_invoke() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let describing = owner_on_token(["dev"], "token-catalog-1", 'c');
        let OperationResult::Describe(description) = backend
            .handle(
                &describing,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: STATUS_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("operation description expected");
        };
        let invoking = owner_on_token(["dev"], "token-invoke-2", 'd');
        backend
            .handle(
                &invoking,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: STATUS_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref,
                    input: json!({"namespace": "b10x", "name": "backend"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .expect("an invoke on the invoke-scoped token must not be refused as stale");
    }

    /// The deployed build answered a read whose binding ref was the datasource ref with
    /// `NotGranted: Kubernetes datasource binding is not granted`, and a reviewing agent took
    /// that to an operator as a missing grant. Describe had just succeeded, which this
    /// Integration only allows when at least one namespace is readable, so no grant was missing.
    #[tokio::test]
    async fn an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let refused = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Read(ReadRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                    binding_ref: DATASOURCE.to_owned(),
                    description_ref: datasource_description_ref(&context),
                    read: DatasourceRead::List {
                        limit: 5,
                        cursor: None,
                    },
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::InvalidInput);
        assert!(refused.message.contains(DATASOURCE), "{refused:?}");
        assert!(!refused.message.contains("not granted"), "{refused:?}");
    }

    /// Describing with no readable namespace fell through to "Kubernetes datasource was not
    /// found", which is the same lie in a different place: the datasource exists, the grant does
    /// not. The second deployed turn saw exactly this state — zero bindings, minutes after the
    /// first turn's describe succeeded — so it also has to read as recoverable.
    #[tokio::test]
    async fn describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner_with_groups("tenant-dev", ["unrelated"]);
        let refused = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::NotGranted);
        assert!(refused.message.contains("b10x"), "{refused:?}");
        assert!(refused.message.contains("dev"), "{refused:?}");
        assert!(refused.retriable, "{refused:?}");
        assert!(!refused.message.contains("not found"), "{refused:?}");
    }

    /// A grant that really is missing must name the namespace and the group that carries it, so a
    /// person knows what to ask for. The group facts behind it moved between two turns minutes
    /// apart against the same deployment, so the refusal is also retriable.
    #[tokio::test]
    async fn a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner_with_groups("tenant-dev", ["unrelated"]);
        let refused = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Read(ReadRequest {
                    datasource_ref: DATASOURCE.to_owned(),
                    binding_ref: namespace_binding(CONNECTION, "b10x").binding_ref,
                    description_ref: datasource_description_ref(&context),
                    read: DatasourceRead::List {
                        limit: 5,
                        cursor: None,
                    },
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::NotGranted);
        assert!(refused.message.contains("b10x"), "{refused:?}");
        assert!(refused.message.contains("dev"), "{refused:?}");
        assert!(refused.message.contains("sre"), "{refused:?}");
        assert!(refused.retriable, "{refused:?}");
    }

    #[tokio::test]
    async fn restart_requires_sre_group_and_exact_resource_authority_without_local_approval() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(Reader),
        )
        .unwrap();
        let dev = owner_with_groups("tenant-dev", ["dev"]);
        assert_eq!(
            backend
                .handle(
                    &dev,
                    OperationRequest::Describe(DescribeRequest {
                        operation_ref: RESTART_OPERATION.to_owned(),
                    }),
                )
                .await
                .unwrap_err()
                .code,
            OperationErrorCode::NotGranted
        );

        let sre = owner_with_groups("tenant-dev", ["sre"]);
        let OperationResult::Describe(description) = backend
            .handle(
                &sre,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: RESTART_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("restart description expected");
        };
        let input = json!({
            "namespace": "b10x",
            "name": "backend",
            "uid": "uid-backend",
            "resource_version": "42"
        });
        // No local approval ceremony: the description declares `ApprovalPosture::Required` and
        // the demanded approval is verified and spent upstream by the proof chain (S-047), so
        // the invocation carries no evidence reference and dispatches.
        let OperationResult::Invoke(accepted) = backend
            .handle(
                &sre,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: RESTART_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref,
                    input,
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("restart invocation expected");
        };
        assert_eq!(accepted.output["patch_accepted"], true);
        assert_eq!(accepted.output["resource_version"], "43");
    }

    #[test]
    fn deployment_projection_requires_observed_available_replicas() {
        let deployment: KubernetesDeployment = serde_json::from_value(json!({
            "metadata": {"namespace": "b10x", "name": "backend", "generation": 2},
            "spec": {"replicas": 1},
            "status": {
                "observedGeneration": 1,
                "readyReplicas": 1,
                "availableReplicas": 1,
                "updatedReplicas": 1,
                "conditions": [{"type": "Available", "status": "True"}]
            }
        }))
        .unwrap();
        assert!(!project(deployment).running);
    }
}
