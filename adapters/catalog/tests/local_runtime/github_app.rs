//! The `github_app` profile through the provider child
//! (story:catalog-github-app-installation-token). A disposable HTTPS fixture
//! stands in for `api.github.com` under the GitLab bundle's base: the child
//! signs an RS256 JWT with the entry's App key, exchanges it at
//! `POST {api_base}/app/installations/{installation_id}/access_tokens` for an
//! installation token, sends that token as `Authorization: Bearer`, and keeps
//! it in memory until five minutes before its `expires_at`.
//!
//! The App key is made once per test binary and never written to a file. Every
//! secret here is fictional and only ever compared, never printed.
use super::*;
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use num_bigint::BigUint;
use ring::rand::{SecureRandom as _, SystemRandom};
use std::sync::OnceLock;

const APP_PROFILE: &str = "fixture.app";
const APP_ID: &str = "424242";
const INSTALLATION_ID: u64 = 4_200_017;
/// Prefix of every installation token the fixture issues.
const INSTALLATION_TOKEN: &str = "fixture-installation-token-";
const EXCHANGE_ROUTE: &str = "/api/v4/app/installations/4200017/access_tokens";
const PROJECT_ROUTE: &str = "/api/v4/projects/org%2Fproject";
const IDENTITY_ROUTE: &str = "/api/v4/installation/repositories";
const GITHUB_JSON: &str = "application/vnd.github+json";
/// The PEM labels, kept apart from the encapsulation boundary so that no line
/// of this file reads as a key.
const PKCS1: &str = "RSA PRIVATE KEY";
const PKCS8: &str = "PRIVATE KEY";

// ---- the App key ----------------------------------------------------------------------------

/// A 2048-bit RSA key made for this test binary: the public half verifies the
/// JWT, the private half is the entry's, as PKCS #1 and as PKCS #8 DER.
struct AppKey {
    n: Vec<u8>,
    e: Vec<u8>,
    pkcs1: Vec<u8>,
    pkcs8: Vec<u8>,
}

fn app_key() -> &'static AppKey {
    static KEY: OnceLock<AppKey> = OnceLock::new();
    KEY.get_or_init(generate)
}

fn small_primes() -> Vec<u32> {
    let limit = 4096_usize;
    let mut composite = vec![false; limit];
    let mut primes = Vec::new();
    for value in 2..limit {
        if !composite[value] {
            primes.push(value as u32);
            for multiple in (value * value..limit).step_by(value) {
                composite[multiple] = true;
            }
        }
    }
    primes
}

/// Trial division, then 24 Miller-Rabin rounds with random bases.
fn probably_prime(candidate: &BigUint, small: &[u32], rng: &SystemRandom) -> bool {
    if small
        .iter()
        .any(|prime| candidate % *prime == BigUint::ZERO)
    {
        return false;
    }
    let one = BigUint::from(1_u32);
    let minus_one = candidate - &one;
    let shift = minus_one.trailing_zeros().unwrap();
    let odd = &minus_one >> shift;
    'rounds: for _ in 0..24 {
        let mut bytes = [0_u8; 160];
        rng.fill(&mut bytes).unwrap();
        let base = BigUint::from_bytes_be(&bytes) % (candidate - 3_u32) + 2_u32;
        let mut x = base.modpow(&odd, candidate);
        if x == one || x == minus_one {
            continue;
        }
        for _ in 1..shift {
            x = &x * &x % candidate;
            if x == minus_one {
                continue 'rounds;
            }
        }
        return false;
    }
    true
}

/// A 1024-bit prime with its top two bits set, so two of them make a 2048-bit
/// modulus, and `p - 1` prime to 65537.
fn prime(small: &[u32], rng: &SystemRandom) -> BigUint {
    loop {
        let mut bytes = [0_u8; 128];
        rng.fill(&mut bytes).unwrap();
        bytes[0] |= 0xc0;
        bytes[127] |= 1;
        let candidate = BigUint::from_bytes_be(&bytes);
        if &candidate % 65_537_u32 != BigUint::from(1_u32) && probably_prime(&candidate, small, rng)
        {
            return candidate;
        }
    }
}

/// One DER element.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut element = vec![tag];
    if content.len() < 0x80 {
        element.push(content.len() as u8);
    } else {
        let length = content.len().to_be_bytes();
        let skip = length.iter().take_while(|byte| **byte == 0).count();
        element.push(0x80 | (length.len() - skip) as u8);
        element.extend_from_slice(&length[skip..]);
    }
    element.extend_from_slice(content);
    element
}

