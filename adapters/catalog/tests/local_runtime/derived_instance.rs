//! story:connector-instance-derived-fields, its acceptance: with a
//! configuration whose only Gmail lines are `[connectors.gmail]` with
//! `enabled = true` and an empty `[connectors.gmail.instances.private]`, one
//! `connections connect` yields one connection under which a Gmail read,
//! `users.drafts.create` and `users.drafts.send`, each write with its approval
//! proof, succeed.
//!
//! The instance names no endpoint, no trust root and no token URL, so the
//! provider the host derives talks to Gmail's own hosts. The journey runs in
//! fresh user, network and mount namespaces in which `gmail.googleapis.com`,
//! `oauth2.googleapis.com` and `accounts.google.com` resolve to a disposable
//! HTTPS fixture on loopback port 443, and the system trust store holds only
//! the fixture's root. Nothing outside the namespaces is reached or changed.
//! The production CLI and the catalog provider are copied side by side into
//! the fixture's private directory, the installation the CLI derives from.
//! The fixture secrets are fictional and only ever compared, never printed.
use super::cli_journey::{Custody, success};
use super::*;
use std::{
    collections::BTreeSet,
    process::{Output, Stdio},
    sync::atomic::AtomicBool,
    time::Instant,
};

/// Set in the re-executed test process that runs inside the namespaces.
const INSIDE: &str = "CONNECTORS_TEST_DERIVED_GMAIL_NAMESPACES";
const NAME: &str = "derived_instance::a_gmail_instance_configured_by_name_alone_connects_once_and_reads_and_writes_drafts";
const HOSTS: [&str; 3] = [
    "gmail.googleapis.com",
    "oauth2.googleapis.com",
    "accounts.google.com",
];
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-derived";
const AUTHORIZATION_CODE: &str = "fixture-authorization-code-derived";
const REFRESH_TOKEN: &str = "fixture-refresh-token-derived";
const ACCESS_TOKEN: &str = "fixture-access-token-derived-";
const COMPOSE_SCOPE: &str = "https://www.googleapis.com/auth/gmail.compose";
const READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const DRAFT: &str = "fixture-draft-1";
const DRAFT_MESSAGE: &str = "fixture-draft-message-1";
/// A base64url RFC 5322 message: `To`, `Subject`, a blank line, a body.
const RAW: &str =
    "VG86IHJlY2lwaWVudEBleGFtcGxlLnRlc3QNClN1YmplY3Q6IEZpeHR1cmUNCg0KRml4dHVyZSBib2R5DQo";

/// How every command selects the instance (story:cli-connector-instance-selectors).
const SELECTION: &[&str] = &["--connector", "gmail", "--instance", "private"];

/// How many objects in `value` name the alias `gmail.private`; each must name
/// `connector` `gmail` and `instance` `private` beside it.
fn names_the_instance(value: &Value) -> usize {
    match value {
        Value::Object(object) => {
            let own = usize::from(object.get("adapter") == Some(&json!("gmail.private")));
            if own == 1 {
                assert_eq!(object.get("connector"), Some(&json!("gmail")), "{value}");
                assert_eq!(object.get("instance"), Some(&json!("private")), "{value}");
            }
            own + object.values().map(names_the_instance).sum::<usize>()
        }
        Value::Array(items) => items.iter().map(names_the_instance).sum(),
        _ => 0,
    }
}

