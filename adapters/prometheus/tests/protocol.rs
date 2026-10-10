//! The Prometheus binding against a local fake server answering recorded
//! provider fixtures (`tests/fixtures`, shaped to the Prometheus HTTP API
//! reference): each operation's request, and the mapping of each answer into
//! its profile's result (`contracts/series/v1alpha1/semantics.md` §§3–6 and 11).
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_prometheus::Prometheus;
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const START: u64 = 1_788_822_000;
const END: u64 = 1_788_825_600;
/// The fixed receiver clock in milliseconds, equal to `END` seconds.
const NOW_MS: u64 = 1_788_825_600_000;
const PROXY: &str = "api/datasources/proxy/uid/P1809F7CD0C75ACF3/";

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// One received request: its path and its decoded query pairs.
#[derive(Clone, Debug)]
struct Seen {
    path: String,
    query: Vec<(String, String)>,
    head: String,
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => {
                out.push(u8::from_str_radix(&text[i + 1..i + 3], 16).unwrap());
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

/// Answers each connection with the next scripted (status, body) and records
/// what it received.
async fn server(answers: Vec<(u16, Value)>) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        for (status, answer) in answers {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let target = head.split(' ').nth(1).unwrap().to_owned();
            let (path, query) = target.split_once('?').unwrap_or((&target, ""));
            record.lock().unwrap().push(Seen {
                path: path.to_owned(),
                query: query
                    .split('&')
                    .filter(|p| !p.is_empty())
                    .map(|p| {
                        let (k, v) = p.split_once('=').unwrap_or((p, ""));
                        (decode(k), decode(v))
                    })
                    .collect(),
                head: head.clone(),
            });
            let bytes = serde_json::to_vec(&answer).unwrap();
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
    (format!("http://{address}/"), seen)
}

fn effective(base: &str) -> Value {
    json!({"format": "connectors-prometheus-local/1", "instance": "p", "base_url": base,
           "ca_digest": null, "query_scope": {"allowed_matchers": []}})
}

fn prometheus(base: &str) -> Prometheus {
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
    Prometheus::new("p", &effective(base), Arc::new(http))
        .unwrap()
        .with_clock(|| NOW_MS)
}

fn query(seen: &Seen) -> Value {
    Value::Object(
        seen.query
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(v.clone())))
            .collect(),
    )
}

fn matrix(result: Vec<Value>) -> Value {
    json!({"status": "success", "data": {"resultType": "matrix", "result": result}})
}

fn provenance(resource: &str) -> Value {
    json!({"instance": "p", "resource": resource, "observed_at_unix_ms": NOW_MS,
           "source_revision": null})
}

#[tokio::test]
async fn an_instant_query_sends_the_time_and_keeps_values_and_fractional_timestamps() {
    let (base, seen) = server(vec![(200, fixture("query_vector.json"))]).await;
    let promql = r#"up{job="api"}"#;
    let out = prometheus(&base)
        .invoke("series.query", json!({"query": promql, "time_unix_s": END}))
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/api/v1/query");
    assert_eq!(
        query(&seen),
        json!({"query": promql, "time": END.to_string()})
    );
    assert!(
        !seen.head.to_ascii_lowercase().contains("authorization"),
        "the anonymous test binding sends no credential"
    );
    assert_eq!(
        out,
        json!({
            "result_type": "vector",
            "series": [
                {"labels": {"__name__": "up", "job": "api", "instance": "api-0"},
                 "samples": [[1788825600.25, "1"]], "samples_truncated": false},
                {"labels": {"__name__": "up", "job": "api", "instance": "api-1"},
                 "samples": [[1788825600.25, "NaN"]], "samples_truncated": false},
            ],
            "series_truncated": false,
            "complete": true,
            "warnings": [],
            "infos": [],
            "time_unix_s": END,
            "provenance": provenance("prometheus:query"),
        })
    );
}

#[tokio::test]
async fn an_instant_query_without_a_time_uses_the_receiver_clock() {
    let (base, seen) = server(vec![(200, fixture("query_vector.json"))]).await;
    let out = prometheus(&base)
        .invoke("series.query", json!({"query": "up"}))
        .await
        .unwrap();
    assert_eq!(query(&seen.lock().unwrap()[0])["time"], END.to_string());
    assert_eq!(out["time_unix_s"], END);
}

