//! The executable against the adapter's own ESS model (`spec/ess`): the schemas
//! the pinned `ess` generates from it, and their invariants, accept the
//! configurations the executable accepts and refuse what it refuses, hold for
//! the profile each engine advertises, and fix the read binding and the MySQL
//! cell rules the library implements.
#![cfg(unix)]
#[path = "mysql/calls.rs"]
mod calls;
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_core::ErrorCode;
use connectors_host::local::runtime::Bootstrap;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{
    Config, Engine, MAX_COLUMNS, Sql,
    mysql::{Family, Rendering},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command, sync::Arc};

/// The schema of one model type, generated now by the pinned `ess`.
fn schema(out: &Path, name: &str) -> Value {
    let path = out.join(format!(
        "schemas/schema/types/connectors_sql.{name}.schema.json"
    ));
    if !path.exists() {
        let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
        let status = Command::new(ess)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args([
                "generate", "--path", "spec/ess", "--kind", "schema", "--out",
            ])
            .arg(out.join("schemas"))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "ess generate exited {status}");
    }
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn field<'a>(document: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(document, |value, key| value.get(key))
}

/// A rendered literal: JSON when it parses (quoted strings, numbers), else the
/// bare text, which is how the rendering writes enum variants and some strings.
fn literal(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_owned()))
}

/// The parts of `text` separated by `separator` outside parentheses and quotes.
fn split_top<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    let (mut parts, mut depth, mut quoted, mut start) = (Vec::new(), 0usize, false, 0);
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => quoted = !quoted,
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth -= 1,
            _ if !quoted && depth == 0 && text[index..].starts_with(separator) => {
                parts.push(&text[start..index]);
                index += separator.len();
                start = index;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    parts.push(&text[start..]);
    parts
}

