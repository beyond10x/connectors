use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
};
use connectors_core::{ErrorCode, Result};
use connectors_host::{
    credentials::{BoundCredential, MemorySecrets},
    http::{HttpConfig, ScopedHttp},
    server::router,
};
use connectors_kubernetes::{Config, Kubernetes};
use connectors_sdk::{Adapter, AuthenticatedHttp, Credential, HttpResponse, Secret};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct ServiceCredential;
#[async_trait::async_trait]
impl Credential for ServiceCredential {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(b"client-token".to_vec()))
    }
}
async fn upstream(
    State(calls): State<Arc<AtomicUsize>>,
    uri: Uri,
    headers: HeaderMap,
) -> axum::response::Response {
    calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(headers["authorization"], "Bearer cluster-token");
    if uri.query().is_some_and(|q| q.contains("continue=")) {
        return StatusCode::GONE.into_response();
    }
    let items = if uri.path().ends_with("endpointslices") {
        assert_eq!(
            uri.path(),
            "/apis/discovery.k8s.io/v1/namespaces/engineering/endpointslices"
        );
        json!([{"metadata":{"uid":"slice-1","labels":{"kubernetes.io/service-name":"postgres"}},"ports":[{"port":5432,"protocol":"TCP"}],"endpoints":[{"addresses":["192.0.2.4"],"conditions":{"ready":false}}]}])
    } else if uri.path().ends_with("nodes") {
        assert_eq!(uri.path(), "/api/v1/nodes");
        json!([{"metadata":{"uid":"node-1","name":"worker","resourceVersion":"9"},"status":{"addresses":[{"type":"InternalIP","address":"192.0.2.1"}],"conditions":[]}}])
    } else {
        assert_eq!(uri.path(), "/api/v1/namespaces/engineering/services");
        json!([{"metadata":{"uid":"service-1","name":"postgres"}}])
    };
    axum::Json(json!({"metadata":{"resourceVersion":"10","continue":"next-chunk"},"items":items}))
        .into_response()
}

#[tokio::test]
async fn discovery_is_attributed_paged_and_does_not_activate_observed_endpoints() {
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(upstream).with_state(calls.clone());
    let provider = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let http = HttpConfig {
        base_url: format!("http://{address}/"),
        credential: None,
        credential_header: "authorization".into(),
        bearer: true,
        allow_plaintext: true,
        ca_file: None,
    };
    let credential = Arc::new(BoundCredential {
        store: Arc::new(MemorySecrets(BTreeMap::from([(
            "cluster".into(),
            b"cluster-token".to_vec(),
        )]))),
        reference: "cluster".into(),
    });
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["services".into()],
        discover_hosts: true,
    };
    let effective = json!({"service":{"instance":"cluster-test","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED_FIXTURE_BINDING"}},"http":http,"adapter":config});
    let adapter = Kubernetes::new(
        "cluster-test",
        config,
        effective,
        Arc::new(ScopedHttp::new(&http, Some(credential)).unwrap()),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let service = tokio::spawn(async move {
        axum::serve(
            listener,
            router(Arc::new(adapter), Arc::new(ServiceCredential)),
        )
        .await
        .unwrap();
    });
    let client = connectors_client::Client::new(&endpoint, "client-token".into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    assert_eq!(
        client
            .invoke(
                &descriptor,
                "endpoints.discover",
                json!({"namespace":"other","limit":1})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(
        client
            .invoke(
                &descriptor,
                "resources.list",
                json!({"namespace":"engineering","kind":"pods","limit":1})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let endpoints = client
        .invoke(
            &descriptor,
            "endpoints.discover",
            json!({"namespace":"engineering","limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(endpoints["items"][0]["address"], "192.0.2.4");
    assert_eq!(endpoints["items"][0]["ready"], false);
    assert_eq!(endpoints["items"][0]["service"], "postgres");
    assert_eq!(endpoints["provenance"]["source_revision"], "10");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let next = endpoints["next_cursor"].as_str().unwrap();
    assert_eq!(
        client
            .invoke(
                &descriptor,
                "endpoints.discover",
                json!({"namespace":"engineering","limit":1,"cursor":next})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    let nodes = client
        .invoke(&descriptor, "hosts.discover", json!({"limit":1}))
        .await
        .unwrap();
    assert_eq!(nodes["items"][0]["id"], "node-1");
    let resources = client
        .invoke(
            &descriptor,
            "resources.list",
            json!({"namespace":"engineering","kind":"services","limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(resources["items"][0]["metadata"]["name"], "postgres");
    service.abort();
    provider.abort();
}

struct ExpansionFixture {
    calls: Arc<AtomicUsize>,
    addresses: usize,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for ExpansionFixture {
    async fn get(&self, segments: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(segments.last(), Some(&"endpointslices"));
        let addresses = (0..self.addresses)
            .map(|i| format!("192.0.2.{i}"))
            .collect::<Vec<_>>();
        let ports = (1..=64)
            .map(|port| json!({"port":port,"protocol":"TCP"}))
            .collect::<Vec<_>>();
        Ok(HttpResponse { status: 200, headers: BTreeMap::new(), body: serde_json::to_vec(&json!({
            "metadata":{"resourceVersion":"1"},
            "items":[{"metadata":{"uid":"slice"},"ports":ports,"endpoints":[{"addresses":addresses}]}]
        })).unwrap() })
    }
}
fn expansion_adapter(addresses: usize) -> (Kubernetes, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["services".into()],
        discover_hosts: false,
    };
    let effective = json!({"service":{"instance":"bounded","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    let adapter = Kubernetes::new(
        "bounded",
        config,
        effective,
        Arc::new(ExpansionFixture {
            calls: calls.clone(),
            addresses,
        }),
    )
    .unwrap();
    (adapter, calls)
}
#[tokio::test]
async fn disabled_host_discovery_is_not_advertised_or_dispatched() {
    let (adapter, calls) = expansion_adapter(1);
    assert!(adapter.descriptor().operation("hosts.discover").is_err());
    assert_eq!(
        adapter
            .invoke("hosts.discover", json!({"limit":1}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn endpoint_cross_product_is_bounded_even_for_one_source_slice() {
    let (adapter, calls) = expansion_adapter(64);
    let result = adapter
        .invoke(
            "endpoints.discover",
            json!({"namespace":"engineering","limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 4096);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let (adapter, calls) = expansion_adapter(65);
    assert_eq!(
        adapter
            .invoke(
                "endpoints.discover",
                json!({"namespace":"engineering","limit":1})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Capacity
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
