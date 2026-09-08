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
use connectors_sdk::{Credential, Secret};
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