/// A DER INTEGER of a nonnegative value.
fn integer(value: &BigUint) -> Vec<u8> {
    let mut bytes = value.to_bytes_be();
    if bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0);
    }
    der(0x02, &bytes)
}

fn generate() -> AppKey {
    let rng = SystemRandom::new();
    let small = small_primes();
    let (mut p, mut q) = (prime(&small, &rng), prime(&small, &rng));
    assert!(p != q);
    if p < q {
        std::mem::swap(&mut p, &mut q);
    }
    let one = BigUint::from(1_u32);
    let e = BigUint::from(65_537_u32);
    let n = &p * &q;
    let d = e.modinv(&((&p - &one) * (&q - &one))).unwrap();
    let dp = &d % (&p - &one);
    let dq = &d % (&q - &one);
    let qinv = q.modinv(&p).unwrap();
    let pkcs1 = der(
        0x30,
        &[
            der(0x02, &[0]),
            integer(&n),
            integer(&e),
            integer(&d),
            integer(&p),
            integer(&q),
            integer(&dp),
            integer(&dq),
            integer(&qinv),
        ]
        .concat(),
    );
    // rsaEncryption, 1.2.840.113549.1.1.1, with NULL parameters.
    let algorithm = der(
        0x30,
        &[
            der(
                0x06,
                &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01],
            ),
            der(0x05, &[]),
        ]
        .concat(),
    );
    let pkcs8 = der(
        0x30,
        &[der(0x02, &[0]), algorithm, der(0x04, &pkcs1)].concat(),
    );
    AppKey {
        n: n.to_bytes_be(),
        e: e.to_bytes_be(),
        pkcs1,
        pkcs8,
    }
}

fn pem(label: &str, der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut text = format!("-----BEGIN {label}-----\n");
    for line in body.as_bytes().chunks(64) {
        text.push_str(std::str::from_utf8(line).unwrap());
        text.push('\n');
    }
    text.push_str(&format!("-----END {label}-----\n"));
    text
}

fn entry_of(document: Value) -> Secret {
    Secret(serde_json::to_vec(&document).unwrap())
}

/// The App's entry with its key as PKCS #1.
fn entry() -> Secret {
    entry_of(json!({
        "app_id": APP_ID,
        "installation_id": INSTALLATION_ID,
        "private_key": pem(PKCS1, &app_key().pkcs1),
    }))
}

