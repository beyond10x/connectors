//! story:local-invoke-admits-a-keyed-write-under-a-standing-grant: a Slack bot
//! post under a standing grant, through the production CLI, owner, catalog
//! child, a local TLS Slack fixture, the `roughtime-quorum/1` clock and the
//! disposable Secret Service. The operator signs one grant with `approvals
//! grant-standing`; `invoke slack/bot chat.postMessage --input-stdin
//! --idempotency-key <key>`, with no `--approval-file`, then posts into the
//! granted channel and is refused outside the grant before any provider
//! request (contracts/cli/v1alpha1/semantics.md, "Standing grants"). The
//! fixture token is fictional and only ever compared, never printed.
//!
//! The story's one journey (steps a to l and the empty approval file) is
//! five tests here, each on a fixture of its own, so that none nears 60 s
//! on a loaded host: the connection's validation evidence lasts 60 s from
//! `connections connect`, and a send after that is `not_granted`, whatever
//! the grant says.
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
                    "identity": {"path": "auth.test", "kind": "slack.user", "subject_pointer": "/user_id"}
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

/// An in-grant post: the granted channel and a text.
fn post(text: &str) -> Value {
    json!({"body": {"channel": CHANNEL, "text": text}})
}

/// One bot instance ready for a keyed write: the Slack fixture, the
/// disposable Secret Service, the clock, the CLI on the guide's instance with
/// a grant file configured, a connected bot token, an approval key and a
/// published policy that lists the operation. No grant is signed yet. Each
/// journey below builds its own, so none runs longer than its own steps take.
struct Bot {
    // Dropped in this order: the reader, the CLI, the Secret Service, the
    // clock, then the Slack fixture with the root directory.
    /// The attempts' reader, opened at the first count.
    recorded: std::cell::OnceCell<recorded_state::Reader>,
    cli: Cli,
    _custody: Custody,
    time: Time,
    slack: Slack,
    private_dir: PathBuf,
    /// The instance's grant file (`standing_grants`).
    grants: PathBuf,
    connection: String,
    begun: Instant,
}

impl Bot {
    fn new() -> Self {
        let begun = Instant::now();
        let slack = Slack::new();
        let root = slack.root.path();
        let custody = Custody::new(root);
        let time = clock();
        let cli = Cli::new(root);
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
        let mut bot = Self {
            recorded: std::cell::OnceCell::new(),
            cli,
            _custody: custody,
            time,
            slack,
            private_dir,
            grants,
            connection: String::new(),
            begun,
        };

        let credential = bot.private_dir.join("credential.json");
        private(
            &credential,
            &serde_json::to_vec(&json!({"token": TOKEN})).unwrap(),
        );
        let connected = success(bot.run(&[
            "connections",
            "connect",
            "--profile",
            "slack.bot",
            "--credential-file",
            credential.to_str().unwrap(),
        ]));
        fs::remove_file(&credential).unwrap();
        assert_eq!(connected["connection"]["summary"]["state"], "ready");
        bot.connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();

        // The prerequisites of every keyed write: an approval key and a
        // published policy that lists the operation. The grant is not in that
        // document.
        success(bot.run(&["approvals", "key-init"]));
        let policy = bot.private_dir.join("policy.json");
        private(&policy, br#"{"operations":["chat.postMessage"]}"#);
        success(bot.run(&[
            "approvals",
            "policy-set",
            "--input-file",
            policy.to_str().unwrap(),
        ]));
        bot.stamp("fixture");
        bot
    }

    /// One line per step on standard error, with the seconds since the
    /// journey began: a run that nears its 60 s shows where the time went.
    fn stamp(&self, step: &str) {
        eprintln!(
            "slack_standing: {step} done at {:.1} s",
            self.begun.elapsed().as_secs_f64()
        );
    }

    /// The two command words, then the instance selectors, then the rest.
    fn run(&self, args: &[&str]) -> Output {
        let output = self.cli.run(&[&args[..2], SELECTION, &args[2..]].concat());
        assert!(!carries_token(&output));
        output
    }

    /// The outcome's command: no pins, the input on standard input, and no
    /// `--approval-file` unless `extra` names one.
    fn send_with(&self, key: &str, input: &Value, extra: &[&str]) -> Output {
        let mut process = Command::new(std::env::var_os("CONNECTORS_TEST_CLI").unwrap())
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_PARENT",
                std::process::id().to_string(),
            )
            .args(["--output", "json", "--config"])
            .arg(&self.cli.paths.config)
            .arg("--state-dir")
            .arg(&self.cli.paths.state)
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
    }

