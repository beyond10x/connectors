//! Adversary cases for catalog binary responses (`connectors_catalog.binary`). Engine cases use a
//! recording port; transport cases run the real `ScopedHttp` against a plain loopback server so
//! framing, compression and header carriage are the transport's, not a fixture's. Every origin,
//! path and byte is synthetic.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Engine, NoHosts, Selection, origin};
use connectors_core::{ErrorCode, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, Credential, HttpResponse, Secret};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MEDIA: &str = "https://media.example.test";
const FILES: &str = "https://files.example.test";
const TOKEN: &str = "fixture-adversary-token";

fn document() -> Vec<u8> {
    let id = json!({"name": "id", "in": "path", "required": true, "schema": {"type": "string"}});
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/v1/files/{id}/content": {"get": {
                "operationId": "getContent",
                "parameters": [id],
                "responses": {"200": {"description": "OK", "content": {"application/octet-stream": {}}}}
            }}
        }
    }))
    .unwrap()
}
fn bundle() -> Bundle {
    let bytes = document();
    let source = ingest("fixture.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "fixture".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}
fn try_engine(selections: &[Value]) -> Result<Engine> {
    let selections: Vec<Selection> = selections
        .iter()
        .cloned()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    Engine::new(&bundle(), "/v1", &selections)
}
fn engine(selections: &[Value]) -> Engine {
    try_engine(selections).unwrap_or_else(|e| panic!("{}", e.message))
}
fn content(max_bytes: u64, hosts: &[&str]) -> Value {
    let mut binary = json!({"max_bytes": max_bytes});
    if !hosts.is_empty() {
        binary["hosts"] = json!(hosts);
    }
    json!({"id": "file.content", "operation_id": "getContent", "effect": "read",
           "response": "binary", "binary": binary})
}
fn download(hosts: &[&str]) -> Value {
    json!({"id": "file.download", "effect": "read", "response": "binary",
           "binary": {"max_bytes": 1024, "hosts": hosts},
           "download": {"path_prefix": "/files-pri/"}})
}

type Call = (Vec<String>, Vec<(String, String)>);
struct Port {
    answers: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Port {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected request"))
    }
}
fn port(answers: Vec<HttpResponse>) -> Arc<Port> {
    Arc::new(Port {
        answers: Mutex::new(answers.into()),
        calls: Mutex::new(Vec::new()),
    })
}
fn answer(status: u16, headers: &[(&str, &str)], body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: body.to_vec(),
    }
}
fn admitted(
    ports: &[(&str, Arc<dyn AuthenticatedHttp>)],
) -> BTreeMap<String, Arc<dyn AuthenticatedHttp>> {
    ports
        .iter()
        .map(|(origin, port)| (origin.to_string(), port.clone()))
        .collect()
}

// ---------------------------------------------------------------- loopback transport

struct Fixed;
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

/// A plain HTTP server answering each connection with the next scripted raw response, then
/// closing; every request head is recorded in lower case.
async fn serve(responses: Vec<Vec<u8>>) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let recorded = heads.clone();
    tokio::spawn(async move {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).await.unwrap_or(0) == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            recorded
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).to_ascii_lowercase());
            let _ = stream.write_all(&response).await;
            let _ = stream.shutdown().await;
        }
    });
    (port, heads)
}
fn scoped(port: u16, credential: bool) -> ScopedHttp {
    ScopedHttp::new(
        &HttpConfig {
            base_url: format!("http://127.0.0.1:{port}/"),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        credential.then(|| Arc::new(Fixed) as Arc<dyn Credential>),
    )
    .unwrap()
}
fn raw(head: &str, body: &[u8]) -> Vec<u8> {
    let mut bytes = head.replace('\n', "\r\n").into_bytes();
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    bytes
}

// ---------------------------------------------------------------- media type vs bytes

/// A body the provider sent with a content coding is not the media type its `Content-Type`
/// names: base64-decoding the result gives gzip bytes, not a PDF. The transport asks for no
/// coding and decodes none, so the result must either refuse the coded body or say what the
/// bytes are; reporting `application/pdf` for gzip bytes breaks "media type matches the bytes".
#[tokio::test]
async fn a_content_coded_body_is_not_reported_as_the_media_type_it_encodes() {
    let gzip: &[u8] = b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03fixture";
    let engine = engine(&[content(1024, &[])]);
    let api = port(vec![answer(
        200,
        &[
            ("content-type", "application/pdf"),
            ("content-encoding", "gzip"),
        ],
        gzip,
    )]);
    let outcome = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F1"}))
        .await;
    if let Ok(value) = &outcome {
        assert_ne!(
            value["body"]["media_type"], "application/pdf",
            "gzip bytes answered as application/pdf: {value}"
        );
    }
}

/// The transport keeps two `Content-Type` lines as one value joined by `, ` (`keep_header`);
/// the model says `media_type` is the answer's Content-Type without its parameters, one type.
#[tokio::test]
async fn a_repeated_content_type_is_not_answered_as_one_media_type() {
    let engine = engine(&[content(1024, &[])]);
    let api = port(vec![answer(
        200,
        &[("content-type", "application/pdf, text/html")],
        b"x",
    )]);
    let outcome = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F1"}))
        .await;
    if let Ok(value) = &outcome {
        let media = value["body"]["media_type"].as_str().unwrap();
        assert!(
            !media.contains(','),
            "two media types answered as one: {media}"
        );
    }
}

// ---------------------------------------------------------------- model vs code

/// `connectors_catalog.binary.Binary` holds `hosts.count >= 1` whenever `hosts` is defined;
/// the engine accepts an explicit empty list.
#[test]
fn an_explicit_empty_hosts_list_is_refused_as_the_model_says() {
    let outcome = try_engine(&[json!({"id": "file.content", "operation_id": "getContent",
        "effect": "read", "response": "binary", "binary": {"max_bytes": 10, "hosts": []}})]);
    assert!(
        outcome.is_err(),
        "binary.hosts [] loaded although the model requires at least one"
    );
}

/// An origin is `https://<host>[:<port>]`; `..`, `a..b` and `.` name no host, yet are accepted
/// as canonical and so may be listed in `binary.hosts` and admitted by a connection.
#[test]
fn an_origin_whose_host_is_no_hostname_is_refused() {
    for text in ["https://..", "https://a..b", "https://.", "https://-"] {
        assert_eq!(origin(text), None, "{text} accepted as an origin");
    }
}

/// The ESS domain names `docs/local-catalog-provider.md`, "Configure the provider", as the
/// normative owner of `response`, `binary`, `download` and `hosts`; the Slack guide documents
/// the bot configuration that should admit `https://files.slack.com` for `file.download`.
#[test]
fn the_owner_document_and_the_slack_guide_state_the_binary_members() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let owner = fs::read_to_string(root.join("docs/local-catalog-provider.md")).unwrap();
    let missing: Vec<&str> = [
        "`binary`",
        "`max_bytes`",
        "`download`",
        "`path_prefix`",
        "`hosts`",
    ]
    .into_iter()
    .filter(|member| !owner.contains(member))
    .collect();
    let slack = fs::read_to_string(root.join("docs/catalog-slack.md")).unwrap();
    assert!(
        missing.is_empty() && slack.contains("file.download") && slack.contains("files.slack.com"),
        "owner lacks {missing:?}; slack guide names file.download: {}, files.slack.com: {}",
        slack.contains("file.download"),
        slack.contains("files.slack.com")
    );
}

