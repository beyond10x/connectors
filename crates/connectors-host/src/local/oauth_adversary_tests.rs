//! Adversary cases for story:cli-oauth-loopback-acquisition.
//!
//! Each case drives the public `oauth::entry` in its own process (the one the
//! CLI calls), reads the consent address it prints on standard error, and
//! plays a browser against the loopback listener. The listener's answer to
//! the redirect decides: it is written before the code exchange, so a `200`
//! means the redirect was accepted and a `400` or silence means it was not.
use super::{
    oauth, registry,
    runtime::{Acquisition, EntryField, Profile},
};
use connectors_sdk::Secret;
use std::{
    io::{Read, Write},
    net::TcpStream,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use url::Url;

const FIXTURE: &str = "CONNECTORS_OAUTH_ADVERSARY_FIXTURE";
// Loopback on the default https port: nothing is contacted off this machine.
const AUTHORIZE: &str = "https://127.0.0.1/o/oauth2/auth";
const TOKEN: &str = "https://127.0.0.1/token";
/// The flow's deadline in the fixture: the capture expires this long after
/// the reserve the flow leaves for completion.
const FLOW_MS: u64 = 10_000;
const RESERVE_MS: u64 = 45_000;

fn profile() -> Profile {
    Profile {
        id: "fixture.oauth".into(),
        revision: "fixture-revision".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: Default::default(),
        evidence_lifetime_ms: 60_000,
        fields: ["client_id", "client_secret", "refresh_token"]
            .iter()
            .map(|name| EntryField {
                name: (*name).into(),
                label: (*name).into(),
                max_bytes: 8192,
            })
            .collect(),
        acquisition: Some(Acquisition {
            authorize_url: AUTHORIZE.into(),
            token_url: TOKEN.into(),
            scopes: ["https://www.googleapis.com/auth/drive.readonly".to_owned()].into(),
        }),
    }
}

fn google() -> Secret {
    Secret(
        serde_json::to_vec(&serde_json::json!({"installed": {
            "client_id": "fixture-client.apps.example.test",
            "project_id": "fixture-project",
            "auth_uri": AUTHORIZE,
            "token_uri": TOKEN,
            "client_secret": "fixture-client-secret",
            "redirect_uris": ["http://localhost"],
        }}))
        .unwrap(),
    )
}

// Re-entered in its own process by the cases below; a no-op otherwise.
#[test]
fn adversary_flow_fixture() {
    if std::env::var_os(FIXTURE).is_none() {
        return;
    }
    let expires = connectors_sdk::now_ms() + RESERVE_MS + FLOW_MS;
    let _ = oauth::entry(&profile(), google(), expires);
}

struct Flow {
    child: Child,
    port: u16,
    state: String,
    _root: tempfile::TempDir,
}
impl Drop for Flow {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn consent_address(stderr: &Path) -> Option<Url> {
    let text = std::fs::read_to_string(stderr).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("connectors: consent-url "))
        .and_then(|line| Url::parse(line).ok())
}

fn param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// Starts the flow in a fresh session, so it has no terminal and prints its
/// consent address on standard error, and waits for that address.
fn start() -> Flow {
    let root = tempfile::tempdir().unwrap();
    let stderr: PathBuf = root.path().join("stderr");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "local::oauth_adversary_tests::adversary_flow_fixture",
            "--nocapture",
        ])
        .env(FIXTURE, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&stderr).unwrap());
    // SAFETY: setsid is async-signal-safe and touches only the child.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let url = loop {
        if let Some(url) = consent_address(&stderr) {
            break url;
        }
        assert!(Instant::now() < until, "the flow never showed consent");
        assert!(child.try_wait().unwrap().is_none(), "fixture ended early");
        std::thread::sleep(Duration::from_millis(10));
    };
    let redirect = Url::parse(&param(&url, "redirect_uri").unwrap()).unwrap();
    Flow {
        child,
        port: redirect.port().unwrap(),
        state: param(&url, "state").unwrap(),
        _root: root,
    }
}

fn connect(flow: &Flow) -> TcpStream {
    TcpStream::connect(("127.0.0.1", flow.port)).unwrap()
}

