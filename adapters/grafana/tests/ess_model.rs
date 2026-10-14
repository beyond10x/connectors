//! The executable's connection surface and the records it returns against the adapter's
//! own ESS model (`spec/ess/domains/connection.yaml`, `spec/ess/domains/records.yaml`,
//! `spec/ess/domains/irm.yaml`): the schemas the pinned `ess` generates from it, and their
//! invariants, accept what the executable accepts, advertises and returns, and refuse what
//! it refuses.
#![cfg(unix)]
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

/// An RFC 3339 date-time as seconds (and nanoseconds) since the epoch, so two `Timestamp`
/// facts compare as instants whatever their offsets.
fn instant(text: &str) -> (i64, u32) {
    let number = |range: std::ops::Range<usize>| -> i64 { text[range].parse().unwrap() };
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let (hour, minute, second) = (number(11..13), number(14..16), number(17..19));
    // Days from 1970-01-01 of a proleptic Gregorian date.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let mut rest = &text[19..];
    let mut nanos = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let end = fraction
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(fraction.len());
        nanos = format!("{:0<9}", &fraction[..end]).parse().unwrap();
        rest = &fraction[end..];
    }
    let offset = match rest {
        "Z" | "z" => 0,
        offset => {
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            sign * (offset[1..3].parse::<i64>().unwrap() * 3600
                + offset[4..6].parse::<i64>().unwrap() * 60)
        }
    };
    (
        days * 86_400 + hour * 3600 + minute * 60 + second - offset,
        nanos,
    )
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
    // A fact compared with a sibling fact: two `Timestamp`s, as instants (`until > since`).
    if let Some((left, right)) = invariant.split_once(" > ")
        && right.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
    {
        return match (field(document, left), field(document, right)) {
            (Some(Value::String(l)), Some(Value::String(r))) => instant(l) > instant(r),
            (None, _) | (_, None) => true,
            (l, r) => panic!("{invariant} over {l:?} and {r:?}"),
        };
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
            " >= " => actual.as_f64().unwrap() >= expected.as_f64().unwrap(),
            _ => actual.as_f64().unwrap() <= expected.as_f64().unwrap(),
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
    let mut with_irm = base.clone();
    with_irm["irm_api_url"] = json!("https://oncall-prod.example/oncall/");
    let mut longest_irm = base.clone();
    longest_irm["irm_api_url"] = json!(format!("https://oncall.example/{}/", "a".repeat(488)));
    assert_eq!(
        longest_irm["irm_api_url"].as_str().unwrap().chars().count(),
        512
    );
    // Grafana served under a sub-path keeps that prefix.
    let admitted = vec![
        base.clone(),
        with_ca,
        config("https://ops.example/grafana/"),
        with_irm,
        longest_irm,
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
    // `irm_api_url` is absent or an https URL of at most 512 characters, never null.
    for irm in [
        json!(null),
        json!(""),
        json!("http://oncall.example/"),
        json!("HTTPS://oncall.example/"),
        json!(format!("https://oncall.example/{}/", "a".repeat(489))),
    ] {
        let mut document = base.clone();
        document["irm_api_url"] = irm;
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
    // `irm_api_url` not in its canonical form.
    for irm in [
        "https://oncall.example",
        "https://OnCall.example/",
        "https://user@oncall.example/",
        "https://oncall.example/?x=1",
    ] {
        let mut document = base.clone();
        document["irm_api_url"] = json!(irm);
        limit.push(document);
    }
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
    // story:grafana-irm-history-reads: two profiles, in this order, sharing one entry.
    let ids: Vec<&str> = bootstrap.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, [auth::PROFILE_ID, auth::IRM_PROFILE_ID]);
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

    // The IRM profile: the same entry and kind of token, proved by the IRM probe.
    let irm_schema = schema(root.path(), "connection.IrmServiceAccountProfile");
    let irm = serde_json::to_value(&bootstrap.profiles[1]).unwrap();
    assert!(irm.get("acquisition").is_none());
    assert_eq!(irm["fields"], profile["fields"]);
    let irm_modelled = json!({
        "id": irm["id"],
        "scheme": irm["scheme"],
        "capability": irm["capability"],
        "purpose": irm["purpose"],
        "subject": irm["subject"],
        "minimum_scopes": irm["minimum_scopes"],
        "evidence_lifetime_ms": irm["evidence_lifetime_ms"],
        "entry": "connectors_grafana.connection.ServiceAccountEntry",
        "identity": {
            "method": "GET",
            "path": auth::IRM_PROBE_PATH.join("/"),
            "query": auth::IRM_PROBE_QUERY
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&"),
            "header": connectors_grafana::STACK_HEADER,
            "source": "configuration",
            "kind": auth::IDENTITY_KIND,
        },
    });
    conforms(&irm_schema, &irm_modelled).unwrap();
    // Neither profile model admits the other's id or probe.
    let with = |document: &Value, member: &str, value: Value| {
        let mut document = document.clone();
        document[member] = value;
        document
    };
    assert!(conforms(&profile_schema, &with(&modelled, "id", irm["id"].clone())).is_err());
    assert!(
        conforms(
            &irm_schema,
            &with(&irm_modelled, "id", profile["id"].clone())
        )
        .is_err()
    );
    assert!(
        conforms(
            &profile_schema,
            &with(&modelled, "identity", irm_modelled["identity"].clone())
        )
        .is_err()
    );

    // `datasources.list` needs the data-source profile, every IRM read the IRM profile;
    // none needs a scope.
    let required: Vec<(&str, &str)> = bootstrap
        .requirements
        .iter()
        .map(|r| (r.operation.as_str(), r.profile.as_str()))
        .collect();
    assert_eq!(
        required,
        [
            ("datasources.list", auth::PROFILE_ID),
            ("irm.alert_groups.list", auth::IRM_PROFILE_ID),
            ("irm.alert_group.get", auth::IRM_PROFILE_ID),
            ("irm.incidents.list", auth::IRM_PROFILE_ID),
            ("irm.incident.get", auth::IRM_PROFILE_ID),
        ]
    );
    assert!(bootstrap.requirements.iter().all(|r| r.scopes.is_empty()));

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

// ---------------------------------------------------------------------------------------
// story:grafana-irm-history-reads: the IRM records and windows against `irm.yaml`.

/// The library's answer to one IRM read whose provider answers `answer` once. Both the
/// stack and the IRM port reach the one fixture server.
async fn irm_read(
    operation: &str,
    input: Value,
    answer: Vec<u8>,
) -> connectors_core::Result<Value> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(socket.read_u8().await.unwrap());
        }
        let head = String::from_utf8(head).unwrap();
        let length: usize = head
            .lines()
            .filter_map(|l| l.split_once(':'))
            .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
            .map(|(_, v)| v.trim().parse().unwrap())
            .unwrap_or(0);
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        let status = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            answer.len()
        );
        socket.write_all(status.as_bytes()).await.unwrap();
        socket.write_all(&answer).await.unwrap();
    });
    let port = || {
        ScopedHttp::new_with_ca_bytes(
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
        .unwrap()
    };
    let stack = port()
        .with_read_posts(
            &connectors_grafana::INCIDENT_READ_POSTS,
            connectors_grafana::READ_POST_TIMEOUT,
        )
        .unwrap();
    let irm = port()
        .with_fixed_header(
            connectors_grafana::STACK_HEADER,
            &connectors_grafana::stack_header_value(&base),
        )
        .unwrap();
    let effective = json!({"format": "connectors-grafana-local/1", "instance": "g",
        "base_url": base, "ca_digest": null, "irm_api_url": base});
    Grafana::new("g", &effective, Arc::new(stack))
        .unwrap()
        .with_irm(Arc::new(irm))
        .invoke(operation, input)
        .await
}

