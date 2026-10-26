//! The ChatGPT sign-in's device flow (story:chatgpt-subscription-connector): the
//! Codex CLI's own device authorization, not RFC 8628, as the
//! [`DeviceDialect`] of the `chatgpt.subscription` profile.
//!
//! Its endpoints sit on the issuer, the origin of the profile's `token_url`
//! (`https://auth.openai.com`), as the Codex CLI places them
//! (`openai/codex@970111f6cb62cc02fe36a82ea7d71b37a2889c97`,
//! `codex-rs/login/src/device_code_auth.rs#L66-L218`):
//!
//! 1. `POST {issuer}/api/accounts/deviceauth/usercode` with the JSON
//!    `{"client_id"}` answers `device_auth_id`, `user_code` and `interval`;
//! 2. the person enters the user code at `{issuer}/codex/device`;
//! 3. `POST {issuer}/api/accounts/deviceauth/token` with the JSON
//!    `{"device_auth_id","user_code"}`, every interval, answers 403 or 404
//!    while the person has not decided, for at most 15 minutes, then
//!    `authorization_code` and `code_verifier`;
//! 4. the core exchanges that code at the token endpoint with the redirect URI
//!    `{issuer}/deviceauth/callback`, like any other code.
//!
//! The device id, the code and the verifier are [`Secret`]s; a failure is one
//! [`Failure`] code and carries no text the server wrote.
use connectors_oauth2::{
    DeviceDialect, DialectContext, DialectGrant, Failure, RESPONSE_LIMIT, Request, Secret,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{io::Write as _, time::Duration};
use zeroize::Zeroizing;

/// The profile of the `chatgpt` connector this dialect runs for.
pub const PROFILE: &str = "chatgpt.subscription";
/// Where the user code is entered, on the issuer.
pub const DEVICE_PAGE: &str = "/codex/device";
const USER_CODE: &str = "/api/accounts/deviceauth/usercode";
const POLL: &str = "/api/accounts/deviceauth/token";
const CALLBACK: &str = "/deviceauth/callback";
/// How long the Codex CLI waits for the person (`device_code_auth.rs`).
const MAX_WAIT: Duration = Duration::from_secs(15 * 60);
/// The polling interval when the server names none, or zero.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);
/// The longest polling interval a server may name; a longer one is refused.
const MAX_INTERVAL: u64 = 600;
/// The longest user code shown.
const USER_CODE_LIMIT: usize = 64;

/// Shows the user code and the page to enter it at.
pub type Show = Box<dyn Fn(&str, &str)>;

/// The ChatGPT device flow. It shows its own prompt: the user code and the
/// page to enter it at.
pub struct CodexDevice {
    show: Show,
}

impl Default for CodexDevice {
    fn default() -> Self {
        Self::showing(Box::new(present))
    }
}

impl CodexDevice {
    /// A dialect that hands the user code and the page to `show`.
    pub fn showing(show: Show) -> Self {
        Self { show }
    }
}

impl DeviceDialect for CodexDevice {
    fn authorize(&self, context: &DialectContext<'_>) -> Result<DialectGrant, Failure> {
        let issuer =
            crate::origin(&context.declaration.token_url).ok_or(Failure::InvalidDeclaration)?;
        let started: Started = post(
            context,
            &format!("{issuer}{USER_CODE}"),
            &StartBody {
                client_id: context.client_id,
            },
            false,
        )?
        .ok_or(Failure::Rejected)?;
        let user_code = started.user_code.expose();
        if !is_token(started.device_auth_id.expose()) || !is_shown(user_code) {
            return Err(Failure::InvalidResponse);
        }
        let interval = started.interval.0;
        (self.show)(user_code, &format!("{issuer}{DEVICE_PAGE}"));
        let poll = PollBody {
            device_auth_id: started.device_auth_id.expose(),
            user_code,
        };
        let poll_url = format!("{issuer}{POLL}");
        let mut waited = Duration::ZERO;
        loop {
            waited += interval;
            if waited > MAX_WAIT {
                return Err(Failure::ExpiredToken);
            }
            context.waiter.wait(interval)?;
            let Some(granted) = post::<Granted>(context, &poll_url, &poll, true)? else {
                continue;
            };
            if !is_token(granted.authorization_code.expose())
                || !is_token(granted.code_verifier.expose())
            {
                return Err(Failure::InvalidResponse);
            }
            return Ok(DialectGrant {
                code: granted.authorization_code,
                verifier: granted.code_verifier,
                redirect_uri: format!("{issuer}{CALLBACK}"),
            });
        }
    }
}

#[derive(Serialize)]
struct StartBody<'a> {
    client_id: &'a str,
}