/// The text inside `prefix(` … `)` when that closing parenthesis ends `text`.
fn enclosed<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let inner = text.strip_prefix(prefix)?.strip_prefix('(')?;
    let mut depth = 1usize;
    for (index, c) in inner.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return (index == inner.len() - 1).then(|| &inner[..index]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The document with `name` bound to `value`, so a path through the binder resolves.
fn bind(document: &Value, name: &str, value: &Value) -> Value {
    let mut bound = document.clone();
    bound
        .as_object_mut()
        .expect("a quantifier ranges inside a struct")
        .insert(name.to_owned(), value.clone());
    bound
}

/// Evaluate one rendered `x-ess-invariants` entry of the forms this model uses.
/// An optional field that is absent satisfies its invariant, as in ESS. Any
/// other form panics, so a new invariant cannot pass unchecked.
fn holds(document: &Value, invariant: &str) -> bool {
    let invariant = invariant.trim();
    // `forall v in path: body`, `exists v in path: body`, `distinct v in path [by v.member]`.
    // An absent (optional) list satisfies all three.
    for quantifier in ["forall ", "exists ", "distinct "] {
        let Some(rest) = invariant.strip_prefix(quantifier) else {
            continue;
        };
        let (name, rest) = rest.split_once(" in ").unwrap();
        if quantifier == "distinct " {
            let (path, member) = match rest.split_once(" by ") {
                Some((path, member)) => (path, Some(member)),
                None => (rest, None),
            };
            let Some(Value::Array(items)) = field(document, path) else {
                return true;
            };
            let keys: Vec<Value> = items
                .iter()
                .map(|item| match member {
                    Some(member) => field(&bind(document, name, item), member)
                        .cloned()
                        .unwrap_or(Value::Null),
                    None => item.clone(),
                })
                .collect();
            return keys
                .iter()
                .enumerate()
                .all(|(index, key)| !keys[..index].contains(key));
        }
        let (path, body) = rest.split_once(": ").unwrap();
        let Some(Value::Array(items)) = field(document, path) else {
            return true;
        };
        let mut each = items
            .iter()
            .map(|item| holds(&bind(document, name, item), body));
        return if quantifier == "forall " {
            each.all(|held| held)
        } else {
            each.any(|held| held)
        };
    }
    if let Some(inner) = enclosed(invariant, "not ") {
        return !holds(document, inner);
    }
    if let Some(inner) = enclosed(invariant, "") {
        let any = split_top(inner, " or ");
        if any.len() > 1 {
            return any.into_iter().any(|part| holds(document, part));
        }
        return split_top(inner, " and ")
            .into_iter()
            .all(|part| holds(document, part));
    }
    if let Some(path) = enclosed(invariant, "defined") {
        return field(document, path).is_some_and(|v| !v.is_null());
    }
    if let Some((left, right)) = invariant.split_once(" starts_with ") {
        let prefix = literal(right);
        return field(document, left).is_none_or(|v| {
            v.as_str()
                .is_some_and(|s| s.starts_with(prefix.as_str().unwrap()))
        });
    }
    // An absent member matches no suffix, so a model guard that must hold
    // for an absent member names `missing(...)` itself.
    if let Some((left, right)) = invariant.split_once(" ends_with ") {
        let suffix = literal(right);
        return field(document, left).is_some_and(|v| {
            v.as_str()
                .is_some_and(|s| s.ends_with(suffix.as_str().unwrap()))
        });
    }
    for operator in [" == ", " != ", " >= ", " <= "] {
        let Some((left, right)) = invariant.split_once(operator) else {
            continue;
        };
        // In ESS a right-hand side with a dot names a member of the same
        // struct; without one it is a literal. An absent member compares as null.
        let expected =
            if right.contains('.') && !right.starts_with('"') && right.parse::<f64>().is_err() {
                field(document, right).cloned().unwrap_or(Value::Null)
            } else {
                literal(right)
            };
        let measured = |name: &str, measure: fn(&Value) -> Value| match field(document, name) {
            // An optional member that is absent or null holds, as in ESS.
            None | Some(Value::Null) => None,
            Some(value) => Some(measure(value)),
        };
        let actual = if let Some(name) = left.strip_suffix(".count") {
            measured(name, |value| match value {
                Value::String(s) => json!(s.chars().count()),
                Value::Array(a) => json!(a.len()),
                other => panic!("count of {other}"),
            })
        } else if let Some(name) = left.strip_suffix(".utf8_bytes") {
            measured(name, |value| {
                json!(value.as_str().expect("utf8_bytes of text").len())
            })
        } else {
            field(document, left).cloned()
        };
        let Some(actual) = actual else {
            return true;
        };
        return match operator {
            " == " => actual == expected,
            " != " => actual != expected,
            " >= " => actual.as_u64().unwrap() >= expected.as_u64().unwrap(),
            _ => actual.as_u64().unwrap() <= expected.as_u64().unwrap(),
        };
    }
    panic!("unrecognised invariant form: {invariant}")
}

/// Every schema rule and every invariant of every definition the document reaches.
fn conforms(schema: &Value, document: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let root = schema["$ref"]
        .as_str()
        .unwrap()
        .trim_start_matches("#/$defs/");
    check(schema, root, document)
}

fn check(schema: &Value, name: &str, document: &Value) -> Result<(), String> {
    let definition = &schema["$defs"][name];
    let subject = if definition["x-ess-kind"] == "newtype" {
        json!({ "value": document })
    } else {
        document.clone()
    };
    for invariant in definition["x-ess-invariants"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let invariant = invariant.as_str().unwrap();
        if !holds(&subject, invariant) {
            return Err(format!("{name}: {invariant}"));
        }
    }
    if let Some(alphabet) = definition["x-ess-alphabet"].as_str() {
        let text = document
            .as_str()
            .ok_or_else(|| format!("{name}: not text"))?;
        if let Some(c) = text.chars().find(|c| !alphabet.contains(*c)) {
            return Err(format!("{name}: {c:?} is outside its alphabet"));
        }
    }
    for (key, property) in definition["properties"].as_object().into_iter().flatten() {
        let reference = |property: &Value| {
            property["$ref"]
                .as_str()
                .map(|r| r.trim_start_matches("#/$defs/").to_owned())
        };
        match (
            document.get(key),
            reference(property),
            reference(&property["items"]),
        ) {
            (Some(value), Some(reference), _) => check(schema, &reference, value)?,
            (Some(Value::Array(items)), None, Some(reference)) => {
                for item in items {
                    check(schema, &reference, item)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// The executable's own verdict on one configuration document.
fn bootstrap(directory: &Path, configuration: &Value) -> Option<Bootstrap> {
    let path = directory.join("sql.json");
    fs::write(&path, serde_json::to_vec(configuration).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-sql"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    output
        .status
        .success()
        .then(|| serde_json::from_slice(&output.stdout).unwrap())
}

/// Marks a member to remove. JSON `null` is kept as a literal null, which is a
/// different document from an absent member and must reach both verdicts.
const ABSENT: &str = "<absent>";

fn configuration(changes: Value) -> Value {
    let mut document = json!({
        "format":"connectors-sql-local/1",
        "instance":"fixture-sql",
        "host":"db.example.test",
        "port":3306,
        "database":"fixture",
        "user":"reader",
        "allow_plaintext":false
    });
    for (key, value) in changes.as_object().unwrap() {
        if value == ABSENT {
            document.as_object_mut().unwrap().remove(key);
        } else {
            document[key] = value.clone();
        }
    }
    document
}

#[test]
fn the_model_admits_exactly_the_configurations_the_executable_admits() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "connection.LocalConfiguration");
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    connectors_host::local::filesystem::directory(&directory, true, true).unwrap();
    for (changes, admitted) in [
        (json!({}), true),
        (json!({"engine":"postgresql"}), true),
        (json!({"engine":"mysql"}), true),
        (json!({"engine":"mysql","allow_plaintext":ABSENT}), true),
        (json!({"engine":"mysql","allow_plaintext":null}), false),
        (json!({"engine":"mysql","ca_file":null}), true),
        (json!({"engine":"postgresql","ca_file":null}), true),
        (json!({"ca_file":null}), true),
        (json!({"engine":"mysql","ca_file":""}), false),
        (json!({"engine":null}), false),
        (json!({"engine":"sqlite"}), false),
        (json!({"engine":"MySQL"}), false),
        (json!({"engine":"mysql","port":ABSENT}), false),
        (json!({"engine":"mysql","port":null}), false),
        (json!({"engine":"mysql","port":0}), false),
        (json!({"engine":"mysql","host":"/var/run/mysqld"}), false),
        (json!({"engine":"mysql","host":""}), false),
        (json!({"engine":"mysql","user":""}), false),
        (json!({"format":"connectors-sql-local/2"}), false),
        (json!({"engine":"mysql","password":"never-here"}), false),
    ]
    .into_iter()
    .chain(
        // The bounds count characters on both sides: 512 two-byte characters
        // (1,024 bytes) are admitted, 513 are refused, on either engine.
        ["host", "database", "user"].into_iter().flat_map(|name| {
            Engine::ALL.into_iter().flat_map(move |engine| {
                [(512, true), (513, false)].map(|(count, admitted)| {
                    let mut changes = json!({"engine": engine.name()});
                    changes[name] = json!("\u{e9}".repeat(count));
                    (changes, admitted)
                })
            })
        }),
    ) {
        let document = configuration(changes);
        let verdict = conforms(&model, &document);
        assert_eq!(
            verdict.is_ok(),
            admitted,
            "model on {document}: {verdict:?}"
        );
        assert_eq!(
            bootstrap(&directory, &document).is_some(),
            admitted,
            "executable on {document}"
        );
    }
}

#[test]
fn each_engine_advertises_the_profile_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "connection.PasswordProfile");
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    connectors_host::local::filesystem::directory(&directory, true, true).unwrap();
    for engine in Engine::ALL {
        let bootstrap =
            bootstrap(&directory, &configuration(json!({"engine": engine.name()}))).unwrap();
        assert_eq!(bootstrap.profiles.len(), 1);
        let profile = serde_json::to_value(&bootstrap.profiles[0]).unwrap();
        let (scheme, _) = bootstrap.provider_authority.split_once("://").unwrap();
        let document = json!({
            "engine": engine.name(),
            "id": profile["id"],
            "label": profile["fields"][0]["label"],
            "scheme": profile["scheme"],
            "capability": profile["capability"],
            "purpose": profile["purpose"],
            "subject": profile["subject"],
            "minimum_scopes": profile["minimum_scopes"],
            "evidence_lifetime_ms": profile["evidence_lifetime_ms"],
            "identity_kind": engine.identity_kind(),
            "authority_scheme": scheme,
        });
        conforms(&model, &document).unwrap_or_else(|e| panic!("{engine:?}: {e}"));
        // The model refuses the other engine's profile on this engine.
        let mut crossed = document.clone();
        crossed["id"] = json!(match engine {
            Engine::Postgresql => "mysql.password",
            Engine::Mysql => "postgres.password",
        });
        assert!(conforms(&model, &crossed).is_err(), "{engine:?}");
    }
}

#[test]
fn the_read_binding_and_the_mysql_cell_rules_match_the_model() {
    let out = tempfile::tempdir().unwrap();
    let binding = schema(out.path(), "reads.ReadBinding");
    for engine in Engine::ALL {
        let mut document = json!({
            "engine": engine.name(),
            "profile": engine.result_profile(),
            "session_statements": engine.session_statements(),
            "minimum_columns": 1,
            "maximum_columns": MAX_COLUMNS,
        });
        // The session time zone is the one a session statement actually sets.
        let zones = engine
            .session_statements()
            .iter()
            .filter_map(|statement| {
                let (_, rest) = statement.split_once("time_zone = '")?;
                Some(rest.split_once('\'')?.0.to_owned())
            })
            .collect::<Vec<_>>();
        assert!(zones.len() <= 1, "{engine:?}: {zones:?}");
        if let Some(zone) = zones.first() {
            document["session_time_zone"] = json!(zone);
        }
        conforms(&binding, &document).unwrap_or_else(|e| panic!("{engine:?}: {e}"));
        // A MySQL session left in the server's time zone is refused.
        if engine == Engine::Mysql {
            let mut system = document.clone();
            system["session_time_zone"] = json!("SYSTEM");
            assert!(conforms(&binding, &system).is_err());
            system.as_object_mut().unwrap().remove("session_time_zone");
            assert!(conforms(&binding, &system).is_err());
        }
    }
    let rule = schema(out.path(), "reads.MysqlCellRule");
    let families = rule["$defs"]["connectors_sql.reads.MysqlTypeFamily"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    // Every family the model names has exactly one rule here, and back.
    assert_eq!(families, Family::ALL.map(|f| f.name().to_owned()).to_vec());
    for family in Family::ALL {
        let document = json!({"family": family.name(), "rendering": family.rendering().name()});
        conforms(&rule, &document).unwrap_or_else(|e| panic!("{family:?}: {e}"));
        let other = if family.rendering() == Rendering::Base64 {
            Rendering::Utf8Text
        } else {
            Rendering::Base64
        };
        let wrong = json!({"family": family.name(), "rendering": other.name()});
        assert!(conforms(&rule, &wrong).is_err(), "{family:?}");
    }
}

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(fixture::PASSWORD.as_bytes().to_vec()))
    }
}

fn sql(engine: Engine, port: u16) -> Sql {
    let config = Config {
        engine,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"fixture-sql","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new("fixture-sql", config, effective, Arc::new(Password)).unwrap()
}

/// One `schema.list` call as the model's `SchemaListCall`, read off the wire:
/// the schema the server was asked for is the parameter bound to its
/// `information_schema` read, and the provenance is the answer's own.
async fn schema_list_call(engine: Engine, requested: Option<&str>) -> Value {
    let text = || fixture::col("c", fixture::VAR_STRING, 0, fixture::UTF8MB4, 0);
    let server = fixture::start(fixture::Script {
        columns: (0..7).map(|_| text()).collect(),
        rows: vec![],
        params: 1,
        ..fixture::Script::default()
    })
    .await;
    // PostgreSQL never reaches the MySQL fixture: its refusal must come
    // before any connection, which a closed port would otherwise turn into
    // `unavailable`.
    let port = match engine {
        Engine::Mysql => server.port,
        Engine::Postgresql => 1,
    };
    let mut input = json!({"limit": 10});
    if let Some(schema) = requested {
        input["schema"] = json!(schema);
    }
    let answer = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql(engine, port).invoke("schema.list", input),
    )
    .await
    .expect("schema.list hung");
    let log = server.log.lock().unwrap();
    let mut call = json!({
        "engine": engine.name(),
        "connected_database": "fixture",
        "outcome": match &answer {
            Ok(_) => "read",
            Err(error) if error.code == ErrorCode::InvalidInput => "invalid_input",
            Err(error) => panic!("{engine:?} {requested:?}: {error:?}"),
        },
    });
    if let Some(schema) = requested {
        call["requested_schema"] = json!(schema);
    }
    if let [bound] = log.executions.as_slice() {
        call["schema_read"] = json!(bound[0]);
    }
    if let Ok(result) = &answer {
        call["provenance_resource"] = result["provenance"]["resource"].clone();
    }
    if answer.is_err() {
        assert_eq!(
            log.sessions, 0,
            "{engine:?} {requested:?}: a session was opened"
        );
    }
    json!({ "call": call })
}

#[tokio::test]
async fn schema_list_reads_only_what_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "reads.SchemaListScope");
    for (engine, requested, outcome) in [
        (Engine::Mysql, None, "read"),
        (Engine::Mysql, Some("fixture"), "read"),
        (Engine::Mysql, Some("other"), "invalid_input"),
        (Engine::Postgresql, None, "invalid_input"),
    ] {
        let document = schema_list_call(engine, requested).await;
        assert_eq!(document["call"]["outcome"], outcome, "{document}");
        conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
    }
    // The model refuses the reads it rules out: another database read on
    // MySQL under the connected one's name, and a PostgreSQL call without a
    // schema that is answered.
    for wrong in [
        json!({"engine":"mysql","connected_database":"fixture","requested_schema":"other",
            "outcome":"read","schema_read":"other","provenance_resource":"fixture"}),
        json!({"engine":"mysql","connected_database":"fixture","requested_schema":"other",
            "outcome":"read","schema_read":"other","provenance_resource":"other"}),
        json!({"engine":"mysql","connected_database":"fixture",
            "outcome":"invalid_input"}),
        json!({"engine":"postgresql","connected_database":"fixture",
            "outcome":"read","schema_read":"public","provenance_resource":"fixture"}),
        json!({"engine":"mysql","connected_database":"fixture","requested_schema":"other",
            "outcome":"invalid_input","schema_read":"other"}),
    ] {
        let document = json!({ "call": wrong });
        assert!(conforms(&model, &document).is_err(), "{document}");
    }
}

