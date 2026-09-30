//! The `oauth2_refresh` profile through the provider child: a disposable HTTPS
//! fixture serves the API, a token route and a tokeninfo route on one host. The
//! child exchanges the stored `{client_id, client_secret, refresh_token}` entry
//! for an access token, sends it as `Authorization: Bearer`, caches it until
//! 60 s before it expires, and refuses by code. The fixture's secrets are
//! fictional and only ever compared, never printed.
use super::*;

const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-one";
const REFRESH_TOKEN: &str = "fixture-refresh-token-one";
const ROTATED_REFRESH_TOKEN: &str = "fixture-refresh-token-rotated";
const ACCESS_TOKEN: &str = "fixture-access-token-";
const OAUTH_PROFILE: &str = "fixture.oauth";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
const SUBJECT: &str = "110000000000000000001";
const TOKENINFO_SUBJECT: &str = "110000000000000000002";

/// Every fixture secret, and the access-token prefix, which no output may carry.
fn carries_oauth_material(haystack: &[u8]) -> bool {
    [
        CLIENT_SECRET,
        REFRESH_TOKEN,
        ROTATED_REFRESH_TOKEN,
        ACCESS_TOKEN,
    ]
    .iter()
    .any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

fn entry() -> Secret {
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
            "refresh_token": REFRESH_TOKEN,
        }))
        .unwrap(),
    )
}

/// An unsigned JWT with `claims` as its payload. The child reads the payload of
/// an `id_token` it received directly from the token endpoint over TLS.
fn id_token(claims: &Value) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(claims),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}

fn google_claims(aud: &str, iss: &str) -> Value {
    json!({"iss": iss, "aud": aud, "azp": aud, "sub": SUBJECT, "iat": 1, "exp": 4_000_000_000_u64})
}

/// What the token route answers next.
#[derive(Clone)]
enum TokenAnswer {
    Grant {
        expires_in: u64,
        scope: String,
        claims: Option<Value>,
        refresh_token: Option<&'static str>,
    },
    Refuse(u16, Value),
}
fn grant(expires_in: u64) -> TokenAnswer {
    TokenAnswer::Grant {
        expires_in,
        scope: format!("openid {DRIVE_SCOPE}"),
        claims: None,
        refresh_token: None,
    }
}

/// One observed request. Compared by the tests, never printed.
#[derive(Clone)]
struct Observed {
    method: String,
    route: String,
    query: String,
    content_type: Option<String>,
    authorization: Option<String>,
    body: Vec<u8>,
}

struct FixtureState {
    requests: Vec<Observed>,
    token: TokenAnswer,
    tokeninfo: (u16, Value),
    issued: u32,
    api_refuses: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Source {
    Api,
    IdToken,
}

struct OAuthProvider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    config: PathBuf,
    port: u16,
    state: Arc<Mutex<FixtureState>>,
}

