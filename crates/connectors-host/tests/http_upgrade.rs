//! The upgraded stream of one admitted write
//! (`connectors.transport.UpgradeOutcome` and `UpgradedStreamEnd`,
//! `ess/domains/transport.yaml`), against loopback WebSocket fixtures. The
//! fixture side uses the framing library's server handshake; the port under
//! test is `ScopedHttp::into_upgrade_write`.
use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{Credential, MessageStream, Secret, UpgradeBounds, Upgraded};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message,
        handshake::server::{Callback, ErrorResponse, Request, Response},
    },
};

const PROTOCOLS: [&str; 2] = ["v5.channel.k8s.io", "v4.channel.k8s.io"];

struct Token(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl Credential for Token {
    async fn resolve(&self) -> Result<Secret> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Secret(b"upgrade-fixture-token".to_vec()))
    }
}

fn http(address: SocketAddr, resolutions: Arc<AtomicUsize>) -> ScopedHttp {
    ScopedHttp::new(
        &HttpConfig {
            base_url: format!("http://{address}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        Some(Arc::new(Token(resolutions))),
    )
    .unwrap()
}

fn bounds(message: usize, total: usize, timeout: Duration) -> UpgradeBounds {
    UpgradeBounds::new(message, total, timeout).unwrap()
}

fn path() -> [&'static str; 7] {
    [
        "api",
        "v1",
        "namespaces",
        "fixture",
        "pods",
        "api-0",
        "exec",
    ]
}

/// Accepts connections and counts them; each is handed to `serve`.
async fn listener<F, Fut>(serve: F) -> (SocketAddr, Arc<AtomicUsize>)
where
    F: Fn(TcpStream) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counted = accepted.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve(stream));
        }
    });
    (address, accepted)
}

/// The server handshake, selecting `selected` and recording the request.
async fn accept(
    stream: TcpStream,
    selected: &'static str,
    seen: Arc<Mutex<Vec<(String, String)>>>,
) -> WebSocketStream<TcpStream> {
    tokio_tungstenite::accept_hdr_async(stream, Handshake { selected, seen })
        .await
        .unwrap()
}