// ---------------------------------------------------------------- download path prefix

/// A segment that decodes to `../..` is one segment to the engine, but the port re-encodes the
/// slash as `%2F` and the URL leaves `/files-pri/` on any server that decodes it, carrying the
/// credential the file host is admitted with. The prefix check reads the raw path only.
#[tokio::test]
async fn a_download_segment_with_an_encoded_slash_is_refused() {
    let (files_port, heads) = serve(vec![raw(
        "HTTP/1.1 200 OK\nContent-Type: application/json\nContent-Length: 2\nConnection: close\n",
        b"{}",
    )])
    .await;
    let engine = engine(&[download(&[FILES])]);
    let api = port(vec![]);
    let files: Arc<dyn AuthenticatedHttp> = Arc::new(scoped(files_port, true));
    let outcome = engine
        .read_reaching(
            api.as_ref(),
            &admitted(&[(FILES, files)]),
            "fixture",
            "file.download",
            json!({"url": "https://files.example.test/files-pri/..%2F..%2Fapi%2Fauth.test"}),
        )
        .await;
    let sent = heads.lock().unwrap().clone();
    assert!(
        outcome.is_err(),
        "sent with the credential: {:?}",
        sent.iter()
            .map(|h| h.lines().next().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------- redirects, canonical origins

/// Each `Location` that is not canonically `https://media.example.test` is refused as
/// `forbidden` with nothing sent; the canonical spellings of that origin are followed.
#[tokio::test]
async fn redirect_targets_are_compared_canonically() {
    let engine = engine(&[content(1024, &[MEDIA])]);
    for location in [
        "https://media.example.test./f",
        "http://media.example.test/f",
        "https://reader@media.example.test/f",
        "https://reader:pw@media.example.test/f",
        "//media.example.test/f",
        "https://media.example.test:8443/f",
        "https://m\u{0435}dia.example.test/f",
        "https://xn--mdia-8cd.example.test/f",
        "https://media.example.test/a/../f",
        "https://media.example.test/a/%2e%2E/f",
        "https://media.example.test\\@other.example.test/f",
        "https://other.example.test#@media.example.test/f",
        "https://127.0.0.1/f",
        "/f",
        "f",
        "https:media.example.test/f",
    ] {
        let api = port(vec![answer(302, &[("Location", location)], b"")]);
        let media = port(vec![]);
        let hosts = admitted(&[(MEDIA, media.clone() as Arc<dyn AuthenticatedHttp>)]);
        let error = engine
            .read_reaching(
                api.as_ref(),
                &hosts,
                "fixture",
                "file.content",
                json!({"id": "F1"}),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::Forbidden,
            "{location}: {}",
            error.message
        );
        assert!(
            media.calls.lock().unwrap().is_empty(),
            "{location} was sent"
        );
    }
    for location in [
        "HTTPS://MEDIA.Example.Test/f",
        "https://media.example.test:443/f",
        " https://media.example.test/f ",
    ] {
        let api = port(vec![answer(307, &[("location", location)], b"")]);
        let media = port(vec![answer(200, &[], b"ok")]);
        let hosts = admitted(&[(MEDIA, media.clone() as Arc<dyn AuthenticatedHttp>)]);
        let value = engine
            .read_reaching(
                api.as_ref(),
                &hosts,
                "fixture",
                "file.content",
                json!({"id": "F1"}),
            )
            .await
            .unwrap_or_else(|e| panic!("{location}: {}", e.message));
        assert_eq!(value["body"]["length"], 2, "{location}");
        assert_eq!(media.calls.lock().unwrap()[0].0, ["f"], "{location}");
    }
}

/// Three redirects are followed, the fourth is refused and never sent; a selection without
/// hosts follows none, even to the API base origin.
#[tokio::test]
async fn the_redirect_bound_is_exactly_three() {
    let engine = engine(&[content(1024, &[MEDIA])]);
    let hop = |n: u16| answer(302, &[("location", &format!("{MEDIA}/h{n}"))], b"");
    let api = port(vec![hop(1)]);
    let media = port(vec![hop(2), hop(3), answer(200, &[], b"ok")]);
    let hosts = admitted(&[(MEDIA, media.clone() as Arc<dyn AuthenticatedHttp>)]);
    engine
        .read_reaching(
            api.as_ref(),
            &hosts,
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await
        .unwrap_or_else(|e| panic!("three redirects: {}", e.message));
    let api = port(vec![hop(1)]);
    let media = port(vec![hop(2), hop(3), hop(4)]);
    let hosts = admitted(&[(MEDIA, media.clone() as Arc<dyn AuthenticatedHttp>)]);
    assert!(
        engine
            .read_reaching(
                api.as_ref(),
                &hosts,
                "fixture",
                "file.content",
                json!({"id": "F1"})
            )
            .await
            .is_err()
    );
    assert_eq!(
        media.calls.lock().unwrap().len(),
        3,
        "the fourth hop was sent"
    );

    let plain = engine_without_hosts();
    let api = port(vec![answer(
        301,
        &[("location", "/v1/files/F1/content")],
        b"",
    )]);
    let error = plain
        .read_reaching(
            api.as_ref(),
            &NoHosts,
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(api.calls.lock().unwrap().len(), 1);
}
fn engine_without_hosts() -> Engine {
    engine(&[content(1024, &[])])
}

// ---------------------------------------------------------------- the real transport

/// Through the real transport: a body over `max_bytes` is `capacity` whether it is
/// close-delimited (no `Content-Length`), chunked, or declared shorter than `max_bytes`; a
/// `Content-Length` longer than what arrives is never answered as a short success; a gzip body
/// is not inflated past the bound.
#[tokio::test]
async fn the_bound_holds_on_the_wire_whatever_the_framing() {
    let big = vec![b'x'; 2000];
    let mut chunked = b"7d0\r\n".to_vec();
    chunked.extend_from_slice(&big);
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "close-delimited",
            raw(
                "HTTP/1.1 200 OK\nContent-Type: application/pdf\nConnection: close\n",
                &big,
            ),
        ),
        (
            "chunked",
            raw(
                "HTTP/1.1 200 OK\nContent-Type: application/pdf\nTransfer-Encoding: chunked\nConnection: close\n",
                &chunked,
            ),
        ),
        (
            "transport limit, no length",
            raw(
                "HTTP/1.1 200 OK\nContent-Type: application/pdf\nConnection: close\n",
                &vec![b'y'; 4 * 1024 * 1024 + 1],
            ),
        ),
    ];
    for (name, response) in cases {
        let (api_port, _) = serve(vec![response]).await;
        let engine = engine(&[content(1024, &[])]);
        let error = engine
            .read(
                &scoped(api_port, true),
                "fixture",
                "file.content",
                json!({"id": "F1"}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Capacity, "{name}: {}", error.message);
    }
    // Declared 100, 50 arrive, the connection closes: not a 50-byte success.
    let (api_port, _) = serve(vec![raw(
        "HTTP/1.1 200 OK\nContent-Type: application/pdf\nContent-Length: 100\nConnection: close\n",
        &[b'z'; 50],
    )])
    .await;
    let outcome = engine(&[content(1024, &[])])
        .read(
            &scoped(api_port, true),
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await;
    assert!(outcome.is_err(), "short body answered: {outcome:?}");
    // The transport requests no coding and inflates none: the bytes are what arrived.
    let gzip = b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03fixture".to_vec();
    let (api_port, heads) = serve(vec![raw(
        &format!(
            "HTTP/1.1 200 OK\nContent-Type: application/octet-stream\nContent-Encoding: gzip\nContent-Length: {}\nConnection: close\n",
            gzip.len()
        ),
        &gzip,
    )])
    .await;
    let value = engine(&[content(1024, &[])])
        .read(
            &scoped(api_port, true),
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await
        .unwrap();
    assert_eq!(value["body"]["length"], gzip.len());
    assert!(!heads.lock().unwrap()[0].contains("accept-encoding: gzip"));
}

/// Through the real transport: the API base sends the credential and answers 302 with a
/// cookie; the hop to a host admitted without the credential carries neither the credential
/// header nor the cookie.
#[tokio::test]
async fn a_redirect_hop_carries_no_credential_and_no_cookie() {
    let (media_port, media_heads) = serve(vec![raw(
        "HTTP/1.1 200 OK\nContent-Type: image/png\nContent-Length: 2\nConnection: close\n",
        b"ok",
    )])
    .await;
    let (api_port, api_heads) = serve(vec![raw(
        &format!(
            "HTTP/1.1 302 Found\nLocation: {MEDIA}/file/one?token=t\nSet-Cookie: session=fixture; Path=/\nContent-Length: 0\nConnection: close\n"
        ),
        b"",
    )])
    .await;
    let engine = engine(&[content(1024, &[MEDIA])]);
    let media: Arc<dyn AuthenticatedHttp> = Arc::new(scoped(media_port, false));
    let value = engine
        .read_reaching(
            &scoped(api_port, true),
            &admitted(&[(MEDIA, media)]),
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await
        .unwrap();
    assert_eq!(value["body"]["media_type"], "image/png");
    assert!(api_heads.lock().unwrap()[0].contains("authorization: bearer"));
    let head = media_heads.lock().unwrap()[0].clone();
    assert!(head.starts_with("get /file/one?token=t "), "{head}");
    assert!(!head.contains("authorization"), "credential carried");
    assert!(!head.contains("cookie"), "cookie carried");
    assert!(!head.contains(TOKEN));
}

// ---------------------------------------------------------------- configuration revision

/// The configuration revision changes when `hosts` changes: absent, admitted without the
/// credential, admitted with it.
#[test]
fn the_configuration_revision_follows_hosts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    let private = directory.path().join("private");
    connectors_host::local::filesystem::directory(&private, true, true).unwrap();
    let config = private.join("catalog.json");
    let base = json!({
        "format": "connectors-catalog-local/2",
        "instance": "slack",
        "provider": "slack",
        "bundle_directory": root.join("generated/bundles").canonicalize().unwrap(),
        "api_base": "https://slack.com/api",
        "auth": {
            "profile": "slack.bot",
            "header": "Authorization",
            "bearer": true,
            "label": "Slack bot token",
            "identity": {"path": "auth.test", "kind": "slack.user", "subject_pointer": "/user_id"}
        },
        "operations_file": root.join("providers/slack/operations.json").canonicalize().unwrap(),
    });
    let revision = |hosts: Option<Value>| {
        let mut document = base.clone();
        if let Some(hosts) = hosts {
            document["hosts"] = hosts;
        }
        fs::write(&config, serde_json::to_vec(&document).unwrap()).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap["configuration_revision"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let none = revision(None);
    let empty = revision(Some(json!([])));
    let without = revision(Some(json!([{"origin": "https://files.slack.com"}])));
    let with = revision(Some(
        json!([{"origin": "https://files.slack.com", "credential": true}]),
    ));
    assert_eq!(none, empty, "an empty hosts list is the absent one");
    assert_ne!(none, without);
    assert_ne!(without, with);
    assert_ne!(none, with);
}
