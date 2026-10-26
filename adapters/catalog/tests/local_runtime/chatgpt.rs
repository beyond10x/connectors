//! The `chatgpt` connector's `chatgpt.subscription` profile
//! (story:chatgpt-subscription-connector): the shipped declaration, held to
//! a fake authorization server that answers the shapes the Codex CLI's
//! sign-in uses (`openai/codex@970111f6`, `codex-rs/login`). It answers the
//! authorize page, the form code exchange, the JSON refresh with rotation,
//! the two device sign-in endpoints and the connector's one read
//! (`models.list`) on one loopback TLS host whose self-signed root is the
//! only one trusted. Its tokens are fictional and only ever compared.
//!
//! The declaration ships no client id: an instance names one in its
//! configuration (`oauth2_client_id`), so an instance derived from the
//! declaration alone is invalid configuration. The connection's identity is
//! the ChatGPT account id the `id_token` carries in its
//! `https://api.openai.com/auth` claim, not its `sub`.
use super::oauth2_grant::unsigned_jwt;
use super::oauth2_refresh::{decode_form, field};
use super::*;

pub(super) const CHATGPT_PROFILE: &str = "chatgpt.subscription";
/// The client id the fixture's instances configure; obviously not a real one.
pub(super) const CHATGPT_CLIENT_ID: &str = "fixture-chatgpt-client";
/// The account id the fixture's `id_token`s carry.
pub(super) const CHATGPT_ACCOUNT: &str = "fixture-chatgpt-account-7";
/// The `sub` of the fixture's `id_token`s: never the connection's identity.
const CHATGPT_SUBJECT: &str = "fixture-chatgpt-user";
/// Prefixes of every token, code, verifier and device id the fixture issues;
/// no output may carry one.
pub(super) const CHATGPT_ACCESS: &str = "fixture-chatgpt-access-";
pub(super) const CHATGPT_REFRESH: &str = "fixture-chatgpt-refresh-";
const CHATGPT_CODE: &str = "fixture-chatgpt-code-";
const CHATGPT_DEVICE: &str = "fixture-chatgpt-device-";
const CHATGPT_VERIFIER: &str = "fixture-chatgpt-verifier-";
/// The scope the Codex CLI requests, as one string
/// (`codex-rs/login/src/server.rs#L597-L601`).
pub(super) const CHATGPT_SCOPE: &str =
    "openid profile email offline_access api.connectors.read api.connectors.invoke";
/// The instance id of every fixture configuration.
pub(super) const CHATGPT_INSTANCE: &str = "fixture-chatgpt";

/// Whether `haystack` carries any token, code, verifier or device id the
/// fixture issued.
pub(super) fn carries_chatgpt_material(haystack: &[u8]) -> bool {
    [
        CHATGPT_ACCESS,
        CHATGPT_REFRESH,
        CHATGPT_CODE,
        CHATGPT_DEVICE,
        CHATGPT_VERIFIER,
    ]
    .iter()
    .any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

/// One observed request: method, route, query, content type and body.
/// Compared by the tests, never printed.
#[derive(Clone)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) route: String,
    pub(super) query: Vec<(String, String)>,
    pub(super) content_type: Option<String>,
    pub(super) authorization: Option<String>,
    pub(super) body: Vec<u8>,
}

/// One authorization code the fixture issued: from a browser consent or a
/// device sign-in, bound to its PKCE challenge and redirect.
struct Code {
    code: String,
    challenge: String,
    redirect_uri: String,
    exchanged: bool,
}

/// One device sign-in: its id, the user code shown for it and how many polls
/// it answered.
struct DeviceSignIn {
    device_auth_id: String,
    user_code: String,
    polls: u32,
}

struct ChatgptState {
    port: u16,
    /// How long every access token the fixture grants is valid, in seconds.
    lifetime: u64,
    requests: Vec<Seen>,
    codes: Vec<Code>,
    devices: Vec<DeviceSignIn>,
    issued: u32,
    /// Refresh tokens a refresh spent: never accepted again.
    spent: Vec<String>,
}

