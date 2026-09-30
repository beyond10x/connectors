//! Browser consent for a profile whose protected entry is acquired by OAuth
//! (`architecture-decision-record:oauth-material-as-static-entry`).
//!
//! The CLI runs this inside the owner's capture window, the only process that
//! can reach a person's browser. Given Google's installed-client JSON for a
//! profile that declares `acquisition`, it obtains a refresh token by an
//! authorization-code grant with a loopback redirect, PKCE S256 and a
//! single-use `state`, and returns `{client_id, client_secret, refresh_token}`,
//! which then enters the ordinary static-entry capture. Any other document, or
//! a profile without `acquisition`, passes through unchanged.
use super::{
    owner::{Code, Error, Result},
    protected,
    runtime::{Acquisition, Profile},
};
use crate::http::{HttpConfig, ScopedHttp};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_sdk::Secret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{ErrorKind, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    time::Duration,
};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

/// How long consent may take; it ends before the owner's 300 s capture window.
pub const FLOW_TIMEOUT: Duration = Duration::from_secs(240);
/// What a flow leaves of the capture window for the code exchange and the
/// owner's own validation, which takes up to 30 s.
const COMPLETION_RESERVE_MS: u64 = 45_000;
/// The profile fields this flow fills, in sorted order.
const FIELDS: [&str; 3] = ["client_id", "client_secret", "refresh_token"];
/// The largest request head the listener reads.
const REQUEST_LIMIT: usize = 8192;
const POLL: Duration = Duration::from_millis(50);

/// The entry the CLI submits for `document` under `profile`: the acquired
/// triple for Google's installed-client JSON under a profile with
/// `acquisition`, else `document` itself. `expires_at_ms` is the capture's.
pub fn entry(profile: &Profile, document: Secret, expires_at_ms: u64) -> Result<Secret> {
    let Some(client) = client_file(profile, &document) else {
        return Ok(document);
    };
    drop(document);
    let deadline_ms = connectors_sdk::now_ms()
        .saturating_add(FLOW_TIMEOUT.as_millis() as u64)
        .min(expires_at_ms.saturating_sub(COMPLETION_RESERVE_MS));
    let trust = follow::trust()?;
    acquire(
        profile,
        &client,
        deadline_ms,
        Environment {
            present: &mut |url| match &trust {
                Some(root) => follow::consent(url, root),
                None => present(url),
            },
            cancelled: &protected::cancellation,
            exchange: &mut |request| exchange(request, trust.as_deref()),
        },
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientFile {
    installed: Installed,
}

/// Google's installed-application client. The other keys Google writes, such
/// as `project_id`, are ignored.
#[derive(Deserialize)]
struct Installed {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    token_uri: String,
    /// Required, so only Google's shape is recognised; the flow uses its own
    /// loopback redirect, which Google admits for any Desktop client.
    #[serde(rename = "redirect_uris")]
    _redirect_uris: serde::de::IgnoredAny,
}
impl Drop for Installed {
    fn drop(&mut self) {
        self.client_secret.zeroize();
    }
}

/// One authorization-code exchange at the profile's `token_url`.
struct Exchange<'a> {
    token_url: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    code: &'a str,
    verifier: &'a str,
    redirect_uri: &'a str,
}

/// What the flow does outside itself. Production and the tests differ here only.
struct Environment<'a> {
    /// Shows the authorize URL to the person who consents.
    present: &'a mut dyn FnMut(&str),
    /// Fails `Interrupted` once the person interrupted the CLI.
    cancelled: &'a dyn Fn() -> Result<()>,
    /// Exchanges the code; answers the refresh token.
    exchange: &'a mut dyn FnMut(&Exchange<'_>) -> Result<Zeroizing<String>>,
}

fn client_file(profile: &Profile, document: &Secret) -> Option<Installed> {
    profile.acquisition.as_ref()?;
    serde_json::from_slice::<ClientFile>(&document.0)
        .ok()
        .map(|file| file.installed)
}

