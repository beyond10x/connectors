//! The executable's connection surface against the adapter's own ESS model
//! (`spec/ess/domains/connection.yaml`): the schemas the pinned `ess` generates
//! from it, and their invariants, accept what the executable accepts and
//! advertises, and refuse what it refuses.
use connectors_host::local::runtime::Bootstrap;
use connectors_prometheus::auth;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

/// The schema of one model type, generated now by the pinned `ess`.
fn schema(out: &Path, name: &str) -> Value {
    model(out, &format!("connectors_prometheus.connection.{name}"))
}

/// The schema of one model type by its full name, generated now by the pinned `ess`.
fn model(out: &Path, name: &str) -> Value {
    let path = out.join(format!("schemas/schema/types/{name}.schema.json"));
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
    // A newtype's invariants read `value`, the representation it wraps.
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
    // `alphabet:` reaches the schema only as this annotation, which no JSON
    // Schema validator enforces.
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

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The executable's own verdict on one configuration document.
fn bootstrap(directory: &Path, configuration: &Value) -> Option<Bootstrap> {
    let path = directory.join("prometheus.json");
    private(&path, &serde_json::to_vec(configuration).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-prometheus"))
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
fn the_executable_refuses_what_the_model_refuses_and_more_only_at_the_recorded_limit() {
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
    let base = json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
        "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}});
    let mut with_ca = base.clone();
    with_ca["ca_file"] = json!(ca.to_str().unwrap());
    let mut admitted = vec![base.clone(), with_ca];
    let mut prefixed = base.clone();
    prefixed["base_url"] = json!("https://gateway.example/prometheus-tenant/");
    admitted.push(prefixed);
    for document in &admitted {
        conforms(&schema, document).unwrap_or_else(|e| panic!("{document}: {e}"));
        assert!(bootstrap(root.path(), document).is_some(), "{document}");
    }

    let refused: Vec<Value> = vec![
        // Plaintext: the model requires https.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "http://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        // A nonempty scope needs a PromQL parser that does not exist.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example/",
            "query_scope": {"allowed_matchers": ["job=\"api\""]}}),
        // The scope is explicit, never implied.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example/"}),
        json!({"format": "connectors-prometheus-local/2", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        // No credential is ever a configuration member.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod", "token": "fixture",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        // The instance alphabet and length.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus prod",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prömetheus",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "a".repeat(129),
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        // The literal lowercase scheme, and at most 512 characters.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "HTTPS://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": format!("https://prometheus.example/{}/", "a".repeat(485)),
            "query_scope": {"allowed_matchers": []}}),
        // `ca_file` is absent or a path, never null or empty.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod", "ca_file": null,
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod", "ca_file": "",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
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
    // The 512-character edge is admitted by both.
    let mut longest = base.clone();
    longest["base_url"] = json!(format!("https://prometheus.example/{}/", "a".repeat(484)));
    assert_eq!(longest["base_url"].as_str().unwrap().chars().count(), 512);
    conforms(&schema, &longest).unwrap();
    assert!(bootstrap(root.path(), &longest).is_some());

    // The model's ESS-LIMIT, asserted: on these the model cannot speak, and only
    // the executable refuses. Never the other way round.
    let limit: Vec<Value> = vec![
        // Not the canonical form: the trailing `/` and lowercase host are missing.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://Prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://user@prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example/?tenant=a", "query_scope": {"allowed_matchers": []}}),
        // `ca_file` relative, or absent from disk.
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod", "ca_file": "ca.pem",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
        json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "ca_file": root.path().join("missing.pem").to_str().unwrap(),
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
    ];
    for document in &limit {
        conforms(&schema, document).unwrap_or_else(|e| panic!("{document}: {e}"));
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
        &json!({"format": "connectors-prometheus-local/1", "instance": "prometheus-prod",
            "base_url": "https://prometheus.example/", "query_scope": {"allowed_matchers": []}}),
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
        "entry": "connectors_prometheus.connection.BearerEntry",
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
    let entry = &entry_schema["$defs"]["connectors_prometheus.connection.BearerEntry"];
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

    // The modelled token alphabet is the parser's: visible ASCII, 0x21–0x7E.
    let every_visible: String = (0x21u8..=0x7e).map(char::from).collect();
    for (token, admitted) in [
        (every_visible.as_str(), true),
        ("~", true),
        ("fixture token", false),
        ("fixture\ttoken", false),
        ("fixture\u{7f}", false),
        ("fïxture", false),
        ("fixture\n", false),
    ] {
        let document = json!({ "token": token });
        assert_eq!(
            conforms(&entry_schema, &document).is_ok(),
            admitted,
            "model on {document}"
        );
        assert_eq!(
            auth::ProtectedEntry::parse(serde_json::to_vec(&document).unwrap()).is_ok(),
            admitted,
            "parser on {document}"
        );
    }
}

/// The keys of an object, sorted.
fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

/// A declared property with a local `$ref` resolved against its schema's `$defs`.
fn property<'a>(schema: &'a Value, name: &str) -> &'a Value {
    let property = &schema["properties"][name];
    match property["$ref"].as_str() {
        Some(reference) => &schema["$defs"][reference.trim_start_matches("#/$defs/")],
        None => property,
    }
}

#[test]
fn the_declared_operations_are_the_modelled_selections_and_rule_record() {
    let root = tempfile::tempdir().unwrap();
    let declaration: Value = serde_json::from_slice(
        &fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/spec/adapter.json")).unwrap(),
    )
    .unwrap();
    let operation = |id: &str| {
        declaration["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == id)
            .unwrap_or_else(|| panic!("{id} is declared"))
            .clone()
    };
    for (name, id) in [
        (
            "connectors_prometheus.reads.InstantSelection",
            "series.query",
        ),
        (
            "connectors_prometheus.reads.RangeSelection",
            "series.query_range",
        ),
        ("connectors_prometheus.rules.RulesSelection", "rules.list"),
    ] {
        let schema = model(root.path(), name);
        let modelled = &schema["$defs"][name];
        let declaration = operation(id);
        let declared = &declaration["input_schema"];
        assert_eq!(
            keys(&modelled["properties"]),
            keys(&declared["properties"]),
            "{id}"
        );
        let required = |v: &Value| -> Vec<String> {
            let mut r: Vec<String> = v["required"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|s| s.as_str().unwrap().to_owned())
                .collect();
            r.sort();
            r
        };
        assert_eq!(required(modelled), required(declared), "{id}");
        // Every modelled range is the declared bound: `.count` on text is its
        // length, otherwise the integer range.
        for invariant in modelled["x-ess-invariants"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let invariant = invariant.as_str().unwrap();
            let inner = invariant
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or_else(|| panic!("unrecognised invariant form: {invariant}"));
            let (low, high) = inner.split_once(" and ").unwrap();
            let (field, low) = low.split_once(" >= ").unwrap();
            let high = high.split_once(" <= ").unwrap().1;
            let (field, minimum, maximum) = match field.strip_suffix(".count") {
                Some(field) => (field, "minLength", "maxLength"),
                None => (field, "minimum", "maximum"),
            };
            let bound = property(declared, field);
            assert_eq!(bound[minimum], literal(low), "{id} {field}");
            assert_eq!(bound[maximum], literal(high), "{id} {field}");
        }
    }
    // The rule kind values are the modelled enum's.
    let kinds = model(root.path(), "connectors_prometheus.rules.RuleKind");
    let kinds = &kinds["$defs"]["connectors_prometheus.rules.RuleKind"]["enum"];
    assert_eq!(
        operation("rules.list")["input_schema"]["properties"]["kind"]["enum"],
        *kinds
    );
    // The rule record's members and required members are the modelled Rule's.
    let rule = model(root.path(), "connectors_prometheus.rules.Rule");
    let rule = &rule["$defs"]["connectors_prometheus.rules.Rule"];
    let rules = operation("rules.list");
    let item = &rules["output_schema"]["properties"]["items"]["items"];
    assert_eq!(keys(&rule["properties"]), keys(&item["properties"]));
    let mut modelled: Vec<&Value> = rule["required"].as_array().unwrap().iter().collect();
    let mut declared: Vec<&Value> = item["required"].as_array().unwrap().iter().collect();
    modelled.sort_by_key(|v| v.as_str());
    declared.sort_by_key(|v| v.as_str());
    assert_eq!(modelled, declared);
}
