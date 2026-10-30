//! story:invoke-definition-file, its acceptance: against a disposable HTTPS
//! provider, a hand-written token-profile definition is invoked with `invoke
//! --definition <file> <operation>` (story:connectors-do-short-invocation)
//! while no instance is configured for it and no connection exists. The one
//! connect question is answered "yes", the token is entered at the token
//! prompt that follows, and the operation's result comes back. A second run
//! with the same file, and a third after only an added operation, return
//! results with neither the question nor the token prompt, and the operator's
//! configuration file is byte-identical before and after.
//!
//! The definition's API is `https://gitlab.example.com/api/v4`. The journey runs
//! in fresh user, network and mount namespaces in which that host resolves to a
//! disposable HTTPS fixture on loopback port 443 whose root is the only one the
//! system trust store holds (`derived_instance::reenter`); nothing outside the
//! namespaces is reached or changed. Each prompt is answered on a fresh
//! pseudo-terminal that is the CLI's controlling terminal. The fixture token is
//! fictional and only ever compared, never printed.
use super::super::derived_instance::{WORK, enter, reenter};
use super::*;
use std::os::{fd::FromRawFd, unix::process::CommandExt};
use std::sync::atomic::AtomicU64;

/// Set in the re-executed test process that runs inside the namespaces.
const INSIDE: &str = "CONNECTORS_TEST_INVOKE_DEFINITION_NAMESPACES";
const NAME: &str = "cli_journey::invoke_definition::a_hand_written_definition_connects_on_its_first_invoke_and_then_invokes_without_asking";
const HOST: &str = "gitlab.example.com";
/// The host a definition is moved to: the same fixture answers it.
const MOVED_HOST: &str = "gitlab-moved.example.com";
const TOKEN: &str = "fixture-definition-token";
/// How the connect question ends on the terminal.
const QUESTION: &[u8] = b"[y/N] ";
/// The token prompt: the profile's label, as every token connect shows it.
const TOKEN_PROMPT: &[u8] = b"Fixture personal access token: ";

/// The vendored source the definitions pin: a hand-written OpenAPI document
/// with the identity and scope probes and three reads.
const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://gitlab.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"},"name":{"type":"string"}}}}}}}}},
"/projects/{id}/issues":{"get":{"operationId":"getApiV4ProjectsIdIssues","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"array","items":{"type":"object"}}}}}}}}
}}"#;

/// A token-profile definition, as an operator writes it by hand, exposing
/// `operations` (`project.get`, `issues.list`) on `hostname`. `catalog` names
/// the bundle when the file stem does not.
fn definition(
    source_sha256: &str,
    hostname: &str,
    catalog: Option<&str>,
    operations: &[&str],
) -> String {
    let mut text = String::from("schema: urn:beyond10x:connectors-definition:1\n");
    if let Some(catalog) = catalog {
        text.push_str(&format!("catalog: {catalog}\n"));
    }
    text.push_str(&format!(
        "source:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {source_sha256}\n\
         base_url: https://{{hostname}}/api/v4\n\
         config: {{hostname: {{type: string, default: {hostname}}}}}\n\
         auth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n\
         \x20   label: Fixture personal access token\n\
         \x20   identity: {{path: user, kind: forge.user, subject: /id}}\n\
         \x20   scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\n\
         operations:\n"
    ));
    for operation in operations {
        let (op, description) = match *operation {
            "project.get" => ("getApiV4ProjectsId", "Read one project"),
            "issues.list" => ("getApiV4ProjectsIdIssues", "List a project's issues"),
            other => panic!("no fixture operation {other}"),
        };
        text.push_str(&format!(
            "  {operation}:\n    op: {op}\n    description: {description}\n"
        ));
    }
    text
}

/// One request the fixture answered: its route and whether it carried the
/// fixture token. Compared, never printed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    host: String,
    route: String,
    authenticated: bool,
}

/// A GitLab-shaped API at `gitlab.example.com`, and at `gitlab-moved.example.com`
/// for a definition moved there, on loopback port 443.
struct Forge {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    requests: Arc<Mutex<Vec<Request>>>,
    /// The user id `/user` answers for the fixture token.
    identity: Arc<AtomicU64>,
}

