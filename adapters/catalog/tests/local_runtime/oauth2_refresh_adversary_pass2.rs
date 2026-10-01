//! Second adversary pass on the `oauth2_refresh` profile. A scripted disposable
//! HTTPS fixture serves a token route and the GitLab API (reads and one guarded
//! write) on one host, with the token host's trust roots in a file of their own.
//! Fixture secrets are fictional and only compared, never printed.
use super::*;
use connectors_host::local::{
    owner,
    runtime::{PrivateProtocol, WriteEffect},
};

const CLIENT_ID: &str = "adversary2-client-id.apps.example.test";
const CLIENT_SECRET: &str = "adversary2-client-secret-one";
const REFRESH: &str = "adversary2-refresh-token-one";
const ACCESS: &str = "adversary2-access-token-";
const PROFILE: &str = "adversary2.oauth";
const DRIVE: &str = "https://www.googleapis.com/auth/drive.readonly";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn entry() -> Secret {
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
            "refresh_token": REFRESH,
        }))
        .unwrap(),
    )
}

fn carries_material(haystack: &[u8]) -> bool {
    [CLIENT_SECRET, REFRESH, ACCESS].iter().any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn unsigned_jwt(claims: &Value) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    format!(
        "{}.{}.{}",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap()),
        URL_SAFE_NO_PAD.encode(b"sig")
    )
}

/// Claims the provider accepts: Google issuer, this client, now-relative times.
fn claims() -> Value {
    json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "sub": "1",
           "iat": now_s(), "exp": now_s() + 3600})
}

#[derive(Clone)]
enum Script {
    /// A grant whose access token is `ACCESS<n>`, with these fields merged in.
    Grant(Value),
    /// A verbatim status and body.
    Raw(u16, Vec<u8>),
}

#[derive(Clone)]
struct Seen {
    route: String,
}

struct State {
    seen: Vec<Seen>,
    token: Script,
    issued: u32,
    refuse_writes: bool,
}

#[derive(Clone, Copy)]
enum Identity {
    Api,
    IdToken,
}

struct Fixture {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    directory: PathBuf,
    token_ca: PathBuf,
    config: PathBuf,
    base: Value,
    state: Arc<Mutex<State>>,
}

fn grant() -> Script {
    Script::Grant(json!({"expires_in": 3600, "token_type": "Bearer"}))
}

