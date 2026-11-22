//! Security review of story:catalog-grafana-reads and
//! story:catalog-grafana-datasource-proxy-reads: integrity conformance checks of the units' own
//! invariants, written against the shipped selection, bundle, definition and served host.
//!
//! 1. The service-account token travels only as `Authorization: Bearer` to the configured host,
//!    never in a request target, a log line or an answer.
//! 2. `catalog.allowed_values` refuses a `uid` outside its non-empty list by name before any
//!    upstream request and before the credential is read; spelling tricks do not pass it; start
//!    refuses a malformed list; it bounds only the operations it names.
//! 3. A caller's `uid` cannot move the request off
//!    `/api/datasources/proxy/uid/{uid}/<fixed sub-path>`.
//! 4. Only reads are selectable.
//! 5. An over-limit answer is refused as `capacity`, also when it arrives without a length.
//! 6. The vendored source matches its pinned digest, and the amendments record it.
//!
//! Every id, name and token is synthetic. No live Grafana, no network.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

const BASE: &str = "/api";
const INSTANCE: &str = "sr-grafana";
const PROFILE: &str = "grafana.service-account";
const TOKEN: &str = "sr-grafana-service-account-token-7f3a";
/// The committed source's digest: Grafana's published document (`89ff3e97…`) less its one
/// example-token member (`adapters/grafana/upstream/README.md`), which every record names.
const PINNED: &str = "be8923b42413a640f92af54a728f5b04f936a2851a04b8dfa277ae0db3bdebdc";
const UPSTREAM: &str = "../grafana/upstream";
const DEFINITION: &str =
    "../../crates/connectors-catalog/tests/fixtures/definitions/connectors/grafana.yaml";
const GUIDE: &str = "../../docs/catalog-grafana.md";
const PROXY_PREFIX: &str = "/api/datasources/proxy/uid/";

const PROM_VECTOR: &str = r#"{"data":{"result":[],"resultType":"vector"},"status":"success"}"#;
const NOT_FOUND: &str = r#"{"message":"Not found"}"#;
const UNAUTHORIZED: &str = r#"{"message":"invalid API key"}"#;
/// Data sources whose answers the fixture streams with no `Content-Length`.
const CHUNKED_OVER: &str = "sr-chunked-over";
const CHUNKED_AT: &str = "sr-chunked-at";
/// A data source the fixture answers 401 and 500 for, so error paths run.
const DENIED: &str = "sr-denied";
const BROKEN: &str = "sr-broken";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn shipped_file() -> Value {
    serde_json::from_slice(&fs::read(root().join("providers/grafana/operations.json")).unwrap())
        .unwrap()
}
fn shipped() -> Vec<Selection> {
    serde_json::from_value(shipped_file()["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "grafana").unwrap()
}

/// A JSON answer of exactly `bytes` bytes.
fn sized(bytes: usize) -> String {
    let head = r#"{"data":{"result":[],"resultType":"vector"},"pad":""#;
    let tail = r#"","status":"success"}"#;
    format!(
        "{head}{}{tail}",
        "a".repeat(bytes - head.len() - tail.len())
    )
}

/// Percent-decoding of one path segment, byte for byte.
fn decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("zz");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---- 6: the pinned source ---------------------------------------------------------------------

/// Invariant 6: the vendored document's bytes are the pinned digest, and every record that names
/// the source (the amendments, the bundle, the index, the definition) names that same digest.
#[test]
fn sr_vendored_source_matches_its_pin_everywhere_it_is_named() {
    let bytes = fs::read(root().join(UPSTREAM).join("openapi3.json")).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), PINNED);
    let amendments: Value = serde_json::from_slice(
        &fs::read(root().join(UPSTREAM).join("grafana.amendments.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(amendments["source_sha256"], PINNED);
    let bundle = committed();
    assert_eq!(bundle.source.source_sha256, PINNED);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    assert_eq!(index.find("grafana").unwrap().source_sha256, PINNED);
    let definition = fs::read_to_string(root().join(DEFINITION)).unwrap();
    assert!(
        definition.contains(&format!("sha256: {PINNED}")),
        "the definition pins another digest"
    );
}

// ---- 4: reads only ----------------------------------------------------------------------------

/// Invariant 4: nothing selectable is a write. The selection file, the engine built from it, the
/// definition fixture and the amended operations each resolve to `GET` reads only.
#[test]
fn sr_only_reads_are_selectable() {
    let bundle = committed();
    let selections = shipped();
    for operation in shipped_file()["operations"].as_array().unwrap() {
        assert_eq!(operation["effect"], "read", "{operation}");
    }
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    let get = |operation_id: &str| {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("`{operation_id}` is not in the bundle"));
        assert_eq!(operation.method, "get", "`{operation_id}`");
    };
    for selection in &selections {
        get(&selection.operation_id);
    }
    let definition = fs::read_to_string(root().join(DEFINITION)).unwrap();
    let mut ops = 0;
    for line in definition.lines() {
        let line = line.trim();
        if let Some(op) = line.strip_prefix("op: ") {
            get(op.trim());
            ops += 1;
        }
        assert!(
            !line.starts_with("effect:") || line == "effect: read",
            "the definition declares `{line}`"
        );
    }
    assert_eq!(
        ops,
        selections.len(),
        "the definition and the selection agree"
    );
    let amendments: Value = serde_json::from_slice(
        &fs::read(root().join(UPSTREAM).join("grafana.amendments.json")).unwrap(),
    )
    .unwrap();
    for amendment in amendments["amendments"].as_array().unwrap() {
        assert_eq!(amendment["add_operation"]["method"], "get", "{amendment}");
    }
}

// ---- the fixture ------------------------------------------------------------------------------

/// Method, request target, `Authorization` header and the whole head of each request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>, String)>>>;

fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join(GUIDE)).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body.to_owned()))
        .find(|body| body.contains("\"provider\": \"grafana\""))
        .expect("the documented Grafana configuration");
    serde_json::from_str(&example).unwrap()
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    port: u16,
    pem: String,
    config: PathBuf,
    requests: Requests,
}

