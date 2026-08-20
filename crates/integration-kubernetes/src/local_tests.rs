#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::ServicePort;

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

    /// The eight Services a default Argo CD install ships, as they actually appear in
    /// `manifests/install.yaml` at v3.5.1 — name plus `app.kubernetes.io/name`. Only `argocd-server`
    /// is the API; the other seven are what a substring test would have swept up with it.
    #[test]
    fn argocd_recognition_takes_the_api_service_and_none_of_its_siblings() {
        let installed = |name: &str| Service {
            metadata: kube::core::ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some("argocd".to_owned()),
                labels: Some(BTreeMap::from([
                    ("app.kubernetes.io/name".to_owned(), name.to_owned()),
                    ("app.kubernetes.io/part-of".to_owned(), "argocd".to_owned()),
                ])),
                ..Default::default()
            },
            ..Service::default()
        };

        assert_eq!(recognize_service(&installed("argocd-server")), Some("argocd"));
        for sibling in [
            "argocd-repo-server",
            "argocd-server-metrics",
            "argocd-redis",
            "argocd-dex-server",
            "argocd-metrics",
            "argocd-applicationset-controller",
            "argocd-notifications-controller-metrics",
        ] {
            assert_eq!(
                recognize_service(&installed(sibling)),
                None,
                "{sibling} is not the Argo CD API"
            );
        }
    }

    /// The `argo-cd` Helm chart prefixes the Service with the release name and keeps
    /// `app.kubernetes.io/name: argocd-server`, so the label is the stable identity here and the
    /// object's name is not.
    #[test]
    fn a_renamed_argocd_release_is_recognized_by_its_identity_label() {
        let renamed = Service {
            metadata: kube::core::ObjectMeta {
                name: Some("platform-argocd-server".to_owned()),
                namespace: Some("gitops".to_owned()),
                labels: Some(BTreeMap::from([(
                    "app.kubernetes.io/name".to_owned(),
                    "argocd-server".to_owned(),
                )])),
                ..Default::default()
            },
            ..Service::default()
        };
        assert_eq!(recognize_service(&renamed), Some("argocd"));
    }

    /// `argocd-server` publishes `http` on 80 and `https` on 443, both targeting container port
    /// 8080. The pin is 443, because that is the listener Argo CD's own clients use.
    #[test]
    fn the_argocd_observation_pins_the_api_port_rather_than_the_redirect() {
        let service = Service {
            metadata: kube::core::ObjectMeta {
                name: Some("argocd-server".to_owned()),
                namespace: Some("argocd".to_owned()),
                uid: Some("uid-argocd-server-1".to_owned()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::core::v1::ServiceSpec {
                ports: Some(vec![
                    ServicePort {
                        name: Some("http".to_owned()),
                        port: 80,
                        protocol: Some("TCP".to_owned()),
                        ..ServicePort::default()
                    },
                    ServicePort {
                        name: Some("https".to_owned()),
                        port: 443,
                        protocol: Some("TCP".to_owned()),
                        ..ServicePort::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Service::default()
        };

        let observations = normalize_services("connection:kubernetes:dev", vec![service]);
        let [observation] = observations.as_slice() else {
            panic!("the Argo CD API Service must be normalized");
        };
        assert_eq!(observation.provider, "argocd");
        assert_eq!(observation.port, "https");
    }

    /// **Observation is not a Connection here.** A mediated call carries no credential, and Argo CD
    /// answers 401 without one, so this placement may see the Service and must not offer it as
    /// something callable. Grafana was already in that position; this pins that Argo CD joined it,
    /// and that the three unauthenticated monitoring backends did not.
    #[test]
    fn providers_that_need_a_credential_are_not_materializable_here() {
        assert!(credential_bearing_provider("argocd"));
        assert!(credential_bearing_provider("grafana"));
        for unauthenticated in ["prometheus", "loki", "alertmanager"] {
            assert!(!credential_bearing_provider(unauthenticated));
        }
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
