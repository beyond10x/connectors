//! Adversary cases for the provider-timeout mark (story
//! provider-refusal-stage-for-writes-and-timeouts). `contracts/cli/v1alpha1/semantics.md`
//! §6: "a request the provider transport sent whose deadline then passed ... report
//! `timeout` at `stage = dispatch`", and "A connection that never opened sent nothing
//! and is not one of them."
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::AuthenticatedHttp;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn config(base: String) -> HttpConfig {
    HttpConfig {
        base_url: base,
        credential: None,
        credential_header: "authorization".into(),
        bearer: false,
        allow_plaintext: true,
        ca_file: None,
    }
}

/// The provider received the whole request and answered its status line and
/// headers, then stalled the body until the transport's 15 s deadline passed.
/// That is a sent request whose deadline passed: the contract marks it as the
/// provider's timeout (dispatch). The body read maps it through
/// `connectors_client::transport_error`, which never marks it.
#[tokio::test]
async fn a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                return;
            }
            request.extend_from_slice(&buffer[..n]);
        }
        socket
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 100\r\n\r\n{\"partial\":")
            .await
            .unwrap();
        socket.flush().await.unwrap();
        tokio::time::sleep(Duration::from_secs(40)).await;
        drop(socket);
    });
    let http = ScopedHttp::from_config(&config(format!("http://{address}/"))).unwrap();
    let started = Instant::now();
    let Err(error) = http.get(&["items"], &[]).await else {
        panic!("the request succeeded")
    };
    server.abort();
    assert!(
        started.elapsed() >= Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        error.upstream_answer,
        "a sent request whose deadline passed while the provider streamed its answer is not marked as the provider's timeout"
    );
}

/// A TLS handshake the peer never answers: no HTTP request was sent, so the
/// timeout must not be marked as the provider's answer.
#[tokio::test]
async fn a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0_u8; 4096];
        // Read the ClientHello and never answer it.
        let _ = socket.read(&mut buffer).await;
        tokio::time::sleep(Duration::from_secs(40)).await;
        drop(socket);
    });
    let mut config = config(format!("https://localhost:{}/", address.port()));
    config.allow_plaintext = false;
    let http = ScopedHttp::from_config(&config).unwrap();
    let Err(error) = http.get(&["items"], &[]).await else {
        panic!("the request succeeded")
    };
    server.abort();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        !error.upstream_answer,
        "a handshake that never completed sent no request"
    );
}
