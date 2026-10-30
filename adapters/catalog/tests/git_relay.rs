//! story:git-smart-http-relay-with-quarantine: a served instance relays git
//! smart-HTTP for the repositories it lists, so a caller with no forge
//! credential clones, fetches and pushes through it with a stock `git`.
//!
//! The served binary runs in a jail (`connectors_custody::mounted::fixture`)
//! as `served.rs` runs it: its credential is a mounted item, its caller is
//! admitted by a service account token that a loopback TokenReview double
//! reviews. A disposable HTTPS forge on loopback stands in for the forge: it
//! answers the TokenReview and serves bare repositories through
//! `git http-backend`, admitting only the instance's token in HTTP basic
//! authentication. Its root reaches the binary through `ca_file`. Every token
//! is fictional and only compared.
#![cfg(target_os = "linux")]
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
        atomic::{AtomicBool, Ordering},
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
        let shared = (
            seen.clone(),
            drop_push_answers.clone(),
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

type Shared = (Arc<Mutex<Vec<Seen>>>, Arc<AtomicBool>, PathBuf);

/// One request on one connection, answered with `Connection: close`.
async fn serve(
    stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    (seen, drop_push_answers, root): Shared,
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

/// Every request the forge saw for a repository carried the instance's
/// token, and none carried a projected token.
fn only_the_instance_token_reached_the_forge(forge: &Forge) {
    for seen in forge.git_seen() {
        assert_eq!(
            seen.authorization.as_deref(),
            Some(basic().as_str()),
            "{seen:?}"
        );
    }
    for seen in forge.seen() {
        let authorization = seen.authorization.unwrap_or_default();
        assert!(
            !authorization.contains(AGENT_TOKEN) && !authorization.contains(OTHER_TOKEN),
            "a projected token reached the forge"
        );
    }
}

// ---- the acceptance -----------------------------------------------------------------------------

/// A clone and a later fetch through the relay reach the forge only with the
/// instance's token, and bring the forge's commits.
#[test]
fn a_clone_and_fetch_through_the_relay_carry_the_installation_token() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    assert_eq!(
        fs::read_to_string(client.work().join("README.md")).unwrap(),
        "project\n"
    );
    assert_eq!(
        git(&client.work(), &["rev-parse", "HEAD"]),
        forge.head(PROJECT, "main").unwrap()
    );

    // The forge moves on; a fetch through the relay brings it.
    let seed = forge.root.path().join(format!("seed-{PROJECT}"));
    fs::write(seed.join("NEWS.md"), "news\n").unwrap();
    git(&seed, &["add", "NEWS.md"]);
    git(&seed, &["commit", "--quiet", "-m", "news"]);
    git(
        &seed,
        &[
            "push",
            "--quiet",
            forge.bare(PROJECT).to_str().unwrap(),
            "main",
        ],
    );
    let output = client.run(&client.work(), &["fetch", "--quiet", "origin"], &[]);
    assert!(output.status.success(), "fetch: {}", stderr(&output));
    assert_eq!(
        git(&client.work(), &["rev-parse", "origin/main"]),
        forge.head(PROJECT, "main").unwrap()
    );
    assert!(
        forge
            .git_seen()
            .iter()
            .any(|seen| seen.path.ends_with("/git-upload-pack"))
    );
    only_the_instance_token_reached_the_forge(&forge);
    assert_eq!(forge.pushes(), 0);
}

/// A push to a branch under the prefix, by any author when no identity is
/// configured, is sent to the forge once and reported applied.
#[test]
fn a_push_to_an_allowed_branch_is_relayed_once_and_reported_applied() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let commit = client.commit("change.md", b"change\n", SOMEONE, SOMEONE);
    let output = client.push("HEAD:refs/heads/agent/one");
    assert!(
        output.status.success(),
        "push: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/one"), Some(commit));
    assert_eq!(forge.pushes(), 1);
    assert!(served.log().contains("outcome=applied"), "{}", served.log());
    only_the_instance_token_reached_the_forge(&forge);
}