/// Records the client's handshake request and selects one subprotocol. The
/// framing library fixes the callback's error type.
struct Handshake {
    selected: &'static str,
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

impl Callback for Handshake {
    fn on_request(
        self,
        request: &Request,
        mut response: Response,
    ) -> std::result::Result<Response, ErrorResponse> {
        let mut seen = self.seen.lock().unwrap();
        seen.push(("uri".into(), request.uri().to_string()));
        for (name, value) in request.headers() {
            seen.push((name.as_str().into(), value.to_str().unwrap_or("").into()));
        }
        response
            .headers_mut()
            .insert("sec-websocket-protocol", self.selected.parse().unwrap());
        Ok(response)
    }
}

fn accepted(outcome: Upgraded) -> Box<dyn MessageStream> {
    match outcome {
        Upgraded::Accepted(stream) => stream,
        Upgraded::NotSent(error) => panic!("not sent: {error:?}"),
        Upgraded::Answered(status) => panic!("answered {status}"),
        Upgraded::Lost(error) => panic!("lost: {error:?}"),
    }
}

/// A capability built for an ordinary write (`into_write`) has no upgraded
/// stream: the upgrade is refused before a credential is resolved or a
/// connection opened. Only the composition preparing an admitted write builds
/// one with bounds (`into_upgrade_write`), and a read port has no upgrade at
/// all (the `compile_fail` example on `AuthenticatedWrite::upgrade`).
#[tokio::test]
async fn an_upgrade_outside_an_upgrade_write_is_refused_without_io() {
    let (address, connections) = listener(|_stream| async {}).await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let outcome = http(address, resolutions.clone())
        .into_write()
        .upgrade(&path(), &[], &PROTOCOLS)
        .await;
    let Upgraded::NotSent(error) = outcome else {
        panic!("an upgrade from a plain write capability was not refused before sending");
    };
    assert_eq!(error.code, ErrorCode::Unsupported);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(connections.load(Ordering::SeqCst), 0);
    assert_eq!(resolutions.load(Ordering::SeqCst), 0);
}

/// The upgrade reaches only the connection's configured host: a path segment
/// that would leave the configured base is refused with no I/O, and a
/// redirect to another host is an answer, never followed, so the other host
/// sees no connection at all.
#[tokio::test]
async fn an_upgrade_to_a_host_the_connection_does_not_admit_is_refused() {
    let (elsewhere, foreign) = listener(|_stream| async {}).await;
    let (address, connections) = listener(move |mut stream| async move {
        let mut buffer = vec![0_u8; 8192];
        let _ = stream.read(&mut buffer).await;
        let answer = format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{elsewhere}/api/v1/namespaces/fixture/pods/api-0/exec\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(answer.as_bytes()).await;
    })
    .await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let limits = bounds(1024, 4096, Duration::from_secs(5));

    let escape = http(address, resolutions.clone())
        .into_upgrade_write(limits)
        .upgrade(&["api", "..", "..", "exec"], &[], &PROTOCOLS)
        .await;
    let Upgraded::NotSent(error) = escape else {
        panic!("a segment leaving the configured base was not refused before sending");
    };
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(connections.load(Ordering::SeqCst), 0);

    let redirected = http(address, resolutions)
        .into_upgrade_write(limits)
        .upgrade(&path(), &[], &PROTOCOLS)
        .await;
    let Upgraded::Answered(status) = redirected else {
        panic!("a redirect answer was not reported as answered");
    };
    assert_eq!(status, 307);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    assert_eq!(
        foreign.load(Ordering::SeqCst),
        0,
        "the redirect was followed"
    );
}

/// The handshake carries the credential, the WebSocket version and key and
/// the offered subprotocols in order; ping is answered below the port and
/// only whole binary messages reach it; a close ends it cleanly.
#[tokio::test]
async fn an_accepted_upgrade_delivers_whole_binary_messages_and_answers_ping() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pong = Arc::new(AtomicUsize::new(0));
    let (address, _) = {
        let seen = seen.clone();
        let pong = pong.clone();
        listener(move |stream| {
            let seen = seen.clone();
            let pong = pong.clone();
            async move {
                let mut socket = accept(stream, "v5.channel.k8s.io", seen).await;
                let _ = socket
                    .send(Message::Ping(b"keepalive".to_vec().into()))
                    .await;
                let _ = socket
                    .send(Message::Binary(b"\x01hello".to_vec().into()))
                    .await;
                // The client answers the ping while it reads.
                if let Some(Ok(Message::Pong(payload))) = socket.next().await {
                    assert_eq!(&payload[..], b"keepalive");
                    pong.fetch_add(1, Ordering::SeqCst);
                }
                let _ = socket
                    .send(Message::Binary(b"\x03{}".to_vec().into()))
                    .await;
                let _ = socket.close(None).await;
            }
        })
        .await
    };
    let resolutions = Arc::new(AtomicUsize::new(0));
    let query = [
        ("container", "api".to_owned()),
        ("command", "true".to_owned()),
    ];
    let mut stream = accepted(
        http(address, resolutions.clone())
            .into_upgrade_write(bounds(1024, 4096, Duration::from_secs(5)))
            .upgrade(&path(), &query, &PROTOCOLS)
            .await,
    );
    assert_eq!(stream.protocol(), "v5.channel.k8s.io");
    assert_eq!(stream.next_message().await.unwrap().unwrap(), b"\x01hello");
    assert_eq!(stream.next_message().await.unwrap().unwrap(), b"\x03{}");
    assert!(stream.next_message().await.unwrap().is_none());
    assert!(stream.next_message().await.unwrap().is_none());
    assert_eq!(pong.load(Ordering::SeqCst), 1);
    assert_eq!(resolutions.load(Ordering::SeqCst), 1);
    let seen = seen.lock().unwrap().clone();
    let header = |name: &str| {
        seen.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    assert_eq!(
        header("uri"),
        "/api/v1/namespaces/fixture/pods/api-0/exec?container=api&command=true"
    );
    assert_eq!(header("authorization"), "Bearer upgrade-fixture-token");
    assert_eq!(header("sec-websocket-version"), "13");
    assert_eq!(
        header("sec-websocket-protocol"),
        "v5.channel.k8s.io, v4.channel.k8s.io"
    );
    assert!(!header("sec-websocket-key").is_empty());
}

/// The host, not the adapter, holds the byte bound: past `max_total_bytes`
/// the stream is closed, the port reports `Capacity` (truncated) and delivers
/// nothing more, and the server sees the connection end. A single message
/// over `max_message_bytes` ends it the same way.
#[tokio::test]
async fn the_byte_bound_closes_the_stream_and_reports_truncation() {
    let closed = Arc::new(AtomicUsize::new(0));
    let (address, _) = {
        let closed = closed.clone();
        listener(move |stream| {
            let closed = closed.clone();
            async move {
                let mut socket = accept(stream, "v5.channel.k8s.io", Arc::default()).await;
                for _ in 0..3 {
                    if socket
                        .send(Message::Binary(vec![1_u8; 400].into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                // The client closes once the bound is reached; the server
                // observes a close frame or the end of the connection.
                let ended = tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        match socket.next().await {
                            None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                            Some(Ok(_)) => continue,
                        }
                    }
                })
                .await;
                if ended.is_ok() {
                    closed.fetch_add(1, Ordering::SeqCst);
                }
            }
        })
        .await
    };
    let mut stream = accepted(
        http(address, Arc::default())
            .into_upgrade_write(bounds(512, 1000, Duration::from_secs(5)))
            .upgrade(&path(), &[], &PROTOCOLS)
            .await,
    );
    assert_eq!(stream.next_message().await.unwrap().unwrap().len(), 400);
    assert_eq!(stream.next_message().await.unwrap().unwrap().len(), 400);
    let error = stream.next_message().await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);
    assert!(error.message.contains("truncated"));
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Capacity
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        closed.load(Ordering::SeqCst),
        1,
        "the server did not see the stream close"
    );

    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, "v5.channel.k8s.io", Arc::default()).await;
        let _ = socket.send(Message::Binary(vec![1_u8; 2048].into())).await;
        let _ = tokio::time::timeout(Duration::from_secs(3), socket.next()).await;
    })
    .await;
    let mut stream = accepted(
        http(address, Arc::default())
            .into_upgrade_write(bounds(512, 4096, Duration::from_secs(5)))
            .upgrade(&path(), &[], &PROTOCOLS)
            .await,
    );
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Capacity
    );
}

