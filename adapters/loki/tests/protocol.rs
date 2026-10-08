//! The Loki binding against a local fake server answering recorded provider
//! fixtures (`tests/fixtures`, shaped to the Loki v3.7.0 HTTP API reference):
//! each operation's request, and the mapping of each answer into its profile's
//! result (`contracts/logs/v1alpha1/semantics.md` §§3–5 and 11).
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_loki::Loki;
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const START: &str = "1788822000000000000";
const END: &str = "1788825600000000000";
/// 1788825600000 ms: the fixed receiver clock, equal to `END`.
const NOW_MS: u64 = 1_788_825_600_000;

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
    json!({"format": "connectors-loki-local/1", "instance": "l", "base_url": base,
           "query_scope": {"required_equalities": []}})
}

fn loki(base: &str) -> Loki {
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
    Loki::new("l", &effective(base), Arc::new(http))
        .unwrap()
        .with_clock(|| NOW_MS)
}

fn streams(groups: Vec<Value>) -> Value {
    json!({"status": "success", "data": {"resultType": "streams", "result": groups}})
}

fn query(seen: &Seen) -> Value {
    Value::Object(
        seen.query
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(v.clone())))
            .collect(),
    )
}

#[tokio::test]
async fn a_range_query_sends_the_exact_window_and_orders_lines_backward() {
    let (base, seen) = server(vec![(200, fixture("query_range_streams.json"))]).await;
    let logql = r#"{app="api"} |= "error""#;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": logql, "start_unix_ns": START, "end_unix_ns": END, "limit": 10}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/loki/api/v1/query_range");
    assert_eq!(
        query(&seen),
        json!({"query": logql, "start": START, "end": END, "direction": "backward", "limit": "10"})
    );
    assert!(
        !seen.head.to_ascii_lowercase().contains("authorization"),
        "the anonymous binding sends no credential"
    );
    let lines: Vec<(&str, &str)> = out["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["timestamp_unix_ns"].as_str().unwrap(),
                l["line"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("1788825599000000000", "boom"),
            ("1788825599000000000", "tie"),
            ("1788825500000000000", "dup"),
            ("1788825000000000000", "early"),
        ],
        "timestamp descending; a tie keeps ascending (group, entry) order"
    );
    assert_eq!(
        out["lines"][1],
        json!({"timestamp_unix_ns": "1788825599000000000",
               "stream": {"app": "api", "level": "info"}, "line": "tie",
               "line_truncated": false, "redacted": false, "source": null})
    );
    assert_eq!(
        out["selection"],
        json!({"kind": "loki-range", "start_unix_ns": START, "end_unix_ns": END, "direction": "backward"})
    );
    assert_eq!(out["order"], "timestamp-backward");
    assert_eq!(out["complete"], true, "four answered below a limit of ten");
    assert_eq!(
        out["truncation"],
        json!({"causes": [], "occurrences_dropped": 0, "stream_groups_dropped": 0})
    );
    assert_eq!(out["next_cursor"], Value::Null);
    assert_eq!(
        out["provenance"],
        json!({"instance": "l", "resource": "loki:query_range",
               "observed_at_unix_ms": NOW_MS, "source_revision": null})
    );
}

#[tokio::test]
async fn a_forward_query_without_an_end_uses_the_receiver_clock() {
    let (base, seen) = server(vec![(200, fixture("query_range_streams_forward.json"))]).await;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{app=\"api\"}", "start_unix_ns": START, "direction": "forward"}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    let sent = query(&seen);
    assert_eq!(sent["end"], END);
    assert_eq!(sent["limit"], "1000");
    assert_eq!(sent["direction"], "forward");
    assert_eq!(out["order"], "timestamp-forward");
    assert_eq!(out["lines"][0]["line"], "early");
    assert_eq!(out["lines"][3]["line"], "tie");
}

#[tokio::test]
async fn an_answer_at_the_limit_is_conservatively_partial_with_unknown_totals() {
    let (base, _) = server(vec![(200, fixture("query_range_streams.json"))]).await;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{app=\"api\"}", "start_unix_ns": START, "end_unix_ns": END, "limit": 4}),
        )
        .await
        .unwrap();
    assert_eq!(out["lines"].as_array().unwrap().len(), 4);
    assert_eq!(out["complete"], false);
    assert_eq!(
        out["truncation"],
        json!({"causes": ["provider_limit"], "occurrences_dropped": null, "stream_groups_dropped": null})
    );
}

