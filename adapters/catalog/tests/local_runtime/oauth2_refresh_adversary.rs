//! Adversary cases for the `oauth2_refresh` profile. A scripted disposable
//! HTTPS fixture serves a token route, a tokeninfo route and the GitLab API
//! (reads and one guarded write) on one host. Fixture secrets are fictional and
//! only compared, never printed.
use super::*;
use connectors_host::local::runtime::{PrivateProtocol, WriteEffect};

const CLIENT_ID: &str = "adversary-client-id.apps.example.test";
const CLIENT_SECRET: &str = "adversary-client-secret-one";
const REFRESH_ONE: &str = "adversary-refresh-token-one";
const REFRESH_TWO: &str = "adversary-refresh-token-two";
const ACCESS: &str = "adversary-access-token-";
const PROFILE: &str = "adversary.oauth";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn entry(refresh_token: &str) -> Secret {
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
            "refresh_token": refresh_token,
        }))
        .unwrap(),
    )
}

/// A raw scripted answer: status, extra header lines and body bytes.
#[derive(Clone)]
struct Raw(u16, Vec<String>, Vec<u8>);
fn json_answer(status: u16, body: Value) -> Raw {
    Raw(status, Vec::new(), serde_json::to_vec(&body).unwrap())
}

#[derive(Clone)]
enum TokenScript {
    /// A grant whose access token is `ACCESS<n>`, with `extra` merged in.
    Grant(Value),
    /// A verbatim answer.
    Raw(Raw),
}

