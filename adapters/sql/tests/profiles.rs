//! `profile.read` on the wire: a configured read profile pages by its keyset cursor across a
//! server that returns fewer rows than any LIMIT asks for, binds only the caller's values for
//! the declared parameters, writes each item under the declared names and types, masks
//! numbers by default, and publishes each profile's exact schemas in the instance descriptor.
//! The scripted MySQL server and the synthetic call table are fictional (`tests/mysql`).
#![cfg(unix)]
#[path = "mysql/calls.rs"]
mod calls;
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_core::ErrorCode;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, ReadProfile, Sql};
use serde_json::{Value, json};
use std::sync::Arc;

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(fixture::PASSWORD.as_bytes().to_vec()))
    }
}

fn config(port: u16) -> Config {
    Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    }
}
fn profiles(documents: &[Value]) -> Vec<ReadProfile> {
    documents
        .iter()
        .map(|d| serde_json::from_value(d.clone()).unwrap())
        .collect()
}
fn sql(port: u16, documents: &[Value]) -> Sql {
    let config = config(port);
    let effective = json!({"service":{"instance":"fixture-sql","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new("fixture-sql", config, effective, Arc::new(Password))
        .unwrap()
        .with_read_profiles(profiles(documents))
        .unwrap()
}
async fn read(sql: &Sql, input: Value) -> connectors_core::Result<Value> {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql.invoke("profile.read", input),
    )
    .await
    .expect("profile.read hung")
}
fn masked(number: &str) -> String {
    let count = number.chars().count();
    let kept: String = number.chars().skip(count.saturating_sub(4)).collect();
    if count <= 4 {
        "*".repeat(count)
    } else {
        format!("{}{kept}", "*".repeat(count - 4))
    }
}

#[tokio::test]
async fn every_row_of_a_number_pages_across_a_server_row_cap_by_keyset_cursor() {
    // The server returns at most three rows whatever LIMIT says; the caller asks for five.
    let server = fixture::start(calls::script(Some(3))).await;
    let sql = sql(server.port, &[calls::profile()]);
    let schema = sql
        .descriptor()
        .operation("profile.read")
        .unwrap()
        .output_schema
        .clone();
    let mut cursor: Option<String> = None;
    let mut items = Vec::new();
    let mut pages = Vec::new();
    loop {
        let page = read(&sql, calls::input(cursor.as_deref(), 5))
            .await
            .unwrap();
        connectors_sdk::validate(&schema, &page).unwrap_or_else(|e| panic!("{page}: {e:?}"));
        assert_eq!(page["profile"], "calls-by-number");
        assert_eq!(page["call_identifier"], "call_id");
        assert_eq!(page["ordering"], "row_key");
        let got = page["items"].as_array().unwrap().clone();
        pages.push(got.len());
        assert_eq!(page["complete"], got.is_empty(), "{page}");
        assert_eq!(page["next_cursor"].is_null(), got.is_empty(), "{page}");
        items.extend(got);
        if page["complete"] == true {
            break;
        }
        cursor = Some(page["next_cursor"].as_str().unwrap().to_owned());
        assert!(pages.len() < 10, "the read does not end");
    }
    // A short page did not end the read: it went on until a page came back empty.
    assert_eq!(pages, [3, 3, 1, 0]);
    let expected = calls::expected();
    assert_eq!(items.len(), expected.len());
    for (item, row) in items.iter().zip(&expected) {
        assert_eq!(item["row_key"], row.row_key.to_string());
        assert_eq!(item["call_id"], row.call_id);
        assert_eq!(item["direction"], row.direction);
        assert_eq!(item["caller"], masked(row.caller));
        assert_eq!(item["callee"], masked(row.callee));
        assert_eq!(item["leg_type"], row.leg_type);
        let s = row.started;
        assert_eq!(
            item["started_at"],
            format!("2026-01-{:02}T{:02}:{:02}:{:02}", s[0], s[1], s[2], s[3])
        );
        assert_eq!(item.as_object().unwrap().len(), 8, "{item}");
    }
    // Both legs of the transferred call are there, each with its type.
    let legs: Vec<_> = items
        .iter()
        .filter(|i| i["call_id"] == calls::TRANSFERRED_CALL)
        .map(|i| i["leg_type"].as_str().unwrap())
        .collect();
    assert_eq!(legs, ["customer", "agent"]);
    let log = server.log.lock().unwrap();
    // The declared parameters are bound in declared order, the cursor after them, and no
    // value is ever written into a statement.
    assert_eq!(
        log.executions,
        [
            vec![
                Some(calls::NUMBER.into()),
                Some(calls::FROM.into()),
                Some(calls::TO.into())
            ],
            vec![
                Some(calls::NUMBER.into()),
                Some(calls::FROM.into()),
                Some(calls::TO.into()),
                Some("14".into())
            ],
            vec![
                Some(calls::NUMBER.into()),
                Some(calls::FROM.into()),
                Some(calls::TO.into()),
                Some("17".into())
            ],
            vec![
                Some(calls::NUMBER.into()),
                Some(calls::FROM.into()),
                Some(calls::TO.into()),
                Some("18".into())
            ],
        ]
    );
    for statement in &log.prepares {
        assert!(!statement.contains(calls::NUMBER), "{statement}");
        assert!(!statement.contains("2026-01"), "{statement}");
    }
    // Every page runs the operator's statement inside the derived table, in keyset order,
    // asking for one row past the limit; a continuation compares the ordering column with
    // the bound cursor as an exact number.
    let wrapped: Vec<_> = log
        .prepares
        .iter()
        .filter(|p| p.starts_with("SELECT * FROM ("))
        .collect();
    assert_eq!(wrapped.len(), 4);
    assert_eq!(
        wrapped[0].as_str(),
        format!(
            "SELECT * FROM ({}\n) AS result (c0,c1,c2,c3,c4,c5,c6,c7) ORDER BY CAST(c0 AS DECIMAL(65,0)) LIMIT 6",
            calls::STATEMENT
        )
    );
    assert_eq!(
        wrapped[1].as_str(),
        format!(
            "SELECT * FROM ({}\n) AS result (c0,c1,c2,c3,c4,c5,c6,c7) WHERE CAST(c0 AS DECIMAL(65,0)) > CAST(? AS DECIMAL(65,0)) ORDER BY CAST(c0 AS DECIMAL(65,0)) LIMIT 6",
            calls::STATEMENT
        )
    );
    // Each page ran in its own read-only session.
    assert_eq!(log.sessions, 4);
    assert_eq!(
        log.statements
            .iter()
            .filter(|s| *s == "SET SESSION TRANSACTION READ ONLY")
            .count(),
        4
    );
}

#[tokio::test]
async fn a_page_that_received_more_than_its_limit_returns_the_limit_and_continues_after_it() {
    let server = fixture::start(calls::script(None)).await;
    let sql = sql(server.port, &[calls::profile()]);
    let first = read(&sql, calls::input(None, 2)).await.unwrap();
    let keys: Vec<_> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["row_key"].clone())
        .collect();
    assert_eq!(keys, [json!("11"), json!("12")]);
    assert_eq!(first["complete"], false);
    let second = read(&sql, calls::input(first["next_cursor"].as_str(), 100))
        .await
        .unwrap();
    let keys: Vec<_> = second["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["row_key"].clone())
        .collect();
    assert_eq!(
        keys,
        [
            json!("14"),
            json!("15"),
            json!("16"),
            json!("17"),
            json!("18")
        ]
    );
    assert_eq!(second["complete"], false);
    let last = read(&sql, calls::input(second["next_cursor"].as_str(), 100))
        .await
        .unwrap();
    assert_eq!(last["items"], json!([]));
    assert_eq!(last["complete"], true);
    assert_eq!(last["next_cursor"], Value::Null);
}

#[tokio::test]
async fn numbers_are_masked_by_default_and_a_profile_may_unmask_a_column() {
    let server = fixture::start(calls::script(None)).await;
    let mut profile = calls::profile();
    profile["columns"][4]["masked"] = json!(false);
    profile["columns"][2]["masked"] = json!(true);
    let sql = sql(server.port, &[profile]);
    let page = read(&sql, calls::input(None, 1)).await.unwrap();
    let item = &page["items"][0];
    assert_eq!(item["caller"], "********0101");
    assert_eq!(item["callee"], "+99900000900");
    // A text column the profile marks masked is masked too: `inbound` keeps its last four.
    assert_eq!(item["direction"], "***ound");
    assert_eq!(
        page["columns"],
        json!([
            {"name":"row_key","type":"integer","masked":false},
            {"name":"call_id","type":"text","masked":false},
            {"name":"direction","type":"text","masked":true},
            {"name":"caller","type":"phone_number","masked":true},
            {"name":"callee","type":"phone_number","masked":false},
            {"name":"started_at","type":"timestamp","masked":false},
            {"name":"ended_at","type":"timestamp","masked":false},
            {"name":"leg_type","type":"text","masked":false}
        ])
    );
    // Values of four characters or fewer are wholly masked, and nothing else is kept.
    for (value, expected) in [
        ("", ""),
        ("1", "*"),
        ("1234", "****"),
        ("12345", "*2345"),
        ("+9 99", "*9 99"),
    ] {
        assert_eq!(connectors_sql::mask(value), expected, "{value:?}");
    }
}

#[tokio::test]
async fn a_value_its_declared_type_does_not_admit_refuses_the_read() {
    // `leg_type` declared as an integer, and `call_id` as a timestamp: neither holds.
    for (index, kind) in [(7, "integer"), (1, "timestamp")] {
        let server = fixture::start(calls::script(None)).await;
        let mut profile = calls::profile();
        profile["columns"][index]["type"] = json!(kind);
        if index == 1 {
            profile["call_identifier"] = json!("row_key");
        }
        let sql = sql(server.port, &[profile]);
        let error = read(&sql, calls::input(None, 5)).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unsupported, "{kind}");
    }
}

#[tokio::test]
async fn a_statement_that_returns_other_columns_than_declared_is_refused_before_it_runs() {
    let server = fixture::start(calls::script(None)).await;
    let mut profile = calls::profile();
    profile["columns"].as_array_mut().unwrap().pop();
    let sql = sql(server.port, &[profile]);
    let error = read(&sql, calls::input(None, 5)).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(server.log.lock().unwrap().executions.is_empty());
}

#[tokio::test]
async fn a_profile_statement_that_writes_is_refused_before_it_runs() {
    let server = fixture::start(calls::script(None)).await;
    let mut profile = calls::profile();
    profile["statement"] = json!("UPDATE fixture_calls SET leg = ? WHERE id = ? AND started < ?");
    let sql = sql(server.port, &[profile]);
    let error = read(&sql, calls::input(None, 5)).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    let log = server.log.lock().unwrap();
    assert!(log.executions.is_empty());
    assert_eq!(log.statements[0], "SET SESSION TRANSACTION READ ONLY");
}

#[tokio::test]
async fn only_the_declared_parameters_are_admitted_and_refusals_open_no_session() {
    let server = fixture::start(calls::script(None)).await;
    let sql = sql(server.port, &[calls::profile()]);
    let issued = read(&sql, calls::input(None, 1)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let sessions = server.log.lock().unwrap().sessions;
    let mut undeclared = calls::input(None, 5);
    undeclared["parameters"]["table"] = json!("other");
    let mut missing = calls::input(None, 5);
    missing["parameters"].as_object_mut().unwrap().remove("to");
    let mut unknown = calls::input(None, 5);
    unknown["profile"] = json!("other-profile");
    let mut statement = calls::input(None, 5);
    statement["query"] = json!("SELECT 1");
    let mut other_values = calls::input(Some(&issued), 5);
    other_values["parameters"]["number"] = json!("+99900000777");
    for (input, code) in [
        (undeclared, ErrorCode::InvalidInput),
        (missing, ErrorCode::InvalidInput),
        (unknown, ErrorCode::InvalidInput),
        (statement, ErrorCode::InvalidInput),
        (other_values, ErrorCode::StaleCursor),
        (
            calls::input(Some("not-a-cursor"), 5),
            ErrorCode::StaleCursor,
        ),
        (
            calls::input(Some("eyJwcm9maWxlIjoib3RoZXIifQ"), 5),
            ErrorCode::StaleCursor,
        ),
    ] {
        let error = read(&sql, input.clone()).await.unwrap_err();
        assert_eq!(error.code, code, "{input}");
    }
    assert_eq!(
        server.log.lock().unwrap().sessions,
        sessions,
        "a refusal opened a session"
    );
    // A hostile value is bound as it is and never reaches a statement's text.
    let mut hostile = calls::input(None, 5);
    let value = "x' OR '1'='1'; -- \n#";
    hostile["parameters"]["number"] = json!(value);
    read(&sql, hostile).await.unwrap();
    let log = server.log.lock().unwrap();
    assert_eq!(log.executions.last().unwrap()[0].as_deref(), Some(value));
    assert!(log.prepares.iter().all(|p| !p.contains("OR '1'")));
}

#[test]
fn the_instance_descriptor_publishes_each_profiles_exact_schemas() {
    let mut second = calls::profile();
    second["id"] = json!("legs-by-call");
    second["parameters"] = json!([]);
    // No profile: the operation is not advertised at all.
    let none = sql(1, &[]);
    assert!(none.descriptor().operation("profile.read").is_err());
    assert_eq!(none.descriptor().operations.len(), 6);
    let both = sql(1, &[calls::profile(), second]);
    let descriptor = both.descriptor();
    let operation = descriptor.operation("profile.read").unwrap();
    assert_eq!(operation.contract, "datasource.records/v1alpha1");
    assert_eq!(operation.profile, "sql-configured-read-profile");
    assert_ne!(descriptor.revision, none.descriptor().revision);
    let input = &operation.input_schema;
    let valid = |schema: &Value, value: Value| connectors_sdk::validate(schema, &value).is_ok();
    assert!(valid(input, calls::input(None, 5)));
    assert!(valid(input, json!({"profile":"legs-by-call","limit":5})));
    assert!(valid(
        input,
        json!({"profile":"legs-by-call","parameters":{},"limit":5})
    ));
    assert!(!valid(
        input,
        json!({"profile":"calls-by-number","limit":5})
    ));
    assert!(!valid(
        input,
        json!({"profile":"legs-by-call","parameters":{"number":"1"},"limit":5})
    ));
    assert!(!valid(input, json!({"profile":"nope","limit":5})));
    // The published output schema names each profile's columns, types and key columns.
    let output = &operation.output_schema;
    let profiles = output["oneOf"].as_array().unwrap();
    assert_eq!(profiles.len(), 2);
    let first = &profiles[0];
    assert_eq!(first["properties"]["profile"]["const"], "calls-by-number");
    assert_eq!(first["properties"]["call_identifier"]["const"], "call_id");
    let item = &first["properties"]["items"]["items"];
    let names: Vec<_> = item["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let mut expected = vec![
        "row_key",
        "call_id",
        "direction",
        "caller",
        "callee",
        "started_at",
        "ended_at",
        "leg_type",
    ];
    expected.sort();
    let mut names = names;
    names.sort();
    assert_eq!(names, expected);
    assert_eq!(item["additionalProperties"], false);
    let page = |item: Value| {
        json!({"profile":"calls-by-number","call_identifier":"call_id","ordering":"row_key",
            "columns": [
                {"name":"row_key","type":"integer","masked":false},
                {"name":"call_id","type":"text","masked":false},
                {"name":"direction","type":"text","masked":false},
                {"name":"caller","type":"phone_number","masked":true},
                {"name":"callee","type":"phone_number","masked":true},
                {"name":"started_at","type":"timestamp","masked":false},
                {"name":"ended_at","type":"timestamp","masked":false},
                {"name":"leg_type","type":"text","masked":false}],
            "items":[item],"next_cursor":"c","complete":false,
            "provenance":{"instance":"fixture-sql","resource":"fixture","observed_at_unix_ms":1,"source_revision":null}})
    };
    let good = json!({"row_key":"11","call_id":"call-0001","direction":"inbound","caller":"********0101",
        "callee":"********0900","started_at":"2026-01-02T09:00:00","ended_at":null,"leg_type":"customer"});
    assert!(valid(output, page(good.clone())));
    for (name, value) in [
        ("call_id", Value::Null),
        ("row_key", json!("eleven")),
        ("started_at", json!("yesterday")),
        ("caller", json!("+99900000101")),
        ("extra", json!("x")),
    ] {
        let mut bad = good.clone();
        bad[name] = value;
        assert!(!valid(output, page(bad.clone())), "{bad}");
    }
}

#[test]
fn a_profile_the_model_refuses_is_refused_by_the_library() {
    let fresh = || {
        let config = config(1);
        let effective = json!({"service":{"instance":"fixture-sql","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
        Sql::new("fixture-sql", config, effective, Arc::new(Password)).unwrap()
    };
    let mut duplicate = calls::profile();
    duplicate["columns"][2]["name"] = json!("call_id");
    let mut masked_key = calls::profile();
    masked_key["columns"][1]["masked"] = json!(true);
    let mut number_ordering = calls::profile();
    number_ordering["ordering"] = json!("caller");
    let mut absent_key = calls::profile();
    absent_key["call_identifier"] = json!("absent");
    for document in [duplicate, masked_key, number_ordering, absent_key] {
        let profile: ReadProfile = serde_json::from_value(document.clone()).unwrap();
        assert!(
            fresh().with_read_profiles(vec![profile]).is_err(),
            "{document}"
        );
    }
    let twice = vec![
        serde_json::from_value(calls::profile()).unwrap(),
        serde_json::from_value(calls::profile()).unwrap(),
    ];
    assert!(fresh().with_read_profiles(twice).is_err());
}

// ---------------------------------------------------------------------------------------
// Cursors: what is checked is shape, type and context (semantics.md, "Read profiles";
// `connectors_sql.profiles.Cursor` and `CursorAdmission`).

fn decode_cursor(token: &str) -> Value {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn encode_cursor(cursor: &Value) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(cursor).unwrap())
}
fn base64url(text: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(text)
}

/// Every refusal below comes before a session: the issued cursor is altered in one respect
/// at a time, and each alteration is one the adapter never issues.
#[tokio::test]
async fn a_cursor_of_the_wrong_shape_or_type_is_stale_before_any_session() {
    let server = fixture::start(calls::script(None)).await;
    let sql = sql(server.port, &[calls::profile()]);
    let issued = read(&sql, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let cursor = decode_cursor(&issued);
    assert_eq!(cursor["after"], "12", "{cursor}");
    let altered = |change: &dyn Fn(&mut Value)| {
        let mut cursor = cursor.clone();
        change(&mut cursor);
        encode_cursor(&cursor)
    };
    let cases = [
        // Type: the ordering column is an integer.
        (
            "a value the declared type does not admit",
            altered(&|c| c["after"] = json!("12x")),
        ),
        ("an empty value", altered(&|c| c["after"] = json!(""))),
        // Shape: longer than any issued value.
        (
            "a value over 8,192 bytes",
            altered(&|c| c["after"] = json!("1".repeat(8193))),
        ),
        // Shape: the base64 form is issued only for a value JSON escapes.
        (
            "a non-canonical base64 value",
            altered(&|c| {
                c.as_object_mut().unwrap().remove("after");
                c["after_base64"] = json!(base64url("12"));
            }),
        ),
        (
            "both value forms",
            altered(&|c| c["after_base64"] = json!(base64url("12"))),
        ),
        (
            "no value",
            altered(&|c| {
                c.as_object_mut().unwrap().remove("after");
            }),
        ),
        (
            "an unknown member",
            altered(&|c| c["signature"] = json!("x")),
        ),
        ("a padded token", format!("{issued}=")),
    ];
    for (why, token) in cases {
        let before = server.log.lock().unwrap().sessions;
        let error = read(&sql, calls::input(Some(&token), 2))
            .await
            .expect_err(why);
        assert_eq!(error.code, ErrorCode::StaleCursor, "{why}: {error:?}");
        assert_eq!(
            server.log.lock().unwrap().sessions,
            before,
            "{why} opened a session"
        );
    }
    // A timestamp ordering admits only a timestamp: the cursor it issued, with its value
    // replaced, is stale for its type alone.
    let mut by_start = calls::profile();
    by_start["ordering"] = json!("started_at");
    let timed = self::sql(server.port, &[by_start]);
    let issued = read(&timed, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut forged = decode_cursor(&issued);
    assert_eq!(forged["after"], "2026-01-02T09:01:10", "{forged}");
    forged["after"] = json!("yesterday");
    let before = server.log.lock().unwrap().sessions;
    let error = read(&timed, calls::input(Some(&encode_cursor(&forged)), 2))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleCursor);
    assert_eq!(server.log.lock().unwrap().sessions, before);
}

/// The context covers the connection: a cursor issued by one connection is stale on
/// another with the same profile, and stays good on its own.
#[tokio::test]
async fn a_cursor_issued_on_another_connection_is_stale() {
    let first = fixture::start(calls::script(None)).await;
    let second = fixture::start(calls::script(None)).await;
    let one = sql(first.port, &[calls::profile()]);
    let other = sql(second.port, &[calls::profile()]);
    let issued = read(&one, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let error = read(&other, calls::input(Some(&issued), 2))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleCursor);
    assert_eq!(second.log.lock().unwrap().sessions, 0);
    read(&one, calls::input(Some(&issued), 2)).await.unwrap();
}

/// A changed profile is a changed context even when its id, parameters and ordering are
/// the same: here only a column's masking changed.
#[tokio::test]
async fn a_cursor_issued_before_any_change_to_the_profile_is_stale() {
    let server = fixture::start(calls::script(None)).await;
    let before = sql(server.port, &[calls::profile()]);
    let issued = read(&before, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut changed = calls::profile();
    changed["columns"][2]["masked"] = json!(true);
    let after = sql(server.port, &[changed]);
    let sessions = server.log.lock().unwrap().sessions;
    let error = read(&after, calls::input(Some(&issued), 2))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleCursor);
    assert_eq!(server.log.lock().unwrap().sessions, sessions);
}

// ---------------------------------------------------------------------------------------
// The cursor's size: every issued token fits the published 16,384 characters.

fn keys_script(key: String) -> fixture::Script {
    fixture::Script {
        columns: vec![fixture::col(
            "k",
            fixture::VAR_STRING,
            0,
            fixture::UTF8MB4,
            0,
        )],
        rows: vec![vec![fixture::text(&key)]],
        params: 0,
        // The one row is served to the first page only.
        select: Some(Arc::new(|values: &[Option<String>], _| values.is_empty())),
        ..fixture::Script::default()
    }
}
fn keys_profile() -> Value {
    json!({
        "id": "k".repeat(64),
        "statement": "SELECT k FROM fixture_keys",
        "parameters": [],
        "columns": [{"name": "key", "type": "text"}],
        "call_identifier": "key",
        "ordering": "key"
    })
}

#[tokio::test]
async fn the_longest_ordering_value_issues_a_cursor_the_published_schemas_admit() {
    assert_eq!(connectors_sql::profile::MAX_CURSOR_VALUE, 8192);
    assert!(
        connectors_sql::profile::worst_cursor_length()
            <= connectors_sql::profile::CURSOR_LENGTH_LIMIT
    );
    // The worst values: every byte escaped by JSON (`"`, `\`, a control character), and
    // multi-byte characters.
    for key in [
        "\"".repeat(8192),
        "\\".repeat(8192),
        "\u{1}".repeat(8192),
        "é".repeat(4096),
        "1".repeat(8192),
    ] {
        let server = fixture::start(keys_script(key.clone())).await;
        let sql = sql(server.port, &[keys_profile()]);
        let schema = sql
            .descriptor()
            .operation("profile.read")
            .unwrap()
            .output_schema
            .clone();
        let input = json!({"profile": "k".repeat(64), "limit": 1});
        let first = read(&sql, input.clone()).await.unwrap();
        connectors_sdk::validate(&schema, &first).unwrap();
        let cursor = first["next_cursor"].as_str().unwrap().to_owned();
        assert!(cursor.len() <= 16384, "{} characters", cursor.len());
        assert!(cursor.len() <= connectors_sql::profile::worst_cursor_length());
        let mut next = input;
        next["cursor"] = json!(cursor);
        let last = read(&sql, next).await.unwrap();
        assert_eq!(last["complete"], true);
    }
    // One byte more is refused, not issued.
    let server = fixture::start(keys_script("1".repeat(8193))).await;
    let sql = sql(server.port, &[keys_profile()]);
    let error = read(&sql, json!({"profile": "k".repeat(64), "limit": 1}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
}

// ---------------------------------------------------------------------------------------
// A shared ordering value at a page boundary.

#[tokio::test]
async fn a_row_past_the_limit_sharing_the_last_ordering_value_refuses_the_page() {
    let mut rows = calls::ROWS;
    rows[2].row_key = rows[1].row_key;
    let script = |rows: [calls::CallRow; 9]| fixture::Script {
        rows: rows.iter().map(row_cells).collect(),
        select: Some(Arc::new(move |values: &[Option<String>], index: usize| {
            let after = values.get(3).cloned().flatten();
            after.is_none_or(|a| rows[index].row_key > a.parse::<i64>().unwrap())
        })),
        ..calls::script(None)
    };
    // Rows 11, 12, 12: a page of two ends between the two 12s.
    let server = fixture::start(script(rows)).await;
    let sql = sql(server.port, &[calls::profile()]);
    let error = read(&sql, calls::input(None, 2)).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(error.message.contains("`row_key`"), "{}", error.message);
    assert!(error.message.contains("not unique"), "{}", error.message);
    // A page of one ends between 11 and 12, and a page of three after both 12s: each is
    // served, and a shared value inside a page is not a boundary.
    read(&sql, calls::input(None, 1)).await.unwrap();
    let page = read(&sql, calls::input(None, 3)).await.unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 3);
}

fn row_cells(row: &calls::CallRow) -> Vec<fixture::Cell> {
    use fixture::*;
    let at = |t: [u8; 4]| datetime(2026, 1, t[0], [t[1], t[2], t[3]], 0);
    vec![
        int8(row.row_key),
        text(row.call_id),
        text(row.direction),
        text(row.caller),
        text(row.callee),
        at(row.started),
        at(row.ended),
        text(row.leg_type),
    ]
}

// ---------------------------------------------------------------------------------------
// One expression for ORDER BY and the keyset comparison, per declared type (MySQL).

/// The wrapper statement each page prepared: after the derived table's closing line.
fn tails(server: &fixture::Server) -> Vec<String> {
    server
        .log
        .lock()
        .unwrap()
        .prepares
        .iter()
        .filter_map(|p| p.split_once("\n) AS result (").map(|(_, t)| t.to_owned()))
        .collect()
}

#[tokio::test]
async fn mysql_orders_and_continues_by_the_same_expression_for_each_declared_type() {
    let aliases = "c0,c1,c2,c3,c4,c5,c6,c7)";
    for (ordering, expression, parameter) in [
        (
            "row_key",
            "CAST(c0 AS DECIMAL(65,0))",
            "CAST(? AS DECIMAL(65,0))",
        ),
        (
            "started_at",
            "CAST(c5 AS DATETIME(6))",
            "CAST(? AS DATETIME(6))",
        ),
        (
            "call_id",
            "CAST(CONVERT(c1 USING utf8mb4) AS BINARY)",
            "CAST(CONVERT(? USING utf8mb4) AS BINARY)",
        ),
    ] {
        // Every row but the first, whatever is bound: only the statement text is under
        // test, and no two of the first rows share a value of any of the three columns.
        let server = fixture::start(fixture::Script {
            rows: calls::ROWS[1..].iter().map(row_cells).collect(),
            select: None,
            ..calls::script(None)
        })
        .await;
        let mut profile = calls::profile();
        profile["ordering"] = json!(ordering);
        let sql = sql(server.port, &[profile]);
        let first = read(&sql, calls::input(None, 1)).await.unwrap();
        read(&sql, calls::input(first["next_cursor"].as_str(), 1))
            .await
            .unwrap();
        assert_eq!(
            tails(&server),
            [
                format!("{aliases} ORDER BY {expression} LIMIT 2"),
                format!("{aliases} WHERE {expression} > {parameter} ORDER BY {expression} LIMIT 2"),
            ],
            "{ordering}"
        );
    }
}

#[tokio::test]
async fn mysql_refuses_an_ordering_column_it_cannot_follow_by_name_before_any_row() {
    use fixture::*;
    let one = |column: Col, cell: Cell, kind: &str| {
        (
            Script {
                columns: vec![column],
                rows: vec![vec![cell]],
                params: 0,
                ..Script::default()
            },
            json!({
                "id": "keys",
                "statement": "SELECT k FROM fixture_keys",
                "parameters": [],
                "columns": [{"name": "the_key", "type": kind}],
                "call_identifier": "the_key",
                "ordering": "the_key"
            }),
        )
    };
    for (script, profile) in [
        // A binary string is written as base64: its bytes and its text order differently.
        one(
            col("k", VAR_STRING, 0, BINARY_CHARSET, 0),
            bytes(b"\x01\x02"),
            "text",
        ),
        // A number is not text the server writes as the page does.
        one(col("k", DOUBLE, 0, BINARY_CHARSET, 0), double(1.5), "text"),
        one(col("k", JSON, 0, BINARY_CHARSET, 0), text("{}"), "text"),
        // An integer is an integer column; a timestamp a DATETIME or TIMESTAMP.
        one(col("k", VAR_STRING, 0, UTF8MB4, 0), text("12"), "integer"),
        one(
            col("k", NEWDECIMAL, 0, BINARY_CHARSET, 0),
            text("12"),
            "integer",
        ),
        one(
            col("k", DATE, 0, BINARY_CHARSET, 0),
            date(2026, 1, 2),
            "timestamp",
        ),
        one(
            col("k", VAR_STRING, 0, UTF8MB4, 0),
            text("2026-01-02 09:00:00"),
            "timestamp",
        ),
    ] {
        let server = start(script).await;
        let sql = sql(server.port, std::slice::from_ref(&profile));
        let error = read(&sql, json!({"profile": "keys", "limit": 5}))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unsupported, "{profile}");
        assert!(error.message.contains("`the_key`"), "{}", error.message);
        let log = server.log.lock().unwrap();
        assert!(log.executions.is_empty(), "{profile}: a row was read");
        assert_eq!(log.prepares.len(), 1, "{profile}: the wrapper was prepared");
    }
    // ENUM and SET are text strings, ordered by their bytes rather than their index.
    for (script, profile) in [
        one(
            col("k", VAR_STRING, ENUM_FLAG, UTF8MB4, 0),
            text("inbound"),
            "text",
        ),
        one(
            col("k", LONG, UNSIGNED, BINARY_CHARSET, 0),
            int4(7),
            "integer",
        ),
        one(
            col("k", TIMESTAMP, 0, BINARY_CHARSET, 0),
            datetime(2026, 1, 2, [9, 0, 0], 0),
            "timestamp",
        ),
    ] {
        let server = start(script).await;
        let sql = sql(server.port, std::slice::from_ref(&profile));
        read(&sql, json!({"profile": "keys", "limit": 5}))
            .await
            .unwrap_or_else(|e| panic!("{profile}: {e:?}"));
    }
}

// ---------------------------------------------------------------------------------------
// A statement that is not a query, on MySQL.

/// A statement that prepares with columns but cannot be a derived table (`SHOW`, `EXPLAIN`)
/// is refused by the server's syntax check of the wrapper: that is `unsupported`, as a
/// statement that returns no columns is, on both `query.read` and `profile.read`.
#[tokio::test]
async fn mysql_a_statement_that_cannot_be_a_derived_table_is_unsupported() {
    let server = fixture::start(fixture::Script {
        wrapped_prepare_error: Some((1064, "42000")),
        ..calls::script(None)
    })
    .await;
    let profiled = sql(server.port, &[calls::profile()]);
    let error = read(&profiled, calls::input(None, 5)).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(server.log.lock().unwrap().executions.is_empty());
    let server = fixture::start(fixture::Script {
        wrapped_prepare_error: Some((1064, "42000")),
        ..fixture::Script::default()
    })
    .await;
    let error = sql(server.port, &[])
        .invoke("query.read", json!({"query": "SHOW TABLES", "limit": 5}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(server.log.lock().unwrap().executions.is_empty());
    // The caller's own statement refused at its preparation keeps the server's class.
    let server = fixture::start(fixture::Script {
        prepare_error: Some((1064, "42000")),
        ..fixture::Script::default()
    })
    .await;
    let error = sql(server.port, &[])
        .invoke("query.read", json!({"query": "SELEC 1", "limit": 5}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}