/// RFC 7636 S256: `BASE64URL(SHA256(ASCII(verifier)))`, unpadded.
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// 32 bytes from the system CSPRNG, unpadded base64url: 43 characters, every
/// one RFC 7636 unreserved.
fn random() -> Result<Zeroizing<String>> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut bytes = Zeroizing::new([0_u8; 32]);
    SystemRandom::new()
        .fill(&mut bytes[..])
        .map_err(|_| Code::Unavailable)?;
    Ok(Zeroizing::new(URL_SAFE_NO_PAD.encode(&bytes[..])))
}

fn https(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some())
}

fn acquire(
    profile: &Profile,
    client: &Installed,
    deadline_ms: u64,
    environment: Environment<'_>,
) -> Result<Secret> {
    let acquisition = profile
        .acquisition
        .as_ref()
        .ok_or(Code::ProtectedEntryUnavailable)?;
    let mut fields: Vec<&str> = profile.fields.iter().map(|f| f.name.as_str()).collect();
    fields.sort_unstable();
    if fields != FIELDS
        || client.auth_uri != acquisition.authorize_url
        || client.token_uri != acquisition.token_url
        || !https(&acquisition.authorize_url)
        || !https(&acquisition.token_url)
    {
        return Err(Code::ProtectedEntryUnavailable.into());
    }
    let verifier = random()?;
    let state = random()?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| Code::Unavailable)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| Code::Unavailable)?;
    let port = listener.local_addr().map_err(|_| Code::Unavailable)?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let url = authorize_url(
        acquisition,
        &client.client_id,
        &redirect_uri,
        &state,
        &challenge(&verifier),
    )?;
    (environment.present)(&url);
    let code = receive(listener, &state, deadline_ms, environment.cancelled)?;
    (environment.cancelled)()?;
    let refresh_token = (environment.exchange)(&Exchange {
        token_url: &acquisition.token_url,
        client_id: &client.client_id,
        client_secret: &client.client_secret,
        code: &code,
        verifier: &verifier,
        redirect_uri: &redirect_uri,
    })?;
    triple(&client.client_id, &client.client_secret, &refresh_token)
}

fn authorize_url(
    acquisition: &Acquisition,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> Result<String> {
    let mut url =
        url::Url::parse(&acquisition.authorize_url).map_err(|_| Code::ProtectedEntryUnavailable)?;
    let mut scopes: Vec<&str> = acquisition.scopes.iter().map(String::as_str).collect();
    if !scopes.contains(&"openid") {
        scopes.push("openid");
    }
    scopes.sort_unstable();
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &scopes.join(" "))
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    Ok(url.into())
}

/// The code of the one request the listener answers. The first request
/// decides: a request that is not a redirect to `/` with the flow's `state`
/// and a code, carries `error`, or has a second request queued behind it, is
/// refused and the flow fails. The listener closes when this returns, so no
/// later request is ever answered.
fn receive(
    listener: TcpListener,
    state: &str,
    deadline_ms: u64,
    cancelled: &dyn Fn() -> Result<()>,
) -> Result<Zeroizing<String>> {
    loop {
        cancelled()?;
        if connectors_sdk::now_ms() >= deadline_ms {
            return Err(Code::Timeout.into());
        }
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error)
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) =>
            {
                std::thread::sleep(POLL);
                continue;
            }
            Err(_) => return Err(Code::Unavailable.into()),
        };
        // A connection closed without a request, such as a browser's
        // speculative one, is not a request.
        let Some(head) = read_head(&mut stream, deadline_ms, cancelled)? else {
            continue;
        };
        let queued = listener.accept().is_ok();
        drop(listener);
        let outcome = if queued {
            Err(Code::ProtectedEntryUnavailable.into())
        } else {
            redirect(&head, state)
        };
        answer(stream, outcome.is_ok());
        return outcome;
    }
}

