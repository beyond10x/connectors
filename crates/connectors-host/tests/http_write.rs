use axum::{
    Router,
    extract::OriginalUri,
    http::{HeaderMap, StatusCode},
    routing::put,
};
use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{Credential, Secret};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Token(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl Credential for Token {
    async fn resolve(&self) -> Result<Secret> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Secret(b"write-fixture-token".to_vec()))
    }
}
fn http(address: std::net::SocketAddr, resolutions: Arc<AtomicUsize>) -> ScopedHttp {
    ScopedHttp::new(
        &HttpConfig {
            base_url: format!("http://{address}/api/v4/"),
            credential: None,
            credential_header: "private-token".into(),
            bearer: false,
            allow_plaintext: true,
            ca_file: None,
        },
        Some(Arc::new(Token(resolutions))),
    )
    .unwrap()
}

#[tokio::test]
async fn put_preserves_exact_segments_body_and_native_response_without_redirect() {
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_address = target.local_addr().unwrap();
    let destination_hits = Arc::new(AtomicUsize::new(0));
    let hits = destination_hits.clone();
    let destination = tokio::spawn(async move {
        axum::serve(
            target,
            Router::new().fallback(put(move || {
                hits.fetch_add(1, Ordering::SeqCst);
                async { StatusCode::OK }
            })),
        )
        .await
        .unwrap();
    });
    let source = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = source.local_addr().unwrap();
    let sends = Arc::new(AtomicUsize::new(0));
    let observed = sends.clone();
    let server = tokio::spawn(async move {
        axum::serve(source, Router::new().fallback(put(move |uri: OriginalUri, headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
            observed.fetch_add(1, Ordering::SeqCst);
            async move {
                assert_eq!(uri.0.path(), "/api/v4/projects/group%2Frepo%3Fname%23literal/merge_requests/7/merge");
                assert_eq!(uri.0.query(), Some("key=a%26b%3Dc"));
                assert_eq!(headers["private-token"], "write-fixture-token");
                assert_eq!(headers["content-type"], "application/json");
                assert_eq!(body, json!({"sha":"a".repeat(40),"auto_merge":false,"should_remove_source_branch":false}));
                (StatusCode::TEMPORARY_REDIRECT, [("location", format!("http://{target_address}/escape"))], "fixture-response")
            }
        }))).await.unwrap();
    });
    let credentials = Arc::new(AtomicUsize::new(0));
    let response = http(address, credentials.clone())
        .into_write()
        .put_json(
            &[
                "projects",
                "group/repo?name#literal",
                "merge_requests",
                "7",
                "merge",
            ],
            &[("key", "a&b=c".into())],
            &json!({"sha":"a".repeat(40),"auto_merge":false,"should_remove_source_branch":false}),
        )
        .await
        .unwrap();
    assert_eq!(response.status, 307);
    assert_eq!(response.body, b"fixture-response");
    assert_eq!(sends.load(Ordering::SeqCst), 1);
    assert_eq!(credentials.load(Ordering::SeqCst), 1);
    assert_eq!(destination_hits.load(Ordering::SeqCst), 0);
    server.abort();
    destination.abort();
}

#[tokio::test]
async fn lost_reply_after_complete_put_is_never_retried() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let provider = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let (end, length) = loop {
            let mut chunk = [0_u8; 512];
            let count = stream.read(&mut chunk).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&chunk[..count]);
            assert!(bytes.len() < 8192);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = std::str::from_utf8(&bytes[..end]).unwrap();
                assert_eq!(header.lines().next(), Some("PUT /api/v4/items/7? HTTP/1.1"));
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while bytes.len() < end + length {
            let mut chunk = [0_u8; 512];
            let count = stream.read(&mut chunk).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&chunk[..count]);
        }
        assert_eq!(&bytes[end..end + length], br#"{"change":true}"#);
        // Model an effect after the complete request, then lose the response.
        drop(stream);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), listener.accept())
                .await
                .is_err(),
            "a second connection attempted to repeat the write"
        );
        1_usize
    });
    let credentials = Arc::new(AtomicUsize::new(0));
    let error = http(address, credentials.clone())
        .into_write()
        .put_json(&["items", "7"], &[], &json!({"change":true}))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert!(!error.to_string().contains("write-fixture-token"));
    assert!(!error.to_string().contains(&address.to_string()));
    assert_eq!(credentials.load(Ordering::SeqCst), 1);
    assert_eq!(provider.await.unwrap(), 1);
}

#[tokio::test]
async fn native_failures_are_returned_once_and_body_limits_remain_errors() {
    let source = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = source.local_addr().unwrap();
    let sends = Arc::new(AtomicUsize::new(0));
    let hits = sends.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            source,
            Router::new()
                .route(
                    "/api/v4/unavailable",
                    put(move || {
                        hits.fetch_add(1, Ordering::SeqCst);
                        async {
                            (
                                StatusCode::SERVICE_UNAVAILABLE,
                                [("retry-after", "0")],
                                "native-error",
                            )
                        }
                    }),
                )
                .route(
                    "/api/v4/large",
                    put(|| async { vec![0_u8; connectors_core::RESPONSE_LIMIT + 1] }),
                ),
        )
        .await
        .unwrap();
    });
    let credentials = Arc::new(AtomicUsize::new(0));
    let response = http(address, credentials.clone())
        .into_write()
        .put_json(&["unavailable"], &[], &json!({}))
        .await
        .unwrap();
    assert_eq!(response.status, 503);
    assert_eq!(response.headers["retry-after"], "0");
    assert_eq!(response.body, b"native-error");
    assert_eq!(sends.load(Ordering::SeqCst), 1);
    let error = http(address, credentials)
        .into_write()
        .put_json(&["large"], &[], &json!({}))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Capacity);
    server.abort();
}

#[tokio::test]
async fn invalid_target_segments_refuse_before_credential_resolution() {
    let credentials = Arc::new(AtomicUsize::new(0));
    for segment in ["", ".", ".."] {
        let error = http("127.0.0.1:1".parse().unwrap(), credentials.clone())
            .into_write()
            .put_json(&["items", segment], &[], &json!({}))
            .await
            .err()
            .unwrap();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
    assert_eq!(credentials.load(Ordering::SeqCst), 0);
}