/// The answer a scripted MySQL server gives a catalogue read: its columns,
/// one row (or none) and the number of parameters its statement takes.
fn catalogue_script(read: &str, answered: bool) -> fixture::Script {
    use fixture::*;
    let (columns, row, params) = match read {
        "database.list" => (database_columns(), vec![text("fixture")], 0),
        "table.list" => (
            table_columns(),
            vec![text("incidents"), text("table"), uint8(1200)],
            1,
        ),
        "table.describe" => (
            describe_columns(),
            vec![
                text("team_id"),
                text("int"),
                text("YES"),
                text("0"),
                int4(2),
                None,
                text("incidents_team_fk"),
                text("fixture"),
                text("teams"),
                text("id"),
            ],
            2,
        ),
        "index.list" => (
            index_columns(),
            vec![
                text("PRIMARY"),
                text("incidents"),
                int4(1),
                text("id"),
                text("YES"),
                text("YES"),
            ],
            2,
        ),
        other => panic!("{other}"),
    };
    Script {
        columns,
        rows: if answered { vec![row] } else { vec![] },
        params,
        ..Script::default()
    }
}

/// One catalogue call as the model's `CatalogCall`, read off the wire: the
/// schema and table the server was asked for are the parameters bound to
/// the statement, and the provenance is the answer's own.
async fn catalogue_call(
    engine: Engine,
    read: &str,
    schema: Option<&str>,
    table: Option<&str>,
    answered: bool,
) -> Value {
    catalogue_call_with(
        engine,
        read,
        schema,
        table,
        catalogue_script(read, answered),
    )
    .await
}

