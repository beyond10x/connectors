//! Adversary pass 1 of story:chatgpt-subscription-connector (unit U7).
//!
//! The guide gives the person about 4 minutes to enter the device code
//! (`website/docs/connectors/chatgpt.md`, "Connect"): the grant's 240 s window
//! (`crates/connectors-host/src/local/oauth2_grant.rs`, `grant_deadline`),
//! while the ChatGPT dialect accepts a polling interval of up to 600 s
//! (`adapters/catalog/src/chatgpt.rs`, `MAX_INTERVAL`). The two device cases
//! drive the production CLI against that promise; pass 1's case held the CLI
//! to the 15 minutes the guide promised then. The other two cases pin the
//! scheme and pointer guards of an `id_token` claim pointer
//! (`adapters/catalog/src/local.rs`, `configure`), which no case exercised.
use super::*;
use crate::chatgpt::{
    CHATGPT_CLIENT_ID, CHATGPT_INSTANCE, CHATGPT_PROFILE, carries_chatgpt_material, chatgpt_config,
    profile_at,
};
use crate::oauth2_refresh::{Source, oauth_config};
use std::io::Read as _;

/// The alias of the adversary's pinned instance.
const ALIAS: &str = "chatgpt";
/// The user-code route of the ChatGPT device sign-in.
const USER_CODE_ROUTE: &str = "POST /api/accounts/deviceauth/usercode";

/// A free loopback port for the fixed-port redirect.
fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A loopback HTTPS issuer that answers the ChatGPT device sign-in's user-code
/// request with `interval` seconds between polls, and every poll as pending
/// (403), as while the person has not entered the code yet. It records the
/// routes it was asked and issues no token.
struct DeviceIssuer {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    config: PathBuf,
    routes: Arc<Mutex<Vec<String>>>,
}

impl DeviceIssuer {
    fn new(interval: &'static str) -> Self {
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
        let routes = Arc::new(Mutex::new(Vec::new()));
        let seen = routes.clone();
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
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, value)| value.trim().parse().ok())
                        })
                        .unwrap_or(0_usize)
                        .min(65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let mut words = head.split_whitespace();
                    let route = format!(
                        "{} {}",
                        words.next().unwrap_or_default(),
                        words.next().unwrap_or_default()
                    );
                    seen.lock().unwrap().push(route.clone());
                    let (status, answer) = match route.as_str() {
                        USER_CODE_ROUTE => (
                            200,
                            json!({
                                "device_auth_id": "fixture-chatgpt-device-adversary-1",
                                "user_code": "ADVX-0001",
                                "interval": interval,
                            }),
                        ),
                        "POST /api/accounts/deviceauth/token" => {
                            (403, json!({"error": "fixture pending"}))
                        }
                        _ => (404, json!({"error": "fixture route"})),
                    };
                    let bytes = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&chatgpt_config(
                &profile_at(port, free_port()),
                Some(CHATGPT_CLIENT_ID),
                Some((port, &ca)),
            ))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            routes,
        }
    }

    fn routes(&self) -> Vec<String> {
        self.routes.lock().unwrap().clone()
    }

    /// The configured selection of this issuer's provider child.
    fn selection(&self) -> Adapter {
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        let bootstrap = bootstrap_of(&self.config).expect("bootstrap inspection failed");
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
}

impl Drop for DeviceIssuer {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

/// The printed bootstrap of the configuration at `path`, or the refusal's
/// stderr; neither stream carries fixture material.
fn bootstrap_of(path: &Path) -> Result<Value, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(!carries_chatgpt_material(&output.stdout));
    assert!(!carries_chatgpt_material(&output.stderr));
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(serde_json::from_slice(&output.stdout).unwrap())
}

/// [`bootstrap_of`] for `config` written to a fresh private file.
fn bootstrap(config: &Value) -> Result<Value, String> {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(config).unwrap());
    bootstrap_of(&path)
}

/// A production CLI's `connect --device` of the ChatGPT profile against
/// `issuer`, running, and the custody it stores in.
struct DeviceConnect {
    process: OwnedProcess,
    _custody: Custody,
}

impl DeviceConnect {
    /// Configures the pinned instance against `issuer`, starts the connect and
    /// returns once the dialect has asked the user code.
    fn start(issuer: &DeviceIssuer) -> Self {
        let custody = Custody::new(issuer.root.path());
        let cli = Cli::new(issuer.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = issuer.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.{ALIAS}]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.{ALIAS}.permissions]\nprofiles=[{}]\noperations=['models.list']\n[adapters.{ALIAS}.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(CHATGPT_PROFILE),
            q(adapter.executable.path.to_str().unwrap()),
            q(&adapter.executable.sha256),
            serde_json::to_string(&adapter.executable.args).unwrap()
        );
        private(&cli.paths.config, configuration.as_bytes());
        let mut connect = Self {
            process: OwnedProcess(
                cli.command(&[
                    "connections",
                    "connect",
                    "--adapter",
                    ALIAS,
                    "--profile",
                    CHATGPT_PROFILE,
                    "--device",
                ])
                .env("CONNECTORS_TEST_OAUTH_FOLLOW", &issuer.ca)
                .spawn()
                .unwrap(),
            ),
            _custody: custody,
        };
        // The dialect asks the user code: the sign-in has begun.
        let until = Instant::now() + Duration::from_secs(30);
        while !issuer.routes().iter().any(|route| route == USER_CODE_ROUTE) {
            if let Some((status, output)) = connect.finished() {
                panic!(
                    "setup: the CLI ended before the dialect asked a user code ({status}): {output}; routes {:?}",
                    issuer.routes()
                );
            }
            assert!(
                Instant::now() < until,
                "setup: no user code asked: {:?}",
                issuer.routes()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        connect
    }

    /// The exit status and both streams once the CLI has ended; neither
    /// stream carries fixture material.
    fn finished(&mut self) -> Option<(std::process::ExitStatus, String)> {
        let status = self.process.0.try_wait().unwrap()?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        self.process
            .0
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut stdout)
            .unwrap();
        self.process
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_end(&mut stderr)
            .unwrap();
        assert!(!carries_chatgpt_material(&stdout), "material on stdout");
        assert!(!carries_chatgpt_material(&stderr), "material on stderr");
        Some((
            status,
            format!(
                "stdout {} stderr {}",
                String::from_utf8_lossy(&stdout).trim(),
                String::from_utf8_lossy(&stderr).trim()
            ),
        ))
    }
}

