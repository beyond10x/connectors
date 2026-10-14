//! The executable's connection surface against the adapter's own ESS model
//! (`spec/ess/domains/connection.yaml`): the schemas the pinned `ess` generates
//! from it, and their invariants, accept what the executable accepts and
//! advertises, and refuse what it refuses.
#![cfg(unix)]
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

#[path = "support/invariants.rs"]
mod invariants;
use invariants::{conforms, holds};

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
        // The instance alphabet and length.
        json!({"format": "connectors-loki-local/1", "instance": "loki prod",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "lökì",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "a".repeat(129),
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        // The literal lowercase scheme, and at most 512 characters.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "HTTPS://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": format!("https://loki.example/{}/", "a".repeat(491)),
            "query_scope": {"required_equalities": []}}),
        // `ca_file` is absent or a path, never null or empty.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod", "ca_file": null,
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod", "ca_file": "",
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
    // The 512-character edge is admitted by both.
    let mut longest = base.clone();
    longest["base_url"] = json!(format!("https://loki.example/{}/", "a".repeat(490)));
    assert_eq!(longest["base_url"].as_str().unwrap().chars().count(), 512);
    conforms(&schema, &longest).unwrap();
    assert!(bootstrap(root.path(), &longest).is_some());

    // The model's ESS-LIMIT, asserted: on these the model cannot speak, and only
    // the executable refuses. Never the other way round.
    let limit: Vec<Value> = vec![
        // Not the canonical form: the trailing `/` and lowercase host are missing.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://loki.example", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://Loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://user@loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "base_url": "https://loki.example/?tenant=a", "query_scope": {"required_equalities": []}}),
        // `ca_file` relative, or absent from disk.
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod", "ca_file": "ca.pem",
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
        json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
            "ca_file": root.path().join("missing.pem").to_str().unwrap(),
            "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}}),
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

#[test]
fn every_rendered_form_reads_as_ess_means_it() {
    let document = json!({
        "name": "a", "items": [{"k": "x", "n": 1}, {"k": "y", "n": 2}],
        "flag": false, "text": "<pattern>", "pick": "a"
    });
    for (invariant, expected) in [
        ("(items.count >= 1 and items.count <= 2)", true),
        ("items.count == 3", false),
        ("exists i in items: ((i.k == y and i.n == 2))", true),
        ("exists i in items: (i.k == z)", false),
        ("forall i in items: (i.n >= 1)", true),
        ("distinct i in items by i.k", true),
        ("(not (defined(absent)) or absent == none)", true),
        ("not (defined(name))", false),
        ("text contains \"<\"", true),
        ("not (text starts_with \"<\")", false),
        ("pick == name", true),
        ("flag == false", true),
        ("absent.count <= 0", true),
        ("forall i in absent: (i.k == x)", true),
    ] {
        assert_eq!(holds(&document, invariant), expected, "{invariant}");
    }
    let duplicated = json!({"items": [{"k": "x"}, {"k": "x"}]});
    assert!(!holds(&duplicated, "distinct i in items by i.k"));
    assert!(std::panic::catch_unwind(|| holds(&document, "items matches \"x\"")).is_err());
}

fn equal(label: &str, value: &str) -> Value {
    json!({"label": label, "operator": "equal", "value": value})
}

/// The synthetic profiles of tests/profiles.rs, as the configuration file carries them.
fn with_profiles() -> Value {
    json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
        "base_url": "https://loki.example/", "query_scope": {"required_equalities": []},
        "line_profiles": [
            {"name": "fixture-cdr",
             "selector": [equal("service", "edge-proxy"), equal("stream_kind", "cdr")],
             "format": {"kind": "logfmt"},
             "fields": [
                {"name": "direction", "key": "dir", "type": "string", "filterable": true},
                {"name": "caller", "key": "src", "type": "phone_number", "filterable": true},
                {"name": "started_at", "key": "start", "type": "unix_seconds"},
                {"name": "duration_ms", "key": "dur", "type": "integer", "optional": true},
                {"name": "call_id", "key": "callid", "type": "string"}],
             "call_identifier": "call_id"},
            {"name": "fixture-pattern",
             "selector": [equal("service", "edge-proxy")],
             "format": {"kind": "pattern", "value": {"pattern": "fx-cdr <dir> id=<callid>"}},
             "fields": [
                {"name": "direction", "key": "dir", "type": "string", "mask": "redact"},
                {"name": "call_id", "key": "callid", "type": "string", "mask": "none"}],
             "call_identifier": "call_id"}],
        "trace_profiles": [
            {"name": "fixture-by-label", "selector": [equal("env", "fixture")],
             "call_identifier": {"kind": "label", "value": {"label": "call"}},
             "source_label": "service"},
            {"name": "fixture-by-field", "selector": [equal("env", "fixture")],
             "call_identifier": {"kind": "field",
                                 "value": {"key": "callid", "format": {"kind": "logfmt"}}},
             "source_label": "service"}]})
}

