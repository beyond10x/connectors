//! Adversary pass on story:git-smart-http-relay-with-quarantine. The harness
//! (forge, jail, client) is copied from `git_relay.rs`; the forge here also
//! keeps each receive-pack body it was sent and can hold a push before applying
//! it.
#![cfg(target_os = "linux")]
#![allow(dead_code)]
use base64::{Engine as _, engine::general_purpose::STANDARD};
use connectors_custody::mounted::fixture::{Jail, Map};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead as _, BufReader, Read as _},
    os::unix::fs::{PermissionsExt as _, symlink},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};
use tokio::{
    io::{
        AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader as AsyncBufReader,
    },
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

/// The forge token in the mounted `gitlab.pat` document.
const TOKEN: &str = "fixture-forge-token";
/// The projected token of the admitted service account, and of another one.
const AGENT_TOKEN: &str = "fixture-projected-agent-token";
const OTHER_TOKEN: &str = "fixture-projected-other-token";
const AUDIENCE: &str = "connectors-forge";
const PRINCIPAL: &str = "system:serviceaccount:agents:coding-agent";
const OTHER_PRINCIPAL: &str = "system:serviceaccount:agents:other-agent";
const REVIEW_PATH: &str = "/apis/authentication.k8s.io/v1/tokenreviews";
/// The repository the instance relays, and one the forge has that it does not.
const OWNER: &str = "fixture-org";
const PROJECT: &str = "project";
const OTHER: &str = "other";
/// The identity a configured `commit_identity` admits.
const IDENTITY_NAME: &str = "Relay Fixture";
const IDENTITY_EMAIL: &str = "relay@fixture.invalid";
/// The literal the fixture pre-push check refuses.
const FORBIDDEN: &str = "fixture-forbidden-literal";
const SOURCE: &str = "/srv/secret";
const MOUNTED: &str = "/run/connectors/credential";
const BINARY: &str = "/bin-test/connectors-catalog-provider";
const CHECK: &str = "/bin-test/pre-push-check";
const CONFIG: &str = "/srv/served.json";

// ---- the forge --------------------------------------------------------------------------------

/// One request the forge answered.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    query: String,
    authorization: Option<String>,
}

impl Seen {
    fn git(&self) -> bool {
        self.path != REVIEW_PATH
    }
    fn receive_pack(&self) -> bool {
        self.method == "POST" && self.path.ends_with("/git-receive-pack")
    }
}

/// A forge on loopback HTTPS: the TokenReview double, and the bare
/// repositories under `root` through `git http-backend`.
struct Forge {
    port: u16,
    pem: String,
    root: tempfile::TempDir,
    seen: Arc<Mutex<Vec<Seen>>>,
    /// When set, a receive-pack request is applied and its answer dropped.
    drop_push_answers: Arc<AtomicBool>,
    /// Milliseconds a receive-pack request is held, after its body was read
    /// and before it is applied.
    hold_push_ms: Arc<AtomicU64>,
    /// The body of every receive-pack request, as the forge read it.
    bodies: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn basic() -> String {
    format!(
        "Basic {}",
        STANDARD.encode(format!("x-access-token:{TOKEN}"))
    )
}

fn review_answer(token: &str, audiences: &[String]) -> Value {
    let principal = match token {
        AGENT_TOKEN => Some(PRINCIPAL),
        OTHER_TOKEN => Some(OTHER_PRINCIPAL),
        _ => None,
    };
    let status = match principal {
        Some(principal) if audiences.iter().any(|a| a == AUDIENCE) => json!({
            "authenticated": true,
            "audiences": [AUDIENCE],
            "user": {"username": principal},
        }),
        _ => json!({}),
    };
    json!({"apiVersion": "authentication.k8s.io/v1", "kind": "TokenReview", "status": status})
}

impl Forge {
    fn new() -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = cert.cert.pem();
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
        let root = tempfile::Builder::new().prefix("forge-").tempdir().unwrap();
        let (address_tx, address_rx) = mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let drop_push_answers = Arc::new(AtomicBool::new(false));
        let hold_push_ms = Arc::new(AtomicU64::new(0));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let shared = (
            seen.clone(),
            drop_push_answers.clone(),
            hold_push_ms.clone(),
            bodies.clone(),
            root.path().to_owned(),
        );
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {
                        _ = &mut stopped => break,
                        accepted = listener.accept() => accepted.unwrap().0,
                    };
                    let acceptor = acceptor.clone();
                    let shared = shared.clone();
                    tokio::spawn(async move {
                        if let Ok(stream) = acceptor.accept(stream).await {
                            let _ = serve(stream, shared).await;
                        }
                    });
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let forge = Self {
            port: address.port(),
            pem,
            root,
            seen,
            drop_push_answers,
            hold_push_ms,
            bodies,
            stop: Some(stop),
            thread: Some(thread),
        };
        forge.repository(PROJECT);
        forge.repository(OTHER);
        forge
    }