/// Status and body chunks for one request; `None` length means chunked.
fn answer(route: &str) -> (u16, Vec<String>, bool) {
    let Some(rest) = route.strip_prefix(PROXY_PREFIX) else {
        return (404, vec![NOT_FOUND.to_owned()], false);
    };
    let uid = rest.split('/').next().unwrap_or_default();
    match uid {
        CHUNKED_OVER | CHUNKED_AT => {
            let size = if uid == CHUNKED_OVER {
                connectors_core::RESPONSE_LIMIT + 1
            } else {
                connectors_core::RESPONSE_LIMIT
            };
            let body = sized(size);
            let chunks = body
                .as_bytes()
                .chunks(64 * 1024)
                .map(|c| String::from_utf8(c.to_vec()).unwrap())
                .collect();
            (200, chunks, true)
        }
        DENIED => (401, vec![UNAUTHORIZED.to_owned()], false),
        BROKEN => (500, vec![r#"{"message":"internal"}"#.to_owned()], false),
        "fixture-prom" | "fixture-loki" | "fixture-other" => {
            (200, vec![PROM_VECTOR.to_owned()], false)
        }
        _ => (404, vec![NOT_FOUND.to_owned()], false),
    }
}

impl Provider {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = cert.cert.pem();
        let ca = directory.join("ca.pem");
        private(&ca, pem.as_bytes());
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
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
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
                        accepted = listener.accept() => accepted.unwrap().0,
                    };
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let authorization = head.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    let (route, _) = target.split_once('?').unwrap_or((&target, ""));
                    let (status, chunks, chunked) =
                        if authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
                            (401, vec![UNAUTHORIZED.to_owned()], false)
                        } else if method != "GET" {
                            (405, vec![NOT_FOUND.to_owned()], false)
                        } else {
                            answer(route)
                        };
                    observed.lock().unwrap().push((
                        method,
                        target.clone(),
                        authorization,
                        head.clone(),
                    ));
                    if chunked {
                        let head = format!(
                            "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                        );
                        let _ = stream.write_all(head.as_bytes()).await;
                        for chunk in &chunks {
                            let framed = format!("{:x}\r\n{chunk}\r\n", chunk.len());
                            if stream.write_all(framed.as_bytes()).await.is_err() {
                                break;
                            }
                        }
                        let _ = stream.write_all(b"0\r\n\r\n").await;
                    } else {
                        let body = chunks.concat();
                        let head = format!(
                            "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(head.as_bytes()).await;
                        let _ = stream.write_all(body.as_bytes()).await;
                    }
                    let _ = stream.shutdown().await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut config = documented_config();
        config["instance"] = json!(INSTANCE);
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["api_base"] = json!(format!("https://localhost:{}/api", address.port()));
        config["ca_file"] = json!(ca);
        config["operations_file"] = json!(root_path("providers/grafana/operations.json"));
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            port: address.port(),
            pem,
            config: path,
            requests,
        }
    }

    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: INSTANCE.into(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap.configuration_revision,
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

    fn requests(&self) -> Vec<(String, String, Option<String>, String)> {
        self.requests.lock().unwrap().clone()
    }

    /// Invariant 1 on the wire: every request carries the token exactly as
    /// `Authorization: Bearer`, and nowhere else in its head.
    fn assert_token_only_as_bearer(&self) {
        for (_, target, authorization, head) in self.requests() {
            assert_eq!(authorization, Some(format!("Bearer {TOKEN}")), "{target}");
            assert!(
                !target.contains(TOKEN),
                "the token is in the target {target}"
            );
            assert_eq!(
                head.matches(TOKEN).count(),
                1,
                "the token appears once: {target}"
            );
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
}

/// The `uid`s a caller might use to move the proxied request, beside the three proxy reads.
const TRICKS: &[&str] = &[
    "a/b",
    "../../../api/admin/settings",
    "fixture-prom/../../../api/admin/settings",
    "a%2Fb",
    "a%2F..%2F..%2Fadmin",
    "%2e%2e",
    ".%2e",
    "a?x=1",
    "a#frag",
    "a\\..\\b",
    "a;b",
    "a b",
    "..",
    ".",
    "\u{e9}",
];

fn proxy_input(operation: &str, uid: &str) -> (Value, &'static str, &'static str) {
    match operation {
        "prometheus.query" => (
            json!({"uid": uid, "query": "up"}),
            "/api/v1/query",
            "query=up",
        ),
        "prometheus.query_range" => (
            json!({"uid": uid, "query": "up", "start": "1", "end": "2", "step": "1"}),
            "/api/v1/query_range",
            "query=up&start=1&end=2&step=1",
        ),
        _ => (
            json!({"uid": uid, "query": "up", "start": "1", "end": "2", "limit": 1}),
            "/loki/api/v1/query_range",
            "query=up&start=1&end=2&limit=1",
        ),
    }
}

const PROXY_READS: [&str; 3] = [
    "prometheus.query",
    "prometheus.query_range",
    "loki.query_range",
];

// ---- 3: the uid stays in its segment ----------------------------------------------------------

/// Invariant 3, as the wire sees it: whatever the caller's `uid`, a request that is sent at all
/// is `/api/datasources/proxy/uid/<one segment>/<the fixed sub-path>?<the declared query>`, the
/// segment holds no raw `/`, `?`, `#` or `\`, and it decodes back to exactly the caller's `uid`.
#[test]
fn sr_a_uid_cannot_leave_its_path_segment_on_the_wire() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for operation in PROXY_READS {
        for uid in TRICKS {
            let before = provider.requests().len();
            let (input, sub, query) = proxy_input(operation, uid);
            let _ = attempt(&mut child, operation, &input);
            let requests = provider.requests();
            assert!(
                requests.len() <= before + 1,
                "`{operation}` {uid:?}: one request at most"
            );
            let Some((method, target, ..)) = requests.get(before) else {
                continue;
            };
            assert_eq!(method, "GET");
            let rest = target
                .strip_prefix(PROXY_PREFIX)
                .unwrap_or_else(|| panic!("`{operation}` {uid:?} left the proxy: {target}"));
            let (segment, after) = rest.split_once('/').unwrap_or((rest, ""));
            assert!(
                !segment.contains(['?', '#', '\\']),
                "`{operation}` {uid:?}: raw delimiter in {target}"
            );
            assert_eq!(
                format!("/{after}"),
                format!("{sub}?{query}"),
                "`{operation}` {uid:?} moved the sub-path: {target}"
            );
            assert_eq!(decode(segment), *uid, "`{operation}` {uid:?}: {target}");
        }
    }
    provider.assert_token_only_as_bearer();
}

/// Invariant 3, against a server that decodes a path before it routes it (as the engine assumes
/// of a download's server, `Target::escapes`): no `uid` is sent whose segment decodes to `/` or
/// `\`, since such a segment lands outside `/api/datasources/proxy/uid/{uid}/` on that server.
#[test]
fn sr_a_uid_segment_never_decodes_to_a_separator() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut sent = Vec::new();
    for operation in PROXY_READS {
        for uid in TRICKS {
            let before = provider.requests().len();
            let (input, ..) = proxy_input(operation, uid);
            let _ = attempt(&mut child, operation, &input);
            if let Some((_, target, ..)) = provider.requests().get(before) {
                let rest = target.strip_prefix(PROXY_PREFIX).unwrap();
                let segment = rest.split('/').next().unwrap();
                if decode(segment).contains(['/', '\\']) {
                    sent.push(format!("{operation} {uid:?} -> {target}"));
                }
            }
        }
    }
    assert!(
        sent.is_empty(),
        "sent a uid segment that decodes to a separator:\n{}",
        sent.join("\n")
    );
}

// ---- 5: size ----------------------------------------------------------------------------------

/// Invariant 5 without a `Content-Length`: an answer streamed one byte over the limit is
/// `capacity`, and one streamed exactly at the limit is returned whole, never cut short.
#[test]
fn sr_a_chunked_over_limit_answer_is_capacity_and_one_at_the_limit_is_whole() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for operation in PROXY_READS {
        let (input, ..) = proxy_input(operation, CHUNKED_OVER);
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::Capacity)),
            "`{operation}`: {outcome:?}"
        );
        let (input, ..) = proxy_input(operation, CHUNKED_AT);
        let raw = attempt(&mut child, operation, &input)
            .unwrap_or_else(|failure| panic!("`{operation}` at the limit: {failure:?}"));
        let result: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            serde_json::to_string(&result["body"]).unwrap(),
            sized(connectors_core::RESPONSE_LIMIT),
            "`{operation}` at the limit is not whole"
        );
    }
    provider.assert_token_only_as_bearer();
}

