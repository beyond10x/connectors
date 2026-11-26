//! Adversary cases for the connect bound a provider budget sets (story
//! a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout,
//! commit 55210be72). That commit ended a connect 250 ms before the budget did, when at least
//! 500 ms were left. Since then a connect started inside `ScopedHttp::send` may run until the
//! end of the budget itself, and a budget that ends while the request's connection is still
//! opening is not the provider's timeout.
//!
//! Every peer is a loopback listener. No test shortens a client timeout: production's 15 s
//! and 5 s apply, and the budget is the bound that ends each call.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::AuthenticatedHttp;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

/// What the peer does with one connection. The peer takes them in the order it accepts;
/// a connection past the end of the plan gets the plan's last entry.
#[derive(Clone, Copy)]
enum Conn {
    /// Reads the first bytes of the connection and never answers them.
    Silent,
    /// Waits, then completes the TLS handshake, reads one request head and answers 200.
    AnswersAfter(Duration),
    /// Completes the TLS handshake, reads one request head, waits, answers 200 and keeps the
    /// connection open; then reads the next request head and never answers it.
    AnswersOnceAfter(Duration),
}

/// Reads one request head from `stream`. False when the peer closed before its end.
async fn read_head(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> bool {
    let mut head = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buffer).await.unwrap_or(0);
        if n == 0 {
            return false;
        }
        head.extend_from_slice(&buffer[..n]);
    }
    true
}

/// A TLS peer on 127.0.0.1 with a certificate for that address, and the file that holds it.
struct Peer {
    address: std::net::SocketAddr,
    pem: String,
    ca: tempfile::TempDir,
    /// The connections the peer accepted.
    accepted: Arc<AtomicUsize>,
    /// The second request heads read on an `AnswersOnceAfter` connection.
    reused: Arc<AtomicUsize>,
}

impl Peer {
    async fn start(plan: Vec<Conn>) -> Self {
        // The address, not a name: no resolver runs before the connect.
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
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
        let acceptor = TlsAcceptor::from(Arc::new(server));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let reused = Arc::new(AtomicUsize::new(0));
        let (accepted_count, reused_count) = (accepted.clone(), reused.clone());
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let index = accepted_count.fetch_add(1, Ordering::SeqCst);
                let conn = plan[index.min(plan.len() - 1)];
                let acceptor = acceptor.clone();
                let reused = reused_count.clone();
                tokio::spawn(async move {
                    match conn {
                        Conn::Silent => {
                            let mut buffer = [0_u8; 4096];
                            let _ = stream.read(&mut buffer).await;
                            tokio::time::sleep(Duration::from_secs(40)).await;
                        }
                        Conn::AnswersAfter(delay) => {
                            tokio::time::sleep(delay).await;
                            let Ok(mut stream) = acceptor.accept(stream).await else {
                                return;
                            };
                            if !read_head(&mut stream).await {
                                return;
                            }
                            let _ = stream
                                .write_all(
                                    b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
                                )
                                .await;
                            let _ = stream.shutdown().await;
                        }
                        Conn::AnswersOnceAfter(delay) => {
                            let Ok(mut stream) = acceptor.accept(stream).await else {
                                return;
                            };
                            if !read_head(&mut stream).await {
                                return;
                            }
                            tokio::time::sleep(delay).await;
                            let _ = stream
                                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}")
                                .await;
                            let _ = stream.flush().await;
                            if read_head(&mut stream).await {
                                reused.fetch_add(1, Ordering::SeqCst);
                                tokio::time::sleep(Duration::from_secs(40)).await;
                            }
                        }
                    }
                });
            }
        });
        let ca = tempfile::tempdir().unwrap();
        let pem = cert.cert.pem();
        std::fs::write(ca.path().join("ca"), &pem).unwrap();
        Self {
            address,
            pem,
            ca,
            accepted,
            reused,
        }
    }

    /// A port of its own on the peer: its own client, so no connection is pooled yet.
    fn port(&self) -> ScopedHttp {
        ScopedHttp::from_config(&HttpConfig {
            base_url: format!("https://{}/", self.address),
            credential: None,
            credential_header: "authorization".into(),
            bearer: false,
            allow_plaintext: false,
            ca_file: Some(self.ca.path().join("ca")),
        })
        .unwrap()
    }

    /// One `get` on a port of its own under a provider budget of `budget`: the status, or
    /// the refusal's code and mark, and how long the call took.
    async fn get_under(&self, budget: Duration) -> (Result<u16, (ErrorCode, bool)>, Duration) {
        let http = self
            .port()
            .with_provider_budget(budget, Instant::now() + Duration::from_secs(30))
            .unwrap();
        let started = Instant::now();
        let result = http
            .get(&["items"], &[])
            .await
            .map(|response| response.status)
            .map_err(|error| (error.code, error.upstream_answer));
        (result, started.elapsed())
    }

    /// The request `ScopedHttp::send` made before the change, on the client it was made on:
    /// the builder of `ScopedHttp::new_with_ca_bytes` without the connector layer, and the
    /// time the budget has left as the request's timeout. Nothing bounds the connect but the
    /// client's connect timeout of 5 s.
    async fn get_as_before_the_change(
        &self,
        remaining: Duration,
    ) -> (Result<u16, String>, Duration) {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .tls_certs_only(reqwest::Certificate::from_pem_bundle(self.pem.as_bytes()).unwrap())
            .build()
            .unwrap();
        let started = Instant::now();
        let result = client
            .get(format!("https://{}/items", self.address))
            .timeout(remaining)
            .send()
            .await
            .map(|response| response.status().as_u16())
            .map_err(|error| {
                format!(
                    "is_timeout = {}, is_connect = {}",
                    error.is_timeout(),
                    error.is_connect()
                )
            });
        (result, started.elapsed())
    }
}

