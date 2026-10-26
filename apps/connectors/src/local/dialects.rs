//! The device dialects composition registers, keyed by profile id
//! (story:connect-runs-an-oauth2-grant). `--device` on an `oauth2_grant`
//! profile runs RFC 8628 against its declaration's `device_authorization_url`,
//! unless a dialect is registered here for that profile id: then the dialect
//! runs, and the code and verifier it returns are exchanged at the token
//! endpoint like any other code. This build registers the ChatGPT sign-in's
//! device flow for `chatgpt.subscription` (story:chatgpt-subscription-connector).
use connectors_host::local::{
    oauth2_grant::{self, Grant},
    owner,
};
use connectors_oauth2::DeviceDialect;
use connectors_sdk::Secret;
use std::{collections::BTreeMap, ffi::OsString};

/// A bare `--device` is `--device true`, and a bare `--reveal` is `--reveal
/// true`: `ess-cli/1` has no valueless switch source. Only the arguments of
/// `connections connect`, `reconnect` or its deprecated name `repair`
/// (`--device`), and of `connections token` (`--reveal`), before a first `--`
/// are rewritten, and a switch already followed by `true` or `false` is left
/// as it is.
pub(super) fn switch_shorthand(args: Vec<OsString>) -> Vec<OsString> {
    let switch = match command(&args).as_slice() {
        ["connections", "connect" | "reconnect" | "repair"] => "--device",
        ["connections", "token"] => "--reveal",
        _ => return args,
    };
    let mut rewritten = Vec::with_capacity(args.len() + 1);
    let mut trailing = false;
    let mut rest = args.into_iter().peekable();
    while let Some(arg) = rest.next() {
        let bare = !trailing && arg.to_str() == Some(switch);
        trailing |= arg.to_str() == Some("--");
        rewritten.push(arg);
        if bare
            && !matches!(
                rest.peek().and_then(|next| next.to_str()),
                Some("true" | "false")
            )
        {
            rewritten.push("true".into());
        }
    }
    rewritten
}

/// The first two command words, after any leading process globals.
fn command(args: &[OsString]) -> Vec<&str> {
    let mut words = Vec::new();
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output" | "--config" | "--state-dir") => {
                rest.next();
            }
            Some(arg)
                if ["--output=", "--config=", "--state-dir="]
                    .iter()
                    .any(|prefix| arg.starts_with(prefix)) => {}
            Some(word) if !word.starts_with('-') && words.len() < 2 => words.push(word),
            _ => break,
        }
    }
    words
}

/// Runs the grant of the `oauth2_grant` profile `capture` began, within its
/// window, and answers the entry it captures. `document` is the protected
/// document a confidential client's secret came in; `device` is `--device`.
pub(super) fn granted(
    capture: &owner::Capture,
    document: Option<Secret>,
    device: bool,
) -> owner::Result<Secret> {
    let dialects = Dialects::composed();
    let dialect = if device {
        dialects.lookup(&capture.profile.id)
    } else {
        None
    };
    oauth2_grant::run(
        &capture.profile,
        Grant {
            device,
            dialect,
            document,
        },
        capture.expires_at_ms,
    )
}

#[derive(Default)]
pub(super) struct Dialects(BTreeMap<String, Box<dyn DeviceDialect>>);

impl Dialects {
    /// The dialects this build composes.
    pub(super) fn composed() -> Self {
        let mut dialects = Self::default();
        dialects.register(
            connectors_catalog_provider::chatgpt::PROFILE,
            Box::new(connectors_catalog_provider::chatgpt::CodexDevice::default()),
        );
        dialects
    }

    /// Registers `dialect` for the profile `profile`, replacing any other.
    pub(super) fn register(&mut self, profile: &str, dialect: Box<dyn DeviceDialect>) {
        self.0.insert(profile.to_owned(), dialect);
    }

    /// The dialect registered for `profile`, if any.
    pub(super) fn lookup(&self, profile: &str) -> Option<&dyn DeviceDialect> {
        self.0.get(profile).map(Box::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors_host::local::oauth2_grant::device_grant;
    use connectors_oauth2::{
        Client, DeviceAuthorization, DialectContext, DialectGrant, Failure, Request, Response,
        Secret, ServerDeclaration, Transport, TransportError, Waiter,
    };
    use serde_json::json;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };
    use zeroize::Zeroizing;

    const PROFILE: &str = "fixture.device";
    const TOKEN_URL: &str = "https://localhost/fixture/token";
    const DEVICE_URL: &str = "https://localhost/fixture/device/code";

    /// Answers each URL with a fixed body and records every URL it was sent.
    #[derive(Clone, Default)]
    struct Recorded(Rc<RefCell<Vec<String>>>);

    impl Transport for Recorded {
        fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
            self.0.borrow_mut().push(request.url.to_owned());
            let body = match request.url {
                TOKEN_URL => json!({
                    "access_token": "fixture-dialect-access",
                    "token_type": "Bearer",
                    "expires_in": 600,
                    "scope": "fixture.read",
                }),
                DEVICE_URL => json!({
                    "device_code": "fixture-dialect-device",
                    "user_code": "FIXT-0001",
                    "verification_uri": "https://localhost/fixture/device",
                    "expires_in": 60,
                    "interval": 1,
                }),
                _ => return Err(TransportError),
            };
            Ok(Response {
                status: 200,
                retry_after: None,
                body: Zeroizing::new(serde_json::to_vec(&body).unwrap()),
            })
        }
    }

    /// A waiter that never waits.
    struct Now;

    impl Waiter for Now {
        fn wait(&self, _duration: Duration) -> Result<(), Failure> {
            Ok(())
        }
    }

    /// A dialect that counts its calls and returns a fixed code.
    struct Counting(Rc<Cell<u32>>);

