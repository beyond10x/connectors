//! The OAuth 2.0 grant of an `oauth2_grant` profile
//! (story:connect-runs-an-oauth2-grant,
//! `architecture-decision-record:oauth2-grants-and-access-token-issue`).
//!
//! The CLI runs it inside the owner's capture window, the only process that can
//! reach the person who consents. It runs the grant the profile's declaration
//! names through `connectors-oauth2`: the authorization code with PKCE S256 over
//! the declared redirect kind, or with `--device` the device grant (RFC 8628, or
//! the device dialect composition registered for the profile). The token
//! response becomes the entry
//! `{client_id, access_token, refresh_token, expires_at_ms, scopes, id_token}`,
//! which then enters the ordinary capture: the owner validates it with the
//! adapter and stores it. A grant that fails, times out or is cancelled returns
//! no entry, so nothing is stored.
//!
//! Every request goes through [`Https`], which sends HTTPS only, follows no
//! redirect and trusts the platform roots. No token, code, verifier or secret
//! reaches an error, a log line or the person's terminal.
use super::{
    oauth::{self, FLOW_TIMEOUT},
    owner::{Code, Error, Result},
    protected,
    runtime::{EntryField, Profile},
};
use connectors_oauth2::{
    Authorization, AuthorizationCode, Cancellation, Client, DeviceAuthorization, DeviceDialect,
    Failure, GrantType, Granted, LoopbackReceiver, RedirectKind, Request, Response, Transport,
    TransportError, Waiter, pasted_code,
};
use connectors_sdk::Secret;
use serde::{Deserialize, Serialize};
use std::{
    io::Write as _,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};
use zeroize::Zeroizing;

/// What a grant leaves of the capture window for the owner's validation and
/// storage, which take up to 30 s.
const COMPLETION_RESERVE_MS: u64 = 45_000;
/// The largest grant entry: the most custody holds. The CLI builds no larger
/// entry, and the adapter reading one bounds each of its fields by it.
pub const ENTRY_LIMIT: usize = connectors_custody::MAX_BYTES;
/// How often a wait looks at the person's interruption.
const SLICE: Duration = Duration::from_millis(50);

/// The HTTPS transport of every grant request: no proxy, no redirect, no
/// retry, 5 s to connect and 15 s per request, the platform trust roots unless
/// `trust` replaces them, and at most [`Request::limit`]` + 1` body bytes read.
/// The request body is handed to the client in a buffer that is zeroized when
/// it has been sent.
pub struct Https {
    runtime: tokio::runtime::Runtime,
    client: reqwest::Client,
    /// The requests this transport was asked to send.
    asked: AtomicU64,
    /// Those that failed before any byte of them left.
    unsent: AtomicU64,
}

impl Https {
    pub fn new(trust: Option<&[u8]>) -> Result<Self> {
        // reqwest carries no crypto provider of its own; ring is the one this workspace selects.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut builder = reqwest::Client::builder()
            .no_proxy()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15));
        if let Some(root) = trust {
            let roots = reqwest::Certificate::from_pem_bundle(root)
                .map_err(|_| Code::InvalidConfiguration)?;
            if roots.is_empty() {
                return Err(Code::InvalidConfiguration.into());
            }
            builder = builder.tls_certs_only(roots);
        }
        let client = builder.build().map_err(|_| Code::Unavailable)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Code::Unavailable)?;
        Ok(Self {
            runtime,
            client,
            asked: AtomicU64::new(0),
            unsent: AtomicU64::new(0),
        })
    }

    /// Whether a request this transport was asked to send may have reached
    /// the server. `false` only when none was asked, or each failed while
    /// its connection was being established (name resolution, TCP connect,
    /// TLS handshake), before the client writes any byte of the request.
    pub fn may_have_sent(&self) -> bool {
        self.asked.load(Ordering::SeqCst) > self.unsent.load(Ordering::SeqCst)
    }
}