/// Whether `haystack` carries an installation token, a JWT, or a line of the
/// App key's PEM body.
fn carries_app_material(haystack: &[u8]) -> bool {
    let key = app_key();
    let jwt_header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
    let mut needles = vec![INSTALLATION_TOKEN.to_owned(), jwt_header];
    for der in [&key.pkcs1, &key.pkcs8] {
        let body = STANDARD.encode(der);
        // Past the fixed DER prefix every RSA key shares.
        needles.push(body[64..128].to_owned());
    }
    needles.iter().any(|needle| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

// ---- the fixture host -----------------------------------------------------------------------

/// What the exchange route answers next.
#[derive(Clone)]
enum Exchange {
    /// A new installation token that expires this many seconds from now.
    Issue(i64),
    Refuse(u16, Value),
}

/// One observed request. Compared by the tests, never printed.
#[derive(Clone)]
struct Seen {
    method: String,
    route: String,
    query: String,
    accept: Option<String>,
    authorization: Option<String>,
    body: Vec<u8>,
}

struct GitHubState {
    requests: Vec<Seen>,
    exchange: Exchange,
    issued: u32,
    /// The API answers 403 with the request's own `Authorization` in its message.
    echo: bool,
}

struct GitHub {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    config: PathBuf,
    port: u16,
    state: Arc<Mutex<GitHubState>>,
}

fn answer(
    state: &mut GitHubState,
    method: &str,
    route: &str,
    authorization: Option<&str>,
) -> (u16, Value) {
    if route == EXCHANGE_ROUTE && method == "POST" {
        return match state.exchange.clone() {
            Exchange::Refuse(status, body) => (status, body),
            Exchange::Issue(seconds) => {
                state.issued += 1;
                let expires_at = (time::OffsetDateTime::now_utc()
                    + time::Duration::seconds(seconds))
                .replace_nanosecond(0)
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap();
                (
                    201,
                    json!({
                        "token": format!("{INSTALLATION_TOKEN}{}", state.issued),
                        "expires_at": expires_at,
                        "permissions": {"contents": "read", "pull_requests": "write"},
                        "repository_selection": "selected",
                    }),
                )
            }
        };
    }
    if !route.starts_with("/api/v4/") || method != "GET" {
        return (404, json!({"message": "Not Found"}));
    }
    let issued = (1..=state.issued)
        .any(|n| authorization == Some(format!("Bearer {INSTALLATION_TOKEN}{n}").as_str()));
    if !issued {
        (401, json!({"message": "Bad credentials"}))
    } else if state.echo {
        (
            403,
            json!({"message": format!("Resource not accessible by integration: {}", authorization.unwrap_or_default())}),
        )
    } else if route == IDENTITY_ROUTE {
        (200, json!({"total_count": 1, "repositories": [{"id": 7}]}))
    } else {
        (200, json!({"id": 7, "name": "fixture-project"}))
    }
}

impl GitHub {
    fn new() -> Self {
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
        let state = Arc::new(Mutex::new(GitHubState {
            requests: Vec::new(),
            exchange: Exchange::Issue(3600),
            issued: 0,
            echo: false,
        }));
        let served = state.clone();
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
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let (route, query) = match target.split_once('?') {
                        Some((route, query)) => (route.to_owned(), query.to_owned()),
                        None => (target.clone(), String::new()),
                    };
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let authorization = header("authorization");
                    let (status, answer) = {
                        let mut state = served.lock().unwrap();
                        state.requests.push(Seen {
                            method: method.clone(),
                            route: route.clone(),
                            query,
                            accept: header("accept"),
                            authorization: authorization.clone(),
                            body,
                        });
                        answer(&mut state, &method, &route, authorization.as_deref())
                    };
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let port = address_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .port();
        let config = directory.join("catalog.json");
        let github = Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            config,
            port,
            state,
        };
        github.write_config(&github.app_config());
        github
    }
    fn app_config(&self) -> Value {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        json!({
            "format": "connectors-catalog-local/2",
            "instance": "fixture-github-app",
            "provider": "gitlab",
            "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
            "api_base": format!("https://localhost:{}/api/v4", self.port),
            "ca_file": self.ca,
            "auth": {
                "profile": APP_PROFILE,
                "scheme": "github_app",
                "header": "Authorization",
                "bearer": true,
                "label": "GitHub App private key",
                "identity": {
                    "source": "configuration",
                    "path": "installation/repositories",
                    "kind": "github.installation",
                },
            },
            "operations_file": repository.join("providers/gitlab/operations.json").canonicalize().unwrap(),
        })
    }
    fn write_config(&self, config: &Value) {
        private(&self.config, &serde_json::to_vec(config).unwrap());
    }
    fn spawn(&self) -> Child {
        let bootstrap = print_bootstrap(&self.config).expect("bootstrap inspection failed");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        let selection = Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-github-app".into(),
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
        };
        Child::spawn(&selection).unwrap_or_else(|failure| panic!("spawn {failure:?}"))
    }
    fn set_exchange(&self, exchange: Exchange) {
        self.state.lock().unwrap().exchange = exchange;
    }
    fn set_echo(&self, echo: bool) {
        self.state.lock().unwrap().echo = echo;
    }
    fn requests(&self) -> Vec<Seen> {
        self.state.lock().unwrap().requests.clone()
    }
    fn exchanges(&self) -> Vec<Seen> {
        self.requests()
            .into_iter()
            .filter(|request| request.route == EXCHANGE_ROUTE)
            .collect()
    }
    /// The `Authorization` of every request other than the exchange, in order.
    fn api_authorizations(&self) -> Vec<Option<String>> {
        self.requests()
            .into_iter()
            .filter(|request| request.route != EXCHANGE_ROUTE)
            .map(|request| request.authorization)
            .collect()
    }
}
impl Drop for GitHub {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

/// The printed bootstrap as JSON, or the refusal's stderr.
fn print_bootstrap(config: &Path) -> Result<Value, Vec<u8>> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(!carries_app_material(&output.stdout));
    assert!(!carries_app_material(&output.stderr));
    if !output.status.success() {
        return Err(output.stderr);
    }
    assert!(output.stderr.is_empty());
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    serde_json::from_value::<Bootstrap>(bootstrap.clone())
        .unwrap()
        .validate()
        .unwrap();
    Ok(bootstrap)
}

