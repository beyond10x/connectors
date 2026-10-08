//! The executable's connection surface against the adapter's own ESS model
//! (`spec/ess/domains/connection.yaml`): the schemas the pinned `ess` generates
//! from it, and their invariants, accept what the executable accepts and
//! advertises, and refuse what it refuses.
use connectors_host::local::runtime::Bootstrap;
use connectors_loki::auth;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

/// The schema of one model type, generated now by the pinned `ess`.
fn schema(out: &Path, name: &str) -> Value {
    let path = out.join(format!(
        "schemas/schema/types/connectors_loki.connection.{name}.schema.json"
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

fn literal(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("unrecognised literal {text}"))
}

/// Evaluate one `x-ess-invariants` entry of the forms this model uses. An
/// optional field that is absent satisfies its invariant, as in ESS. Any other
/// form panics, so a new invariant cannot pass unchecked.
fn holds(document: &Value, invariant: &str) -> bool {
    let invariant = invariant.trim();
    if let Some(inner) = invariant
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
    {
        return inner.split(" and ").all(|part| holds(document, part));
    }
    if let Some((left, right)) = invariant.split_once(" starts_with ") {
        return field(document, left).is_none_or(|v| {
            v.as_str()
                .is_some_and(|s| s.starts_with(literal(right).as_str().unwrap()))
        });
    }
    for operator in [" == ", " >= ", " <= "] {
        let Some((left, right)) = invariant.split_once(operator) else {
            continue;
        };
        let expected = literal(right);
        let actual = match left.strip_suffix(".count") {
            Some(name) => match field(document, name) {
                None => return true,
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
    for invariant in definition["x-ess-invariants"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let invariant = invariant.as_str().unwrap();
        if !holds(document, invariant) {
            return Err(format!("{name}: {invariant}"));
        }
    }
    for (key, property) in definition["properties"].as_object().into_iter().flatten() {
        if let (Some(reference), Some(value)) = (property["$ref"].as_str(), document.get(key)) {
            check(schema, reference.trim_start_matches("#/$defs/"), value)?;
        }
    }
    Ok(())
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The executable's own verdict on one configuration document.
fn bootstrap(directory: &Path, configuration: &Value) -> Option<Bootstrap> {
    let path = directory.join("loki.json");
    private(&path, &serde_json::to_vec(configuration).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-loki"))
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

#[test]
fn the_executable_admits_exactly_the_configurations_the_model_admits() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema(root.path(), "LocalConfiguration");
    let ca = root.path().join("ca.pem");
    private(
        &ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    let base = json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
        "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}});
    let mut with_ca = base.clone();
    with_ca["ca_file"] = json!(ca.to_str().unwrap());
    let mut admitted = vec![base.clone(), with_ca];
    let mut prefixed = base.clone();
    prefixed["base_url"] = json!("https://gateway.example/loki-tenant/");
    admitted.push(prefixed);
    for document in &admitted {
        conforms(&schema, document).unwrap_or_else(|e| panic!("{document}: {e}"));
        assert!(bootstrap(root.path(), document).is_some(), "{document}");
    }

    let refused: Vec<Value> = vec![
        // Plaintext: the model requires https.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "http://loki.example/", "query_scope": {"required_equalities": []}}),
        // A nonempty scope needs a LogQL parser that does not exist.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://loki.example/",
            "query_scope": {"required_equalities": [{"label": "app", "value": "api"}]}}),
        // The scope is explicit, never implied.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://loki.example/"}),
        json!({"format": "connectors-loki-local/2", "instance": "loki-prod",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        // No credential is ever a configuration member.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod", "token": "fixture",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
    ];
    for document in &refused {
        assert!(
            conforms(&schema, document).is_err(),
            "model admits {document}"
        );
        assert!(
            bootstrap(root.path(), document).is_none(),
            "executable admits {document}"
        );
    }
}

#[test]
fn the_advertised_profile_and_its_identity_probe_are_the_modelled_ones() {
    let root = tempfile::tempdir().unwrap();
    let profile_schema = schema(root.path(), "BearerProfile");
    let entry_schema = schema(root.path(), "BearerEntry");
    let bootstrap = bootstrap(
        root.path(),
        &json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
    )
    .expect("bootstrap");
    assert_eq!(bootstrap.profiles.len(), 1);
    let profile = serde_json::to_value(&bootstrap.profiles[0]).unwrap();
    assert!(profile.get("acquisition").is_none());
    let modelled = json!({
        "id": profile["id"],
        "scheme": profile["scheme"],
        "capability": profile["capability"],
        "purpose": profile["purpose"],
        "subject": profile["subject"],
        "minimum_scopes": profile["minimum_scopes"],
        "evidence_lifetime_ms": profile["evidence_lifetime_ms"],
        "entry": "connectors_loki.connection.BearerEntry",
        "identity": {
            "method": "GET",
            "path": auth::PROBE_PATH.join("/"),
            "source": "configuration",
            "kind": auth::IDENTITY_KIND,
        },
    });
    conforms(&profile_schema, &modelled).unwrap();
    assert_eq!(profile["id"], auth::PROFILE_ID);
    // Every operation needs exactly this profile and no scope.
    assert!(
        bootstrap
            .requirements
            .iter()
            .all(|r| r.profile == auth::PROFILE_ID && r.scopes.is_empty())
    );

    // The entry fields are the modelled entry's members, with its bound.
    let entry = &entry_schema["$defs"]["connectors_loki.connection.BearerEntry"];
    let members: Vec<&str> = entry["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let fields: Vec<&str> = profile["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(fields, members);
    assert_eq!(profile["fields"][0]["max_bytes"], 8192);
    conforms(&entry_schema, &json!({"token": "x".repeat(8192)})).unwrap();
    assert!(conforms(&entry_schema, &json!({"token": "x".repeat(8193)})).is_err());
    assert!(conforms(&entry_schema, &json!({"token": ""})).is_err());
    assert!(auth::ProtectedEntry::parse(br#"{"token":"fixture"}"#.to_vec()).is_ok());
    assert!(
        auth::ProtectedEntry::parse(format!(r#"{{"token":"{}"}}"#, "x".repeat(8193)).into_bytes())
            .is_err()
    );
    assert!(auth::ProtectedEntry::parse(br#"{"token":""}"#.to_vec()).is_err());
    assert!(auth::ProtectedEntry::parse(br#"{"token":"a","extra":1}"#.to_vec()).is_err());
}
