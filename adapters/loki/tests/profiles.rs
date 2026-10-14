//! Configured line and trace profiles (`contracts/logs/v1alpha1/semantics.md` §12)
//! against a local fake server answering the synthetic fixtures in `tests/fixtures`
//! (`profile_*` and `trace_*`). Every label, field, number and line in them is
//! invented for these tests.
//!
//! A test outside this adapter drives the same reads with [`effective`]'s
//! configuration shape and the fixture files: `logs.profile_lines` with profile
//! `fixture-cdr` over `profile_cdr_streams.json` answers the legs `fx-leg-0001` and
//! `fx-leg-0002` (call identifier field `call_id`); `logs.call_trace` with
//! `fixture-by-label` over `trace_label_streams.json` answers the lines of
//! `fx-leg-0001` from `edge-proxy` and `media`.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_loki::{Loki, profile};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const START: &str = "1788822000000000000";
const END: &str = "1788825600000000000";
const NOW_MS: u64 = 1_788_825_600_000;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
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

type Seen = Arc<Mutex<Vec<(String, Vec<(String, String)>)>>>;

/// Answers each connection with the next scripted answer and records the path
/// and decoded query pairs it received.
async fn server(answers: Vec<Value>) -> (String, Seen) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        for answer in answers {
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
            record.lock().unwrap().push((
                path.to_owned(),
                query
                    .split('&')
                    .filter(|p| !p.is_empty())
                    .map(|p| {
                        let (k, v) = p.split_once('=').unwrap_or((p, ""));
                        (decode(k), decode(v))
                    })
                    .collect(),
            ));
            let bytes = serde_json::to_vec(&answer).unwrap();
            let head = format!(
                "HTTP/1.1 200 X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    (format!("http://{address}/"), seen)
}

fn sent(seen: &Seen, index: usize) -> Value {
    let seen = seen.lock().unwrap();
    let (path, query) = &seen[index];
    let mut object: serde_json::Map<String, Value> = query
        .iter()
        .map(|(k, v)| (k.clone(), Value::from(v.clone())))
        .collect();
    object.insert("path".into(), Value::from(path.clone()));
    Value::Object(object)
}

fn equal(label: &str, value: &str) -> Value {
    json!({"label": label, "operator": "equal", "value": value})
}

/// The synthetic CDR line profile: logfmt, seven typed fields, two phone numbers.
fn cdr_profile() -> Value {
    json!({
        "name": "fixture-cdr",
        "selector": [equal("service", "edge-proxy"), equal("stream_kind", "cdr")],
        "format": {"kind": "logfmt"},
        "fields": [
            {"name": "direction", "key": "dir", "type": "string", "filterable": true},
            {"name": "caller", "key": "src", "type": "phone_number", "filterable": true},
            {"name": "callee", "key": "dst", "type": "phone_number", "filterable": true},
            {"name": "started_at", "key": "start", "type": "unix_seconds"},
            {"name": "ended_at", "key": "end", "type": "unix_seconds", "optional": true},
            {"name": "call_id", "key": "callid", "type": "string"},
            {"name": "duration_ms", "key": "dur", "type": "integer", "optional": true}
        ],
        "call_identifier": "call_id"
    })
}

/// The same CDR written as fixed text, read with a pattern.
fn pattern_profile() -> Value {
    json!({
        "name": "fixture-pattern",
        "selector": [equal("service", "edge-proxy")],
        "format": {"kind": "pattern",
                   "value": {"pattern": "fx-cdr <dir> <src> -> <dst> at <start> id=<callid>"}},
        "fields": [
            {"name": "direction", "key": "dir", "type": "string"},
            {"name": "caller", "key": "src", "type": "phone_number", "mask": "redact"},
            {"name": "callee", "key": "dst", "type": "phone_number", "mask": "none"},
            {"name": "started_at", "key": "start", "type": "unix_millis"},
            {"name": "call_id", "key": "callid", "type": "string"}
        ],
        "call_identifier": "call_id"
    })
}

fn trace_by_label() -> Value {
    json!({
        "name": "fixture-by-label",
        "selector": [equal("env", "fixture")],
        "call_identifier": {"kind": "label", "value": {"label": "call"}},
        "source_label": "service"
    })
}

fn trace_by_field() -> Value {
    json!({
        "name": "fixture-by-field",
        "selector": [equal("env", "fixture")],
        "call_identifier": {"kind": "field",
                            "value": {"key": "callid", "format": {"kind": "logfmt"}}},
        "source_label": "service"
    })
}

/// The effective configuration with the four fixture profiles.
pub fn effective(base: &str) -> Value {
    json!({"format": "connectors-loki-local/1", "instance": "l", "base_url": base,
           "ca_digest": null, "query_scope": {"required_equalities": []},
           "line_profiles": [cdr_profile(), pattern_profile()],
           "trace_profiles": [trace_by_label(), trace_by_field()]})
}

fn http(base: &str) -> Arc<ScopedHttp> {
    Arc::new(
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
        .unwrap(),
    )
}

fn loki_with(base: &str, configuration: &Value) -> connectors_core::Result<Loki> {
    Loki::new("l", configuration, http(base)).map(|l| l.with_clock(|| NOW_MS))
}

fn loki(base: &str) -> Loki {
    loki_with(base, &effective(base)).expect("the fixture profiles are admitted")
}

/// The answer conforms to the output schema `operations describe` publishes.
fn published(loki: &Loki, operation: &str, answer: &Value) {
    let descriptor = loki.descriptor();
    let declared = descriptor
        .operations
        .iter()
        .find(|o| o.id == operation)
        .unwrap_or_else(|| panic!("{operation} is not described"));
    let validator = jsonschema::validator_for(&declared.output_schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(answer)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{operation}: {errors:?} in {answer}");
}

#[tokio::test]
async fn a_line_profile_answers_one_typed_masked_record_per_line_in_time_order() {
    let (base, seen) = server(vec![fixture("profile_cdr_streams.json")]).await;
    let loki = loki(&base);
    let out = loki
        .invoke(
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
                   "limit": 50, "match": {"caller": "+15550100001"}}),
        )
        .await
        .unwrap();
    // The composed query: the profile's selector and the matched value as a line
    // filter, both string literals; the exact window, forward, the limit.
    assert_eq!(
        sent(&seen, 0),
        json!({"path": "/loki/api/v1/query_range",
               "query": r#"{service="edge-proxy",stream_kind="cdr"} |= "+15550100001""#,
               "start": START, "end": END, "direction": "forward", "limit": "50"})
    );
    assert_eq!(
        out["records"],
        json!([
            {"timestamp_unix_ns": "1788822090000000000", "call_id": "fx-leg-0001",
             "fields": {"direction": "inbound", "caller": "********0001",
                        "callee": "********0002", "started_at": "1788822000250000000",
                        "ended_at": "1788822090000000000", "call_id": "fx-leg-0001",
                        "duration_ms": 89750}},
            {"timestamp_unix_ns": "1788822091000000000", "call_id": "fx-leg-0002",
             "fields": {"direction": "outbound", "caller": "********0001",
                        "callee": "********0003", "started_at": "1788822030000000000",
                        "ended_at": null, "call_id": "fx-leg-0002", "duration_ms": null}}
        ]),
        "{out}"
    );
    // The heartbeat does not read as the profile; the third CDR's caller only
    // contains the matched number.
    assert_eq!(out["skipped"], json!({"unreadable": 1, "unmatched": 1}));
    assert_eq!(out["profile"], "fixture-cdr");
    assert_eq!(out["order"], "timestamp-forward");
    assert_eq!(out["complete"], true);
    assert_eq!(
        out["selection"],
        json!({"kind": "loki-profile", "start_unix_ns": START, "end_unix_ns": END,
               "direction": "forward"})
    );
    // No raw line, so no unmasked number, reaches the answer.
    assert!(!out.to_string().contains("+1555"), "{out}");
    published(&loki, "logs.profile_lines", &out);
}

