//! The executable against the adapter's own ESS model (`spec/ess`): the schemas
//! the pinned `ess` generates from it, and their invariants, accept the
//! configurations the executable accepts and refuse what it refuses, hold for
//! the profile each engine advertises, and fix the read binding and the MySQL
//! cell rules the library implements.
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

/// Evaluate one rendered `x-ess-invariants` entry of the forms this model uses.
/// An optional field that is absent satisfies its invariant, as in ESS. Any
/// other form panics, so a new invariant cannot pass unchecked.
fn holds(document: &Value, invariant: &str) -> bool {
    let invariant = invariant.trim();
    if let Some(inner) = invariant
        .strip_prefix("not (")
        .and_then(|s| s.strip_suffix(')'))
    {
        return !holds(document, inner);
    }
    if let Some(inner) = invariant
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
    {
        if inner.contains(" or ") {
            return inner.split(" or ").any(|part| holds(document, part));
        }
        return inner.split(" and ").all(|part| holds(document, part));
    }
    if let Some(path) = invariant
        .strip_prefix("defined(")
        .and_then(|s| s.strip_suffix(')'))
    {
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
        let actual = match left.strip_suffix(".count") {
            Some(name) => match field(document, name) {
                // An optional member that is absent or null holds, as in ESS.
                None | Some(Value::Null) => return true,
                Some(Value::String(s)) => json!(s.chars().count()),
                Some(Value::Array(a)) => json!(a.len()),
                Some(other) => panic!("count of {other}"),
            },
            None => match field(document, left) {
                None => return true,
                Some(value) => value.clone(),
            },
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
        if let (Some(reference), Some(value)) = (property["$ref"].as_str(), document.get(key)) {
            check(schema, reference.trim_start_matches("#/$defs/"), value)?;
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
