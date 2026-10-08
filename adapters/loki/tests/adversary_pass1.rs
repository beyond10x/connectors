//! Adversary pass 1, wave 20261008e, story `parity-loki-query`.
//!
//! Cases written against the unit's own documents: the connection model
//! (`spec/ess/domains/connection.yaml`), the contract (§§11.1, 11.4,
//! `contracts/logs/v1alpha1/semantics.md`) and the composition in
//! `src/local.rs`. The TLS and redirect cases build the HTTP port exactly as
//! `Local::load` does (`allow_plaintext: false`, `bearer: true`, the CA bytes
//! replacing the roots) and add the credential as `Local::authenticated` does.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_loki::{Loki, auth};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

const END: &str = "1788825600000000000";
const END_NS: u64 = 1_788_825_600_000_000_000;
const NOW_MS: u64 = 1_788_825_600_000;
const TOKEN: &str = "fixture-adversary-token";

// ---------------------------------------------------------------------------
// The ESS model against the executable, on documents outside the unit's list.

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

/// The invariant forms this model uses; anything else panics.
fn holds(document: &Value, invariant: &str) -> bool {
    let invariant = invariant.trim();
    if let Some(inner) = invariant
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
    {
        return inner.split(" and ").all(|part| holds(document, part));
    }
    if let Some((left, right)) = invariant.split_once(" starts_with ") {
        let prefix: String = serde_json::from_str(right).unwrap();
        return field(document, left)
            .is_none_or(|v| v.as_str().is_some_and(|s| s.starts_with(&prefix)));
    }
    for operator in [" == ", " >= ", " <= "] {
        let Some((left, right)) = invariant.split_once(operator) else {
            continue;
        };
        let expected: Value = serde_json::from_str(right).unwrap();
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

fn check(schema: &Value, name: &str, document: &Value) -> bool {
    let definition = &schema["$defs"][name];
    // ESS 0.56.0 lowers a newtype's `alphabet:` to the `x-ess-alphabet`
    // annotation, not to a JSON Schema `pattern`; it is part of the model.
    let within_alphabet = match (definition["x-ess-alphabet"].as_str(), document.as_str()) {
        (Some(alphabet), Some(text)) => text.chars().all(|c| alphabet.contains(c)),
        (Some(_), None) => false,
        (None, _) => true,
    };
    within_alphabet
        && definition["x-ess-invariants"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|i| holds(document, i.as_str().unwrap()))
        && definition["properties"]
            .as_object()
            .into_iter()
            .flatten()
            .all(
                |(key, property)| match (property["$ref"].as_str(), document.get(key)) {
                    (Some(reference), Some(value)) => {
                        check(schema, reference.trim_start_matches("#/$defs/"), value)
                    }
                    _ => true,
                },
            )
}

fn model_admits(schema: &Value, document: &Value) -> bool {
    jsonschema::validator_for(schema)
        .unwrap()
        .is_valid(document)
        && check(
            schema,
            schema["$ref"]
                .as_str()
                .unwrap()
                .trim_start_matches("#/$defs/"),
            document,
        )
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn executable_admits(directory: &Path, configuration: &Value) -> bool {
    let path = directory.join("loki.json");
    private(&path, &serde_json::to_vec(configuration).unwrap());
    Command::new(env!("CARGO_BIN_EXE_connectors-loki"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
        .status
        .success()
}

/// `ess_model.rs` names its test "the executable admits exactly the
/// configurations the model admits", over a closed list. Three documents off
/// that list split them; none is covered by the model's `ESS-LIMIT` note
/// (which names only ca_file absoluteness/ownership, base_url canonical form
/// without credentials/query/fragment, and the subject).
#[test]
fn the_configuration_model_and_the_executable_agree_off_the_unit_list() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema(root.path(), "LocalConfiguration");
    let base = json!({"format": "connectors-loki-local/1", "instance": "loki-prod",
        "base_url": "https://loki.example/", "query_scope": {"required_equalities": []}});
    let mut spaced = base.clone();
    // connection.yaml:31 bounds only the length; the executable (valid_id,
    // local.rs:54) also refuses anything outside [A-Za-z0-9_.-].
    spaced["instance"] = json!("loki prod");
    let mut upper = base.clone();
    // connection.yaml:33 is a literal prefix; the executable canonicalises the
    // scheme first (local.rs:61-66) and admits it.
    upper["base_url"] = json!("HTTPS://loki.example/");
    let mut null_ca = base.clone();
    // connection.yaml:27 generates `"type": "string"`; local.rs:36 is an
    // Option that reads JSON null as absent.
    null_ca["ca_file"] = Value::Null;
    let mut mismatches = Vec::new();
    for document in [spaced, upper, null_ca] {
        let model = model_admits(&schema, &document);
        let executable = executable_admits(root.path(), &document);
        if model != executable {
            mismatches.push(format!(
                "{document}: model admits {model}, executable admits {executable}"
            ));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// Contract §11.4: the entry is "1–8,192 visible ASCII bytes, no spaces".
/// connection.yaml:43 models only a character count, so the model admits
/// entries the parser (auth.rs:45) refuses.
#[test]
fn the_entry_model_and_the_entry_parser_agree() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema(root.path(), "BearerEntry");
    let mut mismatches = Vec::new();
    for token in ["fixture token", "fixture\ttoken", "fïxture"] {
        let document = json!({ "token": token });
        let model = model_admits(&schema, &document);
        let parser = auth::ProtectedEntry::parse(serde_json::to_vec(&document).unwrap()).is_ok();
        if model != parser {
            mismatches.push(format!(
                "{document}: model admits {model}, parser admits {parser}"
            ));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

// ---------------------------------------------------------------------------
// Bounds at their exact edges (the unit's tests sit far from each edge).

/// Answers each connection with the next scripted body (200) and counts them.
async fn plain_server(answers: Vec<Value>) -> (String, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(AtomicUsize::new(0));
    let count = seen.clone();
    tokio::spawn(async move {
        for answer in answers {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            count.fetch_add(1, Ordering::SeqCst);
            let bytes = serde_json::to_vec(&answer).unwrap();
            let _ = socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = socket.write_all(&bytes).await;
        }
    });
    (format!("http://{address}/"), seen)
}

fn plain_loki(base: &str) -> Loki {
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
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
    Loki::new(
        "l",
        &json!({"format": "connectors-loki-local/1", "instance": "l", "base_url": base,
                "ca_digest": null, "query_scope": {"required_equalities": []}}),
        Arc::new(http),
    )
    .unwrap()
    .with_clock(|| NOW_MS)
}

/// §4.3 / §11: "end − start ≤ 24 h". Exactly 24 h dispatches; one nanosecond
/// more is refused before dispatch. Kills `>` → `>=` at lib.rs:161.
#[tokio::test]
async fn a_window_of_exactly_twenty_four_hours_is_admitted_and_one_nanosecond_more_is_not() {
    let empty = json!({"status": "success", "data": {"resultType": "streams", "result": []}});
    let (base, seen) = plain_server(vec![empty]).await;
    let loki = plain_loki(&base);
    let start = (END_NS - connectors_loki::WINDOW_NS).to_string();
    let out = loki
        .invoke(
            "logs.query_range",
            json!({"query": "{a=\"b\"}", "start_unix_ns": start, "end_unix_ns": END}),
        )
        .await
        .unwrap();
    assert_eq!(out["complete"], true);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    let start = (END_NS - connectors_loki::WINDOW_NS - 1).to_string();
    let error = loki
        .invoke(
            "logs.query_range",
            json!({"query": "{a=\"b\"}", "start_unix_ns": start, "end_unix_ns": END}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
}

/// §11.1: at most 11,000 points, `(end − start) / step + 1`. 10,999 s at a
/// 1 s step is 11,000 points and dispatches; 11,000 s is 11,001 and is refused.
/// Kills dropping `+ 1` or `>` → `>=` at lib.rs:500.
#[tokio::test]
async fn eleven_thousand_points_dispatch_and_eleven_thousand_and_one_do_not() {
    let empty = json!({"status": "success", "data": {"resultType": "matrix", "result": []}});
    let (base, seen) = plain_server(vec![empty]).await;
    let loki = plain_loki(&base);
    let start = (END_NS - 10_999 * 1_000_000_000).to_string();
    let out = loki
        .invoke(
            "logs.query_metric",
            json!({"query": "q", "start_unix_ns": start, "end_unix_ns": END, "step_seconds": 1}),
        )
        .await
        .unwrap();
    assert_eq!(out["complete"], true);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    let start = (END_NS - 11_000 * 1_000_000_000).to_string();
    let error = loki
        .invoke(
            "logs.query_metric",
            json!({"query": "q", "start_unix_ns": start, "end_unix_ns": END, "step_seconds": 1}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
}

// ---------------------------------------------------------------------------
// The credential: the configured CA, and redirects.

struct Token;
#[async_trait::async_trait]
impl Credential for Token {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

struct Identity {
    der: Vec<u8>,
    key: Vec<u8>,
    pem: String,
}
fn identity() -> Identity {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    Identity {
        der: cert.cert.der().to_vec(),
        key: cert.signing_key.serialize_der(),
        pem: cert.cert.pem(),
    }
}

/// A TLS listener answering every request with `answer`; records each request
/// head it received after a completed handshake.
async fn tls_server(server: &Identity, answer: Vec<u8>) -> (u16, Arc<Mutex<Vec<String>>>) {
    let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![server.der.clone().into()],
        PrivatePkcs8KeyDer::from(server.key.clone()).into(),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let record = heads.clone();
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(mut stream) = acceptor.accept(stream).await else {
                continue;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                match stream.read_u8().await {
                    Ok(byte) => head.push(byte),
                    Err(_) => break,
                }
            }
            record
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).into_owned());
            let _ = stream.write_all(&answer).await;
            let _ = stream.shutdown().await;
        }
    });
    (port, heads)
}

/// The port `Local::load` builds, with the credential `Local::authenticated` adds.
fn composed(port: u16, ca_pem: &str) -> ScopedHttp {
    ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: format!("https://localhost:{port}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: false,
            ca_file: None,
        },
        None,
        Some(ca_pem.as_bytes()),
    )
    .unwrap()
    .with_credential(Arc::new(Token))
}

/// A server whose certificate the configured CA did not sign never receives a
/// request, so never the token; the probe is Unavailable, not admitted.
#[tokio::test]
async fn a_server_outside_the_configured_ca_never_receives_the_token() {
    let server = identity();
    let other = identity();
    let (port, heads) = tls_server(
        &server,
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec(),
    )
    .await;
    let http = composed(port, &other.pem);
    assert_eq!(
        auth::validate(&http, || 0).await.err(),
        Some(auth::Failure::Unavailable)
    );
    assert!(
        heads.lock().unwrap().is_empty(),
        "{:?}",
        heads.lock().unwrap()
    );
    // The same port with the right CA does receive it: the case above is not
    // vacuous.
    let http = composed(port, &server.pem);
    assert!(auth::validate(&http, || 0).await.is_ok());
    let heads = heads.lock().unwrap();
    assert_eq!(heads.len(), 1);
    assert!(
        heads[0].contains(&format!("Bearer {TOKEN}")),
        "{}",
        heads[0]
    );
}

/// A redirect from the configured base is never followed: the probe is a
/// protocol failure, a read is Unavailable, and the target never connects.
#[tokio::test]
async fn a_redirect_is_never_followed_with_the_token() {
    let sink = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let sink_port = sink.local_addr().unwrap().port();
    let reached = Arc::new(AtomicUsize::new(0));
    let count = reached.clone();
    tokio::spawn(async move {
        while sink.accept().await.is_ok() {
            count.fetch_add(1, Ordering::SeqCst);
        }
    });
    let server = identity();
    let answer = format!(
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{sink_port}/loki/api/v1/labels\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let (port, heads) = tls_server(&server, answer.into_bytes()).await;
    let http = composed(port, &server.pem);
    assert_eq!(
        auth::validate(&http, || 0).await.err(),
        Some(auth::Failure::InvalidResponse)
    );
    let loki = Loki::new(
        "l",
        &json!({"format": "connectors-loki-local/1", "instance": "l",
                "base_url": format!("https://localhost:{port}/"),
                "ca_digest": null, "query_scope": {"required_equalities": []}}),
        Arc::new(composed(port, &server.pem)),
    )
    .unwrap()
    .with_clock(|| NOW_MS);
    let error = loki.invoke("logs.labels", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert!(!format!("{error:?} {error}").contains(TOKEN));
    assert_eq!(heads.lock().unwrap().len(), 2);
    assert_eq!(reached.load(Ordering::SeqCst), 0, "a redirect was followed");
}
