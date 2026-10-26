//! The `oauth2_grant` profile: a profile that declares one OAuth 2.0
//! authorization server (`connectors.oauth2.ServerDeclaration`), whose grant
//! the CLI runs and whose token response the owner stores as the entry
//! `{client_id, access_token, refresh_token, expires_at_ms, scopes, id_token?}`.
//!
//! The catalog reads the declaration, resolves its client id and refuses one it
//! cannot use. The provider child validates a stored entry: the granted scopes
//! must hold the profile's minimum, and the identity is read with the access
//! token, or from the `id_token` the token endpoint returned. The fixture below
//! is the authorization server the CLI journeys run their grants against: it
//! answers the authorize, callback, token, device and API routes on one
//! loopback TLS host. Its tokens are fictional and only ever compared.
use super::oauth2_refresh::{decode_form, field};
use super::*;

pub(super) const GRANT_PROFILE: &str = "fixture.grant";
pub(super) const GRANT_CLIENT_ID: &str = "fixture-grant-client";
/// The scope every grant must hold (`required_scopes` and `minimum_scopes`).
pub(super) const GRANT_SCOPE: &str = "fixture.read";
/// A second requested scope the fixture grants beside it.
pub(super) const GRANT_EXTRA_SCOPE: &str = "fixture.profile";
/// Prefixes of every access token, refresh token, code and device code the
/// fixture issues; no output may carry one.
pub(super) const GRANT_ACCESS: &str = "fixture-grant-access-";
pub(super) const GRANT_REFRESH: &str = "fixture-grant-refresh-";
const GRANT_CODE: &str = "fixture-grant-code-";
const DEVICE_CODE: &str = "fixture-grant-device-";
/// The subject the identity read answers.
pub(super) const GRANT_SUBJECT: &str = "4242";

/// Whether `haystack` carries any token, code or device code the fixture issued.
pub(super) fn carries_grant_material(haystack: &[u8]) -> bool {
    [GRANT_ACCESS, GRANT_REFRESH, GRANT_CODE, DEVICE_CODE]
        .iter()
        .any(|needle| {
            haystack
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
        })
}

/// How the fixture answers a grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Behaviour {
    /// A person consents; the token endpoint grants every requested scope.
    Consent,
    /// The authorize redirect carries a `state` the flow never sent.
    ForgedState,
    /// The token endpoint grants only the extra scope, not the required one.
    NarrowScope,
    /// The device code is answered `expired_token` on its first poll.
    DeviceExpired,
    /// As `Consent`, but the granted access token is valid for 120 s, inside
    /// the declaration's 300 s `refresh_lead_seconds`; a refresh grants one
    /// valid for an hour.
    ShortLived,
    /// As `ShortLived`, but every refresh is answered `invalid_grant`.
    RefreshRefused,
}

/// One consent the fixture granted: its code, the PKCE challenge and the
/// redirect it was issued for, and whether the code was exchanged.
struct Consent {
    code: String,
    challenge: String,
    redirect_uri: String,
    exchanged: bool,
}

/// One device authorization: its code, the user code shown for it, and
/// whether the person approved it and the device exchanged it.
struct Device {
    device_code: String,
    user_code: String,
    approved: bool,
    exchanged: bool,
}

/// One observed request: method, route, and the `Authorization` header.
/// Compared by the tests, never printed.
#[derive(Clone)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) route: String,
    pub(super) query: String,
    pub(super) authorization: Option<String>,
}

struct GrantState {
    port: u16,
    behaviour: Behaviour,
    requests: Vec<Seen>,
    consents: Vec<Consent>,
    devices: Vec<Device>,
    issued: u32,
    /// Refresh requests answered, refused ones included.
    refreshes: u32,
    /// Refresh tokens a refresh spent: never accepted again.
    spent: Vec<String>,
}

/// What one route answers.
enum Answer {
    Json(u16, Value),
    Redirect(String),
    Text(String),
}

pub(super) struct GrantServer {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub(super) root: tempfile::TempDir,
    pub(super) ca: PathBuf,
    pub(super) config: PathBuf,
    pub(super) port: u16,
    state: Arc<Mutex<GrantState>>,
}