/// Replaces pass 1's fifteen-minute case: the guide now gives the person
/// about 4 minutes, the grant's 240 s window. A server that asks to be polled
/// every 241 s leaves no poll inside it, so the CLI refuses the sign-in at
/// once as `unsupported` (no retry helps), never as the person's `timeout`,
/// and sends no poll.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn adversary_chatgpt_device_sign_in_refuses_an_interval_longer_than_the_window_before_any_poll() {
    let issuer = DeviceIssuer::new("241");
    let mut connect = DeviceConnect::start(&issuer);
    let asked = Instant::now();
    let (status, output) = loop {
        if let Some(finished) = connect.finished() {
            break finished;
        }
        assert!(
            asked.elapsed() < Duration::from_secs(20),
            "the CLI still waits {:?} after the user code with a 241 s interval: routes {:?}",
            asked.elapsed(),
            issuer.routes()
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success(), "{status}: {output}");
    assert!(
        output.contains(r#""code":"unsupported""#)
            && output.contains(r#""next_action":"none""#)
            && !output.contains(r#""code":"timeout""#),
        "{status}: {output}"
    );
    assert_eq!(issuer.routes(), [USER_CODE_ROUTE], "a poll was sent");
}

/// At a 1 s interval the CLI keeps polling while the person enters the code:
/// 8 s after the user code it is still waiting and has polled. The fake-clock
/// case `oauth2_grant::tests::a_normal_interval_polls_into_the_window_s_last_ten_seconds`
/// holds the rest of the promise, polling into the window's last 10 s.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn adversary_chatgpt_device_sign_in_keeps_polling_at_a_normal_interval() {
    let issuer = DeviceIssuer::new("1");
    let mut connect = DeviceConnect::start(&issuer);
    let asked = Instant::now();
    while asked.elapsed() < Duration::from_secs(8) {
        if let Some((status, output)) = connect.finished() {
            panic!(
                "the CLI gave up {:?} after the user code, with about 4 minutes promised ({status}): {output}; routes {:?}",
                asked.elapsed(),
                issuer.routes()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let polls = issuer
        .routes()
        .iter()
        .filter(|route| *route == "POST /api/accounts/deviceauth/token")
        .count();
    assert!(polls >= 3, "{polls} polls in 8 s at a 1 s interval");
    // Still waiting for the person, as promised; the journey ends it here.
    drop(connect);
}

/// An `id_token` claim pointer belongs to an `oauth2_grant` profile only
/// (`connectors_catalog.identity.Profile`): an `oauth2_refresh` profile whose
/// `id_token` identity names one is invalid configuration, while the same
/// profile without it loads.
#[test]
fn adversary_an_id_token_claim_pointer_under_oauth2_refresh_is_invalid_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let ca = private_directory.join("ca.pem");
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    private(&ca, cert.cert.pem().as_bytes());
    let config = oauth_config(
        4443,
        &ca,
        Source::IdToken,
        "https://localhost:4443/oauth/token",
        "https://localhost:4443/oauth/authorize",
    );
    bootstrap(&config).expect("the profile without a pointer loads");
    let mut pointed = config.clone();
    pointed["auth"]["identity"]["subject_pointer"] = json!("/email");
    assert_eq!(
        bootstrap(&pointed).unwrap_err(),
        "InvalidConfiguration",
        "a claim pointer under oauth2_refresh"
    );
}

/// Under `oauth2_grant`, the claim pointer is an RFC 6901 pointer into the
/// `id_token`'s claims: one that does not start with `/`, or the empty
/// pointer (the whole claims object), is invalid configuration.
#[test]
fn adversary_an_id_token_claim_pointer_that_is_not_a_member_pointer_is_invalid_configuration() {
    let mut profile = profile_at(4443, 1455);
    bootstrap(&chatgpt_config(&profile, Some(CHATGPT_CLIENT_ID), None))
        .expect("the shipped pointer loads");
    for pointer in ["https:~1~1api.openai.com~1auth/chatgpt_account_id", ""] {
        profile["identity"]["subject_pointer"] = json!(pointer);
        assert_eq!(
            bootstrap(&chatgpt_config(&profile, Some(CHATGPT_CLIENT_ID), None)).unwrap_err(),
            "InvalidConfiguration",
            "pointer {pointer:?}"
        );
    }
}
