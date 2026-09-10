use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, Credential, HttpResponse, Secret};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Each fixture serves exactly one request; dropping it always retires its task.
struct Fixture {
    http: ScopedHttp,
    task: tokio::task::JoinHandle<()>,
    sent: Option<tokio::sync::oneshot::Receiver<()>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn new(reply: Vec<u8>, hold_open: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (sent, received) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(stream.read_u8().await.unwrap());
                assert!(request.len() < 8192);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /api/group%2Fproject?key=value HTTP/1.1\r\n"));
            assert!(request.contains("x-fixture-credential: private-fixture-token\r\n"));
            assert!(!request.contains("range:"));
            stream.write_all(&reply).await.unwrap();
            let _ = sent.send(());
            if hold_open {
                std::future::pending::<()>().await;
            }
        });
        let http = ScopedHttp::new(
            &HttpConfig {
                base_url: format!("http://{address}/api/"),
                credential: None,
                credential_header: "x-fixture-credential".into(),
                bearer: false,
                allow_plaintext: true,
                ca_file: None,
            },
            Some(Arc::new(Fixed)),
        )
        .unwrap();
        Self {
            http,
            task,
            sent: Some(received),
        }
    }
    async fn read(&self, limit: usize) -> Result<connectors_sdk::HttpResponsePrefix> {
        self.http
            .get_prefix(&["group/project"], &[("key", "value".into())], limit)
            .await
    }
}
struct Fixed;
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(b"private-fixture-token".to_vec()))
    }
}

#[tokio::test]
async fn prefix_distinguishes_eof_from_omitted_bytes_for_fixed_and_chunked_bodies() {
    for (wire, body, complete) in [
        ("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n", "", true),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\n12345678",
            "12345678",
            true,
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n123456789",
            "12345678",
            false,
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n1234\r\n4\r\n5678\r\n0\r\n\r\n",
            "12345678",
            true,
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n8\r\n12345678\r\n1\r\n9\r\n0\r\n\r\n",
            "12345678",
            false,
        ),
        (
            "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n12345678",
            "12345678",
            true,
        ),
    ] {
        let fixture = Fixture::new(wire.as_bytes().to_vec(), false).await;
        let result = fixture.read(8).await.unwrap();
        assert_eq!(result.status, 200);
        assert_eq!(result.body, body.as_bytes());
        assert_eq!(result.complete, complete, "{wire}");
        assert!(result.body.capacity() <= 8);
    }
}

#[tokio::test]
async fn premature_fixed_or_chunked_closure_is_an_error_even_after_exact_prefix() {
    for wire in [
        "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n12345678",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n8\r\n12345678\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n123",
    ] {
        let fixture = Fixture::new(wire.as_bytes().to_vec(), false).await;
        assert_eq!(
            fixture.read(8).await.err().unwrap().code,
            ErrorCode::Unavailable
        );
    }
}

#[tokio::test]
async fn excess_bytes_return_without_waiting_for_a_stalled_remainder() {
    let fixture = Fixture::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\n\r\n123456789".to_vec(),
        true,
    )
    .await;
    let result = tokio::time::timeout(Duration::from_secs(2), fixture.read(8))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.body, b"12345678");
    assert!(!result.complete);
}

#[tokio::test]
async fn exact_prefix_still_obeys_original_deadline_and_caller_cancellation() {
    let mut fixture = Fixture::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n12345678".to_vec(),
        true,
    )
    .await;
    let sent = fixture.sent.take().unwrap();
    let read = fixture.read(8);
    tokio::pin!(read);
    tokio::select! {
        result = &mut read => panic!("unproven prefix completed: {}", result.is_ok()),
        result = sent => result.unwrap(),
    }
    // Start virtual time only after actual networking completed. The original
    // request timer continues while the body waits; it is never reset per chunk.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(16)).await;
    assert_eq!(read.await.err().unwrap().code, ErrorCode::Timeout);
    tokio::time::resume();

    let fixture = Fixture::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n12345678".to_vec(),
        true,
    )
    .await;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), fixture.read(8))
            .await
            .is_err()
    );
    // Dropping the timed-out read releases the response; fixture drop retires
    // the owned server without leaving an in-flight retry or spawned read task.
}

#[tokio::test]
async fn prefix_header_bytes_are_bounded_and_errors_do_not_include_provider_secrets() {
    let wire = format!(
        "HTTP/1.1 200 OK\r\nX-Long: {}\r\nContent-Length: 0\r\n\r\n",
        "s".repeat(32768)
    );
    let fixture = Fixture::new(wire.into_bytes(), false).await;
    let error = fixture.read(8).await.err().unwrap();
    assert_eq!(error.code, ErrorCode::Capacity);
    assert!(!error.to_string().contains("private-fixture-token"));
    assert!(!error.to_string().contains(&"s".repeat(100)));
}

struct MustNotResolve;
#[async_trait::async_trait]
impl Credential for MustNotResolve {
    async fn resolve(&self) -> Result<Secret> {
        panic!("invalid limit resolved a credential")
    }
}
struct LegacyPort;
#[async_trait::async_trait]
impl AuthenticatedHttp for LegacyPort {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        panic!("unsupported prefix attempted an unbounded fallback")
    }
}

#[tokio::test]
async fn invalid_limits_and_unsupported_ports_refuse_before_work() {
    let fixture = Fixture::new(Vec::new(), false).await;
    let http = fixture.http.with_credential(Arc::new(MustNotResolve));
    for limit in [0, 1_048_577, usize::MAX] {
        assert_eq!(
            http.get_prefix(&["anything"], &[], limit)
                .await
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(
        LegacyPort
            .get_prefix(&["anything"], &[], 8)
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::Unavailable
    );
}
