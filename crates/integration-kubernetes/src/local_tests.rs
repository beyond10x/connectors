#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_candidates_expose_only_context_label_and_opaque_evidence() {
        let kubeconfig = Kubeconfig::from_yaml(
            r#"
apiVersion: v1
kind: Config
clusters:
- name: dev
  cluster:
    server: https://10.0.0.1:6443
contexts:
- name: dev-cluster
  context:
    cluster: dev
    user: alice
users:
- name: alice
  user:
    token: secret-token
"#,
        )
        .unwrap();
        let candidates = candidates(&kubeconfig);
        let candidate = candidates.values().next().unwrap();
        let encoded = serde_json::to_string(&candidate.summary).unwrap();
        assert_eq!(candidate.summary.title, "dev-cluster");
        assert!(!encoded.contains("secret-token"));
        assert!(!encoded.contains("10.0.0.1"));
        assert!(!encoded.contains("alice"));
    }

    #[test]
    fn monitoring_service_recognition_is_curated() {
        let service = |name: &str| Service {
            metadata: kube::core::ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some("monitoring".to_owned()),
                ..Default::default()
            },
            ..Service::default()
        };
        assert_eq!(
            recognize_service(&service("infra-grafana")),
            Some("grafana")
        );
        assert_eq!(
            recognize_service(&service("kube-prometheus")),
            Some("prometheus")
        );
        assert_eq!(recognize_service(&service("postgres")), None);

        let unrelated = Service {
            metadata: kube::core::ObjectMeta {
                name: Some("database".to_owned()),
                namespace: Some("monitoring".to_owned()),
                labels: Some(BTreeMap::from([
                    ("app.kubernetes.io/name".to_owned(), "postgres".to_owned()),
                    ("managed-by".to_owned(), "grafana-operator".to_owned()),
                ])),
                ..Default::default()
            },
            ..Service::default()
        };
        assert_eq!(recognize_service(&unrelated), None);
    }

    #[test]
    fn service_observation_pins_uid_and_one_closed_tcp_port() {
        let service = Service {
            metadata: kube::core::ObjectMeta {
                name: Some("prometheus".to_owned()),
                namespace: Some("monitoring".to_owned()),
                uid: Some("uid-prometheus-1".to_owned()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                ports: Some(vec![
                    ServicePort {
                        name: Some("metrics".to_owned()),
                        port: 9090,
                        protocol: Some("TCP".to_owned()),
                        ..ServicePort::default()
                    },
                    ServicePort {
                        name: Some("udp".to_owned()),
                        port: 9090,
                        protocol: Some("UDP".to_owned()),
                        ..ServicePort::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Service::default()
        };

        let observations = normalize_services("connection:kubernetes:dev", vec![service]);
        let [observation] = observations.as_slice() else {
            panic!("one supported Service must be normalized");
        };
        assert_eq!(observation.provider, "prometheus");
        assert_eq!(observation.resource_uid, "uid-prometheus-1");
        assert_eq!(observation.port, "metrics");
        assert!(!observation.resource_binding.contains("uid-prometheus-1"));
    }

    #[test]
    fn insecure_api_server_contexts_are_not_candidates() {
        let kubeconfig = Kubeconfig::from_yaml(
            r#"
apiVersion: v1
kind: Config
clusters:
- name: dev
  cluster:
    server: http://cluster.example
contexts:
- name: dev-cluster
  context:
    cluster: dev
"#,
        )
        .unwrap();
        assert!(candidates(&kubeconfig).is_empty());
    }
}
