//! Conformance checks of the git relay's safety invariants
//! (docs/running-services.md, "Git relay"), written by the unit's security
//! review. Each case drives the served binary as `git_relay.rs` does: in a
//! jail, admitted by a service account token that a loopback TokenReview
//! double reviews, against a disposable HTTPS forge on loopback running
//! `git http-backend`. Every token is fictional and only compared.
//!
//! - The forge token reaches no host but the configured upstream, also when
//!   the upstream answers the mirror fetch with a redirect to another host.
//! - `commit_identity` holds for every new commit, whatever bytes its ident
//!   lines carry.
//! - A pre-push check that passes its timeout leaves nothing running.
//! - `work_directory` is private once the instance has started.
#![cfg(target_os = "linux")]
use base64::{Engine as _, engine::general_purpose::STANDARD};
use connectors_custody::mounted::fixture::{Jail, Map};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead as _, BufReader, Write as _},
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

const TOKEN: &str = "fixture-forge-token";
const AGENT_TOKEN: &str = "fixture-projected-agent-token";
const AUDIENCE: &str = "connectors-forge";
const PRINCIPAL: &str = "system:serviceaccount:agents:coding-agent";
const REVIEW_PATH: &str = "/apis/authentication.k8s.io/v1/tokenreviews";
const OWNER: &str = "fixture-org";
const PROJECT: &str = "project";
const IDENTITY_NAME: &str = "Relay Fixture";
const IDENTITY_EMAIL: &str = "relay@fixture.invalid";
const SOURCE: &str = "/srv/secret";
const MOUNTED: &str = "/run/connectors/credential";
const BINARY: &str = "/bin-test/connectors-catalog-provider";
const CONFIG: &str = "/srv/served.json";
/// The host name the forge's certificate also covers, standing for a host
/// other than the configured upstream (`localhost`).
const ELSEWHERE: &str = "127.0.0.1";

// ---- the forge --------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Seen {
    method: String,
    host: String,
    path: String,
    authorization: Option<String>,
}