fn redirect_head(flow: &Flow) -> String {
    format!(
        "GET /?state={}&code=fixture-code&scope=openid HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        flow.state, flow.port
    )
}

/// The listener's whole answer on `stream`, or what arrived within `wait`.
fn answer(stream: &mut TcpStream, wait: Duration) -> String {
    stream.set_read_timeout(Some(wait)).unwrap();
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A connection that opens and sends nothing, as a browser's speculative
/// preconnect does, accepted before the real redirect: the listener must
/// still answer the redirect well inside the flow's deadline.
#[test]
fn an_idle_connection_first_does_not_hide_the_redirect() {
    let flow = start();
    let _idle = connect(&flow);
    // Past several accept polls, so the idle connection is the one accepted.
    std::thread::sleep(Duration::from_millis(400));
    let mut browser = connect(&flow);
    browser.write_all(redirect_head(&flow).as_bytes()).unwrap();
    let started = Instant::now();
    let answered = answer(&mut browser, Duration::from_secs(5));
    assert!(
        answered.starts_with("HTTP/1.1 200 "),
        "the consent redirect was not accepted within {:?} of a {FLOW_MS} ms flow \
         while an idle connection was held open; answer: {answered:?}",
        started.elapsed()
    );
}

/// A connection that is open but has sent no request when the redirect's head
/// completes is not "a second request": the redirect must still be accepted.
#[test]
fn a_queued_connection_without_a_request_is_not_a_second_request() {
    let flow = start();
    let head = redirect_head(&flow);
    let (first, last) = head.split_at(head.len() - 2);
    let mut browser = connect(&flow);
    browser.write_all(first.as_bytes()).unwrap();
    // The redirect's connection is accepted and being read.
    std::thread::sleep(Duration::from_millis(400));
    let _preconnect = connect(&flow);
    std::thread::sleep(Duration::from_millis(100));
    browser.write_all(last.as_bytes()).unwrap();
    let answered = answer(&mut browser, Duration::from_secs(5));
    assert!(
        answered.starts_with("HTTP/1.1 200 "),
        "a queued connection that carried no request refused the consent \
         redirect; answer: {answered:?}"
    );
}

/// Reach: a real browser following the consent redirect. Requires
/// `google-chrome-stable`; the page it renders is the listener's answer.
#[test]
#[ignore = "requires google-chrome-stable"]
fn chrome_following_the_redirect_is_accepted() {
    let flow = start();
    let profile = tempfile::tempdir().unwrap();
    let target = format!(
        "http://127.0.0.1:{}/?state={}&code=fixture-code&scope=openid",
        flow.port, flow.state
    );
    let output = Command::new("google-chrome-stable")
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            &format!("--user-data-dir={}", profile.path().display()),
            "--dump-dom",
            &target,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    let page = String::from_utf8_lossy(&output.stdout);
    assert!(
        page.contains("Consent received."),
        "Chrome's redirect was not accepted; page: {page:?}"
    );
}

/// Reach, without the flow: every connection Chrome opens for one navigation
/// to a loopback address, and the bytes each carries. The flow assumes one.
#[test]
#[ignore = "requires google-chrome-stable"]
fn chrome_opens_one_connection_per_navigation() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let profile = tempfile::tempdir().unwrap();
    let mut chrome = Command::new("google-chrome-stable")
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            &format!("--user-data-dir={}", profile.path().display()),
            "--dump-dom",
            &format!("http://127.0.0.1:{port}/?state=s&code=c"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    listener.set_nonblocking(true).unwrap();
    let started = Instant::now();
    let mut connections = Vec::new();
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok((stream, _)) = listener.accept() {
            stream.set_nonblocking(false).unwrap();
            connections.push((started.elapsed(), stream));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let seen: Vec<(Duration, usize)> = connections
        .iter_mut()
        .map(|(at, stream)| {
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .unwrap();
            let mut bytes = vec![0; 8192];
            let read = stream.read(&mut bytes).unwrap_or(0);
            (*at, read)
        })
        .collect();
    let _ = chrome.kill();
    let _ = chrome.wait();
    assert_eq!(
        seen.len(),
        1,
        "(accepted at, request bytes) per connection: {seen:?}"
    );
}
