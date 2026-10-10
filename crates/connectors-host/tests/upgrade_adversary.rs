//! Adversary cases for `ScopedHttp::into_upgrade_write` (wave 20261010e),
//! driven from `ess/domains/transport.yaml` (`UpgradeBounds`,
//! `UpgradeOutcome`, `UpgradedStreamEnd`) against loopback fixtures.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{MessageStream, UpgradeBounds, Upgraded};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
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
        protocol::frame::{
            Frame,
            coding::{Data, OpCode},
        },
    },
};

const PROTOCOLS: [&str; 2] = ["v5.channel.k8s.io", "v4.channel.k8s.io"];

fn http(address: SocketAddr) -> ScopedHttp {
    ScopedHttp::new(
        &HttpConfig {
            base_url: format!("http://{address}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
    )
    .unwrap()
}
fn bounds(message: usize, total: usize, timeout: Duration) -> UpgradeBounds {
    UpgradeBounds::new(message, total, timeout).unwrap()
}
const PATH: [&str; 7] = [
    "api",
    "v1",
    "namespaces",
    "fixture",
    "pods",
    "api-0",
    "exec",
];

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

struct Handshake(Arc<Mutex<Vec<String>>>);
impl Callback for Handshake {
    fn on_request(
        self,
        request: &Request,
        mut response: Response,
    ) -> std::result::Result<Response, ErrorResponse> {
        self.0.lock().unwrap().push(request.uri().to_string());
        response.headers_mut().insert(
            "sec-websocket-protocol",
            "v5.channel.k8s.io".parse().unwrap(),
        );
        Ok(response)
    }
}
async fn accept(stream: TcpStream, seen: Arc<Mutex<Vec<String>>>) -> WebSocketStream<TcpStream> {
    tokio_tungstenite::accept_hdr_async(stream, Handshake(seen))
        .await
        .unwrap()
}
fn accepted(outcome: Upgraded) -> Box<dyn MessageStream> {
    match outcome {
        Upgraded::Accepted(stream) => stream,
        Upgraded::NotSent(error) => panic!("not sent: {error:?}"),
        Upgraded::Answered(status) => panic!("answered {status}"),
        Upgraded::Lost(error) => panic!("lost: {error:?}"),
    }
}

/// Every argument reaches the server as its own `command` term, unchanged,
/// and no value can add a query term, a path segment or a fragment.
#[tokio::test]
async fn upgrade_query_values_reach_the_server_as_separate_unchanged_terms() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = seen.clone();
    let (address, _) = listener(move |stream| {
        let seen = recorded.clone();
        async move {
            let mut socket = accept(stream, seen).await;
            let _ = socket.close(None).await;
        }
    })
    .await;
    let argv = [
        "sh",
        "a b; rm -rf /",
        "line\nnext\r\n",
        "&command=evil&tty=true",
        "#fragment",
        "%2F..%2F",
        "",
        "\u{fc}n\u{ef}",
        "+=?/",
    ];
    let mut query: Vec<(&str, String)> = vec![("container", "api".into())];
    query.extend(argv.iter().map(|a| ("command", (*a).to_owned())));
    let mut stream = accepted(
        http(address)
            .into_upgrade_write(bounds(64, 256, Duration::from_secs(5)))
            .upgrade(&PATH, &query, &PROTOCOLS)
            .await,
    );
    let _ = stream.next_message().await;
    let uri = seen.lock().unwrap().pop().unwrap();
    let url = reqwest::Url::parse(&format!("http://fixture{uri}")).unwrap();
    assert_eq!(url.path(), "/api/v1/namespaces/fixture/pods/api-0/exec");
    assert_eq!(url.fragment(), None);
    let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let expected: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    assert_eq!(pairs, expected);
}

/// A segment carrying `/`, `?`, `#` or an encoded dot-segment stays one
/// segment; nothing escapes the configured base.
#[tokio::test]
async fn upgrade_segments_cannot_escape_or_inject() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = seen.clone();
    let (address, _) = listener(move |stream| {
        let seen = recorded.clone();
        async move {
            let mut socket = accept(stream, seen).await;
            let _ = socket.close(None).await;
        }
    })
    .await;
    for segment in ["a/../../b", "%2e%2e", "x?command=sh", "y#z", "..%2F"] {
        let mut stream = accepted(
            http(address)
                .into_upgrade_write(bounds(64, 256, Duration::from_secs(5)))
                .upgrade(&["api", segment, "exec"], &[], &PROTOCOLS)
                .await,
        );
        let _ = stream.next_message().await;
        let uri = seen.lock().unwrap().pop().unwrap();
        let url = reqwest::Url::parse(&format!("http://fixture{uri}")).unwrap();
        let segments: Vec<&str> = url.path_segments().unwrap().collect();
        assert_eq!(segments.len(), 3, "{uri}");
        assert_eq!(segments[0], "api");
        assert_eq!(segments[2], "exec");
        assert!(url.query().is_none_or(str::is_empty), "{uri}");
        assert_eq!(url.fragment(), None, "{uri}");
    }
}