impl Fixture {
    fn new(identity: Identity) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
        let token_ca = directory.join("token-ca.pem");
        private(&token_ca, cert.cert.pem().as_bytes());
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
                    let (status, answer) = {
                        let mut state = served.lock().unwrap();
                        state.seen.push(Seen {
                            route: route.clone(),
                        });
                        let issued = (1..=state.issued)
                            .any(|n| authorization == Some(format!("Bearer {ACCESS}{n}")));
                        let json = |status: u16, body: Value| (status, serde_json::to_vec(&body).unwrap());
                        if route == "/token" && method == "POST" {
                            match state.token.clone() {
                                Script::Raw(status, body) => (status, body),
                                Script::Grant(extra) => {
                                    state.issued += 1;
                                    let mut body =
                                        json!({"access_token": format!("{ACCESS}{}", state.issued)});
                                    for (key, value) in extra.as_object().unwrap() {
                                        body[key] = value.clone();
                                    }
                                    json(200, body)
                                }
                            }
                        } else if !route.starts_with("/api/v4/") {
                            json(404, json!({"error": "fixture route"}))
                        } else if !issued {
                            json(401, json!({"error": "fixture refusal"}))
                        } else if route.contains("/repository/branches/") {
                            json(200, json!({"name": "fix", "commit": {"id": SHA}}))
                        } else if method == "POST" && route.ends_with("/merge_requests") {
                            if state.refuse_writes {
                                json(401, json!({"error": "fixture refusal"}))
                            } else {
                                json(201, json!({"id": 10, "iid": 7, "sha": SHA, "state": "opened", "title": "t"}))
                            }
                        } else if route == "/api/v4/user" {
                            json(200, json!({"id": 42}))
                        } else {
                            json(200, json!({"id": 7, "name": "fixture-project"}))
                        }
                    };
                    let response = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
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
        let (identity, minimum) = match identity {
            Identity::Api => (
                json!({"path": "user", "kind": "fixture.user", "subject_pointer": "/id"}),
                json!([]),
            ),
            Identity::IdToken => (
                json!({"source": "id_token", "kind": "google.user"}),
                json!([DRIVE]),
            ),
        };
        let base = json!({
            "format": "connectors-catalog-local/2",
            "instance": "adversary2-oauth",
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
                "token_ca_file": token_ca,
                "authorize_url": "https://localhost/authorize",
                "requested_scopes": ["openid", DRIVE],
            },
            "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
        });
        let config = directory.join("catalog.json");
        private(&config, &serde_json::to_vec(&base).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            directory,
            token_ca,
            config,
            base,
            state,
        }
    }
    /// Rewrites the configuration as the base with `edit` applied.
    fn configure(&self, edit: impl Fn(&mut Value)) {
        let mut config = self.base.clone();
        edit(&mut config);
        private(&self.config, &serde_json::to_vec(&config).unwrap());
    }
    /// The printed bootstrap, or `None` when the provider refused the file.
    fn bootstrap(&self) -> Option<Value> {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(!carries_material(&output.stdout) && !carries_material(&output.stderr));
        output
            .status
            .success()
            .then(|| serde_json::from_slice(&output.stdout).unwrap())
    }
    fn selection(&self, protocol: Option<PrivateProtocol>) -> Adapter {
        let bootstrap = self.bootstrap().expect("bootstrap refused");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: protocol,
            permissions: Default::default(),
            instance_id: "adversary2-oauth".into(),
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
    fn spawn(&self, protocol: Option<PrivateProtocol>) -> Child {
        Child::spawn(&self.selection(protocol))
            .unwrap_or_else(|failure| panic!("spawn {failure:?}"))
    }
    fn set_token(&self, script: Script) {
        self.state.lock().unwrap().token = script;
    }
    fn seen(&self) -> Vec<Seen> {
        self.state.lock().unwrap().seen.clone()
    }
    fn token_requests(&self) -> usize {
        self.seen().iter().filter(|s| s.route == "/token").count()
    }
    fn api_requests(&self) -> usize {
        self.seen()
            .iter()
            .filter(|s| s.route.starts_with("/api/v4/"))
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn project(child: &mut Child) -> Result<Value, Failure> {
    invoke(
        child,
        "project.get",
        "one",
        &entry(),
        json!({"id": "org/project"}),
    )
}

fn write_input() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "id": "org/project",
        "sha": SHA,
        "body": {"source_branch": "fix", "target_branch": "main", "title": "t"}
    }))
    .unwrap()
}

