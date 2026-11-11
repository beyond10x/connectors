//! Security review, second pass, of story:catalog-grafana-reads and
//! story:catalog-grafana-datasource-proxy-reads: integrity conformance checks of the fixes made
//! after the first pass.
//!
//! A. An added operation's parameter `pattern` (`^[A-Za-z0-9_-]{1,40}$` on the three proxy
//!    reads' `uid`) is held on the exact value sent, before any request, on every proxy read and on
//!    the definition route; a bundle whose pattern is malformed, unanchored or oversized does not
//!    load; and a pattern accepted as anchored holds the whole value.
//! B. `catalog.allowed_values` refuses a repeated key at either level, whatever YAML spelling
//!    (quoting, escapes, a tag, an alias) produces the repeat.
//!
//! Every id, name and token is synthetic. No live Grafana, no network.
#![cfg(unix)]
use connectors_catalog::{bundle, definition, inventory};
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{fs, io::Write as _, path::Path, process::Command, sync::Mutex};

const BASE: &str = "/api";
const UID_RULE: &str = "^[A-Za-z0-9_-]{1,40}$";
const UPSTREAM: &str = "../grafana/upstream";
const DEFINITION: &str =
    "../../crates/connectors-catalog/tests/fixtures/definitions/connectors/grafana.yaml";
const PROXY_OPERATIONS: [&str; 3] = [
    "datasourceProxyPrometheusQuery",
    "datasourceProxyPrometheusQueryRange",
    "datasourceProxyLokiQueryRange",
];
const PROXY_READS: [&str; 3] = [
    "prometheus.query",
    "prometheus.query_range",
    "loki.query_range",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/grafana/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "grafana").unwrap()
}

/// The committed bundle with the `uid` pattern of `datasourceProxyPrometheusQuery` replaced.
fn with_uid_pattern(pattern: &str) -> bundle::Bundle {
    let mut bundle = committed();
    let operation = bundle
        .inventory
        .operations
        .iter_mut()
        .find(|o| o.operation_id.as_deref() == Some("datasourceProxyPrometheusQuery"))
        .unwrap();
    let uid = operation
        .parameters
        .iter_mut()
        .find(|p| p.name == "uid")
        .unwrap();
    uid.pattern = Some(pattern.to_owned());
    bundle
}

/// One GET as sent: its path segments.
struct Recorder {
    sent: Mutex<Vec<Vec<String>>>,
}
impl Recorder {
    fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }
    fn count(&self) -> usize {
        self.sent.lock().unwrap().len()
    }
    fn last(&self) -> Vec<String> {
        self.sent.lock().unwrap().last().cloned().unwrap()
    }
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Recorder {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.sent
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: br#"{"data":{"result":[],"resultType":"vector"},"status":"success"}"#.to_vec(),
        })
    }
}

fn proxy_input(read: &str, uid: Value) -> Value {
    match read {
        "prometheus.query" => json!({"uid": uid, "query": "up"}),
        "prometheus.query_range" => {
            json!({"uid": uid, "query": "up", "start": "1", "end": "2", "step": "1"})
        }
        _ => json!({"uid": uid, "query": "up", "start": "1", "end": "2", "limit": 1}),
    }
}

/// The `uid`s outside Grafana's rule: separators, a trailing or leading line break (which a
/// regular expression whose `$` matches before a final newline would admit), non-ASCII letters,
/// digits and hyphens that a Unicode class would admit, the length bound plus one, the empty
/// string, dot segments, and values that are no string.
fn outside_the_rule() -> Vec<Value> {
    let mut uids: Vec<Value> = [
        "fixture-prom\n",
        "fixture-prom\r\n",
        "\nfixture-prom",
        "fixture-prom\n/../../../api/admin/settings",
        "fixt\u{fc}re",
        "\u{ff46}ixture",
        "fixture\u{2010}prom",
        "\u{663}",
        "fixture\u{200b}prom",
        "",
        ".",
        "..",
        "a.b",
        "a b",
        "a/b",
        "a\\b",
        "a%2Fb",
        "a\u{0}",
        "a\t",
    ]
    .into_iter()
    .map(Value::from)
    .collect();
    uids.push(Value::from("a".repeat(41)));
    uids.extend([
        json!(1.5),
        json!(1e3),
        json!(u64::MAX as f64),
        json!(true),
        json!(null),
        json!(["fixture-prom"]),
        json!({"fixture-prom": true}),
    ]);
    uids
}