#[tokio::test]
async fn a_pattern_profile_reads_its_captures_and_masks_as_declared() {
    let (base, seen) = server(vec![fixture("profile_pattern_streams.json")]).await;
    let loki = loki(&base);
    let out = loki
        .invoke(
            "logs.profile_lines",
            json!({"profile": "fixture-pattern", "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    assert_eq!(sent(&seen, 0)["query"], r#"{service="edge-proxy"}"#);
    assert_eq!(sent(&seen, 0)["limit"], "1000");
    assert_eq!(
        out["records"],
        json!([{"timestamp_unix_ns": "1788822090000000000", "call_id": "fx-leg-0001",
                "fields": {"direction": "inbound", "caller": "<redacted>",
                           "callee": "+15550100002", "started_at": "1788822000250000000",
                           "call_id": "fx-leg-0001"}}]),
        "{out}"
    );
    assert_eq!(out["skipped"], json!({"unreadable": 1, "unmatched": 0}));
    published(&loki, "logs.profile_lines", &out);
}

#[tokio::test]
async fn a_trace_by_label_answers_one_calls_lines_from_two_sources_in_time_order() {
    let (base, seen) = server(vec![fixture("trace_label_streams.json")]).await;
    let loki = loki(&base);
    let out = loki
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END, "limit": 100}),
        )
        .await
        .unwrap();
    // The source label is required present; the call identifier is one more
    // equality matcher. Nothing else is composed.
    assert_eq!(
        sent(&seen, 0),
        json!({"path": "/loki/api/v1/query_range",
               "query": r#"{env="fixture",service=~".+",call="fx-leg-0001"}"#,
               "start": START, "end": END, "direction": "forward", "limit": "100"})
    );
    let lines: Vec<(&str, &str, &str)> = out["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["timestamp_unix_ns"].as_str().unwrap(),
                l["source"].as_str().unwrap(),
                l["line"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("1788822000100000000", "edge-proxy", "INVITE received"),
            ("1788822000200000000", "media", "stream opened"),
            ("1788822000300000000", "edge-proxy", "200 OK sent"),
            ("1788822089900000000", "media", "stream closed"),
            ("1788822090000000000", "edge-proxy", "BYE received"),
        ]
    );
    assert_eq!(out["call_id"], "fx-leg-0001");
    assert_eq!(out["excluded"], 0);
    assert_eq!(out["complete"], true);
    assert_eq!(out["order"], "timestamp-forward");
    published(&loki, "logs.call_trace", &out);
}