async fn catalogue_call_with(
    engine: Engine,
    read: &str,
    schema: Option<&str>,
    table: Option<&str>,
    script: fixture::Script,
) -> Value {
    let server = fixture::start(script).await;
    // PostgreSQL never reaches the MySQL fixture: its refusals come before
    // any connection, which a closed port would otherwise turn into
    // `unavailable`.
    let port = match engine {
        Engine::Mysql => server.port,
        Engine::Postgresql => 1,
    };
    let mut input = json!({"limit": 10});
    if let Some(schema) = schema {
        input["schema"] = json!(schema);
    }
    if let Some(table) = table {
        input["table"] = json!(table);
    }
    let answer = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql(engine, port).invoke(read, input),
    )
    .await
    .expect("catalogue read hung");
    let log = server.log.lock().unwrap();
    let mut call = json!({
        "read": read.replace('.', "_"),
        "engine": engine.name(),
        "connected_database": "fixture",
        "outcome": match &answer {
            Ok(_) => "read",
            Err(error) if error.code == ErrorCode::InvalidInput => "invalid_input",
            Err(error) if error.code == ErrorCode::NotFound => "not_found",
            Err(error) => panic!("{engine:?} {read} {schema:?} {table:?}: {error:?}"),
        },
    });
    if let Some(schema) = schema {
        call["requested_schema"] = json!(schema);
    }
    if let Some(table) = table {
        call["requested_table"] = json!(table);
    }
    if let [bound] = log.executions.as_slice() {
        if let Some(Some(schema)) = bound.first() {
            call["schema_read"] = json!(schema);
        }
        if let Some(Some(table)) = bound.get(1) {
            call["table_read"] = json!(table);
        }
    }
    if let Ok(result) = &answer {
        call["provenance_resource"] = result["provenance"]["resource"].clone();
    }
    if answer
        .as_ref()
        .is_err_and(|e| e.code == ErrorCode::InvalidInput)
    {
        assert_eq!(log.sessions, 0, "{engine:?} {read}: a session was opened");
    }
    if engine == Engine::Mysql && table.is_some_and(|t| t.ends_with(' ')) {
        assert_eq!(log.sessions, 0, "{read} {table:?}: a session was opened");
    }
    json!({ "call": call })
}