/// The request head, bounded; `None` for a connection that ends before its
/// first byte. A truncated or oversized head is returned as read, and refused.
fn read_head(
    stream: &mut TcpStream,
    deadline_ms: u64,
    cancelled: &dyn Fn() -> Result<()>,
) -> Result<Option<Zeroizing<Vec<u8>>>> {
    stream
        .set_nonblocking(false)
        .and_then(|_| stream.set_read_timeout(Some(POLL)))
        .map_err(|_| Code::Unavailable)?;
    let mut head = Zeroizing::new(Vec::with_capacity(REQUEST_LIMIT));
    let mut chunk = Zeroizing::new([0_u8; 1024]);
    while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < REQUEST_LIMIT {
        cancelled()?;
        if connectors_sdk::now_ms() >= deadline_ms {
            return Err(Code::Timeout.into());
        }
        let room = (REQUEST_LIMIT - head.len()).min(chunk.len());
        match stream.read(&mut chunk[..room]) {
            Ok(0) if head.is_empty() => return Ok(None),
            Ok(0) => break,
            Ok(read) => head.extend_from_slice(&chunk[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) => {}
            Err(_) if head.is_empty() => return Ok(None),
            Err(_) => break,
        }
    }
    Ok(Some(head))
}

/// The code of a well-formed consent redirect for `state`.
fn redirect(head: &[u8], state: &str) -> Result<Zeroizing<String>> {
    let refused = || -> Error { Code::ProtectedEntryUnavailable.into() };
    let text = std::str::from_utf8(head).map_err(|_| refused())?;
    let line = text.split("\r\n").next().unwrap_or_default();
    let mut words = line.split(' ');
    let (Some("GET"), Some(target), Some(version), None) =
        (words.next(), words.next(), words.next(), words.next())
    else {
        return Err(refused());
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != "/" || !version.starts_with("HTTP/1.") || !text.ends_with("\r\n\r\n") {
        return Err(refused());
    }
    let (mut code, mut received, mut error) = (None, None, false);
    for (name, value) in url::form_urlencoded::parse(query.as_bytes()) {
        let slot = match name.as_ref() {
            "code" => &mut code,
            "state" => &mut received,
            "error" => {
                error = true;
                continue;
            }
            _ => continue,
        };
        if slot.replace(Zeroizing::new(value.into_owned())).is_some() {
            return Err(refused());
        }
    }
    let matches = received
        .as_ref()
        .is_some_and(|received| bool::from(received.as_bytes().ct_eq(state.as_bytes())));
    match code {
        Some(code) if matches && !error && !code.is_empty() => Ok(code),
        _ => Err(refused()),
    }
}

/// Tells the browser how consent ended. Its answer is not the flow's result.
fn answer(mut stream: TcpStream, completed: bool) {
    let (status, body) = if completed {
        (
            "200 OK",
            "Consent received. You can close this window and return to the terminal.",
        )
    } else {
        (
            "400 Bad Request",
            "Consent was not completed. Return to the terminal.",
        )
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
}

/// Writes the consent address where the person running the CLI sees it now:
/// the CLI's ordinary output is held until it exits.
fn present(url: &str) {
    let message = format!(
        "Open this address in a browser on this machine to grant access (waiting {} s):\n{url}\n",
        FLOW_TIMEOUT.as_secs()
    );
    let shown = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .and_then(|mut tty| tty.write_all(message.as_bytes()));
    if shown.is_err() {
        let _ = std::io::stderr().lock().write_all(message.as_bytes());
    }
}

/// The code exchange: one form POST to `token_url`, under the platform trust
/// roots unless `trust` replaces them.
fn exchange(request: &Exchange<'_>, trust: Option<&[u8]>) -> Result<Zeroizing<String>> {
    let refused = || -> Error { Code::ProtectedEntryUnavailable.into() };
    let url = url::Url::parse(request.token_url).map_err(|_| refused())?;
    let segments: Vec<&str> = url
        .path_segments()
        .map(|segments| segments.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if url.scheme() != "https"
        || url.query().is_some()
        || url.fragment().is_some()
        || segments.is_empty()
        || segments.iter().any(|segment| segment.contains('%'))
    {
        return Err(refused());
    }
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: format!("{}/", &url[..url::Position::BeforePath]),
            credential: None,
            credential_header: "authorization".into(),
            bearer: false,
            allow_plaintext: false,
            ca_file: None,
        },
        None,
        trust,
    )
    .map_err(|_| refused())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Code::Unavailable)?;
    let response = runtime
        .block_on(http.post_form(
            &segments,
            &[
                ("grant_type", b"authorization_code".as_slice()),
                ("code", request.code.as_bytes()),
                ("redirect_uri", request.redirect_uri.as_bytes()),
                ("client_id", request.client_id.as_bytes()),
                ("client_secret", request.client_secret.as_bytes()),
                ("code_verifier", request.verifier.as_bytes()),
            ],
        ))
        .map_err(|_| Code::Unavailable)?;
    let body = Zeroizing::new(response.body);
    if response.status != 200 {
        return Err(refused());
    }
    /// A token answer's refresh token; every other field is the provider's.
    #[derive(Deserialize)]
    struct Granted {
        refresh_token: Option<String>,
    }
    let refresh_token = serde_json::from_slice::<Granted>(&body)
        .ok()
        .and_then(|granted| granted.refresh_token)
        .map(Zeroizing::new)
        .ok_or_else(refused)?;
    if refresh_token.is_empty() || !refresh_token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(refused());
    }
    Ok(refresh_token)
}

