//! `contexts.list` (profile `kubernetes-kubeconfig-contexts`) over a recorded
//! kubeconfig carrying every kind of credential a real one does. The
//! projection must name contexts and never carry any of them.
use connectors_core::{ErrorCode, Result};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes, kubeconfig};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

/// Fictional fixture material only. None of it may appear in a result.
const TOKEN: &str = "fixture-kubeconfig-bearer-token";
const CA_DATA: &str = "Rml4dHVyZUNlcnRpZmljYXRlQXV0aG9yaXR5";
const KEY_DATA: &str = "Rml4dHVyZUNsaWVudEtleQ";
const SERVER: &str = "https://staging.cluster.invalid:6443";

fn recorded_kubeconfig() -> String {
    format!(
        r#"apiVersion: v1
kind: Config
preferences: {{}}
current-context: staging
clusters:
- name: staging-cluster
  cluster:
    server: {SERVER}
    certificate-authority-data: {CA_DATA}
- name: production-cluster
  cluster:
    server: https://production.cluster.invalid
    insecure-skip-tls-verify: true
contexts:
- name: staging
  context:
    cluster: staging-cluster
    user: staging-admin
    namespace: engineering
- name: production
  context:
    cluster: production-cluster
    user: production-reader
users:
- name: staging-admin
  user:
    token: {TOKEN}
- name: production-reader
  user:
    client-key-data: {KEY_DATA}
    exec:
      apiVersion: client.authentication.k8s.io/v1
      command: fixture-credential-helper
      args: ["--token", "{TOKEN}"]
"#
    )
}

struct Reads(Mutex<usize>);
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        *self.0.lock().unwrap() += 1;
        Ok(HttpResponse {
            status: 500,
            headers: BTreeMap::new(),
            body: Vec::new(),
        })
    }
}

fn kubernetes(kubeconfig_contexts: bool) -> (Kubernetes, Arc<Reads>) {
    let reads = Arc::new(Reads(Mutex::new(0)));
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec: false,
        kubeconfig_contexts,
    };
    let mut effective = json!({"format":"connectors-kubernetes-local/1","instance":"recorded",
        "api_base":"https://fixture.invalid/","ca_digest":null,"namespaces":["engineering"],
        "resource_kinds":["pods"],"discover_hosts":false,"helm_release_reads":"off"});
    if kubeconfig_contexts {
        effective["kubeconfig_path_digest"] = json!("a".repeat(64));
    }
    let adapter = Kubernetes::new("recorded", config, effective, reads.clone()).unwrap();
    (adapter, reads)
}

#[tokio::test]
async fn contexts_are_listed_without_credentials() {
    let (adapter, reads) = kubernetes(true);
    assert!(
        adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "contexts.list")
    );
    let page = adapter
        .contexts_page(recorded_kubeconfig().as_bytes(), json!({"limit":10}))
        .unwrap();
    assert_eq!(
        page["items"],
        json!([
            {"name":"staging","cluster":"staging-cluster","namespace":"engineering","current":true},
            {"name":"production","cluster":"production-cluster","namespace":null,"current":false},
        ])
    );
    assert_eq!(page["complete"], true);
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["provenance"]["resource"], "kubeconfig/contexts");
    assert_eq!(page["provenance"]["source_revision"], Value::Null);
    // No credential, certificate, server or user name leaves the projection.
    let disclosed = page.to_string();
    for secret in [
        TOKEN,
        CA_DATA,
        KEY_DATA,
        SERVER,
        "production.cluster.invalid",
        "staging-admin",
        "production-reader",
        "fixture-credential-helper",
    ] {
        assert!(!disclosed.contains(secret), "{secret} disclosed");
    }
    // Listing contexts is no provider request.
    assert_eq!(*reads.0.lock().unwrap(), 0);
}

#[tokio::test]
async fn a_smaller_limit_lists_the_first_contexts_and_says_it_is_partial() {
    let (adapter, _) = kubernetes(true);
    let page = adapter
        .contexts_page(recorded_kubeconfig().as_bytes(), json!({"limit":1}))
        .unwrap();
    assert_eq!(page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(page["items"][0]["name"], "staging");
    assert_eq!(page["complete"], false);
    assert_eq!(page["next_cursor"], Value::Null);
    for limit in [0, 257] {
        assert_eq!(
            adapter
                .contexts_page(recorded_kubeconfig().as_bytes(), json!({"limit":limit}))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}

#[test]
fn a_current_context_naming_no_listed_context_marks_none_current() {
    let file = recorded_kubeconfig().replace("current-context: staging", "current-context: gone");
    let contexts = kubeconfig::contexts(file.as_bytes()).unwrap();
    assert!(contexts.iter().all(|context| !context.current));
    let file = recorded_kubeconfig().replace("current-context: staging\n", "");
    let contexts = kubeconfig::contexts(file.as_bytes()).unwrap();
    assert!(contexts.iter().all(|context| !context.current));
}

#[test]
fn an_ambiguous_oversized_or_foreign_file_refuses_as_a_whole() {
    // A repeated name would make "the context named X" ambiguous.
    let duplicated = recorded_kubeconfig().replace("- name: production\n", "- name: staging\n");
    assert_eq!(
        kubeconfig::contexts(duplicated.as_bytes())
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    let mut many = String::from("contexts:\n");
    for index in 0..=kubeconfig::MAX_CONTEXTS {
        many.push_str(&format!("- name: c{index}\n  context:\n    cluster: k\n"));
    }
    assert!(kubeconfig::contexts(many.as_bytes()).is_err());
    assert!(kubeconfig::contexts(b"\x00\xff not yaml").is_err());
    assert!(kubeconfig::contexts(b"contexts:\n- name: c\n  context: {}\n").is_err());
    assert!(kubeconfig::contexts(" ".repeat(kubeconfig::MAX_FILE_BYTES + 1).as_bytes()).is_err());
    // A file with no contexts lists none.
    assert_eq!(
        kubeconfig::contexts(b"apiVersion: v1\nkind: Config\n").unwrap(),
        vec![]
    );
}

#[tokio::test]
async fn contexts_are_neither_advertised_nor_read_without_a_configured_kubeconfig() {
    let (adapter, reads) = kubernetes(false);
    assert!(
        !adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "contexts.list")
    );
    assert_eq!(
        adapter
            .contexts_page(recorded_kubeconfig().as_bytes(), json!({"limit":10}))
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    // The business adapter holds no file: through its own invoke path it
    // refuses even when configured, and makes no request.
    let (adapter, _) = kubernetes(true);
    assert_eq!(
        adapter
            .invoke("contexts.list", json!({"limit":10}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(*reads.0.lock().unwrap(), 0);
}