    fn endpoint(&self) -> String {
        format!("https://localhost:{}", self.port)
    }

    /// The bare repository `<root>/<OWNER>/<name>.git`, with one commit on
    /// `main`.
    fn repository(&self, name: &str) {
        let bare = self.bare(name);
        fs::create_dir_all(&bare).unwrap();
        git(
            &bare,
            &["init", "--quiet", "--bare", "--initial-branch=main"],
        );
        git(&bare, &["config", "http.receivepack", "true"]);
        let work = self.root.path().join(format!("seed-{name}"));
        fs::create_dir_all(&work).unwrap();
        git(&work, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(work.join("README.md"), format!("{name}\n")).unwrap();
        git(&work, &["add", "README.md"]);
        git(&work, &["commit", "--quiet", "-m", "seed"]);
        git(&work, &["push", "--quiet", bare.to_str().unwrap(), "main"]);
    }

    fn bare(&self, name: &str) -> PathBuf {
        self.root.path().join(OWNER).join(format!("{name}.git"))
    }

    /// The object id of `reference` in the forge's repository, if any.
    fn head(&self, name: &str, reference: &str) -> Option<String> {
        let output = git_output(
            &self.bare(name),
            &["rev-parse", "--verify", "--quiet", reference],
        );
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }

    fn git_seen(&self) -> Vec<Seen> {
        self.seen().into_iter().filter(Seen::git).collect()
    }

    fn pushes(&self) -> usize {
        self.seen()
            .iter()
            .filter(|seen| seen.receive_pack())
            .count()
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        let _ = self.stop.take().map(|stop| stop.send(()));
        let _ = self.thread.take().map(std::thread::JoinHandle::join);
    }
}

type Shared = (
    Arc<Mutex<Vec<Seen>>>,
    Arc<AtomicBool>,
    Arc<AtomicU64>,
    Arc<Mutex<Vec<Vec<u8>>>>,
    PathBuf,
);

/// One request on one connection, answered with `Connection: close`.
async fn serve(
    stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    (seen, drop_push_answers, hold_push_ms, bodies, root): Shared,
) -> std::io::Result<()> {
    let mut stream = AsyncBufReader::new(stream);
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read_until(b'\n', &mut head).await? == 0 || head.len() > 65536 {
            return Ok(());
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let mut lines = head.split("\r\n");
    let mut words = lines.next().unwrap_or_default().split_whitespace();
    let method = words.next().unwrap_or_default().to_owned();
    let target = words.next().unwrap_or_default().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let header = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    if header("expect").is_some_and(|value| value.eq_ignore_ascii_case("100-continue")) {
        stream
            .get_mut()
            .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
            .await?;
    }
    let mut body = Vec::new();
    if header("transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked")) {
        loop {
            let mut line = Vec::new();
            stream.read_until(b'\n', &mut line).await?;
            let size = usize::from_str_radix(
                String::from_utf8_lossy(&line)
                    .trim()
                    .split(';')
                    .next()
                    .unwrap_or("0"),
                16,
            )
            .unwrap_or(0);
            if size == 0 {
                let mut rest = Vec::new();
                while stream.read_until(b'\n', &mut rest).await? > 2 {
                    rest.clear();
                }
                break;
            }
            let mut chunk = vec![0; size + 2];
            stream.read_exact(&mut chunk).await?;
            body.extend_from_slice(&chunk[..size]);
        }
    } else if let Some(length) = header("content-length").and_then(|value| value.parse().ok()) {
        body = vec![0; length];
        stream.read_exact(&mut body).await?;
    }
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let (path, query) = (path.to_owned(), query.to_owned());
    let authorization = header("authorization");
    seen.lock().unwrap().push(Seen {
        method: method.clone(),
        path: path.clone(),
        query: query.clone(),
        authorization: authorization.clone(),
    });
    let mut out = stream.into_inner();
    if path == REVIEW_PATH {
        let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        let token = request["spec"]["token"].as_str().unwrap_or_default();
        let audiences: Vec<String> =
            serde_json::from_value(request["spec"]["audiences"].clone()).unwrap_or_default();
        let answer = serde_json::to_vec(&review_answer(token, &audiences)).unwrap();
        let head = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            answer.len()
        );
        out.write_all(head.as_bytes()).await?;
        out.write_all(&answer).await?;
        return out.shutdown().await;
    }
    if authorization.as_deref() != Some(basic().as_str()) {
        out.write_all(
            b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"forge\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await?;
        return out.shutdown().await;
    }
    if method == "POST" && path.ends_with("/git-receive-pack") {
        bodies.lock().unwrap().push(body.clone());
        let hold = hold_push_ms.load(Ordering::SeqCst);
        if hold > 0 {
            tokio::time::sleep(Duration::from_millis(hold)).await;
        }
    }
    let mut backend = tokio::process::Command::new("git");
    backend
        .arg("http-backend")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_PROJECT_ROOT", &root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", &path)
        .env("QUERY_STRING", &query)
        .env("REQUEST_METHOD", &method)
        .env("REMOTE_USER", "x-access-token")
        .env("REMOTE_ADDR", "127.0.0.1")
        .env("CONTENT_LENGTH", body.len().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (name, variable) in [
        ("content-type", "CONTENT_TYPE"),
        ("content-encoding", "HTTP_CONTENT_ENCODING"),
        ("git-protocol", "HTTP_GIT_PROTOCOL"),
    ] {
        if let Some(value) = header(name) {
            backend.env(variable, value);
        }
    }
    let mut child = backend.spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    let feed = async move {
        let _ = stdin.write_all(&body).await;
        drop(stdin);
    };
    let (_, output) = tokio::join!(feed, child.wait_with_output());
    let output = output?.stdout;
    if method == "POST"
        && path.ends_with("/git-receive-pack")
        && drop_push_answers.load(Ordering::SeqCst)
    {
        // Applied, and the answer lost.
        return out.shutdown().await;
    }
    let split = output
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| (at, at + 4))
        .unwrap_or((output.len(), output.len()));
    let cgi = String::from_utf8_lossy(&output[..split.0]).into_owned();
    let mut status = "200 OK".to_owned();
    let mut response = String::new();
    for line in cgi.split("\r\n").filter(|line| !line.is_empty()) {
        match line.split_once(':') {
            Some((name, value)) if name.eq_ignore_ascii_case("status") => {
                status = value.trim().to_owned();
            }
            Some(_) => response.push_str(&format!("{line}\r\n")),
            None => {}
        }
    }
    let content = &output[split.1..];
    let head = format!(
        "HTTP/1.1 {status}\r\n{response}Content-Length: {}\r\nConnection: close\r\n\r\n",
        content.len()
    );
    out.write_all(head.as_bytes()).await?;
    out.write_all(content).await?;
    out.shutdown().await
}

