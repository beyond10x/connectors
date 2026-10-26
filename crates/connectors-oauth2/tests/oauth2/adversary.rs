//! Adversary cases for `story:oauth2-grant-core`: what the crate does with server-sent device
//! intervals past the clock, what a transport's `Debug` of a request shows, what a redirect
//! receiver answers after its decision; and two lines no other case reaches (an error redirect,
//! the device code's clock).
use crate::support::{
    ACCESS_TOKEN, CLIENT_SECRET, FakeServer, NoTransport, REFRESH_TOKEN, Reply, TlsTransport,
    declaration, query_value,
};
use connectors_oauth2::{
    Cancellation, Client, ClientKind, Failure, LoopbackReceiver, RedirectKind, Request, Response,
    Secret, ThreadWaiter, Transport, TransportError,
};
use std::{
    any::Any,
    cell::RefCell,
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

const BUDGET: Duration = Duration::from_secs(20);

/// An RFC 8628 §3.2 device authorization response with the given `interval` and `expires_in`.
fn device_reply(interval: u64, expires_in: u64) -> Reply {
    Reply::json(
        200,
        serde_json::json!({
            "device_code": "fixture-device-code-adversary",
            "user_code": "FIXT-ADVR",
            "verification_uri": "https://localhost/device",
            "expires_in": expires_in,
            "interval": interval,
        }),
    )
}

/// The text a panic carried.
fn message(panic: &(dyn Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// A device authorization response whose `interval` is past what an `Instant` can hold ends the
/// grant with a failure (refused, expired or cancelled), through the crate's own
/// [`ThreadWaiter`]. It is server input: it must not panic the process that runs the grant.
#[test]
fn a_device_interval_past_the_clock_ends_the_grant_without_a_panic() {
    let server = FakeServer::start(vec![device_reply(10_000_000_000_000_000_000, u64::MAX)]);
    let client = Client::new(declaration(Some(&server)), None, None, server.transport()).unwrap();
    let cancellation = Cancellation::new();
    let cancel = cancellation.clone();
    // Should the grant wait the interval out instead, cancellation ends it.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        cancel.cancel();
    });
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let device = client.device_authorize()?;
        client
            .poll_device(&device, &ThreadWaiter::new(cancellation))
            .map(|_| ())
    }));
    match outcome {
        Ok(result) => assert!(result.is_err(), "an interval of 10^19 s granted a token"),
        Err(panic) => panic!(
            "a server-sent device interval of 10^19 s panicked the client: {}",
            message(panic.as_ref())
        ),
    }
}

/// `slow_down` on an interval near the largest `Duration` ends the grant with a failure; adding
/// RFC 8628 §3.5's 5 s must not panic the process that runs the grant.
#[test]
fn slow_down_on_a_server_interval_near_the_limit_ends_the_grant_without_a_panic() {
    let server = FakeServer::start(vec![
        device_reply(u64::MAX - 1, u64::MAX),
        Reply::error(400, "slow_down"),
        Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), Some("fixture.read")),
    ]);
    let client = Client::new(declaration(Some(&server)), None, None, server.transport()).unwrap();
    let waiter = crate::device::Recorded::default();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let device = client.device_authorize()?;
        client.poll_device(&device, &waiter).map(|_| ())
    }));
    match outcome {
        Ok(result) => assert!(
            result.is_err(),
            "an interval of u64::MAX - 1 s granted a token"
        ),
        Err(panic) => panic!(
            "slow_down on a server-sent interval of u64::MAX - 1 s panicked the client: {}",
            message(panic.as_ref())
        ),
    }
}

/// A host transport that logs every request it is handed with `{:?}`, as a `tracing` or `log`
/// call on the request would, and then sends it.
struct Logging {
    inner: TlsTransport,
    logged: RefCell<Vec<String>>,
}

impl Transport for Logging {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
        self.logged.borrow_mut().push(format!("{request:?}"));
        self.inner.post(request)
    }
}

/// `secret`'s bytes as `Debug` renders a byte slice, without the brackets: the run that the
/// rendering of any body carrying `secret` unescaped contains.
fn rendered_bytes(secret: &str) -> String {
    let rendered = format!("{:?}", secret.as_bytes());
    rendered[1..rendered.len() - 1].to_owned()
}

