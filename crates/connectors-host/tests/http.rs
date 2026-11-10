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
    let prefix = http.get_prefix(&["redirect"], &[], 16).await.unwrap();
    assert_eq!(prefix.status, 302);
    assert!(prefix.complete);
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

#[tokio::test]
async fn custom_ca_accepts_its_endpoint_and_refuses_other_roots() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::{
        TlsAcceptor,
        rustls::{self, pki_types::PrivatePkcs8KeyDer},
    };
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let unrelated = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.cert.der().clone()],
        PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(server));
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            if let Ok(mut stream) = acceptor.accept(stream).await {
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    assert!(header.len() < 4096);
                    header.push(stream.read_u8().await.unwrap());
                }
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .await
                    .unwrap();
            }
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ca");
    let mut config = HttpConfig {
        base_url: format!("https://localhost:{}/", addr.port()),
        credential: None,
        credential_header: "authorization".into(),
        bearer: false,
        allow_plaintext: false,
        ca_file: Some(path.clone()),
    };
    // A bundle may hold several explicitly configured roots.
    std::fs::write(
        &path,
        format!("{}{}", unrelated.cert.pem(), cert.cert.pem()),
    )
    .unwrap();
    assert_eq!(
        ScopedHttp::from_config(&config)
            .unwrap()
            .get(&["resource"], &[])
            .await
            .unwrap()
            .status,
        200
    );
    std::fs::write(&path, unrelated.cert.pem()).unwrap();
    assert_eq!(
        ScopedHttp::from_config(&config)
            .unwrap()
            .get(&["resource"], &[])
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::Unavailable
    );
    std::fs::write(&path, "").unwrap();
    assert!(ScopedHttp::from_config(&config).is_err());
    std::fs::write(&path, "not a certificate").unwrap();
    assert!(ScopedHttp::from_config(&config).is_err());
    config.ca_file = None;
    assert_eq!(
        ScopedHttp::from_config(&config)
            .unwrap()
            .get(&["resource"], &[])
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::Unavailable
    );
    task.abort();
}

/// story:http-empty-query-no-trailing-question-mark: a request with no query
/// pairs reaches the provider as its path alone, with no `?`; one with pairs
/// still carries them. Read, read prefix and write share the one URL builder.
#[tokio::test]
async fn empty_query_sends_no_question_mark() {
    use connectors_sdk::WriteMethod;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut targets = Vec::new();
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
            targets.push(head.lines().next().unwrap().to_owned());
        }
        targets
    });
    let config = HttpConfig {
        base_url: format!("http://{address}/api/"),
        credential: None,
        credential_header: "authorization".into(),
        bearer: true,
        allow_plaintext: true,
        ca_file: None,
    };
    let http = ScopedHttp::from_config(&config).unwrap();
    assert_eq!(http.get(&["token"], &[]).await.unwrap().status, 200);
    assert_eq!(
        http.get_prefix(&["token"], &[], 16).await.unwrap().status,
        200
    );
    assert_eq!(
        http.get(&["list"], &[("page", "a b".to_string())])
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(
        http.into_write()
            .send_json(
                WriteMethod::Post,
                &["files", "copy"],
                &[],
                &serde_json::json!({})
            )
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(
        server.await.unwrap(),
        [
            "GET /api/token HTTP/1.1",
            "GET /api/token HTTP/1.1",
            "GET /api/list?page=a+b HTTP/1.1",
            "POST /api/files/copy HTTP/1.1",
        ]
    );
}