impl Transport for Https {
    fn post(&self, request: &Request<'_>) -> std::result::Result<Response, TransportError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let mut body = Zeroizing::new(Vec::with_capacity(request.body.len()));
        body.extend_from_slice(request.body);
        let limit = request.limit.saturating_add(1);
        self.runtime.block_on(async {
            let mut response = self
                .client
                .post(request.url)
                .header(reqwest::header::CONTENT_TYPE, request.content_type)
                .header(reqwest::header::ACCEPT, "application/json")
                .body(reqwest::Body::from(axum::body::Bytes::from_owner(body)))
                .send()
                .await
                .map_err(|error| {
                    // A connect error is the connector's (DNS, TCP, TLS) or
                    // the pool checkout's: the request was not written.
                    if error.is_connect() {
                        self.unsent.fetch_add(1, Ordering::SeqCst);
                    }
                    TransportError
                })?;
            let status = response.status().as_u16();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            // Sized for the bound up front, so no reallocation leaves a copy of
            // a token behind.
            let mut read = Zeroizing::new(Vec::with_capacity(limit));
            while read.len() < limit {
                let Some(chunk) = response.chunk().await.map_err(|_| TransportError)? else {
                    break;
                };
                let take = chunk.len().min(limit - read.len());
                read.extend_from_slice(&chunk[..take]);
            }
            Ok(Response {
                status,
                retry_after,
                body: read,
            })
        })
    }
}

/// A transport that notes the instant before each request it sends. A grant's
/// last request is the one its tokens answered (the code exchange, the
/// successful device poll, a dialect's exchange), and the server issued them
/// no earlier than that request was sent, so their expiry counts from it.
struct Stamped<T> {
    inner: T,
    clock: fn() -> u64,
    sent_ms: AtomicU64,
}

impl<T> Stamped<T> {
    fn new(inner: T, clock: fn() -> u64) -> Self {
        Self {
            inner,
            clock,
            sent_ms: AtomicU64::new(0),
        }
    }

    /// The instant before the last request was sent; 0 before any.
    fn sent_ms(&self) -> u64 {
        self.sent_ms.load(Ordering::SeqCst)
    }
}

impl<T: Transport> Transport for Stamped<T> {
    fn post(&self, request: &Request<'_>) -> std::result::Result<Response, TransportError> {
        self.sent_ms.store((self.clock)(), Ordering::SeqCst);
        self.inner.post(request)
    }
}

/// What the person asked of a grant.
pub struct Grant<'a> {
    /// `--device`: the device grant instead of the authorization code.
    pub device: bool,
    /// The device dialect composition registered for the profile, if any.
    pub dialect: Option<&'a dyn DeviceDialect>,
    /// The protected document a confidential client's secret came in,
    /// `{"client_secret": "…"}`; none for a public client.
    pub document: Option<Secret>,
}

/// Runs the grant `profile` declares and answers its entry. `expires_at_ms` is
/// the capture's: the grant ends 45 s before it, and after 240 s at most.
pub fn run(profile: &Profile, grant: Grant<'_>, expires_at_ms: u64) -> Result<Secret> {
    let declaration = profile.grant().ok_or(Code::InvalidInput)?.clone();
    let deadline_ms = connectors_sdk::now_ms()
        .saturating_add(FLOW_TIMEOUT.as_millis() as u64)
        .min(expires_at_ms.saturating_sub(COMPLETION_RESERVE_MS));
    let declared = |grant_type| declaration.grant_types.contains(&grant_type);
    if (grant.device && grant.dialect.is_none() && !declared(GrantType::DeviceCode))
        || (!grant.device && !declared(GrantType::AuthorizationCode))
    {
        return Err(Code::InvalidInput.into());
    }
    let secret = grant.document.map(client_secret).transpose()?;
    let trust = oauth::follow::trust()?;
    let transport = Stamped::new(Https::new(trust.as_deref())?, connectors_sdk::now_ms);
    let client = Client::new(declaration, None, secret, &transport)
        // A public client given a secret, or a confidential one given none:
        // the protected source is what does not fit.
        .map_err(|_| Code::ProtectedEntryUnavailable)?;
    let waiter = Until(deadline_ms);
    let granted = if grant.device {
        device_grant(
            &client,
            grant.dialect,
            &waiter,
            &mut |device| match &trust {
                Some(root) => follow::approve(device, root),
                None => present_device(device),
            },
        )
    } else {
        code_grant(&client, deadline_ms, trust.as_deref())
    }
    .map_err(refusal)?;
    entry(client.client_id(), &granted, transport.sent_ms())
}

