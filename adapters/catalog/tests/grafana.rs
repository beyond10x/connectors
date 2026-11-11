//! story:catalog-grafana-reads: Grafana alert rules, annotations, dashboards and data sources
//! through the catalog provider.
//!
//! The shipped selection is pinned by id, source `operationId`, method and path, and resolves
//! against the committed bundle compiled from Grafana's pinned OpenAPI 3 document; the guide cites
//! each row. Each read runs through the provider child against a disposable HTTPS fixture that
//! plays Grafana: the exact request (method, path, query, `Authorization: Bearer <token>`) is
//! asserted and the answer's body is the fixture's bytes, unchanged; the fixture's bodies are
//! already compact with sorted keys, the form the provider writes every JSON value in, so a real
//! Grafana body comes back as the same JSON value, not the same bytes (`grafana_adversary.rs`
//! checks that re-encoding). `dashboards.search` and
//! `annotations.list` each walk two pages and stop at the documented end, a page with fewer items
//! than `limit`. A served instance started from the definition file, with the token mounted as a
//! Secret volume presents it, describes exactly the eight reads and answers one of each kind.
//! Every id, name and token is synthetic. No live Grafana, no network.
//!
//! story:catalog-grafana-datasource-proxy-reads: three more reads reach Prometheus and Loki
//! through Grafana's data-source proxy with the same token. Each proxy sub-path is an
//! `add_operation` amendment to the pinned document, since the document declares the proxy only
//! as one `{datasource_proxy_route}` parameter. The exact proxied request, the unchanged answer,
//! the catalog's size refusal of an over-limit answer, Loki's `limit` maximum and the served
//! instance's `uid` allow-list are each checked against the fixture.
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

/// The pinned document's one server is `/api`, so every operation sits below `/api`.
const BASE: &str = "/api";
const INSTANCE: &str = "fixture-grafana";
const PROFILE: &str = "grafana.service-account";
/// The fictional service-account token the fixture accepts as `Authorization: Bearer <token>`.
const TOKEN: &str = "fixture-grafana-service-account-token";
/// The pinned source and its licence, relative to this crate.
const UPSTREAM: &str = "../grafana/upstream";
const SOURCE: &str = "openapi3.json";
/// The committed file: Grafana's document with one member removed ([`REMOVED`]).
const SOURCE_SHA256: &str = "be8923b42413a640f92af54a728f5b04f936a2851a04b8dfa277ae0db3bdebdc";
const SOURCE_BYTES: usize = 794_841;
/// Grafana's document as published at the URL below, before that removal.
const UPSTREAM_SHA256: &str = "89ff3e97e4d0e47ddd5a0bc757f2be2c909e655d275486a776a85fbde44dc661";
const UPSTREAM_BYTES: &str = "794,914 bytes";
/// The one member removed from the published document: Grafana's example service-account token,
/// which the repository's secrets scanner refuses. Its parent is left as exactly
/// [`REMOVED_PARENT`].
const REMOVED: &str = "/components/schemas/NewApiKeyResult/properties/key/example";
const REMOVED_PARENT: &str = "/components/schemas/NewApiKeyResult/properties/key";
const URL: &str = "https://raw.githubusercontent.com/grafana/grafana/v13.3.1/public/openapi3.json";
const LICENSE_SHA256: &str = "0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0";
/// Operations in the bundle (the pinned document's 314 and the three amended proxy reads), and
/// those the bundle cannot represent.
const OPERATIONS: usize = 317;
const UNSUPPORTED: usize = 0;
const GUIDE: &str = "../../docs/catalog-grafana.md";
/// The amendments that fix each proxy sub-path, beside the pinned source.
const AMENDMENTS: &str = "grafana.amendments.json";
/// Loki's documented maximum for `limit`: `max_entries_limit_per_query`, default 5000
/// (Loki v3.7.0, `docs/sources/shared/configuration.md`, `limits_config`).
const LOKI_LIMIT_MAXIMUM: u64 = 5000;

/// The shipped ids, their pinned `operationId` and path; every one a `GET` read. A renamed,
/// dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 5] = [
    (
        "alert_rules.list",
        "RouteGetAlertRules",
        "/api/v1/provisioning/alert-rules",
    ),
    ("annotations.list", "getAnnotations", "/api/annotations"),
    (
        "dashboard.get",
        "getDashboardByUID",
        "/api/dashboards/uid/{uid}",
    ),
    ("dashboards.search", "search", "/api/search"),
    ("datasources.list", "getDataSources", "/api/datasources"),
];

/// story:catalog-grafana-datasource-proxy-reads: the three reads through the data-source proxy,
/// each `operationId` added by the amendments and each path the proxy route of a Prometheus or
/// Loki query. A renamed, dropped or added id fails here.
const PROXY: [(&str, &str, &str); 3] = [
    (
        "loki.query_range",
        "datasourceProxyLokiQueryRange",
        "/api/datasources/proxy/uid/{uid}/loki/api/v1/query_range",
    ),
    (
        "prometheus.query",
        "datasourceProxyPrometheusQuery",
        "/api/datasources/proxy/uid/{uid}/api/v1/query",
    ),
    (
        "prometheus.query_range",
        "datasourceProxyPrometheusQueryRange",
        "/api/datasources/proxy/uid/{uid}/api/v1/query_range",
    ),
];

/// Every shipped read, sorted by id.
fn all_reads() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut all: Vec<_> = SHIPPED.iter().chain(PROXY.iter()).copied().collect();
    all.sort_unstable();
    all
}

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
    let file = shipped_file();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "grafana");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "grafana").unwrap()
}

