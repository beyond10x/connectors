//! The RFC 8628 device grant, and a non-RFC device dialect through its test double.
use crate::support::{ACCESS_TOKEN, CLIENT_ID, FakeServer, REFRESH_TOKEN, Reply, declaration};
use connectors_oauth2::{
    Cancellation, Client, DeviceDialect, DialectContext, DialectGrant, Failure, RESPONSE_LIMIT,
    Request, Secret, ThreadWaiter, Waiter,
};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

const DEVICE_CODE: &str = "fixture-device-code-one";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// A waiter that records every wait and returns at once.
#[derive(Default)]
pub struct Recorded(RefCell<Vec<Duration>>);

impl Recorded {
    fn seconds(&self) -> Vec<u64> {
        self.0.borrow().iter().map(Duration::as_secs).collect()
    }
}

impl Waiter for Recorded {
    fn wait(&self, duration: Duration) -> Result<(), Failure> {
        self.0.borrow_mut().push(duration);
        Ok(())
    }
}

fn device_reply(interval: Option<u64>, expires_in: u64) -> Reply {
    let mut body = serde_json::json!({
        "device_code": DEVICE_CODE,
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://localhost/device",
        "verification_uri_complete": "https://localhost/device?user_code=WDJB-MJHT",
        "expires_in": expires_in,
    });
    if let Some(interval) = interval {
        body["interval"] = interval.into();
    }
    Reply::json(200, body)
}

fn token() -> Reply {
    Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), Some("fixture.read"))
}

fn client(server: &FakeServer) -> Client<crate::support::TlsTransport> {
    Client::new(declaration(Some(server)), None, None, server.transport()).unwrap()
}

/// `authorization_pending` waits one interval (5 s when the server names none); `slow_down`
/// adds 5 s to every later interval.
#[test]
fn device_grant_waits_on_pending_and_slows_down_by_five_seconds() {
    let server = FakeServer::start(vec![
        device_reply(None, 600),
        Reply::error(400, "authorization_pending"),
        Reply::error(400, "slow_down"),
        Reply::error(400, "authorization_pending"),
        token(),
    ]);
    let client = client(&server);
    let device = client.device_authorize().unwrap();
    assert_eq!(device.user_code, "WDJB-MJHT");
    assert_eq!(device.verification_uri, "https://localhost/device");
    assert_eq!(device.interval, Duration::from_secs(5));
    assert_eq!(device.expires_in, Duration::from_secs(600));
    assert!(!format!("{device:?}").contains(DEVICE_CODE));
    let waiter = Recorded::default();
    let granted = client.poll_device(&device, &waiter).unwrap();
    assert_eq!(granted.access_token.expose(), ACCESS_TOKEN);
    assert_eq!(granted.scopes, ["fixture.read"]);
    assert_eq!(waiter.seconds(), [5, 5, 10, 10]);
    let observed = server.observed();
    assert_eq!(observed.len(), 5);
    assert_eq!(observed[0].path, "/device");
    let start = observed[0].fields();
    assert_eq!(start["client_id"], CLIENT_ID);
    assert_eq!(start["scope"], "fixture.read fixture.write");
    for poll in &observed[1..] {
        assert_eq!(poll.path, "/token");
        let fields = poll.fields();
        assert_eq!(fields["grant_type"], DEVICE_GRANT);
        assert_eq!(fields["device_code"], DEVICE_CODE);
        assert_eq!(fields["client_id"], CLIENT_ID);
    }
    // A server-named interval is kept.
    let server = FakeServer::start(vec![
        device_reply(Some(2), 600),
        Reply::error(400, "slow_down"),
        token(),
    ]);
    let client = self::client(&server);
    let device = client.device_authorize().unwrap();
    let waiter = Recorded::default();
    client.poll_device(&device, &waiter).unwrap();
    assert_eq!(waiter.seconds(), [2, 7]);
}

#[test]
fn device_grant_expired_token_ends() {
    let server = FakeServer::start(vec![
        device_reply(Some(1), 600),
        Reply::error(400, "authorization_pending"),
        Reply::error(400, "expired_token"),
        token(),
    ]);
    let client = client(&server);
    let device = client.device_authorize().unwrap();
    assert_eq!(
        client.poll_device(&device, &Recorded::default()).err(),
        Some(Failure::ExpiredToken)
    );
    assert_eq!(server.observed().len(), 3, "no poll after expired_token");
    // The device code's own lifetime ends the grant without another poll.
    let server = FakeServer::start(vec![
        device_reply(Some(5), 12),
        Reply::error(400, "authorization_pending"),
        Reply::error(400, "authorization_pending"),
        token(),
    ]);
    let client = self::client(&server);
    let device = client.device_authorize().unwrap();
    let waiter = Recorded::default();
    assert_eq!(
        client.poll_device(&device, &waiter).err(),
        Some(Failure::ExpiredToken)
    );
    assert_eq!(server.observed().len(), 3, "no poll past expires_in");
}