/// A push to a branch outside every prefix is refused before the relay
/// sends anything for it: no mirror fetch and no receive-pack request.
#[test]
fn a_push_outside_the_ref_prefix_is_refused_with_no_upstream_request() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let main = forge.head(PROJECT, "main");
    client.commit("change.md", b"change\n", ME, ME);
    let before = forge.git_seen().len();
    for refspec in [
        "HEAD:refs/heads/main",
        "HEAD:refs/heads/feature/x",
        "HEAD:refs/heads/agent",
    ] {
        let output = client.push(refspec);
        assert!(!output.status.success(), "{refspec} was pushed");
        assert!(
            both(&output).contains("is not under an allowed branch prefix"),
            "{refspec}: {}",
            both(&output)
        );
    }
    // Each push read only the advertisement.
    let after: Vec<_> = forge.git_seen().into_iter().skip(before).collect();
    assert!(
        after
            .iter()
            .all(|seen| seen.method == "GET" && seen.query == "service=git-receive-pack"),
        "{after:?}"
    );
    assert_eq!(forge.head(PROJECT, "main"), main);
    assert_eq!(forge.head(PROJECT, "refs/heads/feature/x"), None);
}

/// A ref delete and a tag push are refused before any forge request but the
/// advertisement, even under the prefix.
#[test]
fn a_ref_delete_or_a_tag_push_is_refused_with_no_upstream_request() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let commit = client.commit("change.md", b"change\n", ME, ME);
    let output = client.push("HEAD:refs/heads/agent/keep");
    assert!(
        output.status.success(),
        "push: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.pushes(), 1);

    let output = client.push(":refs/heads/agent/keep");
    assert!(!output.status.success());
    assert!(
        both(&output).contains("does not delete refs"),
        "{}",
        both(&output)
    );
    git(&client.work(), &["tag", "v1"]);
    let output = client.push("refs/tags/v1");
    assert!(!output.status.success());
    assert!(
        both(&output).contains("does not push tags"),
        "{}",
        both(&output)
    );
    assert_eq!(forge.pushes(), 1, "no refused push reached the forge");
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/keep"), Some(commit));
    assert_eq!(forge.head(PROJECT, "refs/tags/v1"), None);
}

/// With `commit_identity` configured, a new commit by another author or
/// another committer refuses the push; the relay only read the forge.
#[test]
fn a_commit_by_another_author_or_committer_is_refused_with_no_upstream_request() {
    let (forge, served) = started(|git| {
        git["commit_identity"] = json!({"name": IDENTITY_NAME, "email": IDENTITY_EMAIL});
    });
    for (author, committer) in [(SOMEONE, ME), (ME, SOMEONE)] {
        let client = Client::new(Some(AGENT_TOKEN));
        client.cloned(&served.url(PROJECT));
        client.commit("mine.md", b"mine\n", ME, ME);
        let theirs = client.commit("theirs.md", b"theirs\n", author, committer);
        let output = client.push("HEAD:refs/heads/agent/identity");
        assert!(!output.status.success(), "a foreign commit was pushed");
        assert!(
            both(&output).contains(&format!(
                "commit {theirs} is not authored and committed by the configured identity"
            )),
            "{}\n{}",
            both(&output),
            served.log()
        );
    }
    assert_eq!(forge.pushes(), 0);
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/identity"), None);

    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let commit = client.commit("mine.md", b"mine\n", ME, ME);
    let output = client.push("HEAD:refs/heads/agent/identity");
    assert!(
        output.status.success(),
        "push: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(
        forge.head(PROJECT, "refs/heads/agent/identity"),
        Some(commit)
    );
    assert_eq!(forge.pushes(), 1);
}

/// The fixture pre-push check: this test binary, run by the relay with
/// `CONNECTORS_GIT_REPOSITORY` set, refuses a push whose new tips hold
/// [`FORBIDDEN`]. In an ordinary test run it does nothing.
#[test]
fn pre_push_check_fixture() {
    if std::env::var_os("CONNECTORS_GIT_REPOSITORY").is_none() {
        return;
    }
    assert!(
        std::env::var_os("GIT_CONFIG_COUNT").is_none(),
        "no credential reaches the check"
    );
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

/// A push whose content the configured check refuses is not sent; one it
/// passes is.
#[test]
fn a_failing_pre_push_check_refuses_the_push_with_no_upstream_request() {
    let (forge, served) = started(|git| {
        git["pre_push_check"] = json!({
            "command": [CHECK, "--exact", "pre_push_check_fixture", "--nocapture", "--test-threads=1"],
            "timeout_seconds": 60,
        });
    });
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    client.commit(
        "leak.md",
        format!("a {FORBIDDEN} here\n").as_bytes(),
        ME,
        ME,
    );
    let output = client.push("HEAD:refs/heads/agent/check");
    assert!(!output.status.success(), "a refused push was sent");
    assert!(
        both(&output).contains("the pre-push check refused the push"),
        "{}\n{}",
        both(&output),
        served.log()
    );
    assert!(!both(&output).contains(FORBIDDEN));
    assert_eq!(forge.pushes(), 0);

    git(
        &client.work(),
        &["reset", "--quiet", "--hard", "origin/main"],
    );
    let commit = client.commit("clean.md", b"clean\n", ME, ME);
    let output = client.push("HEAD:refs/heads/agent/check");
    assert!(
        output.status.success(),
        "push: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/check"), Some(commit));
    assert_eq!(forge.pushes(), 1);
}