    impl DeviceDialect for Counting {
        fn authorize(&self, context: &DialectContext<'_>) -> Result<DialectGrant, Failure> {
            assert_eq!(context.client_id, "fixture-device-client");
            self.0.set(self.0.get() + 1);
            Ok(DialectGrant {
                code: Secret::new("fixture-dialect-code".into()),
                verifier: Secret::new("v".repeat(43)),
                redirect_uri: "https://localhost/fixture/callback".into(),
            })
        }
    }

    fn client(transport: Recorded) -> Client<Recorded> {
        let declaration: ServerDeclaration = serde_json::from_value(json!({
            "authorize_url": null,
            "token_url": TOKEN_URL,
            "device_authorization_url": DEVICE_URL,
            "client_id": "fixture-device-client",
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["fixture.read"],
            "required_scopes": ["fixture.read"],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["device_code"],
            "sources": ["https://localhost/fixture-authorization-server"],
            "state_in_exchange": false,
        }))
        .unwrap();
        Client::new(declaration, None, None, transport).unwrap()
    }

    fn args(words: &[&str]) -> Vec<std::ffi::OsString> {
        words.iter().map(|word| (*word).into()).collect()
    }

    #[test]
    fn a_bare_device_switch_reads_true_on_connect_and_reconnect_only() {
        for verb in ["connect", "reconnect", "repair"] {
            assert_eq!(
                switch_shorthand(args(&[
                    "c",
                    "--output",
                    "json",
                    "connections",
                    verb,
                    "--device"
                ])),
                args(&[
                    "c",
                    "--output",
                    "json",
                    "connections",
                    verb,
                    "--device",
                    "true"
                ])
            );
            assert_eq!(
                switch_shorthand(args(&[
                    "c",
                    "connections",
                    verb,
                    "--device",
                    "--profile",
                    "p"
                ])),
                args(&[
                    "c",
                    "connections",
                    verb,
                    "--device",
                    "true",
                    "--profile",
                    "p"
                ])
            );
        }
        for unchanged in [
            &["c", "connections", "connect", "--device", "false"][..],
            &["c", "connections", "connect", "--device", "true"],
            &["c", "connections", "connect", "--device=true"],
            &["c", "connections", "connect", "--", "--device"],
            &["c", "connections", "status", "--device"],
            &["c", "operations", "invoke", "--device"],
        ] {
            assert_eq!(switch_shorthand(args(unchanged)), args(unchanged));
        }
    }

    #[test]
    fn a_bare_reveal_switch_reads_true_on_token_only() {
        assert_eq!(
            switch_shorthand(args(&["c", "connections", "token", "--reveal"])),
            args(&["c", "connections", "token", "--reveal", "true"])
        );
        assert_eq!(
            switch_shorthand(args(&[
                "c",
                "connections",
                "token",
                "--reveal",
                "--connection",
                "x"
            ])),
            args(&[
                "c",
                "connections",
                "token",
                "--reveal",
                "true",
                "--connection",
                "x"
            ])
        );
        for unchanged in [
            &["c", "connections", "token", "--reveal", "false"][..],
            &["c", "connections", "token", "--device"],
            &["c", "connections", "connect", "--reveal"],
            &["c", "connections", "token", "--", "--reveal"],
        ] {
            assert_eq!(switch_shorthand(args(unchanged)), args(unchanged));
        }
    }

    #[test]
    fn the_chatgpt_subscription_profile_has_its_device_dialect() {
        let composed = Dialects::composed();
        assert!(composed.lookup("chatgpt.subscription").is_some());
        assert!(composed.lookup(PROFILE).is_none());
    }

    #[test]
    fn device_grant_uses_the_registered_dialect() {
        let calls = Rc::new(Cell::new(0));
        let mut dialects = Dialects::composed();
        dialects.register(PROFILE, Box::new(Counting(calls.clone())));
        let transport = Recorded::default();
        let mut shown = Vec::<String>::new();
        let granted = device_grant(
            &client(transport.clone()),
            dialects.lookup(PROFILE),
            &Now,
            &mut |device: &DeviceAuthorization| shown.push(device.user_code.clone()),
        )
        .unwrap();
        assert_eq!(calls.get(), 1, "the registered dialect ran");
        assert_eq!(granted.access_token.expose(), "fixture-dialect-access");
        assert_eq!(granted.scopes, vec!["fixture.read".to_owned()]);
        // Only the ordinary exchange: the RFC 8628 endpoint was never asked.
        assert_eq!(*transport.0.borrow(), vec![TOKEN_URL.to_owned()]);
        assert!(shown.is_empty(), "a dialect shows its own prompt");
    }

    #[test]
    fn device_grant_without_a_dialect_uses_rfc8628() {
        let calls = Rc::new(Cell::new(0));
        let mut dialects = Dialects::composed();
        dialects.register("fixture.other", Box::new(Counting(calls.clone())));
        assert!(dialects.lookup(PROFILE).is_none());
        assert!(Dialects::composed().lookup(PROFILE).is_none());
        let transport = Recorded::default();
        let mut shown = Vec::<String>::new();
        let granted = device_grant(
            &client(transport.clone()),
            dialects.lookup(PROFILE),
            &Now,
            &mut |device: &DeviceAuthorization| shown.push(device.user_code.clone()),
        )
        .unwrap();
        assert_eq!(calls.get(), 0, "another profile's dialect never runs");
        assert_eq!(granted.access_token.expose(), "fixture-dialect-access");
        assert_eq!(
            *transport.0.borrow(),
            vec![DEVICE_URL.to_owned(), TOKEN_URL.to_owned()]
        );
        assert_eq!(shown, vec!["FIXT-0001".to_owned()]);
    }
}