impl Forge {
    /// Points the host at loopback and makes the fixture's certificate the
    /// only trusted root, enters the private root (`enter`), and serves.
    fn new(base: &Path) -> Self {
        let cert = rcgen::generate_simple_self_signed(vec![HOST.to_owned(), MOVED_HOST.to_owned()])
            .unwrap();
        let pem = cert.cert.pem();
        let etc = base.join("etc");
        fs::create_dir_all(etc.join("ssl/certs")).unwrap();
        fs::write(
            etc.join("hosts"),
            format!("127.0.0.1 localhost {HOST} {MOVED_HOST}\n"),
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
        let identity = Arc::new(AtomicU64::new(42));
        let answered = identity.clone();
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
                    let target = head.split_whitespace().nth(1).unwrap_or_default();
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let request = Request {
                        host: header("host")
                            .map(|host| host.trim_end_matches(":443").to_owned())
                            .unwrap_or_default(),
                        route,
                        authenticated: header("private-token").as_deref() == Some(TOKEN),
                    };
                    let (status, body) = if !request.authenticated {
                        (401, json!({"message": "401 Unauthorized"}))
                    } else {
                        match request.route.as_str() {
                            "/api/v4/user" => (
                                200,
                                json!({"id": answered.load(Ordering::SeqCst), "username": "fixture"}),
                            ),
                            "/api/v4/personal_access_tokens/self" => (
                                200,
                                json!({"id": 99, "user_id": 42, "active": true, "revoked": false,
                                       "scopes": ["api"], "expires_at": null}),
                            ),
                            "/api/v4/projects/org%2Fproject" => {
                                (200, json!({"id": 7, "name": "fixture-project"}))
                            }
                            "/api/v4/projects/org%2Fproject/issues" => {
                                (200, json!([{"id": 1, "title": "fixture"}]))
                            }
                            _ => (404, json!({"message": "404 Not Found"})),
                        }
                    };
                    served.lock().unwrap().push(request);
                    let body = serde_json::to_vec(&body).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
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
            requests,
            identity,
        }
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    /// From now on `/user` answers `id` for the fixture token: the same
    /// token, another account.
    fn answer_identity(&self, id: u64) {
        self.identity.store(id, Ordering::SeqCst);
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        let _ = self.thread.take().unwrap().join();
    }
}

/// The CLI with its catalog provider beside it, custody, an operator
/// configuration that configures no instance, and a definitions tree.
struct Journey {
    // Dropped in this order: the owner stops before custody.
    cli: PathBuf,
    config: PathBuf,
    state: PathBuf,
    defs: PathBuf,
    source_sha256: String,
    root: PathBuf,
    _custody: Custody,
}

impl Journey {
    fn new() -> Self {
        let root = PathBuf::from(WORK);
        filesystem::directory(&root.join("private"), true, true).unwrap();
        let custody = Custody::new(&root);
        let defs = root.join("defs");
        fs::create_dir_all(defs.join("connectors")).unwrap();
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(OPENAPI.as_bytes()).unwrap();
        fs::write(
            defs.join("upstream/forge/openapi.json.gz"),
            gzip.finish().unwrap(),
        )
        .unwrap();
        let journey = Self {
            cli: root.join("bin/connectors"),
            config: root.join("cli/config.toml"),
            state: root.join("cli/state"),
            defs,
            source_sha256: hex::encode(Sha256::digest(OPENAPI.as_bytes())),
            root,
            _custody: custody,
        };
        success(journey.command(&["setup", "init"]).output().unwrap());
        private(
            &journey.config,
            format!(
                "format='connectors-local/2'\nowner_uid={}\nsecret_service_socket={}\n",
                filesystem::uid(),
                serde_json::to_string(journey._custody.socket.to_str().unwrap()).unwrap(),
            )
            .as_bytes(),
        );
        journey
    }