    fn send(&self, key: &str, input: &Value) -> Output {
        self.send_with(key, input, &[])
    }

    /// The attempts recorded so far.
    fn attempts(&self) -> usize {
        self.recorded
            .get_or_init(|| recorded_state::Reader::open(&self.cli.paths.state))
            .snapshot()
            .subjects(ATTEMPT)
            .len()
    }

    /// What a refusal before the attempt leaves: no request, no attempt.
    #[track_caller]
    fn quiet(&self, step: &str, refusal: &dyn Fn() -> Value) -> Value {
        let before = (self.slack.requests(), self.attempts());
        let data = refusal();
        assert_eq!(
            (self.slack.requests(), self.attempts()),
            before,
            "{step}: {data}"
        );
        assert_eq!(
            data["mutation"]["classification"], "not_attempted",
            "{step}: {data}"
        );
        assert!(!names(&data, "standing"), "{step}: {data}");
        assert!(!names(&data, "axis"), "{step}: {data}");
        data
    }

    /// `approvals grant-standing` for the granted channel, `max_per_hour`
    /// writes an hour, the text and the thread open: the new grant's id.
    #[track_caller]
    fn grant(&self, not_after_unix_ms: i64, max_per_hour: u64, name: &str) -> String {
        self.grant_request(
            &json!({
                "operation": "chat.postMessage",
                "target": [{"pointer": "/body/channel", "value": CHANNEL}],
                "input_keys": ["/body/text", "/body/thread_ts"],
                "not_after_unix_ms": not_after_unix_ms,
                "max_per_hour": max_per_hour
            }),
            name,
        )
    }

    /// `approvals grant-standing` for the document `request`, written to the
    /// private file `name`: the new grant's id.
    #[track_caller]
    fn grant_request(&self, request: &Value, name: &str) -> String {
        let document = request;
        let request = self.private_dir.join(name);
        private(&request, &serde_json::to_vec(document).unwrap());
        let granted = success(self.run(&[
            "approvals",
            "grant-standing",
            "--input-file",
            request.to_str().unwrap(),
        ]));
        assert_eq!(granted["path"], self.grants.to_str().unwrap(), "{granted}");
        granted["grant_id"].as_str().unwrap().to_owned()
    }

    /// 3600 s after the fixture clock's midpoint, in milliseconds.
    fn in_an_hour() -> i64 {
        i64::try_from(clock_fixture::Fixture::default().midpoint * 1000).unwrap() + 3_600_000
    }

    /// The first grant of a journey: it ends 3600 s after the fixture clock's
    /// midpoint.
    #[track_caller]
    fn grant_for_an_hour(&self, max_per_hour: u64) -> String {
        self.grant(Self::in_an_hour(), max_per_hour, "grant-1.json")
    }

    #[track_caller]
    fn listed(&self) -> Value {
        success(self.run(&["approvals", "list-standing"]))["grants"].clone()
    }

    /// A keyed send that is admitted under `grant_id` and applied.
    #[track_caller]
    fn admitted(&self, step: &str, key: &str, input: &Value, grant_id: &str) -> Value {
        let invoked = success(self.send(key, input));
        assert_eq!(
            invoked["mutation"]["classification"], "applied",
            "{step}: {invoked}"
        );
        assert_eq!(invoked["mutation"]["replayed"], false, "{step}: {invoked}");
        assert_eq!(
            invoked["mutation"]["standing"]["grant_id"], grant_id,
            "{step}: {invoked}"
        );
        invoked
    }

    /// A replay of `key` with `input`: the original answer and its grant.
    #[track_caller]
    fn replays(&self, step: &str, key: &str, input: &Value, grant_id: &str) -> Value {
        let before = self.slack.requests();
        let replayed = success(self.send(key, input));
        assert_eq!(
            replayed["mutation"]["classification"], "applied",
            "{step}: {replayed}"
        );
        assert_eq!(replayed["mutation"]["replayed"], true, "{step}: {replayed}");
        assert_eq!(
            replayed["mutation"]["standing"]["grant_id"], grant_id,
            "{step}: {replayed}"
        );
        assert_eq!(
            self.slack.requests(),
            before,
            "{step}: a replay made a request"
        );
        replayed
    }
}