impl OAuthProvider {
    fn new(source: Source) -> Self {
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
        let state = Arc::new(Mutex::new(FixtureState {
            requests: Vec::new(),
            token: grant(3600),
            tokeninfo: (
                200,
                json!({"aud": CLIENT_ID, "azp": CLIENT_ID, "sub": TOKENINFO_SUBJECT,
                       "scope": format!("openid {DRIVE_SCOPE}"), "expires_in": "3599"}),
            ),
            issued: 0,
            api_refuses: false,
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
                    let authorization = header("authorization");
                    let (status, answer) = {
                        let mut state = served.lock().unwrap();
                        state.requests.push(Observed {
                            method: method.clone(),
                            route: route.clone(),
                            query: query.clone(),
                            content_type: header("content-type"),
                            authorization: authorization.clone(),
                            body,
                        });
                        if route == "/token" && method == "POST" {
                            match state.token.clone() {
                                TokenAnswer::Refuse(status, body) => (status, body),
                                TokenAnswer::Grant {
                                    expires_in,
                                    scope,
                                    claims,
                                    refresh_token,
                                } => {
                                    state.issued += 1;
                                    let mut body = json!({
                                        "access_token": format!("{ACCESS_TOKEN}{}", state.issued),
                                        "token_type": "Bearer",
                                        "expires_in": expires_in,
                                        "scope": scope,
                                    });
                                    if let Some(claims) = claims {
                                        body["id_token"] = json!(id_token(&claims));
                                    }
                                    if let Some(refresh_token) = refresh_token {
                                        body["refresh_token"] = json!(refresh_token);
                                    }
                                    (200, body)
                                }
                            }
                        } else if route == "/tokeninfo" && method == "GET" {
                            state.tokeninfo.clone()
                        } else if route.starts_with("/api/v4/") {
                            let issued = (1..=state.issued)
                                .any(|n| authorization == Some(format!("Bearer {ACCESS_TOKEN}{n}")));
                            if state.api_refuses || !issued {
                                (401, json!({"error": "fixture refusal"}))
                            } else if route == "/api/v4/user" {
                                (200, json!({"id": 42}))
                            } else {
                                (200, json!({"id": 7, "name": "fixture-project"}))
                            }
                        } else {
                            (404, json!({"error": "fixture route"}))
                        }
                    };
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let port = address_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .port();
        let config = directory.join("catalog.json");
        let provider = Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            port,
            state,
        };
        provider.write_config(&oauth_config(
            port,
            &provider.ca,
            source,
            &format!("https://localhost:{port}/token"),
            "https://localhost/authorize",
        ));
        provider
    }
    fn write_config(&self, config: &Value) {
        private(&self.config, &serde_json::to_vec(config).unwrap());
    }
    fn selection(&self) -> Adapter {
        let bootstrap = print_bootstrap(&self.config).expect("bootstrap inspection failed");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-oauth".into(),
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
    fn set_token(&self, answer: TokenAnswer) {
        self.state.lock().unwrap().token = answer;
    }
    fn requests(&self) -> Vec<Observed> {
        self.state.lock().unwrap().requests.clone()
    }
    fn token_requests(&self) -> usize {
        self.requests()
            .iter()
            .filter(|request| request.route == "/token")
            .count()
    }
    /// The `Authorization` header of every API request, in order.
    fn api_authorizations(&self) -> Vec<Option<String>> {
        self.requests()
            .into_iter()
            .filter(|request| request.route.starts_with("/api/v4/"))
            .map(|request| request.authorization)
            .collect()
    }
}
impl Drop for OAuthProvider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn oauth_config(
    port: u16,
    ca: &Path,
    source: Source,
    token_url: &str,
    authorize_url: &str,
) -> Value {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let identity = match source {
        Source::Api => json!({"path": "user", "kind": "fixture.user", "subject_pointer": "/id"}),
        Source::IdToken => json!({"source": "id_token", "kind": "google.user"}),
    };
    let minimum = match source {
        Source::Api => json!([]),
        Source::IdToken => json!([DRIVE_SCOPE]),
    };
    json!({
        "format": "connectors-catalog-local/2",
        "instance": "fixture-oauth",
        "provider": "gitlab",
        "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
        "api_base": format!("https://localhost:{port}/api/v4"),
        "ca_file": ca,
        "auth": {
            "profile": OAUTH_PROFILE,
            "scheme": "oauth2_refresh",
            "header": "Authorization",
            "bearer": true,
            "label": "Fixture refresh token",
            "identity": identity,
            "minimum_scopes": minimum,
            "token_url": token_url,
            "token_ca_file": ca,
            "authorize_url": authorize_url,
            "requested_scopes": ["openid", DRIVE_SCOPE],
        },
        "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
    })
}

/// The printed bootstrap as JSON, or the refusal's stderr.
fn print_bootstrap(config: &Path) -> Result<Value, Vec<u8>> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(!carries_oauth_material(&output.stdout));
    assert!(!carries_oauth_material(&output.stderr));
    if !output.status.success() {
        return Err(output.stderr);
    }
    assert!(output.stderr.is_empty());
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    serde_json::from_value::<Bootstrap>(bootstrap.clone())
        .unwrap()
        .validate()
        .unwrap();
    Ok(bootstrap)
}

fn spawn(provider: &OAuthProvider) -> Child {
    Child::spawn(&provider.selection()).unwrap_or_else(|failure| panic!("spawn {failure:?}"))
}

fn project(child: &mut Child, document: &Secret) -> Result<Value, Failure> {
    invoke(
        child,
        "project.get",
        "one",
        document,
        json!({"id": "org/project"}),
    )
}

/// The configuration file and every provider child carry no OAuth material.
fn assert_no_material_at_rest(provider: &OAuthProvider) {
    assert!(!carries_oauth_material(
        &fs::read(&provider.config).unwrap()
    ));
    let children = provider_children(&provider.config);
    assert!(!children.is_empty(), "no provider child observed");
    for pid in children {
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap();
        let environ = fs::read(format!("/proc/{pid}/environ")).unwrap();
        assert!(!carries_oauth_material(&cmdline), "argv of {pid}");
        assert!(!carries_oauth_material(&environ), "environment of {pid}");
    }
}

#[test]
fn oauth_exchange_sends_form_and_uses_bearer() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    let project = project(&mut child, &entry()).unwrap_or_else(|f| panic!("invoke {f:?}"));
    assert_eq!(project["body"]["id"], 7);
    let requests = provider.requests();
    let token: Vec<&Observed> = requests.iter().filter(|r| r.route == "/token").collect();
    assert_eq!(token.len(), 1);
    assert_eq!(token[0].method, "POST");
    assert_eq!(
        token[0].content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert!(
        token[0].authorization.is_none(),
        "credential header on the token request"
    );
    assert!(token[0].query.is_empty());
    let mut form: Vec<(String, String)> = token[0]
        .body
        .split(|b| *b == b'&')
        .map(|pair| {
            let pair = std::str::from_utf8(pair).unwrap();
            let (name, value) = pair.split_once('=').unwrap();
            (name.to_owned(), value.to_owned())
        })
        .collect();
    form.sort();
    let mut expected = vec![
        ("grant_type".to_owned(), "refresh_token".to_owned()),
        ("client_id".to_owned(), CLIENT_ID.to_owned()),
        ("client_secret".to_owned(), CLIENT_SECRET.to_owned()),
        ("refresh_token".to_owned(), REFRESH_TOKEN.to_owned()),
    ];
    expected.sort();
    assert!(form == expected, "token form fields differ");
    let api = provider.api_authorizations();
    assert_eq!(api.len(), 1);
    assert!(api[0].as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}1")));
    // The API request follows the token request.
    assert_eq!(
        requests.last().unwrap().route,
        "/api/v4/projects/org%2Fproject"
    );
    assert_no_material_at_rest(&provider);
}

#[test]
fn oauth_cache_reuses_token_until_skew() {
    let provider = OAuthProvider::new(Source::Api);
    // Valid for 63 s, so cached for 3 s.
    provider.set_token(grant(63));
    let mut child = spawn(&provider);
    let started = std::time::Instant::now();
    project(&mut child, &entry()).unwrap();
    project(&mut child, &entry()).unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "fixture too slow"
    );
    assert_eq!(provider.token_requests(), 1);
    let api = provider.api_authorizations();
    assert!(
        api.iter()
            .all(|a| a.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}1")))
    );
    std::thread::sleep(Duration::from_millis(4500).saturating_sub(started.elapsed()));
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 2);
    let api = provider.api_authorizations();
    assert!(api.last().unwrap().as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}2")));
    // A token that expires within the skew is used once and never cached.
    provider.set_token(grant(60));
    let mut fresh = spawn(&provider);
    project(&mut fresh, &entry()).unwrap();
    project(&mut fresh, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 4);
}