#[tokio::test]
async fn a_trace_by_field_keeps_only_lines_whose_field_is_exactly_the_call() {
    let (base, seen) = server(vec![fixture("trace_field_streams.json")]).await;
    let loki = loki(&base);
    let out = loki
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-field", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    assert_eq!(
        sent(&seen, 0)["query"],
        r#"{env="fixture",service=~".+"} |= "fx-leg-0001""#
    );
    let lines: Vec<(&str, &str)> = out["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["source"].as_str().unwrap(), l["line"].as_str().unwrap()))
        .collect();
    assert_eq!(
        lines,
        [
            ("edge-proxy", r#"callid=fx-leg-0001 msg="INVITE received""#),
            ("app", r#"callid=fx-leg-0001 msg="routing decided""#),
            ("edge-proxy", r#"callid=fx-leg-0001 msg="BYE received""#),
        ]
    );
    // Another call whose identifier contains this one, another call naming it in
    // its message, and a line the format cannot read: none is this call's.
    assert_eq!(out["excluded"], 3);
    published(&loki, "logs.call_trace", &out);
}

#[tokio::test]
async fn a_hostile_call_identifier_is_one_escaped_logql_string_literal() {
    let cases: [(&str, &str); 6] = [
        (r#"a"b"#, r#"a\"b"#),
        (r"a\b", r"a\\b"),
        (r#"x" |= "y"#, r#"x\" |= \"y"#),
        (r#"x"} or {app=~".+"#, r#"x\"} or {app=~\".+"#),
        ("line\nbreak\r\t", r"line\nbreak\r\t"),
        ("nul\u{0}bell\u{7}", r"nul\u0000bell\u0007"),
    ];
    for (id, literal) in cases {
        let (base, seen) = server(vec![
            json!({"status": "success", "data": {"resultType": "streams", "result": []}}),
            json!({"status": "success", "data": {"resultType": "streams", "result": []}}),
        ])
        .await;
        let loki = loki(&base);
        for profile in ["fixture-by-label", "fixture-by-field"] {
            let out = loki
                .invoke(
                    "logs.call_trace",
                    json!({"profile": profile, "call_id": id,
                           "start_unix_ns": START, "end_unix_ns": END}),
                )
                .await
                .unwrap_or_else(|e| panic!("{id:?} {profile}: {e:?}"));
            assert_eq!(out["lines"], json!([]));
            assert_eq!(out["call_id"], id);
        }
        assert_eq!(
            sent(&seen, 0)["query"],
            format!(r#"{{env="fixture",service=~".+",call="{literal}"}}"#),
            "{id:?}"
        );
        // A logfmt line writes such an identifier quoted and escaped, so the line
        // filter looks for that spelling, again as one string literal.
        let quoted = profile::logfmt_spelling(id);
        assert_eq!(
            sent(&seen, 1)["query"],
            format!(
                r#"{{env="fixture",service=~".+"}} |= {}"#,
                profile::string_literal(&quoted)
            ),
            "{id:?}"
        );
    }
    assert_eq!(profile::string_literal(r#"a"b\c"#), r#""a\"b\\c""#);
    assert_eq!(profile::logfmt_spelling("plain-id"), "plain-id");
    assert_eq!(profile::logfmt_spelling(r#"a"b"#), r#""a\"b""#);
    assert_eq!(profile::logfmt_spelling("a b"), r#""a b""#);
}

#[tokio::test]
async fn a_hostile_call_identifier_never_selects_another_calls_lines() {
    // A label-carried answer whose stream names another call is not this call's
    // answer: it refuses rather than mixing calls.
    let mut answer = fixture("trace_label_streams.json");
    answer["data"]["result"][1]["stream"]["call"] = json!("fx-leg-0002");
    let (base, _) = server(vec![answer]).await;
    let error = loki(&base)
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    // A stream without the source label cannot name its source.
    let mut answer = fixture("trace_label_streams.json");
    answer["data"]["result"][0]["stream"]
        .as_object_mut()
        .unwrap()
        .remove("service");
    let (base, _) = server(vec![answer]).await;
    let error = loki(&base)
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

#[tokio::test]
async fn the_window_limit_and_declared_inputs_bound_every_profile_read() {
    let (base, seen) = server(vec![]).await;
    let loki = loki(&base);
    let day_and_one = (1_788_822_000_000_000_000_u64 + 86_400_000_000_001).to_string();
    let refusals: Vec<(&str, Value, ErrorCode)> = vec![
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": day_and_one}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": END,
            "end_unix_ns": START}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "limit": 0}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "limit": 1001}),
            ErrorCode::InvalidInput,
        ),
        // The window is the caller's to state: no implied end.
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START}),
            ErrorCode::InvalidInput,
        ),
        // Only fields the profile declares filterable are matched.
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "match": {"call_id": "fx-leg-0001"}}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "match": {"undeclared": "x"}}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "match": {"caller": ""}}),
            ErrorCode::InvalidInput,
        ),
        // No caller LogQL, selector or label reaches a profile read.
        (
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "query": "{app=~\".+\"}"}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx",
            "start_unix_ns": START, "end_unix_ns": END, "selector": []}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "",
            "start_unix_ns": START, "end_unix_ns": END}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "x".repeat(513),
            "start_unix_ns": START, "end_unix_ns": END}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx",
            "start_unix_ns": START, "end_unix_ns": day_and_one}),
            ErrorCode::InvalidInput,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx",
            "start_unix_ns": START, "end_unix_ns": END, "limit": 1001}),
            ErrorCode::InvalidInput,
        ),
        // A trace profile is not a line profile, and an unconfigured name is not found.
        (
            "logs.profile_lines",
            json!({"profile": "fixture-by-label", "start_unix_ns": START,
            "end_unix_ns": END}),
            ErrorCode::NotFound,
        ),
        (
            "logs.call_trace",
            json!({"profile": "fixture-cdr", "call_id": "fx",
            "start_unix_ns": START, "end_unix_ns": END}),
            ErrorCode::NotFound,
        ),
        (
            "logs.call_trace",
            json!({"profile": "absent", "call_id": "fx",
            "start_unix_ns": START, "end_unix_ns": END}),
            ErrorCode::NotFound,
        ),
    ];
    for (operation, input, code) in refusals {
        let error = loki.invoke(operation, input.clone()).await.unwrap_err();
        assert_eq!(error.code, code, "{operation} {input}");
    }
    assert!(seen.lock().unwrap().is_empty(), "a refusal dispatched");
}