/// Steps a to e of the story's journey, then the empty approval file.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_slack_bot_post_is_admitted_under_a_standing_grant_and_replayed_by_its_key() {
    let bot = Bot::new();
    let slack = &bot.slack;

    // a. A keyed send before any grant file exists.
    assert!(!bot.grants.exists());
    let data = bot.quiet("a", &|| {
        refused(bot.send("standing-a", &post("before a grant")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");
    bot.stamp("a");

    // b. The operator grants once, and sees the grant.
    let first = bot.grant_for_an_hour(3);
    assert_eq!(
        fs::metadata(&bot.grants).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let held = bot.listed();
    assert_eq!(held.as_array().unwrap().len(), 1, "{held}");
    assert_eq!(held[0]["grant_id"], first.as_str(), "{held}");
    // The answer shows each target member with the key it was written with,
    // and a pattern member also with `max_value_bytes`, the longest value it
    // is matched against (contracts/cli/v1alpha1/semantics.md, "Standing
    // grants"): 22,055 bytes for `^DEV-[0-9]+$`. An exact member and a value
    // set hold no such key. The second grant covers no send of this journey:
    // none holds `thread_ts`.
    assert_eq!(
        held[0]["target"],
        json!([{"pointer": "/body/channel", "value": CHANNEL}]),
        "{held}"
    );
    let many = bot.grant_request(
        &json!({
            "operation": "chat.postMessage",
            "target": [
                {"pointer": "/body/channel", "value": CHANNEL},
                {"pointer": "/body/text", "pattern": "^DEV-[0-9]+$"},
                {"pointer": "/body/thread_ts", "one_of": [POSTED_TS, "1780000701.000701"]}
            ],
            "not_after_unix_ms": Bot::in_an_hour(),
            "max_per_hour": 3
        }),
        "grant-many.json",
    );
    let held = bot.listed();
    assert_eq!(held.as_array().unwrap().len(), 2, "{held}");
    let target_of = |id: &str| {
        let grant = held
            .as_array()
            .unwrap()
            .iter()
            .find(|grant| grant["grant_id"] == id);
        grant.unwrap_or_else(|| panic!("{id} is not listed: {held}"))["target"].clone()
    };
    assert_eq!(
        target_of(&first),
        json!([{"pointer": "/body/channel", "value": CHANNEL}]),
        "{held}"
    );
    assert_eq!(
        target_of(&many),
        json!([
            {"pointer": "/body/channel", "value": CHANNEL},
            {"pointer": "/body/text", "pattern": "^DEV-[0-9]+$", "max_value_bytes": 22055},
            {"pointer": "/body/thread_ts", "one_of": [POSTED_TS, "1780000701.000701"]}
        ]),
        "{held}"
    );
    bot.stamp("b");

    // c. A keyed send to the granted channel is admitted and names the grant.
    let original = post("fixture standing post");
    let before = bot.attempts();
    let invoked = bot.admitted("c", "standing-c", &original, &first);
    assert_eq!(invoked["result"]["body"]["ts"], POSTED_TS, "{invoked}");
    assert_eq!(
        slack.posts(),
        [json!({"channel": CHANNEL, "text": "fixture standing post"})]
    );
    // One attempt for the one write; the refusal of step a left none.
    assert_eq!((before, bot.attempts()), (0, 1));
    bot.stamp("c");

    // d. The same key and input: the same answer, no second post.
    let replayed = bot.replays("d", "standing-c", &original, &first);
    assert_eq!(replayed["result"], invoked["result"]);
    assert_eq!(
        replayed["mutation"]["attempt"],
        invoked["mutation"]["attempt"]
    );
    assert_eq!(slack.posts().len(), 1);
    bot.stamp("d");

    // e. The same key with another text.
    let before = slack.requests();
    let data = refused(bot.send("standing-c", &post("another text")));
    assert_eq!(data["code"], "idempotency_conflict", "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(slack.requests(), before);
    bot.stamp("e");

    // An approval file that holds no byte is answered as no approval file is,
    // by the CLI: the gate's reply, and it takes no grant either, although
    // the grant of step b covers this input and has two uses left. No owner
    // is started for it: the one the sends above reached is stopped first
    // (the control: `owner-stop` sees an owner that runs), and none runs
    // afterwards. This needs the registered connection, the published policy
    // and the key this journey has: without them the keyed lookup refuses
    // the call before the approval file is read.
    let stopped = success(bot.cli.run(&["setup", "owner-stop"]));
    assert_eq!(stopped["disposition"], "stopped", "{stopped}");
    let empty = bot.private_dir.join("proof-empty.json");
    private(&empty, b"");
    let before = (slack.requests(), bot.attempts());
    let data = refused(bot.send_with(
        "standing-empty-proof",
        &post("under an empty proof"),
        &["--approval-file", empty.to_str().unwrap()],
    ));
    assert_eq!(
        data,
        json!({"kind":"operational","code":"approval_required","stage":"admission","next_action":"none",
            "approval_steps":["approvals prepare","approvals clock-check","approvals issue"]})
    );
    assert_eq!((slack.requests(), bot.attempts()), before);
    let none = success(bot.cli.run(&["setup", "owner-stop"]));
    assert_eq!(none["disposition"], "no_owner", "{none}");
    bot.stamp("empty proof");
}

/// Steps g and h of the story's journey, then the two parts of step j that
/// show them: the grant admits one write an hour here, so the write after g
/// and h is admitted only if neither kept a use, and its key replays while
/// the grant is at its cap.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_refused_proof_and_a_failed_preflight_take_no_use_of_a_standing_grant() {
    let bot = Bot::new();
    let slack = &bot.slack;
    let first = bot.grant_for_an_hour(1);
    bot.stamp("grant");

    // g. A proof with one changed byte is refused as a proof: it does not
    // fall back to the grant that covers the same input.
    let description =
        success(bot.run(&["operations", "describe", "--operation", "chat.postMessage"]));
    let tampered_input = post("under a tampered proof");
    let input_text = tampered_input.to_string();
    let target = [
        "--connection",
        &bot.connection,
        "--operation",
        "chat.postMessage",
        "--schema",
        description["schema"].as_str().unwrap(),
        "--revision",
        description["revision"].as_str().unwrap(),
        "--input-json",
        &input_text,
    ];
    let prepared = success(bot.run(&[&["approvals", "prepare"][..], &target].concat()));
    let proof = bot.private_dir.join("proof.json");
    success(
        bot.run(
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
        ),
    );
    let mut bytes = fs::read(&proof).unwrap();
    // The signature is the last part of the compact evidence, which ends the
    // document: change one of its characters to another of the alphabet.
    let at = bytes.len() - 20;
    assert!(bytes[at].is_ascii_alphanumeric() || bytes[at] == b'-' || bytes[at] == b'_');
    bytes[at] = if bytes[at] == b'A' { b'B' } else { b'A' };
    let tampered = bot.private_dir.join("proof-tampered.json");
    private(&tampered, &bytes);
    let data = refused(bot.send_with(
        "standing-g",
        &tampered_input,
        &["--approval-file", tampered.to_str().unwrap()],
    ));
    assert_eq!(data["code"], "approval_refused", "{data}");
    assert!(!names(&data, "standing"), "{data}");
    assert_eq!(slack.posts().len(), 0);
    bot.stamp("g");

    // h. A preflight that fails gives the use back.
    slack.refuse_auth.store(true, Ordering::SeqCst);
    let data = refused(bot.send("standing-h", &post("while auth.test refuses")));
    assert!(
        !slack.refuse_auth.load(Ordering::SeqCst),
        "auth.test was not read"
    );
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert!(!names(&data, "standing"), "{data}");
    assert_eq!(slack.posts().len(), 0);
    bot.stamp("h");

    // j, its first send: the one use of the hour is still there. Had g or h
    // kept a use, this send would be at the cap.
    let original = post("the one write of the hour");
    bot.admitted("j", "standing-j", &original, &first);
    assert_eq!(slack.posts().len(), 1);
    bot.stamp("j admitted");

    // j, its replay: the grant is at its cap now, and the settled key still
    // replays with the grant id.
    bot.replays("j", "standing-j", &original, &first);
    bot.stamp("j replay");
}

/// Step i of the story's journey, then the refusal of step j that shows it:
/// the grant admits one write an hour here, so the next fresh key is at the
/// cap only if the write Slack failed kept its use.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_write_sent_under_a_standing_grant_keeps_its_use_and_the_next_is_rate_limited() {
    let bot = Bot::new();
    let slack = &bot.slack;
    let first = bot.grant_for_an_hour(1);
    bot.stamp("grant");

    // i. A write that was sent keeps its use, whatever the provider answered.
    slack.fail_post.store(true, Ordering::SeqCst);
    let data = refused(bot.send("standing-i", &post("while Slack fails")));
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
    assert_eq!(slack.posts().len(), 1);
    bot.stamp("i");

    // j, its refusal: the cap.
    let data = bot.quiet("j", &|| {
        refused(bot.send("standing-j-capped", &post("over the cap")))
    });
    assert_eq!(data["code"], "rate_limited", "{data}");
    let retry = data["retry_after_seconds"].as_u64().unwrap();
    assert!((1..=3600).contains(&retry), "{data}");
    bot.stamp("j capped");
}

/// Steps f and k of the story's journey, on a grant and a first admitted
/// write of this journey's own (what steps b and c are to the story's f and
/// k).
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_slack_bot_post_outside_the_standing_grant_or_after_its_revocation_is_refused() {
    let bot = Bot::new();
    let first = bot.grant_for_an_hour(3);
    let original = post("fixture standing post");
    bot.admitted("first", "standing-c", &original, &first);
    bot.stamp("grant and first write");

    // f. Outside the grant: another channel, then a member the grant's
    // `input_keys` does not list. Neither names what was outside.
    let elsewhere = json!({"body": {"channel": OTHER_CHANNEL, "text": "outside"}});
    let wider = json!({"body": {"channel": CHANNEL, "text": "outside", "mrkdwn": false}});
    for (step, key, input) in [
        ("f channel", "standing-f1", &elsewhere),
        ("f mrkdwn", "standing-f2", &wider),
    ] {
        let data = bot.quiet(step, &|| refused(bot.send(key, input)));
        assert_eq!(data["code"], "approval_required", "{step}: {data}");
    }
    bot.stamp("f");

    // k. Revoked: nothing new is admitted, and the settled key still replays.
    let revoked = success(bot.run(&["approvals", "revoke-standing", &first]));
    assert_eq!(revoked["grant_id"], first.as_str(), "{revoked}");
    assert_eq!(bot.listed(), json!([]));
    let data = bot.quiet("k", &|| {
        refused(bot.send("standing-k", &post("after the revocation")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");
    bot.replays("k", "standing-c", &original, &first);
    bot.stamp("k");
}

/// Step l of the story's journey, on a grant and a first admitted write of
/// this journey's own (what steps b and c are to the story's l).
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn an_expired_standing_grant_admits_nothing_new_and_its_writes_still_replay() {
    let bot = Bot::new();
    let first = bot.grant_for_an_hour(3);
    let original = post("fixture standing post");
    bot.admitted("first", "standing-c", &original, &first);
    bot.stamp("grant and first write");

    // The first grant is revoked, as step k leaves it in the story: the
    // second is then the only grant that covers the next write.
    let revoked = success(bot.run(&["approvals", "revoke-standing", &first]));
    assert_eq!(revoked["grant_id"], first.as_str(), "{revoked}");

    // l. A second grant, 600 s long, admits under its own id; an hour later
    // it is expired, and both settled keys still replay with their grants.
    let second = bot.grant(bot.time.now_ms() + 600_000, 3, "grant-2.json");
    assert_ne!(second, first);
    let later = post("under the second grant");
    bot.admitted("l", "standing-l", &later, &second);
    bot.stamp("l admitted");
    bot.time.offset.store(3600, Ordering::SeqCst);
    let data = bot.quiet("l", &|| {
        refused(bot.send("standing-l-expired", &post("after the expiry")))
    });
    assert_eq!(data["code"], "approval_required", "{data}");
    bot.replays("l", "standing-l", &later, &second);
    bot.replays("l", "standing-c", &original, &first);
    bot.stamp("l");
}
