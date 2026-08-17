#[cfg(test)]
mod tests {
    use super::*;

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
            namespace: &str,
            _limit: u16,
            _cursor: Option<&str>,
        ) -> Result<WorkloadList, DatasourceError> {
            Ok(WorkloadList {
                workloads: vec![WorkloadCompact {
                    namespace: namespace.to_owned(),
                    name: "backend".to_owned(),
                    uid: "uid-backend".to_owned(),
                    resource_version: "42".to_owned(),
                    generation: 3,
                    observed_generation: 3,
                    desired_replicas: 2,
                    updated_replicas: 2,
                    ready_replicas: 2,
                    available_replicas: 2,
                    unavailable_replicas: 0,
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

    fn owner(tenant: &str) -> PrincipalContext {
        owner_with_groups(tenant, ["dev", "sre"])
    }

    fn owner_with_groups<const N: usize>(tenant: &str, groups: [&str; N]) -> PrincipalContext {
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

    fn policy() -> Vec<KubernetesNamespaceAccessConfig> {
        vec![KubernetesNamespaceAccessConfig {
            namespace: "b10x".to_owned(),
            read_groups: vec!["dev".to_owned(), "sre".to_owned()],
            restart_groups: vec!["sre".to_owned()],
        }]
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
        let binding = namespace_binding("b10x");
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

    #[tokio::test]
    async fn restart_requires_sre_group_fresh_approval_and_exact_resource_authority() {
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
        let missing_approval = backend
            .handle(
                &sre,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: RESTART_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref.clone(),
                    input: input.clone(),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(missing_approval.code, OperationErrorCode::ApprovalRequired);

        let OperationResult::Invoke(accepted) = backend
            .handle(
                &sre,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: RESTART_OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref,
                    input,
                    approval_evidence_ref: Some("approval:test:one".to_owned()),
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
