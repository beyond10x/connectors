//! The redirect receivers: the loopback listeners and the pasted `code#state` line.
use crate::support::{NoTransport, declaration, query_value};
use connectors_oauth2::{
    Cancellation, Client, Failure, LoopbackReceiver, RedirectKind, pasted_code,
};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    time::Duration,
};

const BUDGET: Duration = Duration::from_secs(20);

/// Opens a connection to the receiver and sends a complete `GET target` head.
fn get(port: u16, target: &str) -> TcpStream {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes())
        .unwrap();
    stream
}

/// The status code the receiver answered a connection with.
fn status(mut stream: TcpStream) -> u16 {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut answer = String::new();
    let _ = stream.read_to_string(&mut answer);
    answer
        .split_whitespace()
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("no HTTP answer: {answer:?}"))
}

fn client(kind: RedirectKind, redirect_uri: Option<String>) -> Client<NoTransport> {
    let mut declared = declaration(None);
    declared.redirect_kind = kind;
    declared.redirect_uri = redirect_uri;
    Client::new(declared, None, None, NoTransport).unwrap()
}

/// A port nothing listens on now.
fn free_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn fixed_port_receiver_answers_the_declared_path_only() {
    let port = free_port();
    let client = client(
        RedirectKind::LoopbackFixedPort,
        Some(format!("http://127.0.0.1:{port}/fixture/callback")),
    );
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    assert_eq!(receiver.local_addr().port(), port);
    assert!(receiver.local_addr().ip().is_loopback());
    assert_eq!(
        receiver.redirect_uri(),
        format!("http://127.0.0.1:{port}/fixture/callback")
    );
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    let elsewhere = [
        get(port, &format!("/?code=fixture-code-root&state={state}")),
        get(
            port,
            &format!("/fixture?code=fixture-code-other&state={state}"),
        ),
        get(
            port,
            &format!("/fixture/callback/x?code=fixture-code-deeper&state={state}"),
        ),
    ];
    let declared = get(
        port,
        &format!("/fixture/callback?code=fixture-code-declared&state={state}"),
    );
    let code = receiver
        .receive(&authorization, BUDGET, &Cancellation::new())
        .unwrap();
    assert_eq!(code.expose(), "fixture-code-declared");
    for stream in elsewhere {
        assert_eq!(status(stream), 404, "a request off the declared path");
    }
    assert_eq!(status(declared), 200);
    assert!(!format!("{code:?}").contains("fixture-code-declared"));
}

#[test]
fn any_port_receiver_binds_loopback() {
    // Without a template the redirect is `http://127.0.0.1:<bound port>/`.
    let plain = client(RedirectKind::LoopbackAnyPort, None);
    let receiver = LoopbackReceiver::bind(plain.declaration()).unwrap();
    let address = receiver.local_addr();
    assert_eq!(address.ip(), Ipv4Addr::LOCALHOST);
    assert_ne!(address.port(), 0);
    assert_eq!(
        receiver.redirect_uri(),
        format!("http://127.0.0.1:{}/", address.port())
    );
    // A template keeps its host and path and takes the bound port.
    let templated = client(
        RedirectKind::LoopbackAnyPort,
        Some("http://localhost:1455/auth/callback".to_owned()),
    );
    let receiver = LoopbackReceiver::bind(templated.declaration()).unwrap();
    let port = receiver.local_addr().port();
    assert!(receiver.local_addr().ip().is_loopback());
    assert_eq!(
        receiver.redirect_uri(),
        format!("http://localhost:{port}/auth/callback")
    );
    let authorization = templated.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    let answered = get(
        port,
        &format!("/auth/callback?code=fixture-code-any&state={state}"),
    );
    let code = receiver
        .receive(&authorization, BUDGET, &Cancellation::new())
        .unwrap();
    assert_eq!(code.expose(), "fixture-code-any");
    assert_eq!(status(answered), 200);
    // A template that is not loopback HTTP is refused.
    for template in [
        "http://192.0.2.1:1/cb",
        "https://127.0.0.1:1/cb",
        "http://example.test/cb",
    ] {
        let mut declared = declaration(None);
        declared.redirect_uri = Some(template.to_owned());
        assert_eq!(
            LoopbackReceiver::bind(&declared).err(),
            Some(Failure::InvalidDeclaration),
            "{template}"
        );
    }
}