/// Acceptance: exactly the five reads of story:catalog-grafana-reads and the three proxy reads of
/// story:catalog-grafana-datasource-proxy-reads, each `effect: read`, no write, each resolving to
/// the bundle's `operationId`, method and path.
#[test]
fn shipped_grafana_selections_are_exactly_the_eight_reads() {
    let selections = shipped();
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = all_reads().iter().map(|(id, ..)| *id).collect();
    assert_eq!(declared, expected);
    assert_eq!(selections.len(), SHIPPED.len() + PROXY.len());
    assert!(
        engine.declarations(&[Effect::Write]).is_empty(),
        "no write is selected"
    );
    for operation in shipped_file()["operations"].as_array().unwrap() {
        assert_eq!(operation["effect"], "read", "{operation}");
    }
    for (id, operation_id, path) in all_reads() {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert!(selection.guard.is_none(), "`{id}` carries no guard");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the pinned source lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

/// story:catalog-grafana-datasource-proxy-reads, acceptance: the definition exposes exactly the
/// three proxy reads beside the five, each `effect: read` under the one profile
/// `grafana.service-account`. Each is an operation the amendments add, recorded in the bundle,
/// with its inputs as the story lists them: `uid` in the path, the query parameters in the
/// Prometheus or Loki reference's names, and Loki's `limit` held to its documented maximum.
#[test]
fn the_proxy_reads_are_exactly_the_three_queries_under_the_one_profile() {
    let selections = shipped();
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    let record = bundle
        .source
        .amendments
        .as_ref()
        .expect("the bundle records the amendments it applied");
    let bytes = fs::read(root().join(UPSTREAM).join(AMENDMENTS)).unwrap();
    assert_eq!(record.file_name, AMENDMENTS);
    assert_eq!(record.count, PROXY.len());
    assert_eq!(record.sha256, hex::encode(Sha256::digest(&bytes)));
    let amendments: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(amendments["source_sha256"], SOURCE_SHA256);
    let mut amended: Vec<&str> = amendments["amendments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|amendment| {
            assert!(
                amendment["cite"].as_str().unwrap().starts_with("https://"),
                "{amendment}"
            );
            amendment["operation_id"].as_str().unwrap()
        })
        .collect();
    amended.sort_unstable();
    let mut expected: Vec<&str> = PROXY
        .iter()
        .map(|(_, operation_id, _)| *operation_id)
        .collect();
    expected.sort_unstable();
    assert_eq!(amended, expected);

    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let inputs = |id: &str| {
        let declaration = engine
            .declarations(&[Effect::Read])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        let schema = declaration.input_schema;
        let mut names: Vec<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        names.sort();
        let mut required: Vec<String> = schema["required"]
            .as_array()
            .map(|r| r.iter().map(|n| n.as_str().unwrap().to_owned()).collect())
            .unwrap_or_default();
        required.sort();
        (names, required, schema)
    };
    for (id, names, required) in [
        (
            "prometheus.query",
            &["query", "time", "uid"][..],
            &["query", "uid"][..],
        ),
        (
            "prometheus.query_range",
            &["end", "query", "start", "step", "uid"][..],
            &["end", "query", "start", "step", "uid"][..],
        ),
        (
            "loki.query_range",
            &["direction", "end", "limit", "query", "start", "uid"][..],
            &["end", "limit", "query", "start", "uid"][..],
        ),
    ] {
        let (declared, declared_required, _) = inputs(id);
        assert_eq!(declared, names, "`{id}` inputs");
        assert_eq!(declared_required, required, "`{id}` required inputs");
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert!(selection.guard.is_none(), "`{id}` carries no guard");
    }
    let loki = selections
        .iter()
        .find(|s| s.id == "loki.query_range")
        .unwrap();
    let limit = &loki.bounds["limit"];
    assert_eq!(
        (limit.minimum, limit.maximum),
        (Some(1), Some(LOKI_LIMIT_MAXIMUM))
    );
    assert_eq!(loki.bounds["direction"].values, ["forward", "backward"]);
    let (_, _, schema) = inputs("loki.query_range");
    assert_eq!(schema["properties"]["limit"]["maximum"], LOKI_LIMIT_MAXIMUM);
    assert_eq!(
        schema["properties"]["direction"]["enum"],
        json!(["forward", "backward"])
    );
}

/// The committed bundle refuses a selection naming an `operationId` the pinned document lacks.
#[test]
fn an_operation_id_the_pinned_document_lacks_is_refused() {
    let mut selections = shipped();
    let pinned = |operation_id: &str| {
        committed()
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some(operation_id))
    };
    assert!(!pinned("getDashboardsByTitle"));
    selections[0].operation_id = "getDashboardsByTitle".to_owned();
    assert!(Engine::new(&committed(), BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_readme_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    assert_eq!(digest, SOURCE_SHA256);
    assert_eq!(bytes.len(), SOURCE_BYTES);
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(document["servers"][0]["url"], "/api");
    let license = fs::read(upstream.join("LICENSE")).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&license)), LICENSE_SHA256);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("grafana").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, OPERATIONS);
    assert_eq!(entry.unsupported, UNSUPPORTED);
    let bundle = committed();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    for needle in [
        SOURCE_SHA256,
        LICENSE_SHA256,
        URL,
        "v13.3.1",
        "2026-11-11",
        "794,841 bytes",
        "AGPL-3.0",
    ] {
        assert!(readme.contains(needle), "README lacks `{needle}`");
    }
}

/// The committed source differs from Grafana's published document by exactly one member, its
/// example service-account token, checked without network: the file hashes to the committed
/// digest; the README records the upstream and committed digests and sizes and the removed
/// member's JSON pointer; the member is absent; and its parent is exactly `{"type": "string"}`.
/// With the line put back, the file would hash to the upstream digest instead and this fails;
/// with anything else changed, the committed digest fails.
#[test]
fn the_committed_source_differs_from_upstream_by_one_example_member_only() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SOURCE_SHA256);
    assert_eq!(bytes.len(), SOURCE_BYTES);
    assert_ne!(SOURCE_SHA256, UPSTREAM_SHA256);
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    for needle in [
        format!("Upstream SHA-256: `{UPSTREAM_SHA256}`"),
        format!("Upstream size: {UPSTREAM_BYTES}."),
        format!("Committed SHA-256: `{SOURCE_SHA256}`"),
        "Committed size: 794,841 bytes.".to_owned(),
        format!("Removed member: `{REMOVED}`"),
        "secrets scanner".to_owned(),
    ] {
        assert!(readme.contains(&needle), "README lacks `{needle}`");
    }
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(document.pointer(REMOVED).is_none(), "`{REMOVED}` is back");
    assert_eq!(
        document.pointer(REMOVED_PARENT),
        Some(&json!({"type": "string"}))
    );
    // The removal is one whole line: the published size less this file's is that line's length,
    // an `"example": "...",` member at the property's indentation and its newline.
    let removed_line = 794_914 - SOURCE_BYTES;
    let indent = "            ";
    let frame = format!("{indent}\"example\": \"\",\n").len();
    assert!(removed_line > frame, "{removed_line} bytes removed");
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(
        text.contains(&format!(
            "\"key\": {{\n{indent}\"type\": \"string\"\n          }}"
        )),
        "the `key` property is not the expected two lines"
    );
}

fn guide() -> String {
    fs::read_to_string(root().join(GUIDE)).unwrap()
}