impl GrantServer {
    pub(super) fn new(behaviour: Behaviour) -> Self {
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
        let state = Arc::new(Mutex::new(GrantState {
            port: 0,
            behaviour,
            requests: Vec::new(),
            consents: Vec::new(),
            devices: Vec::new(),
            issued: 0,
            refreshes: 0,
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
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
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
                            &decode_form(&body),
                            header("authorization"),
                        )
                    };
                    let (status, content_type, location, bytes) = match answer {
                        Answer::Json(status, value) => {
                            (status, "application/json", None, serde_json::to_vec(&value).unwrap())
                        }
                        Answer::Redirect(target) => (302, "application/json", Some(target), b"{}".to_vec()),
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
        let server = Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            port,
            state,
        };
        server.write_config(&grant_config(
            port,
            &server.ca,
            declaration(port, Redirect::AnyPort),
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
            instance_id: "fixture-grant".into(),
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

    /// How many refresh requests the token endpoint answered.
    pub(super) fn refreshes(&self) -> u32 {
        self.state.lock().unwrap().refreshes
    }

    /// The `Authorization` header of every API request, in order.
    pub(super) fn api_authorizations(&self) -> Vec<Option<String>> {
        self.requests()
            .into_iter()
            .filter(|seen| seen.route.starts_with("/api/v4/"))
            .map(|seen| seen.authorization)
            .collect()
    }
}

impl Drop for GrantServer {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn route_answer(
    state: &mut GrantState,
    method: &str,
    route: &str,
    query: &[(String, String)],
    form: &[(String, String)],
    authorization: Option<String>,
) -> Answer {
    let refused = |error: &str| Answer::Json(400, json!({"error": error}));
    match (method, route) {
        ("GET", "/authorize") => match consent(state, query) {
            Some(target) => Answer::Redirect(target),
            None => refused("invalid_request"),
        },
        // The page a provider shows for a pasted-code redirect: the code and
        // state to copy, as one `code#state` line.
        ("GET", "/callback") => match (field(query, "code"), field(query, "state")) {
            (Some(code), Some(flow)) => Answer::Text(format!("{code}#{flow}\n")),
            _ => refused("invalid_request"),
        },
        ("POST", "/token") => match field(form, "grant_type") {
            Some("authorization_code") => exchange(state, form),
            Some("urn:ietf:params:oauth:grant-type:device_code") => poll(state, form),
            Some("refresh_token") => refresh(state, form),
            _ => refused("unsupported_grant_type"),
        },
        ("POST", "/device/code") => {
            let scopes: Vec<&str> = field(form, "scope")
                .unwrap_or_default()
                .split(' ')
                .collect();
            if field(form, "client_id") != Some(GRANT_CLIENT_ID) || !scopes.contains(&GRANT_SCOPE) {
                return refused("invalid_client");
            }
            let number = state.devices.len() + 1;
            let user_code = format!("FIXT-{number:04}");
            state.devices.push(Device {
                device_code: format!("{DEVICE_CODE}{number}"),
                user_code: user_code.clone(),
                approved: false,
                exchanged: false,
            });
            let port = state.port;
            Answer::Json(
                200,
                json!({
                    "device_code": format!("{DEVICE_CODE}{number}"),
                    "user_code": user_code,
                    "verification_uri": format!("https://localhost:{port}/device"),
                    "verification_uri_complete": format!("https://localhost:{port}/device?user_code={user_code}"),
                    "expires_in": 60,
                    "interval": 1,
                }),
            )
        }
        // The page where a person enters, and so approves, the user code.
        ("GET", "/device") => {
            let wanted = field(query, "user_code").unwrap_or_default().to_owned();
            match state
                .devices
                .iter_mut()
                .find(|device| device.user_code == wanted)
            {
                Some(device) => {
                    device.approved = true;
                    Answer::Json(200, json!({"approved": true}))
                }
                None => refused("invalid_request"),
            }
        }
        ("GET", route) if route.starts_with("/api/v4/") => {
            let issued = (1..=state.issued)
                .any(|n| authorization == Some(format!("Bearer {GRANT_ACCESS}{n}")));
            if !issued {
                Answer::Json(401, json!({"error": "fixture refusal"}))
            } else if route == "/api/v4/user" {
                Answer::Json(200, json!({"id": GRANT_SUBJECT.parse::<u64>().unwrap()}))
            } else {
                Answer::Json(200, json!({"id": 7, "name": "fixture-project"}))
            }
        }
        _ => Answer::Json(404, json!({"error": "fixture route"})),
    }
}

/// Consent to a well-formed authorization request, as a person who approved
/// it: a code bound to its PKCE challenge and redirect, sent back with the
/// request's `state` (or a forged one). `None` refuses it.
fn consent(state: &mut GrantState, query: &[(String, String)]) -> Option<String> {
    let scopes: Vec<&str> = field(query, "scope")?.split(' ').collect();
    let redirect_uri = field(query, "redirect_uri")?;
    let challenge = field(query, "code_challenge")?;
    let request_state = field(query, "state")?;
    let pasted = format!("https://localhost:{}/callback", state.port);
    let well_formed = field(query, "response_type") == Some("code")
        && field(query, "client_id") == Some(GRANT_CLIENT_ID)
        && field(query, "code_challenge_method") == Some("S256")
        && scopes.contains(&GRANT_SCOPE)
        && (redirect_uri.starts_with("http://127.0.0.1:") || redirect_uri == pasted)
        && challenge.len() == 43
        && !request_state.is_empty();
    if !well_formed {
        return None;
    }
    let code = format!("{GRANT_CODE}{}", state.consents.len() + 1);
    state.consents.push(Consent {
        code: code.clone(),
        challenge: challenge.to_owned(),
        redirect_uri: redirect_uri.to_owned(),
        exchanged: false,
    });
    let returned = match state.behaviour {
        Behaviour::ForgedState => "fixture-forged-state",
        _ => request_state,
    };
    Some(format!("{redirect_uri}?code={code}&state={returned}"))
}

/// Exchanges an unused code whose verifier matches its challenge, for the same
/// redirect and public client; anything else is `invalid_grant`.
fn exchange(state: &mut GrantState, form: &[(String, String)]) -> Answer {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let verifier = field(form, "code_verifier").unwrap_or_default();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let client = field(form, "client_id") == Some(GRANT_CLIENT_ID)
        && field(form, "client_secret").is_none()
        && (43..=128).contains(&verifier.len());
    let Some(consent) = state.consents.iter_mut().find(|consent| {
        client
            && !consent.exchanged
            && Some(consent.code.as_str()) == field(form, "code")
            && consent.challenge == challenge
            && Some(consent.redirect_uri.as_str()) == field(form, "redirect_uri")
    }) else {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    };
    consent.exchanged = true;
    let expires_in = granted_for(state.behaviour);
    grant(state, expires_in)
}

/// One RFC 8628 §3.4 poll.
fn poll(state: &mut GrantState, form: &[(String, String)]) -> Answer {
    let expired = state.behaviour == Behaviour::DeviceExpired;
    let wanted = field(form, "device_code").unwrap_or_default().to_owned();
    let client = field(form, "client_id") == Some(GRANT_CLIENT_ID);
    let Some(device) = state
        .devices
        .iter_mut()
        .find(|device| client && device.device_code == wanted && !device.exchanged)
    else {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    };
    if expired {
        return Answer::Json(400, json!({"error": "expired_token"}));
    }
    if !device.approved {
        return Answer::Json(400, json!({"error": "authorization_pending"}));
    }
    device.exchanged = true;
    let expires_in = granted_for(state.behaviour);
    grant(state, expires_in)
}

/// How long a new grant's access token is valid, in seconds.
fn granted_for(behaviour: Behaviour) -> u64 {
    match behaviour {
        Behaviour::ShortLived | Behaviour::RefreshRefused => 120,
        _ => 3600,
    }
}

/// A rotating refresh of an issued refresh token no refresh spent yet: a new
/// access and refresh token, valid for an hour. The refresh token it was sent
/// is spent; anything else is `invalid_grant`, as is every refresh under
/// `RefreshRefused`.
fn refresh(state: &mut GrantState, form: &[(String, String)]) -> Answer {
    state.refreshes += 1;
    let presented = field(form, "refresh_token").unwrap_or_default().to_owned();
    let issued = (1..=state.issued).any(|n| presented == format!("{GRANT_REFRESH}{n}"));
    if state.behaviour == Behaviour::RefreshRefused
        || field(form, "client_id") != Some(GRANT_CLIENT_ID)
        || !issued
        || state.spent.contains(&presented)
    {
        return Answer::Json(400, json!({"error": "invalid_grant"}));
    }
    state.spent.push(presented);
    grant(state, 3600)
}

/// A token response for a new grant, valid for `expires_in` seconds.
fn grant(state: &mut GrantState, expires_in: u64) -> Answer {
    state.issued += 1;
    let scope = match state.behaviour {
        Behaviour::NarrowScope => GRANT_EXTRA_SCOPE.to_owned(),
        _ => format!("{GRANT_SCOPE} {GRANT_EXTRA_SCOPE}"),
    };
    Answer::Json(
        200,
        json!({
            "access_token": format!("{GRANT_ACCESS}{}", state.issued),
            "token_type": "Bearer",
            "expires_in": expires_in,
            "scope": scope,
            "refresh_token": format!("{GRANT_REFRESH}{}", state.issued),
        }),
    )
}

/// An unsigned JWT with `claims` as its payload.
pub(super) fn unsigned_jwt(claims: &Value) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(claims),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}

/// The redirect a declaration names.
#[derive(Clone, Copy, Debug)]
pub(super) enum Redirect {
    /// `loopback_any_port`, no template.
    AnyPort,
    /// `loopback_fixed_port` at this port.
    Fixed(u16),
    /// `pasted_code`, redirecting to the fixture's own callback page.
    Pasted,
}

/// A public client's declaration of the fixture server at `port`, admitting
/// both grants.
pub(super) fn declaration(port: u16, redirect: Redirect) -> Value {
    let (kind, uri) = match redirect {
        Redirect::AnyPort => ("loopback_any_port", Value::Null),
        Redirect::Fixed(fixed) => (
            "loopback_fixed_port",
            json!(format!("http://127.0.0.1:{fixed}/callback")),
        ),
        Redirect::Pasted => (
            "pasted_code",
            json!(format!("https://localhost:{port}/callback")),
        ),
    };
    json!({
        "authorize_url": format!("https://localhost:{port}/authorize"),
        "token_url": format!("https://localhost:{port}/token"),
        "device_authorization_url": format!("https://localhost:{port}/device/code"),
        "client_id": GRANT_CLIENT_ID,
        "client_kind": "public",
        "request_encoding": "form",
        "refresh_encoding": "form",
        "redirect_kind": kind,
        "redirect_uri": uri,
        "authorize_parameters": [],
        "requested_scopes": [GRANT_SCOPE, GRANT_EXTRA_SCOPE],
        "required_scopes": [GRANT_SCOPE],
        "rotation": "rotating",
        "refresh_lead_seconds": 300,
        "grant_types": ["authorization_code", "device_code"],
        "sources": ["https://localhost/fixture-authorization-server"],
        "state_in_exchange": false,
        "scope_in_refresh": false,
    })
}

/// A catalog configuration whose one profile is the `oauth2_grant` profile
/// declaring `declaration`, reading identity from the API with the access token.
pub(super) fn grant_config(port: u16, ca: &Path, declaration: Value) -> Value {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    json!({
        "format": "connectors-catalog-local/2",
        "instance": "fixture-grant",
        "provider": "gitlab",
        "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
        "api_base": format!("https://localhost:{port}/api/v4"),
        "ca_file": ca,
        "auth": {
            "profile": GRANT_PROFILE,
            "scheme": "oauth2_grant",
            "header": "Authorization",
            "bearer": true,
            "label": "Fixture grant",
            "identity": {"path": "user", "kind": "fixture.user", "subject_pointer": "/id"},
            "minimum_scopes": [GRANT_SCOPE],
            "oauth2": declaration,
        },
        "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
    })
}

/// The printed bootstrap of the configuration at `path`, or the refusal's
/// stderr.
fn bootstrap_at(path: &Path) -> Result<Value, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(!carries_grant_material(&output.stdout));
    assert!(!carries_grant_material(&output.stderr));
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

/// A stored grant entry for access token number `issued`.
fn entry(issued: u32, scopes: &[&str], id_token: Option<String>) -> Secret {
    let mut entry = json!({
        "client_id": GRANT_CLIENT_ID,
        "access_token": format!("{GRANT_ACCESS}{issued}"),
        "refresh_token": format!("{GRANT_REFRESH}{issued}"),
        "expires_at_ms": connectors_sdk::now_ms() + 3_600_000,
        "scopes": scopes,
    });
    if let Some(id_token) = id_token {
        entry["id_token"] = json!(id_token);
    }
    Secret(serde_json::to_vec(&entry).unwrap())
}

#[test]
fn oauth2_grant_profile_reads_its_declaration() {
    let server = GrantServer::new(Behaviour::Consent);
    let declared = declaration(server.port, Redirect::Fixed(41_234));
    let printed = bootstrap(&grant_config(server.port, &server.ca, declared.clone())).unwrap();
    let profiles = printed["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 1);
    let profile = &profiles[0];
    assert_eq!(profile["id"], GRANT_PROFILE);
    assert_eq!(profile["scheme"], "http_bearer");
    assert_eq!(profile["capability"], "http-bearer");
    assert_eq!(profile["minimum_scopes"], json!([GRANT_SCOPE]));
    // The acquisition is the authorization server, carried as declared,
    // field for field, and nothing else.
    let acquisition = profile["acquisition"].as_object().expect("acquisition");
    assert_eq!(acquisition.len(), 1, "{profile}");
    let carried = acquisition["oauth2"]
        .as_object()
        .expect("oauth2 declaration");
    for (name, value) in declared.as_object().unwrap() {
        assert_eq!(carried.get(name), Some(value), "field {name}");
    }
    assert_eq!(carried.len(), declared.as_object().unwrap().len());
    // A changed declaration is another profile revision.
    let mut other = declared.clone();
    other["requested_scopes"] = json!([GRANT_SCOPE]);
    let changed = bootstrap(&grant_config(server.port, &server.ca, other)).unwrap();
    assert_ne!(changed["profiles"][0]["revision"], profile["revision"]);
    assert_ne!(
        changed["configuration_revision"],
        printed["configuration_revision"]
    );
}

#[test]
fn oauth2_grant_without_client_id_is_invalid_configuration() {
    let server = GrantServer::new(Behaviour::Consent);
    let mut declared = declaration(server.port, Redirect::AnyPort);
    declared["client_id"] = Value::Null;
    let config = grant_config(server.port, &server.ca, declared.clone());
    assert_eq!(bootstrap(&config).unwrap_err(), "InvalidConfiguration");
    // The instance's configuration supplies a registered client id instead.
    let mut configured = config.clone();
    configured["oauth2_client_id"] = json!("fixture-registered-client");
    let printed = bootstrap(&configured).unwrap();
    assert_eq!(
        printed["profiles"][0]["acquisition"]["oauth2"]["client_id"],
        "fixture-registered-client"
    );
    // Both given, they must agree.
    let mut both = grant_config(
        server.port,
        &server.ca,
        declaration(server.port, Redirect::AnyPort),
    );
    both["oauth2_client_id"] = json!("fixture-registered-client");
    assert_eq!(bootstrap(&both).unwrap_err(), "InvalidConfiguration");
    let mut agreeing = both.clone();
    agreeing["oauth2_client_id"] = json!(GRANT_CLIENT_ID);
    bootstrap(&agreeing).unwrap();
    // An empty client id is none.
    let mut empty = config;
    empty["oauth2_client_id"] = json!("");
    assert_eq!(bootstrap(&empty).unwrap_err(), "InvalidConfiguration");
}

#[test]
fn oauth2_grant_http_endpoint_is_refused() {
    let server = GrantServer::new(Behaviour::Consent);
    let port = server.port;
    // The same declaration over HTTPS is read.
    bootstrap(&grant_config(
        port,
        &server.ca,
        declaration(port, Redirect::AnyPort),
    ))
    .unwrap();
    for (name, plain) in [
        ("token_url", format!("http://localhost:{port}/token")),
        (
            "authorize_url",
            format!("http://localhost:{port}/authorize"),
        ),
        (
            "device_authorization_url",
            format!("http://localhost:{port}/device/code"),
        ),
    ] {
        let mut declared = declaration(port, Redirect::AnyPort);
        declared[name] = json!(plain);
        assert_eq!(
            bootstrap(&grant_config(port, &server.ca, declared)).unwrap_err(),
            "InvalidConfiguration",
            "{name}"
        );
    }
}

#[test]
fn oauth2_grant_incomplete_declaration_is_refused() {
    let server = GrantServer::new(Behaviour::Consent);
    let port = server.port;
    // No grant at all.
    let mut declared = declaration(port, Redirect::AnyPort);
    declared["grant_types"] = json!([]);
    assert_eq!(
        bootstrap(&grant_config(port, &server.ca, declared)).unwrap_err(),
        "InvalidConfiguration"
    );
    // A device grant without its endpoint.
    let mut declared = declaration(port, Redirect::AnyPort);
    declared["device_authorization_url"] = Value::Null;
    assert_eq!(
        bootstrap(&grant_config(port, &server.ca, declared.clone())).unwrap_err(),
        "InvalidConfiguration"
    );
    // Without the device grant it needs none.
    declared["grant_types"] = json!(["authorization_code"]);
    bootstrap(&grant_config(port, &server.ca, declared)).unwrap();
    // The declaration belongs to `oauth2_grant` only, and that scheme needs it.
    let mut config = grant_config(port, &server.ca, declaration(port, Redirect::AnyPort));
    config["auth"]["scheme"] = json!("oauth2_refresh");
    assert_eq!(bootstrap(&config).unwrap_err(), "InvalidConfiguration");
    let mut config = grant_config(port, &server.ca, declaration(port, Redirect::AnyPort));
    config["auth"].as_object_mut().unwrap().remove("oauth2");
    assert_eq!(bootstrap(&config).unwrap_err(), "InvalidConfiguration");
}

/// The child validates a stored entry by reading the identity with its access
/// token; the entry's scopes and expiry are the evidence's. No token endpoint
/// is called: validation never refreshes.
#[test]
fn oauth2_grant_entry_validates_with_its_access_token() {
    let server = GrantServer::new(Behaviour::Consent);
    // The fixture accepts the access token it issued.
    server.state.lock().unwrap().issued = 1;
    let mut child = Child::spawn(&server.selection()).unwrap();
    let stored = entry(1, &[GRANT_SCOPE, GRANT_EXTRA_SCOPE], None);
    let expires_at_ms = serde_json::from_slice::<Value>(&stored.0).unwrap()["expires_at_ms"]
        .as_u64()
        .unwrap();
    let baseline = child
        .validate(GRANT_PROFILE, &stored, deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.kind, "fixture.user");
    assert_eq!(baseline.identity.subject, GRANT_SUBJECT);
    assert_eq!(
        baseline.granted_scopes.unwrap(),
        [GRANT_SCOPE.to_owned(), GRANT_EXTRA_SCOPE.to_owned()].into()
    );
    assert_eq!(baseline.credential_expires_at_ms, Some(expires_at_ms));
    assert!(baseline.valid_until_ms <= expires_at_ms);
    assert_eq!(
        server.api_authorizations(),
        vec![Some(format!("Bearer {GRANT_ACCESS}1"))]
    );
    assert!(server.routes().iter().all(|route| route != "POST /token"));
    // Scopes below the minimum are insufficient, before any read.
    let before = server.requests().len();
    assert!(matches!(
        child.validate(
            GRANT_PROFILE,
            &entry(1, &[GRANT_EXTRA_SCOPE], None),
            deadline()
        ),
        Err(Failure::InsufficientScope)
    ));
    assert_eq!(server.requests().len(), before);
    // An access token the provider refuses is not a valid entry.
    assert!(
        child
            .validate(GRANT_PROFILE, &entry(9, &[GRANT_SCOPE], None), deadline())
            .is_err()
    );
    // An entry that already expired is refused.
    let mut expired: Value = serde_json::from_slice(&entry(1, &[GRANT_SCOPE], None).0).unwrap();
    expired["expires_at_ms"] = json!(connectors_sdk::now_ms() - 1);
    assert!(
        child
            .validate(
                GRANT_PROFILE,
                &Secret(serde_json::to_vec(&expired).unwrap()),
                deadline()
            )
            .is_err()
    );
}

/// With `identity.source = id_token` the subject is the `sub` of the
/// `id_token` the token endpoint returned (OpenID Connect Core §3.1.3.7 item 6:
/// received directly over TLS), issued to this client and not expired.
#[test]
fn oauth2_grant_entry_identity_from_id_token() {
    let server = GrantServer::new(Behaviour::Consent);
    let mut config = grant_config(
        server.port,
        &server.ca,
        declaration(server.port, Redirect::AnyPort),
    );
    config["auth"]["identity"] = json!({"source": "id_token", "kind": "fixture.person"});
    server.write_config(&config);
    let mut child = Child::spawn(&server.selection()).unwrap();
    let claims = |aud: &str, exp: u64| json!({"iss": format!("https://localhost:{}", server.port), "aud": aud, "sub": "person-7", "iat": 1, "exp": exp});
    let baseline = child
        .validate(
            GRANT_PROFILE,
            &entry(
                1,
                &[GRANT_SCOPE],
                Some(unsigned_jwt(&claims(GRANT_CLIENT_ID, 4_000_000_000))),
            ),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.kind, "fixture.person");
    assert_eq!(baseline.identity.subject, "person-7");
    assert!(server.requests().is_empty(), "the id_token needs no read");
    // Issued to another client, expired, or absent: no identity.
    for id_token in [
        Some(unsigned_jwt(&claims("another-client", 4_000_000_000))),
        Some(unsigned_jwt(&claims(GRANT_CLIENT_ID, 1))),
        None,
    ] {
        assert!(
            child
                .validate(
                    GRANT_PROFILE,
                    &entry(1, &[GRANT_SCOPE], id_token),
                    deadline()
                )
                .is_err()
        );
    }
}