#[tokio::test]
async fn a_scalar_or_string_answer_is_one_series_with_empty_labels() {
    for (name, kind, value) in [
        ("query_scalar.json", "scalar", "42"),
        ("query_string.json", "string", "fixture"),
    ] {
        let (base, _) = server(vec![(200, fixture(name))]).await;
        let out = prometheus(&base)
            .invoke(
                "series.query",
                json!({"query": "scalar(1)", "time_unix_s": END}),
            )
            .await
            .unwrap();
        assert_eq!(out["result_type"], kind);
        assert_eq!(
            out["series"],
            json!([{"labels": {}, "samples": [[END, value]], "samples_truncated": false}])
        );
        assert_eq!(out["complete"], true);
    }
}

#[tokio::test]
async fn an_instant_range_vector_selector_answers_a_matrix_bounded_per_series() {
    let (base, _) = server(vec![
        (200, fixture("query_instant_matrix.json")),
        (200, fixture("query_instant_matrix.json")),
    ])
    .await;
    let full = prometheus(&base)
        .invoke(
            "series.query",
            json!({"query": "up[5m]", "time_unix_s": END}),
        )
        .await
        .unwrap();
    assert_eq!(full["result_type"], "matrix");
    assert_eq!(
        full["series"][0]["samples"],
        json!([[1788825300, "1"], [1788825450.5, "1"], [END, "0"]])
    );
    // Samples past the bound are dropped from the end and flagged on their series.
    let bounded = prometheus(&base)
        .invoke(
            "series.query",
            json!({"query": "up[5m]", "time_unix_s": END, "max_samples_per_series": 2}),
        )
        .await
        .unwrap();
    assert_eq!(
        bounded["series"][0],
        json!({"labels": {"__name__": "up", "job": "api"},
               "samples": [[1788825300, "1"], [1788825450.5, "1"]], "samples_truncated": true})
    );
    assert_eq!(bounded["series_truncated"], false);
    assert_eq!(bounded["complete"], false);
}