/// Acceptance: the guide cites, per operation, the pinned `operationId` and path, and per list
/// operation its paging parameters, its end condition and its time-window filter or why there is
/// none.
#[test]
fn guide_cites_each_operation_its_paging_end_and_time_window() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    let row = |id: &str| {
        *rows
            .iter()
            .find(|row| row.starts_with(&format!("| `{id}` |")))
            .unwrap_or_else(|| panic!("no row for `{id}`"))
    };
    for (id, operation_id, path) in all_reads() {
        let row = row(id);
        assert!(row.contains(&format!("`{operation_id}`")), "{row}");
        assert!(row.contains(&format!("`GET {path}`")), "{row}");
    }
    // story:catalog-grafana-datasource-proxy-reads: per proxy read, its time and step bounds.
    for (id, time, step) in [
        (
            "prometheus.query",
            "`time` optional, Prometheus's default now",
            "no step",
        ),
        (
            "prometheus.query_range",
            "`start`, `end` required, both inclusive",
            "`step` required; at most 11,000 points per series",
        ),
        (
            "loki.query_range",
            "`start` inclusive, `end` exclusive, both required",
            "no step; `limit` 1 to 5000",
        ),
    ] {
        let row = row(id);
        for needle in [time, step] {
            assert!(row.contains(needle), "`{id}` row lacks `{needle}`: {row}");
        }
    }
    for needle in [
        "a data source the token's role cannot query answers `forbidden`",
        "refused as `capacity`, never cut short",
        "`max_entries_limit_per_query`",
        "`allowed_values`",
        "`ValidateUID`",
        "``parameter `uid` does not match its pattern``",
        "written compact with object keys sorted",
    ] {
        assert!(guide.contains(needle), "the guide lacks `{needle}`");
    }
    let short = "fewer items than `limit`";
    for (id, paging, end, window) in [
        (
            "alert_rules.list",
            "none, one answer",
            "the one answer",
            "none: the consumer diffs definitions",
        ),
        (
            "annotations.list",
            "`limit`, narrowing `to`",
            short,
            "`from`, `to` (epoch ms)",
        ),
        (
            "dashboards.search",
            "`page`, `limit`",
            short,
            "none: the consumer diffs titles and descriptions",
        ),
        (
            "datasources.list",
            "none, one answer",
            "the one answer",
            "none",
        ),
    ] {
        let row = row(id);
        for needle in [paging, end, window] {
            assert!(row.contains(needle), "`{id}` row lacks `{needle}`: {row}");
        }
    }
}

/// The configuration example in the guide, so the fixture runs the profile the guide tells a
/// reader to write.
fn documented_config() -> Value {
    let example = guide()
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body.to_owned()))
        .find(|body| body.contains("\"provider\": \"grafana\""))
        .expect("the documented Grafana configuration");
    serde_json::from_str(&example).unwrap()
}

/// A bearer service-account token, proved by `GET /api/user`, the account's own record.
#[test]
fn the_documented_profile_is_a_bearer_service_account_token() {
    let config = documented_config();
    assert_eq!(config["format"], "connectors-catalog-local/2");
    assert_eq!(config["api_base"], "https://grafana.example/api");
    let auth = &config["auth"];
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null, "the token scheme");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(
        auth["identity"],
        json!({"path": "user", "kind": "grafana.user", "subject_pointer": "/id"})
    );
    assert!(
        committed()
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("getSignedInUser") && o.path == "/api/user")
    );
}

// ---- the recorded answers --------------------------------------------------------------------

/// The service account's own record (`GET /api/user`).
const USER: &str = r#"{"email":"","id":7,"isGrafanaAdmin":false,"login":"sa-1-knowledge-ingest","name":"knowledge-ingest","orgId":1}"#;
const ALERT_RULES: &str = r#"[{"condition":"C","folderUID":"fixture-folder","for":"5m","id":11,"labels":{"team":"ingest"},"orgID":1,"ruleGroup":"latency","title":"High request latency","uid":"fixture-rule-1","updated":"2026-11-01T10:00:00Z"},{"condition":"B","folderUID":"fixture-folder","for":"1m","id":12,"labels":{},"orgID":1,"ruleGroup":"errors","title":"Error ratio","uid":"fixture-rule-2","updated":"2026-11-03T08:30:00Z"}]"#;
const DASHBOARD: &str = r#"{"dashboard":{"description":"Request latency by route","panels":[{"id":2,"title":"p99 latency","type":"timeseries"}],"tags":["ingest"],"title":"Ingest latency","uid":"fixture-dash-1","version":3},"meta":{"folderUid":"fixture-folder","slug":"ingest-latency","updated":"2026-11-02T09:00:00Z","url":"/d/fixture-dash-1/ingest-latency"}}"#;
const DATASOURCES: &str = r#"[{"access":"proxy","id":1,"isDefault":true,"name":"Prometheus prod","type":"prometheus","uid":"fixture-prom","url":"http://prometheus.example:9090"},{"access":"proxy","id":2,"isDefault":false,"name":"Loki prod","type":"loki","uid":"fixture-loki","url":"http://loki.example:3100"}]"#;
/// Prometheus's instant and range answers and Loki's streams answer, as the data-source proxy
/// passes them through.
const PROM_VECTOR: &str = r#"{"data":{"result":[{"metric":{"__name__":"up","job":"api"},"value":[1788822000,"1"]}],"resultType":"vector"},"status":"success"}"#;
const PROM_MATRIX: &str = r#"{"data":{"result":[{"metric":{"job":"api"},"values":[[1788822000,"0.5"],[1788822060,"0.75"]]}],"resultType":"matrix"},"status":"success"}"#;
const LOKI_STREAMS: &str = r#"{"data":{"result":[{"stream":{"app":"api","level":"error"},"values":[["1788822001000000000","request failed"],["1788822000000000000","upstream timeout"]]}],"resultType":"streams","stats":{"summary":{"totalEntriesReturned":2}}},"status":"success"}"#;
/// The data sources whose every answer is one byte over the catalog's answer-size limit.
const HUGE_PROM: &str = "fixture-prom-huge";
const HUGE_LOKI: &str = "fixture-loki-huge";
const NOT_FOUND: &str = r#"{"message":"Not found"}"#;
const UNAUTHORIZED: &str = r#"{"message":"invalid API key"}"#;

/// The window the annotation walk reads, in epoch milliseconds.
const FROM: u64 = 1_780_000_000_000;
const TO: u64 = 1_780_000_600_000;