/// `{client_id, client_secret, refresh_token}` in a buffer sized once, so no
/// reallocation leaves an unzeroized copy behind.
fn triple(client_id: &str, client_secret: &str, refresh_token: &str) -> Result<Secret> {
    #[derive(Serialize)]
    struct Entry<'a> {
        client_id: &'a str,
        client_secret: &'a str,
        refresh_token: &'a str,
    }
    // An escaped byte is at most six (`\u00XX`); 64 covers keys and punctuation.
    let capacity = 64 + 6 * (client_id.len() + client_secret.len() + refresh_token.len());
    let mut secret = Secret(Vec::with_capacity(capacity));
    serde_json::to_writer(
        &mut secret.0,
        &Entry {
            client_id,
            client_secret,
            refresh_token,
        },
    )
    .map_err(|_| Code::ProtectedEntryUnavailable)?;
    Ok(secret)
}

/// Test-only consent for the CLI journeys. A debug build in which
/// `CONNECTORS_TEST_OAUTH_FOLLOW` names a PEM trust root follows the authorize
/// URL itself, as a browser whose user consents would, and trusts that root
/// for the code exchange. A release build compiles none of this, so no
/// environment variable can redirect its consent or its trust.
#[cfg(debug_assertions)]
mod follow {
    use super::*;

    pub(super) fn trust() -> Result<Option<Vec<u8>>> {
        std::env::var_os("CONNECTORS_TEST_OAUTH_FOLLOW")
            .map(|path| std::fs::read(path).map_err(|_| Code::InvalidConfiguration.into()))
            .transpose()
    }