#[tokio::test]
async fn a_range_query_sends_the_exact_window_and_step_and_echoes_it() {
    let (base, seen) = server(vec![(200, fixture("query_range_matrix.json"))]).await;
    let promql = "sum by (status) (rate(http_requests_total[5m]))";
    let out = prometheus(&base)
        .invoke(
            "series.query_range",
            json!({"query": promql, "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/api/v1/query_range");
    assert_eq!(
        query(&seen),
        json!({"query": promql, "start": START.to_string(), "end": END.to_string(), "step": "60"})
    );
    assert_eq!(
        out,
        json!({
            "result_type": "matrix",
            "series": [
                {"labels": {"status": "200"},
                 "samples": [[1788822000, "12.5"], [1788822060, "13"], [1788822120, "+Inf"]],
                 "samples_truncated": false},
                {"labels": {"status": "500"},
                 "samples": [[1788822000, "0.25"], [1788822060, "0"], [1788822120, "NaN"]],
                 "samples_truncated": false},
            ],
            "series_truncated": false,
            "complete": true,
            "warnings": [],
            "infos": [],
            "window": {"start_unix_s": START, "end_unix_s": END, "step_s": 60},
            "provenance": provenance("prometheus:query_range"),
        })
    );
}

#[tokio::test]
async fn a_range_input_outside_the_profile_is_refused_before_any_request() {
    let (base, seen) = server(vec![]).await;
    let adapter = prometheus(&base);
    for input in [
        // start must be before end.
        json!({"query": "up", "start_unix_s": END, "end_unix_s": END, "step_s": 60}),
        json!({"query": "up", "start_unix_s": END, "end_unix_s": START, "step_s": 60}),
        // At most seven days.
        json!({"query": "up", "start_unix_s": END - 604_801, "end_unix_s": END, "step_s": 3600}),
        // One hour at one second is 3,601 points, past the 2,000 default.
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 1}),
        // 61 points past an explicit bound of 60.
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60,
               "max_samples_per_series": 60}),
        // Schema bounds.
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 0}),
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60,
               "max_series": 501}),
        json!({"query": "", "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END}),
        json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60,
               "timeout": "5s"}),
        json!({"query": "up", "start_unix_s": -1, "end_unix_s": END, "step_s": 60}),
    ] {
        let error = adapter
            .invoke("series.query_range", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    assert!(seen.lock().unwrap().is_empty(), "no request was sent");
}

#[tokio::test]
async fn the_point_bound_admits_exactly_its_edge() {
    let (base, seen) = server(vec![(200, fixture("query_range_matrix.json"))]).await;
    // One hour at 60 s is 61 points: admitted at a bound of 61.
    prometheus(&base)
        .invoke(
            "series.query_range",
            json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60,
                   "max_samples_per_series": 61}),
        )
        .await
        .unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn an_instant_input_outside_the_profile_is_refused_before_any_request() {
    let (base, seen) = server(vec![]).await;
    let adapter = prometheus(&base);
    for input in [
        json!({}),
        json!({"query": ""}),
        json!({"query": "x".repeat(16_385)}),
        json!({"query": "up", "time_unix_s": -1}),
        json!({"query": "up", "time_unix_s": 10_000_000_000_u64}),
        json!({"query": "up", "time_unix_s": "1788825600"}),
        json!({"query": "up", "max_series": 0}),
        json!({"query": "up", "max_samples_per_series": 2001}),
        json!({"query": "up", "start_unix_s": START}),
    ] {
        let error = adapter
            .invoke("series.query", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn series_past_the_bound_are_dropped_and_flagged() {
    let many: Vec<Value> = (0..600)
        .map(|i| json!({"metric": {"pod": format!("p{i}")}, "values": [[START, "1"]]}))
        .collect();
    let (base, _) = server(vec![(200, matrix(many.clone())), (200, matrix(many))]).await;
    let input = json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60});
    let out = prometheus(&base)
        .invoke("series.query_range", input.clone())
        .await
        .unwrap();
    let series = out["series"].as_array().unwrap();
    assert_eq!(series.len(), 500);
    assert_eq!(series[0]["labels"], json!({"pod": "p0"}), "provider order");
    assert_eq!(series[499]["labels"], json!({"pod": "p499"}));
    assert_eq!(out["series_truncated"], true);
    assert_eq!(out["complete"], false);
    let mut bounded = input;
    bounded["max_series"] = json!(3);
    let out = prometheus(&base)
        .invoke("series.query_range", bounded)
        .await
        .unwrap();
    assert_eq!(out["series"].as_array().unwrap().len(), 3);
    assert_eq!(out["series_truncated"], true);
}

#[tokio::test]
async fn provider_warnings_make_the_result_partial_and_are_clipped() {
    let (base, _) = server(vec![(200, fixture("query_range_warnings.json"))]).await;
    let out = prometheus(&base)
        .invoke(
            "series.query_range",
            json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
        )
        .await
        .unwrap();
    assert_eq!(out["complete"], false);
    assert_eq!(out["series_truncated"], false);
    let warnings = out["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings[0],
        "fixture partial response: one store unreachable"
    );
    assert_eq!(warnings[1].as_str().unwrap(), "w".repeat(256));
    assert_eq!(
        out["infos"],
        json!(["fixture info: metric might not be a counter"])
    );
}

#[tokio::test]
async fn infos_alone_leave_a_result_complete_and_notes_past_thirty_two_do_not() {
    let infos: Vec<String> = (0..33).map(|i| format!("info {i}")).collect();
    let mut one = matrix(vec![json!({"metric": {}, "values": [[START, "1"]]})]);
    one["infos"] = json!(["fixture info"]);
    let mut many = one.clone();
    many["infos"] = json!(infos);
    let (base, _) = server(vec![(200, one), (200, many)]).await;
    let input = json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60});
    let adapter = prometheus(&base);
    let out = adapter
        .invoke("series.query_range", input.clone())
        .await
        .unwrap();
    assert_eq!(out["complete"], true);
    assert_eq!(out["infos"], json!(["fixture info"]));
    let out = adapter.invoke("series.query_range", input).await.unwrap();
    assert_eq!(out["infos"].as_array().unwrap().len(), 32);
    assert_eq!(out["complete"], false);
}

#[tokio::test]
async fn provider_refusals_map_to_family_errors_without_provider_text() {
    let cases = [
        (400, fixture("error_bad_data.json"), ErrorCode::InvalidInput),
        (
            401,
            json!({"message": "fixture unauthorized"}),
            ErrorCode::Unauthorized,
        ),
        (
            403,
            json!({"message": "fixture forbidden"}),
            ErrorCode::Forbidden,
        ),
        (
            422,
            fixture("error_execution.json"),
            ErrorCode::UpstreamProtocol,
        ),
        (
            429,
            json!({"message": "fixture rate limit"}),
            ErrorCode::RateLimited,
        ),
        (
            503,
            json!({"status": "error", "errorType": "timeout", "error": "fixture timeout"}),
            ErrorCode::Unavailable,
        ),
        (
            404,
            json!({"message": "fixture not found"}),
            ErrorCode::Unavailable,
        ),
    ];
    let (base, _) = server(cases.iter().map(|(s, b, _)| (*s, b.clone())).collect()).await;
    let adapter = prometheus(&base);
    for (status, _, code) in cases {
        let error = adapter
            .invoke("series.query", json!({"query": "up(", "time_unix_s": END}))
            .await
            .unwrap_err();
        assert_eq!(error.code, code, "status {status}");
        assert!(
            !error.message.contains("fixture"),
            "{status}: {}",
            error.message
        );
        assert!(
            !error.message.contains("up("),
            "{status}: {}",
            error.message
        );
        assert!(
            !error.message.contains("127.0.0.1"),
            "{status}: {}",
            error.message
        );
    }
}