fn hit(n: u64) -> String {
    format!(
        r#"{{"folderUid":"fixture-folder","id":{n},"tags":["ingest"],"title":"Dashboard {n}","type":"dash-db","uid":"fixture-dash-{n}","url":"/d/fixture-dash-{n}"}}"#
    )
}
fn annotation(id: u64, time: u64) -> String {
    format!(
        r#"{{"alertId":11,"dashboardUID":"fixture-dash-1","id":{id},"newState":"alerting","prevState":"normal","tags":[],"text":"High request latency","time":{time},"timeEnd":{time}}}"#
    )
}
fn array(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

/// The recorded body for one request, by route and exact query, or `None`.
fn recorded(route: &str, query: &str) -> Option<String> {
    let at = |offset: u64| TO - 600_000 + offset;
    let body = match (route, query) {
        ("/api/user", "") => USER.to_owned(),
        ("/api/v1/provisioning/alert-rules", "") => ALERT_RULES.to_owned(),
        ("/api/dashboards/uid/fixture-dash-1", "") => DASHBOARD.to_owned(),
        ("/api/datasources", "") => DATASOURCES.to_owned(),
        ("/api/search", "type=dash-db&limit=2&page=1") => array(&[hit(1), hit(2)]),
        ("/api/search", "type=dash-db&limit=2&page=2") => array(&[hit(3)]),
        // Newest first; the second page is asked with `to` narrowed to the oldest `time` of the
        // first, so it repeats that annotation and ends with fewer than `limit`.
        ("/api/annotations", "from=1780000000000&to=1780000600000&limit=3&type=alert") => array(&[
            annotation(105, at(500_000)),
            annotation(104, at(400_000)),
            annotation(103, at(300_000)),
        ]),
        ("/api/annotations", "from=1780000000000&to=1780000300000&limit=3&type=alert") => {
            array(&[annotation(103, at(300_000)), annotation(102, at(200_000))])
        }
        (route, query) if (route, query) == split(PROM_QUERY) => PROM_VECTOR.to_owned(),
        (route, query) if (route, query) == split(PROM_QUERY_RANGE) => PROM_MATRIX.to_owned(),
        (route, query) if (route, query) == split(LOKI_QUERY_RANGE) => LOKI_STREAMS.to_owned(),
        // Every query of the huge data sources: valid JSON one byte over the limit.
        (route, _)
            if route.starts_with(&format!("/api/datasources/proxy/uid/{HUGE_PROM}/"))
                || route.starts_with(&format!("/api/datasources/proxy/uid/{HUGE_LOKI}/")) =>
        {
            over_limit()
        }
        _ => return None,
    };
    Some(body)
}

/// The exact proxied request of each proxy read in [`first_pages`]: the data source `uid` in the
/// path, then the Prometheus or Loki sub-path, the query form-encoded in declared order.
const PROM_QUERY: &str = "/api/datasources/proxy/uid/fixture-prom/api/v1/query?query=sum%28rate%28http_requests_total%7Bjob%3D%22api%22%7D%5B5m%5D%29%29&time=1788822000";
const PROM_QUERY_RANGE: &str = "/api/datasources/proxy/uid/fixture-prom/api/v1/query_range?query=rate%28up%5B5m%5D%29&start=1788822000&end=1788822060&step=60s";
const LOKI_QUERY_RANGE: &str = "/api/datasources/proxy/uid/fixture-loki/loki/api/v1/query_range?query=%7Bapp%3D%22api%22%7D+%7C%3D+%22failed%22&start=1788822000000000000&end=1788822060000000000&limit=100&direction=backward";

fn split(target: &str) -> (&str, &str) {
    target.split_once('?').unwrap_or((target, ""))
}

/// A Prometheus answer of exactly `connectors_core::RESPONSE_LIMIT + 1` bytes.
fn over_limit() -> String {
    let head = r#"{"data":{"result":[],"resultType":"vector"},"pad":""#;
    let tail = r#"","status":"success"}"#;
    let pad = connectors_core::RESPONSE_LIMIT + 1 - head.len() - tail.len();
    format!("{head}{}{tail}", "a".repeat(pad))
}

/// Every recorded body is compact JSON with its keys in order, the form the provider writes a
/// JSON value in, so the bytes the fixture serves can be found unchanged in its output.
#[test]
fn the_recorded_bodies_are_in_the_form_the_provider_writes() {
    let mut bodies = vec![
        USER.to_owned(),
        ALERT_RULES.to_owned(),
        DASHBOARD.to_owned(),
        DATASOURCES.to_owned(),
    ];
    bodies.extend(first_pages().into_iter().map(|(_, _, target)| {
        let (route, query) = target.split_once('?').unwrap_or((target, ""));
        recorded(route, query).unwrap()
    }));
    for body in bodies {
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), body);
    }
}

// ---- the fixture -----------------------------------------------------------------------------

/// Method, request target and `Authorization` header of each request the fixture answered.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