fn project(child: &mut Child, document: &Secret) -> Result<Value, Failure> {
    invoke(
        child,
        "project.get",
        "one",
        document,
        json!({"id": "org/project"}),
    )
}

fn now_s() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// The JWT a bearer value carries, its signature verified with the App's
/// public key: its header and its claims.
fn verified_jwt(authorization: &str) -> (Value, Value) {
    let jwt = authorization.strip_prefix("Bearer ").expect("a bearer JWT");
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3, "a compact JWS");
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let key = app_key();
    ring::signature::RsaPublicKeyComponents {
        n: &key.n,
        e: &key.e,
    }
    .verify(
        &ring::signature::RSA_PKCS1_2048_8192_SHA256,
        format!("{}.{}", parts[0], parts[1]).as_bytes(),
        &signature,
    )
    .expect("the JWT is signed with the App's key");
    let decode = |part: &str| -> Value {
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(part).unwrap()).unwrap()
    };
    (decode(parts[0]), decode(parts[1]))
}

// ---- the acceptance -------------------------------------------------------------------------

#[test]
fn a_github_app_jwt_is_rs256_with_the_app_id_as_issuer_and_lives_nine_minutes() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let key = app_key();
    // Either PEM form of the key signs the same JWT.
    for (label, der) in [(PKCS1, &key.pkcs1), (PKCS8, &key.pkcs8)] {
        let document = entry_of(json!({
            "app_id": APP_ID,
            "installation_id": INSTALLATION_ID.to_string(),
            "private_key": pem(label, der),
        }));
        let before = now_s();
        child
            .validate(APP_PROFILE, &document, deadline())
            .unwrap_or_else(|failure| panic!("{label}: {failure:?}"));
        let after = now_s();
        let exchange = github.exchanges().pop().unwrap();
        assert_eq!(exchange.method, "POST");
        assert!(exchange.query.is_empty());
        assert!(exchange.body.is_empty(), "the exchange sends no document");
        assert_eq!(exchange.accept.as_deref(), Some(GITHUB_JSON));
        let (header, claims) = verified_jwt(exchange.authorization.as_deref().unwrap());
        assert_eq!(header, json!({"alg": "RS256", "typ": "JWT"}), "{label}");
        assert_eq!(claims["iss"], APP_ID, "{label}");
        let iat = claims["iat"].as_i64().unwrap();
        let exp = claims["exp"].as_i64().unwrap();
        assert_eq!(
            claims.as_object().unwrap().len(),
            3,
            "iat, exp and iss only"
        );
        // Backdated by at most 60 s against clock drift, and valid for nine
        // minutes from the moment it was made: at most ten from `iat`.
        assert!(iat >= before - 60 && iat <= after, "{label}: iat {iat}");
        assert!(exp - iat <= 600, "{label}: lives {} s", exp - iat);
        assert!(
            exp >= before + 9 * 60 && exp <= after + 9 * 60,
            "{label}: exp {exp}"
        );
    }
    assert_eq!(github.exchanges().len(), 2);
}

#[test]
fn the_installation_token_is_sent_as_bearer_to_the_operation() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let project = project(&mut child, &entry()).unwrap_or_else(|f| panic!("invoke {f:?}"));
    assert_eq!(project["body"]["id"], 7);
    let requests = github.requests();
    assert_eq!(requests.len(), 2, "one exchange, then the operation");
    assert_eq!(requests[0].route, EXCHANGE_ROUTE);
    assert!(
        requests[0]
            .authorization
            .as_deref()
            .is_some_and(|value| !value.contains(INSTALLATION_TOKEN)),
        "the exchange carries the JWT, not a token"
    );
    assert_eq!(requests[1].route, PROJECT_ROUTE);
    assert!(
        requests[1].authorization.as_deref() == Some(&*format!("Bearer {INSTALLATION_TOKEN}1")),
        "the operation carries the installation token as bearer"
    );
    // The connection is the installation the exchange proved.
    let baseline = child.validate(APP_PROFILE, &entry(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "github.installation");
    assert_eq!(baseline.identity.subject, INSTALLATION_ID.to_string());
    assert_eq!(github.requests().last().unwrap().route, IDENTITY_ROUTE);
}