struct Forge {
    port: u16,
    pem: String,
    root: tempfile::TempDir,
    seen: Arc<Mutex<Vec<Seen>>>,
    /// When set, the upstream (`localhost`) answers an upload-pack
    /// advertisement with a redirect to the same path on [`ELSEWHERE`].
    redirect_upload_pack: Arc<AtomicBool>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn basic() -> String {
    format!(
        "Basic {}",
        STANDARD.encode(format!("x-access-token:{TOKEN}"))
    )
}

impl Forge {
    fn new() -> Self {
        let cert =
            rcgen::generate_simple_self_signed(vec!["localhost".into(), ELSEWHERE.into()]).unwrap();
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
        let redirect_upload_pack = Arc::new(AtomicBool::new(false));
        let shared = (
            seen.clone(),
            redirect_upload_pack.clone(),
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
                let port = listener.local_addr().unwrap().port();
                address_tx.send(port).unwrap();
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
                            let _ = serve(stream, port, shared).await;
                        }
                    });
                }
            });
        });
        let port = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let forge = Self {
            port,
            pem,
            root,
            seen,
            redirect_upload_pack,
            stop: Some(stop),
            thread: Some(thread),
        };
        forge.repository(PROJECT);
        forge
    }

    fn endpoint(&self) -> String {
        format!("https://localhost:{}", self.port)
    }

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

    fn pushes(&self) -> usize {
        self.seen()
            .iter()
            .filter(|seen| seen.method == "POST" && seen.path.ends_with("/git-receive-pack"))
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

fn review_answer(token: &str, audiences: &[String]) -> Value {
    let status = if token == AGENT_TOKEN && audiences.iter().any(|a| a == AUDIENCE) {
        json!({"authenticated": true, "audiences": [AUDIENCE], "user": {"username": PRINCIPAL}})
    } else {
        json!({})
    };
    json!({"apiVersion": "authentication.k8s.io/v1", "kind": "TokenReview", "status": status})
}

/// One request on one connection. A request addressed to [`ELSEWHERE`] is
/// a host other than the upstream: it is served without authentication and
/// recorded with whatever `Authorization` it carried.
async fn serve(
    stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    port: u16,
    (seen, redirect_upload_pack, root): Shared,
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
    let host = header("host").unwrap_or_default();
    let elsewhere = host.starts_with(ELSEWHERE);
    let authorization = header("authorization");
    seen.lock().unwrap().push(Seen {
        method: method.clone(),
        host: host.clone(),
        path: path.clone(),
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
    if !elsewhere
        && redirect_upload_pack.load(Ordering::SeqCst)
        && method == "GET"
        && query == "service=git-upload-pack"
    {
        let head = format!(
            "HTTP/1.1 302 Found\r\nLocation: https://{ELSEWHERE}:{port}{path}?{query}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        out.write_all(head.as_bytes()).await?;
        return out.shutdown().await;
    }
    if !elsewhere && authorization.as_deref() != Some(basic().as_str()) {
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

/// A working copy that talks to the relay with the admitted bearer only.
struct Client {
    directory: tempfile::TempDir,
}

impl Client {
    fn new() -> Self {
        Self {
            directory: tempfile::Builder::new()
                .prefix("client-")
                .tempdir()
                .unwrap(),
        }
    }

    fn work(&self) -> PathBuf {
        self.directory.path().join("work")
    }

    fn run(&self, directory: &Path, args: &[&str]) -> Output {
        let mut command = git_command(directory);
        command.args([
            "-c",
            &format!("http.extraHeader=Authorization: Bearer {AGENT_TOKEN}"),
        ]);
        command.args(args);
        command.output().unwrap()
    }

    /// A clone straight from the forge's bare repository, with `relay` as
    /// `origin`: nothing of it passes the relay.
    fn local_clone(&self, forge: &Forge, relay: &str) {
        git(
            self.directory.path(),
            &[
                "clone",
                "--quiet",
                forge.bare(PROJECT).to_str().unwrap(),
                "work",
            ],
        );
        git(&self.work(), &["remote", "set-url", "origin", relay]);
    }

    fn commit(&self, file: &str, content: &[u8]) -> String {
        fs::write(self.work().join(file), content).unwrap();
        git(&self.work(), &["add", file]);
        git(&self.work(), &["commit", "--quiet", "-m", file]);
        git(&self.work(), &["rev-parse", "HEAD"])
    }

    fn push(&self, refspec: &str) -> Output {
        self.run(&self.work(), &["push", "--porcelain", "origin", refspec])
    }
}

fn both(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---- the served instance ------------------------------------------------------------------------

struct Served {
    jail: Jail,
    child: Option<Child>,
    base: String,
    log: Arc<Mutex<String>>,
}

impl Served {
    fn start(forge: &Forge, git: Value, work_mode: u32) -> Self {
        let mut jail = Jail::new();
        jail.directory("/srv", 0o755);
        jail.directory("/srv/state", 0o700);
        jail.directory("/srv/tmp", 0o700);
        jail.directory("/srv/work", work_mode);
        jail.directory(SOURCE, 0o755);
        jail.directory(MOUNTED, 0o755);
        jail.read_only(SOURCE, MOUNTED);
        jail.install(
            Path::new(env!("CARGO_BIN_EXE_connectors-catalog-provider")),
            BINARY,
        );
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

    fn url(&self) -> String {
        format!("{}/git/{OWNER}/{PROJECT}.git", self.base)
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

fn started(git: impl FnOnce(&mut Value), work_mode: u32) -> (Forge, Served) {
    let forge = Forge::new();
    let mut config = json!({
        "upstream": forge.endpoint(),
        "ca_file": "/srv/ca.pem",
        "repositories": [format!("{OWNER}/{PROJECT}")],
        "ref_prefixes": ["agent/"],
        "work_directory": "/srv/work",
    });
    git(&mut config);
    let served = Served::start(&forge, config, work_mode);
    (forge, served)
}

// ---- the checks ---------------------------------------------------------------------------------

/// The forge token travels to the configured upstream only. The relay's own
/// forge client follows no redirect; the mirror fetch a push starts must not
/// either, or the upstream's redirect to another host carries the token
/// (git's `http.extraHeader`) to that host on the fetch's next request.
#[test]
fn the_mirror_fetch_sends_the_forge_token_to_no_other_host() {
    let (forge, served) = started(|_| {}, 0o700);
    let client = Client::new();
    client.local_clone(&forge, &served.url());
    client.commit("change.md", b"change\n");
    forge.redirect_upload_pack.store(true, Ordering::SeqCst);
    let output = client.push("HEAD:refs/heads/agent/redirect");
    let elsewhere: Vec<Seen> = forge
        .seen()
        .into_iter()
        .filter(|seen| seen.host.starts_with(ELSEWHERE))
        .collect();
    assert!(
        elsewhere
            .iter()
            .all(|seen| seen.authorization.as_deref() != Some(basic().as_str())),
        "the forge token reached a host other than the upstream: {elsewhere:?}\n{}\n{}",
        both(&output),
        served.log()
    );
    assert!(
        elsewhere.is_empty(),
        "the mirror fetch followed the upstream's redirect to another host: {elsewhere:?}"
    );
}

/// `commit_identity` admits a commit only when its author and committer are
/// exactly the configured name and email. A commit whose author name embeds
/// the bytes the relay separates `git log` fields and records with (0x1f and
/// 0x1e, both valid in a git ident) is still by another author.
#[test]
fn commit_identity_holds_for_an_ident_carrying_field_separators() {
    let (forge, served) = started(
        |git| {
            git["commit_identity"] = json!({"name": IDENTITY_NAME, "email": IDENTITY_EMAIL});
        },
        0o700,
    );
    let client = Client::new();
    client.local_clone(&forge, &served.url());
    let parent = git(&client.work(), &["rev-parse", "HEAD"]);
    let tree = git(&client.work(), &["rev-parse", "HEAD^{tree}"]);
    let author = format!(
        "{IDENTITY_NAME}\x1f{IDENTITY_EMAIL}\x1f{IDENTITY_NAME}\x1f{IDENTITY_EMAIL}\x1eMallory\x1f{IDENTITY_NAME}"
    );
    assert_ne!(author, IDENTITY_NAME);
    let object = format!(
        "tree {tree}\nparent {parent}\nauthor {author} <{IDENTITY_EMAIL}> 1700000000 +0000\ncommitter {IDENTITY_NAME} <{IDENTITY_EMAIL}> 1700000000 +0000\n\nforeign\n"
    );
    let mut hash = git_command(&client.work())
        .args(["hash-object", "-t", "commit", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    hash.stdin
        .take()
        .unwrap()
        .write_all(object.as_bytes())
        .unwrap();
    let hashed = hash.wait_with_output().unwrap();
    assert!(
        hashed.status.success(),
        "hash-object: {}",
        String::from_utf8_lossy(&hashed.stderr)
    );
    let commit = String::from_utf8_lossy(&hashed.stdout).trim().to_owned();
    git(&client.work(), &["update-ref", "HEAD", &commit]);
    let recorded = git(&client.work(), &["log", "-1", "--format=%an", &commit]);
    assert_ne!(recorded, IDENTITY_NAME, "the commit is by another author");

    let output = client.push("HEAD:refs/heads/agent/separators");
    assert_eq!(
        forge.head(PROJECT, "refs/heads/agent/separators"),
        None,
        "a commit by another author passed commit_identity and reached the forge: {}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.pushes(), 0);
}

/// A pre-push check that passes its timeout refuses the push, and nothing it
/// started keeps running: a process the check spawned must not outlive the
/// refusal (it would still run over the removed quarantine, and each timed-out
/// push would leave one more behind once the push slot is free again).
#[test]
fn a_timed_out_pre_push_check_leaves_no_process_behind() {
    let (forge, served) = started(
        |git| {
            git["pre_push_check"] = json!({
                "command": [
                    "/bin/sh",
                    "-c",
                    "(sleep 3; : > /srv/work/survived) </dev/null >/dev/null 2>&1 & sleep 8",
                ],
                "timeout_seconds": 1,
            });
        },
        0o700,
    );
    let client = Client::new();
    client.local_clone(&forge, &served.url());
    client.commit("slow.md", b"slow\n");
    let output = client.push("HEAD:refs/heads/agent/slow");
    assert!(
        both(&output).contains("did not finish in time"),
        "{}\n{}",
        both(&output),
        served.log()
    );
    assert_eq!(forge.pushes(), 0);
    std::thread::sleep(Duration::from_secs(5));
    assert!(
        !served.jail.path("/srv/work/survived").exists(),
        "a process the timed-out pre-push check started kept running after the refusal"
    );
}

/// `work_directory` is the instance's private directory once it has started,
/// whatever mode it had before.
#[test]
fn the_work_directory_is_private_after_start() {
    let (_forge, served) = started(|_| {}, 0o755);
    let mode = fs::metadata(served.jail.path("/srv/work"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700, "work_directory is {mode:o} after start");
}