#[test]
fn oauth_rate_limit_and_unavailable() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    provider.set_token(TokenAnswer::Refuse(
        429,
        json!({"error": "rate_limit_exceeded"}),
    ));
    assert!(matches!(
        project(&mut child, &entry()),
        Err(Failure::ProviderRateLimited)
    ));
    provider.set_token(TokenAnswer::Refuse(503, json!({"error": "backend_error"})));
    assert!(matches!(
        project(&mut child, &entry()),
        Err(Failure::Unavailable)
    ));
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::Unavailable)
    ));
    // Neither refusal reached the API, and neither left a token behind.
    assert!(provider.api_authorizations().is_empty());
    provider.set_token(grant(3600));
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 4);
}

#[test]
fn oauth_invalid_grant_is_an_invalid_credential() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    for error in ["invalid_grant", "invalid_client"] {
        provider.set_token(TokenAnswer::Refuse(
            400,
            json!({"error": error, "error_description": "fixture description"}),
        ));
        assert!(matches!(
            project(&mut child, &entry()),
            Err(Failure::InvalidCredential)
        ));
        assert!(matches!(
            child.validate(OAUTH_PROFILE, &entry(), deadline()),
            Err(Failure::InvalidCredential)
        ));
    }
    assert!(provider.api_authorizations().is_empty());
}

