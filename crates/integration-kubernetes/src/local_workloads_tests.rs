#[cfg(test)]
mod tests {
    use super::*;
    use crate::workloads::{datasource_projection_sha256, KubernetesList};

    const DEPLOYMENT_LIST: &str = r#"{
      "metadata": {"continue": "next-page"},
      "items": [{
        "metadata": {"name": "zwirn", "namespace": "b10x", "uid": "u-1",
                     "resourceVersion": "42", "generation": 7},
        "spec": {"replicas": 3, "selector": {"matchLabels": {"app": "zwirn"}}},
        "status": {"observedGeneration": 7, "readyReplicas": 3, "availableReplicas": 3,
                   "updatedReplicas": 3, "unavailableReplicas": 0,
                   "conditions": [{"type": "Available", "status": "True"}]}
      }]
    }"#;

    #[test]
    fn the_local_placement_publishes_the_deployments_projection_verbatim() {
        // The whole point of `crate::workloads`: one datasource ref, one schema, one digest, so a
        // workbench pointed at a workstation and one pointed at the cluster cannot be showing
        // different records under the same name. A second copy of the projection would let the two
        // drift silently — every existing check would stay green while the products diverged.
        assert_eq!(datasource_summary().datasource_ref, DATASOURCE);
        assert_eq!(datasource_projection_sha256().len(), 64);
        assert_eq!(
            crate::workloads::datasource_summary(),
            datasource_summary(),
            "the local placement must not redefine what `{DATASOURCE}` is"
        );
    }

    #[test]
    fn a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns() {
        // The wire types and the projection are shared, so this asserts the kubeconfig path parses
        // a real API response into the record shape the browser reads — the step that was missing
        // while the local placement declared `datasources: false`.
        let list: KubernetesList<crate::workloads::KubernetesDeployment> =
            serde_json::from_str(DEPLOYMENT_LIST).unwrap();
        assert_eq!(list.metadata.continue_token, "next-page");
        let (workload, meta) = project_workload(list.items.into_iter().next().unwrap());
        assert_eq!(workload.name, "zwirn");
        assert_eq!(workload.desired_replicas, 3);
        assert_eq!(workload.ready_replicas, 3);
        assert_eq!(workload.rollout_state, "available");
        // Object identity stays with the detail record, never with the listing (S-063).
        assert_eq!(meta.namespace, "b10x");
        assert_eq!(meta.uid, "u-1");
        assert_eq!(meta.resource_version, "42");
        // The list record is the minimal projection and nothing else — no identity metadata,
        // and nothing a Secret or an annotation could hide in.
        let record = serde_json::to_value(&workload).unwrap();
        let mut keys = record
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            ["desired_replicas", "name", "ready_replicas", "rollout_state"]
        );
    }

    #[test]
    fn a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake() {
        let namespaces = vec!["b10x".to_owned()];
        let refusal = WorkloadSurface::binding_namespace(&namespaces, "binding:kubernetes:nonsense")
            .unwrap_err();
        assert_eq!(refusal.code, DatasourceErrorCode::InvalidInput);
        assert!(refusal.message.contains("list its bindings"));
        assert_eq!(
            WorkloadSurface::binding_namespace(&namespaces, &namespace_binding_ref("b10x"))
                .unwrap(),
            "b10x"
        );
    }

    #[test]
    fn a_name_that_is_not_a_dns_label_never_reaches_a_request_path() {
        // `../../secrets` in a datasource key would address a different resource than the binding
        // admitted. The refusal is InvalidInput rather than NotGranted: nothing about the caller's
        // grant is wrong, the key is.
        assert!(path_segment("../../secrets").is_err());
        assert!(path_segment("b10x/zwirn").is_err());
        assert!(path_segment("zwirn").is_ok());
    }

    #[test]
    fn query_values_are_encoded_rather_than_interpolated() {
        let encoded = query(&[("labelSelector", "app=zwirn,tier=web"), ("limit", "51")]);
        assert_eq!(
            encoded,
            "labelSelector=app%3Dzwirn%2Ctier%3Dweb&limit=51"
        );
    }
}
