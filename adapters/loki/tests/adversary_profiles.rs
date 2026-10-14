//! Adversarial cases for the configured line and trace profiles
//! (`contracts/logs/v1alpha1/semantics.md` §12), driven against a small Loki
//! emulator that answers the query the adapter actually composed, the way Loki
//! answers it: stream matchers select streams, `|= "text"` keeps lines containing
//! the text, `| logfmt` followed by `| key="value"` keeps lines whose parsed key is
//! exactly the value, and a forward read keeps the oldest `limit` entries. Any other
//! syntax panics, so a composed query the emulator cannot judge never passes.
//!
//! Every label, field, number and line here is invented for these tests.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_loki::Loki;
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const START: &str = "1788822000000000000";
const END: &str = "1788825600000000000";
const NOW_MS: u64 = 1_788_825_600_000;

type Stream = (Value, Vec<(u64, &'static str)>);

fn url_decode(text: &str) -> String {
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

/// A LogQL double-quoted literal starting at `text[0] == '"'`: its value and the
/// rest of the text after it.
fn literal(text: &str) -> (String, &str) {
    assert!(
        text.starts_with('"'),
        "emulator: expected a literal in {text:?}"
    );
    let mut out = String::new();
    let mut chars = text[1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, &text[i + 2..]),
            '\\' => match chars.next().unwrap().1 {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = (0..4).map(|_| chars.next().unwrap().1).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
                }
                other => panic!("emulator: unknown escape \\{other}"),
            },
            c => out.push(c),
        }
    }
    panic!("emulator: unterminated literal in {text:?}")
}

/// A logfmt line as Loki's parser reads it, for `| logfmt`: `None` when it does not.
fn logfmt(line: &str) -> Option<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut rest = line.trim_start();
    while !rest.is_empty() {
        let end = rest.find(['=', ' ']).unwrap_or(rest.len());
        let key = &rest[..end];
        rest = &rest[end..];
        let value = if let Some(after) = rest.strip_prefix('=') {
            if after.starts_with('"') {
                let (value, tail) = literal(after);
                rest = tail;
                value
            } else {
                let end = after.find(' ').unwrap_or(after.len());
                rest = &after[end..];
                after[..end].to_owned()
            }
        } else {
            String::new()
        };
        out.insert(key.to_owned(), value);
        rest = rest.trim_start();
    }
    Some(out)
}

enum Stage {
    Contains(String),
    Logfmt,
    Equal(String, String),
}