fn carries_material(haystack: &[u8]) -> bool {
    [
        CLIENT_SECRET,
        AUTHORIZATION_CODE,
        REFRESH_TOKEN,
        ACCESS_TOKEN,
    ]
    .iter()
    .any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn a_gmail_instance_configured_by_name_alone_connects_once_and_reads_and_writes_drafts() {
    let Some(base) = std::env::var_os(INSIDE) else {
        return outside();
    };
    let fixture = Gmail::new(Path::new(&base));
    let journey = Journey::new(&fixture);

    // One connect: consent at accounts.google.com, the code exchanged at
    // oauth2.googleapis.com, and the stored triple validated by a refresh.
    let connect = success(
        journey.follow(
            &[
                &["connections", "connect"][..],
                SELECTION,
                &[
                    "--profile",
                    "google.oauth",
                    "--credential-file",
                    journey.client_file.to_str().unwrap(),
                ],
            ]
            .concat(),
        ),
    );
    let connected = connect["connection"].clone();
    assert_eq!(connected["summary"]["state"], "ready", "{connected}");
    let connection = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let consents = fixture.routes("accounts.google.com", "/o/oauth2/auth");
    assert_eq!(consents.len(), 1, "consent was requested once");
    let scopes: BTreeSet<String> = super::oauth2_refresh::field(
        &super::oauth2_refresh::decode_form(consents[0].query.as_bytes()),
        "scope",
    )
    .unwrap()
    .split(' ')
    .map(str::to_owned)
    .collect();
    assert_eq!(
        scopes,
        ["openid", COMPOSE_SCOPE, READONLY_SCOPE]
            .map(str::to_owned)
            .into()
    );

    // A Gmail read on that connection, at Gmail's own host.
    let listed = journey.invoke(
        &connection,
        "users.messages.list",
        json!({"userId": "me"}),
        &[],
    );
    assert_eq!(
        success(listed)["result"]["body"]["messages"][0]["id"],
        "fixture-message-1"
    );

    // Both writes need an approval proof, and each succeeds with its own.
    journey.cli(&[&["approvals", "key-init"][..], SELECTION].concat());
    let policy = journey.root.join("private/policy.json");
    private(
        &policy,
        br#"{"operations":["users.drafts.create","users.drafts.send"]}"#,
    );
    journey.cli(
        &[
            &["approvals", "policy-set"][..],
            SELECTION,
            &["--input-file", policy.to_str().unwrap()],
        ]
        .concat(),
    );
    let create = json!({"userId": "me", "body": {"message": {"raw": RAW}}});
    let unapproved = journey.invoke(
        &connection,
        "users.drafts.create",
        create.clone(),
        &["--idempotency-key", "derived-create-unapproved"],
    );
    let refused: Value = serde_json::from_slice(&unapproved.stderr).unwrap();
    assert_eq!(
        refused["error"]["data"]["code"], "approval_required",
        "{refused}"
    );
    assert!(fixture.posts().is_empty(), "an unapproved write was sent");

    let created = journey.approved(&connection, "users.drafts.create", create, "derived-create");
    assert_eq!(created["result"]["body"]["id"], DRAFT, "{created}");
    let sent = journey.approved(
        &connection,
        "users.drafts.send",
        json!({"userId": "me", "messageId": DRAFT_MESSAGE, "body": {"id": DRAFT}}),
        "derived-send",
    );
    assert_eq!(sent["result"]["body"]["labelIds"][0], "SENT", "{sent}");
    assert_eq!(
        fixture.posts(),
        [
            (
                "/gmail/v1/users/me/drafts".to_owned(),
                json!({"message": {"raw": RAW}})
            ),
            (
                "/gmail/v1/users/me/drafts/send".to_owned(),
                json!({"id": DRAFT})
            ),
        ]
    );
    // The send read its draft before it was sent.
    assert_eq!(
        fixture
            .routes(
                "gmail.googleapis.com",
                "/gmail/v1/users/me/drafts/fixture-draft-1"
            )
            .len(),
        1
    );

    // One connection carried all three, and no request left the three hosts.
    let listed = success(journey.cli_output(&[&["connections", "list"][..], SELECTION].concat()));
    assert_eq!(
        listed["connections"].as_array().unwrap().len(),
        1,
        "{listed}"
    );
    // Every result that names the derived alias names its connector and
    // instance beside it.
    let adapters = journey.cli(&["adapters", "list"]);
    for result in [&connect, &listed, &adapters] {
        assert!(names_the_instance(result) > 0, "{result}");
    }
    for request in fixture.requests() {
        assert!(HOSTS.contains(&request.host.as_str()), "{}", request.host);
    }
    let gmail = fixture
        .requests()
        .into_iter()
        .filter(|request| request.host == "gmail.googleapis.com")
        .count();
    assert_eq!(
        gmail, 4,
        "the read, the create, the send's preflight and the send"
    );
}

/// Outside the namespaces: copy the production CLI and the catalog provider
/// side by side into a private fixture directory, with a copy of the Secret
/// Service daemon, and run this test again inside fresh user, network and
/// mount namespaces on that directory. The directory is removed here.
fn outside() {
    let base = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let bin = base.path().join("work/bin");
    fs::create_dir_all(&bin).unwrap();
    for directory in [base.path().join("work"), bin.clone()] {
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let built = std::env::var_os("CONNECTORS_TEST_CLI").expect("built production CLI required");
    for (source, target, mode) in [
        (PathBuf::from(built), bin.join("connectors"), 0o700),
        (
            PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider")),
            bin.join("connectors-catalog-provider"),
            0o700,
        ),
        (
            PathBuf::from(KEYRING_DAEMON),
            base.path().join("gnome-keyring-daemon"),
            0o755,
        ),
    ] {
        fs::copy(source, &target).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(mode)).unwrap();
    }
    let output = Command::new("unshare")
        .args(["--user", "--map-root-user", "--net", "--mount", "--"])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            NAME,
            "--ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(INSIDE, base.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!carries_material(&output.stdout) && !carries_material(&output.stderr));
}

const KEYRING_DAEMON: &str = "/usr/bin/gnome-keyring-daemon";
/// The fixture directory inside the namespaces.
const WORK: &str = "/work";

/// Inside the namespaces: a root of our own, so every directory the host
/// admits on a path is owned by this namespace's root, as on a real system.
/// The host's `/usr`, `/etc`, `/dev`, `/proc`, `/sys`, `/run`, `/var` and
/// `/opt` are bound into it, `base/work` is `/work`, the copied Secret Service
/// daemon stands at its system path (the host qualifies it by its owner and
/// content), and `/etc/hosts`, `/etc/nsswitch.conf` and `/etc/ssl` are
/// replaced by `base/etc`. Then loopback is raised and this process changes
/// its root to it; every process it starts inherits that root.
fn enter(base: &Path) {
    let run = |program: &str, args: &[&std::ffi::OsStr]| {
        let status = Command::new(program).args(args).status().unwrap();
        assert!(status.success(), "{program} {args:?}");
    };
    let os = std::ffi::OsStr::new;
    let root = base.join("root");
    fs::create_dir(&root).unwrap();
    run(
        "mount",
        &[os("-t"), os("tmpfs"), os("tmpfs"), root.as_os_str()],
    );
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    for directory in ["usr", "etc", "dev", "proc", "sys", "run", "var", "opt"] {
        let source = Path::new("/").join(directory);
        if source.is_dir() {
            fs::create_dir(root.join(directory)).unwrap();
            run(
                "mount",
                &[
                    os("--rbind"),
                    source.as_os_str(),
                    root.join(directory).as_os_str(),
                ],
            );
        }
    }
    for (link, target) in [
        ("bin", "usr/bin"),
        ("sbin", "usr/bin"),
        ("lib", "usr/lib"),
        ("lib64", "usr/lib"),
    ] {
        std::os::unix::fs::symlink(target, root.join(link)).unwrap();
    }
    fs::create_dir(root.join("tmp")).unwrap();
    fs::set_permissions(root.join("tmp"), fs::Permissions::from_mode(0o1777)).unwrap();
    fs::create_dir(root.join("work")).unwrap();
    let etc = base.join("etc");
    for (source, target) in [
        (base.join("work"), "work"),
        (base.join("gnome-keyring-daemon"), &KEYRING_DAEMON[1..]),
        (etc.join("hosts"), "etc/hosts"),
        (etc.join("nsswitch.conf"), "etc/nsswitch.conf"),
        (etc.join("ssl"), "etc/ssl"),
    ] {
        run(
            "mount",
            &[
                os("--bind"),
                source.as_os_str(),
                root.join(target).as_os_str(),
            ],
        );
    }
    run("ip", &[os("link"), os("set"), os("lo"), os("up")]);
    let root = std::ffi::CString::new(root.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: a valid NUL-terminated path; this test process owns its root.
    assert_eq!(unsafe { libc::chroot(root.as_ptr()) }, 0);
    std::env::set_current_dir("/").unwrap();
}

/// One request the fixture answered. Compared, never printed.
#[derive(Clone)]
struct Request {
    method: String,
    host: String,
    route: String,
    query: String,
    body: Vec<u8>,
}

/// Google's consent, token and Gmail hosts on loopback port 443, behind one
/// certificate for all three names.
struct Gmail {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pem: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Gmail {
    /// Points the three hosts at loopback and makes the fixture's certificate
    /// the only trusted root, enters the private root (`enter`), and serves.
    fn new(base: &Path) -> Self {
        let cert = rcgen::generate_simple_self_signed(HOSTS.map(str::to_owned).to_vec()).unwrap();
        let pem = cert.cert.pem();
        let etc = base.join("etc");
        fs::create_dir_all(etc.join("ssl/certs")).unwrap();
        fs::write(
            etc.join("hosts"),
            format!("127.0.0.1 localhost {}\n", HOSTS.join(" ")),
        )
        .unwrap();
        let nsswitch = fs::read_to_string("/etc/nsswitch.conf").unwrap_or_default();
        let nsswitch: String = nsswitch
            .lines()
            .filter(|line| !line.trim_start().starts_with("hosts:"))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(
            etc.join("nsswitch.conf"),
            format!("{nsswitch}hosts: files\n"),
        )
        .unwrap();
        fs::write(etc.join("ssl/cert.pem"), &pem).unwrap();
        fs::write(etc.join("ssl/certs/ca-certificates.crt"), &pem).unwrap();
        for directory in [&etc, &etc.join("ssl"), &etc.join("ssl/certs")] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        for file in [
            "hosts",
            "nsswitch.conf",
            "ssl/cert.pem",
            "ssl/certs/ca-certificates.crt",
        ] {
            fs::set_permissions(etc.join(file), fs::Permissions::from_mode(0o644)).unwrap();
        }
        enter(base);
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
        let requests = Arc::new(Mutex::new(Vec::new()));
        let served = requests.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:443").await.unwrap();
                ready_tx.send(()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                let state = Arc::new(Mutex::new(Issued::default()));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") {
                        assert!(head.len() < 16_384);
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
                    let host = header("host")
                        .map(|host| host.trim_end_matches(":443").to_owned())
                        .unwrap_or_default();
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let request = Request {
                        method,
                        host,
                        route,
                        query,
                        body,
                    };
                    let authorization = header("authorization");
                    let (status, location, answer) =
                        answer(&mut state.lock().unwrap(), &request, authorization.as_deref());
                    served.lock().unwrap().push(request);
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let location = location
                        .map(|target| format!("Location: {target}\r\n"))
                        .unwrap_or_default();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\n{location}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // Port 443 is bound. Every process started from here on runs without
        // capabilities, as an ordinary user's would: the Secret Service daemon
        // refuses to start as a root that cannot drop them.
        for capability in 0.. {
            // SAFETY: prctl with integer arguments only.
            if unsafe { libc::prctl(libc::PR_CAPBSET_DROP, capability, 0, 0, 0) } != 0 {
                let error = std::io::Error::last_os_error();
                assert_eq!(error.raw_os_error(), Some(libc::EINVAL), "{error}");
                break;
            }
        }
        Self {
            stop: Some(stop),
            thread: Some(thread),
            pem,
            requests,
        }
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    fn routes(&self, host: &str, route: &str) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|request| request.host == host && request.route == route)
            .collect()
    }

    /// Every Gmail POST: route and JSON body.
    fn posts(&self) -> Vec<(String, Value)> {
        self.requests()
            .into_iter()
            .filter(|request| request.host == "gmail.googleapis.com" && request.method == "POST")
            .map(|request| {
                (
                    request.route,
                    serde_json::from_slice(&request.body).unwrap(),
                )
            })
            .collect()
    }
}

impl Drop for Gmail {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        let _ = self.thread.take().unwrap().join();
    }
}

/// What the fixture issued: the PKCE challenge and redirect of the one
/// consent, and how many access tokens.
#[derive(Default)]
struct Issued {
    consent: Option<(String, String)>,
    exchanged: bool,
    access_tokens: u32,
}

/// The fixture's answer to `request`: status, `Location` and JSON body.
fn answer(
    state: &mut Issued,
    request: &Request,
    authorization: Option<&str>,
) -> (u16, Option<String>, Value) {
    use super::oauth2_refresh::{decode_form, field};
    let refuse = |status, error: &str| (status, None, json!({"error": error}));
    match (
        request.host.as_str(),
        request.method.as_str(),
        request.route.as_str(),
    ) {
        ("accounts.google.com", "GET", "/o/oauth2/auth") => {
            let query = decode_form(request.query.as_bytes());
            let redirect = field(&query, "redirect_uri").unwrap_or_default().to_owned();
            let well_formed = field(&query, "client_id") == Some(CLIENT_ID)
                && field(&query, "response_type") == Some("code")
                && field(&query, "code_challenge_method") == Some("S256")
                && redirect.starts_with("http://127.0.0.1:")
                && state.consent.is_none();
            let (Some(challenge), Some(request_state), true) = (
                field(&query, "code_challenge"),
                field(&query, "state"),
                well_formed,
            ) else {
                return refuse(400, "invalid_request");
            };
            state.consent = Some((challenge.to_owned(), redirect.clone()));
            let location = format!("{redirect}?code={AUTHORIZATION_CODE}&state={request_state}");
            (302, Some(location), json!({}))
        }
        ("oauth2.googleapis.com", "POST", "/token") => {
            use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
            let form = decode_form(&request.body);
            let client = field(&form, "client_id") == Some(CLIENT_ID)
                && field(&form, "client_secret") == Some(CLIENT_SECRET);
            let granted = match field(&form, "grant_type") {
                Some("authorization_code") => {
                    let verifier = field(&form, "code_verifier").unwrap_or_default();
                    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
                    let matches = state.consent.as_ref().is_some_and(|(expected, redirect)| {
                        *expected == challenge
                            && field(&form, "redirect_uri") == Some(redirect.as_str())
                    });
                    let fresh =
                        !state.exchanged && field(&form, "code") == Some(AUTHORIZATION_CODE);
                    state.exchanged |= client && matches && fresh;
                    client && matches && fresh
                }
                Some("refresh_token") => {
                    client && field(&form, "refresh_token") == Some(REFRESH_TOKEN)
                }
                _ => false,
            };
            if !granted {
                return refuse(400, "invalid_grant");
            }
            state.access_tokens += 1;
            let mut body = json!({
                "access_token": format!("{ACCESS_TOKEN}{}", state.access_tokens),
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": format!("openid {COMPOSE_SCOPE} {READONLY_SCOPE}"),
                "id_token": id_token(),
            });
            if field(&form, "grant_type") == Some("authorization_code") {
                body["refresh_token"] = json!(REFRESH_TOKEN);
            }
            (200, None, body)
        }
        ("gmail.googleapis.com", method, route) => {
            let issued = (1..=state.access_tokens)
                .any(|n| authorization == Some(format!("Bearer {ACCESS_TOKEN}{n}").as_str()));
            if !issued {
                return refuse(401, "unauthenticated");
            }
            let draft = json!({"id": DRAFT, "message": {"id": DRAFT_MESSAGE,
                "threadId": "fixture-thread-1", "labelIds": ["DRAFT"]}});
            match (method, route) {
                ("GET", "/gmail/v1/users/me/messages") => (
                    200,
                    None,
                    json!({"messages": [{"id": "fixture-message-1", "threadId": "fixture-thread-1"}],
                           "resultSizeEstimate": 1}),
                ),
                ("POST", "/gmail/v1/users/me/drafts") => (200, None, draft),
                ("GET", "/gmail/v1/users/me/drafts/fixture-draft-1") => (200, None, draft),
                ("POST", "/gmail/v1/users/me/drafts/send") => (
                    200,
                    None,
                    json!({"id": "fixture-sent-message-1", "threadId": "fixture-thread-1",
                           "labelIds": ["SENT"]}),
                ),
                _ => refuse(404, "not_found"),
            }
        }
        _ => refuse(404, "not_found"),
    }
}

/// An unsigned JWT as the token answer's `id_token`, read by the provider
/// directly from the token endpoint over TLS.
fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000007", "iat": 1, "exp": 4_000_000_000_u64})
        ),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}

