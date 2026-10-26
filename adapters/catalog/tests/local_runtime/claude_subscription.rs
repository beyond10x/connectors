//! The shipped `claude-subscription` connector (story:claude-subscription-connector): its
//! declaration ships no client id, so an instance runs only with the `oauth2_client_id` its own
//! configuration names. The fixture below answers the shapes of the story's Provider facts on
//! one loopback TLS host: the authorize page requires the extra `code=true`, the token endpoint
//! reads JSON only, the code exchange carries the flow's `state`, a refresh carries `scope` and
//! answers no refresh token (the old one is kept), and the identity is the account `uuid` the
//! profile read answers. Its tokens are fictional and only ever compared.
use super::oauth2_refresh::{decode_form, field};
use super::*;
use std::collections::BTreeMap;

/// The connector, its directory and its two profiles.
pub(super) const CLAUDE_CONNECTOR: &str = "claude-subscription";
pub(super) const CLAUDE_LOOPBACK: &str = "claude.subscription";
pub(super) const CLAUDE_PASTED: &str = "claude.subscription-pasted";
/// The client id the fixture's instance configures; the declaration names none.
pub(super) const CLAUDE_CLIENT_ID: &str = "fixture-claude-client";
/// The scopes every grant requests and must hold, in the declared order.
pub(super) const CLAUDE_SCOPES: [&str; 2] = ["user:profile", "user:inference"];
/// The account the profile read answers.
pub(super) const CLAUDE_ACCOUNT: &str = "fixture-account-0000";
/// Prefixes of every access token, refresh token and code the fixture issues.
pub(super) const CLAUDE_ACCESS: &str = "fixture-claude-access-";
pub(super) const CLAUDE_REFRESH: &str = "fixture-claude-refresh-";
const CLAUDE_CODE: &str = "fixture-claude-code-";

/// The fields of the code exchange (`Tqo`) and of a refresh (`koe`), as the story states them.
pub(super) const EXCHANGE_FIELDS: [&str; 6] = [
    "client_id",
    "code",
    "code_verifier",
    "grant_type",
    "redirect_uri",
    "state",
];
pub(super) const REFRESH_FIELDS: [&str; 4] = ["client_id", "grant_type", "refresh_token", "scope"];

/// Whether `haystack` carries any token or code the fixture issued.
pub(super) fn carries_claude_material(haystack: &[u8]) -> bool {
    [CLAUDE_ACCESS, CLAUDE_REFRESH, CLAUDE_CODE]
        .iter()
        .any(|needle| {
            haystack
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
        })
}

/// How long the fixture's access tokens live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lifetime {
    /// An hour, from the exchange and from every refresh.
    Hour,
    /// 120 s, inside the declared 300 s lead, from the exchange and from every refresh.
    Short,
}

/// One consent: its code, PKCE challenge, redirect and `state`.
struct Consent {
    code: String,
    challenge: String,
    redirect_uri: String,
    state: String,
    exchanged: bool,
}

/// One token-endpoint request: its content type and its JSON object, values kept only to be
/// compared.
#[derive(Clone, Debug)]
pub(super) struct TokenRequest {
    pub(super) content_type: Option<String>,
    pub(super) fields: BTreeMap<String, String>,
}

impl TokenRequest {
    pub(super) fn keys(&self) -> Vec<&str> {
        self.fields.keys().map(String::as_str).collect()
    }
}

/// One observed request: method, route, query and `Authorization` header.
#[derive(Clone)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) route: String,
    pub(super) query: String,
    pub(super) authorization: Option<String>,
}

struct ClaudeState {
    port: u16,
    lifetime: Lifetime,
    requests: Vec<Seen>,
    consents: Vec<Consent>,
    token_requests: Vec<TokenRequest>,
    issued: u32,
    /// Refresh tokens issued; a refresh answers none, so the first stays current.
    refresh_tokens: u32,
}

enum Answer {
    Json(u16, Value),
    Redirect(String),
    Text(String),
}

pub(super) struct ClaudeServer {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub(super) root: tempfile::TempDir,
    pub(super) ca: PathBuf,
    pub(super) config: PathBuf,
    pub(super) port: u16,
    state: Arc<Mutex<ClaudeState>>,
}