/// Which lines of `streams` Loki answers for `query`, forward, at most `limit`.
fn evaluate(streams: &[Stream], query: &str, start: u64, end: u64, limit: usize) -> Value {
    assert!(query.starts_with('{'), "emulator: {query}");
    // The selector: matchers up to the closing brace outside a literal.
    let mut matchers = Vec::new();
    let mut rest = &query[1..];
    loop {
        let label_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap();
        let label = rest[..label_end].to_owned();
        rest = &rest[label_end..];
        let operator = ["=~", "!~", "!=", "="]
            .into_iter()
            .find(|o| rest.starts_with(o))
            .unwrap_or_else(|| panic!("emulator: matcher operator in {rest:?}"));
        rest = &rest[operator.len()..];
        let (value, tail) = literal(rest);
        matchers.push((label, operator, value));
        rest = tail;
        if let Some(tail) = rest.strip_prefix(',') {
            rest = tail;
            continue;
        }
        rest = rest.strip_prefix('}').expect("emulator: selector end");
        break;
    }
    let mut stages = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some(tail) = rest.strip_prefix("|= ") {
            let (value, tail) = literal(tail);
            stages.push(Stage::Contains(value));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("| logfmt") {
            stages.push(Stage::Logfmt);
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("| ") {
            let end = tail.find("=\"").expect("emulator: label filter");
            let (value, tail2) = literal(&tail[end + 1..]);
            stages.push(Stage::Equal(tail[..end].to_owned(), value));
            rest = tail2;
        } else {
            panic!("emulator: unsupported stage {rest:?}");
        }
    }
    let mut kept: Vec<(u64, usize, usize, &str)> = Vec::new();
    for (group, (labels, lines)) in streams.iter().enumerate() {
        let selected = matchers.iter().all(|(label, operator, value)| {
            let actual = labels.get(label).and_then(Value::as_str).unwrap_or("");
            match *operator {
                "=" => actual == value,
                "!=" => actual != value,
                "=~" if value == ".+" => !actual.is_empty(),
                other => panic!("emulator: unsupported matcher {other} {value:?}"),
            }
        });
        if !selected {
            continue;
        }
        for (entry, (timestamp, line)) in lines.iter().enumerate() {
            if *timestamp < start || *timestamp >= end {
                continue;
            }
            let mut parsed: Option<Option<BTreeMap<String, String>>> = None;
            let keep = stages.iter().all(|stage| match stage {
                Stage::Contains(text) => line.contains(text.as_str()),
                Stage::Logfmt => {
                    parsed = Some(logfmt(line));
                    true
                }
                Stage::Equal(key, value) => {
                    parsed
                        .as_ref()
                        .expect("emulator: a label filter before a parser")
                        .as_ref()
                        .and_then(|m| m.get(key))
                        == Some(value)
                }
            });
            if keep {
                kept.push((*timestamp, group, entry, line));
            }
        }
    }
    kept.sort();
    kept.truncate(limit);
    let mut result: Vec<(usize, Value)> = Vec::new();
    for (timestamp, group, _, line) in kept {
        let position = match result.iter().position(|(g, _)| *g == group) {
            Some(p) => p,
            None => {
                result.push((group, json!({"stream": streams[group].0, "values": []})));
                result.len() - 1
            }
        };
        result[position].1["values"]
            .as_array_mut()
            .unwrap()
            .push(json!([timestamp.to_string(), line]));
    }
    result.sort_by_key(|(g, _)| *g);
    json!({"status": "success", "data": {"resultType": "streams",
           "result": result.into_iter().map(|(_, v)| v).collect::<Vec<_>>()}})
}

type Queries = Arc<Mutex<Vec<String>>>;

/// A local Loki answering every connection from `streams`; records each query.
async fn emulator(streams: Vec<Stream>) -> (String, Queries) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let queries: Queries = Arc::new(Mutex::new(Vec::new()));
    let record = queries.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let target = head.split(' ').nth(1).unwrap().to_owned();
            let (_, query) = target.split_once('?').unwrap_or((&target, ""));
            let params: BTreeMap<String, String> = query
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    (url_decode(k), url_decode(v))
                })
                .collect();
            assert_eq!(params["direction"], "forward");
            record.lock().unwrap().push(params["query"].clone());
            let answer = evaluate(
                &streams,
                &params["query"],
                params["start"].parse().unwrap(),
                params["end"].parse().unwrap(),
                params["limit"].parse().unwrap(),
            );
            let bytes = serde_json::to_vec(&answer).unwrap();
            let head = format!(
                "HTTP/1.1 200 X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    (format!("http://{address}/"), queries)
}

fn equal(label: &str, value: &str) -> Value {
    json!({"label": label, "operator": "equal", "value": value})
}

fn configuration(base: &str) -> Value {
    json!({"format": "connectors-loki-local/1", "instance": "l", "base_url": base,
    "ca_digest": null, "query_scope": {"required_equalities": []},
    "line_profiles": [{
        "name": "fixture-cdr",
        "selector": [equal("service", "edge-proxy"), equal("stream_kind", "cdr")],
        "format": {"kind": "logfmt"},
        "fields": [
            {"name": "direction", "key": "dir", "type": "string", "filterable": true},
            {"name": "caller", "key": "src", "type": "phone_number", "filterable": true},
            {"name": "callee", "key": "dst", "type": "phone_number", "filterable": true},
            {"name": "call_id", "key": "callid", "type": "string"}
        ],
        "call_identifier": "call_id"
    }],
    "trace_profiles": [{
        "name": "fixture-by-field",
        "selector": [equal("env", "fixture")],
        "call_identifier": {"kind": "field",
                            "value": {"key": "callid", "format": {"kind": "logfmt"}}},
        "source_label": "service"
    }]})
}