#[tokio::test]
async fn catalogue_reads_read_only_what_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "reads.CatalogScope");
    for (engine, read, schema, table, answered, outcome) in [
        (Engine::Mysql, "database.list", None, None, true, "read"),
        (Engine::Mysql, "database.list", None, None, false, "read"),
        (Engine::Mysql, "table.list", None, None, true, "read"),
        (
            Engine::Mysql,
            "table.list",
            Some("fixture"),
            None,
            true,
            "read",
        ),
        (
            Engine::Mysql,
            "table.list",
            Some("other"),
            None,
            true,
            "invalid_input",
        ),
        (
            Engine::Mysql,
            "table.describe",
            None,
            Some("incidents"),
            true,
            "read",
        ),
        (
            Engine::Mysql,
            "table.describe",
            Some("fixture"),
            Some("missing"),
            false,
            "not_found",
        ),
        (
            Engine::Mysql,
            "table.describe",
            Some("other"),
            Some("incidents"),
            true,
            "invalid_input",
        ),
        (Engine::Mysql, "index.list", None, None, true, "read"),
        (
            Engine::Mysql,
            "index.list",
            None,
            Some("incidents"),
            true,
            "read",
        ),
        (
            Engine::Mysql,
            "index.list",
            Some("other"),
            Some("incidents"),
            true,
            "invalid_input",
        ),
        (
            Engine::Postgresql,
            "table.list",
            None,
            None,
            true,
            "invalid_input",
        ),
        (
            Engine::Postgresql,
            "table.describe",
            None,
            Some("incidents"),
            true,
            "invalid_input",
        ),
        (
            Engine::Postgresql,
            "index.list",
            None,
            None,
            true,
            "invalid_input",
        ),
    ] {
        let document = catalogue_call(engine, read, schema, table, answered).await;
        assert_eq!(document["call"]["outcome"], outcome, "{document}");
        conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
    }
    // A table name MySQL cannot hold: one ending in a space is `not_found`
    // with nothing bound; one the metadata character set cannot represent is
    // bound, answered with error 3988 and `not_found`.
    for (read, schema, table, refused, outcome) in [
        ("table.describe", None, "incidents ", false, "not_found"),
        (
            "table.describe",
            Some("fixture"),
            "incidents ",
            false,
            "not_found",
        ),
        ("index.list", None, "incidents ", false, "not_found"),
        (
            "index.list",
            Some("other"),
            "incidents ",
            false,
            "invalid_input",
        ),
        ("table.describe", None, "\u{1F600}", true, "not_found"),
        ("index.list", None, "\u{1F600}", true, "not_found"),
    ] {
        let mut script = catalogue_script(read, true);
        if refused {
            script.execute_error = Some((3988, "HY000"));
        }
        let document = catalogue_call_with(Engine::Mysql, read, schema, Some(table), script).await;
        assert_eq!(document["call"]["outcome"], outcome, "{document}");
        conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
    }
    // The model refuses the reads it rules out.
    for wrong in [
        // Another database read on MySQL.
        json!({"read":"table_list","engine":"mysql","connected_database":"fixture","requested_schema":"other",
            "outcome":"read","schema_read":"other","provenance_resource":"fixture"}),
        // A PostgreSQL table read without a schema that is answered.
        json!({"read":"index_list","engine":"postgresql","connected_database":"fixture",
            "outcome":"read","schema_read":"public","provenance_resource":"fixture"}),
        // A database listing that binds a schema.
        json!({"read":"database_list","engine":"postgresql","connected_database":"fixture",
            "outcome":"read","schema_read":"public","provenance_resource":"fixture"}),
        // A described table other than the one asked for.
        json!({"read":"table_describe","engine":"mysql","connected_database":"fixture","requested_table":"a",
            "outcome":"read","schema_read":"fixture","table_read":"b","provenance_resource":"fixture"}),
        // not_found from a listing, and a not_found that carries provenance.
        json!({"read":"table_list","engine":"mysql","connected_database":"fixture",
            "outcome":"not_found","schema_read":"fixture"}),
        json!({"read":"table_describe","engine":"mysql","connected_database":"fixture","requested_table":"a",
            "outcome":"not_found","schema_read":"fixture","table_read":"a","provenance_resource":"fixture"}),
        // A refusal that still bound a statement.
        json!({"read":"table_describe","engine":"mysql","connected_database":"fixture","requested_schema":"other",
            "requested_table":"a","outcome":"invalid_input","schema_read":"other","table_read":"a"}),
        // A MySQL name ending in a space answered with the unpadded table's rows,
        // or bound to a statement before it was refused.
        json!({"read":"table_describe","engine":"mysql","connected_database":"fixture","requested_table":"a ",
            "outcome":"read","schema_read":"fixture","table_read":"a ","provenance_resource":"fixture"}),
        json!({"read":"index_list","engine":"mysql","connected_database":"fixture","requested_table":"a ",
            "outcome":"read","schema_read":"fixture","table_read":"a ","provenance_resource":"fixture"}),
        json!({"read":"index_list","engine":"mysql","connected_database":"fixture","requested_table":"a ",
            "outcome":"not_found","schema_read":"fixture","table_read":"a "}),
        // `not_found` from `index.list` on PostgreSQL, and a PostgreSQL name
        // ending in a space that was not bound.
        json!({"read":"index_list","engine":"postgresql","connected_database":"fixture","requested_schema":"public",
            "requested_table":"a","outcome":"not_found","schema_read":"public","table_read":"a"}),
        json!({"read":"table_describe","engine":"postgresql","connected_database":"fixture","requested_schema":"public",
            "requested_table":"a ","outcome":"not_found"}),
    ] {
        let document = json!({ "call": wrong });
        assert!(conforms(&model, &document).is_err(), "{document}");
    }
}

/// A result row as a document of the model's row type: one member per
/// column, a null cell an absent member.
fn row_document(result: &Value, index: usize) -> Value {
    let mut document = serde_json::Map::new();
    for (column, cell) in result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .zip(result["rows"][index].as_array().unwrap())
    {
        if !cell.is_null() {
            document.insert(column["name"].as_str().unwrap().to_owned(), cell.clone());
        }
    }
    Value::Object(document)
}

#[tokio::test]
async fn catalogue_rows_are_the_model_row_types() {
    let out = tempfile::tempdir().unwrap();
    for (read, input, row_type) in [
        ("database.list", json!({"limit":1}), "reads.DatabaseRow"),
        ("table.list", json!({"limit":1}), "reads.TableRow"),
        (
            "table.describe",
            json!({"table":"incidents","limit":1}),
            "reads.ColumnRow",
        ),
        ("index.list", json!({"limit":1}), "reads.IndexRow"),
    ] {
        let model = schema(out.path(), row_type);
        let server = fixture::start(catalogue_script(read, true)).await;
        let result = sql(Engine::Mysql, server.port)
            .invoke(read, input)
            .await
            .unwrap();
        // The answer's columns are exactly the row type's members.
        let definition = &model["$defs"][format!("connectors_sql.{row_type}")];
        let mut members: Vec<_> = definition["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let mut columns: Vec<_> = result["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_owned())
            .collect();
        members.sort();
        columns.sort();
        assert_eq!(columns, members, "{read}");
        let document = row_document(&result, 0);
        conforms(&model, &document).unwrap_or_else(|e| panic!("{read} {document}: {e}"));
    }
    // The row types refuse what the reads must never answer.
    for (row_type, wrong) in [
        (
            "reads.TableRow",
            json!({"table_name":"v","table_kind":"view","row_estimate":"3"}),
        ),
        (
            "reads.TableRow",
            json!({"table_name":"t","table_kind":"table","row_estimate":"-1"}),
        ),
        (
            "reads.TableRow",
            json!({"table_name":"t","table_kind":"materialized"}),
        ),
        (
            "reads.IndexRow",
            json!({"index_name":"p","table_name":"t","column_position":"1",
            "column_name":"id","is_unique":"NO","is_primary":"YES"}),
        ),
        (
            "reads.ColumnRow",
            json!({"column_name":"c","native_type":"int","is_nullable":"YES",
            "ordinal_position":"1","foreign_key":"fk","referenced_table":"t"}),
        ),
        (
            "reads.ColumnRow",
            json!({"column_name":"c","native_type":"int","is_nullable":"true",
            "ordinal_position":"1"}),
        ),
    ] {
        let model = schema(out.path(), row_type);
        assert!(conforms(&model, &wrong).is_err(), "{row_type} {wrong}");
    }
}

/// The fixture's profile with one change applied: `(pointer, value)`, `ABSENT` removes it.
fn profile_with(changes: &[(&str, Value)]) -> Value {
    let mut profile = calls::profile();
    for (pointer, value) in changes {
        if value == ABSENT {
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            let parent = profile.pointer_mut(parent).unwrap();
            match parent {
                Value::Object(map) => {
                    map.remove(key);
                }
                Value::Array(items) => {
                    items.remove(key.parse().unwrap());
                }
                _ => panic!("{pointer}"),
            }
        } else {
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            match profile.pointer_mut(parent).unwrap() {
                Value::Object(map) => {
                    map.insert(key.to_owned(), value.clone());
                }
                Value::Array(items) => items[key.parse::<usize>().unwrap()] = value.clone(),
                _ => panic!("{pointer}"),
            }
        }
    }
    profile
}