/// `port` and `pem` serve the served instance, which runs on Linux only.
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
                    let (route, query) = target.split_once('?').unwrap_or((&target, ""));
                    // Only the fictional token is accepted; the header is compared, never
                    // printed.
                    let (status, body) =
                        if authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
                            (401, UNAUTHORIZED.to_owned())
                        } else if method != "GET" {
                            (405, NOT_FOUND.to_owned())
                        } else {
                            recorded(route, query)
                                .map_or_else(|| (404, NOT_FOUND.to_owned()), |body| (200, body))
                        };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target.clone(), authorization));
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(body.as_bytes()).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The guide's configuration, pointed at the fixture.
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

    fn requests(&self) -> Vec<(String, String, Option<String>)> {
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
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn bearer() -> Option<String> {
    Some(format!("Bearer {TOKEN}"))
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
fn invoke_raw(child: &mut Child, operation: &str, input: &Value) -> Vec<u8> {
    attempt(child, operation, input)
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"))
}

/// The bytes `served` appear unchanged in the provider's output, as the value of `body`.
fn assert_body_unchanged(operation: &str, raw: &[u8], served: &str) {
    let result: Value = serde_json::from_slice(raw).unwrap();
    assert_eq!(result["status"], 200, "`{operation}` status");
    assert_eq!(
        serde_json::to_string(&result["body"]).unwrap(),
        served,
        "`{operation}` body"
    );
    let field = format!("\"body\":{served}");
    assert!(
        raw.windows(field.len())
            .any(|window| window == field.as_bytes()),
        "`{operation}` output does not carry the served body bytes"
    );
}

// ---- the reads -------------------------------------------------------------------------------

#[test]
fn connecting_proves_the_token_with_the_service_accounts_own_record() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "grafana.user");
    assert_eq!(baseline.identity.subject, "7");
    assert_eq!(baseline.granted_scopes, None);
    assert_eq!(
        provider.requests(),
        [("GET".to_owned(), "/api/user".to_owned(), bearer())]
    );
    let wrong = Secret(br#"{"token":"wrong-token"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
}

/// Each read's input and the exact request target the fixture must observe. Query parameters go
/// out in the order the pinned document declares them.
fn first_pages() -> [(&'static str, Value, &'static str); 8] {
    [
        (
            "alert_rules.list",
            json!({}),
            "/api/v1/provisioning/alert-rules",
        ),
        (
            "annotations.list",
            json!({"type": "alert", "limit": 3, "to": TO, "from": FROM}),
            "/api/annotations?from=1780000000000&to=1780000600000&limit=3&type=alert",
        ),
        (
            "dashboards.search",
            json!({"page": 1, "limit": 2, "type": "dash-db"}),
            "/api/search?type=dash-db&limit=2&page=1",
        ),
        (
            "dashboard.get",
            json!({"uid": "fixture-dash-1"}),
            "/api/dashboards/uid/fixture-dash-1",
        ),
        ("datasources.list", json!({}), "/api/datasources"),
        (
            "prometheus.query",
            json!({"time": "1788822000", "query": "sum(rate(http_requests_total{job=\"api\"}[5m]))",
                   "uid": "fixture-prom"}),
            PROM_QUERY,
        ),
        (
            "prometheus.query_range",
            json!({"step": "60s", "end": "1788822060", "start": "1788822000",
                   "query": "rate(up[5m])", "uid": "fixture-prom"}),
            PROM_QUERY_RANGE,
        ),
        (
            "loki.query_range",
            json!({"direction": "backward", "limit": 100, "end": "1788822060000000000",
                   "start": "1788822000000000000", "query": "{app=\"api\"} |= \"failed\"",
                   "uid": "fixture-loki"}),
            LOKI_QUERY_RANGE,
        ),
    ]
}

/// Acceptance: per operation, the exact request sent and the recorded body returned
/// byte-identical.
#[test]
fn each_read_sends_its_exact_request_with_the_bearer_token_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, target) in first_pages() {
        let before = provider.requests().len();
        let raw = invoke_raw(&mut child, operation, &input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(
            requests[before],
            ("GET".to_owned(), target.to_owned(), bearer()),
            "`{operation}` request"
        );
        let result: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(result["provenance"]["instance"], INSTANCE);
        let (route, query) = target.split_once('?').unwrap_or((target, ""));
        assert_body_unchanged(operation, &raw, &recorded(route, query).unwrap());
    }
}

/// `dashboards.search` lists dashboards only: `type` is required and held to `dash-db`. A
/// parameter the selection does not declare, and an out-of-set `type`, are refused before any
/// request.
#[test]
fn a_search_outside_dashboards_or_an_undeclared_parameter_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("dashboards.search", json!({"limit": 2})),
        ("dashboards.search", json!({"type": "dash-folder"})),
        (
            "dashboards.search",
            json!({"type": "dash-db", "apiVersion": "v1"}),
        ),
        ("dashboard.get", json!({})),
        (
            "dashboard.get",
            json!({"uid": "fixture-dash-1", "apiVersion": "v1beta1"}),
        ),
        ("annotations.list", json!({"type": "incident"})),
        ("datasources.list", json!({"uid": "fixture-prom"})),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// story:catalog-grafana-datasource-proxy-reads, acceptance: an over-limit Prometheus or Loki
/// answer is refused with the catalog's size reason, `capacity`, never cut short and reported
/// complete. Each request is sent once and nothing is answered in its place.
#[test]
fn an_over_limit_prometheus_or_loki_answer_is_refused_as_capacity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert_eq!(over_limit().len(), connectors_core::RESPONSE_LIMIT + 1);
    serde_json::from_str::<Value>(&over_limit()).expect("the over-limit answer is valid JSON");
    let cases = [
        (
            "prometheus.query",
            json!({"uid": HUGE_PROM, "query": "up"}),
            format!("/api/datasources/proxy/uid/{HUGE_PROM}/api/v1/query?query=up"),
        ),
        (
            "prometheus.query_range",
            json!({"uid": HUGE_PROM, "query": "up", "start": "1", "end": "2", "step": "1"}),
            format!(
                "/api/datasources/proxy/uid/{HUGE_PROM}/api/v1/query_range?query=up&start=1&end=2&step=1"
            ),
        ),
        (
            "loki.query_range",
            json!({"uid": HUGE_LOKI, "query": "{app=\"api\"}", "start": "1", "end": "2",
                   "limit": LOKI_LIMIT_MAXIMUM}),
            format!(
                "/api/datasources/proxy/uid/{HUGE_LOKI}/loki/api/v1/query_range?query=%7Bapp%3D%22api%22%7D&start=1&end=2&limit=5000"
            ),
        ),
    ];
    for (operation, input, target) in &cases {
        let before = provider.requests().len();
        let outcome = attempt(&mut child, operation, input);
        assert!(
            matches!(outcome, Err(Failure::Capacity)),
            "`{operation}`: {outcome:?}"
        );
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(
            requests[before],
            ("GET".to_owned(), target.clone(), bearer()),
            "`{operation}` request"
        );
    }
}

/// Grafana's data-source uid rule, `ValidateUID` in `pkg/util/shortid_generator.go` at v13.3.1:
/// letters, digits, `-` and `_`, at most 40.
const UID_PATTERN: &str = "^[A-Za-z0-9_-]{1,40}$";