fn loki(base: &str) -> Loki {
    let http = Arc::new(
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
    );
    Loki::new("l", &configuration(base), http)
        .unwrap()
        .with_clock(|| NOW_MS)
}

/// Synthetic CDR lines: two callers, three callees, one non-CDR line.
fn cdr_streams() -> Vec<Stream> {
    vec![(
        json!({"env": "fixture", "service": "edge-proxy", "stream_kind": "cdr"}),
        vec![
            (
                1_788_822_090_000_000_000,
                "dir=inbound src=+15550100001 dst=+15550100002 callid=fx-leg-0001",
            ),
            (1_788_822_095_000_000_000, "event=heartbeat status=ok"),
            (
                1_788_822_100_000_000_000,
                "dir=outbound src=+15550100007 dst=+15550100003 callid=fx-leg-0003",
            ),
        ],
    )]
}

/// Synthetic trace lines of three calls from two sources.
fn trace_streams() -> Vec<Stream> {
    vec![
        (
            json!({"env": "fixture", "service": "edge-proxy"}),
            vec![
                (
                    1_788_822_000_100_000_000,
                    "callid=fx-leg-0001 msg=\"INVITE from +15550100001\"",
                ),
                (
                    1_788_822_000_300_000_000,
                    "callid=fx-leg-0002 msg=\"INVITE from +15550100007\"",
                ),
            ],
        ),
        (
            json!({"env": "fixture", "service": "app"}),
            vec![(
                1_788_822_000_200_000_000,
                "callid=fx-leg-0003 msg=\"routing decided\"",
            )],
        ),
    ]
}

async fn profile_lines(loki: &Loki, matched: &str, limit: usize) -> Value {
    loki.invoke(
        "logs.profile_lines",
        json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
               "limit": limit, "match": {"caller": matched}}),
    )
    .await
    .unwrap()
}

/// §12.2: "A record carries no raw line, so no unmasked value reaches the answer",
/// and the lookup story's acceptance: numbers are masked. The caller may look a
/// call up by its exact number; it must not learn a number it does not already hold.
/// Today the line filter `|= "<value>"` selects every line containing the value
/// anywhere, and `skipped.unmatched` (and, at `limit: 1`, `complete`) tells a prefix
/// of a masked number from a value no line holds, so extending a known prefix one
/// digit at a time recovers every masked number in the window.
#[tokio::test]
async fn a_masked_number_cannot_be_recovered_digit_by_digit_through_match() {
    let (base, _) = emulator(cdr_streams()).await;
    let loki = loki(&base);
    for limit in [1000, 1] {
        // What the answer is when no line holds the value at all.
        let nothing = profile_lines(&loki, "#", limit).await;
        let mut known = String::from("+1");
        'extend: loop {
            for digit in '0'..='9' {
                let candidate = format!("{known}{digit}");
                let answer = profile_lines(&loki, &candidate, limit).await;
                if answer["records"] == json!([]) && answer != nothing {
                    known = candidate;
                    continue 'extend;
                }
            }
            break;
        }
        assert_eq!(
            known, "+1",
            "limit {limit}: the answers to a match on a masked number recovered the \
             prefix {known:?} of a number no record discloses"
        );
    }
}