/// The deadline holds over the HTTP exchange: a server that reads the
/// upgrade request and never answers ends at the bound, lost, not after the
/// client's own 15 s timeout.
#[tokio::test]
async fn upgrade_deadline_ends_a_server_that_never_answers() {
    let (address, _) = listener(|mut stream| async move {
        let mut buffer = vec![0_u8; 8192];
        let _ = stream.read(&mut buffer).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    })
    .await;
    let started = Instant::now();
    let outcome = http(address)
        .into_upgrade_write(bounds(64, 256, Duration::from_millis(400)))
        .upgrade(&PATH, &[], &PROTOCOLS)
        .await;
    let Upgraded::Lost(error) = outcome else {
        panic!("a request that was sent and never answered was not lost");
    };
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

/// Slow drip: pings and one-byte messages that each arrive well inside any
/// per-message wait never extend the whole-stream deadline.
#[tokio::test]
async fn upgrade_deadline_ends_a_slow_drip_stream() {
    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, Arc::default()).await;
        for index in 0..200_u32 {
            let message = if index % 2 == 0 {
                Message::Ping(vec![1].into())
            } else {
                Message::Binary(vec![1].into())
            };
            if socket.send(message).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    })
    .await;
    let mut stream = accepted(
        http(address)
            .into_upgrade_write(bounds(64, 4096, Duration::from_millis(500)))
            .upgrade(&PATH, &[], &PROTOCOLS)
            .await,
    );
    let started = Instant::now();
    let error = loop {
        match stream.next_message().await {
            Ok(Some(_)) => continue,
            Ok(None) => panic!("a slow-drip stream ended as a clean close"),
            Err(error) => break error,
        }
    };
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    // The end repeats; nothing more is delivered.
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Timeout
    );
}

/// A message assembled from fragments, each under the bound, larger in all
/// than `max_message_bytes`, is truncation; nothing after it is delivered.
#[tokio::test]
async fn upgrade_fragmented_message_over_the_message_bound_is_truncation() {
    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, Arc::default()).await;
        let chunk = vec![7_u8; 40];
        let _ = socket
            .send(Message::Frame(Frame::message(
                chunk.clone(),
                OpCode::Data(Data::Binary),
                false,
            )))
            .await;
        let _ = socket
            .send(Message::Frame(Frame::message(
                chunk.clone(),
                OpCode::Data(Data::Continue),
                false,
            )))
            .await;
        let _ = socket
            .send(Message::Frame(Frame::message(
                chunk,
                OpCode::Data(Data::Continue),
                true,
            )))
            .await;
        let _ = socket.send(Message::Binary(vec![1].into())).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), socket.next()).await;
    })
    .await;
    let mut stream = accepted(
        http(address)
            .into_upgrade_write(bounds(64, 4096, Duration::from_secs(5)))
            .upgrade(&PATH, &[], &PROTOCOLS)
            .await,
    );
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Capacity
    );
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Capacity
    );
}