#[test]
fn the_profile_model_and_the_executable_admit_the_same_profiles() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema(root.path(), "LocalConfiguration");
    let admitted = with_profiles();
    conforms(&schema, &admitted).unwrap_or_else(|e| panic!("model: {e}"));
    assert!(bootstrap(root.path(), &admitted).is_some());

    let changed = |pointer: &str, change: &dyn Fn(&mut Value)| {
        let mut document = with_profiles();
        change(document.pointer_mut(pointer).unwrap());
        document
    };
    let refused: Vec<(&str, Value)> = vec![
        (
            "an empty list",
            changed("/line_profiles", &|l| *l = json!([])),
        ),
        (
            "a null list",
            changed("/trace_profiles", &|l| *l = Value::Null),
        ),
        (
            "two profiles of one name",
            changed("/trace_profiles/1/name", &|n| {
                *n = json!("fixture-by-label");
            }),
        ),
        (
            "two fields of one name",
            changed("/line_profiles/0/fields/1/name", &|n| {
                *n = json!("direction");
            }),
        ),
        (
            "the call identifier names no field",
            changed("/line_profiles/0", &|p| {
                p["call_identifier"] = json!("absent");
            }),
        ),
        (
            "the call identifier is masked",
            changed("/line_profiles/0/fields/4", &|f| {
                f["mask"] = json!("last_four");
            }),
        ),
        (
            "the call identifier is an integer",
            changed("/line_profiles/0/fields/4", &|f| {
                f["type"] = json!("integer");
            }),
        ),
        (
            "the call identifier is optional",
            changed("/line_profiles/0/fields/4", &|f| {
                f["optional"] = json!(true);
            }),
        ),
        (
            "a masked integer",
            changed("/line_profiles/0/fields/3", &|f| {
                f["mask"] = json!("redact");
            }),
        ),
        (
            "an unknown type",
            changed("/line_profiles/0/fields/3", &|f| {
                f["type"] = json!("float");
            }),
        ),
        (
            "an empty selector",
            changed("/line_profiles/0/selector", &|s| *s = json!([])),
        ),
        (
            "a selector of regexes only",
            changed("/trace_profiles/0/selector", &|s| {
                *s = json!([{"label": "env", "operator": "matches", "value": ".+"}]);
            }),
        ),
        (
            "an empty equality only",
            changed("/trace_profiles/0/selector", &|s| {
                *s = json!([equal("env", "")]);
            }),
        ),
        (
            "a label name starting with a digit",
            changed("/trace_profiles/0", &|p| {
                p["source_label"] = json!("1service");
            }),
        ),
        (
            "an upper-case profile name",
            changed("/line_profiles/0/name", &|n| {
                *n = json!("Fixture-cdr");
            }),
        ),
        (
            "a pattern without a capture",
            changed("/line_profiles/1/format/value", &|v| {
                v["pattern"] = json!("fixed text");
            }),
        ),
        (
            "an unknown call identifier kind",
            changed("/trace_profiles/0/call_identifier", &|c| {
                c["kind"] = json!("header");
            }),
        ),
        (
            "an unknown member",
            changed("/trace_profiles/1", &|p| {
                p["query"] = json!("{env=\"fixture\"}");
            }),
        ),
    ];
    for (why, document) in &refused {
        assert!(conforms(&schema, document).is_err(), "model admits {why}");
        assert!(
            bootstrap(root.path(), document).is_none(),
            "executable admits {why}"
        );
    }

    // The ESS-LIMIT of reads.yaml, asserted: the model cannot speak on these, and only
    // the executable refuses. Never the other way round.
    let limit: Vec<(&str, Value)> = vec![
        (
            "adjacent captures",
            changed("/line_profiles/1/format/value", &|v| {
                v["pattern"] = json!("fx-cdr <dir><callid>");
            }),
        ),
        (
            "a capture named twice",
            changed("/line_profiles/1/format/value", &|v| {
                v["pattern"] = json!("fx-cdr <dir> id=<callid> <dir>");
            }),
        ),
        (
            "a field key that is not a capture",
            changed("/line_profiles/1/fields/0", &|f| {
                f["key"] = json!("absent");
            }),
        ),
        (
            "the call label already selected",
            changed("/trace_profiles/0/selector", &|s| {
                s.as_array_mut().unwrap().push(equal("call", "fixed"));
            }),
        ),
        (
            "the call label as the source label",
            changed("/trace_profiles/0", &|p| {
                p["source_label"] = json!("call");
            }),
        ),
    ];
    for (why, document) in &limit {
        conforms(&schema, document).unwrap_or_else(|e| panic!("{why}: {e}"));
        assert!(
            bootstrap(root.path(), document).is_none(),
            "executable admits {why}"
        );
    }
}