#[derive(Serialize)]
struct PollBody<'a> {
    device_auth_id: &'a str,
    user_code: &'a str,
}

/// The answer to the user-code request.
#[derive(Deserialize)]
struct Started {
    device_auth_id: Secret,
    #[serde(alias = "usercode")]
    user_code: Secret,
    #[serde(default)]
    interval: Interval,
}

/// The answer to a poll the person has decided.
#[derive(Deserialize)]
struct Granted {
    authorization_code: Secret,
    code_verifier: Secret,
}

/// The polling interval: seconds as a string, as the Codex CLI reads it, or as
/// a number; absent, null or zero is [`DEFAULT_INTERVAL`], and more than
/// [`MAX_INTERVAL`] seconds is refused.
struct Interval(Duration);

impl Default for Interval {
    fn default() -> Self {
        Self(DEFAULT_INTERVAL)
    }
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Number(u64),
            Text(String),
        }
        let seconds = match Option::<Written>::deserialize(deserializer)? {
            None => 0,
            Some(Written::Number(seconds)) => seconds,
            Some(Written::Text(text)) => text
                .trim()
                .parse()
                .map_err(|_| serde::de::Error::custom("interval"))?,
        };
        match seconds {
            0 => Ok(Self::default()),
            seconds if seconds <= MAX_INTERVAL => Ok(Self(Duration::from_secs(seconds))),
            _ => Err(serde::de::Error::custom("interval")),
        }
    }
}

/// Posts `body` as JSON to `url` and reads the 2xx answer as `T`. `None` is a
/// pending poll, a 403 or 404 when `polling`; any other status is the
/// [`Failure`] it means, and the server's own text is dropped.
fn post<T: for<'de> Deserialize<'de>>(
    context: &DialectContext<'_>,
    url: &str,
    body: &impl Serialize,
    polling: bool,
) -> Result<Option<T>, Failure> {
    let body = Zeroizing::new(serde_json::to_vec(body).map_err(|_| Failure::InvalidResponse)?);
    let response = context
        .transport
        .post(&Request {
            url,
            content_type: "application/json",
            body: &body,
            limit: RESPONSE_LIMIT,
        })
        .map_err(|_| Failure::Unavailable)?;
    if response.body.len() > RESPONSE_LIMIT {
        return Err(Failure::InvalidResponse);
    }
    match response.status {
        200..=299 => serde_json::from_slice(&response.body)
            .map(Some)
            .map_err(|_| Failure::InvalidResponse),
        403 | 404 if polling => Ok(None),
        429 => Err(Failure::RateLimited {
            retry_after: response
                .retry_after
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|value| value.parse().ok())
                .map(Duration::from_secs),
        }),
        500..=599 => Err(Failure::Unavailable),
        _ => Err(Failure::Rejected),
    }
}

/// A value sent back as a token: not empty, and visible ASCII only.
fn is_token(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())
}

