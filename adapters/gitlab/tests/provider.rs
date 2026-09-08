use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
};
use connectors_core::{ErrorCode, Result};
use connectors_gitlab::{Config, GitLab};
use connectors_host::{
    credentials::{BoundCredential, MemorySecrets},
    http::{HttpConfig, ScopedHttp},
    server::router,
};
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
    if headers.get("private-token").and_then(|v| v.to_str().ok()) != Some("provider-token") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    assert!(uri.path().starts_with("/api/v4/projects/org%2Fproject"));
    if uri.path().ends_with("/issues") {
        assert!(uri.query().unwrap().contains("order_by=created_at"));
        if uri.query().unwrap().contains("page=2") {
            return ([("x-next-page", "")], axum::Json(json!([]))).into_response();
        }
        return (
            [("x-next-page", "2")],
            axum::Json(json!([{"id":1,"iid":1,"title":"example"}])),
        )
            .into_response();
    }
    if uri.path().contains("/repository/files/") {
        assert!(uri.path().ends_with("README%2Emd") || uri.path().ends_with("README.md"));
        return axum::Json(
            json!({"encoding":"base64","content":"aGVsbG8=","commit_id":"fixture-revision"}),
        )
        .into_response();
    }
    axum::Json(json!({"id":42,"path_with_namespace":"org/project"})).into_response()
}

#[tokio::test]
async fn project_issue_pagination_and_file_reads_use_scoped_auth_over_http() {
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(upstream).with_state(calls.clone());
    let provider = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let http = HttpConfig {
        base_url: format!("http://{address}/api/v4/"),
        credential: None,
        credential_header: "private-token".into(),
        bearer: false,
        allow_plaintext: true,
        ca_file: None,
    };
    let credential = Arc::new(BoundCredential {
        store: Arc::new(MemorySecrets(BTreeMap::from([(
            "gitlab".into(),
            b"provider-token".to_vec(),
        )]))),
        reference: "gitlab".into(),
    });
    let config = Config {
        allowed_projects: vec!["org/project".into()],
    };
    let effective = json!({"service":{"instance":"gitlab-test","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED_FIXTURE_BINDING"}},"http":http,"adapter":config});
    let adapter = GitLab::new(
        "gitlab-test",
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
            .invoke(&descriptor, "project.get", json!({"project":"elsewhere"}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        client
            .invoke(&descriptor, "project.get", json!({"project":"org/project"}))
            .await
            .unwrap()["item"]["id"],
        42
    );
    let first = client
        .invoke(
            &descriptor,
            "issues.list",
            json!({"project":"org/project","limit":1}),
        )
        .await
        .unwrap();
    assert_eq!(first["complete"], false);
    let token = first["next_cursor"].as_str().unwrap();
    assert_eq!(
        client
            .invoke(
                &descriptor,
                "issues.list",
                json!({"project":"org/project","limit":2,"cursor":token})
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    let last = client
        .invoke(
            &descriptor,
            "issues.list",
            json!({"project":"org/project","limit":1,"cursor":token}),
        )
        .await
        .unwrap();
    assert_eq!(last["complete"], true);
    assert_eq!(last["items"], json!([]));
    let file = client
        .invoke(
            &descriptor,
            "file.get",
            json!({"project":"org/project","path":"README.md","ref":"main"}),
        )
        .await
        .unwrap();
    assert_eq!(file["item"]["content"], "aGVsbG8=");
    assert_eq!(file["provenance"]["source_revision"], "fixture-revision");
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    service.abort();
    provider.abort();
}