    /// Write `connectors/<stem>.yaml`; answers its path.
    fn write(
        &self,
        stem: &str,
        hostname: &str,
        catalog: Option<&str>,
        operations: &[&str],
    ) -> String {
        let path = self.defs.join(format!("connectors/{stem}.yaml"));
        fs::write(
            &path,
            definition(&self.source_sha256, hostname, catalog, operations),
        )
        .unwrap();
        path.to_str().unwrap().to_owned()
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
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    /// `args` with no controlling terminal: the CLI leads its own session.
    fn detached(&self, args: &[&str]) -> Output {
        let mut command = self.command(args);
        // SAFETY: setsid in the child between fork and exec; no allocation.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = command.output().unwrap();
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        output
    }

    /// `args` as the foreground of its own session on a fresh
    /// pseudo-terminal. Each answer is typed once the terminal shows the
    /// prompt it follows, in order. Answers the output and everything the
    /// terminal showed.
    fn on_terminal(&self, args: &[&str], answers: &[(&[u8], &[u8])]) -> (Output, Vec<u8>) {
        let (mut master_fd, mut slave_fd) = (-1, -1);
        // SAFETY: initialized output slots; default terminal size and mode.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master_fd,
                    &mut slave_fd,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        // SAFETY: both descriptors were just opened and are owned here.
        let (mut master, slave) = unsafe {
            (
                fs::File::from_raw_fd(master_fd),
                fs::File::from_raw_fd(slave_fd),
            )
        };
        // SAFETY: a live descriptor; scalar flags.
        assert_eq!(
            unsafe { libc::fcntl(master_fd, libc::F_SETFL, libc::O_NONBLOCK) },
            0
        );
        for fd in [master_fd, slave_fd] {
            // SAFETY: a live descriptor; scalar flags.
            assert_eq!(
                unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
                0
            );
        }
        let mut command = self.command(args);
        // SAFETY: the child alone creates its session and adopts the slave as
        // its controlling terminal; no foreign process is signalled.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let (mut stdout, mut stderr) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
        let readers = [
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut stdout, &mut bytes).unwrap();
                bytes
            }),
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut stderr, &mut bytes).unwrap();
                bytes
            }),
        ];
        let mut shown = Vec::new();
        let mut answered = 0;
        let mut since = 0;
        let until = Instant::now() + Duration::from_secs(120);
        let status = loop {
            let mut bytes = [0; 256];
            match std::io::Read::read(&mut master, &mut bytes) {
                Ok(n) => shown.extend_from_slice(&bytes[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                // Every slave descriptor closed: the CLI has exited.
                Err(_) => {}
            }
            if let Some((prompt, answer)) = answers.get(answered)
                && shown[since..].ends_with(prompt)
            {
                master.write_all(answer).unwrap();
                answered += 1;
                since = shown.len();
            }
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < until, "the CLI did not finish");
            std::thread::sleep(Duration::from_millis(5));
        };
        drop(slave);
        let [stdout, stderr] = readers.map(|reader| reader.join().unwrap());
        assert_eq!(
            answered,
            answers.len(),
            "{}",
            String::from_utf8_lossy(&shown)
        );
        assert!(!carries_token(&stdout) && !carries_token(&stderr));
        (
            Output {
                status,
                stdout,
                stderr,
            },
            shown,
        )
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

impl Journey {
    /// Each cache entry under the state directory: its digest and the
    /// identity of its native configuration, which a second lowering would
    /// replace.
    fn cache_entries(&self) -> Vec<(String, u64, i64)> {
        let mut entries: Vec<_> = fs::read_dir(self.state.join("definitions"))
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| entry.file_type().unwrap().is_dir())
            .map(|entry| {
                let native = fs::metadata(entry.path().join("native.json")).unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    std::os::unix::fs::MetadataExt::ino(&native),
                    std::os::unix::fs::MetadataExt::mtime_nsec(&native),
                )
            })
            .collect();
        entries.sort();
        entries
    }
}

