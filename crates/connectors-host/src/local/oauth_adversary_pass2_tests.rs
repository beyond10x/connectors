//! Adversary pass 2 for story:cli-oauth-loopback-acquisition: the concurrent
//! loopback listener.
//!
//! Each case drives the public `oauth::entry` in its own process, reads the
//! consent address from standard error and plays browsers and local processes
//! against the loopback listener. The fixture writes one marker line to its
//! standard output when `entry` returns, then lingers, so a case can tell
//! "the flow returned" apart from "the process exited".
use super::{
    oauth, registry,
    runtime::{Acquisition, EntryField, Profile},
};
use connectors_sdk::Secret;
use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use url::Url;

const FIXTURE: &str = "CONNECTORS_OAUTH_ADVERSARY_PASS2_FLOW_MS";
const TOKEN_ENV: &str = "CONNECTORS_OAUTH_ADVERSARY_PASS2_TOKEN";
// Loopback on the default https port: nothing is contacted off this machine.
const AUTHORIZE: &str = "https://127.0.0.1/o/oauth2/auth";
const TOKEN: &str = "https://127.0.0.1/token";
const RESERVE_MS: u64 = 45_000;
const RETURNED: &str = "connectors-adversary: returned ";
const LINGER: Duration = Duration::from_secs(4);

fn profile(token: &str) -> Profile {
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
            token_url: token.into(),
            scopes: ["https://www.googleapis.com/auth/drive.readonly".to_owned()].into(),
        }),
    }
}

fn google(token: &str) -> Secret {
    Secret(
        serde_json::to_vec(&serde_json::json!({"installed": {
            "client_id": "fixture-client.apps.example.test",
            "project_id": "fixture-project",
            "auth_uri": AUTHORIZE,
            "token_uri": token,
            "client_secret": "fixture-client-secret",
            "redirect_uris": ["http://localhost"],
        }}))
        .unwrap(),
    )
}

// Re-entered in its own process by the cases below; a no-op otherwise.
#[test]
fn pass2_flow_fixture() {
    let Some(flow_ms) = std::env::var(FIXTURE)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return;
    };
    let token = std::env::var(TOKEN_ENV).unwrap_or_else(|_| TOKEN.to_owned());
    let expires = connectors_sdk::now_ms() + RESERVE_MS + flow_ms;
    let outcome = match oauth::entry(&profile(&token), google(&token), expires) {
        Ok(_) => "ok".to_owned(),
        Err(error) => format!("{:?}", error.code),
    };
    println!("{RETURNED}{outcome}");
    std::io::stdout().flush().unwrap();
    std::thread::sleep(LINGER);
}

struct Flow {
    child: Child,
    port: u16,
    state: String,
    stdout: PathBuf,
    stderr: PathBuf,
    started: Instant,
    _root: tempfile::TempDir,
}
impl Drop for Flow {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Flow {
    /// What `entry` returned, once it has.
    fn returned(&self) -> Option<String> {
        let text = std::fs::read_to_string(&self.stdout).ok()?;
        text.lines()
            .find_map(|line| line.strip_prefix(RETURNED))
            .map(str::to_owned)
    }

    /// Waits up to `wait` for `entry` to return.
    fn wait_returned(&self, wait: Duration) -> Option<(Duration, String)> {
        let until = Instant::now() + wait;
        while Instant::now() < until {
            if let Some(outcome) = self.returned() {
                return Some((self.started.elapsed(), outcome));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }

    fn redirect_head(&self) -> String {
        format!(
            "GET /?state={}&code=fixture-code&scope=openid HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
            self.state, self.port
        )
    }

    fn connect(&self) -> TcpStream {
        TcpStream::connect(("127.0.0.1", self.port)).unwrap()
    }
}

fn param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

fn consent_address(stderr: &Path) -> Option<Url> {
    let text = std::fs::read_to_string(stderr).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("connectors: consent-url "))
        .and_then(|line| Url::parse(line).ok())
}

/// Starts a flow of `flow_ms` in a fresh session (no terminal) whose code
/// exchange goes to `token`, and waits for its consent address.
fn start(flow_ms: u64, token: Option<&str>) -> Flow {
    let root = tempfile::tempdir().unwrap();
    let stdout = root.path().join("stdout");
    let stderr = root.path().join("stderr");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "local::oauth_adversary_pass2_tests::pass2_flow_fixture",
            "--nocapture",
        ])
        .env(FIXTURE, flow_ms.to_string())
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&stdout).unwrap())
        .stderr(std::fs::File::create(&stderr).unwrap());
    if let Some(token) = token {
        command.env(TOKEN_ENV, token);
    }
    // SAFETY: setsid is async-signal-safe and touches only the child.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let started = Instant::now();
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
        stdout,
        stderr,
        started,
        _root: root,
    }
}