/// The CLI, its catalog provider beside it, custody, an approval clock, and
/// the configuration that names the Gmail instance and nothing else of it.
struct Journey {
    // Dropped in this order: the owner stops before custody and the clock.
    cli: PathBuf,
    config: PathBuf,
    state: PathBuf,
    root: PathBuf,
    ca: PathBuf,
    client_file: PathBuf,
    _custody: Custody,
    _clock: Vec<clock_fixture::Server>,
}

impl Journey {
    fn new(fixture: &Gmail) -> Self {
        let root = PathBuf::from(WORK);
        filesystem::directory(&root.join("private"), true, true).unwrap();
        let cli = root.join("bin/connectors");
        let ca = root.join("private/ca.pem");
        private(&ca, fixture.pem.as_bytes());
        let custody = Custody::new(&root);
        let count = Arc::new(AtomicUsize::new(0));
        let respond = Arc::new(AtomicBool::new(true));
        let started = Instant::now();
        let clock: Vec<_> = [7u8, 8]
            .into_iter()
            .map(|seed| {
                clock_fixture::Server::shared(
                    count.clone(),
                    respond.clone(),
                    Box::new(move |sequence, request| {
                        let mut time = clock_fixture::Fixture {
                            seed,
                            ..Default::default()
                        };
                        time.midpoint += started.elapsed().as_secs() + sequence as u64;
                        Some(time.reply(request))
                    }),
                )
            })
            .collect();
        let journey = Self {
            cli,
            config: root.join("cli/config.toml"),
            state: root.join("cli/state"),
            client_file: root.join("private/client_secret.json"),
            root,
            ca,
            _custody: custody,
            _clock: clock,
        };
        journey.cli(&["setup", "init"]);
        let clock = clock_fixture::quorum_toml(
            2,
            100,
            &[
                (&journey._clock[0].address, 7),
                (&journey._clock[1].address, 8),
            ],
        );
        private(
            &journey.config,
            format!(
                "format='connectors-local/5'\nowner_uid={}\nsecret_service_socket={}\n{clock}\n[connectors.gmail]\nenabled = true\n\n[connectors.gmail.instances.private]\n",
                filesystem::uid(),
                serde_json::to_string(journey._custody.socket.to_str().unwrap()).unwrap(),
            )
            .as_bytes(),
        );
        // The shape Google's console downloads for a Desktop app client.
        private(
            &journey.client_file,
            &serde_json::to_vec(&json!({"installed": {
                "client_id": CLIENT_ID,
                "project_id": "fixture-project",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": CLIENT_SECRET,
                "redirect_uris": ["http://localhost"],
            }}))
            .unwrap(),
        );
        journey
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(&self.cli);
        command
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(&self.config)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null());
        command
    }

    fn cli_output(&self, args: &[&str]) -> Output {
        let output = self.command(args).output().unwrap();
        assert!(!carries_material(&output.stdout) && !carries_material(&output.stderr));
        output
    }

    #[track_caller]
    fn cli(&self, args: &[&str]) -> Value {
        success(self.cli_output(args))
    }

    /// `args` with the debug-build consent hook following the authorize URL
    /// where a person would open it in a browser.
    fn follow(&self, args: &[&str]) -> Output {
        let output = self
            .command(args)
            .env("CONNECTORS_TEST_OAUTH_FOLLOW", &self.ca)
            .output()
            .unwrap();
        assert!(!carries_material(&output.stdout) && !carries_material(&output.stderr));
        output
    }

    /// `operations invoke` of `operation` with `input` on `connection`, under
    /// the description the CLI reports for it.
    fn invoke(&self, connection: &str, operation: &str, input: Value, extra: &[&str]) -> Output {
        let target = self.target(connection, operation, &input);
        let target: Vec<&str> = target.iter().map(String::as_str).collect();
        self.cli_output(&[&["operations", "invoke"][..], &target, extra].concat())
    }

    fn target(&self, connection: &str, operation: &str, input: &Value) -> Vec<String> {
        let description = self.cli(
            &[
                &["operations", "describe"][..],
                SELECTION,
                &["--operation", operation],
            ]
            .concat(),
        );
        let mut target: Vec<String> = SELECTION.iter().map(|arg| (*arg).to_owned()).collect();
        target.extend(
            [
                "--connection",
                connection,
                "--operation",
                operation,
                "--schema",
                description["schema"].as_str().unwrap(),
                "--revision",
                description["revision"].as_str().unwrap(),
                "--input-json",
                &input.to_string(),
            ]
            .map(str::to_owned),
        );
        target
    }

    /// Prepare, approve and invoke one write with its proof.
    fn approved(&self, connection: &str, operation: &str, input: Value, key: &str) -> Value {
        let target = self.target(connection, operation, &input);
        let target: Vec<&str> = target.iter().map(String::as_str).collect();
        let prepared = self.cli(&[&["approvals", "prepare"][..], &target].concat());
        let proof = self.root.join(format!("private/{key}.proof.json"));
        self.cli(
            &[
                &["approvals", "issue"][..],
                &target,
                &[
                    "--approve-subject",
                    prepared["preparation"]["subject_sha256"].as_str().unwrap(),
                    "--proof-output",
                    proof.to_str().unwrap(),
                ],
            ]
            .concat(),
        );
        success(self.invoke(
            connection,
            operation,
            input,
            &[
                "--idempotency-key",
                key,
                "--approval-file",
                proof.to_str().unwrap(),
            ],
        ))
    }
}

impl Drop for Journey {
    fn drop(&mut self) {
        let paths = connectors_host::local::config::Paths {
            config: self.config.clone(),
            state: self.state.clone(),
        };
        if let Ok(client) = connectors_host::local::owner::Client::connect(&paths, false) {
            let host = client.host_incarnation.clone();
            let _ = client.shutdown(&host);
            let until = Instant::now() + Duration::from_secs(5);
            while self.state.join("owner.sock").exists() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