/// The [`Request`] the crate hands every transport carries the refresh token and the client
/// secret in its body; its `Debug` must show neither (the crate's rule: no token in any `Debug`
/// output).
#[test]
fn request_debug_shows_no_secret_of_its_body() {
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, None, None)]);
    let mut declared = declaration(Some(&server));
    declared.client_kind = ClientKind::Confidential;
    let logging = Logging {
        inner: server.transport(),
        logged: RefCell::default(),
    };
    let client = Client::new(
        declared,
        None,
        Some(Secret::new(CLIENT_SECRET.to_owned())),
        &logging,
    )
    .unwrap();
    client
        .refresh(
            &Secret::new(REFRESH_TOKEN.to_owned()),
            &["fixture.read".to_owned()],
        )
        .unwrap();
    let logged = logging.logged.borrow();
    assert_eq!(logged.len(), 1);
    for (name, secret) in [
        ("refresh token", REFRESH_TOKEN),
        ("client secret", CLIENT_SECRET),
    ] {
        assert!(
            !logged[0].contains(&rendered_bytes(secret)),
            "Debug of the request shows the {name}: {}",
            logged[0]
        );
    }
}

/// `LoopbackReceiver::receive` documents that requests completed alongside or after the
/// deciding one are answered 400. A connection accepted before the decision whose head
/// completes after it gets no answer at all.
#[test]
fn a_request_completed_after_the_decision_is_answered_400() {
    let mut declared = declaration(None);
    declared.redirect_kind = RedirectKind::LoopbackAnyPort;
    let client = Client::new(declared, None, None, NoTransport).unwrap();
    let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
    let port = receiver.local_addr().port();
    let authorization = client.begin(receiver.redirect_uri()).unwrap();
    let state = query_value(authorization.url(), "state");
    // Its request line only: the head is still open when the redirect decides.
    let mut late = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    late.write_all(format!("GET /?code=fixture-code-late&state={state} HTTP/1.1\r\n").as_bytes())
        .unwrap();
    let authorization = &authorization;
    let decided = std::thread::scope(|scope| {
        let receiving = scope.spawn(move || {
            receiver
                .receive(authorization, BUDGET, &Cancellation::new())
                .map(|code| code.expose().to_owned())
        });
        std::thread::sleep(Duration::from_millis(300));
        let mut deciding = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        deciding
            .write_all(
                format!(
                    "GET /?code=fixture-code-real&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
                )
                .as_bytes(),
            )
            .unwrap();
        let decided = receiving.join().unwrap();
        let mut answer = String::new();
        deciding
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let _ = deciding.read_to_string(&mut answer);
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer:?}");
        decided
    });
    assert_eq!(decided, Ok("fixture-code-real".to_owned()));
    let _ = late.write_all(b"Host: 127.0.0.1\r\n\r\n");
    late.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut answer = String::new();
    let _ = late.read_to_string(&mut answer);
    assert!(
        answer.starts_with("HTTP/1.1 400"),
        "a request completed after the decision was answered {answer:?}, not 400"
    );
}

/// An error redirect on the receiver's path (the person clicked "deny"): with the flow's
/// `state`, `access_denied` is [`Failure::AccessDenied`] and another error
/// [`Failure::Rejected`]; with another `state` it is [`Failure::StateMismatch`]. Each is
/// answered 400. No other case sends `error=` to a receiver.
#[test]
fn an_error_redirect_is_access_denied_only_with_the_flow_state() {
    let client = Client::new(declaration(None), None, None, NoTransport).unwrap();
    for (query, expected) in [
        ("error=access_denied&state=STATE", Failure::AccessDenied),
        ("error=server_error&state=STATE", Failure::Rejected),
        ("error=access_denied&state=forged", Failure::StateMismatch),
    ] {
        let receiver = LoopbackReceiver::bind(client.declaration()).unwrap();
        let port = receiver.local_addr().port();
        let authorization = client.begin(receiver.redirect_uri()).unwrap();
        let query = query.replace("STATE", &query_value(authorization.url(), "state"));
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .write_all(format!("GET /?{query} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
            .unwrap();
        assert_eq!(
            receiver
                .receive(&authorization, BUDGET, &Cancellation::new())
                .err(),
            Some(expected),
            "{query}"
        );
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut answer = String::new();
        let _ = stream.read_to_string(&mut answer);
        assert!(answer.starts_with("HTTP/1.1 400"), "{query}: {answer:?}");
    }
}

/// The device code's lifetime runs on the clock as well as on the waits: a device code that
/// lapsed before the first poll ends [`Failure::ExpiredToken`] without a poll. Every other case
/// waits through [`crate::device::Recorded`], which returns at once, so none reaches the clock.
#[test]
fn a_device_code_that_lapsed_before_polling_ends_without_a_poll() {
    let server = FakeServer::start(vec![
        device_reply(1, 2),
        Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), Some("fixture.read")),
    ]);
    let client = Client::new(declaration(Some(&server)), None, None, server.transport()).unwrap();
    let device = client.device_authorize().unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(
        client
            .poll_device(&device, &crate::device::Recorded::default())
            .err(),
        Some(Failure::ExpiredToken)
    );
    assert_eq!(
        server.observed().len(),
        1,
        "a lapsed device code was polled"
    );
}