/// The total bound counts every message: two that fit alone but not
/// together end the stream truncated at the second, which is not delivered.
#[tokio::test]
async fn upgrade_total_bound_counts_across_messages() {
    let (address, _) = listener(|stream| async move {
        let mut socket = accept(stream, Arc::default()).await;
        let _ = socket.send(Message::Binary(vec![1; 6].into())).await;
        let _ = socket.send(Message::Binary(vec![2; 6].into())).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), socket.next()).await;
    })
    .await;
    let mut stream = accepted(
        http(address)
            .into_upgrade_write(bounds(8, 10, Duration::from_secs(5)))
            .upgrade(&PATH, &[], &PROTOCOLS)
            .await,
    );
    assert_eq!(stream.next_message().await.unwrap(), Some(vec![1; 6]));
    assert_eq!(
        stream.next_message().await.unwrap_err().code,
        ErrorCode::Capacity
    );
}

/// A raw 101 that does not verify (two subprotocols selected, a missing
/// Upgrade header, a wrong accept key) is a lost upgrade, never accepted.
#[tokio::test]
async fn upgrade_unverified_switch_is_lost() {
    for variant in 0..3_u8 {
        let (address, _) = listener(move |mut stream| async move {
            let mut buffer = vec![0_u8; 8192];
            let read = stream.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let key = request
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("sec-websocket-key")
                        .then(|| value.trim().to_owned())
                })
                .unwrap_or_default();
            let accept =
                tokio_tungstenite::tungstenite::handshake::derive_accept_key(key.as_bytes());
            let answer = match variant {
                0 => format!(
                    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: v5.channel.k8s.io, v4.channel.k8s.io\r\n\r\n"
                ),
                1 => format!(
                    "HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: v5.channel.k8s.io\r\n\r\n"
                ),
                _ => "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: AAAAAAAAAAAAAAAAAAAAAAAAAAA=\r\nSec-WebSocket-Protocol: v5.channel.k8s.io\r\n\r\n".to_owned(),
            };
            let _ = stream.write_all(answer.as_bytes()).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        })
        .await;
        let outcome = http(address)
            .into_upgrade_write(bounds(64, 256, Duration::from_secs(5)))
            .upgrade(&PATH, &[], &PROTOCOLS)
            .await;
        let Upgraded::Lost(error) = outcome else {
            panic!("variant {variant}: an unverified 101 was not lost");
        };
        assert_eq!(error.code, ErrorCode::UpstreamProtocol, "variant {variant}");
    }
}

/// A connection refused before any byte was sent is `not_sent`; the
/// subprotocol list must be tokens, refused with no I/O.
#[tokio::test]
async fn upgrade_refusals_before_sending_are_not_sent() {
    let unused = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = unused.local_addr().unwrap();
    drop(unused);
    let outcome = http(address)
        .into_upgrade_write(bounds(64, 256, Duration::from_secs(5)))
        .upgrade(&PATH, &[], &PROTOCOLS)
        .await;
    assert!(
        matches!(outcome, Upgraded::NotSent(_)),
        "connect refusal was not not_sent"
    );
    let (address, connections) = listener(|_stream| async {}).await;
    for protocols in [
        &[][..],
        &["v5 channel"][..],
        &["a,b"][..],
        &["x\r\nX-Injected: 1"][..],
    ] {
        let outcome = http(address)
            .into_upgrade_write(bounds(64, 256, Duration::from_secs(5)))
            .upgrade(&PATH, &[], protocols)
            .await;
        assert!(matches!(outcome, Upgraded::NotSent(_)), "{protocols:?}");
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(connections.load(Ordering::SeqCst), 0);
}