// ---- 1, 2: the served instance ----------------------------------------------------------------

#[cfg(target_os = "linux")]
mod served {
    use super::*;
    use connectors_custody::mounted::fixture::{Jail, Map};
    use std::{
        io::{BufRead as _, BufReader, Write as _},
        os::unix::fs::symlink,
        process::Stdio,
        sync::mpsc,
    };

    const SERVICE_TOKEN: &str = "sr-service-bearer";
    const SOURCE_DIRECTORY: &str = "/srv/secret";
    const MOUNTED: &str = "/run/connectors/credential";
    const BINARY: &str = "/bin-test/connectors-catalog-provider";
    const CONFIG: &str = "/srv/served.json";
    const SERVED_INSTANCE: &str = "grafana.fixture";
    /// A `catalog` member whose string value is spliced in as the raw text of `allowed_values`,
    /// for configuration JSON cannot express as a value (a repeated key).
    const RAW_ALLOWED: &str = "__sr_raw_allowed_values__";

    struct Served {
        _jail: Jail,
        child: Option<std::process::Child>,
        base: String,
        log: Arc<Mutex<String>>,
        source: PathBuf,
    }

    impl Served {
        /// Started with `catalog` extended by `extra`, or the log of a start that refused.
        fn start_with(provider: &Provider, extra: Value) -> Result<Self, String> {
            let mut jail = Jail::new();
            jail.directory("/srv", 0o755);
            jail.directory("/srv/state", 0o700);
            jail.directory("/srv/tmp", 0o700);
            jail.directory(SOURCE_DIRECTORY, 0o755);
            jail.directory(MOUNTED, 0o755);
            jail.read_only(SOURCE_DIRECTORY, MOUNTED);
            jail.install(
                Path::new(env!("CARGO_BIN_EXE_connectors-catalog-provider")),
                BINARY,
            );
            let write = |inside: &str, bytes: &[u8], mode: u32| {
                let path = jail.path(inside);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, bytes).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            };
            write("/srv/ca.pem", provider.pem.as_bytes(), 0o644);
            write("/srv/service-token", SERVICE_TOKEN.as_bytes(), 0o600);
            write(
                "/srv/defs/connectors/grafana.yaml",
                &fs::read(root().join(DEFINITION)).unwrap(),
                0o644,
            );
            let mut gzip =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gzip.write_all(&fs::read(root().join(UPSTREAM).join("openapi3.json")).unwrap())
                .unwrap();
            write(
                "/srv/defs/upstream/grafana/openapi3.json.gz",
                &gzip.finish().unwrap(),
                0o644,
            );
            write(
                "/srv/defs/upstream/grafana/grafana.amendments.json",
                &fs::read(root().join(UPSTREAM).join("grafana.amendments.json")).unwrap(),
                0o644,
            );
            let mut config = json!({
                "service": {
                    "instance": SERVED_INSTANCE,
                    "listen": "127.0.0.1:0",
                    "service_credential": {"kind": "file", "path": "/srv/service-token"},
                    "state": "/srv/state",
                },
                "catalog": {
                    "definition": "/srv/defs/connectors/grafana.yaml",
                    "definitions_root": "/srv/defs",
                    "instance": "fixture",
                    "endpoint": format!("https://localhost:{}", provider.port),
                },
                "credential": {"binding": "mounted_secret", "directory": MOUNTED},
            });
            let mut raw = None;
            for (key, value) in extra.as_object().unwrap() {
                if key == RAW_ALLOWED {
                    raw = value.as_str().map(str::to_owned);
                    config["catalog"]["allowed_values"] = json!(RAW_ALLOWED);
                } else {
                    config["catalog"][key] = value.clone();
                }
            }
            let mut text = serde_json::to_string(&config).unwrap();
            if let Some(raw) = raw {
                text = text.replace(&format!("\"{RAW_ALLOWED}\""), &raw);
            }
            write(CONFIG, text.as_bytes(), 0o600);
            let source = jail.path(SOURCE_DIRECTORY);
            let stamp = "..2026_11_11_00_00_00.000000001";
            let item = source.join(stamp).join(SERVED_INSTANCE).join(PROFILE);
            fs::create_dir_all(item.parent().unwrap()).unwrap();
            for directory in [source.join(stamp), source.join(stamp).join(SERVED_INSTANCE)] {
                fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
            }
            fs::write(&item, serde_json::to_vec(&json!({"token": TOKEN})).unwrap()).unwrap();
            fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
            symlink(
                Path::new("..data").join(SERVED_INSTANCE),
                source.join(SERVED_INSTANCE),
            )
            .unwrap();
            symlink(stamp, source.join("..data")).unwrap();

