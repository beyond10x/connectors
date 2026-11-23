//! story:local-invoke-admits-a-keyed-write-under-a-standing-grant: a Slack bot
//! post under a standing grant, through the production CLI, owner, catalog
//! child, a local TLS Slack fixture, the `roughtime-quorum/1` clock and the
//! disposable Secret Service. The operator signs one grant with `approvals
//! grant-standing`; `invoke slack/bot chat.postMessage --input-stdin
//! --idempotency-key <key>`, with no `--approval-file`, then posts into the
//! granted channel and is refused outside the grant before any provider
//! request (contracts/cli/v1alpha1/semantics.md, "Standing grants"). The
//! fixture token is fictional and only ever compared, never printed.
use super::cli_journey::{Cli, Custody, success};
use super::*;
use std::io::Write;
use std::process::{Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Instant;

const TOKEN: &str = "fixture-slack-standing-bot-token";
const CHANNEL: &str = "C0FIXTURE01";
const OTHER_CHANNEL: &str = "C0FIXTURE02";
const POSTED_TS: &str = "1780000700.000700";
const SELECTION: &[&str] = &["--connector", "slack", "--instance", "bot"];

/// Slack's Web API on loopback: `auth.test` and `chat.postMessage` to the
/// fixture bot token, `ok: false` to anything else. A copy of
/// `slack_approval`'s fixture with the count of all requests and two
/// switches, each answering the next request of its method once.
struct Slack {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    /// Method, route and JSON body (`null` when none) of every request.
    requests: Arc<Mutex<Vec<(String, String, Value)>>>,
    /// The next `auth.test` answers `ok: false`.
    refuse_auth: Arc<AtomicBool>,
    /// The next `chat.postMessage` answers HTTP 500.
    fail_post: Arc<AtomicBool>,
}

impl Slack {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
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
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
        let refuse_auth = Arc::new(AtomicBool::new(false));
        let fail_post = Arc::new(AtomicBool::new(false));
        let (refusing, failing) = (refuse_auth.clone(), fail_post.clone());
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
                    let route = words
                        .next()
                        .unwrap_or_default()
                        .split('?')
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    let bot = header("authorization").as_deref() == Some(&format!("Bearer {TOKEN}"));
                    let switched = match route.as_str() {
                        "/api/auth.test" => refusing.swap(false, Ordering::SeqCst),
                        "/api/chat.postMessage" => failing.swap(false, Ordering::SeqCst),
                        _ => false,
                    };
                    let (status, answer) = match (bot, method.as_str(), route.as_str()) {
                        (true, _, "/api/auth.test") if switched => {
                            ("200 fixture", json!({"ok": false, "error": "fatal_error"}))
                        }
                        (true, "POST", "/api/chat.postMessage") if switched => {
                            ("500 fixture", json!({"ok": false, "error": "internal_error"}))
                        }
                        (true, _, "/api/auth.test") => ("200 fixture", json!({
                            "ok": true, "url": "https://fixture.example.test/",
                            "team": "Fixture Team", "user": "fixture-bot",
                            "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01",
                            "bot_id": "B0FIXTURE01", "is_enterprise_install": false})),
                        (true, "POST", "/api/chat.postMessage") if body["channel"] == CHANNEL => {
                            ("200 fixture", json!({"ok": true, "channel": CHANNEL, "ts": POSTED_TS,
                                "message": {"type": "message", "user": "U0FIXTURE01",
                                    "bot_id": "B0FIXTURE01", "text": body["text"], "ts": POSTED_TS}}))
                        }
                        (true, ..) => ("200 fixture", json!({"ok": false, "error": "channel_not_found"})),
                        (false, ..) => ("200 fixture", json!({"ok": false, "error": "invalid_auth"})),
                    };
                    observed.lock().unwrap().push((method, route, body));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config = directory.join("slack-bot.json");
        // The guide's bot configuration (docs/catalog-slack.md), at the fixture.
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-slack",
                "provider": "slack",
                "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
                "api_base": format!("https://localhost:{}/api", address.port()),
                "ca_file": ca,
                "auth": {
                    "profile": "slack.bot",
                    "header": "Authorization",
                    "bearer": true,
                    "label": "Slack bot token",
                    "identity": {"path": "auth.test", "kind": "slack.user", "subject_pointer": "/user_id"},
                    // The one member the guide's configuration does not
                    // hold: the connection's validation evidence lasts 60 s
                    // without it, and a send after that is `not_granted`,
                    // whatever the grant says. The journey connects once and
                    // must not depend on how fast the machine runs it.
                    "evidence_lifetime_ms": 300_000
                },
                "operations_file": repository.join("providers/slack/operations.json").canonicalize().unwrap(),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            config,
            requests,
            refuse_auth,
            fail_post,
        }
    }

    /// The configuration revision the catalog child prints for its bootstrap.
    fn configuration_revision(&self) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(output.status.success(), "bootstrap inspection failed");
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        bootstrap.configuration_revision
    }

    /// Every `chat.postMessage` the fixture received, as its JSON body.
    fn posts(&self) -> Vec<Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(method, route, _)| method == "POST" && route == "/api/chat.postMessage")
            .map(|(.., body)| body.clone())
            .collect()
    }

    /// How many requests the fixture received, `auth.test` included.
    fn requests(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Drop for Slack {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        let _ = self.thread.take().unwrap().join();
    }
}