/// The listener's whole answer on `stream`, or what arrived within `wait`.
fn answer(stream: &mut TcpStream, wait: Duration) -> String {
    stream.set_read_timeout(Some(wait)).unwrap();
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A local process opening and closing connections to `port` from `threads`
/// threads until dropped.
struct Flood {
    stop: Arc<AtomicBool>,
    connects: Arc<AtomicU64>,
    threads: Vec<std::thread::JoinHandle<()>>,
}
impl Flood {
    fn start(port: u16, threads: usize) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let connects = Arc::new(AtomicU64::new(0));
        let threads = (0..threads)
            .map(|_| {
                let (stop, connects) = (stop.clone(), connects.clone());
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                            connects.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect();
        Self {
            stop,
            connects,
            threads,
        }
    }
}
impl Drop for Flood {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

/// A local process that keeps connecting to the loopback port and closing
/// again must not keep the flow from reaching its deadline: the accept loop
/// in `Loopback::poll` returns only on `WouldBlock`, so the deadline and the
/// Ctrl-C check in `receive` are never reached while the backlog is never
/// empty.
#[test]
fn a_connection_flood_does_not_hold_the_flow_past_its_deadline() {
    const FLOW_MS: u64 = 3_000;
    let flow = start(FLOW_MS, None);
    let flood = Flood::start(flow.port, 16);
    let returned = flow.wait_returned(Duration::from_millis(FLOW_MS + 7_000));
    let connects = flood.connects.load(Ordering::Relaxed);
    drop(flood);
    let after = flow.wait_returned(Duration::from_secs(10));
    assert!(
        returned
            .as_ref()
            .is_some_and(|(at, _)| *at < Duration::from_millis(FLOW_MS + 3_000)),
        "a {FLOW_MS} ms flow had not returned {} ms after start while a local \
         process flooded the listener ({connects} connects); it returned {after:?} \
         once the flood stopped",
        FLOW_MS + 7_000
    );
}

/// A token host that accepts and never answers, so the code exchange stays
/// open until `release` is dropped.
fn stalled_token_host() -> (TcpListener, String) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, format!("https://127.0.0.1:{port}/token"))
}

/// After consent the `Drain` thread answers late requests until the flow
/// drops it, and dropping it joins that thread. Its `Loopback::poll` has the
/// same accept loop, so a local process flooding the port during the code
/// exchange keeps the join, and the CLI, from ever returning.
#[test]
fn a_connection_flood_during_the_exchange_does_not_hold_the_flow_open() {
    let (token_host, token) = stalled_token_host();
    let flow = start(20_000, Some(&token));
    let mut browser = flow.connect();
    browser.write_all(flow.redirect_head().as_bytes()).unwrap();
    let answered = answer(&mut browser, Duration::from_secs(5));
    assert!(answered.starts_with("HTTP/1.1 200 "), "{answered:?}");
    // The exchange is now waiting on the stalled token host.
    let (exchange, _) = token_host.accept().unwrap();
    let flood = Flood::start(flow.port, 16);
    std::thread::sleep(Duration::from_millis(500));
    // The token host gives up: the exchange fails at once and the flow drops
    // its `Drain`.
    drop(exchange);
    drop(token_host);
    let released = Instant::now();
    let returned = flow.wait_returned(Duration::from_secs(6));
    let connects = flood.connects.load(Ordering::Relaxed);
    drop(flood);
    let after = flow.wait_returned(Duration::from_secs(10));
    assert!(
        returned.is_some(),
        "the flow had not returned {:?} after its code exchange failed while a \
         local process flooded the listener ({connects} connects); it returned \
         {after:?} once the flood stopped",
        released.elapsed()
    );
}

/// 64 idle connections held by a local process fill `PENDING_LIMIT`; the
/// browser's redirect arriving then is closed on accept with no answer, and
/// the same redirect is accepted as soon as a slot frees. This is the
/// documented behaviour: docs/catalog-google-oauth.md, "More than 64 pending
/// local connections delay the redirect".
#[test]
fn sixty_four_idle_connections_delay_the_redirect_until_a_slot_frees() {
    const GUIDE: &str = "docs/catalog-google-oauth.md: \"More than 64 pending local \
                         connections delay the redirect\"";
    let flow = start(20_000, None);
    let idle: Vec<TcpStream> = (0..64).map(|_| flow.connect()).collect();
    // Past several accept polls, so every idle connection is pending.
    std::thread::sleep(Duration::from_millis(300));
    let mut browser = flow.connect();
    let sent = browser.write_all(flow.redirect_head().as_bytes());
    let answered = answer(&mut browser, Duration::from_secs(1));
    assert!(
        !answered.starts_with("HTTP/1.1 "),
        "with every slot held the redirect was answered (write: {sent:?}): \
         {answered:?}; the behaviour changed, so update {GUIDE}"
    );
    drop(idle);
    // A slot frees once the listener sees the held connections close; a browser
    // retries, so the redirect is retried until it is answered or 5 s pass.
    let deadline = Instant::now() + Duration::from_secs(5);
    let retried = loop {
        let mut retry = flow.connect();
        let _ = retry.write_all(flow.redirect_head().as_bytes());
        let got = answer(&mut retry, Duration::from_millis(500));
        if got.starts_with("HTTP/1.1 ") || Instant::now() >= deadline {
            break got;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        retried.starts_with("HTTP/1.1 200 "),
        "the redirect was not accepted once the held connections closed: {:?}; \
         see {GUIDE}",
        retried.lines().next()
    );
}

/// Measurement for the case above: a browser retrying the redirect every
/// 100 ms while a local process holds 64 idle connections once (`refresh`
/// false) or re-opens each as the listener drops it (`refresh` true). The
/// flow survives both; this reports how long the redirect is turned away.
fn redirect_delay(refresh: bool, give_up: Duration) -> (Option<Duration>, u32) {
    let flow = start(20_000, None);
    let mut idle: Vec<TcpStream> = (0..64).map(|_| flow.connect()).collect();
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let mut turned_away = 0;
    while started.elapsed() < give_up {
        if refresh {
            // Re-open every connection the listener has closed.
            for stream in &mut idle {
                stream.set_nonblocking(true).unwrap();
                let mut byte = [0_u8; 1];
                if matches!(stream.read(&mut byte), Ok(0) | Err(_))
                    && !matches!(stream.read(&mut byte), Err(ref e) if e.kind() == ErrorKind::WouldBlock)
                {
                    *stream = flow.connect();
                }
            }
        }
        let mut browser = flow.connect();
        let _ = browser.write_all(flow.redirect_head().as_bytes());
        if answer(&mut browser, Duration::from_millis(500)).starts_with("HTTP/1.1 200 ") {
            return (Some(started.elapsed()), turned_away);
        }
        turned_away += 1;
        std::thread::sleep(Duration::from_millis(100));
    }
    (None, turned_away)
}

#[test]
fn how_long_sixty_four_idle_connections_delay_the_redirect() {
    let once = redirect_delay(false, Duration::from_secs(8));
    let held = redirect_delay(true, Duration::from_secs(8));
    eprintln!("adversary-measure: held once {once:?}; re-opened {held:?}");
    assert!(once.0.is_some(), "one-shot exhaustion: {once:?}");
}

/// The unit's own `second_request_refused` stopped asserting that the
/// listener closes when the flow returns. After `entry` returns (the fixture
/// then lingers), nothing may still listen on the port: a leaked `Drain`
/// would keep it open and answering.
#[test]
fn the_listener_is_closed_once_the_flow_returns() {
    let flow = start(20_000, None);
    let mut browser = flow.connect();
    browser.write_all(flow.redirect_head().as_bytes()).unwrap();
    let answered = answer(&mut browser, Duration::from_secs(5));
    assert!(answered.starts_with("HTTP/1.1 200 "), "{answered:?}");
    let returned = flow.wait_returned(Duration::from_secs(10));
    assert!(returned.is_some(), "the flow did not return");
    // Still inside the fixture's linger, so the process is alive.
    assert!(flow.returned().is_some());
    let reconnect = TcpStream::connect(("127.0.0.1", flow.port));
    assert!(
        matches!(&reconnect, Err(error) if error.kind() == ErrorKind::ConnectionRefused),
        "the loopback port still accepts after the flow returned ({returned:?}): {reconnect:?}"
    );
}

/// A head that completes just inside the 2 s idle limit is a request and
/// decides; one completing after it was dropped and decides nothing.
#[test]
fn a_head_completing_at_the_idle_limit() {
    let flow = start(20_000, None);
    let head = flow.redirect_head();
    // Past the limit: dropped without an answer, and without effect.
    let mut late = flow.connect();
    late.write_all(&head.as_bytes()[..10]).unwrap();
    std::thread::sleep(Duration::from_millis(2_300));
    let _ = late.write_all(&head.as_bytes()[10..]);
    let late_answer = answer(&mut late, Duration::from_secs(2));
    assert!(
        !late_answer.starts_with("HTTP/1.1 "),
        "a head completed after the idle limit was answered: {late_answer:?}"
    );
    // Inside it: accepted.
    let mut browser = flow.connect();
    browser.write_all(&head.as_bytes()[..10]).unwrap();
    std::thread::sleep(Duration::from_millis(1_800));
    browser.write_all(&head.as_bytes()[10..]).unwrap();
    let answered = answer(&mut browser, Duration::from_secs(5));
    assert!(
        answered.starts_with("HTTP/1.1 200 "),
        "a head completed 1.8 s after connect was not accepted: {answered:?}"
    );
}

/// Without a terminal, standard error carries exactly the one marked line,
/// and nothing of the client secret or the PKCE verifier.
#[test]
fn standard_error_carries_only_the_consent_line() {
    let flow = start(20_000, None);
    let mut browser = flow.connect();
    browser.write_all(flow.redirect_head().as_bytes()).unwrap();
    let _ = answer(&mut browser, Duration::from_secs(5));
    assert!(flow.wait_returned(Duration::from_secs(10)).is_some());
    let text = std::fs::read_to_string(&flow.stderr).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text:?}");
    assert!(lines[0].starts_with("connectors: consent-url https://127.0.0.1/o/oauth2/auth?"));
    assert!(!text.contains("fixture-client-secret"));
    assert!(!text.contains("code_verifier"));
}

/// A browser sends every cookie it holds for `127.0.0.1`, whatever the port,
/// with the consent redirect. A head over `REQUEST_LIMIT` (8 KiB) is cut there,
/// taken as complete, and refused, which ends the flow.
#[test]
fn a_redirect_carrying_the_browsers_loopback_cookies_is_accepted() {
    let flow = start(20_000, None);
    let cookie: String = (0..6)
        .map(|n| format!("dev_session_{n}={}", "j".repeat(1_400)))
        .collect::<Vec<_>>()
        .join("; ");
    let head = flow.redirect_head();
    let head = head.replacen(
        "\r\n\r\n",
        &format!("\r\nUser-Agent: fixture\r\nCookie: {cookie}\r\n\r\n"),
        1,
    );
    let mut browser = flow.connect();
    browser.write_all(head.as_bytes()).unwrap();
    let answered = answer(&mut browser, Duration::from_secs(5));
    let returned = flow.wait_returned(Duration::from_secs(2));
    assert!(
        answered.starts_with("HTTP/1.1 200 "),
        "a {} byte redirect head was not accepted; answer: {:?}; flow: {returned:?}",
        head.len(),
        answered.lines().next()
    );
}

/// Reach for the case above: Chrome, holding cookies a local development
/// server set on another port of `127.0.0.1`, follows the consent redirect.
#[test]
#[ignore = "requires google-chrome-stable"]
fn chrome_with_loopback_cookies_following_the_redirect_is_accepted() {
    let profile = tempfile::tempdir().unwrap();
    let chrome = |target: &str| {
        Command::new("google-chrome-stable")
            .args([
                "--headless=new",
                "--disable-gpu",
                "--no-first-run",
                "--no-default-browser-check",
                &format!("--user-data-dir={}", profile.path().display()),
                "--dump-dom",
                target,
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap()
    };
    // A local development server on another port sets six 1.4 KB cookies.
    let dev = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let dev_port = dev.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = dev.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request);
        let mut response = String::from("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n");
        for n in 0..6 {
            response.push_str(&format!(
                "Set-Cookie: dev_session_{n}={}; Max-Age=3600; Path=/\r\n",
                "j".repeat(1_400)
            ));
        }
        response.push_str("Content-Length: 2\r\nConnection: close\r\n\r\nok");
        stream.write_all(response.as_bytes()).unwrap();
    });
    chrome(&format!("http://127.0.0.1:{dev_port}/"));
    server.join().unwrap();
    let flow = start(20_000, None);
    let output = chrome(&format!(
        "http://127.0.0.1:{}/?state={}&code=fixture-code&scope=openid",
        flow.port, flow.state
    ));
    let page = String::from_utf8_lossy(&output.stdout);
    assert!(
        page.contains("Consent received."),
        "Chrome holding loopback cookies was not accepted; page: {page:?}; flow: {:?}",
        flow.wait_returned(Duration::from_secs(2))
    );
}