#[test]
fn the_installation_token_is_reused_until_five_minutes_before_expiry() {
    let github = GitHub::new();
    // An hour, and five and a half minutes: both outlast the five-minute margin.
    for seconds in [3600, 330] {
        github.set_exchange(Exchange::Issue(seconds));
        let mut child = github.spawn();
        let before = github.exchanges().len();
        project(&mut child, &entry()).unwrap();
        project(&mut child, &entry()).unwrap();
        assert_eq!(
            github.exchanges().len(),
            before + 1,
            "two operations, one exchange, for a token valid {seconds} s"
        );
        let issued = github.state.lock().unwrap().issued;
        let api = github.api_authorizations();
        assert!(
            api[api.len() - 2..]
                .iter()
                .all(|value| value.as_deref()
                    == Some(&*format!("Bearer {INSTALLATION_TOKEN}{issued}"))),
            "both operations carry the one token"
        );
    }
}

#[test]
fn a_token_within_five_minutes_of_expiry_is_exchanged_again() {
    let github = GitHub::new();
    // Four minutes fifty: inside the margin, so used once and never kept.
    github.set_exchange(Exchange::Issue(290));
    let mut child = github.spawn();
    project(&mut child, &entry()).unwrap();
    project(&mut child, &entry()).unwrap();
    assert_eq!(
        github.exchanges().len(),
        2,
        "each operation exchanged again"
    );
    let api = github.api_authorizations();
    assert!(api[0].as_deref() == Some(&*format!("Bearer {INSTALLATION_TOKEN}1")));
    assert!(api[1].as_deref() == Some(&*format!("Bearer {INSTALLATION_TOKEN}2")));
}

#[test]
fn a_refused_exchange_is_refused_by_name_with_no_operation_request() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for (status, body, failure) in [
        (
            401,
            json!({"message": "A JSON web token could not be decoded"}),
            Failure::InvalidCredential,
        ),
        (
            404,
            json!({"message": "Not Found"}),
            Failure::InvalidCredential,
        ),
        (
            403,
            json!({"message": "This installation has been suspended"}),
            Failure::InvalidCredential,
        ),
        (
            429,
            json!({"message": "API rate limit exceeded"}),
            Failure::ProviderRateLimited,
        ),
        (
            503,
            json!({"message": "Service unavailable"}),
            Failure::Unavailable,
        ),
        (
            422,
            json!({"message": "Validation Failed"}),
            Failure::Protocol,
        ),
    ] {
        github.set_exchange(Exchange::Refuse(status, body));
        let refusal = child
            .invoke_explained(
                "project.get",
                &revision,
                "one",
                &entry(),
                &serde_json::to_vec(&json!({"id": "org/project"})).unwrap(),
                deadline(),
            )
            .unwrap_err();
        assert_eq!(refusal.failure, failure, "exchange answered {status}");
        assert_eq!(refusal.reason, None, "no answer text for {status}");
        assert!(matches!(
            child.validate(APP_PROFILE, &entry(), deadline()),
            Err(found) if found == failure
        ));
    }
    assert_eq!(github.exchanges().len(), 12);
    assert!(
        github.api_authorizations().is_empty(),
        "no operation or identity request follows a refused exchange"
    );
}

#[test]
fn the_installation_token_never_appears_in_a_result_an_error_or_a_log() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let baseline = child.validate(APP_PROFILE, &entry(), deadline()).unwrap();
    assert!(!carries_app_material(
        &serde_json::to_vec(&baseline).unwrap()
    ));
    let result = project(&mut child, &entry()).unwrap();
    assert!(!carries_app_material(&serde_json::to_vec(&result).unwrap()));
    // The API echoes the request's token in its refusal: the child withholds
    // that reason, since only it knows the token.
    github.set_echo(true);
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let refusal = child
        .invoke_explained(
            "project.get",
            &revision,
            "one",
            &entry(),
            &serde_json::to_vec(&json!({"id": "org/project"})).unwrap(),
            deadline(),
        )
        .unwrap_err();
    assert_eq!(refusal.failure, Failure::ProviderForbidden);
    assert!(
        github
            .api_authorizations()
            .last()
            .unwrap()
            .as_deref()
            .is_some_and(|value| value.contains(INSTALLATION_TOKEN)),
        "the fixture did echo a token"
    );
    assert!(!carries_app_material(format!("{refusal:?}").as_bytes()));
    // Nothing at rest: not the configuration, not any file under the fixture
    // root, not any provider child's argv or environment. The child's stdio
    // is closed by the host, so it has no log stream to write one to.
    let mut pending = vec![github.root.path().to_owned()];
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                assert!(
                    !carries_app_material(&fs::read(&path).unwrap()),
                    "{}",
                    path.display()
                );
            }
        }
    }
    let children = provider_children(&github.config);
    assert!(!children.is_empty(), "no provider child observed");
    for pid in children {
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap();
        assert!(!carries_app_material(&cmdline), "argv of {pid}");
        assert!(
            !carries_app_material(&child_environ(pid)),
            "environment of {pid}"
        );
    }
    // A refused configuration names no material either (print_bootstrap
    // checks both streams).
    let mut config = github.app_config();
    config["auth"]["bearer"] = json!(false);
    github.write_config(&config);
    assert!(print_bootstrap(&github.config).is_err());
}

