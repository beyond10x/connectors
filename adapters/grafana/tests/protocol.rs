//! The Grafana binding against a local fake server answering recorded provider fixtures
//! (`tests/fixtures`, hand-written in the shape of Grafana's `GET /api/datasources`): the
//! request `datasources.list` sends, and the projection of each answer onto
//! `connectors_grafana.records.DatasourceRecord`.
use connectors_core::ErrorCode;
use connectors_grafana::Grafana;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The fixed receiver clock in milliseconds.
const NOW_MS: u64 = 1_788_825_600_000;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// One received request: its method, path and raw query.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    query: String,
}

/// Answers each connection with the next scripted (status, body) and records what it
/// received.
async fn server(prefix: &str, answers: Vec<(u16, Vec<u8>)>) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        for (status, bytes) in answers {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let mut line = head.split(' ');
            let method = line.next().unwrap().to_owned();
            let target = line.next().unwrap().to_owned();
            let (path, query) = target.split_once('?').unwrap_or((&target, ""));
            record.lock().unwrap().push(Seen {
                method,
                path: path.to_owned(),
                query: query.to_owned(),
            });
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    (format!("http://{address}{prefix}"), seen)
}

fn effective(base: &str) -> Value {
    json!({"format": "connectors-grafana-local/1", "instance": "g", "base_url": base,
           "ca_digest": null})
}

fn grafana(base: &str) -> Grafana {
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap();
    Grafana::new("g", &effective(base), Arc::new(http))
        .unwrap()
        .with_clock(|| NOW_MS)
}

/// A Grafana data source in the list endpoint's shape, with every private member set.
fn source(uid: &str, name: &str) -> Value {
    json!({"id": 9, "uid": uid, "orgId": 1, "name": name, "type": "loki",
        "typeName": "Loki", "access": "proxy", "url": "http://private-backend.example:3100",
        "user": "fixture-private-user", "database": "", "basicAuth": false,
        "isDefault": false, "jsonData": {}, "readOnly": false})
}

fn list(sources: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&Value::Array(sources)).unwrap()
}

fn output_schema(g: &Grafana) -> Value {
    g.descriptor()
        .operations
        .iter()
        .find(|o| o.id == "datasources.list")
        .unwrap()
        .output_schema
        .clone()
}

#[tokio::test]
async fn the_list_is_one_get_and_returns_only_the_selected_members() {
    let (base, seen) = server("/", vec![(200, fixture("datasources.json"))]).await;
    let g = grafana(&base);
    let out = g.invoke("datasources.list", json!({})).await.unwrap();
    assert_eq!(
        out["items"],
        json!([
            {"uid": "P8E80F9AEF21F6940", "name": "Loki", "type": "loki",
             "access": "proxy", "is_default": false},
            {"uid": "PBFA97CFB590B2093", "name": "Prometheus", "type": "prometheus",
             "access": "proxy", "is_default": true},
            {"uid": "legacy_browser-1", "name": "Browser Elasticsearch",
             "type": "elasticsearch", "access": "direct", "is_default": false},
        ])
    );
    assert_eq!(out["complete"], true);
    assert!(out["next_cursor"].is_null());
    assert_eq!(
        out["provenance"],
        json!({"instance": "g", "resource": "grafana:datasources",
               "observed_at_unix_ms": NOW_MS, "source_revision": null})
    );
    // Backend origins, accounts, databases and settings never reach the result.
    let text = out.to_string();
    for private in [
        "loki-gateway",
        "prometheus-operated",
        "es.backend.example",
        "fixture-backend-user",
        "fixture-index",
        "basicAuth",
        "jsonData",
        "secureJsonFields",
        "orgId",
        "typeLogoUrl",
    ] {
        assert!(!text.contains(private), "{private} leaked: {text}");
    }
    assert!(
        jsonschema::validator_for(&output_schema(&g))
            .unwrap()
            .is_valid(&out)
    );
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, "/api/datasources");
    assert_eq!(seen[0].query, "");
}