/// A device grant: the dialect registered for the profile when there is one,
/// which shows its own prompt and returns a code for the ordinary exchange;
/// otherwise RFC 8628 at the declaration's `device_authorization_url`, with
/// the user code and verification address handed to `show`.
pub fn device_grant<T: Transport>(
    client: &Client<T>,
    dialect: Option<&dyn DeviceDialect>,
    waiter: &dyn Waiter,
    show: &mut dyn FnMut(&DeviceAuthorization),
) -> std::result::Result<Granted, Failure> {
    match dialect {
        Some(dialect) => client.device_dialect(dialect, waiter),
        None => {
            let device = client.device_authorize()?;
            show(&device);
            client.poll_device(&device, waiter)
        }
    }
}

/// The authorization-code grant over the declared redirect kind.
fn code_grant<T: Transport>(
    client: &Client<T>,
    deadline_ms: u64,
    trust: Option<&[u8]>,
) -> std::result::Result<Granted, Failure> {
    let declaration = client.declaration();
    let (authorization, code) = match declaration.redirect_kind {
        RedirectKind::PastedCode => {
            let redirect = declaration
                .redirect_uri
                .as_deref()
                .ok_or(Failure::InvalidDeclaration)?;
            let authorization = client.begin(redirect)?;
            let line = match trust {
                Some(root) => follow::pasted(authorization.url(), root),
                None => {
                    oauth::present(authorization.url());
                    pasted_line(deadline_ms)
                }
            }?;
            let code = pasted_code(&authorization, line.expose())?;
            (authorization, code)
        }
        RedirectKind::LoopbackAnyPort | RedirectKind::LoopbackFixedPort => {
            let receiver = LoopbackReceiver::bind(declaration)?;
            let authorization = client.begin(receiver.redirect_uri())?;
            match trust {
                Some(root) => oauth::follow::consent(authorization.url(), root),
                None => oauth::present(authorization.url()),
            }
            let code = receive(receiver, &authorization, deadline_ms)?;
            (authorization, code)
        }
    };
    client.exchange(authorization, code)
}

/// The loopback redirect, ended by the deadline or by the person's
/// interruption, which a watcher reads every 50 ms.
fn receive(
    receiver: LoopbackReceiver,
    authorization: &Authorization,
    deadline_ms: u64,
) -> std::result::Result<AuthorizationCode, Failure> {
    let timeout = Duration::from_millis(deadline_ms.saturating_sub(connectors_sdk::now_ms()));
    let cancellation = Cancellation::new();
    let finished = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !finished.load(Ordering::SeqCst) {
                if protected::cancellation().is_err() {
                    cancellation.cancel();
                    return;
                }
                std::thread::sleep(SLICE);
            }
        });
        let received = receiver.receive(authorization, timeout, &cancellation);
        finished.store(true, Ordering::SeqCst);
        received
    })
}

/// The `code#state` line the person pastes, read hidden from the terminal.
fn pasted_line(deadline_ms: u64) -> std::result::Result<connectors_oauth2::Secret, Failure> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Pasted {
        code: connectors_oauth2::Secret,
    }
    let document = protected::terminal(
        &[EntryField {
            name: "code".into(),
            label: "Paste the code#state line the authorization page shows".into(),
            max_bytes: 8192,
        }],
        deadline_ms,
    )
    .map_err(|error| match error.code {
        Code::Interrupted => Failure::Cancelled,
        Code::Timeout => Failure::Timeout,
        _ => Failure::InvalidResponse,
    })?;
    serde_json::from_slice::<Pasted>(&document.0)
        .map(|pasted| pasted.code)
        .map_err(|_| Failure::InvalidResponse)
}

/// Waits between device polls until the grant's deadline, and ends at once
/// when the person interrupts.
struct Until(u64);