#[tokio::test]
async fn an_answer_that_is_not_a_valid_success_envelope_is_unavailable() {
    let ok = fixture("query_range_matrix.json");
    let mut extra = ok.clone();
    extra["stats"] = json!({});
    let mut error_status = ok.clone();
    error_status["status"] = json!("error");
    let mut notes = ok.clone();
    notes["warnings"] = json!("one");
    let vector = fixture("query_vector.json");
    let answers = vec![
        extra,
        error_status,
        notes,
        // A range query answers a matrix, never a vector.
        vector,
        // A value that is not the provider's string.
        matrix(vec![json!({"metric": {}, "values": [[START, 1]]})]),
        // A sample outside the window.
        matrix(vec![json!({"metric": {}, "values": [[END + 60, "1"]]})]),
        // Samples not strictly ascending.
        matrix(vec![
            json!({"metric": {}, "values": [[START + 60, "1"], [START, "1"]]}),
        ]),
        // A label value that is not a string, and an unknown series member.
        matrix(vec![json!({"metric": {"a": 1}, "values": [[START, "1"]]})]),
        matrix(vec![
            json!({"metric": {}, "values": [[START, "1"]], "histograms": []}),
        ]),
        json!({"status": "success", "data": {"resultType": "streams", "result": []}}),
        json!({"status": "success"}),
    ];
    let count = answers.len();
    let (base, _) = server(answers.into_iter().map(|a| (200, a)).collect()).await;
    let adapter = prometheus(&base);
    for index in 0..count {
        let error = adapter
            .invoke(
                "series.query_range",
                json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "answer {index}");
    }
}

#[tokio::test]
async fn rules_are_listed_one_record_per_rule_with_group_and_alert_state() {
    let (base, seen) = server(vec![(200, fixture("rules.json"))]).await;
    let out = prometheus(&base)
        .invoke("rules.list", json!({}))
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/api/v1/rules");
    assert!(seen.query.is_empty(), "{:?}", seen.query);
    assert_eq!(
        out,
        json!({
            "items": [
                {"group": "api.alerts", "group_interval_ms": 30_000, "name": "ApiDown",
                 "kind": "alerting", "query": "up{job=\"api\"} == 0",
                 "labels": {"severity": "page"},
                 "annotations": {"summary": "fixture api instance down"},
                 "health": "ok", "state": "firing", "duration_ms": 300_000,
                 "active_alerts": 1},
                {"group": "api.alerts", "group_interval_ms": 30_000, "name": "ApiSlow",
                 "kind": "alerting",
                 "query": "histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket{job=\"api\"}[5m]))) > 1.5",
                 "labels": {"severity": "ticket"}, "annotations": {},
                 "health": "err", "last_error": "fixture evaluation failure",
                 "state": "inactive", "duration_ms": 600_500, "active_alerts": 0},
                {"group": "api.recording", "group_interval_ms": 60_000,
                 "name": "job:http_requests:rate5m", "kind": "recording",
                 "query": "sum by (job) (rate(http_requests_total[5m]))",
                 "labels": {}, "annotations": {}, "health": "ok"},
            ],
            "complete": true,
            "truncation": [],
            "next_cursor": null,
            "provenance": provenance("prometheus:rules"),
        })
    );
    assert!(
        !serde_json::to_string(&out).unwrap().contains("/etc/"),
        "the rule file path is never returned"
    );
}