/// The schema of one `connectors_loki.reads` type, generated with the others.
fn reads(out: &Path, name: &str) -> Value {
    schema(out, "LocalConfiguration");
    let path = out.join(format!(
        "schemas/schema/types/connectors_loki.reads.{name}.schema.json"
    ));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn the_profile_read_inputs_the_descriptor_admits_are_the_modelled_ones() {
    let root = tempfile::tempdir().unwrap();
    let descriptor: Value =
        serde_json::from_str(include_str!("../generated/descriptor.json")).unwrap();
    let input = |operation: &str| {
        let declared = descriptor["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == operation)
            .unwrap()["input_schema"]
            .clone();
        jsonschema::validator_for(&declared).unwrap()
    };
    const START: &str = "1788822000000000000";
    const END: &str = "1788825600000000000";
    for (operation, model, documents) in [
        (
            "logs.profile_lines",
            "ProfileLinesSelection",
            vec![
                (
                    json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END}),
                    true,
                ),
                (
                    json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
                        "limit": 1000, "match": {"caller": "+15550100001"}}),
                    true,
                ),
                (
                    json!({"profile": "fixture-cdr", "start_unix_ns": START}),
                    false,
                ),
                (
                    json!({"profile": "Fixture", "start_unix_ns": START, "end_unix_ns": END}),
                    false,
                ),
                (
                    json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
                        "limit": 0}),
                    false,
                ),
                (
                    json!({"profile": "fixture-cdr", "start_unix_ns": START, "end_unix_ns": END,
                        "query": "{}"}),
                    false,
                ),
            ],
        ),
        (
            "logs.call_trace",
            "CallTraceSelection",
            vec![
                (
                    json!({"profile": "fixture-by-label", "call_id": "x\"} or {",
                        "start_unix_ns": START, "end_unix_ns": END, "limit": 1}),
                    true,
                ),
                (
                    json!({"profile": "fixture-by-label", "call_id": "x".repeat(512),
                        "start_unix_ns": START, "end_unix_ns": END}),
                    true,
                ),
                (
                    json!({"profile": "fixture-by-label", "call_id": "x".repeat(513),
                        "start_unix_ns": START, "end_unix_ns": END}),
                    false,
                ),
                (
                    json!({"profile": "fixture-by-label", "call_id": "",
                        "start_unix_ns": START, "end_unix_ns": END}),
                    false,
                ),
                (
                    json!({"profile": "fixture-by-label", "start_unix_ns": START,
                        "end_unix_ns": END}),
                    false,
                ),
                (
                    json!({"profile": "fixture-by-label", "call_id": "x", "start_unix_ns": START,
                        "end_unix_ns": END, "limit": 1001}),
                    false,
                ),
                (
                    json!({"profile": "fixture-by-label", "call_id": "x", "start_unix_ns": START,
                        "end_unix_ns": END, "selector": []}),
                    false,
                ),
            ],
        ),
    ] {
        let modelled = reads(root.path(), model);
        let declared = input(operation);
        for (document, admitted) in documents {
            assert_eq!(
                conforms(&modelled, &document).is_ok(),
                admitted,
                "model, {operation}: {document}"
            );
            assert_eq!(
                declared.is_valid(&document),
                admitted,
                "descriptor, {operation}: {document}"
            );
        }
    }
    // The recorded ESS-LIMIT, asserted: a map key reaches the generated schema as
    // `String`, so only the descriptor refuses a `match` key outside FieldName.
    // Never the other way round.
    for key in ["Caller", "caller-id", ""] {
        let document = json!({"profile": "fixture-cdr", "start_unix_ns": START,
            "end_unix_ns": END, "match": {key: "x"}});
        conforms(&reads(root.path(), "ProfileLinesSelection"), &document)
            .unwrap_or_else(|e| panic!("{document}: {e}"));
        assert!(
            !input("logs.profile_lines").is_valid(&document),
            "{document}"
        );
    }
}