impl Waiter for Until {
    fn wait(&self, duration: Duration) -> std::result::Result<(), Failure> {
        let end = connectors_sdk::now_ms().saturating_add(duration.as_millis() as u64);
        if end > self.0 {
            return Err(Failure::Timeout);
        }
        loop {
            if protected::cancellation().is_err() {
                return Err(Failure::Cancelled);
            }
            let now = connectors_sdk::now_ms();
            if now >= end {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(end - now).min(SLICE));
        }
    }
}

/// Shows the device grant's user code and where to enter it, where the person
/// running the CLI sees it now: on the terminal, or without one as the single
/// stderr line `connectors: device-code <user_code> <verification_uri>`.
fn present_device(device: &DeviceAuthorization) {
    let message = format!(
        "Open {} and enter the code {} (waiting up to {} s):\n",
        device.verification_uri,
        device.user_code,
        device.expires_in.as_secs().min(FLOW_TIMEOUT.as_secs())
    );
    let shown = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .and_then(|mut tty| tty.write_all(message.as_bytes()));
    if shown.is_err() {
        let _ = std::io::stderr()
            .lock()
            .write_all(device_line(device).as_bytes());
    }
}

/// The stderr form of a device grant's prompt.
fn device_line(device: &DeviceAuthorization) -> String {
    format!(
        "connectors: device-code {} {}\n",
        device.user_code, device.verification_uri
    )
}

/// A confidential client's secret from its protected document.
fn client_secret(document: Secret) -> Result<connectors_oauth2::Secret> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        client_secret: connectors_oauth2::Secret,
    }
    let document: Document =
        serde_json::from_slice(&document.0).map_err(|_| Code::ProtectedEntryUnavailable)?;
    let secret = document.client_secret.expose();
    if secret.is_empty() || !secret.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Code::ProtectedEntryUnavailable.into());
    }
    Ok(document.client_secret)
}

/// The entry a grant stores, in a buffer sized once so no reallocation leaves
/// an unzeroized copy behind. The access token expires `expires_in` after
/// `issued_at_ms`, which is no later than its issue.
pub(in crate::local) fn entry(
    client_id: &str,
    granted: &Granted,
    issued_at_ms: u64,
) -> Result<Secret> {
    #[derive(Serialize)]
    struct Entry<'a> {
        client_id: &'a str,
        access_token: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        refresh_token: Option<&'a str>,
        expires_at_ms: u64,
        scopes: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        id_token: Option<&'a str>,
    }
    let refresh_token = granted.refresh_token.as_ref().map(|token| token.expose());
    let id_token = granted.id_token.as_ref().map(|token| token.expose());
    let expires_at_ms = issued_at_ms
        .checked_add(u64::try_from(granted.expires_in.as_millis()).unwrap_or(u64::MAX))
        .ok_or(Code::ProtectedEntryUnavailable)?;
    // An escaped byte is at most six (`\u00XX`); 256 covers keys, punctuation
    // and the expiry.
    let size = client_id.len()
        + granted.access_token.expose().len()
        + refresh_token.map_or(0, str::len)
        + id_token.map_or(0, str::len)
        + granted
            .scopes
            .iter()
            .map(|scope| scope.len() + 3)
            .sum::<usize>();
    let mut secret = Secret(Vec::with_capacity(256 + 6 * size));
    serde_json::to_writer(
        &mut secret.0,
        &Entry {
            client_id,
            access_token: granted.access_token.expose(),
            refresh_token,
            expires_at_ms,
            scopes: &granted.scopes,
            id_token,
        },
    )
    .map_err(|_| Code::ProtectedEntryUnavailable)?;
    if secret.0.len() > ENTRY_LIMIT {
        return Err(Code::ProtectedEntryUnavailable.into());
    }
    Ok(secret)
}

