//! The executable against the adapter's own ESS model (`spec/ess`): the schemas
//! the pinned `ess` generates from it, and their invariants, accept the
//! configurations the executable accepts and refuse what it refuses, hold for
//! the profile each engine advertises, and fix the read binding and the MySQL
//! cell rules the library implements.
use connectors_host::local::runtime::Bootstrap;
use connectors_sql::{
    Engine, MAX_COLUMNS,
    mysql::{Family, Rendering},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

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
    if let Some((left, right)) = invariant.split_once(" starts_with ") {
        let prefix = literal(right);
        return field(document, left).is_none_or(|v| {
            v.as_str()
                .is_some_and(|s| s.starts_with(prefix.as_str().unwrap()))
        });
    }
    for operator in [" == ", " != ", " >= ", " <= "] {
        let Some((left, right)) = invariant.split_once(operator) else {
            continue;
        };
        let expected = literal(right);
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
    ] {
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
        let document = json!({
            "engine": engine.name(),
            "profile": engine.result_profile(),
            "session_statements": engine.session_statements(),
            "minimum_columns": 1,
            "maximum_columns": MAX_COLUMNS,
        });
        conforms(&binding, &document).unwrap_or_else(|e| panic!("{engine:?}: {e}"));
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