            let mut attempts = 0;
            let mut child = loop {
                let spawned = jail
                    .command(BINARY, Map::Own)
                    .args(["--config", CONFIG])
                    .env_clear()
                    .env("SSL_CERT_FILE", "/srv/ca.pem")
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
            let log: Arc<Mutex<String>> = Arc::default();
            let written = log.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    written.lock().unwrap().push_str(&format!("{line}\n"));
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
                        return Ok(Self {
                            _jail: jail,
                            child: Some(child),
                            base: format!("http://{listen}"),
                            log,
                            source,
                        });
                    }
                    Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let status = child.wait().unwrap();
                        let log = log.lock().unwrap().clone();
                        return Err(format!("exited {status}: {log}"));
                    }
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "the served binary did not listen within 60 s: {}",
                log.lock().unwrap()
            );
        }

        async fn send(
            &self,
            method: reqwest::Method,
            route: &str,
            body: Option<&Value>,
        ) -> (u16, Vec<u8>) {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let mut request = reqwest::Client::new()
                .request(method, format!("{}{route}", self.base))
                .bearer_auth(SERVICE_TOKEN)
                .timeout(Duration::from_secs(30));
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request.send().await.expect("the served host answers");
            let status = response.status().as_u16();
            (status, response.bytes().await.unwrap().to_vec())
        }

        fn log(&self) -> String {
            self.log.lock().unwrap().clone()
        }

        async fn revision(&self) -> String {
            let (status, descriptor) = self.send(reqwest::Method::GET, "/v1/describe", None).await;
            assert_eq!(status, 200);
            let descriptor: Value = serde_json::from_slice(&descriptor).unwrap();
            descriptor["revision"].as_str().unwrap().to_owned()
        }

        /// One invocation on `route` (`/v1alpha2/invoke` or `/v1/invoke`): status and bytes.
        async fn invoke(
            &self,
            route: &str,
            revision: &str,
            request_id: &str,
            operation: &str,
            input: Value,
        ) -> (u16, Value, Vec<u8>) {
            let version = if route == "/v1/invoke" {
                "v1alpha1"
            } else {
                "v1alpha2"
            };
            let body = json!({
                "version": version,
                "request_id": request_id,
                "operation": operation,
                "revision": revision,
                "input": input,
            });
            let (status, raw) = self.send(reqwest::Method::POST, route, Some(&body)).await;
            let answer = serde_json::from_slice(&raw).unwrap_or(Value::Null);
            (status, answer, raw)
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

    fn contains(haystack: &[u8], needle: &str) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    }

    /// Invariant 2: with `prometheus.query` bounded to `fixture-prom`, no other spelling of a
    /// data source reaches Grafana, on either invocation route: case, whitespace, control bytes,
    /// a look-alike hyphen, percent-encoding, a path tail, a dot segment, a container, a number,
    /// `null`, and a qualified key that names the path parameter. Each is refused, nothing is
    /// sent, and no answer or log line carries the token.
    #[tokio::test(flavor = "multi_thread")]
    async fn sr_the_allow_list_admits_only_its_exact_values() {
        let provider = Provider::new();
        let served = Served::start_with(
            &provider,
            json!({"allowed_values": {"prometheus.query": {"uid": ["fixture-prom"]}}}),
        )
        .unwrap();
        let revision = served.revision().await;
        let inputs = [
            json!({"uid": "fixture-prom ", "query": "up"}),
            json!({"uid": " fixture-prom", "query": "up"}),
            json!({"uid": "fixture-prom\n", "query": "up"}),
            json!({"uid": "fixture-prom\u{0}", "query": "up"}),
            json!({"uid": "Fixture-Prom", "query": "up"}),
            json!({"uid": "fixture\u{2010}prom", "query": "up"}),
            json!({"uid": "fixture%2Dprom", "query": "up"}),
            json!({"uid": "fixture-prom/", "query": "up"}),
            json!({"uid": "fixture-prom/../fixture-other", "query": "up"}),
            json!({"uid": "./fixture-prom", "query": "up"}),
            json!({"uid": ["fixture-prom"], "query": "up"}),
            json!({"uid": {"fixture-prom": true}, "query": "up"}),
            json!({"uid": null, "query": "up"}),
            json!({"uid": 0, "query": "up"}),
            json!({"query": "up", "path:uid": "fixture-other"}),
            json!({"uid": "fixture-prom", "query": "up", "path:uid": "fixture-other"}),
            json!({"uid": "fixture-other", "query": "up", "query:uid": "fixture-prom"}),
        ];
        let mut answers = Vec::new();
        for route in ["/v1alpha2/invoke", "/v1/invoke"] {
            for (n, input) in inputs.iter().enumerate() {
                let request = format!("bypass-{}-{n}", route.len());
                let (status, answer, raw) = served
                    .invoke(
                        route,
                        &revision,
                        &request,
                        "prometheus.query",
                        input.clone(),
                    )
                    .await;
                assert_ne!(status, 200, "{route} {input} was answered: {answer}");
                assert!(
                    !contains(&raw, TOKEN),
                    "{route} {input}: the answer holds the token"
                );
                answers.push(raw);
            }
        }
        assert!(
            provider.requests().is_empty(),
            "a value outside the list reached Grafana: {:?}",
            provider.requests().iter().map(|r| &r.1).collect::<Vec<_>>()
        );
        // The exact value is read, so the refusals above were the list's.
        let (status, answer, _) = served
            .invoke(
                "/v1alpha2/invoke",
                &revision,
                "listed",
                "prometheus.query",
                json!({"uid": "fixture-prom", "query": "up"}),
            )
            .await;
        assert_eq!(status, 200, "{answer}\n{}", served.log());
        assert_eq!(provider.requests().len(), 1);
        provider.assert_token_only_as_bearer();
        assert!(!served.log().contains(TOKEN), "the log holds the token");
    }

    /// Invariant 2: a refusal by the list is named, and comes before the credential is read: with
    /// the mounted item gone, an unlisted `uid` is still `forbidden` by name while a listed one
    /// is `unavailable`. An operation the list does not name is not bounded by it.
    #[tokio::test(flavor = "multi_thread")]
    async fn sr_the_allow_list_refuses_before_the_credential_and_bounds_only_its_operations() {
        let provider = Provider::new();
        let served = Served::start_with(
            &provider,
            json!({"allowed_values": {"prometheus.query": {"uid": ["fixture-prom"]}}}),
        )
        .unwrap();
        let revision = served.revision().await;
        let (status, answer, _) = served
            .invoke(
                "/v1alpha2/invoke",
                &revision,
                "unnamed-op",
                "prometheus.query_range",
                json!({"uid": "fixture-other", "query": "up", "start": "1", "end": "2", "step": "1"}),
            )
            .await;
        assert_eq!(
            status,
            200,
            "an unnamed operation is bounded: {answer}\n{}",
            served.log()
        );
        assert_eq!(provider.requests().len(), 1);

        // The Secret volume no longer holds the item: `..data` swapped onto a stamp without it.
        let stamp = "..2026_11_11_00_00_00.000000002";
        let alias = served.source.join(stamp).join(SERVED_INSTANCE);
        fs::create_dir_all(&alias).unwrap();
        for directory in [served.source.join(stamp), alias] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        symlink(stamp, served.source.join("..data_tmp")).unwrap();
        fs::rename(
            served.source.join("..data_tmp"),
            served.source.join("..data"),
        )
        .unwrap();
        let (status, answer, _) = served
            .invoke(
                "/v1alpha2/invoke",
                &revision,
                "outside-no-credential",
                "prometheus.query",
                json!({"uid": "fixture-other", "query": "up"}),
            )
            .await;
        assert_eq!(status, 403, "{answer}\n{}", served.log());
        assert_eq!(answer["error"]["code"], "forbidden");
        assert_eq!(
            answer["error"]["message"],
            "input `uid` is not one of the instance's allowed values"
        );
        let (status, answer, _) = served
            .invoke(
                "/v1alpha2/invoke",
                &revision,
                "listed-no-credential",
                "prometheus.query",
                json!({"uid": "fixture-prom", "query": "up"}),
            )
            .await;
        assert_eq!(answer["error"]["code"], "unavailable", "{status} {answer}");
        assert_eq!(provider.requests().len(), 1, "nothing more was sent");
    }

    /// Invariant 2: start refuses a malformed `catalog.allowed_values`, never serving it as an
    /// absent (admit-all) list.
    #[tokio::test(flavor = "multi_thread")]
    async fn sr_start_refuses_a_malformed_allow_list() {
        let provider = Provider::new();
        let mut served_anyway = Vec::new();
        for allowed in [
            json!(null),
            json!([]),
            json!({"prometheus.query": null}),
            json!({"prometheus.query": ["fixture-prom"]}),
            json!({"prometheus.query": {"uid": null}}),
            json!({"prometheus.query": {"uid": "fixture-prom"}}),
            json!({"prometheus.query": {"uid": [""]}}),
            json!({"prometheus.query": {"uid": ["fixture-prom", "fixture-prom"]}}),
            json!({"prometheus.query": {"path:uid": ["fixture-prom"]}}),
            json!({"prometheus.query": {"UID": ["fixture-prom"]}}),
            json!({"Prometheus.Query": {"uid": ["fixture-prom"]}}),
            json!({"prometheus.write": {"uid": ["fixture-prom"]}}),
        ] {
            if let Ok(served) = Served::start_with(&provider, json!({"allowed_values": allowed})) {
                drop(served);
                served_anyway.push(allowed.to_string());
            }
        }
        // A repeated key must not let a later empty list replace a non-empty one (admit-all).
        for raw in [
            r#"{"prometheus.query": {"uid": ["fixture-prom"]}, "prometheus.query": {"uid": []}}"#,
            r#"{"prometheus.query": {"uid": ["fixture-prom"], "uid": []}}"#,
        ] {
            if Served::start_with(&provider, json!({RAW_ALLOWED: raw})).is_ok() {
                served_anyway.push(raw.to_owned());
            }
        }
        // A misspelt member is not silently an absent list.
        if Served::start_with(
            &provider,
            json!({"allowed_value": {"prometheus.query": {"uid": ["fixture-prom"]}}}),
        )
        .is_ok()
        {
            served_anyway.push("allowed_value (misspelt)".to_owned());
        }
        assert!(
            served_anyway.is_empty(),
            "start served a malformed list: {served_anyway:?}"
        );
        assert!(provider.requests().is_empty());
    }

    /// Invariant 2, what a repeated key does once served: a configuration that lists
    /// `fixture-prom` for `prometheus.query` and repeats the key with an empty list must not serve
    /// an unlisted data source.
    #[tokio::test(flavor = "multi_thread")]
    async fn sr_a_repeated_key_does_not_open_the_allow_list() {
        let mut reached = Vec::new();
        for raw in [
            r#"{"prometheus.query": {"uid": ["fixture-prom"]}, "prometheus.query": {"uid": []}}"#,
            r#"{"prometheus.query": {"uid": ["fixture-prom"], "uid": []}}"#,
        ] {
            let provider = Provider::new();
            let Ok(served) = Served::start_with(&provider, json!({RAW_ALLOWED: raw})) else {
                continue;
            };
            let revision = served.revision().await;
            let (status, answer, _) = served
                .invoke(
                    "/v1alpha2/invoke",
                    &revision,
                    "unlisted",
                    "prometheus.query",
                    json!({"uid": "fixture-other", "query": "up"}),
                )
                .await;
            if !provider.requests().is_empty() {
                reached.push(format!("{raw} -> {status} {}", answer["status"]));
            }
        }
        assert!(
            reached.is_empty(),
            "an unlisted uid reached Grafana:\n{}",
            reached.join("\n")
        );
    }

    /// Invariant 1 on the served route: the token read from
    /// `<directory>/grafana.<instance>/<profile id>` goes to Grafana as `Authorization: Bearer`
    /// only, and neither a success, a 401, a 500 nor a size refusal puts it in an answer or the
    /// log.
    #[tokio::test(flavor = "multi_thread")]
    async fn sr_the_token_is_only_ever_the_bearer_header() {
        let provider = Provider::new();
        let served = Served::start_with(&provider, json!({})).unwrap();
        let revision = served.revision().await;
        for (n, uid) in ["fixture-prom", DENIED, BROKEN, CHUNKED_OVER]
            .into_iter()
            .enumerate()
        {
            let (_, answer, raw) = served
                .invoke(
                    "/v1alpha2/invoke",
                    &revision,
                    &format!("token-{n}"),
                    "prometheus.query",
                    json!({"uid": uid, "query": "up"}),
                )
                .await;
            assert!(
                !contains(&raw, TOKEN),
                "{uid}: the answer holds the token: {answer}"
            );
        }
        assert_eq!(provider.requests().len(), 4);
        provider.assert_token_only_as_bearer();
        assert!(!served.log().contains(TOKEN), "the log holds the token");
    }
}