    /// Asks the authorize URL, and follows its redirect to the listener.
    pub(super) fn consent(url: &str, root: &[u8]) {
        let (url, root) = (url.to_owned(), root.to_vec());
        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                let _ = rustls::crypto::ring::default_provider().install_default();
                let Ok(roots) = reqwest::Certificate::from_pem_bundle(&root) else {
                    return;
                };
                let Ok(client) = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .tls_certs_only(roots)
                    .timeout(Duration::from_secs(15))
                    .build()
                else {
                    return;
                };
                let Ok(consented) = client.get(url).send().await else {
                    return;
                };
                let Some(location) = consented
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned)
                else {
                    return;
                };
                let _ = client.get(location).send().await;
            });
        });
    }
}
#[cfg(not(debug_assertions))]
mod follow {
    pub(super) fn trust() -> super::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    pub(super) fn consent(_url: &str, _root: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::{
        protected, registry,
        runtime::{Acquisition, EntryField},
    };
    use std::{
        io::{Read, Write},
        net::TcpStream,
        process::{Command, Stdio},
        time::Instant,
    };
    use url::Url;

    const AUTHORIZE: &str = "https://accounts.example.test/o/oauth2/auth";
    const TOKEN: &str = "https://oauth2.example.test/token";
    const SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
    const CLIENT_ID: &str = "fixture-client.apps.example.test";
    const CLIENT_SECRET: &str = "fixture-client-secret";
    const REFRESH: &str = "fixture-refresh-token-acquired";
    const TRIPLE: [&str; 3] = ["client_id", "client_secret", "refresh_token"];

    fn profile(fields: &[&str]) -> Profile {
        Profile {
            id: "fixture.oauth".into(),
            revision: "fixture-revision".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: Default::default(),
            evidence_lifetime_ms: 60_000,
            fields: fields
                .iter()
                .map(|name| EntryField {
                    name: (*name).into(),
                    label: (*name).into(),
                    max_bytes: 8192,
                })
                .collect(),
            acquisition: Some(Acquisition {
                authorize_url: AUTHORIZE.into(),
                token_url: TOKEN.into(),
                scopes: [SCOPE.to_owned()].into(),
            }),
        }
    }

    /// The shape Google's console downloads for a Desktop app client.
    fn google(auth_uri: &str, token_uri: &str) -> Secret {
        Secret(
            serde_json::to_vec(&serde_json::json!({"installed": {
                "client_id": CLIENT_ID,
                "project_id": "fixture-project",
                "auth_uri": auth_uri,
                "token_uri": token_uri,
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": CLIENT_SECRET,
                "redirect_uris": ["http://localhost"],
            }}))
            .unwrap(),
        )
    }

    fn client(auth_uri: &str, token_uri: &str) -> Installed {
        client_file(&profile(&TRIPLE), &google(auth_uri, token_uri))
            .unwrap_or_else(|| panic!("Google's installed-client JSON was not recognised"))
    }

    fn code(result: &Result<Secret>) -> Option<Code> {
        result.as_ref().err().map(|error| error.code)
    }

    fn param(url: &Url, name: &str) -> Option<String> {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    }

    /// What one run of the flow showed the user, sent from the browser and
    /// asked the token endpoint.
    #[derive(Default)]
    struct Record {
        presented: Vec<Url>,
        exchanged: Vec<(String, String, String)>,
        browser: Vec<TcpStream>,
    }
    impl Record {
        /// Each browser connection's full answer; empty when none came.
        fn answers(&mut self) -> Vec<String> {
            self.browser
                .iter_mut()
                .map(|stream| {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let _ = stream.read_to_end(&mut bytes);
                    String::from_utf8_lossy(&bytes).into_owned()
                })
                .collect()
        }
    }

    /// Runs the flow with a browser that, as soon as the authorize URL is
    /// shown, sends each of `browse(url)` on its own connection, and a token
    /// endpoint that answers `REFRESH` for any code.
    fn run(
        profile: &Profile,
        client: &Installed,
        budget: Duration,
        browse: impl Fn(&Url) -> Vec<String>,
    ) -> (Result<Secret>, Record) {
        let mut record = Record::default();
        let (presented, browser, exchanged) = (
            &mut record.presented,
            &mut record.browser,
            &mut record.exchanged,
        );
        let result = acquire(
            profile,
            client,
            connectors_sdk::now_ms() + budget.as_millis() as u64,
            Environment {
                present: &mut |shown| {
                    let url = Url::parse(shown).unwrap();
                    let redirect = Url::parse(&param(&url, "redirect_uri").unwrap()).unwrap();
                    let port = redirect.port().unwrap();
                    for target in browse(&url) {
                        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
                        stream
                            .write_all(
                                format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
                                    .as_bytes(),
                            )
                            .unwrap();
                        browser.push(stream);
                    }
                    presented.push(url);
                },
                cancelled: &|| Ok(()),
                exchange: &mut |request| {
                    exchanged.push((
                        request.code.to_owned(),
                        request.verifier.to_owned(),
                        request.redirect_uri.to_owned(),
                    ));
                    Ok(Zeroizing::new(REFRESH.to_owned()))
                },
            },
        );
        (result, record)
    }

    /// The redirect a browser follows after consent to the shown URL.
    fn consented(url: &Url) -> String {
        format!(
            "/?state={}&code=fixture-code&scope=openid",
            param(url, "state").unwrap()
        )
    }

    const BUDGET: Duration = Duration::from_secs(20);

    #[test]
    fn pkce_challenge_matches_rfc7636_appendix_b() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn consent_submits_the_acquired_triple() {
        let (result, mut record) = run(
            &profile(&TRIPLE),
            &client(AUTHORIZE, TOKEN),
            BUDGET,
            |url| vec![consented(url)],
        );
        let Ok(secret) = result else {
            panic!("consent failed: {:?}", code(&result));
        };
        let entry: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&secret.0).unwrap();
        assert_eq!(entry.len(), 3);
        assert!(entry["client_id"] == CLIENT_ID);
        assert!(entry["client_secret"] == CLIENT_SECRET);
        assert!(entry["refresh_token"] == REFRESH);

        let [url] = &record.presented[..] else {
            panic!(
                "the authorize URL was shown {} times",
                record.presented.len()
            );
        };
        assert!(url.as_str().starts_with(&format!("{AUTHORIZE}?")));
        for (name, value) in [
            ("response_type", "code"),
            ("client_id", CLIENT_ID),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ] {
            assert_eq!(param(url, name).as_deref(), Some(value), "{name}");
        }
        let scopes: std::collections::BTreeSet<String> = param(url, "scope")
            .unwrap()
            .split(' ')
            .map(str::to_owned)
            .collect();
        assert_eq!(scopes, ["openid".to_owned(), SCOPE.to_owned()].into());
        let redirect_uri = param(url, "redirect_uri").unwrap();
        assert!(redirect_uri.starts_with("http://127.0.0.1:"));
        assert!(param(url, "state").unwrap().len() >= 43);

        let [(code, verifier, exchanged_redirect)] = &record.exchanged[..] else {
            panic!("exchanged {} times", record.exchanged.len());
        };
        assert_eq!(code, "fixture-code");
        assert_eq!(exchanged_redirect, &redirect_uri);
        assert!((43..=128).contains(&verifier.len()));
        assert!(
            verifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
        );
        assert_eq!(
            param(url, "code_challenge").unwrap(),
            challenge(verifier),
            "the challenge is S256 of the verifier sent at exchange"
        );
        assert!(record.answers()[0].starts_with("HTTP/1.1 200 "));
    }

