use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, Credential, Secret, upstream_json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixed;
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(b"private-token-value".to_vec()))
    }
}

#[tokio::test]
async fn upstream_redirects_cannot_exfiltrate_bound_credentials() {
    let hits = Arc::new(AtomicUsize::new(0));
    let observed = hits.clone();
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_address = target.local_addr().unwrap();
    let receiver = tokio::spawn(async move {
        axum::serve(
            target,
            Router::new().route(
                "/secret",
                get(move || {
                    let hits = observed.clone();
                    async move {
                        hits.fetch_add(1, Ordering::SeqCst);
                        StatusCode::OK
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let source = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let source_address = source.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            source,
            Router::new().route(
                "/redirect",
                get(move |headers: HeaderMap| async move {
                    assert_eq!(headers["x-fixture-credential"], "private-token-value");
                    (
                        StatusCode::FOUND,
                        [("location", format!("http://{target_address}/secret"))],
                    )
                }),
            ),
        )
        .await
        .unwrap();
    });
    let config = HttpConfig {
        base_url: format!("http://{source_address}/"),
        credential: None,
        credential_header: "x-fixture-credential".into(),
        bearer: false,
        allow_plaintext: true,
        ca_file: None,
    };
    let http = ScopedHttp::new(&config, Some(Arc::new(Fixed))).unwrap();
    let response = http.get(&["redirect"], &[]).await.unwrap();
    assert_eq!(response.status, 302);
    assert_eq!(
        upstream_json(&response).unwrap_err().code,
        ErrorCode::UpstreamProtocol
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    server.abort();
    receiver.abort();
}

#[tokio::test]
async fn error_bodies_are_not_exposed_and_large_responses_are_bounded() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route(
            "/rate",
            get(|| async {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("retry-after", "30")],
                    "private-token-value",
                )
            }),
        )
        .route(
            "/large",
            get(|| async { vec![b'x'; connectors_core::RESPONSE_LIMIT + 1] }),
        )
        .route("/ambiguous", get(|| async { r#"{"id":1,"id":2}"# }));
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let config = HttpConfig {
        base_url: format!("http://{address}/"),
        credential: None,
        credential_header: "authorization".into(),
        bearer: true,
        allow_plaintext: true,
        ca_file: None,
    };
    let http = ScopedHttp::from_config(&config).unwrap();
    let error = upstream_json(&http.get(&["rate"], &[]).await.unwrap()).unwrap_err();
    assert_eq!(error.code, ErrorCode::RateLimited);
    assert_eq!(error.retry_after_seconds, Some(30));
    assert!(!error.to_string().contains("private-token-value"));
    assert_eq!(
        http.get(&["large"], &[]).await.err().unwrap().code,
        ErrorCode::Capacity
    );
    assert_eq!(
        upstream_json(&http.get(&["ambiguous"], &[]).await.unwrap())
            .unwrap_err()
            .code,
        ErrorCode::UpstreamProtocol
    );
    server.abort();
}