#[test]
fn oauth_rotated_refresh_token_refused() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    provider.set_token(TokenAnswer::Grant {
        expires_in: 3600,
        scope: DRIVE_SCOPE.into(),
        claims: None,
        refresh_token: Some(ROTATED_REFRESH_TOKEN),
    });
    assert!(matches!(
        project(&mut child, &entry()),
        Err(Failure::InvalidCredential)
    ));
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert!(provider.api_authorizations().is_empty());
    // Nothing was stored: the next invoke exchanges again. Returning the same
    // refresh token is not a rotation.
    provider.set_token(TokenAnswer::Grant {
        expires_in: 3600,
        scope: DRIVE_SCOPE.into(),
        claims: None,
        refresh_token: Some(REFRESH_TOKEN),
    });
    let before = provider.token_requests();
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), before + 1);
    let api = provider.api_authorizations();
    assert!(api.last().unwrap().as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}3")));
}

#[test]
fn oauth_validate_identity_from_id_token() {
    let provider = OAuthProvider::new(Source::IdToken);
    let mut child = spawn(&provider);
    for iss in ["https://accounts.google.com", "accounts.google.com"] {
        provider.set_token(TokenAnswer::Grant {
            expires_in: 3600,
            scope: format!("openid {DRIVE_SCOPE}"),
            claims: Some(google_claims(CLIENT_ID, iss)),
            refresh_token: None,
        });
        let baseline = child
            .validate(OAUTH_PROFILE, &entry(), deadline())
            .unwrap_or_else(|failure| panic!("validation {failure:?}"));
        assert_eq!(baseline.identity.kind, "google.user");
        assert_eq!(baseline.identity.subject, SUBJECT);
        assert_eq!(
            baseline.granted_scopes.unwrap(),
            ["openid".to_owned(), DRIVE_SCOPE.to_owned()].into()
        );
        assert!(baseline.credential_expires_at_ms.is_none());
    }
    // `validate` always exchanges fresh, and never asks tokeninfo or the API.
    assert_eq!(provider.token_requests(), 2);
    assert!(provider.requests().iter().all(|r| r.route == "/token"));
    // An id_token issued to another client is refused.
    provider.set_token(TokenAnswer::Grant {
        expires_in: 3600,
        scope: format!("openid {DRIVE_SCOPE}"),
        claims: Some(google_claims(
            "another-client.apps.example.test",
            "https://accounts.google.com",
        )),
        refresh_token: None,
    });
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::Protocol)
    ));
    // Granted scopes below `minimum_scopes` are insufficient.
    provider.set_token(TokenAnswer::Grant {
        expires_in: 3600,
        scope: "openid".into(),
        claims: Some(google_claims(CLIENT_ID, "https://accounts.google.com")),
        refresh_token: None,
    });
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::InsufficientScope)
    ));
    assert_no_material_at_rest(&provider);
}

#[test]
fn oauth_validate_identity_from_tokeninfo() {
    let provider = OAuthProvider::new(Source::IdToken);
    let mut child = spawn(&provider);
    let baseline = child
        .validate(OAUTH_PROFILE, &entry(), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.subject, TOKENINFO_SUBJECT);
    assert_eq!(
        baseline.granted_scopes.unwrap(),
        ["openid".to_owned(), DRIVE_SCOPE.to_owned()].into()
    );
    let requests = provider.requests();
    let info: Vec<&Observed> = requests
        .iter()
        .filter(|r| r.route == "/tokeninfo")
        .collect();
    assert_eq!(info.len(), 1);
    assert_eq!(info[0].method, "GET");
    assert!(info[0].authorization.is_none());
    assert!(info[0].query == format!("access_token={ACCESS_TOKEN}1"));
    // Tokeninfo naming another client is refused.
    provider.state.lock().unwrap().tokeninfo = (
        200,
        json!({"aud": "another-client.apps.example.test", "sub": TOKENINFO_SUBJECT,
               "scope": format!("openid {DRIVE_SCOPE}")}),
    );
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::Protocol)
    ));
    // Tokeninfo refusing the fresh access token refuses the credential.
    provider.state.lock().unwrap().tokeninfo = (400, json!({"error": "invalid_token"}));
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::InvalidCredential)
    ));
}