/// The old band edge at 500 ms. A peer that answers the TLS handshake after 340 ms and the
/// request at once answers a call that has 495 ms of its budget left: under 500 ms 55210be72
/// set no connect bound. The same peer must then answer a call that has more left. With
/// 560 ms left 55210be72 bounded the connect at 560 - 250 = 310 ms, before the peer's 340 ms.
#[tokio::test]
async fn a_call_with_more_of_its_budget_left_is_answered_where_one_with_less_is() {
    let peer = Peer::start(vec![Conn::AnswersAfter(Duration::from_millis(340))]).await;
    let (less, less_took) = peer.get_under(Duration::from_millis(495)).await;
    assert_eq!(
        less,
        Ok(200),
        "the fixture: with 495 ms left the call was not answered, after {less_took:?}"
    );
    let (more, more_took) = peer.get_under(Duration::from_millis(560)).await;
    assert_eq!(
        more,
        Ok(200),
        "with 560 ms left the call ended after {more_took:?} as (code, upstream_answer); with 495 ms left the same peer answered 200 after {less_took:?}"
    );
}

/// The lead 55210be72 took from a call that would have been answered. The peer answers the
/// TLS handshake after 800 ms and the request at once, inside a budget of 1,000 ms. The
/// request as `send` made it before 55210be72 is answered 200. The call under the budget must
/// be answered too: its budget had not ended.
#[tokio::test]
async fn a_handshake_answered_in_the_last_250_ms_of_the_budget_is_a_call_answered_inside_it() {
    let budget = Duration::from_millis(1000);
    let peer = Peer::start(vec![Conn::AnswersAfter(Duration::from_millis(800))]).await;
    let (before, before_took) = peer.get_as_before_the_change(budget).await;
    assert_eq!(
        before,
        Ok(200),
        "the fixture: the request as sent before the change was not answered, after {before_took:?}"
    );
    let (after, after_took) = peer.get_under(budget).await;
    assert_eq!(
        after,
        Ok(200),
        "under a budget of 1,000 ms the call ended after {after_took:?} as (code, upstream_answer); the request as sent before the change was answered 200 after {before_took:?}"
    );
}