fn irm_fixture(name: &str) -> Vec<u8> {
    fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn window() -> Value {
    json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"})
}

/// A page as the model reads it: the shared envelope's `provenance` is not modelled, and
/// a null cursor is the model's absent one (ESS-LIMIT in irm.yaml).
fn modelled_page(out: &Value) -> Value {
    let mut page = out.clone();
    let object = page.as_object_mut().unwrap();
    object.remove("provenance");
    if object["next_cursor"].is_null() {
        object.remove("next_cursor");
    }
    page
}

#[tokio::test]
async fn alert_group_records_carry_only_the_modelled_members() {
    let root = tempfile::tempdir().unwrap();
    let record_schema = schema(root.path(), "irm.AlertGroupRecord");
    let page_schema = schema(root.path(), "irm.AlertGroupPage");
    // The recorded pages, and the widest page the binding admits: 100 groups at the
    // model's id and title edges with every optional member present.
    let edge = |i: usize| {
        json!({"id": format!("{i:0>64}"), "integration_id": "C".repeat(64),
            "route_id": "R".repeat(64), "alerts_count": 0, "state": "silenced",
            "created_at": "2026-01-01T00:00:00.123456789+14:00", "alert_group_number": i,
            "resolved_at": "2026-01-01T00:00:01Z", "acknowledged_at": "2026-01-01T00:00:02Z",
            "silenced_at": "2026-01-01T00:00:03Z", "root_alert_group_id": "I1",
            "title": if i == 0 { "\u{2603}".repeat(4096) } else { "t".to_owned() },
            "permalinks": {"web": "https://stack.example/"},
            "last_alert": {"payload": {"message": "fixture-private"}}})
    };
    let widest = serde_json::to_vec(&json!({"count": 100, "next": null,
        "results": (0..100).map(edge).collect::<Vec<_>>()}))
    .unwrap();
    for answer in [
        irm_fixture("alert_groups_page1.json"),
        irm_fixture("alert_groups_page2.json"),
        widest,
    ] {
        let out = irm_read("irm.alert_groups.list", window(), answer)
            .await
            .unwrap();
        for record in out["items"].as_array().unwrap() {
            conforms(&record_schema, record).unwrap_or_else(|e| panic!("{record}: {e}"));
        }
        conforms(&page_schema, &modelled_page(&out)).unwrap_or_else(|e| panic!("page: {e}"));
    }
    // One read by id is the same record.
    let page: Value = serde_json::from_slice(&irm_fixture("alert_groups_page1.json")).unwrap();
    let one = irm_read(
        "irm.alert_group.get",
        json!({"alert_group_id": "I68T24C13IFW1"}),
        serde_json::to_vec(&page["results"][0]).unwrap(),
    )
    .await
    .unwrap();
    conforms(&record_schema, &one["item"]).unwrap();
    // The members the binding never returns are outside the model, as are the values it
    // refuses (tests/protocol.rs: alert_group_projection_keeps_the_modelled_members_and_refuses_others_whole).
    let base = json!({"id": "I1", "integration_id": "C1", "alerts_count": 1,
        "state": "new", "created_at": "2026-01-01T00:00:00Z"});
    conforms(&record_schema, &base).unwrap();
    for (member, value) in [
        ("permalinks", json!({"web": "https://stack.example/"})),
        ("last_alert", json!({"payload": {}})),
        ("dependent_alert_group_ids", json!([])),
        ("resolved_by", json!("U1")),
        ("route_id", json!(null)),
        ("state", json!("firing")),
        ("id", json!("a/b")),
        ("id", json!("I".repeat(65))),
        ("alerts_count", json!(-1)),
        ("title", json!("")),
        ("title", json!("t".repeat(4097))),
    ] {
        let mut record = base.clone();
        record[member] = value;
        assert!(
            conforms(&record_schema, &record).is_err(),
            "model admits {record}"
        );
    }
    let mut over = json!({"items": vec![base.clone(); 101], "complete": true,
        "window": window()});
    assert!(conforms(&page_schema, &over).is_err());
    over["items"] = json!(vec![base; 100]);
    conforms(&page_schema, &over).unwrap();
}

#[tokio::test]
async fn incident_records_and_links_are_the_modelled_ones() {
    let root = tempfile::tempdir().unwrap();
    let record_schema = schema(root.path(), "irm.IncidentRecord");
    let link_schema = schema(root.path(), "irm.IncidentAlertGroupLink");
    let page_schema = schema(root.path(), "irm.IncidentPage");
    for answer in [
        irm_fixture("incidents_page1.json"),
        irm_fixture("incidents_page2.json"),
    ] {
        let out = irm_read("irm.incidents.list", window(), answer)
            .await
            .unwrap();
        for record in out["items"].as_array().unwrap() {
            conforms(&record_schema, record).unwrap_or_else(|e| panic!("{record}: {e}"));
        }
        for link in out["links"].as_array().unwrap() {
            conforms(&link_schema, link).unwrap_or_else(|e| panic!("{link}: {e}"));
        }
        conforms(&page_schema, &modelled_page(&out)).unwrap_or_else(|e| panic!("page: {e}"));
    }
    let one = irm_read(
        "irm.incident.get",
        json!({"incident_id": "incident-101"}),
        irm_fixture("incident.json"),
    )
    .await
    .unwrap();
    conforms(&record_schema, &one["item"]).unwrap();
    let base = json!({"incident_id": "incident-1", "title": "t", "status": "active",
        "is_drill": false, "created_time": "2026-01-01T00:00:00Z",
        "modified_time": "2026-01-01T00:00:00Z"});
    conforms(&record_schema, &base).unwrap();
    for (member, value) in [
        ("labels", json!([])),
        ("summary", json!("s")),
        ("refs", json!([])),
        ("severity", json!("")),
        ("severity", json!("s".repeat(65))),
        ("closed_time", json!(null)),
        ("status", json!("investigating")),
        ("incident_id", json!("a/b")),
        ("title", json!("")),
    ] {
        let mut record = base.clone();
        record[member] = value;
        assert!(
            conforms(&record_schema, &record).is_err(),
            "model admits {record}"
        );
    }
    for link in [
        json!({"incident_id": "incident-1", "alert_group_id": "abc/123"}),
        json!({"incident_id": "incident-1", "alert_group_id": ""}),
        json!({"incident_id": "incident-1", "alert_group_id": "I1", "url": "https://x.example/"}),
    ] {
        assert!(
            conforms(&link_schema, &link).is_err(),
            "model admits {link}"
        );
    }
}

#[tokio::test]
async fn the_window_the_binding_admits_is_inside_the_model() {
    let root = tempfile::tempdir().unwrap();
    let window_schema = schema(root.path(), "irm.Window");
    let page = irm_fixture("alert_groups_page2.json");
    let at = |since: &str, until: &str| json!({"since": since, "until": until});
    // Admitted by both.
    for admitted in [
        window(),
        at("2026-01-01T00:00:00Z", "2026-01-01T00:00:01Z"),
        at("2028-02-29T23:59:59Z", "2028-03-01T00:00:00Z"),
    ] {
        conforms(&window_schema, &admitted).unwrap_or_else(|e| panic!("{admitted}: {e}"));
        irm_read("irm.alert_groups.list", admitted.clone(), page.clone())
            .await
            .unwrap_or_else(|e| panic!("{admitted}: {e:?}"));
    }
    // Refused by both: an empty or reversed window.
    for refused in [
        at("2026-01-02T00:00:00Z", "2026-01-02T00:00:00Z"),
        at("2026-01-02T00:00:01Z", "2026-01-02T00:00:00Z"),
        json!({"since": "2026-01-01T00:00:00Z"}),
    ] {
        assert!(
            conforms(&window_schema, &refused).is_err(),
            "model admits {refused}"
        );
        let error = irm_read("irm.alert_groups.list", refused.clone(), page.clone())
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            connectors_core::ErrorCode::InvalidInput,
            "{refused}"
        );
    }
    // The ESS-LIMIT, asserted: only the binding refuses an offset or a fraction, which
    // the model's `Timestamp` admits. Never the other way round.
    for limit in [
        at("2026-01-01T00:00:00+01:00", "2026-01-02T00:00:00Z"),
        at("2026-01-01T00:00:00.5Z", "2026-01-02T00:00:00Z"),
        at("2026-01-01T00:00:00Z", "2026-01-02T00:00:00-00:00"),
    ] {
        conforms(&window_schema, &limit).unwrap_or_else(|e| panic!("{limit}: {e}"));
        let error = irm_read("irm.alert_groups.list", limit.clone(), page.clone())
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            connectors_core::ErrorCode::InvalidInput,
            "{limit}"
        );
    }
}