    #[test]
    fn state_mismatch_refused() {
        let flipped = |url: &Url| {
            let mut state = param(url, "state").unwrap().into_bytes();
            let last = state.last_mut().unwrap();
            *last = if *last == b'A' { b'B' } else { b'A' };
            String::from_utf8(state).unwrap()
        };
        for forged in [
            Box::new(|_: &Url| "forged".to_owned()) as Box<dyn Fn(&Url) -> String>,
            Box::new(flipped),
        ] {
            let (result, mut record) = run(
                &profile(&TRIPLE),
                &client(AUTHORIZE, TOKEN),
                BUDGET,
                |url| vec![format!("/?state={}&code=fixture-code", forged(url))],
            );
            assert_eq!(code(&result), Some(Code::ProtectedEntryUnavailable));
            assert!(record.exchanged.is_empty(), "a forged state was exchanged");
            assert!(record.answers()[0].starts_with("HTTP/1.1 400 "));
        }
    }

    #[test]
    fn error_redirect_refused() {
        for query in [
            "error=access_denied",
            "error=access_denied&code=fixture-code",
        ] {
            let (result, mut record) = run(
                &profile(&TRIPLE),
                &client(AUTHORIZE, TOKEN),
                BUDGET,
                |url| vec![format!("/?state={}&{query}", param(url, "state").unwrap())],
            );
            assert_eq!(code(&result), Some(Code::ProtectedEntryUnavailable));
            assert!(
                record.exchanged.is_empty(),
                "an error redirect was exchanged"
            );
            assert!(record.answers()[0].starts_with("HTTP/1.1 400 "));
        }
    }