/// The owner code a grant's failure is reported as. The grant's own closed
/// code carries no server text; the person sees which step to take next.
fn refusal(failure: Failure) -> Error {
    match failure {
        // The CLI's code for a grant without what the profile needs.
        Failure::InsufficientScope => Code::NotGranted,
        Failure::Timeout => Code::Timeout,
        Failure::Cancelled => Code::Interrupted,
        Failure::Unavailable | Failure::RateLimited { .. } => Code::Unavailable,
        Failure::InvalidDeclaration => Code::InvalidConfiguration,
        Failure::ReauthorizationRequired
        | Failure::AccessDenied
        | Failure::StateMismatch
        | Failure::ExpiredToken
        | Failure::Rejected
        | Failure::InvalidResponse
        | Failure::UnexpectedRotation => Code::ProtectedEntryUnavailable,
    }
    .into()
}

/// Test-only consent for the CLI journeys, beside `oauth`'s: a debug build in
/// which `CONNECTORS_TEST_OAUTH_FOLLOW` names a PEM trust root reads the pasted
/// line from the redirect's page and approves a device code itself, as a
/// person would. A release build compiles none of this.
#[cfg(debug_assertions)]
mod follow {
    use super::*;

    fn client(root: &[u8]) -> Option<reqwest::Client> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let roots = reqwest::Certificate::from_pem_bundle(root).ok()?;
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .tls_certs_only(roots)
            .timeout(Duration::from_secs(15))
            .build()
            .ok()
    }

    /// Asks the authorize URL, follows its redirect to the page that shows
    /// the `code#state` line, and answers that line.
    pub(super) fn pasted(
        url: &str,
        root: &[u8],
    ) -> std::result::Result<connectors_oauth2::Secret, Failure> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Failure::Unavailable)?;
        runtime.block_on(async {
            let client = client(root).ok_or(Failure::Unavailable)?;
            let consented = client
                .get(url)
                .send()
                .await
                .map_err(|_| Failure::Unavailable)?;
            let location = consented
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(Failure::InvalidResponse)?
                .to_owned();
            let page = client
                .get(location)
                .send()
                .await
                .map_err(|_| Failure::Unavailable)?;
            page.text()
                .await
                .map(connectors_oauth2::Secret::new)
                .map_err(|_| Failure::InvalidResponse)
        })
    }

    /// Approves the device code at its complete verification address.
    pub(super) fn approve(device: &DeviceAuthorization, root: &[u8]) {
        let Some(url) = device.verification_uri_complete.clone() else {
            return;
        };
        let root = root.to_vec();
        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                if let Some(client) = client(&root) {
                    let _ = client.get(url).send().await;
                }
            });
        });
    }
}
#[cfg(not(debug_assertions))]
mod follow {
    use super::*;

    pub(super) fn pasted(
        _url: &str,
        _root: &[u8],
    ) -> std::result::Result<connectors_oauth2::Secret, Failure> {
        Err(Failure::Unavailable)
    }