fn carries_token(haystack: &[u8]) -> bool {
    haystack
        .windows(TOKEN.len())
        .any(|window| window == TOKEN.as_bytes())
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[track_caller]
fn refused_with(output: &Output) -> Value {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    envelope["error"]["data"].clone()
}

/// `invoke --definition <file> <operation>` (story:connectors-do-short-invocation).
fn invoke<'a>(definition: &'a str, operation: &'a str) -> Vec<&'a str> {
    vec![
        "invoke",
        "--definition",
        definition,
        operation,
        "--input-json",
        r#"{"id":"org/project"}"#,
    ]
}

#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn a_hand_written_definition_connects_on_its_first_invoke_and_then_invokes_without_asking() {
    let Some(base) = std::env::var_os(INSIDE) else {
        let output = reenter(INSIDE, NAME);
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        return;
    };
    let fixture = Forge::new(Path::new(&base));
    let journey = Journey::new();
    let configuration = fs::read(&journey.config).unwrap();
    let forge = journey.write("forge", HOST, None, &["project.get"]);

    // No terminal: nothing is asked, the refusal names the connect, and
    // nothing reaches the provider.
    let data = refused_with(&journey.detached(&invoke(&forge, "project.get")));
    assert_eq!(
        (&data["code"], &data["stage"], &data["next_action"]),
        (
            &json!("not_found"),
            &json!("admission"),
            &json!("create_connection")
        ),
        "{data}"
    );
    // Asked once and answered no: the same refusal, no token prompt.
    let (output, shown) =
        journey.on_terminal(&invoke(&forge, "project.get"), &[(QUESTION, b"n\n")]);
    let data = refused_with(&output);
    assert_eq!(data["next_action"], "create_connection", "{data}");
    assert_eq!(count(&shown, QUESTION), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 0);
    assert!(fixture.requests().is_empty(), "{:?}", fixture.requests());

    // Asked once, answered yes, the token entered at the prompt that
    // follows: the operation's result.
    let token = format!("{TOKEN}\n");
    let (output, shown) = journey.on_terminal(
        &invoke(&forge, "project.get"),
        &[(QUESTION, b"y\n"), (TOKEN_PROMPT, token.as_bytes())],
    );
    let first = success(output);
    assert_eq!(first["result"]["body"]["id"], 7, "{first}");
    assert_eq!(
        (
            &first["connector"],
            &first["instance"],
            first.get("adapter")
        ),
        (&json!("forge"), &json!("definition"), None),
        "story:connectors-do-short-invocation: the answer names no alias: {first}"
    );
    assert_eq!(count(&shown, QUESTION), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 1);
    assert!(!carries_token(&shown), "the token is not echoed");
    let lowered = journey.cache_entries();
    assert_eq!(lowered.len(), 1, "{lowered:?}");

    // The same file again: the result, nothing asked.
    let (output, shown) = journey.on_terminal(&invoke(&forge, "project.get"), &[]);
    assert_eq!(success(output)["result"]["body"]["id"], 7);
    assert!(shown.is_empty(), "{}", String::from_utf8_lossy(&shown));
    // Unchanged inputs: the same cache entry, not lowered again.
    assert_eq!(journey.cache_entries(), lowered);

    // Only an operation added: the new operation's result, nothing asked.
    journey.write("forge", HOST, None, &["project.get", "issues.list"]);
    let (output, shown) = journey.on_terminal(&invoke(&forge, "issues.list"), &[]);
    let added = success(output);
    assert_eq!(added["result"]["body"][0]["id"], 1, "{added}");
    assert!(shown.is_empty(), "{}", String::from_utf8_lossy(&shown));
    // A changed definition is a new cache entry beside the first.
    let both = journey.cache_entries();
    assert_eq!(both.len(), 2, "{both:?}");
    assert!(lowered.iter().all(|entry| both.contains(entry)));

    // One connection, the instance's; the operator's file is untouched.
    let listed = success(
        journey
            .command(&[
                "connections",
                "list",
                "--connector",
                "forge",
                "--instance",
                "definition",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        listed["connections"].as_array().unwrap().len(),
        1,
        "{listed}"
    );
    assert_eq!(fs::read(&journey.config).unwrap(), configuration);

    // The cache and the instance entry are the owner's alone.
    let definitions = journey.state.join("definitions");
    assert_eq!(
        fs::metadata(&definitions).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for entry in fs::read_dir(&definitions).unwrap() {
        let entry = entry.unwrap();
        let mode = entry.metadata().unwrap().permissions().mode() & 0o777;
        let expected = if entry.file_type().unwrap().is_dir() {
            0o700
        } else {
            0o600
        };
        assert_eq!(mode, expected, "{:?}", entry.file_name());
    }

    // Every request went to the definition's host and carried the token: the
    // connect's identity and scope probes and the reads. None was sent before
    // the operator said yes (above).
    let requests = fixture.requests();
    assert!(
        requests.iter().all(|r| r.host == HOST && r.authenticated),
        "{requests:?}"
    );
    for route in [
        "/api/v4/projects/org%2Fproject",
        "/api/v4/projects/org%2Fproject/issues",
    ] {
        assert!(requests.iter().any(|r| r.route == route), "{route}");
    }

    // The OAuth client file branch, here with a token profile whose file is
    // the entry itself: asked once, no token prompt.
    let client_file = journey.root.join("private/entry.json");
    private(&client_file, format!(r#"{{"token":"{TOKEN}"}}"#).as_bytes());
    let file_definition = journey.write("forge-file", HOST, Some("forge"), &["project.get"]);
    let mut args = invoke(&file_definition, "project.get");
    args.extend(["--credential-file", client_file.to_str().unwrap()]);
    let (output, shown) = journey.on_terminal(&args, &[(QUESTION, b"y\n")]);
    let by_file = success(output);
    assert_eq!(by_file["result"]["body"]["id"], 7, "{by_file}");
    assert_eq!(by_file["connector"], "forge-file", "{by_file}");
    assert_eq!(count(&shown, QUESTION), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 0);

    // An authority change to the file resolves to a new instance id
    // (task:definition-moved-authority-new-instance): its first invoke asks
    // the one connect question naming it; answered no, the refusal names the
    // connect and nothing is sent.
    let before = fixture.requests().len();
    journey.write("forge", MOVED_HOST, None, &["project.get"]);
    let (output, shown) =
        journey.on_terminal(&invoke(&forge, "project.get"), &[(QUESTION, b"n\n")]);
    let data = refused_with(&output);
    assert_eq!(
        (&data["code"], &data["next_action"]),
        (&json!("not_found"), &json!("create_connection")),
        "{data}"
    );
    assert_eq!(count(&shown, b"No connection for forge/definition-"), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 0);
    assert_eq!(fixture.requests().len(), before, "nothing was sent");
    assert_eq!(fs::read(&journey.config).unwrap(), configuration);
}

/// story:invoke-definition-reuse-credential: the instance `main` of the
/// connector `forge`, which the operator configured, holds a token connection
/// at the definition's host.
const REUSE_NAME: &str = "cli_journey::invoke_definition::a_stored_credential_of_a_configured_instance_is_reused_without_a_token_prompt";
const MISMATCH_NAME: &str = "cli_journey::invoke_definition::a_reuse_the_provider_answers_with_another_identity_publishes_nothing_and_offers_connect_fresh";
/// How the reuse question ends on the terminal with one candidate.
const REUSE_QUESTION: &[u8] = b"[1/c/N] ";
/// What the question lists for the candidate: its instance and its host.
const CANDIDATE: &[u8] = b"forge/main";
const API: &[u8] = b"https://gitlab.example.com/api/v4";

impl Journey {
    /// Configure the instance `main` of the connector `forge` as an operator
    /// pins one: the catalog provider over the native configuration the
    /// definition at `definition` lowers to, under the instance id
    /// `forge.main`, so its endpoint and profile declaration are the
    /// definition's. The definition is lowered by a run without a terminal,
    /// which asks nothing and connects nothing.
    fn configure_main(&self, definition: &str) {
        let data = refused_with(&self.detached(&invoke(definition, "project.get")));
        assert_eq!(data["next_action"], "create_connection", "{data}");
        let [(digest, _, _)]: [(String, u64, i64); 1] = self.cache_entries().try_into().unwrap();
        let native = self
            .state
            .join("definitions")
            .join(digest)
            .join("native.json");
        let mut native: Value = serde_json::from_slice(&fs::read(native).unwrap()).unwrap();
        native["instance"] = json!("forge.main");
        let main = self.root.join("private/forge-main.json");
        private(&main, &serde_json::to_vec_pretty(&native).unwrap());
        let provider = self.root.join("bin/connectors-catalog-provider");
        let printed = Command::new(&provider)
            .env_clear()
            .arg("--local-config")
            .arg(&main)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(printed.status.success(), "{printed:?}");
        let bootstrap: Value = serde_json::from_slice(&printed.stdout).unwrap();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let table = "connectors.forge.instances.main";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={}\nsecret_service_socket={}\n\
             [connectors.forge]\nenabled=true\n\
             [{table}]\nalias='forge.main'\ninstance_id='forge.main'\nadapter_id='catalog'\n\
             configuration_revision={}\ncredential_revision={}\nprotocol='v1alpha1'\n\
             private_protocol='connectors-private/2'\n\
             [{table}.executable]\npath={}\nsha256={}\nargs=['--local-config',{}]\n\
             [{table}.permissions]\nprofiles=['forge.pat']\noperations=['project.get']\n",
            filesystem::uid(),
            q(self._custody.socket.to_str().unwrap()),
            q(bootstrap["configuration_revision"].as_str().unwrap()),
            q(bootstrap["credential_revision"].as_str().unwrap()),
            q(provider.to_str().unwrap()),
            q(&hex::encode(Sha256::digest(fs::read(&provider).unwrap()))),
            q(main.to_str().unwrap()),
        );
        private(&self.config, text.as_bytes());
    }

    /// `connections connect` of `forge/main` with the fixture token on
    /// standard input; answers the connection.
    fn connect_main(&self) -> String {
        let mut command = self.command(&[
            "connections",
            "connect",
            "--connector",
            "forge",
            "--instance",
            "main",
            "--profile",
            "forge.pat",
            "--credential-stdin",
        ]);
        let mut child = command.stdin(Stdio::piped()).spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!(r#"{{"token":"{TOKEN}"}}"#).as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        let connected = success(output);
        connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// `connections describe` of `connection` under `forge/<instance>`.
    fn describe(&self, instance: &str, connection: &str) -> Value {
        success(
            self.command(&[
                "connections",
                "describe",
                "--connector",
                "forge",
                "--instance",
                instance,
                "--connection",
                connection,
            ])
            .output()
            .unwrap(),
        )["connection"]
            .clone()
    }

    /// The connections `forge/<instance>` lists.
    fn listed(&self, instance: &str) -> Vec<Value> {
        success(
            self.command(&[
                "connections",
                "list",
                "--connector",
                "forge",
                "--instance",
                instance,
            ])
            .output()
            .unwrap(),
        )["connections"]
            .as_array()
            .unwrap()
            .clone()
    }
}

/// What must not change about a source connection: its revision, state,
/// external identity and staleness.
fn unchanged(connection: &Value) -> Value {
    json!([
        connection["summary"]["revision"],
        connection["summary"]["state"],
        connection["external_identity"],
        connection["stale"],
    ])
}

/// The acceptance: with a token connection under the configured instance
/// `forge/main` at `gitlab.example.com` and a hand-written definition of
/// `forge` at the same host, `invoke --definition` asks once; the
/// answer "reuse" yields the operation's result without a token prompt, a new
/// connection under the definition's instance with the same validated
/// identity, and the configured instance's connection unchanged. Without a
/// terminal nothing is asked and nothing is copied.
#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn a_stored_credential_of_a_configured_instance_is_reused_without_a_token_prompt() {
    let Some(base) = std::env::var_os(INSIDE) else {
        let output = reenter(INSIDE, REUSE_NAME);
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        return;
    };
    let fixture = Forge::new(Path::new(&base));
    let journey = Journey::new();
    let forge = journey.write("forge", HOST, None, &["project.get"]);
    journey.configure_main(&forge);
    let configuration = fs::read(&journey.config).unwrap();
    let source = journey.connect_main();
    let before = journey.describe("main", &source);
    assert_eq!(before["summary"]["state"], "ready", "{before}");
    let connects = fixture.requests().len();

    // No terminal: nothing is asked, the refusal names the connect, and
    // nothing is copied or sent.
    let data = refused_with(&journey.detached(&invoke(&forge, "project.get")));
    assert_eq!(data["next_action"], "create_connection", "{data}");
    assert!(journey.listed("definition").is_empty());
    assert_eq!(fixture.requests().len(), connects);

    // Asked once, the candidate listed by its instance and host, answered
    // with its number: the operation's result, and no token prompt.
    let (output, shown) =
        journey.on_terminal(&invoke(&forge, "project.get"), &[(REUSE_QUESTION, b"1\n")]);
    let result = success(output);
    assert_eq!(result["result"]["body"]["id"], 7, "{result}");
    assert_eq!(result["instance"], "definition", "{result}");
    assert_eq!(count(&shown, REUSE_QUESTION), 1);
    assert_eq!(count(&shown, QUESTION), 0);
    assert_eq!(count(&shown, TOKEN_PROMPT), 0);
    assert_eq!(count(&shown, CANDIDATE), 1);
    assert_eq!(count(&shown, API), 1);
    assert!(!carries_token(&shown));

    // A new connection under the definition's instance, with the source's
    // validated identity; the source is as it was.
    let listed = journey.listed("definition");
    assert_eq!(listed.len(), 1, "{listed:?}");
    let copy = listed[0]["connection"].as_str().unwrap().to_owned();
    assert_ne!(copy, source);
    let copied = journey.describe("definition", &copy);
    assert_eq!(copied["summary"]["state"], "ready", "{copied}");
    assert_eq!(
        copied["external_identity"], before["external_identity"],
        "{copied}"
    );
    assert_eq!(
        unchanged(&journey.describe("main", &source)),
        unchanged(&before)
    );
    assert_eq!(journey.listed("main").len(), 1);

    // The copy was validated at the definition's host, then the read ran.
    let requests = fixture.requests();
    assert!(
        requests[connects..]
            .iter()
            .all(|r| r.host == HOST && r.authenticated),
        "{requests:?}"
    );
    assert!(
        requests[connects..]
            .iter()
            .any(|r| r.route == "/api/v4/user"),
        "{requests:?}"
    );

    // The next run asks nothing.
    let (output, shown) = journey.on_terminal(&invoke(&forge, "project.get"), &[]);
    assert_eq!(success(output)["result"]["body"]["id"], 7);
    assert!(shown.is_empty(), "{}", String::from_utf8_lossy(&shown));

    // Revoking the copy leaves the source as it was: two custody entries.
    let revision = copied["summary"]["revision"].as_str().unwrap();
    success(
        journey
            .command(&[
                "connections",
                "revoke",
                "--connector",
                "forge",
                "--instance",
                "definition",
                "--connection",
                &copy,
                "--expected-revision",
                revision,
            ])
            .output()
            .unwrap(),
    );
    let after = journey.describe("main", &source);
    assert_eq!(unchanged(&after), unchanged(&before));
    assert_eq!(fs::read(&journey.config).unwrap(), configuration);
}

/// Beside the acceptance: when the provider answers the reused credential
/// with another identity than the source's, nothing is published, the source
/// is unchanged, and connect fresh is offered. A revoked source is never
/// offered.
#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn a_reuse_the_provider_answers_with_another_identity_publishes_nothing_and_offers_connect_fresh() {
    let Some(base) = std::env::var_os(INSIDE) else {
        let output = reenter(INSIDE, MISMATCH_NAME);
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        return;
    };
    let fixture = Forge::new(Path::new(&base));
    let journey = Journey::new();
    let forge = journey.write("forge", HOST, None, &["project.get"]);
    journey.configure_main(&forge);
    let source = journey.connect_main();
    let before = journey.describe("main", &source);

    // The same token now answers for another account.
    fixture.answer_identity(43);
    let (output, shown) = journey.on_terminal(
        &invoke(&forge, "project.get"),
        &[(REUSE_QUESTION, b"1\n"), (QUESTION, b"n\n")],
    );
    let data = refused_with(&output);
    assert_eq!(data["code"], "identity_mismatch", "{data}");
    assert_eq!(data["next_action"], "create_connection", "{data}");
    assert_eq!(count(&shown, REUSE_QUESTION), 1);
    // Connect fresh is offered, and declined.
    assert_eq!(count(&shown, QUESTION), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 0);
    assert!(!carries_token(&shown));
    assert!(journey.listed("definition").is_empty());
    assert_eq!(
        unchanged(&journey.describe("main", &source)),
        unchanged(&before)
    );
    // Not the source's state either: another identity says nothing about it.
    assert_eq!(
        journey.describe("main", &source)["summary"]["state"],
        "ready"
    );

    // Revoked, the source is not offered: the question is the plain connect.
    let revision = before["summary"]["revision"].as_str().unwrap();
    success(
        journey
            .command(&[
                "connections",
                "revoke",
                "--connector",
                "forge",
                "--instance",
                "main",
                "--connection",
                &source,
                "--expected-revision",
                revision,
            ])
            .output()
            .unwrap(),
    );
    let (output, shown) =
        journey.on_terminal(&invoke(&forge, "project.get"), &[(QUESTION, b"n\n")]);
    assert_eq!(refused_with(&output)["next_action"], "create_connection");
    assert_eq!(count(&shown, REUSE_QUESTION), 0);
    assert_eq!(count(&shown, CANDIDATE), 0);
}

const MOVED_NAME: &str = "cli_journey::invoke_definition::a_definition_moved_to_another_host_connects_fresh_under_a_new_instance_id";

/// The second check beside the acceptance, restated by
/// task:definition-moved-authority-new-instance. With the definition's own
/// connection at the file's host and the file then naming another host,
/// nothing is reused (option A, 2026-10-18; D12 point 1): no candidate is
/// listed and nothing is copied. The definition resolves to a new instance id,
/// `definition-<16 hex>`, because a connect under `definition` would be refused
/// (the registry's "only a new instance id helps"), so the run is a first
/// invoke there: one connect question, the token prompt, the result from the
/// new host. The id is a function of the definition: a second run asks nothing
/// on the same instance, and a file moved back resolves to `definition` and
/// its connection again. The connection left at the old host keeps its
/// revision and is revoked through the moved-entry path; the configured
/// instance's connection is unchanged.
#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn a_definition_moved_to_another_host_connects_fresh_under_a_new_instance_id() {
    let Some(base) = std::env::var_os(INSIDE) else {
        let output = reenter(INSIDE, MOVED_NAME);
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        return;
    };
    let fixture = Forge::new(Path::new(&base));
    let journey = Journey::new();
    let forge = journey.write("forge", HOST, None, &["project.get"]);
    journey.configure_main(&forge);
    let source = journey.connect_main();
    // The definition's own connection at the file's host, made by reuse.
    let (output, _) =
        journey.on_terminal(&invoke(&forge, "project.get"), &[(REUSE_QUESTION, b"1\n")]);
    assert_eq!(success(output)["result"]["body"]["id"], 7);
    let own = journey.listed("definition");
    assert_eq!(own.len(), 1, "{own:?}");
    let left = own[0]["connection"].as_str().unwrap().to_owned();
    let left_revision = own[0]["revision"].as_str().unwrap().to_owned();
    let before = journey.describe("main", &source);
    let sent = fixture.requests().len();

    // The file now names another host: no reuse offered, one connect
    // question under a new instance id, the token, the result.
    journey.write("forge", MOVED_HOST, None, &["project.get"]);
    let token = format!("{TOKEN}\n");
    let (output, shown) = journey.on_terminal(
        &invoke(&forge, "project.get"),
        &[(QUESTION, b"y\n"), (TOKEN_PROMPT, token.as_bytes())],
    );
    let moved = success(output);
    assert_eq!(moved["result"]["body"]["id"], 7, "{moved}");
    let instance = moved["instance"].as_str().unwrap().to_owned();
    let digest = instance.strip_prefix("definition-").unwrap_or_default();
    assert!(
        digest.len() == 16 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
        "{moved}"
    );
    assert_eq!(
        (&moved["connector"], moved.get("adapter")),
        (&json!("forge"), None),
        "story:connectors-do-short-invocation: the answer names no alias: {moved}"
    );
    assert_eq!(count(&shown, REUSE_QUESTION), 0);
    assert_eq!(count(&shown, CANDIDATE), 0);
    assert_eq!(count(&shown, QUESTION), 1);
    assert_eq!(count(&shown, TOKEN_PROMPT), 1);
    assert_eq!(
        count(
            &shown,
            format!("No connection for forge/{instance}.").as_bytes()
        ),
        1,
        "{}",
        String::from_utf8_lossy(&shown)
    );
    // Only the new host was reached: the fresh connect's validation, then
    // the read.
    let requests = fixture.requests();
    assert!(
        requests.len() > sent
            && requests[sent..]
                .iter()
                .all(|r| r.host == MOVED_HOST && r.authenticated),
        "{requests:?}"
    );
    let fresh = journey.listed(&instance);
    assert_eq!(fresh.len(), 1, "{fresh:?}");
    assert_ne!(fresh[0]["connection"].as_str().unwrap(), left);

    // Repeated runs resolve to the same instance and ask nothing.
    let (output, shown) = journey.on_terminal(&invoke(&forge, "project.get"), &[]);
    let again = success(output);
    assert_eq!(again["instance"], instance.as_str(), "{again}");
    assert!(shown.is_empty(), "{}", String::from_utf8_lossy(&shown));
    assert_eq!(journey.listed(&instance), fresh);

    // Moved back, the file is the instance `definition` again, on the
    // connection it left behind, with nothing asked.
    journey.write("forge", HOST, None, &["project.get"]);
    let (output, shown) = journey.on_terminal(&invoke(&forge, "project.get"), &[]);
    let back = success(output);
    assert_eq!(back["instance"], "definition", "{back}");
    assert!(shown.is_empty(), "{}", String::from_utf8_lossy(&shown));
    assert_eq!(journey.listed("definition"), own);
    journey.write("forge", MOVED_HOST, None, &["project.get"]);
    let (output, _) = journey.on_terminal(&invoke(&forge, "project.get"), &[]);
    assert_eq!(success(output)["instance"], instance.as_str());

    // The connection left at the old host is unchanged and revocable
    // through the moved-entry path: its instance id no entry configures.
    let revoked = success(
        journey
            .command(&[
                "connections",
                "revoke",
                "--connector",
                "forge",
                "--instance",
                &instance,
                "--connection",
                &left,
                "--expected-revision",
                &left_revision,
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(revoked["local_state"], "revoked", "{revoked}");
    // The configured instance's connection, a candidate at the old host, was
    // not offered and is as it was; the new instance's connection is too.
    assert_eq!(
        unchanged(&journey.describe("main", &source)),
        unchanged(&before)
    );
    assert_eq!(journey.listed(&instance), fresh);
}

#[cfg(unix)]
#[path = "adversary_reuse_race.rs"]
mod adversary_reuse_race;