/// story:catalog-grafana-datasource-proxy-reads, review finding A: each proxy read's `uid` carries
/// Grafana's uid rule from the amendments into the bundle, and nothing else in the bundle carries
/// a pattern. A `uid` outside it, such as one holding `/` or `\` (sent as `%2F` or `%5C`, which a
/// server that decodes before routing reads as another path), is refused by name before any
/// request; 40 characters of the alphabet are sent.
#[tokio::test(flavor = "multi_thread")]
async fn a_proxy_uid_outside_grafanas_uid_rule_is_refused_by_name_before_any_request() {
    let bundle = committed();
    let mut patterned: Vec<(String, String, String)> = bundle
        .inventory
        .operations
        .iter()
        .flat_map(|operation| {
            operation.parameters.iter().filter_map(|parameter| {
                parameter.pattern.clone().map(|pattern| {
                    (
                        operation.operation_id.clone().unwrap_or_default(),
                        parameter.name.clone(),
                        pattern,
                    )
                })
            })
        })
        .collect();
    patterned.sort();
    let expected: Vec<(String, String, String)> = [
        "datasourceProxyLokiQueryRange",
        "datasourceProxyPrometheusQuery",
        "datasourceProxyPrometheusQueryRange",
    ]
    .iter()
    .map(|id| (id.to_string(), "uid".to_owned(), UID_PATTERN.to_owned()))
    .collect();
    assert_eq!(patterned, expected);

    struct Recorded(std::sync::Mutex<Vec<Vec<String>>>);
    #[async_trait::async_trait]
    impl connectors_sdk::AuthenticatedHttp for Recorded {
        async fn get(
            &self,
            path: &[&str],
            _query: &[(&str, String)],
        ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
            self.0
                .lock()
                .unwrap()
                .push(path.iter().map(|s| s.to_string()).collect());
            Ok(connectors_sdk::HttpResponse {
                status: 200,
                headers: Default::default(),
                body: PROM_VECTOR.as_bytes().to_vec(),
            })
        }
    }
    let http = Recorded(std::sync::Mutex::new(Vec::new()));
    let engine = Engine::new(&bundle, BASE, &shipped()).unwrap();
    let input = |operation: &str, uid: &str| match operation {
        "prometheus.query" => json!({"uid": uid, "query": "up"}),
        "prometheus.query_range" => {
            json!({"uid": uid, "query": "up", "start": "1", "end": "2", "step": "1"})
        }
        _ => json!({"uid": uid, "query": "up", "start": "1", "end": "2", "limit": 1}),
    };
    let long = "a".repeat(41);
    for (operation, _, _) in PROXY {
        for uid in [
            "a/b",
            "a\\b",
            "../../api/admin/settings",
            "a%2Fb",
            "a%5Cb",
            "a.b",
            "a b",
            "a?x=1",
            "\u{e9}",
            long.as_str(),
        ] {
            let error = engine
                .read(&http, "one", operation, input(operation, uid))
                .await
                .unwrap_err();
            assert_eq!(
                (error.code, error.message.as_str()),
                (
                    connectors_core::ErrorCode::InvalidInput,
                    "parameter `uid` does not match its pattern `^[A-Za-z0-9_-]{1,40}$`"
                ),
                "`{operation}` {uid:?}"
            );
        }
    }
    assert!(http.0.lock().unwrap().is_empty(), "a refused uid was sent");
    let longest = "A-z_9".repeat(8);
    for (operation, _, _) in PROXY {
        engine
            .read(&http, "one", operation, input(operation, &longest))
            .await
            .unwrap();
    }
    let sent = http.0.lock().unwrap().clone();
    assert_eq!(sent.len(), 3);
    assert!(sent.iter().all(|path| path[3] == longest), "{sent:?}");

    // A bundle whose pattern no value could be checked against is refused when it loads.
    for broken in ["[A-Za-z0-9_-]{1,40}", "^[A-Za-z0-9_-{1,40}$"] {
        let mut bundle = committed();
        for operation in &mut bundle.inventory.operations {
            for parameter in &mut operation.parameters {
                if parameter.pattern.is_some() {
                    parameter.pattern = Some(broken.to_owned());
                }
            }
        }
        let error = Engine::new(&bundle, BASE, &shipped()).err().unwrap();
        assert!(
            error
                .message
                .ends_with("exposes `uid`, whose pattern is not an anchored regular expression"),
            "{broken}: {}",
            error.message
        );
    }
}

/// story:catalog-grafana-datasource-proxy-reads: Loki's `limit` is held to 1 to its documented
/// maximum, `direction` to `forward` or `backward`, and every input the story requires is
/// required; anything else is refused before any request. The maximum itself is sent.
#[test]
fn a_proxy_read_outside_its_inputs_or_loki_limit_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let loki = |change: Value| {
        let mut input = json!({"uid": "fixture-loki", "query": "{app=\"api\"}", "start": "1",
                               "end": "2", "limit": 100});
        for (key, value) in change.as_object().unwrap() {
            if value.is_null() {
                input.as_object_mut().unwrap().remove(key);
            } else {
                input[key] = value.clone();
            }
        }
        input
    };
    for (operation, input) in [
        (
            "loki.query_range",
            loki(json!({"limit": LOKI_LIMIT_MAXIMUM + 1})),
        ),
        ("loki.query_range", loki(json!({"limit": 0}))),
        ("loki.query_range", loki(json!({"limit": null}))),
        ("loki.query_range", loki(json!({"direction": "sideways"}))),
        ("loki.query_range", loki(json!({"start": null}))),
        ("loki.query_range", loki(json!({"uid": null}))),
        ("loki.query_range", loki(json!({"step": "60s"}))),
        ("prometheus.query", json!({"uid": "fixture-prom"})),
        (
            "prometheus.query",
            json!({"uid": "fixture-prom", "query": "up", "timeout": "5s"}),
        ),
        (
            "prometheus.query_range",
            json!({"uid": "fixture-prom", "query": "up", "start": "1", "end": "2"}),
        ),
        (
            "prometheus.query_range",
            json!({"query": "up", "start": "1", "end": "2", "step": "1"}),
        ),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
    let _ = attempt(
        &mut child,
        "loki.query_range",
        &loki(json!({"limit": LOKI_LIMIT_MAXIMUM, "direction": "forward"})),
    );
    assert_eq!(
        provider.requests(),
        [(
            "GET".to_owned(),
            "/api/datasources/proxy/uid/fixture-loki/loki/api/v1/query_range?query=%7Bapp%3D%22api%22%7D&start=1&end=2&limit=5000&direction=forward".to_owned(),
            bearer()
        )]
    );
}