/// Finding A, on the shipped selection: every value outside Grafana's uid rule is refused on
/// each of the three proxy reads with no request made, and every value inside it is sent as
/// exactly the one segment after `/datasources/proxy/uid/`.
#[tokio::test]
async fn r2_a_uid_outside_grafanas_rule_is_refused_before_any_request_on_every_proxy_read() {
    let engine = Engine::new(&committed(), BASE, &shipped()).unwrap();
    let http = Recorder::new();
    let mut admitted = Vec::new();
    for read in PROXY_READS {
        for uid in outside_the_rule() {
            let before = http.count();
            let outcome = engine
                .read(&http, "fixture", read, proxy_input(read, uid.clone()))
                .await;
            if outcome.is_ok() || http.count() != before {
                admitted.push(format!("{read} {uid}"));
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "admitted outside the rule: {admitted:?}"
    );
    for read in PROXY_READS {
        for uid in ["fixture-prom", "A-_9", &"a".repeat(40), "0"] {
            engine
                .read(&http, "fixture", read, proxy_input(read, json!(uid)))
                .await
                .unwrap_or_else(|e| panic!("{read} `{uid}` refused: {}", e.message));
            let segments = http.last();
            assert_eq!(
                &segments[..4],
                ["datasources", "proxy", "uid", uid],
                "{read} `{uid}`: {segments:?}"
            );
        }
        // An integer is held to the rule as the text it is sent as, and that text is the segment.
        for (uid, text) in [(json!(42), "42"), (json!(-1), "-1")] {
            let before = http.count();
            if engine
                .read(&http, "fixture", read, proxy_input(read, uid.clone()))
                .await
                .is_ok()
            {
                assert_eq!(http.count(), before + 1);
                assert_eq!(http.last()[3], text, "{read} {uid}");
            }
        }
    }
}

/// Finding A, at bundle load: a `uid` pattern that is not a regular expression, not anchored at
/// either end, ends in an escaped `$`, is over 256 bytes or is empty refuses the engine.
#[test]
fn r2_engine_refuses_a_bundle_whose_pattern_is_malformed_unanchored_or_oversized() {
    let oversized = format!("^{}$", "a".repeat(inventory::PATTERN_MAX_BYTES - 1));
    assert_eq!(oversized.len(), inventory::PATTERN_MAX_BYTES + 1);
    let fits = format!("^{}$", "a".repeat(inventory::PATTERN_MAX_BYTES - 2));
    assert!(Engine::new(&with_uid_pattern(&fits), BASE, &shipped()).is_ok());
    let mut loaded = Vec::new();
    for pattern in [
        "[A-Za-z0-9_-]{1,40}",
        "^[A-Za-z0-9_-]{1,40}",
        "[A-Za-z0-9_-]{1,40}$",
        "^[A-Za-z0-9_-]{1,40}\\$",
        "^[A-Za-z0-9_-{1,40}$",
        "^(?<x>a)(?<x>b)$",
        "",
        oversized.as_str(),
    ] {
        if Engine::new(&with_uid_pattern(pattern), BASE, &shipped()).is_ok() {
            loaded.push(pattern.to_owned());
        }
    }
    assert!(loaded.is_empty(), "the engine loaded: {loaded:?}");
}

/// Finding A, the definition route: the served host lowers `connectors/grafana.yaml` with its
/// amendments at start, so the pattern must survive that pipeline into the engine it serves.
#[tokio::test]
async fn r2_the_definition_route_carries_the_uid_pattern_into_its_engine() {
    let temp = tempfile::tempdir().unwrap();
    let defs = temp.path().join("defs");
    fs::create_dir_all(defs.join("connectors")).unwrap();
    fs::create_dir_all(defs.join("upstream/grafana")).unwrap();
    fs::copy(
        root().join(DEFINITION),
        defs.join("connectors/grafana.yaml"),
    )
    .unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&fs::read(root().join(UPSTREAM).join("openapi3.json")).unwrap())
        .unwrap();
    fs::write(
        defs.join("upstream/grafana/openapi3.json.gz"),
        gzip.finish().unwrap(),
    )
    .unwrap();
    fs::copy(
        root().join(UPSTREAM).join("grafana.amendments.json"),
        defs.join("upstream/grafana/grafana.amendments.json"),
    )
    .unwrap();
    let scratch = temp.path().join("scratch");
    fs::create_dir_all(&scratch).unwrap();
    let out = temp.path().join("out");
    let lowered = definition::lower(&definition::Request {
        definition: &defs.join("connectors/grafana.yaml"),
        root: &defs,
        out: &out,
        scratch: &scratch,
    })
    .unwrap_or_else(|refusal| panic!("the definition lowers: {refusal}"));
    let bundle = bundle::load(&out, &lowered.provider).unwrap();
    for id in PROXY_OPERATIONS {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(id))
            .unwrap_or_else(|| panic!("`{id}` is not in the lowered bundle"));
        let uid = operation
            .parameters
            .iter()
            .find(|p| p.name == "uid")
            .unwrap();
        assert_eq!(uid.pattern.as_deref(), Some(UID_RULE), "`{id}`");
    }
    let selections: Vec<Selection> = serde_json::from_value(
        serde_json::from_slice::<Value>(&fs::read(&lowered.operations).unwrap()).unwrap()
            ["operations"]
            .clone(),
    )
    .unwrap();
    let engine = Engine::new(&bundle, "", &selections)
        .or_else(|_| Engine::new(&bundle, BASE, &selections))
        .unwrap_or_else(|e| panic!("the lowered selection loads: {}", e.message));
    let http = Recorder::new();
    for read in PROXY_READS {
        for uid in ["a/b", "a\\b", "fixture-prom\n", "fixt\u{fc}re"] {
            let outcome = engine
                .read(&http, "fixture", read, proxy_input(read, json!(uid)))
                .await;
            assert!(outcome.is_err(), "{read} {uid:?} was answered");
        }
    }
    assert_eq!(
        http.count(),
        0,
        "a uid outside the rule reached the provider"
    );
}