    #[test]
    fn second_request_refused() {
        // Two redirects queued together: neither is trusted, nothing is exchanged.
        let (result, mut record) = run(
            &profile(&TRIPLE),
            &client(AUTHORIZE, TOKEN),
            BUDGET,
            |url| {
                vec![
                    consented(url),
                    consented(url).replace("fixture-code", "other"),
                ]
            },
        );
        assert_eq!(code(&result), Some(Code::ProtectedEntryUnavailable));
        assert!(record.exchanged.is_empty());
        let answers = record.answers();
        assert!(answers[0].starts_with("HTTP/1.1 400 "));
        assert!(answers[1].is_empty(), "the second request was answered");

        // After the one request a flow answers, its listener is gone.
        let (result, record) = run(
            &profile(&TRIPLE),
            &client(AUTHORIZE, TOKEN),
            BUDGET,
            |url| vec![consented(url)],
        );
        assert!(result.is_ok());
        let redirect = Url::parse(&param(&record.presented[0], "redirect_uri").unwrap()).unwrap();
        assert!(TcpStream::connect(("127.0.0.1", redirect.port().unwrap())).is_err());
        assert_eq!(record.exchanged.len(), 1);
    }

    #[test]
    fn timeout_stores_nothing() {
        let started = Instant::now();
        let (result, record) = run(
            &profile(&TRIPLE),
            &client(AUTHORIZE, TOKEN),
            Duration::from_millis(300),
            |_| Vec::new(),
        );
        assert_eq!(code(&result), Some(Code::Timeout));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(record.presented.len(), 1);
        assert!(record.exchanged.is_empty());
        let redirect = Url::parse(&param(&record.presented[0], "redirect_uri").unwrap()).unwrap();
        assert!(TcpStream::connect(("127.0.0.1", redirect.port().unwrap())).is_err());
    }

    #[test]
    fn client_file_uri_mismatch_refused() {
        for (auth_uri, token_uri) in [
            ("https://accounts.example.test/o/oauth2/v2/auth", TOKEN),
            (AUTHORIZE, "https://oauth2.example.test/other"),
            ("http://accounts.example.test/o/oauth2/auth", TOKEN),
        ] {
            let (result, record) = run(
                &profile(&TRIPLE),
                &client(auth_uri, token_uri),
                BUDGET,
                |url| vec![consented(url)],
            );
            assert_eq!(code(&result), Some(Code::ProtectedEntryUnavailable));
            assert!(record.presented.is_empty(), "consent was offered");
            assert!(record.exchanged.is_empty());
        }
    }

    #[test]
    fn profile_field_mismatch_refused() {
        for fields in [
            &["token"][..],
            &["client_id", "client_secret"][..],
            &["client_id", "client_secret", "refresh_token", "extra"][..],
            &["client_id", "client_id", "refresh_token"][..],
            &["client_id", "client_secret", "access_token"][..],
        ] {
            let (result, record) =
                run(&profile(fields), &client(AUTHORIZE, TOKEN), BUDGET, |url| {
                    vec![consented(url)]
                });
            assert_eq!(code(&result), Some(Code::ProtectedEntryUnavailable));
            assert!(record.presented.is_empty(), "consent was offered");
            assert!(record.exchanged.is_empty());
            // The same refusal through the CLI's entry point, before any consent.
            let refused = entry(
                &profile(fields),
                google(AUTHORIZE, TOKEN),
                connectors_sdk::now_ms() + 60_000,
            );
            assert_eq!(code(&refused), Some(Code::ProtectedEntryUnavailable));
        }
    }