#[test]
fn a_github_app_entry_missing_a_field_or_with_a_bad_key_is_refused_with_no_request() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let key = pem(PKCS1, &app_key().pkcs1);
    let mut corrupted = key.clone().into_bytes();
    // A byte of the modulus, so the components no longer agree.
    corrupted[200] = if corrupted[200] == b'A' { b'B' } else { b'A' };
    let corrupted = String::from_utf8(corrupted).unwrap();
    let encrypted = key.replace(PKCS1, &format!("ENCRYPTED {PKCS8}"));
    let headed = key.replacen('\n', "\nProc-Type: 4,ENCRYPTED\n", 1);
    let documents = [
        json!({"installation_id": INSTALLATION_ID, "private_key": key}),
        json!({"app_id": APP_ID, "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": key, "token": "x"}),
        json!({"app_id": "", "installation_id": INSTALLATION_ID, "private_key": key}),
        json!({"app_id": "424 242", "installation_id": INSTALLATION_ID, "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": 0, "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": "42x", "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": "-1", "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": ""}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": "not a key"}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": corrupted}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": encrypted}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": headed}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": key.replace(PKCS1, "EC PRIVATE KEY")}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": key.replace(PKCS1, PKCS8)}),
    ];
    for (index, document) in documents.into_iter().enumerate() {
        let document = entry_of(document);
        assert!(
            matches!(project(&mut child, &document), Err(Failure::InvalidInput)),
            "entry {index} reached an operation"
        );
        assert!(
            matches!(
                child.validate(APP_PROFILE, &document, deadline()),
                Err(Failure::InvalidInput)
            ),
            "entry {index} validated"
        );
    }
    assert!(github.requests().is_empty(), "no request for a bad entry");
    // The well-formed entry still works on the same child.
    project(&mut child, &entry()).unwrap();
}

#[test]
fn a_github_app_profile_asks_for_the_app_its_installation_and_key_and_acquires_nothing() {
    let github = GitHub::new();
    let bootstrap = print_bootstrap(&github.config).unwrap();
    let profile = &bootstrap["profiles"][0];
    assert_eq!(profile["id"], APP_PROFILE);
    assert_eq!(profile["scheme"], "http_bearer");
    let fields: Vec<&str> = profile["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["app_id", "installation_id", "private_key"]);
    // Nothing to acquire and nothing that rotates: a read-only mounted entry
    // is enough (`docs/local-mounted-secret-custody.md`).
    assert!(profile.get("acquisition").is_none_or(Value::is_null));
}

#[test]
fn a_github_app_configuration_outside_its_shape_is_refused() {
    let github = GitHub::new();
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut config = github.app_config();
        edit(&mut config);
        github.write_config(&config);
        print_bootstrap(&github.config).is_err()
    };
    assert!(refused(&|c| c["auth"]["bearer"] = json!(false)));
    assert!(refused(&|c| c["auth"]["header"] = json!("X-Token")));
    assert!(refused(&|c| c["auth"]["account_label"] = json!("Account")));
    assert!(refused(&|c| c["auth"]["prefix"] = json!("token")));
    assert!(refused(
        &|c| c["auth"]["token_url"] = json!("https://localhost/token")
    ));
    assert!(refused(&|c| c["auth"]["requested_scopes"] = json!(["repo"])));
    // The connection is the installation: a subject read from the API, or an
    // identity with no read behind the exchange, is not this scheme's.
    assert!(refused(&|c| {
        c["auth"]["identity"] =
            json!({"path": "user", "kind": "github.user", "subject_pointer": "/id"})
    }));
    assert!(refused(&|c| {
        c["auth"]["identity"] = json!({"source": "configuration", "kind": "github.installation"})
    }));
    assert!(!refused(&|_| {}));
}