/// `contracts/cli/v1alpha1/semantics.md` and `docs/local-catalog-provider.md`:
/// a stored credential the provider refuses on `operations invoke` of a read
/// reports `next_action = repair_connection`; a guarded write refused for its
/// credential reports `retry_status` until
/// `story:guarded-write-credential-refusal-says-repair` lands. This case pins
/// today's write behaviour. Its refusal is classified by
/// the host's own chain: `mutation.rs:281-288` for a committed write the
/// provider refused, `execution.rs:242` for a refused preflight, each through
/// `owner::Error::from` and `mutation::Failure::from`, which this case applies
/// to what the real child returned.
#[test]
fn adversary2_write_credential_refusal_names_repair_like_a_read() {
    use owner::mutation::{Classification, Failure as WriteFailure};
    let fixture = Fixture::new(Identity::Api);
    let mut child = fixture.spawn(Some(PrivateProtocol::V2));
    project(&mut child).unwrap_or_else(|f| panic!("read {f:?}"));
    let revision = child.bootstrap().descriptor().unwrap().revision;

    // The API refuses the committed write with 401.
    fixture.state.lock().unwrap().refuse_writes = true;
    let written = child
        .prepare_write(
            "merge_request.create",
            &revision,
            "one",
            &entry(),
            &write_input(),
            deadline(),
        )
        .unwrap_or_else(|f| panic!("prepare {f:?}"))
        .commit();
    assert!(matches!(written.effect, WriteEffect::Refused));
    let Err(committed) = written.result else {
        panic!("a refused write carried a result")
    };
    assert!(
        matches!(committed, Failure::InvalidCredential),
        "{committed:?}"
    );
    let committed = WriteFailure::from(owner::Error::from(committed));

    // The token host refuses the stored refresh token at the write's preflight.
    fixture.set_token(Script::Raw(400, br#"{"error":"invalid_grant"}"#.to_vec()));
    let Err(preflight) = child.prepare_write(
        "merge_request.create",
        &revision,
        "one",
        &entry(),
        &write_input(),
        deadline(),
    ) else {
        panic!("a revoked refresh token prepared a write")
    };
    assert!(
        matches!(preflight, Failure::InvalidCredential),
        "{preflight:?}"
    );
    let preflight = WriteFailure::from(owner::Error::from(preflight));
    // The same refusal on a read is what the CLI reports as repair_connection.
    assert!(matches!(
        project(&mut child),
        Err(Failure::InvalidCredential)
    ));

    let reported = (
        committed.next_action(Classification::Refused),
        preflight.next_action(Classification::NotAttempted),
    );
    assert_eq!(
        reported,
        ("retry_status", "retry_status"),
        "a write whose stored credential the provider refused reports {reported:?} \
         (preflight stage {}); the contract names retry_status for writes until \
         story:guarded-write-credential-refusal-says-repair lands — when it does, flip \
         this case to (\"repair_connection\", \"repair_connection\")",
        preflight.stage(Classification::NotAttempted)
    );
}

/// A `minimum_scopes` entry that no granted set can ever contain — a
/// space-separated scope string written as one entry, or one with a stray space
/// — loads, and every validation is then `InsufficientScope` even when the
/// token answer grants every requested scope. The id_token path splits the
/// granted `scope` on whitespace (`local.rs:479`), so the entry can never match.
#[test]
fn adversary2_unsatisfiable_minimum_scope_is_refused_at_load() {
    let fixture = Fixture::new(Identity::IdToken);
    let mut loaded = Vec::new();
    for minimum in [format!("openid {DRIVE}"), format!("{DRIVE} ")] {
        fixture.configure(|config| config["auth"]["minimum_scopes"] = json!([minimum]));
        if fixture.bootstrap().is_none() {
            continue;
        }
        let mut child = fixture.spawn(None);
        fixture.set_token(Script::Grant(json!({
            "expires_in": 3600, "token_type": "Bearer",
            "scope": format!("openid {DRIVE}"),
            "id_token": unsigned_jwt(&claims()),
        })));
        let result = child.validate(PROFILE, &entry(), deadline()).map(|_| ());
        loaded.push(format!(
            "{minimum:?} loaded; validate granting every requested scope gave {result:?}"
        ));
    }
    assert!(
        loaded.is_empty(),
        "unsatisfiable minimum_scopes were accepted: {loaded:#?}"
    );
}

/// The token host's trust roots are captured at load and covered by the
/// revision: changed bytes refuse a spawn of the old selection, and roots that
/// do not trust the token host fail the exchange before any API request.
#[test]
fn adversary2_token_ca_bytes_are_bound_and_enforced() {
    let fixture = Fixture::new(Identity::Api);
    let selection = fixture.selection(None);
    let mut child = Child::spawn(&selection).unwrap();
    let other = rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .unwrap()
        .cert
        .pem();
    private(&fixture.token_ca, other.as_bytes());
    // The running child keeps the roots it loaded.
    project(&mut child).unwrap_or_else(|f| panic!("read {f:?}"));
    assert!(matches!(
        Child::spawn(&selection),
        Err(Failure::ReadinessMismatch)
    ));
    let (tokens, api) = (fixture.token_requests(), fixture.api_requests());
    let mut untrusting = fixture.spawn(None);
    assert!(project(&mut untrusting).is_err());
    assert_eq!(
        fixture.token_requests(),
        tokens,
        "the exchange reached HTTP"
    );
    assert_eq!(fixture.api_requests(), api, "an API request was made");
}

/// `ca_file` enters the revision by its bytes only (`local.rs:710`); moving it
/// keeps the revision. `token_ca_file` enters by its bytes (`local.rs:716-718`)
/// and also by its path, through the serialized `auth` (`local.rs:711`), so
/// the same roots at another path are a new revision.
#[test]
fn adversary2_token_ca_path_is_not_part_of_the_revision() {
    let fixture = Fixture::new(Identity::Api);
    let revision = |fixture: &Fixture| {
        fixture.bootstrap().expect("bootstrap refused")["configuration_revision"].clone()
    };
    let before = revision(&fixture);
    let moved = fixture.directory.join("moved-ca.pem");
    private(&moved, &fs::read(&fixture.token_ca).unwrap());
    fixture.configure(|config| config["ca_file"] = json!(moved));
    assert_eq!(revision(&fixture), before, "ca_file moved");
    fixture.configure(|config| config["auth"]["token_ca_file"] = json!(moved));
    assert_eq!(
        revision(&fixture),
        before,
        "the same token trust roots at another path changed the revision"
    );
}

/// Token answers over the provider's document bound, and over the transport's
/// response bound, fail without an API request and cache nothing.
#[test]
fn adversary2_oversized_token_answers_cache_nothing() {
    let fixture = Fixture::new(Identity::Api);
    let mut child = fixture.spawn(None);
    for size in [64 * 1024 + 1, connectors_core::RESPONSE_LIMIT + 1] {
        let body = format!(
            r#"{{"access_token":"{ACCESS}0","token_type":"Bearer","expires_in":3600,"padding":"{}"}}"#,
            "x".repeat(size)
        );
        serde_json::from_str::<Value>(&body).unwrap();
        fixture.set_token(Script::Raw(200, body.into_bytes()));
        assert!(project(&mut child).is_err(), "{size}");
    }
    assert_eq!(fixture.api_requests(), 0);
    fixture.set_token(grant());
    let before = fixture.token_requests();
    project(&mut child).unwrap_or_else(|f| panic!("read {f:?}"));
    assert_eq!(fixture.token_requests(), before + 1);
}

/// The correction's `iat` check (`local.rs:429-431`): no case in the suite
/// sends an `iat` ahead of now, so dropping the check leaves it green. An
/// `id_token` issued an hour ahead is refused; one a minute ahead is inside the
/// five-minute skew; a missing `exp` is refused.
#[test]
fn adversary2_id_token_time_claims_are_enforced() {
    let fixture = Fixture::new(Identity::IdToken);
    let mut child = fixture.spawn(None);
    let answer = |claims: &Value| {
        Script::Grant(json!({
            "expires_in": 3600, "token_type": "Bearer",
            "scope": format!("openid {DRIVE}"),
            "id_token": unsigned_jwt(claims),
        }))
    };
    let mut ahead = claims();
    ahead["iat"] = json!(now_s() + 3600);
    fixture.set_token(answer(&ahead));
    assert!(matches!(
        child.validate(PROFILE, &entry(), deadline()),
        Err(Failure::Protocol)
    ));
    let mut near = claims();
    near["iat"] = json!(now_s() + 60);
    fixture.set_token(answer(&near));
    child
        .validate(PROFILE, &entry(), deadline())
        .unwrap_or_else(|f| panic!("iat within skew {f:?}"));
    let mut no_exp = claims();
    no_exp.as_object_mut().unwrap().remove("exp");
    fixture.set_token(answer(&no_exp));
    assert!(matches!(
        child.validate(PROFILE, &entry(), deadline()),
        Err(Failure::Protocol)
    ));
}

/// The granted `scope` is space-separated: repeated and edge spaces still name
/// the same scopes, a non-ASCII space is refused, and an empty grant is
/// insufficient against a non-empty minimum.
#[test]
fn adversary2_scope_whitespace_forms() {
    let fixture = Fixture::new(Identity::IdToken);
    let mut child = fixture.spawn(None);
    let answer = |scope: &str| {
        Script::Grant(json!({
            "expires_in": 3600, "token_type": "Bearer", "scope": scope,
            "id_token": unsigned_jwt(&claims()),
        }))
    };
    fixture.set_token(answer(&format!("  openid   {DRIVE}  ")));
    let baseline = child
        .validate(PROFILE, &entry(), deadline())
        .unwrap_or_else(|f| panic!("spaced scope {f:?}"));
    assert_eq!(
        baseline.granted_scopes.unwrap(),
        ["openid".to_owned(), DRIVE.to_owned()].into()
    );
    fixture.set_token(answer(&format!("openid\u{a0}{DRIVE}")));
    assert!(matches!(
        child.validate(PROFILE, &entry(), deadline()),
        Err(Failure::Protocol)
    ));
    fixture.set_token(answer(""));
    assert!(matches!(
        child.validate(PROFILE, &entry(), deadline()),
        Err(Failure::InsufficientScope)
    ));
}

/// The host's own check of a bootstrap (`runtime.rs:329`) bounds every other
/// profile field and none of `acquisition`: a child may claim a plaintext
/// token endpoint and consent URL and any number of scopes, and the owner
/// caches that record (`er.rs:785-797`) as it would a checked one.
#[test]
fn adversary2_host_bootstrap_check_bounds_acquisition() {
    let fixture = Fixture::new(Identity::Api);
    let mut bootstrap = fixture.bootstrap().expect("bootstrap refused");
    bootstrap["profiles"][0]["acquisition"] = json!({
        "authorize_url": "http://consent.example.test/authorize",
        "token_url": "",
        "scopes": (0..1000).map(|n| format!("scope-{n}")).collect::<Vec<_>>(),
    });
    let bootstrap: Bootstrap = serde_json::from_value(bootstrap).unwrap();
    assert!(
        bootstrap.validate().is_err(),
        "an acquisition with a plaintext consent URL, an empty token URL and 1000 scopes validated"
    );
}

/// The OAuth profile's cached bootstrap, `acquisition` included, survives an
/// owner restart: the record reads back equal to what the provider printed,
/// `adapters describe` serves it as cached, and a fresh owner invokes again.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary2_oauth_cached_bootstrap_round_trips_across_owner_restart() {
    use super::cli_journey::{Cli, Custody, success};
    let fixture = Fixture::new(Identity::Api);
    let custody = Custody::new(fixture.root.path());
    let cli = Cli::new(fixture.root.path());
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = fixture.selection(None);
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.drive]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.drive.permissions]\nprofiles=[{}]\noperations=['project.get']\n[adapters.drive.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(PROFILE),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    let credential = fixture.directory.join("credential.json");
    private(&credential, &entry().0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "drive",
        "--profile",
        PROFILE,
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "drive",
        "--operation",
        "project.get",
    ]));
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "drive",
        "--connection",
        &reference,
        "--operation",
        "project.get",
        "--schema",
        description["schema"].as_str().unwrap(),
        "--revision",
        description["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"id":"org/project"}"#,
    ];
    success(cli.run(&invoke));

    let client = owner::Client::connect(&cli.paths, false).unwrap();
    let host = client.host_incarnation.clone();
    client.shutdown(&host).unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(5);
    while cli.paths.state.join("owner.sock").exists() {
        assert!(std::time::Instant::now() < until, "owner did not stop");
        std::thread::sleep(Duration::from_millis(10));
    }

    let printed = fixture.bootstrap().unwrap()["profiles"][0].clone();
    let cached =
        owner::cached(&cli.paths, "drive").unwrap_or_else(|e| panic!("cached bootstrap {e:?}"));
    let cached = serde_json::to_value(cached.profile(PROFILE).unwrap()).unwrap();
    assert_eq!(cached, printed);
    assert!(cached["acquisition"].is_object());
    let described = success(cli.run(&["adapters", "describe", "--adapter", "drive"]));
    assert_eq!(described["source"], "cached");
    let tokens = fixture.token_requests();
    success(cli.run(&invoke));
    assert_eq!(fixture.token_requests(), tokens + 1);
    let status = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "drive",
        "--connection",
        &reference,
    ]));
    assert_eq!(status["connection"]["summary"]["state"], "ready");
    for path in [&cli.paths.config, &fixture.config] {
        assert!(!carries_material(&fs::read(path).unwrap()));
    }
}