/// Acceptance: `dashboards.search` walks two pages with the caller's `page` and `limit` and stops
/// at the documented end, a page with fewer hits than `limit`.
#[test]
fn dashboards_search_walks_two_pages_and_stops_at_a_short_page() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let limit = 2;
    let mut page = 1;
    let mut uids = Vec::new();
    loop {
        let input = json!({"type": "dash-db", "limit": limit, "page": page});
        let raw = invoke_raw(&mut child, "dashboards.search", &input);
        let hits = serde_json::from_slice::<Value>(&raw).unwrap()["body"]
            .as_array()
            .unwrap()
            .clone();
        uids.extend(
            hits.iter()
                .map(|hit| hit["uid"].as_str().unwrap().to_owned()),
        );
        if hits.len() < limit {
            break;
        }
        page += 1;
        assert!(page <= 3, "the walk does not end");
    }
    assert_eq!(page, 2);
    assert_eq!(uids, ["fixture-dash-1", "fixture-dash-2", "fixture-dash-3"]);
    let targets: Vec<String> = provider.requests().into_iter().map(|r| r.1).collect();
    assert_eq!(
        targets,
        [
            "/api/search?type=dash-db&limit=2&page=1",
            "/api/search?type=dash-db&limit=2&page=2"
        ]
    );
}

/// Acceptance: `annotations.list` walks two pages of a `from`/`to` window, each next page asked
/// with `to` narrowed to the oldest `time` read, and stops at the documented end, a page with
/// fewer items than `limit`. The annotation at the narrowed `to` is read twice and kept once.
#[test]
fn annotations_list_walks_two_pages_by_narrowing_to_and_stops_at_a_short_page() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let limit = 3;
    let mut to = TO;
    let mut pages = 0;
    let mut ids = std::collections::BTreeSet::new();
    loop {
        let input = json!({"from": FROM, "to": to, "limit": limit, "type": "alert"});
        let raw = invoke_raw(&mut child, "annotations.list", &input);
        let items = serde_json::from_slice::<Value>(&raw).unwrap()["body"]
            .as_array()
            .unwrap()
            .clone();
        pages += 1;
        ids.extend(items.iter().map(|item| item["id"].as_u64().unwrap()));
        if items.len() < limit {
            break;
        }
        let oldest = items
            .iter()
            .map(|item| item["time"].as_u64().unwrap())
            .min()
            .unwrap();
        assert!(
            oldest < to,
            "a full page at one instant cannot narrow the window"
        );
        to = oldest;
        assert!(pages < 3, "the walk does not end");
    }
    assert_eq!(pages, 2);
    assert_eq!(ids.into_iter().collect::<Vec<_>>(), [102, 103, 104, 105]);
    let requests = provider.requests();
    let targets: Vec<&str> = requests.iter().map(|r| r.1.as_str()).collect();
    assert_eq!(
        targets,
        [
            "/api/annotations?from=1780000000000&to=1780000600000&limit=3&type=alert",
            "/api/annotations?from=1780000000000&to=1780000300000&limit=3&type=alert"
        ]
    );
    for request in &requests {
        assert_eq!(request.2, bearer());
    }
}

// ---- served from the definition file ---------------------------------------------------------

