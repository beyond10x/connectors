//! The executable's connection surface and the records it returns against the adapter's
//! own ESS model (`spec/ess/domains/connection.yaml`, `spec/ess/domains/records.yaml`): the
//! schemas the pinned `ess` generates from it, and their invariants, accept what the
//! executable accepts, advertises and returns, and refuse what it refuses.
use connectors_grafana::{Grafana, auth};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::runtime::Bootstrap,
};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The schema of one model type, generated now by the pinned `ess`.
fn schema(out: &Path, name: &str) -> Value {
    let path = out.join(format!(
        "schemas/schema/types/connectors_grafana.{name}.schema.json"
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

/// Evaluate one `x-ess-invariants` entry of the forms this model uses. An optional field
/// that is absent satisfies its invariant, as in ESS. Any other form panics, so a new
/// invariant cannot pass unchecked.
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
    // `alphabet:` reaches the schema only as this annotation, which no JSON Schema
    // validator enforces.
    if let Some(alphabet) = definition["x-ess-alphabet"].as_str() {
        let text = document
            .as_str()
            .ok_or_else(|| format!("{name}: not text"))?;
        if let Some(c) = text.chars().find(|c| !alphabet.contains(*c)) {
            return Err(format!("{name}: {c:?} is outside its alphabet"));
        }
    }
    for (key, property) in definition["properties"].as_object().into_iter().flatten() {
        let Some(value) = document.get(key) else {
            continue;
        };
        if let Some(reference) = property["$ref"].as_str() {
            check(schema, reference.trim_start_matches("#/$defs/"), value)?;
        }
        // A list's members are checked against their own definition.
        if let (Some(reference), Some(members)) =
            (property["items"]["$ref"].as_str(), value.as_array())
        {
            for member in members {
                check(schema, reference.trim_start_matches("#/$defs/"), member)?;
            }
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
    let path = directory.join("grafana.json");
    private(&path, &serde_json::to_vec(configuration).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-grafana"))
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

fn config(base_url: &str) -> Value {
    json!({"format": "connectors-grafana-local/1", "instance": "grafana-prod",
        "base_url": base_url})
}

#[test]
fn the_executable_refuses_what_the_model_refuses_and_more_only_at_the_recorded_limit() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema(root.path(), "connection.LocalConfiguration");
    let ca = root.path().join("ca.pem");
    private(
        &ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    let base = config("https://grafana.example/");
    let mut with_ca = base.clone();
    with_ca["ca_file"] = json!(ca.to_str().unwrap());
    // Grafana served under a sub-path keeps that prefix.
    let admitted = vec![
        base.clone(),
        with_ca,
        config("https://ops.example/grafana/"),
    ];
    for document in &admitted {
        conforms(&schema, document).unwrap_or_else(|e| panic!("{document}: {e}"));
        assert!(bootstrap(root.path(), document).is_some(), "{document}");
    }

    let mut refused: Vec<Value> = vec![
        // Plaintext: the model requires https.
        config("http://grafana.example/"),
        // The literal lowercase scheme, and at most 512 characters.
        config("HTTPS://grafana.example/"),
        config(&format!("https://grafana.example/{}/", "a".repeat(488))),
        json!({"format": "connectors-grafana-local/2", "instance": "grafana-prod",
            "base_url": "https://grafana.example/"}),
        // No credential is ever a configuration member.
        json!({"format": "connectors-grafana-local/1", "instance": "grafana-prod",
            "base_url": "https://grafana.example/", "token": "fixture"}),
        // A Loki-shaped document is not a Grafana one.
        json!({"format": "connectors-grafana-local/1", "instance": "grafana-prod",
            "base_url": "https://grafana.example/", "query_scope": {"required_equalities": []}}),
    ];
    // The instance alphabet and length.
    for instance in ["", "grafana prod", "gräfana", "a".repeat(129).as_str()] {
        let mut document = base.clone();
        document["instance"] = json!(instance);
        refused.push(document);
    }
    // `ca_file` is absent or a path, never null or empty.
    for ca_file in [json!(null), json!("")] {
        let mut document = base.clone();
        document["ca_file"] = ca_file;
        refused.push(document);
    }
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
    let longest = config(&format!("https://grafana.example/{}/", "a".repeat(487)));
    assert_eq!(longest["base_url"].as_str().unwrap().chars().count(), 512);
    conforms(&schema, &longest).unwrap();
    assert!(bootstrap(root.path(), &longest).is_some());

    // The model's ESS-LIMIT, asserted: on these the model cannot speak, and only the
    // executable refuses. Never the other way round.
    let mut limit: Vec<Value> = vec![
        // Not the canonical form: the trailing `/` and lowercase host are missing.
        config("https://grafana.example"),
        config("https://Grafana.example/"),
        config("https://user@grafana.example/"),
        config("https://grafana.example/?orgId=1"),
    ];
    // `ca_file` relative, or absent from disk.
    for ca_file in [
        "ca.pem".to_owned(),
        root.path().join("missing.pem").to_str().unwrap().to_owned(),
    ] {
        let mut document = base.clone();
        document["ca_file"] = json!(ca_file);
        limit.push(document);
    }
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
    let profile_schema = schema(root.path(), "connection.ServiceAccountProfile");
    let entry_schema = schema(root.path(), "connection.ServiceAccountEntry");
    let bootstrap = bootstrap(root.path(), &config("https://grafana.example/")).expect("bootstrap");
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
        "entry": "connectors_grafana.connection.ServiceAccountEntry",
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
    let entry = &entry_schema["$defs"]["connectors_grafana.connection.ServiceAccountEntry"];
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

    // The modelled token alphabet and bound are the parser's.
    let every_visible: String = (0x21u8..=0x7e).map(char::from).collect();
    let longest = "x".repeat(8192);
    let over = "x".repeat(8193);
    for (token, admitted) in [
        (every_visible.as_str(), true),
        (longest.as_str(), true),
        (over.as_str(), false),
        ("", false),
        ("fixture token", false),
        ("fixture\ttoken", false),
        ("fixture\u{7f}", false),
        ("fïxture", false),
    ] {
        let document = json!({ "token": token });
        assert_eq!(
            conforms(&entry_schema, &document).is_ok(),
            admitted,
            "model on {token:?}"
        );
        assert_eq!(
            auth::ProtectedEntry::parse(serde_json::to_vec(&document).unwrap()).is_ok(),
            admitted,
            "parser on {token:?}"
        );
    }
    assert!(auth::ProtectedEntry::parse(br#"{"token":"a","extra":1}"#.to_vec()).is_err());
}

/// The library's answer to one recorded `GET /api/datasources`.
async fn listed(answer: Vec<u8>) -> Value {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(socket.read_u8().await.unwrap());
        }
        let status = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            answer.len()
        );
        socket.write_all(status.as_bytes()).await.unwrap();
        socket.write_all(&answer).await.unwrap();
    });
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.clone(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap();
    let effective = json!({"format": "connectors-grafana-local/1", "instance": "g",
        "base_url": base, "ca_digest": null});
    Grafana::new("g", &effective, Arc::new(http))
        .unwrap()
        .invoke("datasources.list", json!({}))
        .await
        .unwrap()
}

#[tokio::test]
async fn every_returned_record_and_the_list_are_the_modelled_ones() {
    let root = tempfile::tempdir().unwrap();
    let record_schema = schema(root.path(), "records.DatasourceRecord");
    let list_schema = schema(root.path(), "records.DatasourceList");
    let fixture = fs::read(format!(
        "{}/tests/fixtures/datasources.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    // The recorded answer, and the widest one the binding admits: 1,001 sources, each
    // at the model's uid, name and type edges.
    let edge = |i: usize| {
        json!({"uid": format!("{i:0>40}"), "name": "\u{2603}".repeat(190),
            "type": "t".repeat(128), "access": "direct", "isDefault": i == 0,
            "url": "https://private-backend.example/"})
    };
    let widest = serde_json::to_vec(&(0..1001).map(edge).collect::<Vec<_>>()).unwrap();
    for answer in [fixture, widest] {
        let out = listed(answer).await;
        for record in out["items"].as_array().unwrap() {
            conforms(&record_schema, record).unwrap_or_else(|e| panic!("{record}: {e}"));
        }
        let list = json!({"items": out["items"], "complete": out["complete"]});
        conforms(&list_schema, &list).unwrap_or_else(|e| panic!("list: {e}"));
    }
    // One past each edge is outside the model, as the binding refuses it
    // (tests/protocol.rs: an_answer_outside_the_model_is_refused_whole).
    for record in [
        json!({"uid": "u".repeat(41), "name": "n", "type": "t", "access": "proxy", "is_default": false}),
        json!({"uid": "a/b", "name": "n", "type": "t", "access": "proxy", "is_default": false}),
        json!({"uid": "u", "name": "\u{2603}".repeat(191), "type": "t", "access": "proxy", "is_default": false}),
        json!({"uid": "u", "name": "", "type": "t", "access": "proxy", "is_default": false}),
        json!({"uid": "u", "name": "n", "type": "t".repeat(129), "access": "proxy", "is_default": false}),
        json!({"uid": "u", "name": "n", "type": "t", "access": "browser", "is_default": false}),
        json!({"uid": "u", "name": "n", "type": "t", "access": "proxy", "is_default": false, "url": "https://x.example/"}),
    ] {
        assert!(
            conforms(&record_schema, &record).is_err(),
            "model admits {record}"
        );
    }
}
