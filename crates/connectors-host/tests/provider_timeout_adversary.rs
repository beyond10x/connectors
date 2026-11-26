//! Adversary cases for the provider-timeout mark (story
//! provider-refusal-stage-for-writes-and-timeouts). `contracts/cli/v1alpha1/semantics.md`
//! §6: "a request the provider transport sent whose deadline then passed ... report
//! `timeout` at `stage = dispatch`", and "A connection that never opened sent nothing
//! and is not one of them."
//!
//! Each case has two tests on one body. The gate's test runs the body in a child process of
//! this binary whose environment shortens the one client timeout the case waits for; the
//! `slow_lane_` test runs it in this process at production's 15 s and 5 s.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::AuthenticatedHttp;
use std::{
    ops::Range,
    process::Command,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The client's request timeout in a test build (`connectors_host::http`, production 15 s).
const REQUEST_MS: &str = "CONNECTORS_TEST_OVERRIDE_HTTP_REQUEST_MS";
/// The client's connect timeout in a test build (production 5 s).
const CONNECT_MS: &str = "CONNECTORS_TEST_OVERRIDE_HTTP_CONNECT_MS";
/// What the gate's tests set the two to.
const SHORT_REQUEST: Duration = Duration::from_millis(500);
const SHORT_CONNECT: Duration = Duration::from_millis(300);
/// Present in the child a gate test starts: that process runs the test's body.
const CHILD: &str = "CONNECTORS_HTTP_TEST_CHILD";

fn is_short_timeout_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

/// Runs `test` of this binary alone in a child process whose environment sets `variable` to
/// `timeout`, and holds that the child ran that one test and that it passed. The other
/// timeout keeps production's value there, so it cannot be the one that cuts the request.
/// The variable reaches the child only: no other test of this binary sees it.
fn run_in_short_timeout_child(test: &str, variable: &str, timeout: Duration) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env(CHILD, "1")
        .env(variable, timeout.as_millis().to_string())
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && printed.contains("test result: ok. 1 passed"),
        "the child failed or ran no test: {}\n{printed}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

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
/// headers, then stalled the body until the transport's request deadline passed.
/// That is a sent request whose deadline passed: the contract marks it as the
/// provider's timeout (dispatch). The body read maps it through
/// `connectors_client::transport_error`, which never marks it.
///
/// `refused_within` is how long after the request started the refusal may come:
/// no sooner than its start, sooner than its end.
async fn a_stalled_body_is_the_providers_timeout(refused_within: Range<Duration>) {
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
    let elapsed = started.elapsed();
    server.abort();
    assert!(
        refused_within.contains(&elapsed),
        "refused after {elapsed:?}, outside {refused_within:?}"
    );
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        error.upstream_answer,
        "a sent request whose deadline passed while the provider streamed its answer is not marked as the provider's timeout"
    );
}

/// At the request timeout the child's environment sets: no sooner than it, and sooner than
/// 10 s, which production's 15 s would not meet.
#[tokio::test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the request timeout override is debug-only"
)]
async fn a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout() {
    if !is_short_timeout_child() {
        return run_in_short_timeout_child(
            "a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout",
            REQUEST_MS,
            SHORT_REQUEST,
        );
    }
    a_stalled_body_is_the_providers_timeout(SHORT_REQUEST..Duration::from_secs(10)).await;
}

/// Lane `slow`: the one run that lives through production's 15 s request timeout. The
/// refusal comes no sooner than those 15 s, and before the fixture gives up after 40 s.
#[tokio::test]
#[ignore = "lane slow: waits out the production request timeout of 15 s"]
async fn slow_lane_a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout() {
    a_stalled_body_is_the_providers_timeout(Duration::from_secs(15)..Duration::from_secs(40)).await;
}

/// A TLS handshake the peer never answers: no HTTP request was sent, so the
/// timeout must not be marked as the provider's answer.
///
/// `refused_within` is how long after the request, and with it the connect, started the
/// refusal may come: no sooner than its start, sooner than its end.
async fn an_unanswered_tls_handshake_sent_no_request(refused_within: Range<Duration>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (hello, mut hello_read) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0_u8; 4096];
        // Read the ClientHello and never answer it.
        let read = socket.read(&mut buffer).await.unwrap_or(0);
        let _ = hello.send(read);
        tokio::time::sleep(Duration::from_secs(40)).await;
        drop(socket);
    });
    // The listener's own address, not a name: a name with two address families can spend the
    // connect timeout before the first byte of a handshake is sent.
    let mut config = config(format!("https://{address}/"));
    config.allow_plaintext = false;
    let http = ScopedHttp::from_config(&config).unwrap();
    let started = Instant::now();
    let Err(error) = http.get(&["items"], &[]).await else {
        panic!("the request succeeded")
    };
    let elapsed = started.elapsed();
    let hello_bytes = hello_read.try_recv().unwrap_or(0);
    server.abort();
    assert!(
        hello_bytes > 0,
        "the peer read no byte before the refusal after {elapsed:?}: no handshake started"
    );
    assert!(
        refused_within.contains(&elapsed),
        "refused after {elapsed:?}, outside {refused_within:?}"
    );
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        !error.upstream_answer,
        "a handshake that never completed sent no request"
    );
}

/// At the connect timeout the child's environment sets: no sooner than it, and sooner than
/// production's 5 s. The request timeout keeps production's 15 s in the child.
#[tokio::test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the connect timeout override is debug-only"
)]
async fn a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked() {
    if !is_short_timeout_child() {
        return run_in_short_timeout_child(
            "a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked",
            CONNECT_MS,
            SHORT_CONNECT,
        );
    }
    an_unanswered_tls_handshake_sent_no_request(SHORT_CONNECT..Duration::from_secs(5)).await;
}

/// Lane `slow`: the one run that lives through production's 5 s connect timeout. The refusal
/// comes no sooner than 5 s after the connect started, and sooner than the 15 s request
/// timeout: the handshake is cut by the connect timeout.
#[tokio::test]
#[ignore = "lane slow: waits out the production connect timeout of 5 s"]
async fn slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked() {
    an_unanswered_tls_handshake_sent_no_request(Duration::from_secs(5)..Duration::from_secs(15))
        .await;
}