/// §12.3: "No line of another call is answered". The trace answer must not tell a
/// caller whether lines of other calls exist, nor what they hold: today
/// `excluded` counts every selected line containing the caller's text, so a
/// fragment probe reveals other calls' lines and recovers their raw content.
#[tokio::test]
async fn a_trace_answer_tells_nothing_about_another_calls_lines() {
    let (base, _) = emulator(trace_streams()).await;
    let loki = loki(&base);
    let trace = |call_id: &'static str| {
        let loki = &loki;
        async move {
            let mut out = loki
                .invoke(
                    "logs.call_trace",
                    json!({"profile": "fixture-by-field", "call_id": call_id,
                           "start_unix_ns": START, "end_unix_ns": END}),
                )
                .await
                .unwrap();
            out.as_object_mut().unwrap().remove("call_id");
            out
        }
    };
    // No call has either identifier; one is a fragment of other calls' lines.
    let fragment = trace("+15550100007").await;
    let nothing = trace("fx-none").await;
    assert_eq!(fragment["lines"], json!([]));
    assert_eq!(
        fragment, nothing,
        "a trace of an identifier no call has disclosed that other calls' lines hold it"
    );
}

/// §12.3: "a line is answered only when it reads in the declared format and its key
/// is exactly the identifier". The narrowing line filter must select every such
/// line; today it is the value's `logfmt_spelling`, which the reader does not
/// invert: the reader accepts a bare `=` and a `\u` escape that the spelling never
/// writes, so a line the adapter would read as this call's is never fetched, and the
/// answer says `complete: true`.
#[tokio::test]
async fn every_line_the_reader_reads_as_the_call_is_fetched() {
    let cases: [(&str, &'static str); 2] = [
        ("fx=7", "callid=fx=7 msg=one"),
        ("fx-caf\u{e9}", "callid=\"fx-caf\\u00e9\" msg=one"),
    ];
    let mut missing = Vec::new();
    for (call_id, line) in cases {
        let (base, queries) = emulator(vec![(
            json!({"env": "fixture", "service": "app"}),
            vec![(1_788_822_000_100_000_000, line)],
        )])
        .await;
        let out = loki(&base)
            .invoke(
                "logs.call_trace",
                json!({"profile": "fixture-by-field", "call_id": call_id,
                       "start_unix_ns": START, "end_unix_ns": END}),
            )
            .await
            .unwrap();
        if out["lines"].as_array().unwrap().len() != 1 {
            missing.push(format!(
                "{call_id:?}: line {line:?} not fetched by {:?}; complete={}",
                queries.lock().unwrap()[0],
                out["complete"]
            ));
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

/// §12.2: `match` values are "1–512 characters" and "any other input the operation
/// does not declare is InvalidInput before dispatch". `profile_lines` does not check
/// a match value's length itself; the published input schema's `maxLength`, which
/// `invoke` validates first, is what holds this bound.
#[tokio::test]
async fn a_match_value_over_512_characters_is_refused_before_dispatch() {
    let (base, queries) = emulator(cdr_streams()).await;
    let error = loki(&base)
        .invoke(
            "logs.profile_lines",
            json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
                   "match": {"caller": "9".repeat(513)}}),
        )
        .await
        .err();
    let dispatched = queries.lock().unwrap().len();
    assert_eq!(
        (error.map(|e| e.code), dispatched),
        (Some(ErrorCode::InvalidInput), 0)
    );
}

/// The emulator itself: what it answers for the composed queries the profiles
/// produce today, so the cases above fail for the adapter and not the emulator.
#[test]
fn the_emulator_answers_like_loki() {
    let streams = cdr_streams();
    let answer = evaluate(
        &streams,
        r#"{service="edge-proxy",stream_kind="cdr"} |= "+1555010000""#,
        0,
        u64::MAX,
        1000,
    );
    assert_eq!(
        answer["data"]["result"][0]["values"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let answer = evaluate(
        &streams,
        r#"{service="edge-proxy",stream_kind="cdr"} | logfmt | src="+1555010000""#,
        0,
        u64::MAX,
        1000,
    );
    assert_eq!(answer["data"]["result"], json!([]));
    let answer = evaluate(
        &streams,
        r#"{service="edge-proxy",stream_kind="other"}"#,
        0,
        u64::MAX,
        1000,
    );
    assert_eq!(answer["data"]["result"], json!([]));
}