enum Answer {
    Json(u16, Value),
    Redirect(String),
}

pub(super) struct ChatgptServer {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub(super) root: tempfile::TempDir,
    pub(super) ca: PathBuf,
    pub(super) config: PathBuf,
    pub(super) port: u16,
    state: Arc<Mutex<ChatgptState>>,
}

impl ChatgptServer {
    /// A fixture whose access tokens are valid for `lifetime` seconds, and
    /// its instance's configuration with the browser redirect on
    /// `redirect_port`.
    pub(super) fn new(lifetime: u64, redirect_port: u16) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
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
        let state = Arc::new(Mutex::new(ChatgptState {
            port: 0,
            lifetime,
            requests: Vec::new(),
            codes: Vec::new(),
            devices: Vec::new(),
            issued: 0,
            spent: Vec::new(),
        }));
        let served = state.clone();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {
                        _ = &mut stopped => break,
                        value = listener.accept() => value.unwrap().0,
                    };
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") {
                        assert!(head.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8(head).unwrap();
                    let header = |wanted: &str| {
                        head.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let (route, query) = match target.split_once('?') {
                        Some((route, query)) => (route.to_owned(), query.to_owned()),
                        None => (target.clone(), String::new()),
                    };
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let seen = Seen {
                        method,
                        route,
                        query: decode_form(query.as_bytes()),
                        content_type: header("content-type"),
                        authorization: header("authorization"),
                        body,
                    };
                    let answer = {
                        let mut state = served.lock().unwrap();
                        state.requests.push(seen.clone());
                        route_answer(&mut state, &seen)
                    };
                    let (status, location, bytes) = match answer {
                        Answer::Json(status, value) => {
                            (status, None, serde_json::to_vec(&value).unwrap())
                        }
                        Answer::Redirect(target) => (302, Some(target), b"{}".to_vec()),
                    };
                    let location = location
                        .map(|target| format!("Location: {target}\r\n"))
                        .unwrap_or_default();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\n{location}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&bytes).await;
                }
            });
        });
        let port = address_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .port();
        state.lock().unwrap().port = port;
        let config = directory.join("catalog.json");
        let server = Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            port,
            state,
        };
        server.write_config(&chatgpt_config(
            &profile_at(port, redirect_port),
            Some(CHATGPT_CLIENT_ID),
            Some((port, &server.ca)),
        ));
        server
    }

    pub(super) fn write_config(&self, config: &Value) {
        private(&self.config, &serde_json::to_vec(config).unwrap());
    }

    /// The configured selection of this fixture's provider child.
    pub(super) fn selection(&self) -> Adapter {
        let bootstrap = bootstrap_at(&self.config).expect("bootstrap inspection failed");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: CHATGPT_INSTANCE.into(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap["configuration_revision"]
                .as_str()
                .unwrap()
                .to_owned(),
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec![
                    "--local-config".into(),
                    self.config.to_str().unwrap().into(),
                ],
            },
        }
    }

    pub(super) fn requests(&self) -> Vec<Seen> {
        self.state.lock().unwrap().requests.clone()
    }

    /// The routes requested so far, in order, as `METHOD /route`.
    pub(super) fn routes(&self) -> Vec<String> {
        self.requests()
            .iter()
            .map(|seen| format!("{} {}", seen.method, seen.route))
            .collect()
    }

    /// How many access tokens the token endpoint issued.
    pub(super) fn issued(&self) -> u32 {
        self.state.lock().unwrap().issued
    }

    /// The refresh tokens refreshes spent, in order.
    pub(super) fn spent(&self) -> Vec<String> {
        self.state.lock().unwrap().spent.clone()
    }
}