#[tokio::test]
async fn a_rule_kind_filter_is_sent_and_held() {
    // The fixture answers both kinds, as a provider that ignores `type` would.
    let (base, seen) = server(vec![
        (200, fixture("rules.json")),
        (200, fixture("rules.json")),
    ])
    .await;
    let adapter = prometheus(&base);
    let alerting = adapter
        .invoke("rules.list", json!({"kind": "alerting"}))
        .await
        .unwrap();
    let recording = adapter
        .invoke("rules.list", json!({"kind": "recording"}))
        .await
        .unwrap();
    let seen = seen.lock().unwrap().clone();
    assert_eq!(query(&seen[0]), json!({"type": "alert"}));
    assert_eq!(query(&seen[1]), json!({"type": "record"}));
    let names = |out: &Value| -> Vec<String> {
        out["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(names(&alerting), ["ApiDown", "ApiSlow"]);
    assert_eq!(names(&recording), ["job:http_requests:rate5m"]);
    assert_eq!(
        adapter
            .invoke("rules.list", json!({"kind": "both"}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[tokio::test]
async fn rules_past_two_thousand_are_dropped_and_flagged() {
    let rules: Vec<Value> = (0..2001)
        .map(|i| {
            json!({"name": format!("r{i}"), "query": "up", "labels": {}, "health": "ok",
                   "type": "recording"})
        })
        .collect();
    let answer = json!({"status": "success", "data": {"groups": [
        {"name": "g", "file": "f", "interval": 60, "rules": rules}]}});
    let (base, _) = server(vec![(200, answer)]).await;
    let out = prometheus(&base)
        .invoke("rules.list", json!({}))
        .await
        .unwrap();
    assert_eq!(out["items"].as_array().unwrap().len(), 2000);
    assert_eq!(out["items"][1999]["name"], "r1999");
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"], json!(["rule_limit"]));
}

#[tokio::test]
async fn a_rule_answer_outside_the_profile_is_unavailable() {
    let rule = json!({"name": "r", "query": "up", "labels": {}, "health": "ok",
                      "type": "recording"});
    let group = |rule: Value| json!({"status": "success", "data": {"groups": [{"name": "g", "rules": [rule]}]}});
    let mut bad_health = rule.clone();
    bad_health["health"] = json!("fine");
    let mut bad_type = rule.clone();
    bad_type["type"] = json!("other");
    let mut no_state = rule.clone();
    no_state["type"] = json!("alerting");
    no_state["alerts"] = json!([]);
    let mut bad_duration = no_state.clone();
    bad_duration["state"] = json!("firing");
    bad_duration["duration"] = json!(-1);
    let answers = vec![
        group(bad_health),
        group(bad_type),
        group(no_state),
        group(bad_duration),
        json!({"status": "success", "data": {"groups": {}}}),
        json!({"status": "success", "data": {}}),
    ];
    let count = answers.len();
    let (base, _) = server(answers.into_iter().map(|a| (200, a)).collect()).await;
    let adapter = prometheus(&base);
    for index in 0..count {
        let error = adapter.invoke("rules.list", json!({})).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "answer {index}");
    }
}

#[test]
fn a_nonempty_query_scope_is_not_admitted_without_a_parser() {
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: "http://127.0.0.1:9/".into(),
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
    let mut configuration = effective("http://127.0.0.1:9/");
    configuration["query_scope"]["allowed_matchers"] = json!(["job=\"api\""]);
    assert!(Prometheus::new("p", &configuration, Arc::new(http)).is_err());
}

/// §6 route transparency: behind Grafana's data-source proxy each request goes
/// below the proxy prefix the connection's `base_url` carries, and the same
/// answers give the same results as direct.
#[tokio::test]
async fn the_grafana_datasource_proxy_route_answers_as_direct() {
    let answers = || {
        vec![
            (200, fixture("query_vector.json")),
            (200, fixture("query_range_matrix.json")),
            (200, fixture("rules.json")),
        ]
    };
    let calls = [
        ("series.query", json!({"query": "up", "time_unix_s": END})),
        (
            "series.query_range",
            json!({"query": "up", "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
        ),
        ("rules.list", json!({})),
    ];
    let (direct_base, direct_seen) = server(answers()).await;
    let (proxy_base, proxy_seen) = server(answers()).await;
    let direct = prometheus(&direct_base);
    let proxied = prometheus(&format!("{proxy_base}{PROXY}"));
    for (operation, input) in &calls {
        assert_eq!(
            direct.invoke(operation, input.clone()).await.unwrap(),
            proxied.invoke(operation, input.clone()).await.unwrap(),
            "{operation}"
        );
    }
    let direct_seen = direct_seen.lock().unwrap().clone();
    let proxy_seen = proxy_seen.lock().unwrap().clone();
    for (direct, proxied) in direct_seen.iter().zip(&proxy_seen) {
        assert_eq!(proxied.path, format!("/{PROXY}{}", &direct.path[1..]));
        assert_eq!(proxied.query, direct.query);
    }
    assert_eq!(proxy_seen.len(), 3);
}