#[tokio::test]
async fn groups_past_five_hundred_are_omitted_and_counted() {
    let groups: Vec<Value> = (0..501)
        .map(|i| json!({"stream": {"pod": format!("p{i}")}, "values": [[START, "x"]]}))
        .collect();
    let (base, _) = server(vec![(200, streams(groups))]).await;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{app=\"api\"}", "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    assert_eq!(out["lines"].as_array().unwrap().len(), 500);
    assert_eq!(out["complete"], false);
    assert_eq!(
        out["truncation"],
        json!({"causes": ["stream_limit"], "occurrences_dropped": 1, "stream_groups_dropped": 1})
    );
}

#[tokio::test]
async fn lines_past_the_result_ceiling_are_omitted_and_counted() {
    // Seven 4,000-byte labels repeat on every emitted line: 200 lines are about
    // 5.6 MB serialized from a provider answer of under 40 KB.
    let stream: serde_json::Map<String, Value> = (0..7)
        .map(|i| (format!("l{i}"), Value::from("v".repeat(4000))))
        .collect();
    let values: Vec<Value> = (0..200)
        .map(|i| json!([(1_788_825_000_000_000_000_u64 - i).to_string(), "x"]))
        .collect();
    let (base, _) = server(vec![(
        200,
        streams(vec![json!({"stream": stream, "values": values})]),
    )])
    .await;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{a=\"b\"}", "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    let kept = out["lines"].as_array().unwrap().len();
    assert!((100..200).contains(&kept), "{kept}");
    assert!(serde_json::to_vec(&out).unwrap().len() <= connectors_loki::RESULT_BYTES);
    assert_eq!(out["complete"], false);
    assert_eq!(
        out["truncation"],
        json!({"causes": ["response_bytes"], "occurrences_dropped": 200 - kept,
               "stream_groups_dropped": 0})
    );
}

#[tokio::test]
async fn a_long_line_is_clipped_on_a_character_boundary_and_stays_complete() {
    let long = format!("{}é{}", "a".repeat(8191), "b".repeat(12_000));
    let (base, _) = server(vec![(
        200,
        streams(vec![
            json!({"stream": {"app": "api"}, "values": [[START, long]]}),
        ]),
    )])
    .await;
    let out = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{app=\"api\"}", "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    let line = out["lines"][0]["line"].as_str().unwrap();
    assert_eq!(line.len(), 8191, "the two-byte character is not split");
    assert_eq!(out["lines"][0]["line_truncated"], true);
    assert_eq!(out["complete"], true);
    assert_eq!(out["truncation"]["causes"], json!(["line_bytes"]));
}

#[tokio::test]
async fn a_range_input_the_profile_does_not_admit_is_refused_before_any_request() {
    let (base, seen) = server(vec![]).await;
    let l = loki(&base);
    for input in [
        // window over 24 h
        json!({"query": "{a=\"b\"}", "start_unix_ns": "1788700000000000000", "end_unix_ns": END}),
        // start not before end
        json!({"query": "{a=\"b\"}", "start_unix_ns": END, "end_unix_ns": END}),
        // leading zero, sign, overflow
        json!({"query": "{a=\"b\"}", "start_unix_ns": "01788822000000000000"}),
        json!({"query": "{a=\"b\"}", "start_unix_ns": "-1"}),
        json!({"query": "{a=\"b\"}", "start_unix_ns": "9999999999999999999"}),
        // paging and unknown members are not accepted by the unpaged binding
        json!({"query": "{a=\"b\"}", "start_unix_ns": START, "collection_limit": 10}),
        json!({"query": "{a=\"b\"}", "start_unix_ns": START, "cursor": "x"}),
        json!({"query": "{a=\"b\"}", "start_unix_ns": START, "limit": 1001}),
        json!({"query": "", "start_unix_ns": START}),
    ] {
        let error = l
            .invoke("logs.query_range", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    let error = l.invoke("logs.tail", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(
        seen.lock().unwrap().is_empty(),
        "a refused input reached the provider"
    );
}

#[tokio::test]
async fn an_answer_that_cannot_be_log_lines_never_becomes_one() {
    let mut warned = fixture("query_range_streams.json");
    warned["warnings"] = json!(["query was partial"]);
    let entry = |ts: &str| streams(vec![json!({"stream": {"a": "b"}, "values": [[ts, "x"]]})]);
    let unordered = streams(vec![
        json!({"stream": {"a": "b"}, "values": [["1788822000000000001", "x"], ["1788822000000000002", "y"]]}),
    ]);
    let cases = vec![
        // a metric query: the provider says so, and it is the caller's input
        (fixture("query_range_matrix.json"), ErrorCode::InvalidInput),
        (warned, ErrorCode::Unavailable),
        (entry(END), ErrorCode::Unavailable), // end is exclusive
        (entry("1788821999999999999"), ErrorCode::Unavailable),
        (unordered, ErrorCode::Unavailable), // backward requested
        (
            json!({"status": "success", "data": {"result": []}}),
            ErrorCode::Unavailable,
        ),
        (
            streams(vec![json!({"stream": {"a": 1}, "values": []})]),
            ErrorCode::Unavailable,
        ),
        (
            streams(vec![
                json!({"stream": {"a": "b"}, "values": [[START, "x", {"extra": "1"}]]}),
            ]),
            ErrorCode::Unavailable,
        ),
    ];
    let answers = cases.iter().map(|(a, _)| (200, a.clone())).collect();
    let (base, _) = server(answers).await;
    let l = loki(&base);
    for (answer, want) in &cases {
        let error = l
            .invoke(
                "logs.query_range",
                json!({"query": "{a=\"b\"}", "start_unix_ns": START, "end_unix_ns": END, "limit": 1}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, *want, "{answer}");
    }
}

#[tokio::test]
async fn more_entries_than_the_limit_is_a_protocol_failure() {
    let (base, _) = server(vec![(200, fixture("query_range_streams.json"))]).await;
    let error = loki(&base)
        .invoke(
            "logs.query_range",
            json!({"query": "{a=\"b\"}", "start_unix_ns": START, "end_unix_ns": END, "limit": 3}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

#[tokio::test]
async fn provider_refusals_map_to_family_errors_without_provider_text() {
    let refusal = json!({"message": "raw provider text parse error at line 1"});
    let (base, _) = server(vec![
        (400, refusal.clone()),
        (401, refusal.clone()),
        (403, refusal.clone()),
        (429, refusal.clone()),
        (500, refusal.clone()),
    ])
    .await;
    let l = loki(&base);
    for want in [
        ErrorCode::InvalidInput,
        ErrorCode::Unauthorized,
        ErrorCode::Forbidden,
        ErrorCode::RateLimited,
        ErrorCode::Unavailable,
    ] {
        let error = l
            .invoke(
                "logs.query_range",
                json!({"query": "{a=\"b\"} |= \"secret-query-text\"", "start_unix_ns": START}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, want);
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("raw provider text"), "{rendered}");
        assert!(!rendered.contains("secret-query-text"), "{rendered}");
    }
}

#[tokio::test]
async fn a_metric_range_query_sends_the_step_and_keeps_sample_precision() {
    let (base, seen) = server(vec![(200, fixture("query_range_matrix.json"))]).await;
    let logql = r#"sum by (app) (count_over_time({app="api"}[1m]))"#;
    let out = loki(&base)
        .invoke(
            "logs.query_metric",
            json!({"query": logql, "start_unix_ns": START, "end_unix_ns": END, "step_seconds": 60}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/loki/api/v1/query_range");
    assert_eq!(
        query(&seen),
        json!({"query": logql, "start": START, "end": END, "step": "60s"})
    );
    assert_eq!(
        out,
        json!({"result_type": "matrix",
               "series": [{"labels": {"app": "api"}, "samples": [
                   {"timestamp_unix_ns": "1788822000000000000", "value": "3"},
                   {"timestamp_unix_ns": "1788822060500000000", "value": "NaN"}]}],
               "complete": true, "truncation": [],
               "provenance": {"instance": "l", "profile": "logql-metric", "observed_at_unix_ms": NOW_MS}})
    );
}

#[tokio::test]
async fn an_instant_metric_query_defaults_to_now() {
    let (base, seen) = server(vec![(200, fixture("query_vector.json"))]).await;
    let out = loki(&base)
        .invoke(
            "logs.query_metric",
            json!({"query": "sum(rate({app=\"api\"}[5m]))"}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/loki/api/v1/query");
    assert_eq!(query(&seen)["time"], END);
    assert_eq!(out["result_type"], "vector");
    assert_eq!(
        out["series"],
        json!([{"labels": {"level": "error"},
                "samples": [{"timestamp_unix_ns": "1788825600123000000", "value": "7"}]}])
    );
    assert_eq!(out["complete"], true);
}

#[tokio::test]
async fn a_metric_input_the_profile_does_not_admit_is_refused_before_any_request() {
    let (base, seen) = server(vec![]).await;
    let l = loki(&base);
    for input in [
        // the two forms do not mix
        json!({"query": "q", "time_unix_ns": END, "start_unix_ns": START, "end_unix_ns": END, "step_seconds": 60}),
        // a range needs all three
        json!({"query": "q", "start_unix_ns": START, "end_unix_ns": END}),
        // more than 11,000 points per series
        json!({"query": "q", "start_unix_ns": "1788740000000000000", "end_unix_ns": END, "step_seconds": 1}),
        json!({"query": "q", "start_unix_ns": START, "end_unix_ns": END, "step_seconds": 0}),
    ] {
        let error = l
            .invoke("logs.query_metric", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_log_query_sent_as_a_metric_query_is_the_callers_error() {
    let (base, _) = server(vec![(200, fixture("query_range_streams.json"))]).await;
    let error = loki(&base)
        .invoke("logs.query_metric", json!({"query": "{app=\"api\"}"}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}

#[tokio::test]
async fn series_past_five_hundred_are_omitted_and_reported() {
    let result: Vec<Value> = (0..501)
        .map(|i| json!({"metric": {"pod": format!("p{i}")}, "value": [1788825600, "1"]}))
        .collect();
    let (base, _) = server(vec![(
        200,
        json!({"status": "success", "data": {"resultType": "vector", "result": result}}),
    )])
    .await;
    let out = loki(&base)
        .invoke(
            "logs.query_metric",
            json!({"query": "count_over_time({a=\"b\"}[1m])"}),
        )
        .await
        .unwrap();
    assert_eq!(out["series"].as_array().unwrap().len(), 500);
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"], json!(["series_limit"]));
}

#[tokio::test]
async fn label_names_are_listed_over_the_default_six_hour_window() {
    let (base, seen) = server(vec![(200, fixture("labels.json"))]).await;
    let out = loki(&base).invoke("logs.labels", json!({})).await.unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/loki/api/v1/labels");
    assert_eq!(
        query(&seen),
        json!({"start": "1788804000000000000", "end": END})
    );
    assert_eq!(
        out,
        json!({"label": null, "values": ["app", "level", "namespace"], "complete": true,
               "truncation": [],
               "provenance": {"instance": "l", "profile": "loki-labels", "observed_at_unix_ms": NOW_MS}})
    );
}

#[tokio::test]
async fn label_values_are_listed_for_one_label_under_a_selector() {
    let (base, seen) = server(vec![(200, fixture("label_values.json"))]).await;
    let out = loki(&base)
        .invoke(
            "logs.labels",
            json!({"label": "app", "start_unix_ns": START, "end_unix_ns": END,
                   "query": "{namespace=\"prod\"}"}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert_eq!(seen.path, "/loki/api/v1/label/app/values");
    assert_eq!(
        query(&seen),
        json!({"start": START, "end": END, "query": "{namespace=\"prod\"}"})
    );
    assert_eq!(out["label"], "app");
    assert_eq!(out["values"], json!(["api", "web"]));
    assert_eq!(out["complete"], true);
}

#[tokio::test]
async fn a_label_input_or_answer_outside_the_profile_is_refused() {
    let many: Vec<String> = (0..10_001).map(|i| format!("v{i}")).collect();
    let (base, seen) = server(vec![
        (200, json!({"status": "success", "data": many})),
        (200, json!({"status": "success", "data": [1, 2]})),
    ])
    .await;
    let l = loki(&base);
    for input in [
        json!({"label": "bad-name"}),
        json!({"label": "../x"}),
        json!({"start_unix_ns": "1788700000000000000", "end_unix_ns": END}),
    ] {
        let error = l.invoke("logs.labels", input.clone()).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    assert!(seen.lock().unwrap().is_empty());
    let out = l
        .invoke("logs.labels", json!({"label": "pod"}))
        .await
        .unwrap();
    assert_eq!(out["values"].as_array().unwrap().len(), 10_000);
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"], json!(["value_limit"]));
    let error = l.invoke("logs.labels", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

#[test]
fn a_nonempty_query_scope_is_not_admitted_without_a_parser() {
    let base = "http://127.0.0.1:9/";
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
    let mut scoped = effective(base);
    scoped["query_scope"]["required_equalities"] = json!([{"label": "job", "value": "api"}]);
    assert!(Loki::new("l", &scoped, Arc::new(http)).is_err());
}
