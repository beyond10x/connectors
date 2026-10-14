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
            "SELECT * FROM ({}\n) AS result (c0,c1,c2,c3,c4,c5,c6,c7) ORDER BY c0 LIMIT 6",
            calls::STATEMENT
        )
    );
    assert_eq!(
        wrapped[1].as_str(),
        format!(
            "SELECT * FROM ({}\n) AS result (c0,c1,c2,c3,c4,c5,c6,c7) WHERE c0 > CAST(? AS DECIMAL(65,0)) ORDER BY c0 LIMIT 6",
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