#[test]
fn oauth_validate_wrong_iss_refused() {
    let provider = OAuthProvider::new(Source::IdToken);
    let mut child = spawn(&provider);
    for iss in [
        "https://issuer.example.test",
        "http://accounts.google.com",
        "https://accounts.google.com/",
    ] {
        provider.set_token(TokenAnswer::Grant {
            expires_in: 3600,
            scope: format!("openid {DRIVE_SCOPE}"),
            claims: Some(google_claims(CLIENT_ID, iss)),
            refresh_token: None,
        });
        assert!(
            matches!(
                child.validate(OAUTH_PROFILE, &entry(), deadline()),
                Err(Failure::Protocol)
            ),
            "{iss}"
        );
    }
    assert!(provider.requests().iter().all(|r| r.route == "/token"));
}

#[test]
fn oauth_token_url_must_be_https() {
    let provider = OAuthProvider::new(Source::Api);
    assert!(print_bootstrap(&provider.config).is_ok());
    let port = provider.port;
    for token_url in [
        format!("http://localhost:{port}/token"),
        format!("https://localhost:{port}/token?key=value"),
        format!("https://localhost:{port}/"),
        "token".to_owned(),
    ] {
        provider.write_config(&oauth_config(
            port,
            &provider.ca,
            Source::Api,
            &token_url,
            "https://localhost/authorize",
        ));
        assert!(print_bootstrap(&provider.config).is_err(), "{token_url}");
    }
    assert!(provider.requests().is_empty());
}

#[test]
fn oauth_authorize_url_must_be_https() {
    let provider = OAuthProvider::new(Source::Api);
    assert!(print_bootstrap(&provider.config).is_ok());
    let port = provider.port;
    for authorize_url in [
        "http://localhost/authorize",
        "https://localhost/authorize#fragment",
        "https://user:password@localhost/authorize",
    ] {
        provider.write_config(&oauth_config(
            port,
            &provider.ca,
            Source::Api,
            &format!("https://localhost:{port}/token"),
            authorize_url,
        ));
        assert!(
            print_bootstrap(&provider.config).is_err(),
            "{authorize_url}"
        );
    }
}

#[test]
fn oauth_configuration_refuses_inconsistent_profiles() {
    let provider = OAuthProvider::new(Source::Api);
    let port = provider.port;
    let base = oauth_config(
        port,
        &provider.ca,
        Source::Api,
        &format!("https://localhost:{port}/token"),
        "https://localhost/authorize",
    );
    let refused = |edit: &dyn Fn(&mut Value), what: &str| {
        let mut config = base.clone();
        edit(&mut config);
        provider.write_config(&config);
        assert!(print_bootstrap(&provider.config).is_err(), "{what}");
    };
    refused(&|c| c["auth"]["bearer"] = json!(false), "bearer false");
    refused(
        &|c| c["auth"]["header"] = json!("PRIVATE-TOKEN"),
        "another header",
    );
    refused(
        &|c| c["auth"]["requested_scopes"] = json!([]),
        "no requested scopes",
    );
    refused(
        &|c| {
            c["auth"]
                .as_object_mut()
                .unwrap()
                .remove("token_url")
                .map(drop)
                .unwrap()
        },
        "no token_url",
    );
    refused(
        &|c| {
            c["auth"]
                .as_object_mut()
                .unwrap()
                .remove("authorize_url")
                .map(drop)
                .unwrap()
        },
        "no authorize_url",
    );
    refused(
        &|c| {
            c["auth"]["identity"] =
                json!({"source": "id_token", "kind": "google.user", "path": "user"})
        },
        "an id_token identity naming an API probe",
    );
    refused(
        &|c| c["auth"]["identity"] = json!({"kind": "fixture.user"}),
        "an API identity without a probe",
    );
    // OAuth fields belong to the oauth2_refresh scheme only.
    let mut token = base.clone();
    token["auth"]["scheme"] = json!("token");
    token["auth"]["header"] = json!("PRIVATE-TOKEN");
    token["auth"]["bearer"] = json!(false);
    provider.write_config(&token);
    assert!(
        print_bootstrap(&provider.config).is_err(),
        "token scheme with OAuth fields"
    );
    let auth = token["auth"].as_object_mut().unwrap();
    for field in [
        "token_url",
        "token_ca_file",
        "authorize_url",
        "requested_scopes",
    ] {
        auth.remove(field);
    }
    provider.write_config(&token);
    assert!(
        print_bootstrap(&provider.config).is_ok(),
        "the plain token profile"
    );
    token["auth"]["identity"] = json!({"source": "id_token", "kind": "google.user"});
    provider.write_config(&token);
    assert!(
        print_bootstrap(&provider.config).is_err(),
        "id_token outside oauth2_refresh"
    );
}