/// Finding A, the anchoring contract (`amendment.yaml`: "the pipeline refuses one that ... is
/// not anchored at both ends"; `pattern_is_anchored`: "so it holds the whole value"). A pattern
/// that starts with `^` and ends with an unescaped `$` is not thereby anchored: a top-level
/// alternation leaves each branch with one anchor, `(?m)` makes `$` match at any line end, and
/// `(?x)` turns a final `#$` into a comment. Each is accepted today, and each admits a `uid`
/// holding `/`; loaded into the shipped engine, the first sends that `uid` to the provider.
#[tokio::test]
async fn r2_a_pattern_accepted_as_anchored_holds_the_whole_value() {
    const ESCAPE: &str = "fixture-prom/../../../api/admin/settings";
    let mut open = Vec::new();
    for (pattern, probe) in [
        ("^[A-Za-z0-9_-]{1,40}|x$", ESCAPE),
        (
            "^(?m)[A-Za-z0-9_-]{1,40}$",
            "fixture-prom\n/../../../api/admin/settings",
        ),
        ("^[A-Za-z0-9_-]{1,40}(?x)#$", ESCAPE),
    ] {
        let accepted = inventory::pattern_is_anchored(pattern);
        let matched = inventory::pattern_matches(pattern, probe);
        if accepted && matched == Some(true) {
            open.push(format!("{pattern} accepted, matches {probe:?}"));
        }
    }
    let engine = Engine::new(
        &with_uid_pattern("^[A-Za-z0-9_-]{1,40}|x$"),
        BASE,
        &shipped(),
    );
    if let Ok(engine) = engine {
        let http = Recorder::new();
        let outcome = engine
            .read(
                &http,
                "fixture",
                "prometheus.query",
                json!({"uid": ESCAPE, "query": "up"}),
            )
            .await;
        if outcome.is_ok() && http.count() == 1 {
            open.push(format!(
                "engine loaded `^[A-Za-z0-9_-]{{1,40}}|x$` and sent {:?}",
                http.last()
            ));
        }
    }
    assert!(
        open.is_empty(),
        "a pattern accepted as anchored does not hold the whole value:\n{}",
        open.join("\n")
    );
}

