//! The Grafana binding against a local fake server answering recorded provider fixtures
//! (`tests/fixtures`, hand-written in the shape of Grafana's `GET /api/datasources`, of
//! IRM's `GET api/v1/alert_groups/` and of the IRM incident service's `QueryIncidents` and
//! `GetIncident` answers): the requests each operation sends, and the projection of each
//! answer onto `connectors_grafana.records` and `connectors_grafana.irm`.
use connectors_core::ErrorCode;
use connectors_grafana::Grafana;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Fictional fixture material only.
const TOKEN: &str = "fixture-grafana-token";

/// The fixed receiver clock in milliseconds.
const NOW_MS: u64 = 1_788_825_600_000;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// One received request: its method, path, raw query, header lines and body.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    query: String,
    head: String,
    body: Vec<u8>,
}

impl Seen {
    /// The value of one header, by case-insensitive name.
    fn header(&self, name: &str) -> Option<String> {
        self.head
            .lines()
            .filter_map(|l| l.split_once(':'))
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_owned())
    }
    /// The query, percent-decoded, as the provider reads it.
    fn decoded_query(&self) -> String {
        let bytes = self.query.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                out.push(u8::from_str_radix(&self.query[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
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
            let length: usize = head
                .lines()
                .filter_map(|l| l.split_once(':'))
                .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse().unwrap())
                .unwrap_or(0);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            record.lock().unwrap().push(Seen {
                method,
                path: path.to_owned(),
                query: query.to_owned(),
                head: head.clone(),
                body,
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

    // story:grafana-irm-history-reads: every IRM read maps the same statuses, and a
    // single read by id maps 404 to not found, as the provider's own answer.
    let irm_cases = [
        (401, ErrorCode::Unauthorized, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden, ErrorCode::Forbidden),
        (404, ErrorCode::Unavailable, ErrorCode::NotFound),
        (429, ErrorCode::RateLimited, ErrorCode::RateLimited),
        (500, ErrorCode::Unavailable, ErrorCode::Unavailable),
        (503, ErrorCode::Unavailable, ErrorCode::Unavailable),
    ];
    for (operation, input, single) in [
        ("irm.alert_groups.list", window(), false),
        (
            "irm.alert_group.get",
            json!({"alert_group_id": "I68T24C13IFW1"}),
            true,
        ),
        ("irm.incidents.list", window(), false),
        (
            "irm.incident.get",
            json!({"incident_id": "incident-101"}),
            true,
        ),
    ] {
        let answers: Vec<(u16, Vec<u8>)> = irm_cases
            .iter()
            .map(|(status, _, _)| (*status, body.clone()))
            .collect();
        let (stack, _) = server("/", answers.clone()).await;
        let (irm, _) = server("/oncall/", answers).await;
        let g = irm_grafana(&stack, &irm);
        for (status, list_code, single_code) in irm_cases.iter().cloned() {
            let error = g.invoke(operation, input.clone()).await.unwrap_err();
            let code = if single { single_code } else { list_code };
            assert_eq!(error.code, code, "{operation} status {status}");
            assert!(!error.message.contains("fixture provider text"));
            if matches!(status, 403 | 404) && code != ErrorCode::Unavailable {
                assert!(error.upstream_answer, "{operation} status {status}");
            }
        }
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

// ---------------------------------------------------------------------------------------
// story:grafana-irm-history-reads: IRM alert groups, incidents and their links.

struct Token;
#[async_trait::async_trait]
impl Credential for Token {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

fn plaintext(base: &str) -> ScopedHttp {
    ScopedHttp::new_with_ca_bytes(
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
    .unwrap()
}

/// The binding as the composition builds it for one admitted call: the stack port may
/// POST only the two incident reads, and the IRM port carries the stack header.
fn irm_grafana(stack: &str, irm: &str) -> Grafana {
    let stack_http = plaintext(stack)
        .with_read_posts(
            &connectors_grafana::INCIDENT_READ_POSTS,
            connectors_grafana::READ_POST_TIMEOUT,
        )
        .unwrap()
        .with_credential(Arc::new(Token));
    let irm_http = plaintext(irm)
        .with_fixed_header(
            connectors_grafana::STACK_HEADER,
            &connectors_grafana::stack_header_value(stack),
        )
        .unwrap()
        .with_credential(Arc::new(Token));
    let effective = json!({"format": "connectors-grafana-local/1", "instance": "g",
        "base_url": stack, "ca_digest": null, "irm_api_url": irm});
    Grafana::new("g", &effective, Arc::new(stack_http))
        .unwrap()
        .with_clock(|| NOW_MS)
        .with_irm(Arc::new(irm_http))
}

fn window() -> Value {
    json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"})
}

fn schema_of(g: &Grafana, operation: &str) -> Value {
    g.descriptor()
        .operations
        .iter()
        .find(|o| o.id == operation)
        .unwrap()
        .output_schema
        .clone()
}

fn valid(g: &Grafana, operation: &str, out: &Value) {
    let schema = schema_of(g, operation);
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator.iter_errors(out).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "{operation}: {errors:?} in {out}");
}

#[tokio::test]
async fn alert_groups_send_the_window_and_the_stack_header() {
    let (stack, stack_seen) = server("/", vec![]).await;
    let (irm, seen) = server("/oncall/", vec![(200, fixture("alert_groups_page2.json"))]).await;
    let g = irm_grafana(&stack, &irm);
    let out = g.invoke("irm.alert_groups.list", window()).await.unwrap();
    valid(&g, "irm.alert_groups.list", &out);
    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].path, "/oncall/api/v1/alert_groups/");
        assert_eq!(
            seen[0].decoded_query(),
            "started_at=2026-01-01T00:00:00_2026-01-02T00:00:00&perpage=100&page=1"
        );
        assert_eq!(
            seen[0].header("authorization").as_deref(),
            Some(format!("Bearer {TOKEN}").as_str())
        );
        // The configured stack URL, as IRM's documentation writes it (no trailing `/`).
        let expected = stack.trim_end_matches('/');
        assert_eq!(seen[0].header("x-grafana-url").as_deref(), Some(expected));
        assert_eq!(
            seen[0]
                .head
                .to_ascii_lowercase()
                .matches("x-grafana-url")
                .count(),
            1
        );
    }
    // Optional exact filters travel after the window and the page.
    let (irm, seen) = server("/oncall/", vec![(200, fixture("alert_groups_page2.json"))]).await;
    let g = irm_grafana(&stack, &irm);
    let mut input = window();
    input["state"] = json!("acknowledged");
    input["integration_id"] = json!("C0FIXTUREINT2");
    input["team_id"] = json!("TFIXTURE1");
    g.invoke("irm.alert_groups.list", input).await.unwrap();
    assert_eq!(
        seen.lock().unwrap()[0].decoded_query(),
        "started_at=2026-01-01T00:00:00_2026-01-02T00:00:00&perpage=100&page=1\
         &state=acknowledged&integration_id=C0FIXTUREINT2&team_id=TFIXTURE1"
    );
    // The stack itself is never asked for alert groups.
    assert!(stack_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_window_without_until_or_with_an_offset_is_refused_before_provider_work() {
    let (stack, stack_seen) = server("/", vec![]).await;
    let (irm, irm_seen) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    let bad = [
        json!({"since": "2026-01-01T00:00:00Z"}),
        json!({"until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-01T00:00:00+01:00", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00-00:00"}),
        json!({"since": "2026-01-01T00:00:00z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-01T00:00:00.5Z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-01 00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
        // since >= until
        json!({"since": "2026-01-02T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-02T00:00:01Z", "until": "2026-01-02T00:00:00Z"}),
        // Not a calendar instant.
        json!({"since": "2026-02-30T00:00:00Z", "until": "2026-03-02T00:00:00Z"}),
        json!({"since": "2026-01-01T24:00:00Z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-13-01T00:00:00Z", "until": "2026-12-02T00:00:00Z"}),
        json!({"since": "2026-01-01T00:60:00Z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2026-01-01T00:00:60Z", "until": "2026-01-02T00:00:00Z"}),
        json!({"since": "2025-02-29T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
    ];
    for operation in ["irm.alert_groups.list", "irm.incidents.list"] {
        for input in &bad {
            let error = g.invoke(operation, input.clone()).await.unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{operation} {input}");
        }
        // A cursor that is not this operation's, and an unknown member.
        let mut wrong = window();
        wrong["cursor"] = json!(if operation == "irm.alert_groups.list" {
            "0"
        } else {
            "has space"
        });
        assert_eq!(
            g.invoke(operation, wrong).await.unwrap_err().code,
            ErrorCode::InvalidInput
        );
        let mut extra = window();
        extra["queryString"] = json!("title:x");
        assert_eq!(
            g.invoke(operation, extra).await.unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    // A leap day is an instant.
    let (irm, _) = server("/oncall/", vec![(200, fixture("alert_groups_page2.json"))]).await;
    let leap = irm_grafana(&stack, &irm);
    leap.invoke(
        "irm.alert_groups.list",
        json!({"since": "2028-02-29T00:00:00Z", "until": "2028-03-01T00:00:00Z"}),
    )
    .await
    .unwrap();
    assert!(stack_seen.lock().unwrap().is_empty());
    assert!(irm_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn alert_groups_walk_two_pages_and_end_on_null_next() {
    let (stack, _) = server("/", vec![]).await;
    let (irm, seen) = server(
        "/oncall/",
        vec![
            (200, fixture("alert_groups_page1.json")),
            (200, fixture("alert_groups_page2.json")),
        ],
    )
    .await;
    let g = irm_grafana(&stack, &irm);
    let first = g.invoke("irm.alert_groups.list", window()).await.unwrap();
    valid(&g, "irm.alert_groups.list", &first);
    assert_eq!(first["complete"], false);
    assert_eq!(first["next_cursor"], "2");
    assert_eq!(first["window"], window());
    assert_eq!(
        first["provenance"],
        json!({"instance": "g", "resource": "grafana:irm.alert_groups",
               "observed_at_unix_ms": NOW_MS, "source_revision": null})
    );
    let ids: Vec<&str> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["I68T24C13IFW1", "I7XKBQYJYFW3M"]);
    let mut next = window();
    next["cursor"] = first["next_cursor"].clone();
    let second = g.invoke("irm.alert_groups.list", next).await.unwrap();
    valid(&g, "irm.alert_groups.list", &second);
    assert_eq!(second["complete"], true);
    assert!(second["next_cursor"].is_null());
    assert_eq!(second["items"][0]["id"], "I9LPV30QXJ5KM");
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.iter().map(Seen::decoded_query).collect::<Vec<_>>(),
        [
            "started_at=2026-01-01T00:00:00_2026-01-02T00:00:00&perpage=100&page=1",
            "started_at=2026-01-01T00:00:00_2026-01-02T00:00:00&perpage=100&page=2",
        ]
    );
    // IRM's `next` URL is never followed: both requests went to the configured IRM URL.
    assert!(
        seen.iter()
            .all(|s| s.path == "/oncall/api/v1/alert_groups/")
    );
}

#[tokio::test]
async fn alert_group_projection_keeps_the_modelled_members_and_refuses_others_whole() {
    let (stack, _) = server("/", vec![]).await;
    let (irm, _) = server("/oncall/", vec![(200, fixture("alert_groups_page1.json"))]).await;
    let g = irm_grafana(&stack, &irm);
    let out = g.invoke("irm.alert_groups.list", window()).await.unwrap();
    assert_eq!(
        out["items"],
        json!([
            {"id": "I68T24C13IFW1", "integration_id": "CFRPV98RPR1U8",
             "route_id": "RIYGUJXCPFHXY", "alert_group_number": 42, "alerts_count": 3,
             "state": "resolved", "title": "Memory above 90% threshold",
             "created_at": "2026-01-01T12:37:01.430444Z",
             "resolved_at": "2026-01-01T13:37:01.429805Z"},
            {"id": "I7XKBQYJYFW3M", "integration_id": "CFRPV98RPR1U8",
             "alert_group_number": 43, "alerts_count": 1, "state": "new",
             "created_at": "2026-01-01T18:00:00Z", "root_alert_group_id": "I68T24C13IFW1"},
        ])
    );
    let text = out.to_string();
    for private in [
        "permalinks",
        "slack",
        "last_alert",
        "payload",
        "fixture-private-payload",
        "dependent_alert_group_ids",
        "UFIXTUREUSER1",
        "resolved_by",
    ] {
        assert!(!text.contains(private), "{private} leaked: {text}");
    }

    let base: Value = serde_json::from_slice(&fixture("alert_groups_page2.json")).unwrap();
    let with = |change: &dyn Fn(&mut Value)| {
        let mut page = base.clone();
        change(&mut page["results"][0]);
        serde_json::to_vec(&page).unwrap()
    };
    let answers = vec![
        b"not json".to_vec(),
        br#"{"results": []}"#.to_vec(),
        with(&|g| g["state"] = json!("firing")),
        with(&|g| g["id"] = json!("a/b")),
        with(&|g| g["id"] = json!("")),
        with(&|g| {
            g.as_object_mut().unwrap().remove("integration_id");
        }),
        with(&|g| g["alerts_count"] = json!(-1)),
        with(&|g| g["created_at"] = json!("yesterday")),
        with(&|g| g["resolved_at"] = json!(17)),
        with(&|g| g["title"] = json!("t".repeat(4097))),
        with(&|g| g["title"] = json!("")),
        {
            let mut page = base.clone();
            page["next"] = json!(17);
            serde_json::to_vec(&page).unwrap()
        },
        {
            let mut page = base.clone();
            let one = page["results"][0].clone();
            page["results"] = Value::Array(vec![one; 101]);
            serde_json::to_vec(&page).unwrap()
        },
    ];
    let count = answers.len();
    let (irm, _) = server("/oncall/", answers.into_iter().map(|a| (200, a)).collect()).await;
    let g = irm_grafana(&stack, &irm);
    for case in 0..count {
        let error = g
            .invoke("irm.alert_groups.list", window())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "case {case}");
    }
}

#[tokio::test]
async fn incidents_query_is_one_post_to_the_fixed_path_with_date_from_and_cursor() {
    let (stack, seen) = server("/", vec![(200, fixture("incidents_page2.json"))]).await;
    let (irm, irm_seen) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    let out = g.invoke("irm.incidents.list", window()).await.unwrap();
    valid(&g, "irm.incidents.list", &out);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "POST");
    assert_eq!(
        seen[0].path,
        "/api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents"
    );
    assert_eq!(
        seen[0].header("authorization").as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    let body = seen[0].json();
    assert_eq!(body["query"]["dateFrom"], "2026-01-01T00:00:00Z");
    assert_eq!(body["query"]["dateTo"], "2026-01-02T00:00:00Z");
    assert_eq!(
        body["query"]["limit"],
        connectors_grafana::INCIDENT_PAGE as u64
    );
    assert_eq!(body["query"]["orderDirection"], "ASC");
    assert_eq!(body["cursor"]["nextValue"], "");
    // `queryString` would make the service ignore the window.
    assert!(body["query"].get("queryString").is_none());
    assert!(!seen[0].body.windows(11).any(|w| w == b"queryString"));
    // The stack header is the IRM API's; the incident service is the stack itself.
    assert!(seen[0].header("x-grafana-url").is_none());
    assert!(irm_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_read_post_to_any_other_path_is_refused_with_no_io() {
    let (stack, seen) = server("/", vec![]).await;
    let http = plaintext(&stack)
        .with_read_posts(
            &connectors_grafana::INCIDENT_READ_POSTS,
            connectors_grafana::READ_POST_TIMEOUT,
        )
        .unwrap();
    use connectors_sdk::AuthenticatedHttp;
    for path in [
        &[
            "api",
            "plugins",
            "grafana-irm-app",
            "resources",
            "api",
            "v1",
            "IncidentsService.CreateIncident",
        ][..],
        &[
            "api",
            "plugins",
            "grafana-irm-app",
            "resources",
            "api",
            "v1",
            "IncidentsService.UpdateStatus",
        ][..],
        &["api", "ds", "query"][..],
        &["api", "datasources"][..],
    ] {
        let Err(error) = http.post_json(path, &[], &json!({})).await else {
            panic!("{path:?} was sent");
        };
        assert_eq!(error.code, ErrorCode::Forbidden, "{path:?}");
    }
    assert!(seen.lock().unwrap().is_empty());
    // The two fixed reads are exactly the incident query and the incident read.
    assert_eq!(
        connectors_grafana::INCIDENT_READ_POSTS
            .iter()
            .map(|p| p.join("/"))
            .collect::<Vec<_>>(),
        [
            "api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents",
            "api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.GetIncident",
        ]
    );
}

#[tokio::test]
async fn incidents_walk_two_pages_and_end_on_has_more_false() {
    let (stack, seen) = server(
        "/",
        vec![
            (200, fixture("incidents_page1.json")),
            (200, fixture("incidents_page2.json")),
        ],
    )
    .await;
    let (irm, _) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    let first = g.invoke("irm.incidents.list", window()).await.unwrap();
    valid(&g, "irm.incidents.list", &first);
    assert_eq!(first["complete"], false);
    assert_eq!(first["next_cursor"], "fixture-next-value-1");
    assert_eq!(first["window"], window());
    assert_eq!(
        first["provenance"],
        json!({"instance": "g", "resource": "grafana:irm.incidents",
               "observed_at_unix_ms": NOW_MS, "source_revision": null})
    );
    assert_eq!(
        first["items"],
        json!([
            {"incident_id": "incident-101", "title": "Memory pressure on the fixture cluster",
             "status": "resolved", "severity": "minor", "is_drill": false,
             "created_time": "2026-01-01T12:40:00Z", "modified_time": "2026-01-01T13:40:00Z",
             "closed_time": "2026-01-01T13:40:00Z"},
            {"incident_id": "incident-102", "title": "Drill: failover", "status": "active",
             "is_drill": true, "created_time": "2026-01-01T20:00:00Z",
             "modified_time": "2026-01-01T20:01:00Z"},
        ])
    );
    let text = first.to_string();
    for private in [
        "fixture-private",
        "Fixture Person",
        "user-fixture",
        "labels",
        "overviewURL",
        "summary",
        "refs",
        "tracker.example",
    ] {
        assert!(!text.contains(private), "{private} leaked: {text}");
    }
    let mut next = window();
    next["cursor"] = first["next_cursor"].clone();
    let second = g.invoke("irm.incidents.list", next).await.unwrap();
    valid(&g, "irm.incidents.list", &second);
    assert_eq!(second["complete"], true);
    assert!(second["next_cursor"].is_null());
    assert_eq!(second["items"][0]["incident_id"], "incident-103");
    {
        let seen = seen.lock().unwrap();
        assert_eq!(
            seen[1].json()["cursor"]["nextValue"],
            "fixture-next-value-1"
        );
        assert_eq!(seen[1].json()["cursor"]["hasMore"], true);
        assert_eq!(seen[1].json()["query"]["dateFrom"], "2026-01-01T00:00:00Z");
    }

    // A page that claims more with no cursor, a cursor outside the model, a legacy error
    // in a 200, or an answer outside the model is refused whole.
    let base: Value = serde_json::from_slice(&fixture("incidents_page2.json")).unwrap();
    let with = |change: &dyn Fn(&mut Value)| {
        let mut page = base.clone();
        change(&mut page);
        serde_json::to_vec(&page).unwrap()
    };
    let answers = vec![
        with(&|p| p["cursor"] = json!({"hasMore": true, "nextValue": ""})),
        with(&|p| p["cursor"] = json!({"hasMore": true, "nextValue": "has space"})),
        with(&|p| p["cursor"] = json!({"hasMore": true, "nextValue": "x".repeat(1025)})),
        with(&|p| {
            p.as_object_mut().unwrap().remove("cursor");
        }),
        with(&|p| p["error"] = json!("something went wrong")),
        with(&|p| p["incidents"][0]["status"] = json!("investigating")),
        with(&|p| p["incidents"][0]["incidentID"] = json!("a/b")),
        with(&|p| p["incidents"][0]["title"] = json!("")),
        with(&|p| p["incidents"][0]["createdTime"] = json!("")),
        with(&|p| p["incidents"][0]["severity"] = json!("s".repeat(65))),
        with(&|p| p["incidents"][0]["isDrill"] = json!("no")),
        with(&|p| p["incidents"][0]["refs"] = json!("I9LPV30QXJ5KM")),
        with(&|p| {
            let one = p["incidents"][0].clone();
            p["incidents"] = Value::Array(vec![one; 51]);
        }),
    ];
    let count = answers.len();
    let (stack, _) = server("/", answers.into_iter().map(|a| (200, a)).collect()).await;
    let g = irm_grafana(&stack, &irm);
    for case in 0..count {
        let error = g.invoke("irm.incidents.list", window()).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "case {case}");
    }
}

#[tokio::test]
async fn incident_links_are_projected_from_oncall_refs_only() {
    let (stack, _) = server(
        "/",
        vec![
            (200, fixture("incidents_page1.json")),
            (200, fixture("incident.json")),
        ],
    )
    .await;
    let (irm, _) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    let listed = g.invoke("irm.incidents.list", window()).await.unwrap();
    // incident-101 has two OnCall refs and one tracker ref; incident-102 has none.
    assert_eq!(
        listed["links"],
        json!([
            {"incident_id": "incident-101", "alert_group_id": "I68T24C13IFW1"},
            {"incident_id": "incident-101", "alert_group_id": "I7XKBQYJYFW3M"},
        ])
    );
    // Two identical OnCall refs are one link.
    let read = g
        .invoke("irm.incident.get", json!({"incident_id": "incident-101"}))
        .await
        .unwrap();
    valid(&g, "irm.incident.get", &read);
    assert_eq!(
        read["links"],
        json!([{"incident_id": "incident-101", "alert_group_id": "I68T24C13IFW1"}])
    );

    // An OnCall ref that is not an alert-group id refuses the whole answer.
    let base: Value = serde_json::from_slice(&fixture("incident.json")).unwrap();
    let mut bad = base.clone();
    bad["incident"]["refs"][0]["ref"] = json!("abc/123");
    let mut empty = base.clone();
    empty["incident"]["refs"][0]["ref"] = json!("");
    let mut many = base.clone();
    many["incident"]["refs"] = Value::Array(
        (0..65)
            .map(|i| json!({"key": "com.grafana.oncall", "ref": format!("I{i:03}"), "url": ""}))
            .collect(),
    );
    let answers = [bad, empty, many]
        .iter()
        .map(|v| (200, serde_json::to_vec(v).unwrap()))
        .collect();
    let (stack, _) = server("/", answers).await;
    let g = irm_grafana(&stack, &irm);
    for case in 0..3 {
        let error = g
            .invoke("irm.incident.get", json!({"incident_id": "incident-101"}))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "case {case}");
    }
}

#[tokio::test]
async fn single_reads_re_read_one_alert_group_and_one_incident_by_id() {
    let group: Value = serde_json::from_slice(&fixture("alert_groups_page2.json")).unwrap();
    let (stack, stack_seen) = server("/", vec![(200, fixture("incident.json"))]).await;
    let (irm, irm_seen) = server(
        "/oncall/",
        vec![(200, serde_json::to_vec(&group["results"][0]).unwrap())],
    )
    .await;
    let g = irm_grafana(&stack, &irm);
    let one = g
        .invoke(
            "irm.alert_group.get",
            json!({"alert_group_id": "I9LPV30QXJ5KM"}),
        )
        .await
        .unwrap();
    valid(&g, "irm.alert_group.get", &one);
    assert_eq!(one["item"]["state"], "acknowledged");
    assert_eq!(one["item"]["acknowledged_at"], "2026-01-02T00:05:00Z");
    assert_eq!(one["item"]["silenced_at"], "2026-01-02T00:06:00Z");
    assert_eq!(one["provenance"]["resource"], "grafana:irm.alert_group");
    {
        let seen = irm_seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].path, "/oncall/api/v1/alert_groups/I9LPV30QXJ5KM");
        assert_eq!(seen[0].query, "");
        assert!(seen[0].header("x-grafana-url").is_some());
    }
    let incident = g
        .invoke("irm.incident.get", json!({"incident_id": "incident-101"}))
        .await
        .unwrap();
    assert_eq!(incident["item"]["status"], "resolved");
    assert_eq!(incident["item"]["closed_time"], "2026-01-03T09:00:00Z");
    assert_eq!(incident["provenance"]["resource"], "grafana:irm.incident");
    {
        let seen = stack_seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(
            seen[0].path,
            "/api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.GetIncident"
        );
        assert_eq!(seen[0].json(), json!({"incidentID": "incident-101"}));
    }
    // An id outside the model never reaches the provider.
    for (operation, input) in [
        ("irm.alert_group.get", json!({"alert_group_id": "../x"})),
        ("irm.alert_group.get", json!({"alert_group_id": ""})),
        ("irm.incident.get", json!({"incident_id": "a b"})),
        ("irm.incident.get", json!({})),
    ] {
        assert_eq!(
            g.invoke(operation, input).await.unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(irm_seen.lock().unwrap().len(), 1);
    assert_eq!(stack_seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn without_an_irm_api_url_every_irm_read_is_unavailable_with_no_io() {
    let (stack, seen) = server("/", vec![]).await;
    let g = grafana(&stack);
    for (operation, input) in [
        ("irm.alert_groups.list", window()),
        (
            "irm.alert_group.get",
            json!({"alert_group_id": "I68T24C13IFW1"}),
        ),
        ("irm.incidents.list", window()),
        ("irm.incident.get", json!({"incident_id": "incident-101"})),
    ] {
        let error = g.invoke(operation, input).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "{operation}");
        assert!(!error.upstream_answer, "{operation}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

/// The configuration revision the configuration below had before `irm_api_url` existed:
/// the SHA-256 of the canonical effective document the composition built then,
/// `{"base_url":"https://grafana.example/","ca_digest":null,"format":"connectors-grafana-local/1","instance":"grafana-prod"}`.
const PREVIOUS_REVISION: &str = "4752dc45d6252d203e810658beb44589f5c06c86c64e4071edb86540cf4b7cec";

#[cfg(unix)]
#[test]
fn an_existing_configuration_without_irm_api_url_keeps_its_revision() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let print = |document: &Value| -> Option<Value> {
        let path = root.path().join("grafana.json");
        std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_connectors-grafana"))
            .arg("--local-config")
            .arg(&path)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        output
            .status
            .success()
            .then(|| serde_json::from_slice(&output.stdout).unwrap())
    };
    let existing = json!({"format": "connectors-grafana-local/1", "instance": "grafana-prod",
        "base_url": "https://grafana.example/"});
    let before = print(&existing).unwrap();
    assert_eq!(before["configuration_revision"], PREVIOUS_REVISION);
    let mut with_irm = existing.clone();
    with_irm["irm_api_url"] = json!("https://oncall.example/");
    let after = print(&with_irm).unwrap();
    assert_ne!(after["configuration_revision"], PREVIOUS_REVISION);
    // The data-source profile is the same profile, revision included.
    let profile = |b: &Value, id: &str| {
        b["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .cloned()
    };
    assert_eq!(
        profile(&before, "grafana.service_account"),
        profile(&after, "grafana.service_account")
    );
}

/// story:grafana-irm-history-reads, security review: a since-window cursor the caller
/// passes back can neither add a query parameter nor a path segment. An alert-group
/// cursor shaped as a query or path fragment is refused with no I/O; an incident cursor
/// that admits such characters travels only inside the JSON body, never in the target.
#[tokio::test]
async fn a_returned_cursor_cannot_inject_a_query_parameter_or_path_segment() {
    let (stack, stack_seen) = server("/", vec![]).await;
    let (irm, irm_seen) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    for cursor in [
        "2&perpage=1000",
        "2&page=3",
        "2/../x",
        "2/",
        "2?x=1",
        "2#x",
        "%32",
        "+2",
        " 2",
        "2\n",
        "\u{ff12}",
        "1000000",
        "02",
        "",
    ] {
        let mut input = window();
        input["cursor"] = json!(cursor);
        let error = g.invoke("irm.alert_groups.list", input).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{cursor:?}");
    }
    assert!(irm_seen.lock().unwrap().is_empty());
    assert!(stack_seen.lock().unwrap().is_empty());

    let cursor = "x&page=9/../?y=1#z%2F\"}";
    let (stack, seen) = server("/", vec![(200, fixture("incidents_page2.json"))]).await;
    let (irm, irm_seen) = server("/oncall/", vec![]).await;
    let g = irm_grafana(&stack, &irm);
    let mut input = window();
    input["cursor"] = json!(cursor);
    g.invoke("irm.incidents.list", input).await.unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].path,
        "/api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents"
    );
    assert_eq!(seen[0].query, "");
    assert_eq!(seen[0].json()["cursor"]["nextValue"], cursor);
    assert!(irm_seen.lock().unwrap().is_empty());
}