/// Constructed, to show what the lead of 55210be72 rested on: the test's thread does not run
/// from 700 ms to 1,100 ms after the call started, which covers that commit's connect bound
/// (750 ms) and the end of the budget (1,000 ms). The peer never answers the handshake, so no
/// request was sent and the timeout must not be marked (acceptance 1 of the story).
#[tokio::test]
async fn constructed_a_thread_that_does_not_run_over_the_lead_leaves_an_unanswered_handshake_unmarked()
 {
    let peer = Peer::start(vec![Conn::Silent]).await;
    let http = peer
        .port()
        .with_provider_budget(
            Duration::from_millis(1000),
            Instant::now() + Duration::from_secs(30),
        )
        .unwrap();
    let started = tokio::time::Instant::now();
    tokio::spawn(async move {
        tokio::time::sleep_until(started + Duration::from_millis(700)).await;
        // Blocks the current-thread runtime: nothing is polled until it returns.
        std::thread::sleep(Duration::from_millis(400));
    });
    let Err(error) = http.get(&["items"], &[]).await else {
        panic!("the request succeeded")
    };
    let elapsed = started.elapsed();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        !error.upstream_answer,
        "an unanswered handshake sent no request, after {elapsed:?}: upstream_answer = {}",
        error.upstream_answer
    );
}

/// A budget that ends before the TCP connection opens (story, "Change": "No test builds that
/// case: a TCP connect that hangs needs an address outside the host"). A loopback listener
/// whose accept queue is full drops the SYN, so the connect hangs on this host. The connect
/// runs until the budget ends, not a lead before it, and the timeout is not marked.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_budget_that_ends_before_the_tcp_connection_opens_is_not_marked() {
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    // Never accepted from.
    let listener = socket.listen(0).unwrap();
    let address = listener.local_addr().unwrap();
    let mut held = Vec::new();
    let mut full = false;
    for _ in 0..16 {
        let connect = tokio::net::TcpStream::connect(address);
        match tokio::time::timeout(Duration::from_millis(300), connect).await {
            Ok(stream) => held.push(stream.unwrap()),
            Err(_) => {
                full = true;
                break;
            }
        }
    }
    assert!(
        full,
        "the fixture: {} connections opened and none hung",
        held.len()
    );
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
        Duration::from_millis(1000),
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let started = Instant::now();
    let Err(error) = http.get(&["items"], &[]).await else {
        panic!("the request succeeded")
    };
    let elapsed = started.elapsed();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        !error.upstream_answer,
        "a connection that never opened sent nothing, after {elapsed:?}"
    );
    assert!(
        (Duration::from_millis(900)..Duration::from_millis(2000)).contains(&elapsed),
        "the call ended after {elapsed:?}, not with the end of the budget"
    );
    drop(held);
}

/// The budget belongs to the request that set it. Two requests on one client are polled on
/// one task: the first under a budget of 1,000 ms to a connection that never answers the
/// handshake, the second without a budget to a connection that answers after 1,200 ms, later
/// than the end of the first one's budget. The first ends with its budget and is not marked;
/// the second is answered. (Against 55210be72 the second connection answered after 900 ms,
/// later than that commit's connect bound of 750 ms.)
#[tokio::test]
async fn a_budget_bounds_its_own_connect_and_not_that_of_a_request_beside_it() {
    let peer = Peer::start(vec![
        Conn::Silent,
        Conn::AnswersAfter(Duration::from_millis(1200)),
    ])
    .await;
    let unbounded = peer.port();
    let bounded = unbounded
        .with_provider_budget(
            Duration::from_millis(1000),
            Instant::now() + Duration::from_secs(30),
        )
        .unwrap();
    let started = Instant::now();
    let (cut, answered) = tokio::join!(
        async {
            let result = bounded.get(&["items"], &[]).await;
            (result, started.elapsed())
        },
        async {
            let result = unbounded.get(&["items"], &[]).await;
            (result, started.elapsed())
        },
    );
    let (cut, cut_after) = cut;
    let (answered, answered_after) = answered;
    assert_eq!(
        cut.map(|response| response.status)
            .map_err(|error| (error.code, error.upstream_answer)),
        Err((ErrorCode::Timeout, false)),
        "the request under the budget, after {cut_after:?}"
    );
    assert!(
        (Duration::from_millis(900)..Duration::from_millis(2000)).contains(&cut_after),
        "the call ended after {cut_after:?}, not with the end of the budget"
    );
    assert_eq!(
        answered
            .map(|response| response.status)
            .map_err(|error| (error.code, error.upstream_answer)),
        Ok(200),
        "the request without a budget, after {answered_after:?}"
    );
}

