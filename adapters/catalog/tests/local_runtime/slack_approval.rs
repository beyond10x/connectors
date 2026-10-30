//! story:approval-issue-slack-bot-post (issue #180): a Slack bot post under a
//! revision-2 write policy and the `roughtime-quorum/1` clock, through the
//! production CLI, owner, catalog child, a local TLS Slack fixture and the
//! disposable Secret Service. `approvals prepare` names a subject;
//! `approvals issue --approve-subject` with that subject issues its approval,
//! and `invoke slack/bot chat.postMessage --approval-file` posts the message once. An
//! `invalid_input` from `issue` names the argument it refused: here the proof
//! output, the only argument `issue` reads that `prepare` does not. The
//! fixture token is fictional and only ever compared, never printed.
use super::cli_journey::{Cli, Custody, success};
use super::*;
use std::process::{Output, Stdio};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const TOKEN: &str = "fixture-slack-approval-bot-token";
const CHANNEL: &str = "C0FIXTURE01";
const POSTED_TS: &str = "1780000700.000700";
const SELECTION: &[&str] = &["--connector", "slack", "--instance", "bot"];

/// Slack's Web API on loopback: `auth.test` and `chat.postMessage` to the
/// fixture bot token, `ok: false` to anything else.
struct Slack {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    /// Method, route and JSON body (`null` when none) of every request.
    requests: Arc<Mutex<Vec<(String, String, Value)>>>,
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
                    let answer = match (bot, method.as_str(), route.as_str()) {
                        (true, _, "/api/auth.test") => json!({
                            "ok": true, "url": "https://fixture.example.test/",
                            "team": "Fixture Team", "user": "fixture-bot",
                            "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01",
                            "bot_id": "B0FIXTURE01", "is_enterprise_install": false}),
                        (true, "POST", "/api/chat.postMessage") if body["channel"] == CHANNEL => {
                            json!({"ok": true, "channel": CHANNEL, "ts": POSTED_TS,
                                "message": {"type": "message", "user": "U0FIXTURE01",
                                    "bot_id": "B0FIXTURE01", "text": body["text"], "ts": POSTED_TS}})
                        }
                        (true, ..) => json!({"ok": false, "error": "channel_not_found"}),
                        (false, ..) => json!({"ok": false, "error": "invalid_auth"}),
                    };
                    observed.lock().unwrap().push((method, route, body));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 200 fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
}

impl Drop for Slack {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        let _ = self.thread.take().unwrap().join();
    }
}

/// Two answering quorum sources whose synthetic time advances across exchanges.
fn clock() -> Vec<clock_fixture::Server> {
    let count = Arc::new(AtomicUsize::new(0));
    let respond = Arc::new(AtomicBool::new(true));
    let started = Instant::now();
    [7u8, 8]
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
        .collect()
}

fn carries_token(output: &Output) -> bool {
    [&output.stdout, &output.stderr].iter().any(|bytes| {
        bytes
            .windows(TOKEN.len())
            .any(|window| window == TOKEN.as_bytes())
    })
}