/// `count` distinct text columns, the first of which is the call identifier and ordering.
fn wide_profile(count: usize) -> Value {
    let columns: Vec<Value> = (0..count)
        .map(|i| json!({"name": format!("c{i}"), "type": "text"}))
        .collect();
    json!({"id":"wide","statement":"SELECT 1","parameters":[],"columns":columns,
        "call_identifier":"c0","ordering":"c0"})
}

#[test]
fn the_model_admits_exactly_the_read_profiles_the_executable_admits() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "connection.LocalConfiguration");
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    connectors_host::local::filesystem::directory(&directory, true, true).unwrap();
    let mut second = calls::profile();
    second["id"] = json!("legs-by-call");
    let parameters = |n: usize| Value::Array((0..n).map(|i| json!(format!("p{i}"))).collect());
    let cases: Vec<(Value, bool)> = vec![
        (json!([calls::profile()]), true),
        (json!([calls::profile(), second]), true),
        (json!([]), true),
        (Value::Null, false),
        (json!([calls::profile(), calls::profile()]), false),
        (
            Value::Array(
                (0..32)
                    .map(|i| {
                        let mut p = calls::profile();
                        p["id"] = json!(format!("p{i}"));
                        p
                    })
                    .collect(),
            ),
            true,
        ),
        (
            Value::Array(
                (0..33)
                    .map(|i| {
                        let mut p = calls::profile();
                        p["id"] = json!(format!("p{i}"));
                        p
                    })
                    .collect(),
            ),
            false,
        ),
        (
            json!([profile_with(&[("/call_identifier", json!("absent"))])]),
            false,
        ),
        (
            json!([profile_with(&[("/ordering", json!("absent"))])]),
            false,
        ),
        (
            json!([profile_with(&[("/ordering", json!("caller"))])]),
            false,
        ),
        (
            json!([profile_with(&[("/call_identifier", json!("callee"))])]),
            false,
        ),
        (
            json!([profile_with(&[
                ("/columns/3/masked", json!(false)),
                ("/ordering", json!("caller"))
            ])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/1/masked", json!(true))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/1/masked", json!(false))])]),
            true,
        ),
        (
            json!([profile_with(&[("/columns/0/masked", json!(true))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/3/masked", json!(false))])]),
            true,
        ),
        (
            json!([profile_with(&[("/columns/3/masked", Value::Null)])]),
            false,
        ),
        (
            json!([profile_with(&[("/call_identifier", json!("row_key"))])]),
            true,
        ),
        (
            json!([profile_with(&[("/columns/2/name", json!("call_id"))])]),
            false,
        ),
        (
            json!([profile_with(&[(
                "/parameters",
                json!(["number", "number", "to"])
            )])]),
            false,
        ),
        (json!([profile_with(&[("/parameters", json!([]))])]), true),
        (
            json!([profile_with(&[("/parameters", parameters(63))])]),
            true,
        ),
        (
            json!([profile_with(&[("/parameters", parameters(64))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/2/name", json!("Direction"))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/2/name", json!("d".repeat(64)))])]),
            true,
        ),
        (
            json!([profile_with(&[("/columns/2/name", json!("d".repeat(65)))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/2/name", json!(""))])]),
            false,
        ),
        (
            json!([profile_with(&[("/parameters/0", json!("num-ber"))])]),
            false,
        ),
        (json!([profile_with(&[("/id", json!("Calls"))])]), false),
        (
            json!([profile_with(&[("/id", json!("c".repeat(65)))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/2/type", json!("number"))])]),
            false,
        ),
        (
            json!([profile_with(&[("/columns/2/limit", json!(5))])]),
            false,
        ),
        (json!([profile_with(&[("/limit", json!(5))])]), false),
        (json!([profile_with(&[("/columns", json!([]))])]), false),
        (
            json!([profile_with(&[("/parameters", ABSENT.into())])]),
            false,
        ),
        (
            json!([profile_with(&[("/ordering", ABSENT.into())])]),
            false,
        ),
        (json!([profile_with(&[("/statement", json!(""))])]), false),
        (
            json!([profile_with(&[("/statement", json!("s".repeat(8192)))])]),
            true,
        ),
        (
            json!([profile_with(&[("/statement", json!("s".repeat(8193)))])]),
            false,
        ),
        // The byte bound: 4,096 two-byte characters are 8,192 bytes, one more is over.
        (
            json!([profile_with(&[(
                "/statement",
                json!("\u{e9}".repeat(4096))
            )])]),
            true,
        ),
        (
            json!([profile_with(&[(
                "/statement",
                json!("\u{e9}".repeat(4097))
            )])]),
            false,
        ),
        (json!([wide_profile(256)]), true),
        (json!([wide_profile(257)]), false),
    ];
    for (profiles, admitted) in cases {
        let document = configuration(json!({ "read_profiles": profiles }));
        let verdict = conforms(&model, &document);
        assert_eq!(
            verdict.is_ok(),
            admitted,
            "model on {profiles}: {verdict:?}"
        );
        assert_eq!(
            bootstrap(&directory, &document).is_some(),
            admitted,
            "executable on {profiles}"
        );
    }
}

#[test]
fn each_columns_masking_is_the_one_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.ColumnMasking");
    for kind in ["text", "integer", "timestamp", "phone_number"] {
        for masked in [None, Some(true), Some(false)] {
            let mut column = json!({"name": "c", "type": kind});
            if let Some(masked) = masked {
                column["masked"] = json!(masked);
            }
            let column: connectors_sql::ProfileColumn = serde_json::from_value(column).unwrap();
            let effective = column.effective_masked();
            let mut document = json!({"type": kind, "effective": effective});
            if let Some(masked) = masked {
                document["masked"] = json!(masked);
            }
            conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
            document["effective"] = json!(!effective);
            assert!(conforms(&model, &document).is_err(), "{document}");
        }
    }
}

fn profile_sql(port: u16) -> Sql {
    sql(Engine::Mysql, port)
        .with_read_profiles(vec![serde_json::from_value(calls::profile()).unwrap()])
        .unwrap()
}

#[tokio::test]
async fn every_page_is_one_the_model_admits() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.PageScope");
    let expected = calls::expected();
    for (cap, limit) in [
        (None, 2),
        (None, 5),
        (None, 7),
        (None, 100),
        (Some(3), 5),
        (Some(3), 3),
        (Some(1), 2),
    ] {
        let server = fixture::start(calls::script(cap)).await;
        let sql = profile_sql(server.port);
        let mut cursor: Option<String> = None;
        let mut after = i64::MIN;
        loop {
            let page = sql
                .invoke("profile.read", calls::input(cursor.as_deref(), limit))
                .await
                .unwrap();
            // What the server sent: one row past the limit at most, never past its cap.
            let remaining = expected.iter().filter(|r| r.row_key > after).count();
            let received = remaining
                .min(usize::from(limit) + 1)
                .min(cap.unwrap_or(usize::MAX));
            let items = page["items"].as_array().unwrap();
            let document = json!({"page": {
                "limit": limit,
                "rows_received": received,
                "rows_returned": items.len(),
                "next_cursor": !page["next_cursor"].is_null(),
            }});
            conforms(&model, &document)
                .unwrap_or_else(|e| panic!("{cap:?} {limit}: {document}: {e}"));
            if page["complete"] == true {
                break;
            }
            after = items.last().unwrap()["row_key"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            cursor = page["next_cursor"].as_str().map(str::to_owned);
        }
    }
    // The model refuses the pages a read must never answer: a short page that ends the
    // read, a received row without a cursor, more than the limit, and a cursor after
    // nothing.
    for wrong in [
        json!({"limit":5,"rows_received":3,"rows_returned":3,"next_cursor":false}),
        json!({"limit":5,"rows_received":6,"rows_returned":5,"next_cursor":false}),
        json!({"limit":5,"rows_received":6,"rows_returned":6,"next_cursor":true}),
        json!({"limit":5,"rows_received":6,"rows_returned":4,"next_cursor":true}),
        json!({"limit":5,"rows_received":3,"rows_returned":2,"next_cursor":true}),
        json!({"limit":5,"rows_received":0,"rows_returned":0,"next_cursor":true}),
    ] {
        let document = json!({ "page": wrong });
        assert!(conforms(&model, &document).is_err(), "{document}");
    }
}

#[tokio::test]
async fn every_refusal_comes_before_a_session_as_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.ProfileReadCall");
    let server = fixture::start(calls::script(None)).await;
    let sql = profile_sql(server.port);
    let issued = sql
        .invoke("profile.read", calls::input(None, 1))
        .await
        .unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut undeclared = calls::input(None, 5);
    undeclared["parameters"]["extra"] = json!("x");
    let mut unknown = calls::input(None, 5);
    unknown["profile"] = json!("absent");
    let mut foreign = calls::input(Some(&issued), 5);
    foreign["parameters"]["to"] = json!("2026-01-04T00:00:00");
    for (input, outcome) in [
        (calls::input(None, 5), "read"),
        (calls::input(Some(&issued), 5), "read"),
        (undeclared, "invalid_input"),
        (unknown, "invalid_input"),
        (foreign, "stale_cursor"),
        (calls::input(Some("AAAA"), 5), "stale_cursor"),
    ] {
        let before = server.log.lock().unwrap().sessions;
        let answer = sql.invoke("profile.read", input.clone()).await;
        let after = server.log.lock().unwrap().sessions;
        let document = json!({
            "outcome": match &answer {
                Ok(_) => "read",
                Err(e) if e.code == ErrorCode::InvalidInput => "invalid_input",
                Err(e) if e.code == ErrorCode::StaleCursor => "stale_cursor",
                Err(e) => panic!("{input}: {e:?}"),
            },
            "session_opened": after > before,
        });
        assert_eq!(document["outcome"], outcome, "{input}");
        conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
    }
    let wrong = json!({"outcome":"stale_cursor","session_opened":true});
    assert!(conforms(&model, &wrong).is_err());
}

#[test]
fn each_keyset_ordering_is_the_one_the_model_fixes() {
    use connectors_sql::profile::{
        ColumnType, PostgresqlOrderingType, mysql_keyset, postgresql_keyset,
    };
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.KeysetOrdering");
    let mut admitted = 0;
    for kind in ColumnType::ALL {
        let mut documents = Vec::new();
        for native in PostgresqlOrderingType::ALL {
            let expression = postgresql_keyset(kind, native);
            documents.push((
                json!({"engine": "postgresql", "column_type": kind.name(),
                    "postgresql_type": native.name()}),
                expression.map(|e| e.name()),
            ));
        }
        for family in Family::ALL {
            let expression = mysql_keyset(kind, family);
            documents.push((
                json!({"engine": "mysql", "column_type": kind.name(),
                    "mysql_family": family.name()}),
                expression.map(|e| e.name()),
            ));
        }
        for (mut document, expression) in documents {
            document["admitted"] = json!(expression.is_some());
            if let Some(expression) = expression {
                admitted += 1;
                document["expression"] = json!(expression);
            }
            conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
            // The other verdict, and every other expression, is refused.
            let mut flipped = document.clone();
            flipped["admitted"] = json!(expression.is_none());
            if expression.is_none() {
                flipped["expression"] = json!("native");
            } else {
                flipped.as_object_mut().unwrap().remove("expression");
            }
            assert!(conforms(&model, &flipped).is_err(), "{flipped}");
            if let Some(expression) = expression {
                for other in [
                    "native",
                    "c_collated_text",
                    "exact_decimal",
                    "datetime6",
                    "utf8mb4_bytes",
                ]
                .into_iter()
                .filter(|o| *o != expression)
                {
                    let mut wrong = document.clone();
                    wrong["expression"] = json!(other);
                    assert!(conforms(&model, &wrong).is_err(), "{wrong}");
                }
            }
        }
    }
    // PostgreSQL: 6 text + 6 phone_number + 3 integer + 2 timestamp; MySQL: 1 + 1 + 3 + 2.
    assert_eq!(admitted, 6 + 6 + 3 + 2 + 1 + 1 + 3 + 2);
}

fn decode_token(token: &str) -> Value {
    use base64::Engine as _;
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token)
            .unwrap(),
    )
    .unwrap()
}
fn encode_token(cursor: &Value) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(cursor).unwrap())
}

#[tokio::test]
async fn every_issued_cursor_is_one_the_model_admits() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.Cursor");
    // A plain value (the call table's row key) and values JSON must escape.
    let server = fixture::start(calls::script(None)).await;
    let token = profile_sql(server.port)
        .invoke("profile.read", calls::input(None, 2))
        .await
        .unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut cursors = vec![decode_token(&token)];
    for key in ["\"".repeat(8192), "\u{1}".repeat(8192), "x".repeat(8192)] {
        let server = fixture::start(fixture::Script {
            columns: vec![fixture::col(
                "k",
                fixture::VAR_STRING,
                0,
                fixture::UTF8MB4,
                0,
            )],
            rows: vec![vec![fixture::text(&key)]],
            params: 0,
            ..fixture::Script::default()
        })
        .await;
        let profile = json!({"id": "keys", "statement": "SELECT k FROM fixture_keys",
            "parameters": [], "columns": [{"name": "key", "type": "text"}],
            "call_identifier": "key", "ordering": "key"});
        let token = sql(Engine::Mysql, server.port)
            .with_read_profiles(vec![serde_json::from_value(profile).unwrap()])
            .unwrap()
            .invoke("profile.read", json!({"profile": "keys", "limit": 1}))
            .await
            .unwrap()["next_cursor"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(token.len() <= 16384);
        cursors.push(decode_token(&token));
    }
    assert_eq!(cursors[0]["after"], "12");
    assert!(cursors[1].get("after_base64").is_some(), "{}", cursors[1]);
    assert!(cursors[3].get("after").is_some());
    for cursor in &cursors {
        conforms(&model, cursor).unwrap_or_else(|e| panic!("{cursor}: {e}"));
    }
    let mut both = cursors[0].clone();
    both["after_base64"] = json!("MTI");
    let mut neither = cursors[0].clone();
    neither.as_object_mut().unwrap().remove("after");
    let mut long = cursors[0].clone();
    long["after"] = json!("1".repeat(8193));
    let mut context = cursors[0].clone();
    context["context"] = json!("abc");
    for wrong in [both, neither, long, context] {
        assert!(conforms(&model, &wrong).is_err(), "{wrong}");
    }
}

#[tokio::test]
async fn each_cursor_admission_is_the_one_the_model_fixes() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.CursorAdmission");
    let server = fixture::start(calls::script(None)).await;
    let sql = profile_sql(server.port);
    let token = sql
        .invoke("profile.read", calls::input(None, 2))
        .await
        .unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let issued = decode_token(&token);
    let with = |key: &str, value: Value| {
        let mut cursor = issued.clone();
        cursor[key] = value;
        encode_token(&cursor)
    };
    let mut other_values = calls::input(None, 2);
    other_values["parameters"]["from"] = json!("2026-01-01T00:00:00");
    // (token, input, decodes, same_profile, same_context, value_admitted)
    let cases = [
        (token.clone(), calls::input(None, 2), true, true, true, true),
        (
            "AAAA".to_owned(),
            calls::input(None, 2),
            false,
            false,
            false,
            false,
        ),
        (
            with("after", json!(7)),
            calls::input(None, 2),
            false,
            true,
            true,
            false,
        ),
        (
            with("profile", json!("other")),
            calls::input(None, 2),
            true,
            false,
            false,
            true,
        ),
        (token.clone(), other_values, true, true, false, true),
        (
            with("after", json!("x12")),
            calls::input(None, 2),
            true,
            true,
            true,
            false,
        ),
        (
            with("context", json!("0".repeat(64))),
            calls::input(None, 2),
            true,
            true,
            false,
            true,
        ),
    ];
    for (token, mut input, decodes, same_profile, same_context, value_admitted) in cases {
        input["cursor"] = json!(token);
        let before = server.log.lock().unwrap().sessions;
        let answer = sql.invoke("profile.read", input.clone()).await;
        assert_eq!(
            server.log.lock().unwrap().sessions > before,
            answer.is_ok(),
            "{input}"
        );
        let document = json!({
            "decodes": decodes, "same_profile": same_profile, "same_context": same_context,
            "value_admitted": value_admitted,
            "outcome": match &answer {
                Ok(_) => "read",
                Err(e) if e.code == ErrorCode::StaleCursor => "stale_cursor",
                Err(e) => panic!("{input}: {e:?}"),
            },
        });
        conforms(&model, &document).unwrap_or_else(|e| panic!("{document}: {e}"));
    }
}

#[tokio::test]
async fn every_page_boundary_is_one_the_model_admits() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "profiles.PageBoundary");
    // Row keys 11, 12, 12, 14…: the boundary after the first, the second and the third row.
    let mut rows = calls::ROWS;
    rows[2].row_key = rows[1].row_key;
    for (limit, past, shares) in [
        (1, true, false),
        (2, true, true),
        (3, true, false),
        (100, false, false),
    ] {
        let server = fixture::start(fixture::Script {
            rows: rows
                .iter()
                .map(|r| {
                    let mut cells = calls::script(None).rows[0].clone();
                    cells[0] = fixture::int8(r.row_key);
                    cells
                })
                .collect(),
            select: None,
            ..calls::script(None)
        })
        .await;
        let answer = profile_sql(server.port)
            .invoke("profile.read", calls::input(None, limit))
            .await;
        let document = json!({
            "received_past_limit": past,
            "past_limit_row_bounded": true,
            "shares_ordering_value": shares,
            "outcome": match &answer {
                Ok(_) => "page",
                Err(e) if e.code == ErrorCode::Unsupported => "unsupported",
                Err(e) => panic!("{limit}: {e:?}"),
            },
        });
        conforms(&model, &document).unwrap_or_else(|e| panic!("{limit}: {document}: {e}"));
    }
    for wrong in [
        json!({"received_past_limit":true,"past_limit_row_bounded":true,"shares_ordering_value":true,"outcome":"page"}),
        json!({"received_past_limit":true,"past_limit_row_bounded":false,"shares_ordering_value":false,"outcome":"page"}),
        json!({"received_past_limit":false,"past_limit_row_bounded":true,"shares_ordering_value":true,"outcome":"unsupported"}),
        json!({"received_past_limit":true,"past_limit_row_bounded":true,"shares_ordering_value":false,"outcome":"unsupported"}),
    ] {
        assert!(conforms(&model, &wrong).is_err(), "{wrong}");
    }
}