/// Finding B, through the served binary's own configuration reader: a repeated
/// `catalog.allowed_values` key, at the operation or the input level or the member itself,
/// refuses the configuration however the YAML spells the repeat (plain and quoted, an escape, a
/// `!!str` tag, an alias), while the same configuration without the repeat parses and fails
/// later, at the absent definition.
#[test]
fn r2_a_repeated_allowed_values_key_is_refused_in_any_yaml_spelling() {
    const SCHEMA: &str = "configuration does not match the adapter schema";
    let temp = tempfile::tempdir().unwrap();
    let head = "service:\n  instance: grafana.fixture\n  listen: 127.0.0.1:0\n  service_credential: {kind: file, path: /nonexistent/service-token}\n  state: /nonexistent/state\ncredential: {binding: mounted_secret, directory: /nonexistent/credential}\ncatalog:\n  definition: /nonexistent/defs/connectors/grafana.yaml\n  definitions_root: /nonexistent/defs\n  instance: fixture\n";
    let run = |name: &str, catalog_tail: &str| -> (bool, String) {
        let path = temp.path().join(name);
        fs::write(&path, format!("{head}{catalog_tail}")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--config")
            .arg(&path)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    let (ok, stderr) = run(
        "control.yaml",
        "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    prometheus.query_range: {uid: [fixture-prom]}\n",
    );
    assert!(!ok);
    assert!(
        !stderr.contains(SCHEMA),
        "the control must parse and fail later: {stderr}"
    );
    let mut parsed = Vec::new();
    for (name, tail) in [
        (
            "op-double-quoted",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    \"prometheus.query\": {uid: []}\n",
        ),
        (
            "op-single-quoted",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    'prometheus.query': {uid: []}\n",
        ),
        (
            "op-escaped",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    \"prometheus\\x2equery\": {uid: []}\n",
        ),
        (
            "op-explicit-key",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    ? prometheus.query\n    : {uid: []}\n",
        ),
        (
            "op-alias",
            "  allowed_values:\n    &op prometheus.query: {uid: [fixture-prom]}\n    *op : {uid: []}\n",
        ),
        (
            "op-tagged",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    !!str prometheus.query: {uid: []}\n",
        ),
        (
            "input-unicode-escape",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom], \"\\u0075id\": []}\n",
        ),
        (
            "input-block",
            "  allowed_values:\n    prometheus.query:\n      uid: [fixture-prom]\n      'uid': []\n",
        ),
        (
            "member-repeated",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n  allowed_values: {}\n",
        ),
    ] {
        let (ok, stderr) = run(&format!("{name}.yaml"), tail);
        if ok || !stderr.contains(SCHEMA) {
            parsed.push(format!("{name}: {}", stderr.trim()));
        }
    }
    assert!(
        parsed.is_empty(),
        "a repeated key was read past the configuration reader:\n{}",
        parsed.join("\n")
    );
    // Each spelling with its second key changed to a distinct one parses, so the refusals above
    // are the repeat's and not a syntax error of the case.
    let mut refused = Vec::new();
    for (name, tail) in [
        (
            "op-double-quoted-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    \"prometheus.query_range\": {uid: []}\n",
        ),
        (
            "op-single-quoted-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    'prometheus.query_range': {uid: []}\n",
        ),
        (
            "op-escaped-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    \"prometheus\\x2equery_range\": {uid: []}\n",
        ),
        (
            "op-explicit-key-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    ? prometheus.query_range\n    : {uid: []}\n",
        ),
        (
            "op-alias-twin",
            "  allowed_values:\n    &op prometheus.query: {uid: [fixture-prom]}\n    prometheus.query_range: {uid: []}\n",
        ),
        (
            "op-tagged-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom]}\n    !!str prometheus.query_range: {uid: []}\n",
        ),
        (
            "input-unicode-escape-twin",
            "  allowed_values:\n    prometheus.query: {uid: [fixture-prom], \"\\u0071uery\": []}\n",
        ),
        (
            "input-block-twin",
            "  allowed_values:\n    prometheus.query:\n      uid: [fixture-prom]\n      'query': []\n",
        ),
    ] {
        let (_, stderr) = run(&format!("{name}.yaml"), tail);
        if stderr.contains(SCHEMA) {
            refused.push(name);
        }
    }
    assert!(refused.is_empty(), "twins that do not parse: {refused:?}");
}