/// A pseudo-random, incompressible content of `length` bytes.
fn noise(length: usize) -> Vec<u8> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

/// A push larger than `push_limit_bytes` is refused and never sent; a small
/// one still is.
#[test]
fn a_push_over_the_buffer_bound_is_refused() {
    let (forge, served) = started(|git| {
        git["push_limit_bytes"] = json!(16 * 1024);
    });
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    client.commit("large.bin", &noise(256 * 1024), ME, ME);
    let output = client.push("HEAD:refs/heads/agent/large");
    assert!(!output.status.success(), "an over-bound push was sent");
    assert_eq!(forge.pushes(), 0);
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/large"), None);
    assert!(served.log().contains("over the bound"), "{}", served.log());

    git(
        &client.work(),
        &["reset", "--quiet", "--hard", "origin/main"],
    );
    let commit = client.commit("small.md", b"small\n", ME, ME);
    let output = client.push("HEAD:refs/heads/agent/small");
    assert!(
        output.status.success(),
        "push: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.head(PROJECT, "refs/heads/agent/small"), Some(commit));
}

/// The forge applies a push and its answer is lost: the relay reports the
/// outcome unknown and sends the push no second time.
#[test]
fn a_lost_upstream_answer_is_unknown_and_never_resent() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    let commit = client.commit("change.md", b"change\n", ME, ME);
    forge.drop_push_answers.store(true, Ordering::SeqCst);
    let output = client.push("HEAD:refs/heads/agent/lost");
    assert!(
        !output.status.success(),
        "a lost answer was reported applied"
    );
    assert!(
        both(&output).contains("the outcome is unknown"),
        "{}\n{}",
        both(&output),
        served.log()
    );
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(forge.pushes(), 1, "the push was sent once");
    assert_eq!(
        forge.head(PROJECT, "refs/heads/agent/lost"),
        Some(commit),
        "the forge applied it, which is why the outcome is unknown and not refused"
    );
    assert!(served.log().contains("outcome=unknown"), "{}", served.log());
}

/// Only the configured service account is admitted: no bearer, a token the
/// review does not authenticate, and another service account's token are
/// refused before any forge request for the repository.
#[test]
fn the_relay_admits_only_the_configured_principal() {
    let (forge, served) = started(|_| {});
    for bearer in [None, Some("fixture-unknown-token"), Some(OTHER_TOKEN)] {
        let client = Client::new(bearer);
        let output = client.clone(&served.url(PROJECT));
        assert!(!output.status.success(), "{bearer:?} was admitted");
    }
    assert!(forge.git_seen().is_empty(), "{:?}", forge.git_seen());
    let client = Client::new(Some(AGENT_TOKEN));
    client.cloned(&served.url(PROJECT));
    only_the_instance_token_reached_the_forge(&forge);
}

/// A repository the forge has but the configuration does not list is
/// `404`, with no forge request for it.
#[test]
fn a_repository_outside_the_configuration_is_not_found() {
    let (forge, served) = started(|_| {});
    let client = Client::new(Some(AGENT_TOKEN));
    let output = client.clone(&served.url(OTHER));
    assert!(
        !output.status.success(),
        "an unlisted repository was cloned"
    );
    assert!(stderr(&output).contains("not found"), "{}", stderr(&output));
    let output = client.clone(&format!("{}/git/{OWNER}/../{PROJECT}.git", served.base));
    assert!(!output.status.success());
    assert!(forge.git_seen().is_empty(), "{:?}", forge.git_seen());
}
