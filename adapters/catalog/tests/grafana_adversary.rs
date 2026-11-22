//! Adversary cases for story:catalog-grafana-reads and
//! story:catalog-grafana-datasource-proxy-reads.
//!
//! Each case drives the provider child against a disposable HTTPS fixture that records the whole
//! request head and answers with what the case hands it: a recorded body in the order Grafana,
//! Prometheus and Loki write it, an answer at or over the size limit with or without a
//! `Content-Length`, a status. Every id, name and token is synthetic. No live Grafana, no network.
#![cfg(unix)]
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

const INSTANCE: &str = "fixture-grafana";
const TOKEN: &str = "fixture-grafana-service-account-token";
const GUIDE: &str = "../../docs/catalog-grafana.md";

/// How the fixture frames an answer's body.
#[derive(Clone, Copy)]
enum Length {
    /// `Content-Length` is the body's length.
    Exact,
    /// No `Content-Length`: the body ends when the connection closes.
    Omitted,
    /// `Content-Length` claims this many bytes; fewer are sent before the close.
    Claimed(usize),
}

struct Answer {
    status: u16,
    body: Vec<u8>,
    length: Length,
}

impl Answer {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            length: Length::Exact,
        }
    }
    fn status(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.as_bytes().to_vec(),
            length: Length::Exact,
        }
    }
}

type Responder = Arc<dyn Fn(&str, &str) -> Answer + Send + Sync>;

/// Method, request target and the whole request head of each request the fixture answered.
type Requests = Arc<Mutex<Vec<(String, String, String)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}

/// The guide's pinned-instance configuration, the one a reader is told to write.
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

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}

impl Provider {
    fn new(responder: Responder) -> Self {
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
                    let (route, query) = target.split_once('?').unwrap_or((&target, ""));
                    let answer = if authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
                        Answer::status(401, r#"{"message":"invalid API key"}"#)
                    } else {
                        responder(route, query)
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target.clone(), head.clone()));
                    let length = match answer.length {
                        Length::Exact => format!("Content-Length: {}\r\n", answer.body.len()),
                        Length::Omitted => String::new(),
                        Length::Claimed(claimed) => format!("Content-Length: {claimed}\r\n"),
                    };
                    let head = format!(
                        "HTTP/1.1 {} fixture\r\nContent-Type: application/json\r\n{length}Connection: close\r\n\r\n",
                        answer.status
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer.body).await;
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
            config: path,
            requests,
        }
    }

    fn child(&self) -> Child {
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
        Child::spawn(&Adapter {
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
        })
        .unwrap()
    }

    fn requests(&self) -> Vec<(String, String, String)> {
        self.requests.lock().unwrap().clone()
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

fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap()),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )
}