/// Two answering quorum sources whose synthetic time advances across
/// exchanges, as `slack_approval`'s do, plus `offset` seconds the test adds to
/// every answer.
struct Time {
    sources: Vec<clock_fixture::Server>,
    offset: Arc<AtomicU64>,
    started: Instant,
}

impl Time {
    /// The fixture clock's midpoint now, in Unix milliseconds, before the
    /// offset: what the next exchange answers, give or take the exchanges
    /// between.
    fn now_ms(&self) -> i64 {
        let seconds = clock_fixture::Fixture::default().midpoint
            + self.started.elapsed().as_secs()
            + self.sources[0].requests.load(Ordering::SeqCst) as u64;
        i64::try_from(seconds * 1000).unwrap()
    }
}

fn clock() -> Time {
    let count = Arc::new(AtomicUsize::new(0));
    let respond = Arc::new(AtomicBool::new(true));
    let offset = Arc::new(AtomicU64::new(0));
    let started = Instant::now();
    let sources = [7u8, 8]
        .into_iter()
        .map(|seed| {
            let offset = offset.clone();
            clock_fixture::Server::shared(
                count.clone(),
                respond.clone(),
                Box::new(move |sequence, request| {
                    let mut time = clock_fixture::Fixture {
                        seed,
                        ..Default::default()
                    };
                    time.midpoint += started.elapsed().as_secs()
                        + sequence as u64
                        + offset.load(Ordering::SeqCst);
                    Some(time.reply(request))
                }),
            )
        })
        .collect();
    Time {
        sources,
        offset,
        started,
    }
}

fn carries_token(output: &Output) -> bool {
    [&output.stdout, &output.stderr].iter().any(|bytes| {
        bytes
            .windows(TOKEN.len())
            .any(|window| window == TOKEN.as_bytes())
    })
}