/// A user code shown on one line: not empty, at most [`USER_CODE_LIMIT`]
/// bytes, and visible ASCII or spaces.
fn is_shown(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= USER_CODE_LIMIT
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

/// Shows the prompt where the person running the CLI sees it now: on the
/// terminal, or without one as the single standard-error line
/// `connectors: device-code <user_code> <page>`, as the RFC 8628 grant does.
fn present(user_code: &str, page: &str) {
    let shown = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .and_then(|mut tty| {
            tty.write_all(
                format!("Open {page} and enter the code {user_code} (ChatGPT device sign-in):\n")
                    .as_bytes(),
            )
        });
    if shown.is_err() {
        let _ = std::io::stderr()
            .lock()
            .write_all(format!("connectors: device-code {user_code} {page}\n").as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors_oauth2::{Response, ServerDeclaration, Transport, TransportError, Waiter};
    use serde_json::{Value, json};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    const ISSUER: &str = "https://localhost:4443";

    /// One request the fake server was sent: its URL, content type and body.
    #[derive(Clone, Debug)]
    struct Sent {
        url: String,
        content_type: String,
        body: Value,
    }

    /// The device endpoints of a fake issuer: `pending` polls answered with
    /// `pending_status`, then the code; `user_code` answered with `start`.
    struct Fake {
        sent: RefCell<Vec<Sent>>,
        start: (u16, Value),
        pending: Cell<u32>,
        pending_status: u16,
        granted: (u16, Value),
    }

    impl Fake {
        fn new(pending: u32) -> Self {
            Self {
                sent: RefCell::default(),
                start: (
                    200,
                    json!({
                        "device_auth_id": "fixture-device-auth-1",
                        "user_code": "FIXT-0001",
                        "interval": "3",
                    }),
                ),
                pending: Cell::new(pending),
                pending_status: 403,
                granted: (
                    200,
                    json!({
                        "authorization_code": "fixture-device-code-1",
                        "code_challenge": "fixture-challenge",
                        "code_verifier": "fixture-device-verifier-1",
                    }),
                ),
            }
        }
    }

    impl Transport for Fake {
        fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
            self.sent.borrow_mut().push(Sent {
                url: request.url.to_owned(),
                content_type: request.content_type.to_owned(),
                body: serde_json::from_slice(request.body).unwrap(),
            });
            let (status, body) = if request.url == format!("{ISSUER}{USER_CODE}") {
                self.start.clone()
            } else if request.url == format!("{ISSUER}{POLL}") {
                if self.pending.get() > 0 {
                    self.pending.set(self.pending.get() - 1);
                    (self.pending_status, json!({"error": "fixture pending"}))
                } else {
                    self.granted.clone()
                }
            } else {
                (404, json!({}))
            };
            Ok(Response {
                status,
                retry_after: None,
                body: Zeroizing::new(serde_json::to_vec(&body).unwrap()),
            })
        }
    }

    /// Records every wait and never waits.
    #[derive(Default)]
    struct Waits(RefCell<Vec<Duration>>);

    impl Waiter for Waits {
        fn wait(&self, duration: Duration) -> Result<(), Failure> {
            self.0.borrow_mut().push(duration);
            Ok(())
        }
    }

    /// A change to the fake server's answers.
    type Edit = Box<dyn Fn(&mut Fake)>;

    fn declaration() -> ServerDeclaration {
        serde_json::from_value(json!({
            "authorize_url": format!("{ISSUER}/oauth/authorize"),
            "token_url": format!("{ISSUER}/oauth/token"),
            "device_authorization_url": null,
            "client_id": null,
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "json",
            "scope_in_refresh": false,
            "redirect_kind": "loopback_fixed_port",
            "redirect_uri": "http://127.0.0.1:1455/auth/callback",
            "authorize_parameters": [],
            "requested_scopes": ["openid"],
            "required_scopes": [],
            "rotation": "rotating",
            "refresh_lead_seconds": 300,
            "grant_types": ["authorization_code"],
            "sources": [],
            "state_in_exchange": false,
        }))
        .unwrap()
    }

    /// Runs the dialect against `fake`, recording what it showed.
    fn run(fake: &Fake, waits: &Waits) -> (Result<DialectGrant, Failure>, Vec<(String, String)>) {
        let shown = Rc::new(RefCell::new(Vec::new()));
        let recorder = shown.clone();
        let dialect = CodexDevice::showing(Box::new(move |code: &str, page: &str| {
            recorder
                .borrow_mut()
                .push((code.to_owned(), page.to_owned()))
        }));
        let declaration = declaration();
        let result = dialect.authorize(&DialectContext {
            declaration: &declaration,
            client_id: "fixture-configured-client",
            transport: fake,
            waiter: waits,
        });
        let shown = shown.borrow().clone();
        (result, shown)
    }

    #[test]
    fn the_device_flow_asks_a_user_code_polls_and_returns_the_code_for_the_exchange() {
        let fake = Fake::new(2);
        let waits = Waits::default();
        let (result, shown) = run(&fake, &waits);
        let grant = result.unwrap();
        assert_eq!(grant.code.expose(), "fixture-device-code-1");
        assert_eq!(grant.verifier.expose(), "fixture-device-verifier-1");
        assert_eq!(grant.redirect_uri, format!("{ISSUER}/deviceauth/callback"));
        assert_eq!(
            shown,
            vec![("FIXT-0001".to_owned(), format!("{ISSUER}/codex/device"))]
        );
        let sent = fake.sent.borrow();
        assert_eq!(sent.len(), 4, "{sent:?}");
        assert_eq!(sent[0].url, format!("{ISSUER}{USER_CODE}"));
        assert_eq!(sent[0].content_type, "application/json");
        assert_eq!(
            sent[0].body,
            json!({"client_id": "fixture-configured-client"})
        );
        for poll in &sent[1..] {
            assert_eq!(poll.url, format!("{ISSUER}{POLL}"));
            assert_eq!(poll.content_type, "application/json");
            assert_eq!(
                poll.body,
                json!({"device_auth_id": "fixture-device-auth-1", "user_code": "FIXT-0001"})
            );
        }
        // The server's interval, a string as the Codex CLI reads it, before
        // every poll.
        assert_eq!(*waits.0.borrow(), vec![Duration::from_secs(3); 3]);
    }

    #[test]
    fn a_numeric_interval_or_none_is_read() {
        for (interval, expected) in [
            (json!(2), Duration::from_secs(2)),
            (Value::Null, DEFAULT_INTERVAL),
            (json!("0"), DEFAULT_INTERVAL),
        ] {
            let mut fake = Fake::new(0);
            fake.start.1["interval"] = interval;
            let waits = Waits::default();
            run(&fake, &waits).0.unwrap();
            assert_eq!(*waits.0.borrow(), vec![expected]);
        }
    }

    #[test]
    fn a_404_poll_is_pending_too() {
        let mut fake = Fake::new(1);
        fake.pending_status = 404;
        let waits = Waits::default();
        run(&fake, &waits).0.unwrap();
        assert_eq!(waits.0.borrow().len(), 2);
    }

    #[test]
    fn the_wait_ends_after_fifteen_minutes() {
        let fake = Fake::new(u32::MAX);
        let waits = Waits::default();
        let (result, _) = run(&fake, &waits);
        assert_eq!(result.unwrap_err(), Failure::ExpiredToken);
        let waited: Duration = waits.0.borrow().iter().sum();
        assert!(waited <= MAX_WAIT && waited + Duration::from_secs(3) > MAX_WAIT);
    }

    #[test]
    fn a_refused_or_unusable_answer_ends_the_flow_without_server_text() {
        // Device sign-in not enabled for the account, a refused poll, a
        // server failure, a rate limit.
        let cases: Vec<(Edit, Failure)> = vec![
            (Box::new(|fake| fake.start.0 = 404), Failure::Rejected),
            (Box::new(|fake| fake.start.0 = 503), Failure::Unavailable),
            (
                Box::new(|fake| fake.start.0 = 429),
                Failure::RateLimited { retry_after: None },
            ),
            (Box::new(|fake| fake.granted.0 = 400), Failure::Rejected),
            (Box::new(|fake| fake.granted.0 = 500), Failure::Unavailable),
            // A user code that cannot be shown as one line, or none.
            (
                Box::new(|fake| fake.start.1["user_code"] = json!("FIXT\n0001")),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.start.1["user_code"] = json!("")),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.start.1["user_code"] = json!("F".repeat(65))),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.start.1["device_auth_id"] = json!("a b")),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.start.1["interval"] = json!("601")),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.start.1["interval"] = json!("soon")),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.granted.1 = json!({"authorization_code": "x"})),
                Failure::InvalidResponse,
            ),
            (
                Box::new(|fake| fake.granted.1["code_verifier"] = json!("")),
                Failure::InvalidResponse,
            ),
        ];
        for (index, (change, failure)) in cases.into_iter().enumerate() {
            let mut fake = Fake::new(0);
            change(&mut fake);
            let (result, _) = run(&fake, &Waits::default());
            assert_eq!(result.unwrap_err(), failure, "case {index}");
        }
    }

    #[test]
    fn a_token_url_that_is_not_https_names_no_issuer() {
        let fake = Fake::new(0);
        let mut declaration = declaration();
        declaration.token_url = "http://localhost:4443/oauth/token".into();
        let dialect = CodexDevice::showing(Box::new(|_: &str, _: &str| {}));
        let result = dialect.authorize(&DialectContext {
            declaration: &declaration,
            client_id: "fixture-configured-client",
            transport: &fake,
            waiter: &Waits::default(),
        });
        assert_eq!(result.unwrap_err(), Failure::InvalidDeclaration);
        assert!(fake.sent.borrow().is_empty());
    }

    #[test]
    fn the_grant_shows_no_secret() {
        let fake = Fake::new(0);
        let grant = run(&fake, &Waits::default()).0.unwrap();
        let shown = format!("{grant:?}");
        assert!(!shown.contains("fixture-device"), "{shown}");
    }
}