#[test]
fn oauth_cache_evicted_on_invalid_credential() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    project(&mut child, &entry()).unwrap();
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 1);
    provider.state.lock().unwrap().api_refuses = true;
    assert!(matches!(
        project(&mut child, &entry()),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(provider.token_requests(), 1);
    provider.state.lock().unwrap().api_refuses = false;
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 2);
    let api = provider.api_authorizations();
    assert!(api.last().unwrap().as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}2")));
    // A validation the token endpoint refuses evicts the cached token too, and
    // a validation that succeeds leaves its own fresh token cached.
    provider.set_token(TokenAnswer::Refuse(400, json!({"error": "invalid_grant"})));
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::InvalidCredential)
    ));
    provider.set_token(grant(3600));
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 4);
    child
        .validate(OAUTH_PROFILE, &entry(), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(provider.token_requests(), 5);
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 5);
    let api = provider.api_authorizations();
    assert!(api.last().unwrap().as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}4")));
    // A validation whose identity read the API refuses caches nothing.
    provider.state.lock().unwrap().api_refuses = true;
    assert!(matches!(
        child.validate(OAUTH_PROFILE, &entry(), deadline()),
        Err(Failure::InvalidCredential)
    ));
    provider.state.lock().unwrap().api_refuses = false;
    project(&mut child, &entry()).unwrap();
    assert_eq!(provider.token_requests(), 7);
}

#[test]
fn oauth_profile_carries_acquisition() {
    let provider = OAuthProvider::new(Source::Api);
    let bootstrap = print_bootstrap(&provider.config).unwrap();
    let profile = &bootstrap["profiles"][0];
    assert_eq!(profile["id"], OAUTH_PROFILE);
    assert_eq!(profile["scheme"], "http_bearer");
    assert_eq!(profile["capability"], "http-bearer");
    let fields: Vec<&str> = profile["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["client_id", "client_secret", "refresh_token"]);
    assert_eq!(
        profile["acquisition"],
        json!({
            "authorize_url": "https://localhost/authorize",
            "token_url": format!("https://localhost:{}/token", provider.port),
            "scopes": [DRIVE_SCOPE, "openid"],
        })
    );
    // The child's own bootstrap carries the same profile.
    let child = spawn(&provider);
    let spawned = serde_json::to_value(child.bootstrap().profile(OAUTH_PROFILE).unwrap()).unwrap();
    assert_eq!(&spawned, profile);
    // A token profile serializes with no acquisition key at all.
    let token = Provider::new();
    let bootstrap = print_bootstrap(&token.config).unwrap();
    let profile = bootstrap["profiles"][0].as_object().unwrap();
    assert!(!profile.contains_key("acquisition"));
    let basic = Provider::basic("api");
    let bootstrap = print_bootstrap(&basic.config).unwrap();
    assert!(
        !bootstrap["profiles"][0]
            .as_object()
            .unwrap()
            .contains_key("acquisition")
    );
}

#[test]
fn oauth_entry_unknown_field_refused() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = spawn(&provider);
    for document in [
        json!({"client_id": CLIENT_ID, "client_secret": CLIENT_SECRET,
               "refresh_token": REFRESH_TOKEN, "access_token": "fixture-extra"}),
        json!({"client_id": CLIENT_ID, "refresh_token": REFRESH_TOKEN}),
        json!({"client_id": CLIENT_ID, "client_secret": "", "refresh_token": REFRESH_TOKEN}),
        json!({"client_id": CLIENT_ID, "client_secret": CLIENT_SECRET, "refresh_token": "has space"}),
        json!({"token": "fixture-pat-one"}),
    ] {
        let document = Secret(serde_json::to_vec(&document).unwrap());
        assert!(matches!(
            project(&mut child, &document),
            Err(Failure::InvalidInput)
        ));
        assert!(matches!(
            child.validate(OAUTH_PROFILE, &document, deadline()),
            Err(Failure::InvalidInput)
        ));
    }
    assert!(provider.requests().is_empty());
}