/// story:catalog-grafana-reads, "Route": a served instance started from the definition file
/// (`catalog.definition` under `catalog.definitions_root`), with the service-account token as the
/// mounted item `<directory>/grafana.<instance>/grafana.service-account`, describes exactly the
/// five reads and, from story:catalog-grafana-datasource-proxy-reads, the three proxy reads beside
/// them, and answers one of each against the recorded fixture, its body unchanged.
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

    const SERVICE_TOKEN: &str = "fixture-service-bearer";
    const SOURCE_DIRECTORY: &str = "/srv/secret";
    const MOUNTED: &str = "/run/connectors/credential";
    const BINARY: &str = "/bin-test/connectors-catalog-provider";
    const CONFIG: &str = "/srv/served.json";
    const SERVED_INSTANCE: &str = "grafana.fixture";
    /// The committed definition, the copy of `connectors/grafana.yaml` in connectors-definitions.
    const DEFINITION: &str =
        "../../crates/connectors-catalog/tests/fixtures/definitions/connectors/grafana.yaml";

    struct Served {
        /// Kept so its directory outlives the served binary.
        _jail: Jail,
        child: Option<std::process::Child>,
        base: String,
        log: Arc<Mutex<String>>,
    }

    impl Served {
        fn start(provider: &Provider) -> Self {
            Self::start_with(provider, json!({}))
        }

        /// Started with `catalog` extended by the members of `extra`.
        fn start_with(provider: &Provider, extra: Value) -> Self {
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
            // The definitions root: the committed definition and its vendored source, gzipped
            // as the definitions repository holds it.
            write(
                "/srv/defs/connectors/grafana.yaml",
                &fs::read(root().join(DEFINITION)).unwrap(),
                0o644,
            );
            let mut gzip =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gzip.write_all(&fs::read(root().join(UPSTREAM).join(SOURCE)).unwrap())
                .unwrap();
            write(
                "/srv/defs/upstream/grafana/openapi3.json.gz",
                &gzip.finish().unwrap(),
                0o644,
            );
            // The amendments that fix each proxy sub-path, as the definition names them.
            write(
                &format!("/srv/defs/upstream/grafana/{AMENDMENTS}"),
                &fs::read(root().join(UPSTREAM).join(AMENDMENTS)).unwrap(),
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
            for (key, value) in extra.as_object().unwrap() {
                config["catalog"][key] = value.clone();
            }
            write(CONFIG, &serde_json::to_vec(&config).unwrap(), 0o600);
            // The Secret volume as the kubelet's atomic writer lays it out.
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
                        return Self {
                            _jail: jail,
                            child: Some(child),
                            base: format!("http://{listen}"),
                            log,
                        };
                    }
                    Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let status = child.wait().unwrap();
                        panic!("exited {status}: {}", log.lock().unwrap());
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

        /// The description's revision and its operation ids, sorted.
        async fn describe(&self) -> (String, Vec<String>) {
            let (status, descriptor) = self.send(reqwest::Method::GET, "/v1/describe", None).await;
            let descriptor: Value = serde_json::from_slice(&descriptor).unwrap();
            assert_eq!(status, 200, "{descriptor}\n{}", self.log());
            let mut listed: Vec<String> = descriptor["operations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|operation| operation["id"].as_str().unwrap().to_owned())
                .collect();
            listed.sort_unstable();
            (descriptor["revision"].as_str().unwrap().to_owned(), listed)
        }

        /// One `v1alpha2` invocation: its status, its answer and the answer's bytes.
        async fn invoke(
            &self,
            revision: &str,
            request_id: &str,
            operation: &str,
            input: Value,
        ) -> (u16, Value, Vec<u8>) {
            let body = json!({
                "version": "v1alpha2",
                "request_id": request_id,
                "operation": operation,
                "revision": revision,
                "input": input,
            });
            let (status, raw) = self
                .send(reqwest::Method::POST, "/v1alpha2/invoke", Some(&body))
                .await;
            (status, serde_json::from_slice(&raw).unwrap(), raw)
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

    #[tokio::test(flavor = "multi_thread")]
    async fn a_served_definition_instance_describes_the_eight_reads_and_answers_two() {
        let provider = Provider::new();
        let served = Served::start(&provider);
        let (status, descriptor) = served
            .send(reqwest::Method::GET, "/v1/describe", None)
            .await;
        let descriptor: Value = serde_json::from_slice(&descriptor).unwrap();
        assert_eq!(status, 200, "{descriptor}\n{}", served.log());
        assert_eq!(descriptor["instance"], SERVED_INSTANCE);
        let mut listed: Vec<&str> = descriptor["operations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| operation["id"].as_str().unwrap())
            .collect();
        listed.sort_unstable();
        // story:catalog-grafana-datasource-proxy-reads: the three proxy reads beside the five.
        let expected: Vec<&str> = all_reads().iter().map(|(id, ..)| *id).collect();
        assert_eq!(listed, expected);
        for operation in descriptor["operations"].as_array().unwrap() {
            assert_ne!(operation["profile"], "mutation", "{operation}");
        }
        let revision = descriptor["revision"].as_str().unwrap();
        let body = json!({
            "version": "v1alpha2",
            "request_id": "read-1",
            "operation": "datasources.list",
            "revision": revision,
            "input": {},
        });
        let (status, raw) = served
            .send(reqwest::Method::POST, "/v1alpha2/invoke", Some(&body))
            .await;
        let answer: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(status, 200, "{answer}\n{}", served.log());
        assert_eq!(
            serde_json::to_string(&answer["result"]["body"]).unwrap(),
            DATASOURCES
        );
        let field = format!("\"body\":{DATASOURCES}");
        assert!(
            raw.windows(field.len())
                .any(|window| window == field.as_bytes()),
            "the served answer does not carry the recorded body bytes: {answer}"
        );
        assert_eq!(
            provider.requests(),
            [("GET".to_owned(), "/api/datasources".to_owned(), bearer())]
        );
        // With no allow-list, a proxy read reaches any data source the token can read.
        let (prometheus, input, target) = first_pages()
            .into_iter()
            .find(|(id, ..)| *id == "prometheus.query")
            .unwrap();
        let (status, answer, raw) = served.invoke(revision, "read-2", prometheus, input).await;
        assert_eq!(status, 200, "{answer}\n{}", served.log());
        let field = format!("\"body\":{PROM_VECTOR}");
        assert!(
            raw.windows(field.len())
                .any(|window| window == field.as_bytes()),
            "the served answer does not carry the recorded body bytes: {answer}"
        );
        assert_eq!(
            provider.requests()[1],
            ("GET".to_owned(), target.to_owned(), bearer())
        );
    }

    /// story:catalog-grafana-datasource-proxy-reads, acceptance: `uid` is a caller input. A `uid`
    /// outside a non-empty allow-list in the instance configuration (`catalog.allowed_values`) is
    /// refused before any upstream request, by a named reason; a listed one is read; an empty list
    /// admits every data source the token can read. Operations the list does not name, such as
    /// `dashboard.get`, whose `uid` is a dashboard's, are not bounded by it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_uid_outside_the_instance_allow_list_is_refused_before_any_request() {
        let provider = Provider::new();
        let served = Served::start_with(
            &provider,
            json!({"allowed_values": {
                "prometheus.query": {"uid": ["fixture-prom"]},
                "prometheus.query_range": {"uid": ["fixture-prom"]},
                "loki.query_range": {"uid": []},
            }}),
        );
        let (revision, listed) = served.describe().await;
        let expected: Vec<&str> = all_reads().iter().map(|(id, ..)| *id).collect();
        assert_eq!(listed, expected, "the allow-list hides no operation");

        for (request, operation, input) in [
            (
                "outside-1",
                "prometheus.query",
                json!({"uid": "fixture-loki", "query": "up"}),
            ),
            (
                "outside-2",
                "prometheus.query_range",
                json!({"uid": "fixture-other", "query": "up", "start": "1", "end": "2", "step": "1"}),
            ),
            (
                "outside-3",
                "prometheus.query",
                json!({"uid": "FIXTURE-PROM", "query": "up"}),
            ),
        ] {
            let (status, answer, _) = served.invoke(&revision, request, operation, input).await;
            assert_eq!(status, 403, "{request}: {answer}\n{}", served.log());
            assert_eq!(answer["error"]["code"], "forbidden", "{request}: {answer}");
            assert_eq!(
                answer["error"]["message"],
                "input `uid` is not one of the instance's allowed values",
                "{request}: {answer}"
            );
        }
        assert!(
            provider.requests().is_empty(),
            "a refused uid reaches nothing: {:?}",
            provider.requests()
        );

        // A listed data source is read.
        let mut sent = Vec::new();
        for (operation, input, target) in first_pages()
            .into_iter()
            .filter(|(id, ..)| id.starts_with("prometheus."))
        {
            let (status, answer, _) = served
                .invoke(&revision, &format!("listed-{operation}"), operation, input)
                .await;
            assert_eq!(status, 200, "{operation}: {answer}\n{}", served.log());
            sent.push(("GET".to_owned(), target.to_owned(), bearer()));
        }
        // An empty list admits every data source; the fixture has none by this `uid`.
        let (status, answer, _) = served
            .invoke(
                &revision,
                "empty-list",
                "loki.query_range",
                json!({"uid": "fixture-any", "query": "{app=\"api\"}", "start": "1", "end": "2",
                       "limit": 10}),
            )
            .await;
        assert_eq!(status, 404, "{answer}\n{}", served.log());
        sent.push((
            "GET".to_owned(),
            "/api/datasources/proxy/uid/fixture-any/loki/api/v1/query_range?query=%7Bapp%3D%22api%22%7D&start=1&end=2&limit=10".to_owned(),
            bearer(),
        ));
        // A dashboard's `uid` is not a data source's.
        let (status, answer, _) = served
            .invoke(
                &revision,
                "dashboard",
                "dashboard.get",
                json!({"uid": "fixture-dash-1"}),
            )
            .await;
        assert_eq!(status, 200, "{answer}\n{}", served.log());
        sent.push((
            "GET".to_owned(),
            "/api/dashboards/uid/fixture-dash-1".to_owned(),
            bearer(),
        ));
        assert_eq!(provider.requests(), sent);
    }
}