/// hyper-util's pool race. A request under a budget of 1,000 ms finds no idle connection, so
/// it starts a connect of its own, to a connection that never answers the TLS handshake, and
/// waits on the pool beside it. A request beside it on the same client gets its answer, its
/// connection comes back to the pool, and the waiting request is sent on it; its own connect
/// goes on in another task. The peer reads the request and never answers. The request was
/// sent, so the end of the budget is the provider's timeout, though its own connect never
/// opened.
#[tokio::test]
async fn a_request_sent_on_a_pooled_connection_while_its_own_connect_runs_is_the_providers_timeout()
{
    let peer = Peer::start(vec![
        Conn::AnswersOnceAfter(Duration::from_millis(300)),
        Conn::Silent,
    ])
    .await;
    let unbounded = peer.port();
    let started = Instant::now();
    let (first, (second, second_after)) = tokio::join!(
        async {
            unbounded
                .get(&["items"], &[])
                .await
                .map(|response| response.status)
                .map_err(|error| (error.code, error.upstream_answer))
        },
        async {
            // The first request holds the one connection by then.
            tokio::time::sleep(Duration::from_millis(100)).await;
            let bounded = unbounded
                .with_provider_budget(
                    Duration::from_millis(1000),
                    Instant::now() + Duration::from_secs(30),
                )
                .unwrap();
            let result = bounded
                .get(&["items"], &[])
                .await
                .map(|response| response.status)
                .map_err(|error| (error.code, error.upstream_answer));
            (result, started.elapsed())
        },
    );
    assert_eq!(first, Ok(200), "the fixture: the first request");
    assert_eq!(
        peer.accepted.load(Ordering::SeqCst),
        2,
        "the fixture: the second request started no connect of its own"
    );
    assert_eq!(
        peer.reused.load(Ordering::SeqCst),
        1,
        "the fixture: the second request was not sent on the first connection"
    );
    assert_eq!(
        second,
        Err((ErrorCode::Timeout, true)),
        "a request sent on a pooled connection, after {second_after:?}"
    );
}

/// A refused connection is `Unavailable` and unmarked, with a budget and without one: the
/// layer leaves a connect error as it was.
///
/// Linux refuses a connect to a closed loopback port at once; Windows retries the SYN and
/// reports the refusal after about 2 s. The budget outlasts that, so the refusal arrives
/// while the connect is still inside it; a budget that ended first would be an unmarked
/// `Timeout` by design. The call must end well before the budget, so the refusal, not the
/// budget, ended it.
#[tokio::test]
async fn a_refused_connection_is_unavailable_with_and_without_a_budget() {
    const BUDGET: Duration = Duration::from_secs(5);
    const REFUSED_WITHIN: Duration = Duration::from_secs(4);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    for budget in [None, Some(BUDGET)] {
        let http = ScopedHttp::from_config(&HttpConfig {
            base_url: format!("http://{address}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: false,
            allow_plaintext: true,
            ca_file: None,
        })
        .unwrap();
        let http = match budget {
            Some(budget) => http
                .with_provider_budget(budget, Instant::now() + Duration::from_secs(30))
                .unwrap(),
            None => http,
        };
        let started = Instant::now();
        let result = http
            .get(&["items"], &[])
            .await
            .map(|response| response.status)
            .map_err(|error| (error.code, error.upstream_answer));
        let elapsed = started.elapsed();
        assert_eq!(
            result,
            Err((ErrorCode::Unavailable, false)),
            "budget {budget:?}, after {elapsed:?}"
        );
        assert!(
            elapsed < REFUSED_WITHIN,
            "budget {budget:?}: the refusal came after {elapsed:?}, not within {REFUSED_WITHIN:?}"
        );
    }
}