#[tokio::test]
async fn a_saturated_answer_is_partial_and_never_complete() {
    let (base, _) = server(vec![fixture("trace_label_streams.json")]).await;
    let out = loki(&base)
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END, "limit": 5}),
        )
        .await
        .unwrap();
    assert_eq!(out["lines"].as_array().unwrap().len(), 5);
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"]["causes"], json!(["provider_limit"]));
    // More entries than the limit asked for is a malformed answer, not more lines.
    let (base, _) = server(vec![fixture("trace_label_streams.json")]).await;
    let error = loki(&base)
        .invoke(
            "logs.call_trace",
            json!({"profile": "fixture-by-label", "call_id": "fx-leg-0001",
                   "start_unix_ns": START, "end_unix_ns": END, "limit": 4}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

#[test]
fn a_profile_cannot_widen_the_configured_stream_scope() {
    let scope = [json!({"label": "team", "value": "voice"})];
    let mut within = cdr_profile();
    within["selector"]
        .as_array_mut()
        .unwrap()
        .push(equal("team", "voice"));
    let composed = profile::admit(&[within.clone()], &[], &scope).unwrap();
    assert_eq!(
        composed.line[0].selector(),
        r#"{service="edge-proxy",stream_kind="cdr",team="voice"}"#
    );
    // Missing the scope label, naming another value, or reaching it only through a
    // regex or a negation would read streams the scope does not admit.
    for matcher in [
        None,
        Some(equal("team", "other")),
        Some(json!({"label": "team", "operator": "matches", "value": "voice|.*"})),
        Some(json!({"label": "team", "operator": "not_equal", "value": "billing"})),
        Some(equal("Team", "voice")),
    ] {
        let mut widened = cdr_profile();
        if let Some(matcher) = matcher.clone() {
            widened["selector"].as_array_mut().unwrap().push(matcher);
        }
        assert!(
            profile::admit(&[widened], &[], &scope).is_err(),
            "admitted {matcher:?}"
        );
    }
    let mut trace = trace_by_label();
    assert!(profile::admit(&[], &[trace.clone()], &scope).is_err());
    trace["selector"]
        .as_array_mut()
        .unwrap()
        .push(equal("team", "voice"));
    let composed = profile::admit(&[], &[trace], &scope).unwrap();
    assert_eq!(
        composed.trace[0].selector_for("fx"),
        r#"{env="fixture",team="voice",service=~".+",call="fx"}"#
    );
    // The entire tenant (the empty scope) admits every well-formed profile.
    assert!(profile::admit(&[cdr_profile()], &[trace_by_field()], &[]).is_ok());
}

#[test]
fn a_profile_the_adapter_cannot_honour_is_an_invalid_configuration() {
    let base = "http://127.0.0.1:9/";
    let mutated = |path: &str, change: &dyn Fn(&mut Value)| {
        let mut configuration = effective(base);
        change(configuration.pointer_mut(path).unwrap());
        configuration
    };
    let refused: Vec<(&str, Value)> = vec![
        (
            "the call identifier names no field",
            mutated("/line_profiles/0", &|p| {
                p["call_identifier"] = json!("absent");
            }),
        ),
        (
            "the call identifier is masked",
            mutated("/line_profiles/0/fields/5", &|f| {
                f["mask"] = json!("last_four");
            }),
        ),
        (
            "the call identifier is not text",
            mutated("/line_profiles/0/fields/5", &|f| {
                f["type"] = json!("integer");
            }),
        ),
        (
            "the call identifier is optional",
            mutated("/line_profiles/0/fields/5", &|f| {
                f["optional"] = json!(true);
            }),
        ),
        (
            "a masked integer",
            mutated("/line_profiles/0/fields/6", &|f| {
                f["mask"] = json!("redact");
            }),
        ),
        (
            "two fields of one name",
            mutated("/line_profiles/0/fields/1", &|f| {
                f["name"] = json!("direction");
            }),
        ),
        (
            "two profiles of one name",
            mutated("/line_profiles/1", &|p| {
                p["name"] = json!("fixture-cdr");
            }),
        ),
        (
            "a pattern key that is not a capture",
            mutated("/line_profiles/1/fields/0", &|f| {
                f["key"] = json!("absent");
            }),
        ),
        (
            "a pattern without a capture",
            mutated("/line_profiles/1/format/value", &|v| {
                v["pattern"] = json!("fx-cdr <_> fixed");
            }),
        ),
        (
            "adjacent captures",
            mutated("/line_profiles/1/format/value", &|v| {
                v["pattern"] = json!("fx-cdr <dir><src> -> <dst> at <start> id=<callid>");
            }),
        ),
        (
            "a selector of regexes only",
            mutated("/line_profiles/0/selector", &|s| {
                *s = json!([{"label": "service", "operator": "matches", "value": ".+"}]);
            }),
        ),
        (
            "an empty equality only",
            mutated("/trace_profiles/0/selector", &|s| {
                *s = json!([equal("env", "")]);
            }),
        ),
        (
            "a label name starting with a digit",
            mutated("/trace_profiles/0", &|p| {
                p["source_label"] = json!("1service");
            }),
        ),
        (
            "the call label already selected",
            mutated("/trace_profiles/0/selector", &|s| {
                s.as_array_mut().unwrap().push(equal("call", "fixed"));
            }),
        ),
        (
            "the call label as the source label",
            mutated("/trace_profiles/0", &|p| {
                p["source_label"] = json!("call");
            }),
        ),
        (
            "an unknown member",
            mutated("/trace_profiles/1", &|p| {
                p["query"] = json!("{env=\"fixture\"}");
            }),
        ),
    ];
    for (why, configuration) in refused {
        assert!(loki_with(base, &configuration).is_err(), "admitted: {why}");
    }
    // Without profiles the configuration is the one every existing connection has.
    let mut plain = effective(base);
    plain.as_object_mut().unwrap().remove("line_profiles");
    plain.as_object_mut().unwrap().remove("trace_profiles");
    assert!(loki_with(base, &plain).is_ok());
}