impl Drop for ChatgptServer {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn route_answer(state: &mut ChatgptState, seen: &Seen) -> Answer {
    let refused = |status: u16, error: &str| Answer::Json(status, json!({"error": error}));
    let json_body = || -> Option<serde_json::Map<String, Value>> {
        if seen.content_type.as_deref() != Some("application/json") {
            return None;
        }
        serde_json::from_slice::<Value>(&seen.body)
            .ok()?
            .as_object()
            .cloned()
    };
    match (seen.method.as_str(), seen.route.as_str()) {
        ("GET", "/oauth/authorize") => match consent(state, &seen.query) {
            Some(target) => Answer::Redirect(target),
            None => refused(400, "invalid_request"),
        },
        ("POST", "/oauth/token") => {
            if seen.content_type.as_deref() == Some("application/x-www-form-urlencoded") {
                let form = decode_form(&seen.body);
                return match field(&form, "grant_type") {
                    Some("authorization_code") => exchange(state, &form),
                    // The Codex CLI refreshes with JSON only.
                    _ => refused(400, "unsupported_grant_type"),
                };
            }
            match json_body() {
                Some(body) if body.get("grant_type") == Some(&json!("refresh_token")) => {
                    refresh(state, &body)
                }
                _ => refused(400, "unsupported_grant_type"),
            }
        }
        ("POST", "/api/accounts/deviceauth/usercode") => {
            let Some(body) = json_body() else {
                return refused(400, "invalid_request");
            };
            if body.len() != 1 || body.get("client_id") != Some(&json!(CHATGPT_CLIENT_ID)) {
                return refused(400, "invalid_client");
            }
            let number = state.devices.len() + 1;
            let device = DeviceSignIn {
                device_auth_id: format!("{CHATGPT_DEVICE}{number}"),
                user_code: format!("FIXT-{number:04}"),
                polls: 0,
            };
            let answer = json!({
                "device_auth_id": device.device_auth_id,
                "user_code": device.user_code,
                "interval": "1",
            });
            state.devices.push(device);
            Answer::Json(200, answer)
        }
        // Pending (403) on the first poll, as while the person has not
        // entered the code; then the code and verifier for the exchange.
        ("POST", "/api/accounts/deviceauth/token") => {
            let Some(body) = json_body() else {
                return refused(400, "invalid_request");
            };
            let port = state.port;
            let Some(device) = state.devices.iter_mut().find(|device| {
                body.len() == 2
                    && body.get("device_auth_id") == Some(&json!(device.device_auth_id))
                    && body.get("user_code") == Some(&json!(device.user_code))
            }) else {
                return refused(400, "invalid_request");
            };
            device.polls += 1;
            if device.polls == 1 {
                return refused(403, "fixture pending");
            }
            let number = state.codes.len() + 1;
            let code = format!("{CHATGPT_CODE}device-{number}");
            let verifier = format!("{CHATGPT_VERIFIER}{number}-{}", "v".repeat(32));
            let challenge = challenge_of(&verifier);
            state.codes.push(Code {
                code: code.clone(),
                challenge: challenge.clone(),
                redirect_uri: format!("https://localhost:{port}/deviceauth/callback"),
                exchanged: false,
            });
            Answer::Json(
                200,
                json!({
                    "authorization_code": code,
                    "code_challenge": challenge,
                    "code_verifier": verifier,
                }),
            )
        }
        // The connector's one read, answered to an access token the fixture
        // issued.
        ("GET", "/backend-api/codex/models") => {
            let issued = (1..=state.issued)
                .any(|n| seen.authorization == Some(format!("Bearer {CHATGPT_ACCESS}{n}")));
            if issued {
                Answer::Json(200, json!({"models": [{"slug": "fixture-model"}]}))
            } else {
                refused(401, "fixture refusal")
            }
        }
        _ => refused(404, "fixture route"),
    }
}

/// The S256 challenge of `verifier` (RFC 7636 §4.2).
fn challenge_of(verifier: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Consent to an authorization request exactly as the Codex CLI makes it: the
/// configured client, the exact scope string, S256 PKCE, the extras, and the
/// fixed loopback redirect. `None` refuses anything else.
fn consent(state: &mut ChatgptState, query: &[(String, String)]) -> Option<String> {
    let redirect_uri = field(query, "redirect_uri")?;
    let challenge = field(query, "code_challenge")?;
    let request_state = field(query, "state")?;
    let well_formed = field(query, "response_type") == Some("code")
        && field(query, "client_id") == Some(CHATGPT_CLIENT_ID)
        && field(query, "scope") == Some(CHATGPT_SCOPE)
        && field(query, "code_challenge_method") == Some("S256")
        && field(query, "id_token_add_organizations") == Some("true")
        && field(query, "codex_cli_simplified_flow") == Some("true")
        && field(query, "originator") == Some("codex_cli_rs")
        && redirect_uri.starts_with("http://127.0.0.1:")
        && redirect_uri.ends_with("/auth/callback")
        && challenge.len() == 43
        && !request_state.is_empty();
    if !well_formed {
        return None;
    }
    let code = format!("{CHATGPT_CODE}{}", state.codes.len() + 1);
    state.codes.push(Code {
        code: code.clone(),
        challenge: challenge.to_owned(),
        redirect_uri: redirect_uri.to_owned(),
        exchanged: false,
    });
    Some(format!("{redirect_uri}?code={code}&state={request_state}"))
}

/// Exchanges an unused code, browser or device, whose verifier matches its
/// challenge, for its redirect and the configured public client; anything
/// else is `invalid_grant`.
fn exchange(state: &mut ChatgptState, form: &[(String, String)]) -> Answer {
    let verifier = field(form, "code_verifier").unwrap_or_default();
    let challenge = challenge_of(verifier);
    let client = field(form, "client_id") == Some(CHATGPT_CLIENT_ID)
        && field(form, "client_secret").is_none();
    let Some(code) = state.codes.iter_mut().find(|code| {
        client
            && !code.exchanged
            && Some(code.code.as_str()) == field(form, "code")
            && code.challenge == challenge
            && Some(code.redirect_uri.as_str()) == field(form, "redirect_uri")
    }) else {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    };
    code.exchanged = true;
    grant(state)
}

/// A rotating refresh of an issued refresh token no refresh spent yet, sent
/// as the JSON `{grant_type, client_id, refresh_token}` and nothing else (no
/// `scope`); the token it was sent is spent.
fn refresh(state: &mut ChatgptState, body: &serde_json::Map<String, Value>) -> Answer {
    let presented = body
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let issued = (1..=state.issued).any(|n| presented == format!("{CHATGPT_REFRESH}{n}"));
    if body.len() != 3
        || body.get("client_id") != Some(&json!(CHATGPT_CLIENT_ID))
        || !issued
        || state.spent.contains(&presented)
    {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    }
    state.spent.push(presented);
    grant(state)
}

/// A token response with a new access and refresh token and an `id_token`
/// naming the fixture account.
fn grant(state: &mut ChatgptState) -> Answer {
    state.issued += 1;
    Answer::Json(
        200,
        json!({
            "access_token": format!("{CHATGPT_ACCESS}{}", state.issued),
            "token_type": "Bearer",
            "expires_in": state.lifetime,
            "refresh_token": format!("{CHATGPT_REFRESH}{}", state.issued),
            "id_token": unsigned_jwt(&claims(state.port, CHATGPT_CLIENT_ID, Some(CHATGPT_ACCOUNT))),
        }),
    )
}

/// The claims of a fixture `id_token` issued to `audience`, carrying
/// `account` in the ChatGPT auth claim when it is given.
fn claims(port: u16, audience: &str, account: Option<&str>) -> Value {
    let mut claims = json!({
        "iss": format!("https://localhost:{port}"),
        "aud": audience,
        "sub": CHATGPT_SUBJECT,
        "iat": 1,
        "exp": 4_000_000_000_u64,
        "email": "fixture@example.test",
    });
    if let Some(account) = account {
        claims["https://api.openai.com/auth"] = json!({
            "chatgpt_account_id": account,
            "chatgpt_plan_type": "fixture",
        });
    }
    claims
}

/// The shipped declaration of the `chatgpt` connector.
pub(super) fn shipped() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("providers/chatgpt/connector.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// The shipped profile with its authorization server moved to the fixture at
/// `port` and its fixed loopback redirect to `redirect_port`; every other
/// value as shipped.
pub(super) fn profile_at(port: u16, redirect_port: u16) -> Value {
    let mut profile = shipped()["profiles"][0].clone();
    let oauth2 = &mut profile["oauth2"];
    oauth2["authorize_url"] = json!(format!("https://localhost:{port}/oauth/authorize"));
    oauth2["token_url"] = json!(format!("https://localhost:{port}/oauth/token"));
    oauth2["redirect_uri"] = json!(format!("http://127.0.0.1:{redirect_port}/auth/callback"));
    profile
}

/// A hand-written catalog configuration of the `chatgpt` provider with
/// `profile`, naming `client_id` as its `oauth2_client_id` when given, and
/// served by the fixture at `fixture` (its port and root) or the shipped
/// endpoint.
pub(super) fn chatgpt_config(
    profile: &Value,
    client_id: Option<&str>,
    fixture: Option<(u16, &Path)>,
) -> Value {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut config = json!({
        "format": "connectors-catalog-local/2",
        "instance": CHATGPT_INSTANCE,
        "provider": "chatgpt",
        "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
        "api_base": "https://chatgpt.com/backend-api/codex",
        "auth": profile,
        "operations_file": repository.join("providers/chatgpt/operations.json").canonicalize().unwrap(),
    });
    if let Some((port, ca)) = fixture {
        config["api_base"] = json!(format!("https://localhost:{port}/backend-api/codex"));
        config["ca_file"] = json!(ca);
    }
    if let Some(client_id) = client_id {
        config["oauth2_client_id"] = json!(client_id);
    }
    config
}

/// The printed bootstrap of the configuration at `path`, or the refusal's
/// stderr.
fn bootstrap_at(path: &Path) -> Result<Value, String> {
    printed(
        Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(path)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap(),
    )
}

/// The bootstrap a run printed, or its refusal's stderr.
fn printed(output: std::process::Output) -> Result<Value, String> {
    assert!(!carries_chatgpt_material(&output.stdout));
    assert!(!carries_chatgpt_material(&output.stderr));
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    assert!(output.stderr.is_empty());
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    serde_json::from_value::<Bootstrap>(bootstrap.clone())
        .unwrap()
        .validate()
        .unwrap();
    Ok(bootstrap)
}

/// [`bootstrap_at`] for a configuration written to a fresh private file.
fn bootstrap(config: &Value) -> Result<Value, String> {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(config).unwrap());
    bootstrap_at(&path)
}

/// A stored grant entry for access token number `issued` with `id_token`.
fn entry(issued: u32, id_token: Option<String>) -> Secret {
    let mut entry = json!({
        "client_id": CHATGPT_CLIENT_ID,
        "access_token": format!("{CHATGPT_ACCESS}{issued}"),
        "refresh_token": format!("{CHATGPT_REFRESH}{issued}"),
        "expires_at_ms": connectors_sdk::now_ms() + 3_600_000,
        "scopes": CHATGPT_SCOPE.split(' ').collect::<Vec<_>>(),
    });
    if let Some(id_token) = id_token {
        entry["id_token"] = json!(id_token);
    }
    Secret(serde_json::to_vec(&entry).unwrap())
}

/// The shipped profile names no client id, so neither a derived instance nor
/// a hand-written one without `oauth2_client_id` has a client to run the
/// grant as: both are invalid configuration. One that names a client id
/// carries it, and the rest of the declaration as shipped, in its profile's
/// acquisition. Nothing here reaches the network.
#[test]
fn chatgpt_profile_without_client_id_is_invalid_configuration() {
    let declaration = shipped();
    let profile = &declaration["profiles"][0];
    assert_eq!(profile["profile"], CHATGPT_PROFILE);
    assert_eq!(profile["scheme"], "oauth2_grant");
    assert!(
        profile["oauth2"]["client_id"].is_null(),
        "no client id is shipped"
    );
    // A derived instance carries no client id.
    let derived = printed(
        Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .args([
                "--connector",
                "chatgpt",
                "--instance",
                CHATGPT_INSTANCE,
                "--print-local-bootstrap",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(derived.unwrap_err(), "InvalidConfiguration");
    // Nor does a hand-written one that names none, or an empty one.
    assert_eq!(
        bootstrap(&chatgpt_config(profile, None, None)).unwrap_err(),
        "InvalidConfiguration"
    );
    assert_eq!(
        bootstrap(&chatgpt_config(profile, Some(""), None)).unwrap_err(),
        "InvalidConfiguration"
    );
    // The instance's configured client id is the one the grant runs as.
    let printed = bootstrap(&chatgpt_config(profile, Some(CHATGPT_CLIENT_ID), None)).unwrap();
    let profiles = printed["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0]["id"], CHATGPT_PROFILE);
    let carried = &profiles[0]["acquisition"]["oauth2"];
    assert_eq!(carried["client_id"], CHATGPT_CLIENT_ID);
    for (name, value) in profile["oauth2"].as_object().unwrap() {
        if name != "client_id" {
            assert_eq!(&carried[name], value, "field {name}");
        }
    }
    assert_eq!(carried["refresh_encoding"], "json");
    assert_eq!(carried["request_encoding"], "form");
    assert_eq!(carried["refresh_lead_seconds"], 300);
    assert_eq!(carried["requested_scopes"].as_array().unwrap().len(), 6);
}

/// The connection's identity is the ChatGPT account id in the `id_token`'s
/// `https://api.openai.com/auth` claim, read without any request: not the
/// token's `sub`. An `id_token` without that claim, issued to another client,
/// or absent names nobody.
#[test]
fn chatgpt_identity_is_the_account_id_of_the_id_token() {
    let server = ChatgptServer::new(3600, 1455);
    let mut child = Child::spawn(&server.selection()).unwrap();
    let port = server.port;
    let baseline = child
        .validate(
            CHATGPT_PROFILE,
            &entry(
                1,
                Some(unsigned_jwt(&claims(
                    port,
                    CHATGPT_CLIENT_ID,
                    Some(CHATGPT_ACCOUNT),
                ))),
            ),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.kind, "chatgpt.account");
    assert_eq!(baseline.identity.subject, CHATGPT_ACCOUNT);
    assert!(server.requests().is_empty(), "the id_token needs no read");
    for id_token in [
        Some(unsigned_jwt(&claims(port, CHATGPT_CLIENT_ID, None))),
        Some(unsigned_jwt(&claims(
            port,
            "another-client",
            Some(CHATGPT_ACCOUNT),
        ))),
        None,
    ] {
        assert!(
            child
                .validate(CHATGPT_PROFILE, &entry(1, id_token), deadline())
                .is_err()
        );
    }
    // A claim that is not a non-empty string names nobody either.
    for account in [json!(""), json!(7), json!({"id": CHATGPT_ACCOUNT})] {
        let mut claims = claims(port, CHATGPT_CLIENT_ID, Some(CHATGPT_ACCOUNT));
        claims["https://api.openai.com/auth"]["chatgpt_account_id"] = account;
        assert!(
            child
                .validate(
                    CHATGPT_PROFILE,
                    &entry(1, Some(unsigned_jwt(&claims))),
                    deadline()
                )
                .is_err()
        );
    }
    assert!(server.requests().is_empty());
}