// ---- git --------------------------------------------------------------------------------------

/// `git` in `directory` with no user or system configuration, committing as
/// `IDENTITY_NAME`.
fn git_command(directory: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(directory)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", directory)
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_AUTHOR_NAME", IDENTITY_NAME)
        .env("GIT_AUTHOR_EMAIL", IDENTITY_EMAIL)
        .env("GIT_COMMITTER_NAME", IDENTITY_NAME)
        .env("GIT_COMMITTER_EMAIL", IDENTITY_EMAIL)
        .stdin(Stdio::null());
    command
}

fn git_output(directory: &Path, args: &[&str]) -> Output {
    git_command(directory).args(args).output().unwrap()
}

fn git(directory: &Path, args: &[&str]) -> String {
    let output = git_output(directory, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A working copy of the client, outside the jail, that talks to the relay
/// with `bearer` as its only credential.
struct Client {
    directory: tempfile::TempDir,
    bearer: Option<&'static str>,
}

impl Client {
    fn new(bearer: Option<&'static str>) -> Self {
        Self {
            directory: tempfile::Builder::new()
                .prefix("client-")
                .tempdir()
                .unwrap(),
            bearer,
        }
    }

    fn work(&self) -> PathBuf {
        self.directory.path().join("work")
    }

    /// `git <args>` through the relay: `http.extraHeader` is how a stock git
    /// carries a bearer, as the caller's boundary adds it.
    fn run(&self, directory: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = git_command(directory);
        if let Some(bearer) = self.bearer {
            command.args([
                "-c",
                &format!("http.extraHeader=Authorization: Bearer {bearer}"),
            ]);
        }
        command.args(args);
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().unwrap()
    }

    fn clone(&self, url: &str) -> Output {
        self.run(
            self.directory.path(),
            &["clone", "--quiet", url, "work"],
            &[],
        )
    }

    fn cloned(&self, url: &str) {
        let output = self.clone(url);
        assert!(output.status.success(), "clone: {}", stderr(&output));
    }

    /// One commit writing `content` to `file`, under the given identities.
    fn commit(
        &self,
        file: &str,
        content: &[u8],
        author: (&str, &str),
        committer: (&str, &str),
    ) -> String {
        fs::write(self.work().join(file), content).unwrap();
        git(&self.work(), &["add", file]);
        let output = self.run(
            &self.work(),
            &["commit", "--quiet", "-m", file],
            &[
                ("GIT_AUTHOR_NAME", author.0),
                ("GIT_AUTHOR_EMAIL", author.1),
                ("GIT_COMMITTER_NAME", committer.0),
                ("GIT_COMMITTER_EMAIL", committer.1),
            ],
        );
        assert!(output.status.success(), "commit: {}", stderr(&output));
        git(&self.work(), &["rev-parse", "HEAD"])
    }

    fn push(&self, refspec: &str) -> Output {
        self.run(
            &self.work(),
            &["push", "--porcelain", "origin", refspec],
            &[],
        )
    }
}

const ME: (&str, &str) = (IDENTITY_NAME, IDENTITY_EMAIL);
const SOMEONE: (&str, &str) = ("Someone Else", "someone@fixture.invalid");

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn both(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---- the served instance ------------------------------------------------------------------------

/// The jail the served binary runs in, configured with `git` as its relay.
struct Served {
    jail: Jail,
    child: Option<Child>,
    base: String,
    log: Arc<Mutex<String>>,
}

impl Served {
    fn start(forge: &Forge, git: Value) -> Self {
        let mut jail = Jail::new();
        jail.directory("/srv", 0o755);
        jail.directory("/srv/state", 0o700);
        jail.directory("/srv/tmp", 0o700);
        jail.directory("/srv/work", 0o700);
        jail.directory(SOURCE, 0o755);
        jail.directory(MOUNTED, 0o755);
        jail.read_only(SOURCE, MOUNTED);
        jail.install(
            Path::new(env!("CARGO_BIN_EXE_connectors-catalog-provider")),
            BINARY,
        );
        if git.get("pre_push_check").is_some() {
            jail.install(&std::env::current_exe().unwrap(), CHECK);
        }
        let private = |inside: &str, bytes: &[u8], mode: u32| {
            let path = jail.path(inside);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        };
        private("/srv/ca.pem", forge.pem.as_bytes(), 0o644);
        private("/srv/review-token", b"fixture-instance-own-token", 0o600);
        let config = json!({
            "service": {
                "instance": "gitlab.fixture",
                "listen": "127.0.0.1:0",
                "state": "/srv/state",
                "admission": "service-account-token",
                "audience": AUDIENCE,
                "principal": PRINCIPAL,
                "token_review": {
                    "api": forge.endpoint(),
                    "ca_file": "/srv/ca.pem",
                    "credential_file": "/srv/review-token",
                },
            },
            "catalog": {"connector": "gitlab", "instance": "fixture", "endpoint": forge.endpoint()},
            "credential": {"binding": "mounted_secret", "directory": MOUNTED},
            "git": git,
        });
        private(CONFIG, &serde_json::to_vec(&config).unwrap(), 0o600);
        // The mounted item, laid out as the kubelet's atomic writer does.
        let source = jail.path(SOURCE);
        let stamp = "..2026_10_30_00_00_00.000000001";
        let item = source.join(stamp).join("gitlab.fixture").join("gitlab.pat");
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        for directory in [
            source.join(stamp),
            source.join(stamp).join("gitlab.fixture"),
        ] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(&item, serde_json::to_vec(&json!({"token": TOKEN})).unwrap()).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
        symlink(
            Path::new("..data").join("gitlab.fixture"),
            source.join("gitlab.fixture"),
        )
        .unwrap();
        symlink(stamp, source.join("..data")).unwrap();

        let mut served = Self {
            jail,
            child: None,
            base: String::new(),
            log: Arc::default(),
        };
        served.spawn();
        served
    }

    fn spawn(&mut self) {
        let mut attempts = 0;
        let mut child = loop {
            let spawned = self
                .jail
                .command(BINARY, Map::Own)
                .args(["--config", CONFIG])
                .env_clear()
                .env("PATH", "/usr/local/bin:/usr/bin:/bin")
                .env("TMPDIR", "/srv/tmp")
                .env("NO_COLOR", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();
            match spawned {
                Ok(child) => break child,
                Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) && attempts < 100 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        };
        let stderr = child.stderr.take().unwrap();
        let (lines_tx, lines_rx) = mpsc::channel();
        let log = self.log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                log.lock().unwrap().push_str(&format!("{line}\n"));
                let _ = lines_tx.send(line);
            }
        });
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(60) {
            match lines_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(line) if line.contains("adapter service listening") => {
                    let listen = line
                        .split_whitespace()
                        .find_map(|field| field.strip_prefix("listen="))
                        .expect("the listening line names its address");
                    self.base = format!("http://{listen}");
                    self.child = Some(child);
                    return;
                }
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let status = child.wait().unwrap();
                    panic!("the served binary exited {status}: {}", self.log());
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "the served binary did not listen within 60 s: {}",
            self.log()
        );
    }

    fn log(&self) -> String {
        self.log.lock().unwrap().clone()
    }

    fn url(&self, repository: &str) -> String {
        format!("{}/git/{OWNER}/{repository}.git", self.base)
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn relay_config(forge: &Forge) -> Value {
    json!({
        "upstream": forge.endpoint(),
        "ca_file": "/srv/ca.pem",
        "repositories": [format!("{OWNER}/{PROJECT}")],
        "ref_prefixes": ["agent/"],
        "work_directory": "/srv/work",
    })
}

fn started(git: impl FnOnce(&mut Value)) -> (Forge, Served) {
    let forge = Forge::new();
    let mut config = relay_config(&forge);
    git(&mut config);
    let served = Served::start(&forge, config);
    (forge, served)
}

// ---- raw requests -------------------------------------------------------------------------------

const ZERO: &str = "0000000000000000000000000000000000000000";
const RECEIVE_REQUEST: &str = "application/x-git-receive-pack-request";

fn pkt(data: &[u8]) -> Vec<u8> {
    let mut line = format!("{:04x}", data.len() + 4).into_bytes();
    line.extend_from_slice(data);
    line
}

/// A receive-pack request: the commands, the first with the capabilities a
/// stock git sends, a flush, then `pack`.
fn receive_request(commands: &[(&str, &str, &str)], pack: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (index, (old, new, name)) in commands.iter().enumerate() {
        let line = if index == 0 {
            format!("{old} {new} {name}\0report-status side-band-64k agent=git/2.55.0\n")
        } else {
            format!("{old} {new} {name}\n")
        };
        body.extend(pkt(line.as_bytes()));
    }
    body.extend_from_slice(b"0000");
    body.extend_from_slice(pack);
    body
}

fn connect(served: &Served) -> std::net::TcpStream {
    let address = served.base.strip_prefix("http://").unwrap();
    let stream = std::net::TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(180)))
        .unwrap();
    stream
}

fn send(stream: &mut std::net::TcpStream, path: &str, encoding: Option<&str>, body: &[u8]) {
    use std::io::Write as _;
    let encoding = encoding
        .map(|value| format!("Content-Encoding: {value}\r\n"))
        .unwrap_or_default();
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: relay\r\nAuthorization: Bearer {AGENT_TOKEN}\r\nUser-Agent: git/2.55.0\r\nContent-Type: {RECEIVE_REQUEST}\r\n{encoding}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}

/// The status and the (de-chunked) body of the answer on `stream`.
fn answer(mut stream: std::net::TcpStream) -> (u16, Vec<u8>) {
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("an HTTP answer");
    let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let mut body = raw[split + 4..].to_vec();
    if head.contains("transfer-encoding: chunked") {
        let mut plain = Vec::new();
        let mut at = 0;
        while let Some(end) = body[at..].windows(2).position(|w| w == b"\r\n") {
            let size =
                usize::from_str_radix(String::from_utf8_lossy(&body[at..at + end]).trim(), 16)
                    .unwrap_or(0);
            if size == 0 {
                break;
            }
            let start = at + end + 2;
            plain.extend_from_slice(&body[start..start + size]);
            at = start + size + 2;
        }
        body = plain;
    }
    (status, body)
}

fn post(served: &Served, path: &str, encoding: Option<&str>, body: &[u8]) -> (u16, String) {
    let mut stream = connect(served);
    send(&mut stream, path, encoding, body);
    let (status, body) = answer(stream);
    (status, String::from_utf8_lossy(&body).into_owned())
}

fn receive_path() -> String {
    format!("/git/{OWNER}/{PROJECT}.git/git-receive-pack")
}

/// `git <args>` in `directory` with `input` on its standard input.
fn git_with_input(directory: &Path, args: &[&str], env: &[(&str, &str)], input: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut command = git_command(directory);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// The ids of every object `tip` reaches and `not` does not.
fn objects(directory: &Path, tip: &str, not: &str) -> Vec<String> {
    git(directory, &["rev-list", "--objects", tip, "--not", not])
        .lines()
        .filter_map(|line| line.split(' ').next())
        .map(str::to_owned)
        .collect()
}

/// A pack of exactly `objects`, as `git pack-objects` writes it.
fn pack(directory: &Path, objects: &[String]) -> Vec<u8> {
    let list = objects
        .iter()
        .map(|id| format!("{id}\n"))
        .collect::<String>();
    git_with_input(
        directory,
        &["pack-objects", "--stdout", "-q"],
        &[],
        list.as_bytes(),
    )
}

fn forge_has(forge: &Forge, object: &str) -> bool {
    git_output(&forge.bare(PROJECT), &["cat-file", "-e", object])
        .status
        .success()
}

fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let started = std::time::Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "{what} within 60 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

// ---- pre-push check fixtures --------------------------------------------------------------------

/// The scanning check: refuses a push whose new tips hold [`FORBIDDEN`], as
/// `git_relay.rs`'s fixture does. In an ordinary test run it does nothing.
#[test]
fn fixture_scan_check() {
    if std::env::var_os("CONNECTORS_GIT_REPOSITORY").is_none() {
        return;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    for line in input.lines() {
        let new = line.split(' ').nth(1).unwrap();
        let found = Command::new("git")
            .args(["grep", "--quiet", "--fixed-strings", FORBIDDEN, new])
            .status()
            .unwrap()
            .success();
        if found {
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

fn check(fixture: &str) -> Value {
    json!({
        "command": [CHECK, "--exact", fixture, "--nocapture", "--test-threads=1"],
        "timeout_seconds": 60,
    })
}

// ---- the attacks --------------------------------------------------------------------------------

/// A pack may carry objects no command's new tip reaches. The quarantine only
/// asks that the tips be connected, the identity check and the pre-push check
/// only see what the tips reach, and the original request goes to the forge,
/// whose receive-pack keeps every object of the pack. So a blob the pre-push
/// check would refuse, and a commit by another identity, land in the forge's
/// object store, readable there by id.
#[test]
fn an_object_no_pushed_ref_reaches_never_reaches_the_forge() {
    let (forge, served) = started(|git| {
        git["commit_identity"] = json!({"name": IDENTITY_NAME, "email": IDENTITY_EMAIL});
        git["pre_push_check"] = check("fixture_scan_check");
    });
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let work = client.work();
    let tip = client.commit("clean.md", b"clean\n", ME, ME);
    let blob = String::from_utf8(git_with_input(
        &work,
        &["hash-object", "-w", "--stdin"],
        &[],
        format!("a {FORBIDDEN} here\n").as_bytes(),
    ))
    .unwrap()
    .trim()
    .to_owned();
    let tree = git(&work, &["rev-parse", "origin/main^{tree}"]);
    let foreign = String::from_utf8(git_with_input(
        &work,
        &["commit-tree", &tree, "-p", "origin/main", "-F", "-"],
        &[
            ("GIT_AUTHOR_NAME", SOMEONE.0),
            ("GIT_AUTHOR_EMAIL", SOMEONE.1),
            ("GIT_COMMITTER_NAME", SOMEONE.0),
            ("GIT_COMMITTER_EMAIL", SOMEONE.1),
        ],
        b"foreign\n",
    ))
    .unwrap()
    .trim()
    .to_owned();
    let mut carried = objects(&work, &tip, "origin/main");
    carried.push(blob.clone());
    carried.push(foreign.clone());
    let body = receive_request(
        &[(ZERO, &tip, "refs/heads/agent/smuggle")],
        &pack(&work, &carried),
    );
    let (status, answer) = post(&served, &receive_path(), None, &body);
    let delivered: Vec<&str> = [blob.as_str(), foreign.as_str()]
        .into_iter()
        .filter(|object| forge_has(&forge, object))
        .collect();
    assert!(
        delivered.is_empty(),
        "objects no pushed ref reaches, unseen by the identity and pre-push checks, \
         reached the forge: {delivered:?} (blob {blob} holds the refused literal, commit \
         {foreign} is by another identity); relay answered {status}: {answer:?}; forge \
         agent/smuggle = {:?}",
        forge.head(PROJECT, "refs/heads/agent/smuggle")
    );
}

/// The relay passes the forge's advertisement through unchanged, so a stock
/// git sends push options whenever the forge advertises them, and the relay
/// sends them on. A forge acts on them beside the ref update (a merge request
/// into `main` set to merge itself, a skipped pipeline), which no ref policy
/// sees and the pre-push check is not told about.
#[test]
fn push_options_never_reach_the_forge() {
    let (forge, served) = started(|_| {});
    git(
        &forge.bare(PROJECT),
        &["config", "receive.advertisePushOptions", "true"],
    );
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    client.commit("change.md", b"change\n", ME, ME);
    let output = client.run(
        &client.work(),
        &[
            "push",
            "--porcelain",
            "-o",
            "merge_request.create",
            "-o",
            "merge_request.target=main",
            "-o",
            "merge_request.merge_when_pipeline_succeeds",
            "-o",
            "ci.skip",
            "origin",
            "HEAD:refs/heads/agent/options",
        ],
        &[],
    );
    let bodies = forge.bodies.lock().unwrap().clone();
    let carried = bodies.iter().any(|body| {
        body.windows(b"merge_request.target=main".len())
            .any(|window| window == b"merge_request.target=main")
    });
    assert!(
        !carried,
        "the caller's push options reached the forge's receive-pack ({} request(s)); push: {}",
        bodies.len(),
        both(&output)
    );
}

/// The caller hangs up while the forge is applying its push. The forge has the
/// whole request and applies it; the relay must still log the command, as
/// `unknown` at least, or an applied update leaves no record.
#[test]
fn a_push_the_forge_applies_after_the_caller_hung_up_is_still_logged() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let work = client.work();
    let tip = client.commit("change.md", b"change\n", ME, ME);
    let body = receive_request(
        &[(ZERO, &tip, "refs/heads/agent/hangup")],
        &pack(&work, &objects(&work, &tip, "origin/main")),
    );
    forge.hold_push_ms.store(3000, Ordering::SeqCst);
    let mut stream = connect(&served);
    send(&mut stream, &receive_path(), None, &body);
    eventually("the forge reads the relayed push", || forge.pushes() == 1);
    let _ = stream.shutdown(std::net::Shutdown::Both);
    drop(stream);
    eventually("the forge applies the push", || {
        forge.head(PROJECT, "refs/heads/agent/hangup").as_deref() == Some(tip.as_str())
    });
    std::thread::sleep(Duration::from_secs(3));
    let log = served.log();
    assert!(
        log.lines()
            .any(|line| line.contains("refs/heads/agent/hangup") && line.contains("outcome=")),
        "the forge applied refs/heads/agent/hangup and the relay logged no outcome for it:\n{log}"
    );
}

/// Ref-name, mixed-command, path and inflation tricks: every one is refused,
/// none reaches the forge's receive-pack.
#[test]
fn ref_path_and_encoding_tricks_never_reach_the_forge() {
    let (forge, served) = started(|git| {
        git["push_limit_bytes"] = json!(64 * 1024);
    });
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let work = client.work();
    let main = forge.head(PROJECT, "main").unwrap();
    let tip = client.commit("change.md", b"change\n", ME, ME);
    let pack = pack(&work, &objects(&work, &tip, "origin/main"));
    let single = |name: &'static str| vec![(ZERO, tip.as_str(), name)];
    let cases: Vec<Vec<(&str, &str, &str)>> = vec![
        single("refs/heads/agent/../main"),
        single("refs/heads/agentX/one"),
        single("refs/heads/AGENT/one"),
        single("refs/heads/agent/one.lock"),
        single("refs/heads/agent/one/"),
        single("refs/heads/agent"),
        single("refs/heads/agent/one refs/heads/main"),
        single("refs/remotes/agent/one"),
        single("refs/heads/agent/one\u{7f}"),
        vec![
            (ZERO, tip.as_str(), "refs/heads/agent/ok"),
            (main.as_str(), tip.as_str(), "refs/heads/main"),
        ],
        vec![
            (main.as_str(), tip.as_str(), "refs/heads/main"),
            (ZERO, tip.as_str(), "refs/heads/agent/ok"),
        ],
        vec![
            (ZERO, tip.as_str(), "refs/heads/agent/ok"),
            (ZERO, tip.as_str(), "refs/tags/agent/v1"),
        ],
    ];
    for commands in &cases {
        let (status, answer) = post(
            &served,
            &receive_path(),
            None,
            &receive_request(commands, &pack),
        );
        assert!(
            status != 200 || answer.contains("ng "),
            "{commands:?}: {status} {answer:?}"
        );
    }
    for path in [
        format!("/git/{OWNER}/{PROJECT}.git.git/git-receive-pack"),
        format!("/git/{OWNER}/%2e%2e/git-receive-pack"),
        format!("/git/{OWNER}/{OTHER}.git/git-receive-pack"),
    ] {
        let body = receive_request(&single("refs/heads/agent/path"), &pack);
        let (status, answer) = post(&served, &path, None, &body);
        assert_eq!(status, 404, "{path}: {answer:?}");
    }
    // A gzip body far below the bound that inflates past it.
    let mut padded = receive_request(&single("refs/heads/agent/bomb"), &pack);
    padded.extend(std::iter::repeat_n(0_u8, 8 * 1024 * 1024));
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    std::io::Write::write_all(&mut encoder, &padded).unwrap();
    let compressed = encoder.finish().unwrap();
    assert!(compressed.len() < 64 * 1024);
    let (status, answer) = post(&served, &receive_path(), Some("gzip"), &compressed);
    assert_eq!(status, 413, "{answer:?}");

    assert_eq!(forge.pushes(), 0, "{:?}", forge.git_seen());
    assert_eq!(forge.head(PROJECT, "main"), Some(main));
    for name in [
        "refs/heads/agent/ok",
        "refs/heads/agent/bomb",
        "refs/heads/agent/path",
    ] {
        assert_eq!(forge.head(PROJECT, name), None, "{name}");
    }
    assert!(
        !forge.bare(OTHER).join("refs/heads/agent").exists(),
        "the unlisted repository was written"
    );
}