/// The configuration revision of the token and basic fixtures is the digest the
/// provider computed before the OAuth profile existed: the same effective
/// document, with `auth` serialized exactly as it was.
#[test]
fn oauth_existing_configuration_revisions_unchanged() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundles = repository.join("generated/bundles");
    let index = connectors_catalog::bundle::read_index(&bundles).unwrap();
    let bundle = connectors_catalog::bundle::load(&bundles, "gitlab").unwrap();
    let operations: Value = serde_json::from_slice(
        &fs::read(repository.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let operations: Vec<connectors_catalog_provider::Selection> =
        serde_json::from_value(operations["operations"].clone()).unwrap();
    for (provider, auth) in [
        (
            Provider::new(),
            json!({
                "profile": "gitlab.pat",
                "header": "PRIVATE-TOKEN",
                "bearer": false,
                "label": "GitLab personal access token",
                "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
                "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                "minimum_scopes": ["api"],
                "evidence_lifetime_ms": 60_000,
            }),
        ),
        (
            Provider::basic("api"),
            json!({
                "profile": BASIC_PROFILE,
                "scheme": "basic",
                "header": "Authorization",
                "bearer": false,
                "account_label": "Fixture account email",
                "label": "Fixture API token",
                "identity": {"path": "user", "kind": "fixture.user", "subject_pointer": "/id"},
                "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                "minimum_scopes": ["api"],
                "evidence_lifetime_ms": 60_000,
            }),
        ),
    ] {
        let config: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
        let ca = fs::read(&provider.ca).unwrap();
        let effective = json!({
            "format": "connectors-catalog-local/2",
            "instance": "fixture-gitlab",
            "provider": "gitlab",
            "bundle_sha256": index.find("gitlab").map(|e| e.bundle_sha256.clone()),
            "source_sha256": bundle.source.source_sha256,
            "api_base": format!("{}/", config["api_base"].as_str().unwrap()),
            "ca_digest": connectors_core::digest(&json!(ca)),
            "auth": auth,
            "operations": serde_json::to_value(&operations).unwrap(),
        });
        let bootstrap = print_bootstrap(&provider.config).unwrap();
        assert_eq!(
            bootstrap["configuration_revision"],
            json!(connectors_core::digest(&effective)),
            "{}",
            auth["profile"]
        );
    }
}

/// The production CLI bound to an `oauth2_refresh` provider, with disposable
/// custody. The adapter alias is `drive` and it grants `project.get`.
struct OAuthCli {
    provider: OAuthProvider,
    _custody: super::cli_journey::Custody,
    cli: super::cli_journey::Cli,
    credential: PathBuf,
}
impl OAuthCli {
    fn new() -> Self {
        use super::cli_journey::{Cli, Custody, success};
        let provider = OAuthProvider::new(Source::Api);
        let custody = Custody::new(provider.root.path());
        let cli = Cli::new(provider.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = provider.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.drive]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.drive.permissions]\nprofiles=[{}]\noperations=['project.get']\n[adapters.drive.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(OAUTH_PROFILE),
            q(adapter.executable.path.to_str().unwrap()),
            q(&adapter.executable.sha256),
            serde_json::to_string(&adapter.executable.args).unwrap()
        );
        private(&cli.paths.config, configuration.as_bytes());
        let credential = provider.root.path().join("private/credential.json");
        private(&credential, &entry().0);
        Self {
            provider,
            _custody: custody,
            cli,
            credential,
        }
    }
    fn connect(&self) -> std::process::Output {
        self.cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "drive",
            "--profile",
            OAUTH_PROFILE,
            "--credential-file",
            self.credential.to_str().unwrap(),
        ])
    }
    /// Connects, and returns the `project.get` invoke arguments for it.
    fn connected(&self) -> Vec<String> {
        use super::cli_journey::success;
        let connected = success(self.connect())["connection"].clone();
        assert_eq!(connected["summary"]["state"], "ready");
        let reference = connected["summary"]["connection"].as_str().unwrap();
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "drive",
            "--operation",
            "project.get",
        ]));
        [
            "operations",
            "invoke",
            "--adapter",
            "drive",
            "--connection",
            reference,
            "--operation",
            "project.get",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            r#"{"id":"org/project"}"#,
        ]
        .map(str::to_owned)
        .to_vec()
    }
    fn run(&self, args: &[String]) -> std::process::Output {
        self.cli
            .run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    }
    /// The refusal's data, after checking no OAuth material reached either stream.
    fn refused(output: std::process::Output, what: &str) -> Value {
        assert!(!carries_oauth_material(&output.stdout), "{what}");
        assert!(!carries_oauth_material(&output.stderr), "{what}");
        let refused = super::cli_journey::refused_data(output);
        assert_eq!(refused["code"], "service_failure", "{what}: {refused}");
        assert_eq!(refused["service_code"], "unauthorized", "{what}: {refused}");
        assert_eq!(refused["stage"], "dispatch", "{what}: {refused}");
        refused
    }
    /// Written configuration, owner state and every provider child carry no
    /// OAuth material.
    fn assert_no_material(&self) {
        fn files_under(directory: &Path, found: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(directory).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    files_under(&entry.path(), found);
                } else {
                    found.push(entry.path());
                }
            }
        }
        let mut written = vec![self.cli.paths.config.clone(), self.provider.config.clone()];
        files_under(&self.cli.paths.state, &mut written);
        for path in &written {
            assert!(
                !carries_oauth_material(&fs::read(path).unwrap_or_default()),
                "{}",
                path.display()
            );
        }
        assert_no_material_at_rest(&self.provider);
    }
}
fn invalid_grant() -> TokenAnswer {
    TokenAnswer::Refuse(
        400,
        json!({"error": "invalid_grant", "error_description": "Token has been expired or revoked."}),
    )
}