/// The refusal's failure data, after checking that `output` is one: exit 1,
/// nothing on stdout.
#[track_caller]
fn refused(output: Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

/// The entity of one write attempt in the local metadata authority. The
/// store's file holds the authority's recorded entities and no table named
/// `mutation_attempts`, so the attempts are counted as the other journeys
/// read them: the subjects of this entity in a complete snapshot
/// (`recorded_state::Reader`).
const ATTEMPT: &str = "connectors.mutations.AttemptRecord";

/// Whether any object in `value` has a member named `name`.
fn names(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(members) => {
            members.contains_key(name) || members.values().any(|member| names(member, name))
        }
        Value::Array(elements) => elements.iter().any(|element| names(element, name)),
        _ => false,
    }
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_slack_bot_post_is_admitted_under_a_standing_grant_and_refused_outside_it() {
    let slack = Slack::new();
    let root = slack.root.path();
    let custody = Custody::new(root);
    let time = clock();
    let cli = Cli::new(root);
    let run = |args: &[&str]| {
        // The two command words, then the instance selectors.
        let output = cli.run(&[&args[..2], SELECTION, &args[2..]].concat());
        assert!(!carries_token(&output));
        output
    };
    // The outcome's command: no `--approval-file`, no pins, the input on
    // standard input.
    let send_with = |key: &str, input: &Value, extra: &[&str]| {
        let mut process = Command::new(std::env::var_os("CONNECTORS_TEST_CLI").unwrap())
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_PARENT",
                std::process::id().to_string(),
            )
            .args(["--output", "json", "--config"])
            .arg(&cli.paths.config)
            .arg("--state-dir")
            .arg(&cli.paths.state)
            .args([
                "invoke",
                "slack/bot",
                "chat.postMessage",
                "--input-stdin",
                "--idempotency-key",
                key,
            ])
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(!carries_token(&output));
        output
    };
    let send = |key: &str, input: &Value| send_with(key, input, &[]);
    let post = |text: &str| json!({"body": {"channel": CHANNEL, "text": text}});
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .canonicalize()
        .unwrap();
    let private_dir = root.join("private");
    let grants = private_dir.join("slack-bot.grants");
    success(cli.run(&["setup", "init"]));
    // The guide's bot instance (docs/catalog-slack.md), on private protocol
    // two, with the approval clock and the instance's grant file.
    private(
        &cli.paths.config,
        format!(
            "format='connectors-local/5'\nowner_uid={}\nsecret_service_socket={}\n{}\n[connectors.slack]\nenabled=true\n\n[connectors.slack.instances.bot]\nalias='slack'\ninstance_id='fixture-slack'\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\nstartup='on-demand'\nrestart='never'\nstanding_grants={}\n\n[connectors.slack.instances.bot.permissions]\nprofiles=['slack.bot']\noperations=['auth.test','chat.postMessage']\n\n[connectors.slack.instances.bot.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            clock_fixture::quorum_toml(
                2,
                100,
                &[(&time.sources[0].address, 7), (&time.sources[1].address, 8)]
            ),
            q(&slack.configuration_revision()),
            q(grants.to_str().unwrap()),
            q(binary.to_str().unwrap()),
            q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),
            serde_json::to_string(&["--local-config", slack.config.to_str().unwrap()]).unwrap(),
        )
        .as_bytes(),
    );

    let credential = private_dir.join("credential.json");
    private(
        &credential,
        &serde_json::to_vec(&json!({"token": TOKEN})).unwrap(),
    );
    let connected = success(run(&[
        "connections",
        "connect",
        "--profile",
        "slack.bot",
        "--credential-file",
        credential.to_str().unwrap(),
    ]));
    fs::remove_file(&credential).unwrap();
    assert_eq!(connected["connection"]["summary"]["state"], "ready");
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();

    // The prerequisites of every keyed write: an approval key and a published
    // policy that lists the operation. The grant is not in that document.
    success(run(&["approvals", "key-init"]));
    let policy = private_dir.join("policy.json");
    private(&policy, br#"{"operations":["chat.postMessage"]}"#);
    success(run(&[
        "approvals",
        "policy-set",
        "--input-file",
        policy.to_str().unwrap(),
    ]));

    // The attempts recorded so far, by one reader opened at the first count.
    let recorded = std::cell::OnceCell::new();
    let attempts = || {
        recorded
            .get_or_init(|| recorded_state::Reader::open(&cli.paths.state))
            .snapshot()
            .subjects(ATTEMPT)
            .len()
    };
    // What a refusal before the attempt leaves: no request, no attempt.
    let quiet = |step: &str, refusal: &dyn Fn() -> Value| {
        let before = (slack.requests(), attempts());
        let data = refusal();
        assert_eq!((slack.requests(), attempts()), before, "{step}: {data}");
        assert_eq!(
            data["mutation"]["classification"], "not_attempted",
            "{step}: {data}"
        );
        assert!(!names(&data, "standing"), "{step}: {data}");
        assert!(!names(&data, "axis"), "{step}: {data}");
        data
    };
    let grant = |not_after_unix_ms: i64, name: &str| {
        let request = private_dir.join(name);
        private(
            &request,
            &serde_json::to_vec(&json!({
                "operation": "chat.postMessage",
                "target": [{"pointer": "/body/channel", "value": CHANNEL}],
                "input_keys": ["/body/text", "/body/thread_ts"],
                "not_after_unix_ms": not_after_unix_ms,
                "max_per_hour": 3
            }))
            .unwrap(),
        );
        let granted = success(run(&[
            "approvals",
            "grant-standing",
            "--input-file",
            request.to_str().unwrap(),
        ]));
        assert_eq!(granted["path"], grants.to_str().unwrap(), "{granted}");
        granted["grant_id"].as_str().unwrap().to_owned()
    };
    let listed = || success(run(&["approvals", "list-standing"]))["grants"].clone();
    // A replay of `key` with `input`: the original answer and its grant.
    let replays = |step: &str, key: &str, input: &Value, grant_id: &str| {
        let before = slack.requests();
        let replayed = success(send(key, input));
        assert_eq!(
            replayed["mutation"]["classification"], "applied",
            "{step}: {replayed}"
        );
        assert_eq!(replayed["mutation"]["replayed"], true, "{step}: {replayed}");
        assert_eq!(
            replayed["mutation"]["standing"]["grant_id"], grant_id,
            "{step}: {replayed}"
        );
        assert_eq!(slack.requests(), before, "{step}: a replay made a request");
        replayed
    };

    // a. A keyed send before any grant file exists.
    assert!(!grants.exists());
    let data = quiet("a", &|| {
        refused(send("standing-a", &post("before a grant")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");

    // b. The operator grants once, and sees the grant.
    let not_after =
        i64::try_from(clock_fixture::Fixture::default().midpoint * 1000).unwrap() + 3_600_000;
    let first = grant(not_after, "grant-1.json");
    assert_eq!(
        fs::metadata(&grants).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let held = listed();
    assert_eq!(held.as_array().unwrap().len(), 1, "{held}");
    assert_eq!(held[0]["grant_id"], first.as_str(), "{held}");

    // c. A keyed send to the granted channel is admitted and names the grant.
    let original = post("fixture standing post");
    let before = attempts();
    let invoked = success(send("standing-c", &original));
    assert_eq!(
        invoked["mutation"]["classification"], "applied",
        "{invoked}"
    );
    assert_eq!(invoked["mutation"]["replayed"], false, "{invoked}");
    assert_eq!(
        invoked["mutation"]["standing"]["grant_id"],
        first.as_str(),
        "{invoked}"
    );
    assert_eq!(invoked["result"]["body"]["ts"], POSTED_TS, "{invoked}");
    assert_eq!(
        slack.posts(),
        [json!({"channel": CHANNEL, "text": "fixture standing post"})]
    );
    // One attempt for the one write; the refusal of step a left none.
    assert_eq!((before, attempts()), (0, 1));

    // d. The same key and input: the same answer, no second post.
    let replayed = replays("d", "standing-c", &original, &first);
    assert_eq!(replayed["result"], invoked["result"]);
    assert_eq!(
        replayed["mutation"]["attempt"],
        invoked["mutation"]["attempt"]
    );
    assert_eq!(slack.posts().len(), 1);

    // e. The same key with another text.
    let before = slack.requests();
    let data = refused(send("standing-c", &post("another text")));
    assert_eq!(data["code"], "idempotency_conflict", "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(slack.requests(), before);

    // f. Outside the grant: another channel, then a member the grant's
    // `input_keys` does not list. Neither names what was outside.
    let elsewhere = json!({"body": {"channel": OTHER_CHANNEL, "text": "outside"}});
    let wider = json!({"body": {"channel": CHANNEL, "text": "outside", "mrkdwn": false}});
    for (step, key, input) in [
        ("f channel", "standing-f1", &elsewhere),
        ("f mrkdwn", "standing-f2", &wider),
    ] {
        let data = quiet(step, &|| refused(send(key, input)));
        assert_eq!(data["code"], "approval_required", "{step}: {data}");
    }

    // g. A proof with one changed byte is refused as a proof: it does not
    // fall back to the grant that covers the same input.
    let description = success(run(&[
        "operations",
        "describe",
        "--operation",
        "chat.postMessage",
    ]));
    let tampered_input = post("under a tampered proof");
    let input_text = tampered_input.to_string();
    let target = [
        "--connection",
        &connection,
        "--operation",
        "chat.postMessage",
        "--schema",
        description["schema"].as_str().unwrap(),
        "--revision",
        description["revision"].as_str().unwrap(),
        "--input-json",
        &input_text,
    ];
    let prepared = success(run(&[&["approvals", "prepare"][..], &target].concat()));
    let proof = private_dir.join("proof.json");
    success(run(&[
        &["approvals", "issue"][..],
        &target,
        &[
            "--approve-subject",
            prepared["preparation"]["subject_sha256"].as_str().unwrap(),
            "--proof-output",
            proof.to_str().unwrap(),
        ],
    ]
    .concat()));
    let mut bytes = fs::read(&proof).unwrap();
    // The signature is the last part of the compact evidence, which ends the
    // document: change one of its characters to another of the alphabet.
    let at = bytes.len() - 20;
    assert!(bytes[at].is_ascii_alphanumeric() || bytes[at] == b'-' || bytes[at] == b'_');
    bytes[at] = if bytes[at] == b'A' { b'B' } else { b'A' };
    let tampered = private_dir.join("proof-tampered.json");
    private(&tampered, &bytes);
    let posts = slack.posts().len();
    let data = refused(send_with(
        "standing-g",
        &tampered_input,
        &["--approval-file", tampered.to_str().unwrap()],
    ));
    assert_eq!(data["code"], "approval_refused", "{data}");
    assert!(!names(&data, "standing"), "{data}");
    assert_eq!(slack.posts().len(), posts);

    // h. A preflight that fails gives the use back.
    slack.refuse_auth.store(true, Ordering::SeqCst);
    let data = refused(send("standing-h", &post("while auth.test refuses")));
    assert!(
        !slack.refuse_auth.load(Ordering::SeqCst),
        "auth.test was not read"
    );
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert!(!names(&data, "standing"), "{data}");
    assert_eq!(slack.posts().len(), posts);

    // i. A write that was sent keeps its use, whatever the provider answered.
    slack.fail_post.store(true, Ordering::SeqCst);
    let data = refused(send("standing-i", &post("while Slack fails")));
    assert!(
        !slack.fail_post.load(Ordering::SeqCst),
        "the post was not sent"
    );
    assert_ne!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(
        data["mutation"]["standing"]["grant_id"],
        first.as_str(),
        "{data}"
    );
    assert_eq!(slack.posts().len(), posts + 1);

    // j. The third use of the hour (c, i and this one), then the cap. Had g
    // or h kept a use, this send would be at the cap.
    let third = success(send("standing-j", &post("the third of the hour")));
    assert_eq!(third["mutation"]["classification"], "applied", "{third}");
    assert_eq!(
        third["mutation"]["standing"]["grant_id"],
        first.as_str(),
        "{third}"
    );
    let data = quiet("j", &|| {
        refused(send("standing-j-capped", &post("over the cap")))
    });
    assert_eq!(data["code"], "rate_limited", "{data}");
    let retry = data["retry_after_seconds"].as_u64().unwrap();
    assert!((1..=3600).contains(&retry), "{data}");
    replays("j", "standing-c", &original, &first);

    // k. Revoked: nothing new is admitted, and the settled key still replays.
    let revoked = success(run(&["approvals", "revoke-standing", &first]));
    assert_eq!(revoked["grant_id"], first.as_str(), "{revoked}");
    assert_eq!(listed(), json!([]));
    let data = quiet("k", &|| {
        refused(send("standing-k", &post("after the revocation")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");
    replays("k", "standing-c", &original, &first);

    // l. A second grant, 600 s long, admits under its own id; an hour later
    // it is expired, and both settled keys still replay with their grants.
    let second = grant(time.now_ms() + 600_000, "grant-2.json");
    assert_ne!(second, first);
    let later = post("under the second grant");
    let invoked = success(send("standing-l", &later));
    assert_eq!(
        invoked["mutation"]["classification"], "applied",
        "{invoked}"
    );
    assert_eq!(
        invoked["mutation"]["standing"]["grant_id"],
        second.as_str(),
        "{invoked}"
    );
    time.offset.store(3600, Ordering::SeqCst);
    let data = quiet("l", &|| {
        refused(send("standing-l-expired", &post("after the expiry")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");
    replays("l", "standing-l", &later, &second);
    replays("l", "standing-c", &original, &first);

    // An approval file that holds no byte is answered as no approval file is,
    // by the CLI: the gate's reply, and it takes no grant either. No owner is
    // started for it: the one the sends above reached is stopped first (the
    // control: `owner-stop` sees an owner that runs), and none runs
    // afterwards. This needs the registered connection, the published policy
    // and the key this journey has: without them the keyed lookup refuses
    // the call before the approval file is read.
    let stopped = success(cli.run(&["setup", "owner-stop"]));
    assert_eq!(stopped["disposition"], "stopped", "{stopped}");
    let empty = private_dir.join("proof-empty.json");
    private(&empty, b"");
    let before = (slack.requests(), attempts());
    let data = refused(send_with(
        "standing-empty-proof",
        &post("under an empty proof"),
        &["--approval-file", empty.to_str().unwrap()],
    ));
    assert_eq!(
        data,
        json!({"kind":"operational","code":"approval_required","stage":"admission","next_action":"none",
            "approval_steps":["approvals prepare","approvals clock-check","approvals issue"]})
    );
    assert_eq!((slack.requests(), attempts()), before);
    let none = success(cli.run(&["setup", "owner-stop"]));
    assert_eq!(none["disposition"], "no_owner", "{none}");
    drop(recorded);
    drop(cli);
    drop(custody);
}