/// The host holds the deadline: a server that switches protocols and then
/// sends nothing ends the stream at `timeout`, reported as `Timeout`.
#[tokio::test]
async fn the_deadline_ends_a_silent_stream() {
    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, "v5.channel.k8s.io", Arc::default()).await;
        let _ = tokio::time::timeout(Duration::from_secs(5), socket.next()).await;
    })
    .await;
    let mut stream = accepted(
        http(address, Arc::default())
            .into_upgrade_write(bounds(512, 4096, Duration::from_millis(300)))
            .upgrade(&path(), &[], &PROTOCOLS)
            .await,
    );
    let started = std::time::Instant::now();
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Timeout
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// A server that switches protocols to one this binding did not offer may
/// already have started the effect: the outcome is lost, not refused.
#[tokio::test]
async fn an_unoffered_subprotocol_is_a_lost_upgrade() {
    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, "channel.k8s.io", Arc::default()).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), socket.next()).await;
    })
    .await;
    let outcome = http(address, Arc::default())
        .into_upgrade_write(bounds(512, 4096, Duration::from_secs(5)))
        .upgrade(&path(), &[], &PROTOCOLS)
        .await;
    let Upgraded::Lost(error) = outcome else {
        panic!("an unoffered subprotocol was not a lost upgrade");
    };
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
}

/// Bounds outside their ceilings, or a message bound above the total bound,
/// are refused when composition builds them.
#[test]
fn upgrade_bounds_refuse_values_outside_their_ceilings() {
    let second = Duration::from_secs(1);
    assert!(UpgradeBounds::new(0, 10, second).is_err());
    assert!(UpgradeBounds::new(11, 10, second).is_err());
    assert!(UpgradeBounds::new(10, 10, Duration::ZERO).is_err());
    assert!(UpgradeBounds::new(10, 10, Duration::from_secs(121)).is_err());
    assert!(UpgradeBounds::new(10, 64 * 1024 * 1024 + 1, second).is_err());
    assert!(UpgradeBounds::new(10, 10, second).is_ok());
}
