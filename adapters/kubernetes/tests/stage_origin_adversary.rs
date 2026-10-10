//! Adversary: a refusal the adapter raises from its own configured scope,
//! before any provider request, is connectors configuration refusing, not the
//! provider. contracts/cli/v1alpha1/semantics.md section 6 reports "an
//! operation, profile or connection the configuration ... does not allow" at
//! `stage = admission`; the CLI reports `stage = dispatch` for every
//! `owner::Error` whose origin is `Provider`.
use connectors_core::Result;
use connectors_host::local::{
    owner::{Code, Error, Origin},
    runtime::Failure,
};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CountingHttp(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl AuthenticatedHttp for CountingHttp {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("a configured-scope refusal must not reach the provider");
    }
}

fn adapter() -> (Kubernetes, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["services".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec: false,
        kubeconfig_contexts: false,
    };
    let effective = json!({"service":{"instance":"scoped","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    let adapter = Kubernetes::new(
        "scoped",
        config,
        effective,
        Arc::new(CountingHttp(calls.clone())),
    )
    .unwrap();
    (adapter, calls)
}

/// The child's own path: `adapters/kubernetes/src/local.rs` maps an invoke
/// error with `Failure::from_provider`, and the owner converts the private
/// failure with `From<runtime::Failure> for owner::Error`.
async fn owner_error(operation: &str, input: Value) -> (Error, usize) {
    let (adapter, calls) = adapter();
    let failure = Failure::from_provider(adapter.invoke(operation, input).await.unwrap_err());
    (failure.into(), calls.load(Ordering::SeqCst))
}

#[tokio::test]
async fn a_namespace_outside_the_configured_scope_is_not_reported_as_a_provider_refusal() {
    let (error, calls) = owner_error(
        "resources.list",
        json!({"namespace":"kube-system","kind":"services","limit":10}),
    )
    .await;
    assert_eq!(calls, 0, "refused before provider work");
    assert_eq!(error.code, Code::Forbidden);
    assert_eq!(
        error.origin,
        Origin::Host,
        "configured-scope refusal would read stage=dispatch at the CLI"
    );
}

#[tokio::test]
async fn a_resource_kind_outside_the_configured_scope_is_not_reported_as_a_provider_refusal() {
    let (error, calls) = owner_error(
        "resources.list",
        json!({"namespace":"engineering","kind":"pods","limit":10}),
    )
    .await;
    assert_eq!(calls, 0, "refused before provider work");
    assert_eq!(error.code, Code::Forbidden);
    assert_eq!(error.origin, Origin::Host);
}

#[tokio::test]
async fn disabled_host_discovery_is_not_reported_as_a_provider_refusal() {
    let (error, calls) = owner_error("hosts.discover", json!({"limit":1})).await;
    assert_eq!(calls, 0, "refused before provider work");
    assert_eq!(error.code, Code::Forbidden);
    assert_eq!(error.origin, Origin::Host);
}