fn not_found() -> Answer {
    Answer::status(404, r#"{"message":"Not found"}"#)
}

/// Each of the eight reads with an input that reaches the fixture.
fn eight_reads() -> [(&'static str, Value); 8] {
    [
        ("alert_rules.list", json!({})),
        (
            "annotations.list",
            json!({"from": 1_780_000_000_000_u64, "to": 1_780_000_600_000_u64, "limit": 3}),
        ),
        (
            "dashboards.search",
            json!({"type": "dash-db", "limit": 2, "page": 1}),
        ),
        ("dashboard.get", json!({"uid": "fixture-dash-1"})),
        ("datasources.list", json!({})),
        (
            "prometheus.query",
            json!({"uid": "fixture-prom", "query": "up"}),
        ),
        (
            "prometheus.query_range",
            json!({"uid": "fixture-prom", "query": "up", "start": "1", "end": "2", "step": "1"}),
        ),
        (
            "loki.query_range",
            json!({"uid": "fixture-loki", "query": "{app=\"api\"}", "start": "1", "end": "2",
                   "limit": 10}),
        ),
    ]
}

/// The pinned `uid` path parameter of a proxy read cannot leave its one path segment. Since the
/// review fix for finding A, a `uid` outside Grafana's uid rule (letters, digits, `-`, `_`, at
/// most 40), which every one holding `/`, `?`, `#`, `%`, `\` or a space is, is refused as
/// `invalid_input` before any request, so it reaches neither another proxy sub-path nor another
/// Grafana API, whether or not the server decodes `%2F` before routing. Should any of them still
/// be sent, it must be percent-encoded inside its one segment and decode to itself. `.`, `..` and
/// the empty `uid` (which would make `/uid//api/v1/query`) are refused before any request.
#[test]
fn a_proxy_uid_cannot_leave_its_path_segment() {
    let provider = Provider::new(Arc::new(|_, _| Answer::ok(r#"{"status":"success"}"#)));
    let mut child = provider.child();
    let prefix = "/api/datasources/proxy/uid/";
    for uid in [
        "fixture-prom/../../../api/admin/settings",
        "fixture-prom%2F..%2F..%2Fapi%2Fuser",
        "fixture-prom?query=up#",
        "fixture-prom\\..\\..\\api",
        "fixture prom/../loki/api/v1/query_range",
        "../../../api/user",
    ] {
        let before = provider.requests().len();
        let outcome = attempt(
            &mut child,
            "prometheus.query",
            &json!({"uid": uid, "query": "up"}),
        );
        let requests = provider.requests();
        assert!(
            requests.len() <= before + 1,
            "`{uid}`: more than one request"
        );
        if requests.len() == before {
            assert!(
                matches!(outcome, Err(Failure::InvalidInput)),
                "`{uid}` was neither sent nor refused as invalid input: {outcome:?}"
            );
            continue;
        }
        let target = &requests[before].1;
        let (path, query) = target.split_once('?').expect("a query");
        assert_eq!(query, "query=up", "`{uid}` changed the query: {target}");
        let segment = path
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix("/api/v1/query"))
            .unwrap_or_else(|| panic!("`{uid}` left the proxy route: {target}"));
        assert!(
            !segment.contains(['/', '?', '#', '\\', ' ']),
            "`{uid}` is not one segment: {target}"
        );
        let decoded: String = url::form_urlencoded::parse(format!("x={segment}").as_bytes())
            .next()
            .unwrap()
            .1
            .into_owned();
        assert_eq!(decoded, uid, "`{uid}` was not sent as itself: {target}");
    }
    for uid in ["..", ".", ""] {
        let outcome = attempt(
            &mut child,
            "prometheus.query",
            &json!({"uid": uid, "query": "up"}),
        );
        assert!(outcome.is_err(), "`{uid}` was answered: {outcome:?}");
    }
    let sent = provider.requests().len();
    assert_eq!(
        sent, 0,
        "a uid outside Grafana's rule, `.`, `..` or the empty uid reached the fixture"
    );
}

/// A PromQL or LogQL query holding form-reserved characters (`+`, `&`, `#`, `%`, `=`, `;`) goes
/// out form-encoded so the proxied data source reads exactly the caller's query: a raw `+` would
/// arrive as a space, a raw `&` would end the parameter.
#[test]
fn reserved_characters_in_a_query_reach_the_data_source_unchanged() {
    let provider = Provider::new(Arc::new(|_, _| Answer::ok(r#"{"status":"success"}"#)));
    let mut child = provider.child();
    let promql = "sum(rate(x{a=\"b&c\"}[5m])) + 1 % 2 # comment; step=1&query=up";
    let logql = "{app=\"api\"} |= \"a+b&c=d#e%f\"";
    for (operation, input, query) in [
        (
            "prometheus.query",
            json!({"uid": "fixture-prom", "query": promql}),
            promql,
        ),
        (
            "loki.query_range",
            json!({"uid": "fixture-loki", "query": logql, "start": "1", "end": "2", "limit": 10}),
            logql,
        ),
    ] {
        let before = provider.requests().len();
        attempt(&mut child, operation, &input).unwrap();
        let target = provider.requests()[before].1.clone();
        let (_, raw) = target.split_once('?').unwrap();
        let pairs: Vec<(String, String)> = url::form_urlencoded::parse(raw.as_bytes())
            .into_owned()
            .collect();
        let queries: Vec<&str> = pairs
            .iter()
            .filter(|(name, _)| name == "query")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(queries, [query], "`{operation}`: {target}");
        assert!(
            !pairs.iter().any(|(name, _)| name == "step"),
            "`{operation}` gained a parameter: {target}"
        );
    }
}

/// The token is sent once, as `Authorization: Bearer <token>`, and in no other header, cookie or
/// the request target, on each of the eight reads.
#[test]
fn the_token_travels_only_in_the_authorization_header() {
    let provider = Provider::new(Arc::new(|_, _| Answer::ok("[]")));
    let mut child = provider.child();
    for (operation, input) in eight_reads() {
        let before = provider.requests().len();
        attempt(&mut child, operation, &input)
            .unwrap_or_else(|failure| panic!("`{operation}`: {failure:?}"));
        let (method, target, head) = provider.requests()[before].clone();
        assert_eq!(method, "GET", "`{operation}`");
        assert!(
            !target.contains(TOKEN),
            "`{operation}` target carries the token"
        );
        let carrying: Vec<String> = head
            .lines()
            .skip(1)
            .filter(|line| line.contains(TOKEN))
            .map(|line| line.split(':').next().unwrap().to_ascii_lowercase())
            .collect();
        assert_eq!(carrying, ["authorization"], "`{operation}` headers");
        assert!(
            !head
                .lines()
                .any(|line| line.to_ascii_lowercase().starts_with("cookie:")),
            "`{operation}` sends a cookie"
        );
    }
}

/// A Prometheus answer of exactly `RESPONSE_LIMIT` bytes.
fn at_limit(extra: usize) -> String {
    let head = r#"{"data":{"result":[],"resultType":"vector"},"pad":""#;
    let tail = r#"","status":"success"}"#;
    let pad = connectors_core::RESPONSE_LIMIT + extra - head.len() - tail.len();
    format!("{head}{}{tail}", "a".repeat(pad))
}

/// The answer-size bound is a refusal, never a cut: an answer of exactly the limit comes back
/// whole; one byte over with no `Content-Length` (the body ends at the close) is `capacity`; an
/// answer whose connection closes before its `Content-Length` is never reported complete.
#[test]
fn the_answer_bound_refuses_and_never_cuts() {
    let provider = Provider::new(Arc::new(|route, _| {
        if route.contains("/uid/at-limit/") {
            Answer::ok(at_limit(0))
        } else if route.contains("/uid/over-unframed/") {
            Answer {
                status: 200,
                body: at_limit(1).into_bytes(),
                length: Length::Omitted,
            }
        } else if route.contains("/uid/short/") {
            Answer {
                status: 200,
                body: br#"{"data":{"result":[]},"status":"success"}"#.to_vec(),
                length: Length::Claimed(4096),
            }
        } else {
            not_found()
        }
    }));
    let mut child = provider.child();
    let raw = attempt(
        &mut child,
        "prometheus.query",
        &json!({"uid": "at-limit", "query": "up"}),
    )
    .expect("an answer of exactly the limit is not over it");
    let field = format!("\"body\":{}", at_limit(0));
    assert!(
        raw.windows(field.len()).any(|w| w == field.as_bytes()),
        "the at-limit answer did not come back whole"
    );
    let over = attempt(
        &mut child,
        "loki.query_range",
        &json!({"uid": "over-unframed", "query": "{a=\"b\"}", "start": "1", "end": "2",
               "limit": 5000}),
    );
    assert!(
        matches!(over, Err(Failure::Capacity)),
        "an unframed answer one byte over: {:?}",
        over.as_ref().map(|raw| raw.len())
    );
    let short = attempt(
        &mut child,
        "prometheus.query",
        &json!({"uid": "short", "query": "up"}),
    );
    assert!(
        short.is_err(),
        "an answer shorter than its Content-Length was reported complete: {:?}",
        short.map(|raw| String::from_utf8_lossy(&raw).into_owned())
    );
}

/// Loki's `limit` is held to 1 to 5000 whatever form the caller sends it in: a decimal string, a
/// padded string, a float literal, an exponent, a negative and a number past `u64` are refused
/// before any request; 1 and 5000 themselves are sent.
#[test]
fn loki_limit_outside_one_to_5000_in_any_form_is_refused_before_any_request() {
    let provider = Provider::new(Arc::new(|_, _| Answer::ok(r#"{"status":"success"}"#)));
    let mut child = provider.child();
    let input = |limit: Value| {
        json!({"uid": "fixture-loki", "query": "{a=\"b\"}", "start": "1", "end": "2",
               "limit": limit})
    };
    let huge: Value = serde_json::from_str("18446744073709551616").unwrap();
    let float: Value = serde_json::from_str("5000.0").unwrap();
    let exponent: Value = serde_json::from_str("1e3").unwrap();
    for limit in [
        json!("5001"),
        json!("0005001"),
        json!("-1"),
        json!(-1),
        json!("0"),
        json!(""),
        json!(" 100"),
        json!("100 "),
        float,
        exponent,
        huge,
        json!(null),
        json!([100]),
    ] {
        let outcome = attempt(&mut child, "loki.query_range", &input(limit.clone()));
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "limit {limit}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
    for limit in [json!(1), json!("5000")] {
        attempt(&mut child, "loki.query_range", &input(limit.clone()))
            .unwrap_or_else(|failure| panic!("limit {limit}: {failure:?}"));
    }
    let limits: Vec<String> = provider
        .requests()
        .into_iter()
        .map(|(_, target, _)| {
            url::form_urlencoded::parse(target.split_once('?').unwrap().1.as_bytes())
                .find(|(name, _)| name == "limit")
                .unwrap()
                .1
                .into_owned()
        })
        .collect();
    assert_eq!(limits, ["1", "5000"]);
}

/// The guide: a data source the token's role cannot query answers `forbidden` (Grafana's `403`),
/// and Prometheus's refusal of more than 11,000 points (`400`) is `invalid_input`.
#[test]
fn proxy_refusals_answer_as_the_guide_says() {
    let provider = Provider::new(Arc::new(|route, _| {
        if route.contains("/uid/denied/") {
            Answer::status(403, r#"{"message":"Access denied to data source"}"#)
        } else if route.contains("/uid/too-many-points/") {
            Answer::status(
                400,
                r#"{"status":"error","errorType":"bad_data","error":"exceeded maximum resolution of 11,000 points per timeseries. Try decreasing the query resolution (?step=XX)"}"#,
            )
        } else {
            not_found()
        }
    }));
    let mut child = provider.child();
    let denied = attempt(
        &mut child,
        "loki.query_range",
        &json!({"uid": "denied", "query": "{a=\"b\"}", "start": "1", "end": "2", "limit": 1}),
    );
    // The provider's own `403` is `provider_forbidden`, the upstream half of `forbidden`.
    assert!(
        matches!(denied, Err(Failure::ProviderForbidden)),
        "{denied:?}"
    );
    let points = attempt(
        &mut child,
        "prometheus.query_range",
        &json!({"uid": "too-many-points", "query": "up", "start": "0", "end": "1000000",
               "step": "1"}),
    );
    assert!(matches!(points, Err(Failure::InvalidInput)), "{points:?}");
}

/// No write of the bundle and no unselected read, the generic proxy route among them, is
/// reachable by its `operationId` or its path through the child: each is refused before any
/// request.
#[test]
fn no_bundle_write_or_generic_proxy_route_is_reachable() {
    let provider = Provider::new(Arc::new(|_, _| Answer::ok("{}")));
    let mut child = provider.child();
    for (operation, input) in [
        ("deleteDashboardByUID", json!({"uid": "fixture-dash-1"})),
        ("postAnnotation", json!({"body": {"text": "x"}})),
        ("postDashboard", json!({"body": {"dashboard": {}}})),
        (
            "datasourceProxyGETByUIDcalls",
            json!({"uid": "fixture-prom", "datasource_proxy_route": "api/v1/status/config"}),
        ),
        (
            "datasourceProxyPOSTByUIDcalls",
            json!({"uid": "fixture-prom", "datasource_proxy_route": "api/v1/admin/tsdb/snapshot"}),
        ),
        (
            "callDatasourceResourceWithUID",
            json!({"uid": "fixture-prom", "datasource_proxy_route": "x"}),
        ),
        (
            "datasourceProxyPrometheusQuery",
            json!({"uid": "fixture-prom", "query": "up"}),
        ),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(outcome.is_err(), "`{operation}` was answered");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}

/// Acceptance (both stories): each read returns the JSON value it was answered, written as the
/// guide says (docs/catalog-grafana.md): compact, object keys sorted, no escape where the
/// character itself is valid JSON. The bodies here are recorded in the form each upstream writes
/// them: Prometheus and Loki put `status` before `data` and `resultType` before `result`
/// (`web/api/v1/api.go`'s `Response`), Grafana's dashboard answer puts `meta` before `dashboard`
/// and Go's `encoding/json` escapes `&`, `<` and `>` as six-character backslash-`u` escapes
/// (`u0026`, `u003c`, `u003e`). So the value is the same and the bytes are not: the engine
/// re-encodes every 2xx JSON answer, for every catalog provider. (Coordinator decision on the
/// review finding: that re-encoding stays; this case pins it instead of byte equality.)
#[test]
fn recorded_upstream_bodies_come_back_as_the_same_value_compact_and_sorted() {
    const PROMETHEUS: &str = r#"{"status":"success","data":{"resultType":"vector","result":[{"metric":{"__name__":"up","job":"api"},"value":[1788822000,"1"]}]}}"#;
    // Go's HTML-safe escapes, built at run time: a backslash, then `u` and four hex digits.
    let go = |text: &str| {
        let backslash = char::from(0x5c_u8);
        text.replace('&', &format!("{backslash}u0026"))
            .replace('<', &format!("{backslash}u003c"))
    };
    let loki = go(
        r#"{"status":"success","data":{"resultType":"streams","result":[{"stream":{"app":"api"},"values":[["1788822000000000000","a < b"]]}],"stats":{}}}"#,
    );
    let dashboard = go(
        r#"{"meta":{"slug":"ingest-latency","url":"/d/fixture-dash-1/ingest-latency"},"dashboard":{"title":"Ingest & export","uid":"fixture-dash-1"}}"#,
    );
    assert!(loki.contains("u003c") && dashboard.contains("u0026"));
    let (served_loki, served_dashboard) = (loki.clone(), dashboard.clone());
    let provider = Provider::new(Arc::new(move |route, _| {
        if route.contains("/proxy/uid/fixture-prom/") {
            Answer::ok(PROMETHEUS)
        } else if route.contains("/proxy/uid/fixture-loki/") {
            Answer::ok(served_loki.clone())
        } else if route == "/api/dashboards/uid/fixture-dash-1" {
            Answer::ok(served_dashboard.clone())
        } else {
            not_found()
        }
    }));
    let mut child = provider.child();
    let mut changed = Vec::new();
    for (operation, input, served) in [
        (
            "prometheus.query",
            json!({"uid": "fixture-prom", "query": "up"}),
            PROMETHEUS,
        ),
        (
            "loki.query_range",
            json!({"uid": "fixture-loki", "query": "{app=\"api\"}", "start": "1", "end": "2",
                   "limit": 10}),
            loki.as_str(),
        ),
        (
            "dashboard.get",
            json!({"uid": "fixture-dash-1"}),
            dashboard.as_str(),
        ),
    ] {
        let raw = attempt(&mut child, operation, &input)
            .unwrap_or_else(|failure| panic!("`{operation}`: {failure:?}"));
        let result: Value = serde_json::from_slice(&raw).unwrap();
        let upstream: Value = serde_json::from_str(served).unwrap();
        // The same JSON value.
        if result["body"] != upstream {
            changed.push(format!(
                "`{operation}`: another value\n  served:   {served}\n  answered: {}",
                result["body"]
            ));
        }
        // Written as documented: compact, keys sorted, escapes only where JSON needs one; and so,
        // for these upstream-ordered, Go-escaped bodies, not the served bytes.
        let field = format!("\"body\":{}", sorted_compact(&upstream));
        if !raw.windows(field.len()).any(|w| w == field.as_bytes()) {
            changed.push(format!(
                "`{operation}`: not written compact with sorted keys\n  expected: {field}\n  output:   {}",
                String::from_utf8_lossy(&raw)
            ));
        }
        let served_field = format!("\"body\":{served}");
        if raw
            .windows(served_field.len())
            .any(|w| w == served_field.as_bytes())
        {
            changed.push(format!(
                "`{operation}`: the served bytes came back unchanged, so this case no longer pins the re-encoding"
            ));
        }
        let escape = format!("{}u00", char::from(0x5c_u8));
        if raw.windows(escape.len()).any(|w| w == escape.as_bytes()) {
            changed.push(format!("`{operation}`: an escape of `&` or `<` survived"));
        }
    }
    assert!(
        changed.is_empty(),
        "bodies not returned as the documented re-encoding of the same value:\n{}",
        changed.join("\n")
    );
}

/// `value` as the provider writes JSON: compact, every object's keys in byte order, strings as
/// `serde_json` escapes them (only `"`, the backslash and control characters). Built by hand so
/// the expectation does not depend on how `serde_json::Map` orders keys in this build.
fn sorted_compact(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let members: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        sorted_compact(&map[key])
                    )
                })
                .collect();
            format!("{{{}}}", members.join(","))
        }
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(sorted_compact)
                .collect::<Vec<_>>()
                .join(",")
        ),
        scalar => serde_json::to_string(scalar).unwrap(),
    }
}