impl ClaudeServer {
    pub(super) fn new(lifetime: Lifetime) -> Self {
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
        let state = Arc::new(Mutex::new(ClaudeState {
            port: 0,
            lifetime,
            requests: Vec::new(),
            consents: Vec::new(),
            token_requests: Vec::new(),
            issued: 0,
            refresh_tokens: 0,
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
                    let answer = {
                        let mut state = served.lock().unwrap();
                        state.requests.push(Seen {
                            method: method.clone(),
                            route: route.clone(),
                            query: query.clone(),
                            authorization: header("authorization"),
                        });
                        route_answer(
                            &mut state,
                            &method,
                            &route,
                            &decode_form(query.as_bytes()),
                            header("content-type"),
                            &body,
                            header("authorization"),
                        )
                    };
                    let (status, content_type, location, bytes) = match answer {
                        Answer::Json(status, value) => (
                            status,
                            "application/json",
                            None,
                            serde_json::to_vec(&value).unwrap(),
                        ),
                        Answer::Redirect(target) => {
                            (302, "application/json", Some(target), b"{}".to_vec())
                        }
                        Answer::Text(text) => (200, "text/plain", None, text.into_bytes()),
                    };
                    let location = location
                        .map(|target| format!("Location: {target}\r\n"))
                        .unwrap_or_default();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\n{location}Content-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            port,
            state,
        }
    }

    pub(super) fn write_config(&self, config: &Value) {
        private(&self.config, &serde_json::to_vec(config).unwrap());
    }

    /// The pinned selection of this fixture's provider child, instance `fixture-claude`.
    pub(super) fn selection(&self) -> Adapter {
        let bootstrap = bootstrap_at(&self.config).expect("bootstrap inspection failed");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-claude".into(),
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

    /// Every token-endpoint request, in order.
    pub(super) fn token_requests(&self) -> Vec<TokenRequest> {
        self.state.lock().unwrap().token_requests.clone()
    }

    /// The `Authorization` header of every profile read, in order.
    pub(super) fn profile_authorizations(&self) -> Vec<Option<String>> {
        self.requests()
            .into_iter()
            .filter(|seen| seen.route == "/api/oauth/profile")
            .map(|seen| seen.authorization)
            .collect()
    }
}

impl Drop for ClaudeServer {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn route_answer(
    state: &mut ClaudeState,
    method: &str,
    route: &str,
    query: &[(String, String)],
    content_type: Option<String>,
    body: &[u8],
    authorization: Option<String>,
) -> Answer {
    let refused = |error: &str| Answer::Json(400, json!({"error": error}));
    match (method, route) {
        ("GET", "/cai/oauth/authorize") => match consent(state, query) {
            Some(target) => Answer::Redirect(target),
            None => refused("invalid_request"),
        },
        // The page the pasted-code redirect shows: `code#state` on one line.
        ("GET", "/oauth/code/callback") => match (field(query, "code"), field(query, "state")) {
            (Some(code), Some(flow)) => Answer::Text(format!("{code}#{flow}\n")),
            _ => refused("invalid_request"),
        },
        ("POST", "/v1/oauth/token") => {
            // JSON only: a form body is not read.
            let fields: Option<BTreeMap<String, String>> = content_type
                .as_deref()
                .filter(|kind| kind.starts_with("application/json"))
                .and_then(|_| serde_json::from_slice(body).ok());
            let request = TokenRequest {
                content_type: content_type.clone(),
                fields: fields.clone().unwrap_or_default(),
            };
            state.token_requests.push(request);
            let Some(fields) = fields else {
                return refused("invalid_request");
            };
            match fields.get("grant_type").map(String::as_str) {
                Some("authorization_code") => exchange(state, &fields),
                Some("refresh_token") => refresh(state, &fields),
                _ => refused("unsupported_grant_type"),
            }
        }
        ("GET", "/api/oauth/profile") => {
            let issued = (1..=state.issued)
                .any(|n| authorization == Some(format!("Bearer {CLAUDE_ACCESS}{n}")));
            if issued {
                Answer::Json(
                    200,
                    json!({
                        "account": {"uuid": CLAUDE_ACCOUNT, "email_address": "fixture-claude@example.test"},
                        "organization": {"uuid": "fixture-organization-0000"},
                    }),
                )
            } else {
                Answer::Json(401, json!({"error": "fixture refusal"}))
            }
        }
        _ => Answer::Json(404, json!({"error": "fixture route"})),
    }
}

/// Consent to a well-formed authorization request carrying `code=true`.
fn consent(state: &mut ClaudeState, query: &[(String, String)]) -> Option<String> {
    let scopes: Vec<&str> = field(query, "scope")?.split(' ').collect();
    let redirect_uri = field(query, "redirect_uri")?;
    let challenge = field(query, "code_challenge")?;
    let request_state = field(query, "state")?;
    let pasted = format!("https://localhost:{}/oauth/code/callback", state.port);
    let loopback = redirect_uri
        .strip_prefix("http://localhost:")
        .and_then(|rest| rest.strip_suffix("/callback"))
        .is_some_and(|port| port.parse::<u16>().is_ok());
    let well_formed = field(query, "response_type") == Some("code")
        && field(query, "client_id") == Some(CLAUDE_CLIENT_ID)
        && field(query, "code") == Some("true")
        && field(query, "code_challenge_method") == Some("S256")
        && CLAUDE_SCOPES.iter().all(|scope| scopes.contains(scope))
        && (loopback || redirect_uri == pasted)
        && challenge.len() == 43
        && !request_state.is_empty();
    if !well_formed {
        return None;
    }
    let code = format!("{CLAUDE_CODE}{}", state.consents.len() + 1);
    state.consents.push(Consent {
        code: code.clone(),
        challenge: challenge.to_owned(),
        redirect_uri: redirect_uri.to_owned(),
        state: request_state.to_owned(),
        exchanged: false,
    });
    Some(format!("{redirect_uri}?code={code}&state={request_state}"))
}

/// Exchanges an unused code whose verifier matches its challenge, for the same redirect and
/// public client, carrying exactly the story's fields with the flow's own `state`.
fn exchange(state: &mut ClaudeState, fields: &BTreeMap<String, String>) -> Answer {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let get = |name: &str| fields.get(name).map(String::as_str);
    let verifier = get("code_verifier").unwrap_or_default();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let shaped = fields.keys().map(String::as_str).eq(EXCHANGE_FIELDS)
        && get("client_id") == Some(CLAUDE_CLIENT_ID)
        && (43..=128).contains(&verifier.len());
    let Some(consent) = state.consents.iter_mut().find(|consent| {
        shaped
            && !consent.exchanged
            && Some(consent.code.as_str()) == get("code")
            && consent.challenge == challenge
            && Some(consent.redirect_uri.as_str()) == get("redirect_uri")
            && Some(consent.state.as_str()) == get("state")
    }) else {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    };
    consent.exchanged = true;
    state.issued += 1;
    state.refresh_tokens += 1;
    Answer::Json(
        200,
        json!({
            "token_type": "Bearer",
            "access_token": format!("{CLAUDE_ACCESS}{}", state.issued),
            "refresh_token": format!("{CLAUDE_REFRESH}{}", state.refresh_tokens),
            "expires_in": lifetime(state.lifetime),
            "scope": CLAUDE_SCOPES.join(" "),
            "account": {"uuid": CLAUDE_ACCOUNT, "email_address": "fixture-claude@example.test"},
            "organization": {"uuid": "fixture-organization-0000"},
        }),
    )
}

/// A refresh carrying exactly the story's fields, `scope` the requested scopes, of a refresh
/// token the fixture issued: a new access token and no refresh token, so the one sent stays.
fn refresh(state: &mut ClaudeState, fields: &BTreeMap<String, String>) -> Answer {
    let get = |name: &str| fields.get(name).map(String::as_str);
    let presented = get("refresh_token").unwrap_or_default();
    let known = (1..=state.refresh_tokens).any(|n| presented == format!("{CLAUDE_REFRESH}{n}"));
    if !fields.keys().map(String::as_str).eq(REFRESH_FIELDS)
        || get("client_id") != Some(CLAUDE_CLIENT_ID)
        || get("scope") != Some(CLAUDE_SCOPES.join(" ").as_str())
        || !known
    {
        return Answer::Json(400, json!({"error": "invalid_request"}));
    }
    state.issued += 1;
    Answer::Json(
        200,
        json!({
            "token_type": "Bearer",
            "access_token": format!("{CLAUDE_ACCESS}{}", state.issued),
            "expires_in": lifetime(state.lifetime),
            "scope": CLAUDE_SCOPES.join(" "),
        }),
    )
}

fn lifetime(lifetime: Lifetime) -> u64 {
    match lifetime {
        Lifetime::Hour => 3600,
        Lifetime::Short => 120,
    }
}

/// The shipped declaration.
pub(super) fn shipped_declaration() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("providers")
        .join(CLAUDE_CONNECTOR)
        .join("connector.json");
    serde_json::from_slice(&fs::read(path).expect("the claude-subscription declaration")).unwrap()
}

/// The shipped profile `profile`, as declared.
pub(super) fn shipped_profile(profile: &str) -> Value {
    shipped_declaration()["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declared| declared["profile"] == profile)
        .unwrap_or_else(|| panic!("the declaration names no {profile}"))
        .clone()
}

/// The provider's hosts as the fixture serves them: every endpoint of the shipped profile
/// `profile` moved onto the fixture's origin, path kept, and nothing else changed.
fn on_fixture(profile: &str, port: u16) -> Value {
    let mut auth = shipped_profile(profile);
    let origin = format!("https://localhost:{port}");
    let oauth2 = &mut auth["oauth2"];
    for key in ["authorize_url", "token_url"] {
        let declared = oauth2[key].as_str().unwrap();
        let path = &declared[declared["https://".len()..].find('/').unwrap() + "https://".len()..];
        oauth2[key] = json!(format!("{origin}{path}"));
    }
    if oauth2["redirect_kind"] == "pasted_code" {
        oauth2["redirect_uri"] = json!(format!("{origin}/oauth/code/callback"));
    }
    auth
}

/// The native configuration of an instance of the shipped profile `profile` against the
/// fixture at `port`, with `client_id` as its `oauth2_client_id` when given.
pub(super) fn claude_config(port: u16, ca: &Path, profile: &str, client_id: Option<&str>) -> Value {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut config = json!({
        "format": "connectors-catalog-local/2",
        "instance": "fixture-claude",
        "provider": CLAUDE_CONNECTOR,
        "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
        "api_base": format!("https://localhost:{port}"),
        "ca_file": ca,
        "auth": on_fixture(profile, port),
        "operations_file": repository
            .join("providers")
            .join(CLAUDE_CONNECTOR)
            .join("operations.json")
            .canonicalize()
            .unwrap(),
    });
    if let Some(client_id) = client_id {
        config["oauth2_client_id"] = json!(client_id);
    }
    config
}

/// The printed bootstrap of the configuration at `path`, or the refusal's stderr.
fn bootstrap_at(path: &Path) -> Result<Value, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    printed(output)
}

fn printed(output: std::process::Output) -> Result<Value, String> {
    assert!(!carries_claude_material(&output.stdout));
    assert!(!carries_claude_material(&output.stderr));
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
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

/// Acceptance: the shipped profiles name no client id, so neither a native configuration
/// without `oauth2_client_id` nor an instance derived from the connector name alone can run
/// them; the instance's configured client id is the one the grant runs as.
#[test]
fn claude_profile_without_client_id_is_invalid_configuration() {
    let server = ClaudeServer::new(Lifetime::Hour);
    for profile in [CLAUDE_LOOPBACK, CLAUDE_PASTED] {
        let declared = shipped_profile(profile);
        assert_eq!(declared["scheme"], "oauth2_grant", "{profile}");
        assert!(
            declared["oauth2"]["client_id"].is_null(),
            "{profile} ships a client id"
        );
        let without = claude_config(server.port, &server.ca, profile, None);
        assert_eq!(
            bootstrap(&without).unwrap_err(),
            "InvalidConfiguration",
            "{profile}"
        );
        let with = claude_config(server.port, &server.ca, profile, Some(CLAUDE_CLIENT_ID));
        let printed = bootstrap(&with).unwrap_or_else(|error| panic!("{profile}: {error}"));
        let carried = &printed["profiles"][0]["acquisition"]["oauth2"];
        assert_eq!(carried["client_id"], CLAUDE_CLIENT_ID, "{profile}");
        assert_eq!(carried["request_encoding"], "json");
        assert_eq!(carried["refresh_encoding"], "json");
        assert_eq!(carried["state_in_exchange"], true);
        assert_eq!(
            carried["authorize_parameters"],
            json!([{"name": "code", "value": "true"}])
        );
        // A derived instance has no configuration to name a client id in.
        let derived = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .args([
                "--connector",
                CLAUDE_CONNECTOR,
                "--instance",
                "fixture",
                "--profile",
                profile,
                "--print-local-bootstrap",
            ])
            .output()
            .unwrap();
        assert_eq!(
            printed_refusal(derived),
            "InvalidConfiguration",
            "{profile}"
        );
    }
}

fn printed_refusal(output: std::process::Output) -> String {
    printed(output).expect_err("refused")
}