/// The refusal's failure data, after checking that `output` is one.
#[track_caller]
fn refused(output: Output) -> Value {
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_slack_bot_post_is_issued_the_approval_its_prepare_named_and_invoked_with_it() {
    let slack = Slack::new();
    let root = slack.root.path();
    let custody = Custody::new(root);
    let sources = clock();
    let cli = Cli::new(root);
    let run = |args: &[&str]| {
        // The two command words, then the instance selectors.
        let output = cli.run(&[&args[..2], SELECTION, &args[2..]].concat());
        assert!(!carries_token(&output));
        output
    };
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .canonicalize()
        .unwrap();
    success(cli.run(&["setup", "init"]));
    // The guide's bot instance (docs/catalog-slack.md, "Configure the two
    // instances"), on private protocol two, with the approval clock.
    private(
        &cli.paths.config,
        format!(
            "format='connectors-local/5'\nowner_uid={}\nsecret_service_socket={}\n{}\n[connectors.slack]\nenabled=true\n\n[connectors.slack.instances.bot]\nalias='slack'\ninstance_id='fixture-slack'\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\nstartup='on-demand'\nrestart='never'\n\n[connectors.slack.instances.bot.permissions]\nprofiles=['slack.bot']\noperations=['auth.test','chat.postMessage']\n\n[connectors.slack.instances.bot.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            clock_fixture::quorum_toml(
                2,
                100,
                &[(&sources[0].address, 7), (&sources[1].address, 8)]
            ),
            q(&slack.configuration_revision()),
            q(binary.to_str().unwrap()),
            q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),
            serde_json::to_string(&["--local-config", slack.config.to_str().unwrap()]).unwrap(),
        )
        .as_bytes(),
    );

    let private_dir = root.join("private");
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

    // A published write policy at revision 2.
    success(run(&["approvals", "key-init"]));
    let policy = private_dir.join("policy.json");
    private(&policy, br#"{"operations":["chat.postMessage"]}"#);
    let first = success(run(&[
        "approvals",
        "policy-set",
        "--input-file",
        policy.to_str().unwrap(),
    ]));
    assert_eq!(first["policy"]["revision"], 1, "{first}");
    let second = success(run(&[
        "approvals",
        "policy-set",
        "--input-file",
        policy.to_str().unwrap(),
        "--expected-revision",
        "1",
    ]));
    assert_eq!(second["policy"]["revision"], 2, "{second}");
    assert_eq!(second["policy"]["operations"], json!(["chat.postMessage"]));

    let description = success(run(&[
        "operations",
        "describe",
        "--operation",
        "chat.postMessage",
    ]));
    let input = json!({"body": {"channel": CHANNEL, "text": "fixture approved post"}}).to_string();
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
        &input,
    ];
    let prepared = success(run(&[&["approvals", "prepare"][..], &target].concat()));
    let subject = prepared["preparation"]["subject_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        prepared["preparation"]["presentation"]["operation"],
        "chat.postMessage"
    );
    let issue_args = |output: &'static str| -> Vec<String> {
        [&["approvals", "issue"][..], SELECTION, &target[..]]
            .concat()
            .into_iter()
            .chain(["--approve-subject", &subject, "--proof-output", output])
            .map(str::to_owned)
            .collect()
    };
    let issue = |output: &str| {
        run(&[
            &["approvals", "issue"][..],
            &target,
            &["--approve-subject", &subject, "--proof-output", output],
        ]
        .concat())
    };

    // A proof output `issue` cannot publish to is `invalid_input` naming it:
    // a relative path, a directory others can enter, and a file that exists.
    let mut refusals = Vec::new();
    let relative = issue_args("proof.json");
    let relative: Vec<&str> = relative.iter().map(String::as_str).collect();
    let output = command_in(&cli, &private_dir, &relative);
    assert!(!carries_token(&output));
    let data = refused(output);
    refusals.push((
        data["code"].clone(),
        data["stage"].clone(),
        data["field"].clone(),
    ));
    assert!(!private_dir.join("proof.json").exists());
    let open = root.join("open");
    fs::create_dir(&open).unwrap();
    fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
    let data = refused(issue(open.join("proof.json").to_str().unwrap()));
    refusals.push((
        data["code"].clone(),
        data["stage"].clone(),
        data["field"].clone(),
    ));
    assert!(!open.join("proof.json").exists());

    // The subject prepare named is issued to a new file in a private directory.
    let proof = private_dir.join("proof-post-1.json");
    let published = success(issue(proof.to_str().unwrap()));
    assert_eq!(published["subject_sha256"], subject.as_str());
    assert_eq!(published["disposition"], "published");
    assert_eq!(
        fs::metadata(&proof).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let bytes = fs::read(&proof).unwrap();
    let data = refused(issue(proof.to_str().unwrap()));
    refusals.push((
        data["code"].clone(),
        data["stage"].clone(),
        data["field"].clone(),
    ));
    assert!(fs::read(&proof).unwrap() == bytes, "the proof was replaced");
    let proof_output = (
        json!("invalid_input"),
        json!("arguments"),
        json!("proof_output"),
    );
    assert_eq!(
        refusals,
        [proof_output.clone(), proof_output.clone(), proof_output]
    );
    assert!(slack.posts().is_empty(), "issuing posted a message");

    // The write runs once with that proof, through `invoke <connector>/<instance>
    // <operation>` (story:connectors-do-short-invocation): the prepared
    // target's pins without its `--operation`, which is positional.
    let pins: Vec<&str> = target
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 2 | 3))
        .map(|(_, arg)| *arg)
        .collect();
    let invoked = success(run(&[
        &["invoke", "slack/bot", "chat.postMessage"][..],
        &pins,
        &[
            "--approval-file",
            proof.to_str().unwrap(),
            "--idempotency-key",
            "slack-approved-post-1",
        ],
    ]
    .concat()));
    assert_eq!(
        invoked["mutation"]["classification"], "applied",
        "{invoked}"
    );
    assert_eq!(invoked["result"]["status"], 200, "{invoked}");
    assert_eq!(invoked["result"]["body"]["ts"], POSTED_TS, "{invoked}");
    assert_eq!(
        slack.posts(),
        [json!({"channel": CHANNEL, "text": "fixture approved post"})]
    );
    drop(cli);
    drop(custody);
}

/// The production CLI the journey runs, with `cwd` as its working directory:
/// a relative proof output would land there, never in the repository.
fn command_in(cli: &Cli, cwd: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var_os("CONNECTORS_TEST_CLI").unwrap())
        .env_clear()
        .env(
            "CONNECTORS_TEST_OWNER_PARENT",
            std::process::id().to_string(),
        )
        .current_dir(cwd)
        .args(["--output", "json", "--config"])
        .arg(&cli.paths.config)
        .arg("--state-dir")
        .arg(&cli.paths.state)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}
