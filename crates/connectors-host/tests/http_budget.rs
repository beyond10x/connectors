use axum::{Router, routing::any};
use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, Credential, Secret};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Fixture {
    http: ScopedHttp,
    calls: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn new(delay: Duration) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let app = Router::new().route(
            "/call",
            any(move || {
                let counted = counted.clone();
                async move {
                    counted.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(delay).await;
                    "{}"
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let http = ScopedHttp::from_config(&HttpConfig {
            base_url: format!("http://{address}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        })
        .unwrap();
        Self {
            http,
            calls,
            server,
        }
    }
    async fn close(self) {
        self.server.abort();
        assert!(self.server.await.err().unwrap().is_cancelled());
    }
}

struct CredentialProbe {
    calls: Arc<AtomicUsize>,
    stalls: bool,
}
#[async_trait::async_trait]
impl Credential for CredentialProbe {
    async fn resolve(&self) -> Result<Secret> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.stalls {
            std::future::pending::<()>().await;
        }
        Ok(Secret(b"fixture-only".to_vec()))
    }
}

#[tokio::test]
async fn provider_budget_bounds_credentials_before_any_http_dispatch() {
    let fixture = Fixture::new(Duration::ZERO).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let http = fixture
        .http
        .with_provider_budget(
            Duration::from_millis(100),
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap()
        .with_credential(Arc::new(CredentialProbe {
            calls: calls.clone(),
            stalls: true,
        }));
    let error = tokio::time::timeout(Duration::from_secs(2), http.get(&["call"], &[]))
        .await
        .expect("credential resolution escaped the provider cutoff")
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(!error.upstream_answer);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    fixture.close().await;
}

#[tokio::test]
async fn provider_budget_cannot_be_reset_by_another_request_or_derived_capability() {
    let fixture = Fixture::new(Duration::ZERO).await;
    let until = Instant::now() + Duration::from_millis(200);
    let http = fixture
        .http
        .with_provider_budget(Duration::from_secs(15), until)
        .unwrap();
    assert_eq!(http.get(&["call"], &[]).await.unwrap().status, 200);
    // Derive while valid, then cross the original execution cutoff.
    let calls = Arc::new(AtomicUsize::new(0));
    let credential = http.with_credential(Arc::new(CredentialProbe {
        calls: calls.clone(),
        stalls: false,
    }));
    let probe = http.probe_capability(&["call"]).unwrap();
    let write = http
        .with_provider_budget(Duration::from_secs(30), until + Duration::from_secs(30))
        .unwrap()
        .into_write();
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        until + Duration::from_millis(30),
    ))
    .await;
    for error in [
        http.get(&["call"], &[]).await.err().unwrap(),
        credential
            .get_prefix(&["call"], &[], 10)
            .await
            .err()
            .unwrap(),
        probe.probe(&serde_json::json!({})).await.err().unwrap(),
        write
            .put_json(&["call"], &[], &serde_json::json!({}))
            .await
            .err()
            .unwrap(),
        http.post_form(&["call"], &[("grant_type", b"fixture")])
            .await
            .err()
            .unwrap(),
    ] {
        assert_eq!(error.code, ErrorCode::Timeout);
        assert!(!error.upstream_answer);
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "expired capability resolved a credential"
    );
    assert_eq!(
        fixture.calls.load(Ordering::SeqCst),
        1,
        "expired capability sent HTTP"
    );
    fixture.close().await;
}

#[tokio::test]
async fn provider_budget_uses_the_earlier_execution_deadline_during_http() {
    let fixture = Fixture::new(Duration::from_secs(5)).await;
    let http = fixture
        .http
        .with_provider_budget(
            Duration::from_secs(30),
            Instant::now() + Duration::from_millis(150),
        )
        .unwrap();
    let error = tokio::time::timeout(Duration::from_secs(2), http.get(&["call"], &[]))
        .await
        .expect("HTTP escaped the original execution deadline")
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(error.upstream_answer);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    fixture.close().await;
}

#[tokio::test]
async fn provider_budget_generic_30_seconds_is_not_clipped_to_legacy_15() {
    let fixture = Fixture::new(Duration::from_secs(16)).await;
    let http = fixture
        .http
        .with_provider_budget(
            Duration::from_secs(30),
            Instant::now() + Duration::from_secs(40),
        )
        .unwrap();
    let response = http
        .get(&["call"], &[])
        .await
        .expect("selected generic budget was clipped");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"{}");
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    fixture.close().await;
}

#[tokio::test]
async fn provider_budget_rejects_unsupported_intervals_and_expired_deadlines() {
    let fixture = Fixture::new(Duration::ZERO).await;
    for budget in [Duration::ZERO, Duration::from_secs(31), Duration::MAX] {
        assert!(
            matches!(fixture.http.with_provider_budget(budget, Instant::now() + Duration::from_secs(40)),
            Err(error) if error.code == ErrorCode::InvalidInput)
        );
    }
    assert!(
        matches!(fixture.http.with_provider_budget(Duration::from_secs(15), Instant::now()),
        Err(error) if error.code == ErrorCode::Timeout)
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    fixture.close().await;
}

#[tokio::test]
async fn provider_budget_remains_in_force_while_every_capability_reads_the_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for family in ["get", "prefix", "write", "probe", "form"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let answered = Arc::new(AtomicUsize::new(0));
        let count = answered.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\n{")
                .await
                .unwrap();
            socket.flush().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            std::future::pending::<()>().await;
            drop(socket);
        });
        let http = ScopedHttp::from_config(&HttpConfig {
            base_url: format!("http://{address}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: false,
            allow_plaintext: true,
            ca_file: None,
        })
        .unwrap()
        .with_provider_budget(
            Duration::from_millis(500),
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), async {
            match family {
                "get" => http.get(&["call"], &[]).await.map(|_| ()),
                "prefix" => http.get_prefix(&["call"], &[], 10).await.map(|_| ()),
                "write" => http
                    .into_write()
                    .put_json(&["call"], &[], &serde_json::json!({}))
                    .await
                    .map(|_| ()),
                "probe" => http
                    .probe_capability(&["call"])
                    .unwrap()
                    .probe(&serde_json::json!({}))
                    .await
                    .map(|_| ()),
                "form" => http
                    .post_form(&["call"], &[("grant_type", b"fixture")])
                    .await
                    .map(|_| ()),
                _ => unreachable!(),
            }
        })
        .await;
        server.abort();
        assert!(server.await.err().unwrap().is_cancelled());
        let error = result
            .expect("body consumption escaped the cutoff")
            .err()
            .unwrap();
        assert_eq!(error.code, ErrorCode::Timeout, "{family}");
        assert!(error.upstream_answer, "{family}");
        assert_eq!(answered.load(Ordering::SeqCst), 1, "{family}");
    }
}