#[tokio::test]
async fn a_grafana_served_under_a_path_prefix_is_read_under_that_prefix() {
    let (base, seen) = server("/grafana/", vec![(200, fixture("datasources.json"))]).await;
    let out = grafana(&base)
        .invoke("datasources.list", json!({}))
        .await
        .unwrap();
    assert_eq!(out["items"].as_array().unwrap().len(), 3);
    assert_eq!(seen.lock().unwrap()[0].path, "/grafana/api/datasources");
}

#[tokio::test]
async fn more_than_a_thousand_sources_return_the_first_thousand_as_incomplete() {
    let sources: Vec<Value> = (0..1001)
        .map(|i| source(&format!("uid-{i}"), &format!("s{i}")))
        .collect();
    let (base, _) = server(
        "/",
        vec![
            (200, list(sources.clone())),
            (200, list(sources[..1000].to_vec())),
        ],
    )
    .await;
    let g = grafana(&base);
    let over = g.invoke("datasources.list", json!({})).await.unwrap();
    assert_eq!(over["items"].as_array().unwrap().len(), 1000);
    assert_eq!(over["items"][999]["uid"], "uid-999");
    assert_eq!(over["complete"], false);
    let exact = g.invoke("datasources.list", json!({})).await.unwrap();
    assert_eq!(exact["items"].as_array().unwrap().len(), 1000);
    assert_eq!(exact["complete"], true);
}

#[tokio::test]
async fn names_are_bounded_in_characters_not_bytes() {
    // 190 three-byte characters is the model's edge; 191 is outside it.
    let edge = "\u{2603}".repeat(190);
    let over = "\u{2603}".repeat(191);
    let (base, _) = server(
        "/",
        vec![
            (200, list(vec![source("edge", &edge)])),
            (200, list(vec![source("over", &over)])),
        ],
    )
    .await;
    let g = grafana(&base);
    let out = g.invoke("datasources.list", json!({})).await.unwrap();
    assert_eq!(out["items"][0]["name"], edge);
    let error = g.invoke("datasources.list", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

#[tokio::test]
async fn provider_statuses_map_to_the_family_errors_without_provider_text() {
    let body = br#"{"message":"fixture provider text"}"#.to_vec();
    let cases = [
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (429, ErrorCode::RateLimited),
        (500, ErrorCode::Unavailable),
        (502, ErrorCode::Unavailable),
        (404, ErrorCode::Unavailable),
    ];
    let (base, _) = server(
        "/",
        cases
            .iter()
            .map(|(status, _)| (*status, body.clone()))
            .collect(),
    )
    .await;
    let g = grafana(&base);
    for (status, code) in cases {
        let error = g.invoke("datasources.list", json!({})).await.unwrap_err();
        assert_eq!(error.code, code, "status {status}");
        assert!(!error.message.contains("fixture provider text"));
    }
}

#[tokio::test]
async fn an_answer_outside_the_model_is_refused_whole() {
    let mut no_uid = source("x", "x");
    no_uid.as_object_mut().unwrap().remove("uid");
    let mut no_default = source("x", "x");
    no_default.as_object_mut().unwrap().remove("isDefault");
    let mut unknown_access = source("x", "x");
    unknown_access["access"] = json!("browser");
    let mut empty_type = source("x", "x");
    empty_type["type"] = json!("");
    let answers = vec![
        b"not json".to_vec(),
        br#"{"datasources":[]}"#.to_vec(),
        list(vec![json!("loki")]),
        list(vec![source("ok", "ok"), no_uid]),
        list(vec![source("a/b", "x")]),
        list(vec![source("", "x")]),
        list(vec![source(&"u".repeat(41), "x")]),
        list(vec![source("x", "")]),
        list(vec![no_default]),
        list(vec![unknown_access]),
        list(vec![empty_type]),
    ];
    let count = answers.len();
    let (base, _) = server("/", answers.into_iter().map(|a| (200, a)).collect()).await;
    let g = grafana(&base);
    for case in 0..count {
        let error = g.invoke("datasources.list", json!({})).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "case {case}");
    }
}

#[tokio::test]
async fn an_input_member_is_refused_before_provider_work() {
    let (base, seen) = server("/", vec![]).await;
    let g = grafana(&base);
    for input in [json!({"type": "loki"}), json!({"limit": 10}), json!([])] {
        let error = g
            .invoke("datasources.list", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    let error = g.invoke("datasource.get", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(seen.lock().unwrap().is_empty());
}