#[test]
fn device_grant_access_denied_ends() {
    let server = FakeServer::start(vec![
        device_reply(Some(1), 600),
        Reply::error(400, "authorization_pending"),
        Reply::error(400, "access_denied"),
        token(),
    ]);
    let client = client(&server);
    let device = client.device_authorize().unwrap();
    assert_eq!(
        client.poll_device(&device, &Recorded::default()).err(),
        Some(Failure::AccessDenied)
    );
    assert_eq!(server.observed().len(), 3, "no poll after access_denied");
}

#[test]
fn device_grant_cancellation_ends_within_one_interval() {
    const INTERVAL: Duration = Duration::from_secs(2);
    let mut replies = vec![device_reply(Some(INTERVAL.as_secs()), 600)];
    replies.extend((0..20).map(|_| Reply::error(400, "authorization_pending")));
    let server = FakeServer::start(replies);
    let client = client(&server);
    let device = client.device_authorize().unwrap();
    // Cancelled before the first wait: no poll at all.
    let cancellation = Cancellation::new();
    cancellation.cancel();
    assert_eq!(
        client
            .poll_device(&device, &ThreadWaiter::new(cancellation))
            .err(),
        Some(Failure::Cancelled)
    );
    assert_eq!(server.observed().len(), 1);
    // Cancelled during the second interval.
    let cancellation = Cancellation::new();
    let cancel = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(INTERVAL + INTERVAL / 4);
        cancel.cancel();
        Instant::now()
    });
    let outcome = client.poll_device(&device, &ThreadWaiter::new(cancellation));
    let returned = Instant::now();
    let cancelled_at = canceller.join().unwrap();
    assert_eq!(outcome.err(), Some(Failure::Cancelled));
    assert!(
        returned.duration_since(cancelled_at) < INTERVAL,
        "returned {:?} after cancellation",
        returned.duration_since(cancelled_at)
    );
    assert_eq!(server.observed().len(), 2, "one poll before cancellation");
}

/// The test double of a non-RFC device dialect: it starts the provider's own flow through the
/// client's transport and returns the code and verifier it was handed.
struct FixtureDialect {
    start_url: String,
    calls: RefCell<u32>,
}

impl DeviceDialect for FixtureDialect {
    fn authorize(&self, context: &DialectContext<'_>) -> Result<DialectGrant, Failure> {
        *self.calls.borrow_mut() += 1;
        assert_eq!(context.client_id, CLIENT_ID);
        context.waiter.wait(Duration::from_secs(3))?;
        let body = serde_json::to_vec(&serde_json::json!({"client_id": context.client_id}))
            .map_err(|_| Failure::Unavailable)?;
        let response = context
            .transport
            .post(&Request {
                url: &self.start_url,
                content_type: "application/json",
                body: &body,
                limit: RESPONSE_LIMIT,
            })
            .map_err(|_| Failure::Unavailable)?;
        let answer: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|_| Failure::InvalidResponse)?;
        Ok(DialectGrant {
            code: Secret::new(answer["authorization_code"].as_str().unwrap().to_owned()),
            verifier: Secret::new(answer["code_verifier"].as_str().unwrap().to_owned()),
            redirect_uri: "https://localhost/dialect/callback".to_owned(),
        })
    }
}

#[test]
fn device_dialect_code_enters_the_ordinary_exchange() {
    let server = FakeServer::start(vec![
        Reply::json(
            200,
            serde_json::json!({
                "authorization_code": "fixture-dialect-code",
                "code_verifier": "fixture-dialect-verifier-0123456789-0123456789",
            }),
        ),
        token(),
    ]);
    let client = client(&server);
    let dialect = FixtureDialect {
        start_url: server.url("/dialect/start"),
        calls: RefCell::new(0),
    };
    let waiter = Recorded::default();
    let granted = client.device_dialect(&dialect, &waiter).unwrap();
    assert_eq!(granted.access_token.expose(), ACCESS_TOKEN);
    assert_eq!(*dialect.calls.borrow(), 1);
    assert_eq!(waiter.seconds(), [3]);
    let observed = server.observed();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[1].path, "/token");
    let fields = observed[1].fields();
    assert_eq!(fields["grant_type"], "authorization_code");
    assert_eq!(fields["code"], "fixture-dialect-code");
    assert_eq!(
        fields["code_verifier"],
        "fixture-dialect-verifier-0123456789-0123456789"
    );
    assert_eq!(fields["redirect_uri"], "https://localhost/dialect/callback");
    assert_eq!(fields["client_id"], CLIENT_ID);
    assert!(!fields.contains_key("state"));
}