    #[test]
    fn documents_other_than_a_google_client_pass_through() {
        let expires = connectors_sdk::now_ms() + 60_000;
        let triple = br#"{"client_id":"a","client_secret":"b","refresh_token":"c"}"#;
        let passed = entry(&profile(&TRIPLE), Secret(triple.to_vec()), expires);
        assert!(passed.is_ok_and(|secret| secret.0 == triple));
        let mut plain = profile(&TRIPLE);
        plain.acquisition = None;
        let google = google(AUTHORIZE, TOKEN);
        let bytes = google.0.clone();
        let passed = entry(&plain, google, expires);
        assert!(passed.is_ok_and(|secret| secret.0 == bytes));
        let web = br#"{"web":{"client_id":"a","client_secret":"b"}}"#;
        let passed = entry(&profile(&TRIPLE), Secret(web.to_vec()), expires);
        assert!(passed.is_ok_and(|secret| secret.0 == web));
    }

    const CANCEL_FIXTURE: &str = "CONNECTORS_OAUTH_CANCEL_FIXTURE";

    // Re-entered in its own process by `cancellation_stores_nothing`, which
    // interrupts it while the flow waits for the browser.
    #[test]
    fn cancellation_fixture() {
        let Some(ready) = std::env::var_os(CANCEL_FIXTURE) else {
            return;
        };
        let ready = std::path::PathBuf::from(ready);
        let signals = protected::Signals::install().unwrap();
        let mut exchanged = 0;
        let result = acquire(
            &profile(&TRIPLE),
            &client(AUTHORIZE, TOKEN),
            connectors_sdk::now_ms() + 60_000,
            Environment {
                present: &mut |url| {
                    let staged = ready.with_extension("staged");
                    std::fs::write(&staged, url).unwrap();
                    std::fs::rename(&staged, &ready).unwrap();
                },
                cancelled: &protected::cancellation,
                exchange: &mut |_| {
                    exchanged += 1;
                    Ok(Zeroizing::new(REFRESH.to_owned()))
                },
            },
        );
        assert_eq!(code(&result), Some(Code::Interrupted));
        assert!(signals.interrupted());
        assert_eq!(exchanged, 0);
    }

    #[test]
    fn cancellation_stores_nothing() {
        struct Child(std::process::Child);
        impl Drop for Child {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let root = tempfile::tempdir().unwrap();
        let ready = root.path().join("ready");
        let mut diagnostics = tempfile::tempfile().unwrap();
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "local::oauth::tests::cancellation_fixture",
                    "--nocapture",
                ])
                .env(CANCEL_FIXTURE, &ready)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(diagnostics.try_clone().unwrap())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(60);
        while !ready.exists() {
            assert!(Instant::now() < until, "the flow never showed consent");
            assert!(child.0.try_wait().unwrap().is_none(), "fixture ended early");
            std::thread::sleep(Duration::from_millis(10));
        }
        let url = Url::parse(&std::fs::read_to_string(&ready).unwrap()).unwrap();
        assert!(param(&url, "code_challenge").is_some());
        // SAFETY: signals only the child this test spawned and still owns.
        assert_eq!(
            unsafe { libc::kill(child.0.id() as libc::pid_t, libc::SIGINT) },
            0
        );
        let interrupted = Instant::now();
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                interrupted.elapsed() < Duration::from_secs(30),
                "the flow ignored the interrupt"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        let mut why = String::new();
        use std::io::Seek;
        diagnostics.rewind().unwrap();
        diagnostics.read_to_string(&mut why).unwrap();
        assert!(status.success(), "{why}");
    }
}