/// A stored connection whose refresh token the token host now refuses as
/// `invalid_grant` is reported with `next_action = repair_connection` on the
/// next invoke; so is a cached access token the API refuses.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn oauth_invalid_grant_reports_repair() {
    use super::cli_journey::success;
    let journey = OAuthCli::new();
    let provider = &journey.provider;
    // Tokens expire within the skew, so every invoke exchanges again.
    provider.set_token(grant(60));
    let invoke = journey.connected();
    let result = success(journey.run(&invoke));
    assert_eq!(
        serde_json::from_str::<Value>(result["result"].as_str().unwrap()).unwrap()["body"]["id"],
        7
    );

    // The grant is revoked at the token host.
    provider.set_token(invalid_grant());
    let exchanged = provider.token_requests();
    let refused = OAuthCli::refused(journey.run(&invoke), "invoke after revocation");
    assert_eq!(refused["next_action"], "repair_connection", "{refused}");
    assert_eq!(provider.token_requests(), exchanged + 1);
    assert!(
        provider.api_authorizations().len() == 2,
        "the refused exchange reached the API"
    );

    // A cached access token the API refuses is the stored credential's too.
    provider.set_token(grant(3600));
    success(journey.run(&invoke));
    provider.state.lock().unwrap().api_refuses = true;
    let exchanged = provider.token_requests();
    let refused = OAuthCli::refused(journey.run(&invoke), "invoke with the cached token");
    assert_eq!(refused["next_action"], "repair_connection", "{refused}");
    assert_eq!(provider.token_requests(), exchanged);
    journey.assert_no_material();
}

/// A credential refused while the connection is being made is not a stored
/// credential: there is nothing to repair, and the refusal keeps the next
/// action every other `service_failure` has.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn oauth_connect_refusal_does_not_say_repair() {
    let journey = OAuthCli::new();
    let provider = &journey.provider;
    provider.set_token(invalid_grant());
    let refused = OAuthCli::refused(journey.connect(), "connect");
    assert_eq!(refused["next_action"], "retry_explicitly", "{refused}");
    assert_eq!(provider.token_requests(), 1);
    assert!(provider.api_authorizations().is_empty());
    // The API refusing the fresh token at connect reads the same.
    provider.set_token(grant(3600));
    provider.state.lock().unwrap().api_refuses = true;
    let refused = OAuthCli::refused(journey.connect(), "connect, API refusal");
    assert_eq!(refused["next_action"], "retry_explicitly", "{refused}");
    journey.assert_no_material();
}