    pub(super) fn approve(_device: &DeviceAuthorization, _root: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn granted(refresh: Option<&str>, id_token: Option<&str>) -> Granted {
        Granted {
            access_token: connectors_oauth2::Secret::new("fixture-unit-access".into()),
            refresh_token: refresh.map(|token| connectors_oauth2::Secret::new(token.into())),
            expires_in: Duration::from_secs(600),
            scopes: vec!["fixture.read".into(), "fixture.profile".into()],
            id_token: id_token.map(|token| connectors_oauth2::Secret::new(token.into())),
        }
    }

    #[test]
    fn the_entry_holds_the_token_response_and_its_expiry() {
        let stored = entry(
            "fixture-unit-client",
            &granted(Some("fixture-unit-refresh"), Some("a.b.c")),
            1_000,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&stored.0).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "client_id": "fixture-unit-client",
                "access_token": "fixture-unit-access",
                "refresh_token": "fixture-unit-refresh",
                "expires_at_ms": 601_000,
                "scopes": ["fixture.read", "fixture.profile"],
                "id_token": "a.b.c",
            })
        );
        // Absent tokens are absent keys.
        let stored = entry("fixture-unit-client", &granted(None, None), 0).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&stored.0).unwrap();
        let keys: BTreeSet<_> = value.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["access_token", "client_id", "expires_at_ms", "scopes"]
                .map(str::to_owned)
                .into()
        );
    }

    #[test]
    fn an_entry_over_the_custody_limit_is_refused() {
        let mut large = granted(None, None);
        large.access_token = connectors_oauth2::Secret::new("a".repeat(ENTRY_LIMIT));
        assert!(matches!(
            entry("fixture-unit-client", &large, 0),
            Err(error) if error.code == Code::ProtectedEntryUnavailable
        ));
    }

    thread_local! {
        static CLOCK: std::cell::Cell<u64> = const { std::cell::Cell::new(1_000_000) };
    }

    /// The fake clock of [`SlowServer`]'s tests: each answer moves it on.
    fn clock() -> u64 {
        CLOCK.with(std::cell::Cell::get)
    }

    /// An authorization server whose every answer takes 7 s of [`clock`]. Its
    /// token endpoint answers `authorization_pending` `pending` times, then
    /// grants a token valid for 600 s, and notes when that granting request
    /// arrived.
    struct SlowServer {
        pending: std::cell::Cell<u32>,
        granting: std::cell::Cell<Option<u64>>,
    }

    impl SlowServer {
        fn new(pending: u32) -> Self {
            Self {
                pending: std::cell::Cell::new(pending),
                granting: std::cell::Cell::new(None),
            }
        }
    }

    impl Transport for SlowServer {
        fn post(&self, request: &Request<'_>) -> std::result::Result<Response, TransportError> {
            let arrived = clock();
            CLOCK.with(|now| now.set(now.get() + 7_000));
            let (status, body) = if request.url.ends_with("/device/code") {
                (
                    200,
                    serde_json::json!({
                        "device_code": "fixture-unit-device",
                        "user_code": "FIXT-0001",
                        "verification_uri": "https://localhost/fixture/device",
                        "expires_in": 600,
                        "interval": 1,
                    }),
                )
            } else if request.url.ends_with("/dialect") {
                (200, serde_json::json!({}))
            } else if self.pending.get() > 0 {
                self.pending.set(self.pending.get() - 1);
                (400, serde_json::json!({"error": "authorization_pending"}))
            } else {
                self.granting.set(Some(arrived));
                (
                    200,
                    serde_json::json!({
                        "access_token": "fixture-unit-access",
                        "token_type": "Bearer",
                        "expires_in": 600,
                        "scope": "fixture.read",
                    }),
                )
            };
            Ok(Response {
                status,
                retry_after: None,
                body: Zeroizing::new(serde_json::to_vec(&body).unwrap()),
            })
        }
    }

    /// A waiter that never waits.
    struct Immediately;

    impl Waiter for Immediately {
        fn wait(&self, _duration: Duration) -> std::result::Result<(), Failure> {
            Ok(())
        }
    }

    /// A device dialect that makes one request of its own and returns a code.
    struct Dialect;

    impl DeviceDialect for Dialect {
        fn authorize(
            &self,
            context: &connectors_oauth2::DialectContext<'_>,
        ) -> std::result::Result<connectors_oauth2::DialectGrant, Failure> {
            context
                .transport
                .post(&Request {
                    url: "https://localhost/fixture/dialect",
                    content_type: "application/json",
                    body: b"{}",
                    limit: 1024,
                })
                .map_err(|_| Failure::Unavailable)?;
            Ok(connectors_oauth2::DialectGrant {
                code: connectors_oauth2::Secret::new("fixture-unit-code".into()),
                verifier: connectors_oauth2::Secret::new("fixture-unit-verifier".into()),
                redirect_uri: "https://localhost/fixture/dialect/callback".into(),
            })
        }
    }

    /// The access token expires no later than `expires_in` after the request
    /// that obtained it arrived, however long the answer took: the successful
    /// RFC 8628 poll after two pending ones, and a dialect's exchange after
    /// its own request.
    #[test]
    fn a_device_grant_expires_from_the_request_that_obtained_the_token() {
        for (name, pending, dialect) in [
            ("rfc8628", 2, None),
            ("dialect", 0, Some(&Dialect as &dyn DeviceDialect)),
        ] {
            let server = SlowServer::new(pending);
            let transport = Stamped::new(&server, clock);
            let client = Client::new(device_declaration(), None, None, &transport).unwrap();
            let granted = device_grant(&client, dialect, &Immediately, &mut |_| {}).unwrap();
            let stored = entry(client.client_id(), &granted, transport.sent_ms()).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&stored.0).unwrap();
            let expires_at_ms = value["expires_at_ms"].as_u64().unwrap();
            let arrived = server.granting.get().unwrap();
            assert!(
                expires_at_ms <= arrived + 600_000,
                "{name}: expires at {expires_at_ms}, the granting request arrived at {arrived}"
            );
        }
    }

    #[test]
    fn a_failed_grant_names_the_next_step_and_no_server_text() {
        for (failure, code) in [
            (Failure::StateMismatch, Code::ProtectedEntryUnavailable),
            (Failure::ExpiredToken, Code::ProtectedEntryUnavailable),
            (Failure::AccessDenied, Code::ProtectedEntryUnavailable),
            (Failure::InsufficientScope, Code::NotGranted),
            (Failure::Timeout, Code::Timeout),
            (Failure::Cancelled, Code::Interrupted),
            (Failure::Unavailable, Code::Unavailable),
            (Failure::InvalidDeclaration, Code::InvalidConfiguration),
        ] {
            assert_eq!(refusal(failure).code, code, "{failure}");
        }
    }

    #[test]
    fn the_device_prompt_line_names_the_code_and_address_only() {
        let device: DeviceAuthorization = {
            // Built through the crate's own parsing: the type keeps its device
            // code private.
            use connectors_oauth2::{Request, Response, TransportError};
            struct Answer;
            impl Transport for Answer {
                fn post(
                    &self,
                    _request: &Request<'_>,
                ) -> std::result::Result<Response, TransportError> {
                    Ok(Response {
                        status: 200,
                        retry_after: None,
                        body: Zeroizing::new(
                            serde_json::to_vec(&serde_json::json!({
                                "device_code": "fixture-unit-device",
                                "user_code": "FIXT-0001",
                                "verification_uri": "https://localhost/fixture/device",
                                "expires_in": 60,
                            }))
                            .unwrap(),
                        ),
                    })
                }
            }
            Client::new(device_declaration(), None, None, Answer)
                .unwrap()
                .device_authorize()
                .unwrap()
        };
        assert_eq!(
            device_line(&device),
            "connectors: device-code FIXT-0001 https://localhost/fixture/device\n"
        );
    }

    /// A public client's declaration of the device grant alone.
    fn device_declaration() -> connectors_oauth2::ServerDeclaration {
        serde_json::from_value(serde_json::json!({
            "authorize_url": null,
            "token_url": "https://localhost/fixture/token",
            "device_authorization_url": "https://localhost/fixture/device/code",
            "client_id": "fixture-unit-client",
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["fixture.read"],
            "required_scopes": [],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["device_code"],
            "sources": [],
            "state_in_exchange": false,
            "scope_in_refresh": false,
        }))
        .unwrap()
    }

    #[test]
    fn a_confidential_secret_document_is_one_field() {
        let secret = client_secret(Secret(
            br#"{"client_secret":"fixture-unit-secret"}"#.to_vec(),
        ))
        .unwrap();
        assert_eq!(secret.expose(), "fixture-unit-secret");
        for document in [
            br#"{"client_secret":""}"#.as_slice(),
            br#"{"client_secret":"a b"}"#,
            br#"{"client_secret":"x","other":"y"}"#,
            br#"fixture-unit-secret"#,
        ] {
            assert_eq!(
                client_secret(Secret(document.to_vec())).unwrap_err().code,
                Code::ProtectedEntryUnavailable
            );
        }
    }

    #[test]
    fn a_wait_past_the_deadline_times_out_at_once() {
        let waiter = Until(connectors_sdk::now_ms() + 1_000);
        assert_eq!(waiter.wait(Duration::from_secs(5)), Err(Failure::Timeout));
        assert_eq!(waiter.wait(Duration::from_millis(1)), Ok(()));
    }
}