#[derive(Clone)]
struct Seen {
    method: String,
    route: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

struct State {
    seen: Vec<Seen>,
    token: TokenScript,
    issued: u32,
    refuse_writes: bool,
}

struct Fixture {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    state: Arc<Mutex<State>>,
    port: u16,
}

fn grant() -> TokenScript {
    TokenScript::Grant(json!({"expires_in": 3600, "token_type": "Bearer"}))
}

impl Fixture {
    fn new(identity: Value, minimum: Value) -> Self {
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
        let state = Arc::new(Mutex::new(State {
            seen: Vec::new(),
            token: grant(),
            issued: 0,
            refuse_writes: false,
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
                        if head.len() > 8192 {
                            break;
                        }
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
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
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let length: usize = header("content-length")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0)
                        .min(65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let authorization = header("authorization");
                    let Raw(status, extra, answer) = {
                        let mut state = served.lock().unwrap();
                        state.seen.push(Seen {
                            method: method.clone(),
                            route: route.clone(),
                            authorization: authorization.clone(),
                            body,
                        });
                        let issued = (1..=state.issued)
                            .any(|n| authorization == Some(format!("Bearer {ACCESS}{n}")));
                        if route == "/token" && method == "POST" {
                            match state.token.clone() {
                                TokenScript::Raw(raw) => raw,
                                TokenScript::Grant(extra) => {
                                    state.issued += 1;
                                    let mut body =
                                        json!({"access_token": format!("{ACCESS}{}", state.issued)});
                                    for (key, value) in extra.as_object().unwrap() {
                                        body[key] = value.clone();
                                    }
                                    json_answer(200, body)
                                }
                            }
                        } else if !route.starts_with("/api/v4/") {
                            json_answer(404, json!({"error": "fixture route"}))
                        } else if !issued {
                            json_answer(401, json!({"error": "fixture refusal"}))
                        } else if route.contains("/repository/branches/") {
                            json_answer(200, json!({"name": "fix", "commit": {"id": SHA}}))
                        } else if method == "POST" && route.ends_with("/merge_requests") {
                            if state.refuse_writes {
                                json_answer(401, json!({"error": "fixture refusal"}))
                            } else {
                                json_answer(
                                    201,
                                    json!({"id": 10, "iid": 7, "sha": SHA, "state": "opened", "title": "t"}),
                                )
                            }
                        } else if route == "/api/v4/user" {
                            json_answer(200, json!({"id": 42}))
                        } else {
                            json_answer(200, json!({"id": 7, "name": "fixture-project"}))
                        }
                    };
                    let mut response = format!("HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n", answer.len());
                    for line in extra {
                        response.push_str(&line);
                        response.push_str("\r\n");
                    }
                    response.push_str("\r\n");
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let port = address_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .port();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let authority = format!("{}{port}", concat!("https://", "localhost:"));
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "adversary-oauth",
                "provider": "gitlab",
                "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
                "api_base": format!("{authority}/api/v4"),
                "ca_file": ca,
                "auth": {
                    "profile": PROFILE,
                    "scheme": "oauth2_refresh",
                    "header": "Authorization",
                    "bearer": true,
                    "label": "Adversary refresh token",
                    "identity": identity,
                    "minimum_scopes": minimum,
                    "token_url": format!("{authority}/token"),
                    "token_ca_file": ca,
                    "authorize_url": "https://localhost/authorize",
                    "requested_scopes": ["openid"],
                },
                "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            state,
            port,
        }
    }
    fn api() -> Self {
        Self::new(
            json!({"path": "user", "kind": "fixture.user", "subject_pointer": "/id"}),
            json!([]),
        )
    }
    fn id_token() -> Self {
        Self::new(
            json!({"source": "id_token", "kind": "google.user"}),
            json!(["openid"]),
        )
    }
    fn spawn(&self, protocol: Option<PrivateProtocol>) -> Child {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(output.status.success(), "bootstrap refused");
        let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Child::spawn(&Adapter {
            private_protocol: protocol,
            permissions: Default::default(),
            instance_id: "adversary-oauth".into(),
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
        })
        .unwrap_or_else(|failure| panic!("spawn {failure:?}"))
    }
    fn set_token(&self, script: TokenScript) {
        self.state.lock().unwrap().token = script;
    }
    fn seen(&self) -> Vec<Seen> {
        self.state.lock().unwrap().seen.clone()
    }
    fn token_requests(&self) -> usize {
        self.seen().iter().filter(|s| s.route == "/token").count()
    }
    fn last_api_authorization(&self) -> Option<String> {
        self.seen()
            .into_iter()
            .rfind(|s| s.route.starts_with("/api/v4/"))
            .and_then(|s| s.authorization)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
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

fn unsigned_jwt(payload: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    format!(
        "{}.{}.{}",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#),
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(b"sig")
    )
}

/// A 401 on a committed write is the provider refusing the cached access
/// token exactly as a 401 on a read is. The read path evicts it; the write path
/// must too, or every later write and read reuses a token the provider has
/// already refused until the cache entry expires.
#[test]
fn adversary_write_401_evicts_cached_access_token() {
    let fixture = Fixture::api();
    let mut child = fixture.spawn(Some(PrivateProtocol::V2));
    let document = entry(REFRESH_ONE);
    project(&mut child, &document).unwrap_or_else(|f| panic!("read {f:?}"));
    assert_eq!(fixture.token_requests(), 1);

    fixture.state.lock().unwrap().refuse_writes = true;
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let input = serde_json::to_vec(&json!({
        "id": "org/project",
        "sha": SHA,
        "body": {"source_branch": "fix", "target_branch": "main", "title": "t"}
    }))
    .unwrap();
    let prepared = child
        .prepare_write(
            "merge_request.create",
            &revision,
            "one",
            &document,
            &input,
            deadline(),
        )
        .unwrap_or_else(|f| panic!("prepare {f:?}"));
    let written = prepared.commit();
    assert!(
        matches!(written.effect, WriteEffect::Refused),
        "the fixture refused the write with 401"
    );
    assert!(
        fixture
            .seen()
            .iter()
            .any(|s| s.method == "POST" && s.route.ends_with("/merge_requests")),
        "the write was dispatched"
    );
    // The refused token is not used again: the next request exchanges anew.
    fixture.state.lock().unwrap().refuse_writes = false;
    project(&mut child, &document).unwrap_or_else(|f| panic!("read {f:?}"));
    assert_eq!(
        fixture.token_requests(),
        2,
        "a token refused with 401 on a write was served again from the cache"
    );
    assert!(fixture.last_api_authorization().as_deref() == Some(&*format!("Bearer {ACCESS}2")));
}

/// Two protected entries never share a cached access token: the second entry
/// exchanges its own refresh token, and the API sees its own access token.
#[test]
fn adversary_distinct_entries_never_share_a_cached_token() {
    let fixture = Fixture::api();
    let mut child = fixture.spawn(None);
    project(&mut child, &entry(REFRESH_ONE)).unwrap();
    project(&mut child, &entry(REFRESH_TWO)).unwrap();
    assert_eq!(fixture.token_requests(), 2);
    let seen = fixture.seen();
    let token_bodies: Vec<&Seen> = seen.iter().filter(|s| s.route == "/token").collect();
    let carries = |body: &[u8], needle: &str| {
        body.windows(needle.len())
            .any(|window| window == needle.as_bytes())
    };
    assert!(carries(&token_bodies[1].body, REFRESH_TWO));
    assert!(!carries(&token_bodies[1].body, REFRESH_ONE));
    assert!(fixture.last_api_authorization().as_deref() == Some(&*format!("Bearer {ACCESS}2")));
    // Each entry keeps using its own token.
    project(&mut child, &entry(REFRESH_ONE)).unwrap();
    assert_eq!(fixture.token_requests(), 2);
    assert!(fixture.last_api_authorization().as_deref() == Some(&*format!("Bearer {ACCESS}1")));
}

/// A token endpoint redirect is never followed, and the form is sent once.
#[test]
fn adversary_token_redirect_is_not_followed() {
    let fixture = Fixture::api();
    let mut child = fixture.spawn(None);
    for status in [301, 302, 307, 308] {
        let location = format!(
            "Location: {}{}/api/v4/user",
            concat!("https://", "localhost:"),
            fixture.port
        );
        fixture.set_token(TokenScript::Raw(Raw(
            status,
            vec![location],
            b"{}".to_vec(),
        )));
        let before = fixture.seen().len();
        assert!(
            matches!(
                project(&mut child, &entry(REFRESH_ONE)),
                Err(Failure::Protocol)
            ),
            "{status}"
        );
        let after = fixture.seen();
        assert_eq!(after.len(), before + 1, "{status}: a redirect was followed");
        assert_eq!(after.last().unwrap().route, "/token");
    }
}

/// Expiry arithmetic at the extremes: an enormous `expires_in` is capped and
/// cached, zero and non-integer values are refused and nothing is cached.
#[test]
fn adversary_expires_in_extremes() {
    let fixture = Fixture::api();
    let mut child = fixture.spawn(None);
    fixture.set_token(TokenScript::Grant(
        json!({"expires_in": u64::MAX, "token_type": "Bearer"}),
    ));
    project(&mut child, &entry(REFRESH_ONE)).unwrap();
    project(&mut child, &entry(REFRESH_ONE)).unwrap();
    assert_eq!(fixture.token_requests(), 1);
    for expires_in in [
        json!(0),
        json!(-1),
        json!("3599"),
        json!(3599.5),
        json!(null),
    ] {
        let mut child = fixture.spawn(None);
        fixture.set_token(TokenScript::Grant(
            json!({"expires_in": expires_in, "token_type": "Bearer"}),
        ));
        let before = fixture.token_requests();
        assert!(
            matches!(
                project(&mut child, &entry(REFRESH_ONE)),
                Err(Failure::Protocol)
            ),
            "{expires_in}"
        );
        assert_eq!(fixture.token_requests(), before + 1);
    }
}

/// A malformed `id_token` or one whose claims do not match is refused, and
/// nothing is cached from the refused validation.
#[test]
fn adversary_malformed_id_token_refused() {
    let fixture = Fixture::id_token();
    let mut child = fixture.spawn(None);
    let good = json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "sub": "1"});
    let cases: Vec<(&str, String)> = vec![
        ("two parts", {
            let jwt = unsigned_jwt(&serde_json::to_vec(&good).unwrap());
            jwt.rsplit_once('.').unwrap().0.to_owned()
        }),
        ("four parts", format!("{}.x", unsigned_jwt(&serde_json::to_vec(&good).unwrap()))),
        ("payload not json", unsigned_jwt(b"not json")),
        ("payload not base64", "e30.@@@@.c2ln".to_owned()),
        (
            "aud array",
            unsigned_jwt(
                &serde_json::to_vec(
                    &json!({"iss": "https://accounts.google.com", "aud": [CLIENT_ID, "other"], "sub": "1"}),
                )
                .unwrap(),
            ),
        ),
        (
            "no sub",
            unsigned_jwt(
                &serde_json::to_vec(&json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID}))
                    .unwrap(),
            ),
        ),
        (
            "aud prefix",
            unsigned_jwt(
                &serde_json::to_vec(
                    &json!({"iss": "https://accounts.google.com", "aud": &CLIENT_ID[..10], "sub": "1"}),
                )
                .unwrap(),
            ),
        ),
        (
            "iss case",
            unsigned_jwt(
                &serde_json::to_vec(
                    &json!({"iss": "https://Accounts.Google.com", "aud": CLIENT_ID, "sub": "1"}),
                )
                .unwrap(),
            ),
        ),
    ];
    for (what, id_token) in cases {
        fixture.set_token(TokenScript::Grant(json!({
            "expires_in": 3600, "token_type": "Bearer", "scope": "openid", "id_token": id_token
        })));
        let result = child.validate(PROFILE, &entry(REFRESH_ONE), deadline());
        assert!(matches!(result, Err(Failure::Protocol)), "{what}");
    }
    // No refused validation left a token behind: an invoke exchanges again.
    fixture.set_token(grant());
    let before = fixture.token_requests();
    project(&mut child, &entry(REFRESH_ONE)).unwrap();
    assert_eq!(fixture.token_requests(), before + 1);
    // Nothing but the token route and that one read was contacted.
    assert!(
        fixture
            .seen()
            .iter()
            .all(|s| s.route == "/token" || s.route.starts_with("/api/v4/projects/"))
    );
}

/// The provider cites OpenID Connect Core 3.1.3.7 for skipping the signature
/// of an `id_token` received over TLS; the same section requires the current
/// time to be before `exp`. An `id_token` that expired long ago is accepted.
#[test]
fn adversary_expired_id_token_refused() {
    let fixture = Fixture::id_token();
    let mut child = fixture.spawn(None);
    let expired = json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID,
                         "sub": "1", "iat": 1, "exp": 2});
    fixture.set_token(TokenScript::Grant(json!({
        "expires_in": 3600, "token_type": "Bearer", "scope": "openid",
        "id_token": unsigned_jwt(&serde_json::to_vec(&expired).unwrap())
    })));
    assert!(
        matches!(
            child.validate(PROFILE, &entry(REFRESH_ONE), deadline()),
            Err(Failure::Protocol)
        ),
        "an id_token whose exp is in 1970 validated a connection"
    );
}