#[test]
fn pasted_code_with_wrong_state_is_refused() {
    let client = client(
        RedirectKind::PastedCode,
        Some("https://localhost:1/code/callback".to_owned()),
    );
    let authorization = client.begin("https://localhost:1/code/callback").unwrap();
    let state = query_value(authorization.url(), "state");
    let mut wrong = state.clone();
    wrong.replace_range(0..1, if wrong.starts_with('A') { "B" } else { "A" });
    for line in [
        format!("fixture-code#{wrong}"),
        format!("fixture-code#{state}x"),
        "fixture-code".to_owned(),
        "fixture-code#".to_owned(),
    ] {
        assert_eq!(
            pasted_code(&authorization, &line).err(),
            Some(Failure::StateMismatch),
            "{line}"
        );
    }
    let code = pasted_code(&authorization, &format!("  fixture-code#{state}\r\n")).unwrap();
    assert_eq!(code.expose(), "fixture-code");
}

/// The first request on the redirect path decides: a later one, right or wrong, is answered
/// 400 and changes nothing.
#[test]
fn second_request_does_not_change_the_outcome() {
    let client = client(RedirectKind::LoopbackAnyPort, None);
    // A wrong state first: the right one after it does not rescue the flow.
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let port = receiver.local_addr().port();
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    let first = get(port, "/?code=fixture-code-forged&state=forged-state");
    let second = get(port, &format!("/?code=fixture-code-real&state={state}"));
    assert_eq!(
        receiver
            .receive(&authorization, BUDGET, &Cancellation::new())
            .err(),
        Some(Failure::StateMismatch)
    );
    assert_eq!(status(first), 400);
    assert_eq!(status(second), 400);
    // The right state first: a forged one after it does not replace the code.
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let port = receiver.local_addr().port();
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    let first = get(port, &format!("/?code=fixture-code-real&state={state}"));
    let second = get(port, &format!("/?code=fixture-code-forged&state={state}"));
    let code = receiver
        .receive(&authorization, BUDGET, &Cancellation::new())
        .unwrap();
    assert_eq!(code.expose(), "fixture-code-real");
    assert_eq!(status(first), 200);
    assert_eq!(status(second), 400);
}

/// Everything a connection sent until the receiver closed it, or `None` when it is still open
/// after `wait`.
fn closed_within(mut stream: TcpStream, wait: Duration) -> Option<Vec<u8>> {
    stream.set_read_timeout(Some(wait)).unwrap();
    let mut received = Vec::new();
    match stream.read_to_end(&mut received) {
        Ok(_) => Some(received),
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => Some(received),
        Err(_) => None,
    }
}

/// A head over 64 KiB, and a head left incomplete for 2 s, are dropped unanswered and decide
/// nothing, even with the flow's own `state`; the redirect after them still decides.
#[test]
fn an_oversized_or_idle_head_decides_nothing() {
    let client = client(RedirectKind::LoopbackAnyPort, None);
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let port = receiver.local_addr().port();
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    let (outcome_tx, outcome) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let code = receiver
                .receive(&authorization, BUDGET, &Cancellation::new())
                .map(|code| code.expose().to_owned());
            outcome_tx.send(code).unwrap();
        });
        let mut oversized = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        let mut head = format!("GET /?code=fixture-code-big&state={state} HTTP/1.1\r\nX-Fill: ");
        head.push_str(&"a".repeat(64 * 1024));
        head.push_str("\r\n\r\n");
        let _ = oversized.write_all(head.as_bytes());
        let mut idle = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        idle.write_all(
            format!("GET /?code=fixture-code-idle&state={state} HTTP/1.1\r\n").as_bytes(),
        )
        .unwrap();
        let started = std::time::Instant::now();
        let dropped = closed_within(idle, Duration::from_secs(10)).expect("idle head closed");
        assert!(dropped.is_empty(), "an idle head was answered");
        let after = started.elapsed();
        assert!(
            after >= Duration::from_millis(1500) && after < Duration::from_secs(5),
            "idle head closed after {after:?}"
        );
        let dropped = closed_within(oversized, Duration::from_secs(5)).expect("oversized closed");
        assert!(dropped.is_empty(), "an oversized head was answered");
        assert!(
            outcome.try_recv().is_err(),
            "a dropped head decided the flow"
        );
        let decides = get(port, &format!("/?code=fixture-code-late&state={state}"));
        assert_eq!(status(decides), 200);
    });
    assert_eq!(outcome.recv().unwrap().unwrap(), "fixture-code-late");
}

#[test]
fn timeout_and_cancellation_end_the_receive() {
    let client = client(RedirectKind::LoopbackAnyPort, None);
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        receiver
            .receive(
                &authorization,
                Duration::from_millis(200),
                &Cancellation::new()
            )
            .err(),
        Some(Failure::Timeout)
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let cancellation = Cancellation::new();
    cancellation.cancel();
    assert_eq!(
        receiver
            .receive(&authorization, BUDGET, &cancellation)
            .err(),
        Some(Failure::Cancelled)
    );
}
