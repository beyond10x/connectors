//! Binary responses through the catalog engine (`connectors_catalog.binary`): a selection whose
//! response is `binary` answers with the body's media type, length, SHA-256 and base64, bounded
//! by its `max_bytes` and refused above it, never truncated; a redirect is followed only to an
//! origin the selection reaches and the connection admits, and refused by name otherwise; a
//! `download` reads a provider-issued URL held to those origins and a path prefix. JSON and text
//! selections are unchanged: they follow no redirect and read their bodies as before. Every
//! origin, path and byte is synthetic; no network.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{BINARY_LIMIT, Effect, Engine, NoHosts, Selection, origin};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

/// A body that is not UTF-8, so a reader that took it as text or JSON would fail.
const BYTES: &[u8] = b"%PDF-1.7\n\xff\xfe\x00\x01 fixture binary body\n";
const MEDIA: &str = "https://media.example.test";
const FILES: &str = "https://files.example.test";
const OTHER: &str = "https://other.example.test";

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
            }},
            "/v1/files/{id}": {"get": {
                "operationId": "getFile",
                "parameters": [id],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }},
            "/v1/files/{id}/log": {"get": {
                "operationId": "getLog",
                "parameters": [id],
                "responses": {"200": {"description": "OK", "content": {"text/plain": {}}}}
            }},
            "/v1/files": {"post": {
                "operationId": "createFile",
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                "responses": {"201": {"description": "Created", "content": {"application/json": {}}}}
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

fn selection(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}
fn engine(selections: &[Value]) -> Engine {
    let selections: Vec<Selection> = selections.iter().cloned().map(selection).collect();
    Engine::new(&bundle(), "/v1", &selections).unwrap_or_else(|e| panic!("{}", e.message))
}
/// The binary content read, reaching `hosts`.
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
/// One origin's port: answers in order, every request recorded.
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
impl Port {
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
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
fn moved(status: u16, location: &str) -> HttpResponse {
    answer(status, &[("location", location)], b"")
}
fn admitted(ports: &[(&str, &Arc<Port>)]) -> BTreeMap<String, Arc<dyn AuthenticatedHttp>> {
    ports
        .iter()
        .map(|(origin, port)| {
            let port: Arc<dyn AuthenticatedHttp> = (*port).clone();
            (origin.to_string(), port)
        })
        .collect()
}
fn hex_sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn calls(path: &[&str], query: &[(&str, &str)]) -> Vec<Call> {
    vec![(
        path.iter().map(|s| s.to_string()).collect(),
        query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )]
}

#[tokio::test]
async fn a_binary_selection_answers_media_type_length_digest_and_the_bytes() {
    let engine = engine(&[content(1024, &[])]);
    let declaration = engine.declarations(&[Effect::Read]).remove(0);
    assert_eq!(
        declaration.output_schema["properties"]["body"]["required"],
        json!(["media_type", "length", "sha256", "content_base64"])
    );
    let api = port(vec![answer(
        200,
        &[("content-type", "Application/PDF; name=\"fixture.pdf\"")],
        BYTES,
    )]);
    let value = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F1"}))
        .await
        .unwrap();
    connectors_sdk::validate(&declaration.output_schema, &value).unwrap();
    assert_eq!(value["status"], 200);
    let body = &value["body"];
    assert_eq!(body["media_type"], "application/pdf");
    assert_eq!(body["length"], BYTES.len());
    assert_eq!(body["sha256"], hex_sha256(BYTES));
    assert_eq!(
        STANDARD
            .decode(body["content_base64"].as_str().unwrap())
            .unwrap(),
        BYTES
    );
    assert_eq!(value["provenance"]["resource"], "/v1/files/{id}/content");
    assert_eq!(api.calls(), calls(&["files", "F1", "content"], &[]));

    // No media type named, and an empty body: still a binary answer.
    let api = port(vec![answer(200, &[], b"")]);
    let value = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F2"}))
        .await
        .unwrap();
    assert_eq!(
        value["body"],
        json!({"media_type": "application/octet-stream", "length": 0,
               "sha256": hex_sha256(b""), "content_base64": ""})
    );
}

#[tokio::test]
async fn a_body_over_max_bytes_is_refused_by_name_never_truncated() {
    let exact = BYTES.len() as u64;
    let engine = engine(&[content(exact - 1, &[])]);
    let api = port(vec![answer(200, &[], BYTES)]);
    let error = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F1"}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);
    assert!(error.message.contains("max_bytes"), "{}", error.message);

    // At exactly the bound the whole body is answered.
    let engine = self::engine(&[content(exact, &[])]);
    let api = port(vec![answer(200, &[], BYTES)]);
    let value = engine
        .read(api.as_ref(), "fixture", "file.content", json!({"id": "F1"}))
        .await
        .unwrap();
    assert_eq!(value["body"]["length"], BYTES.len());
}

#[tokio::test]
async fn a_redirect_is_followed_only_to_an_origin_the_selection_reaches_and_the_connection_admits()
{
    let engine = engine(&[content(1024, &[MEDIA])]);
    for status in [301, 302, 303, 307, 308] {
        let api = port(vec![moved(
            status,
            "https://MEDIA.example.test:443/file/abc%20def/binary?token=t%2B1&client=c+d#fragment",
        )]);
        let media = port(vec![answer(200, &[("content-type", "image/png")], BYTES)]);
        let value = engine
            .read_reaching(
                api.as_ref(),
                &admitted(&[(MEDIA, &media)]),
                "fixture",
                "file.content",
                json!({"id": "F1"}),
            )
            .await
            .unwrap();
        assert_eq!(value["status"], 200, "{status}");
        assert_eq!(value["body"]["media_type"], "image/png");
        assert_eq!(value["body"]["sha256"], hex_sha256(BYTES));
        assert_eq!(api.calls().len(), 1);
        assert_eq!(
            media.calls(),
            calls(
                &["file", "abc def", "binary"],
                &[("token", "t+1"), ("client", "c d")]
            ),
            "{status}"
        );
    }
}

#[tokio::test]
async fn a_redirect_anywhere_else_is_refused_by_name_and_nothing_is_sent_there() {
    let reaching = engine(&[content(1024, &[MEDIA])]);
    let unreaching = engine(&[content(1024, &[])]);
    let cases: [(&Engine, HttpResponse, bool, ErrorCode, &str); 7] = [
        // Admitted by the connection, but not a host the selection reaches.
        (
            &reaching,
            moved(302, "https://other.example.test/file/x"),
            true,
            ErrorCode::Forbidden,
            "`https://other.example.test`, which this read does not reach",
        ),
        // Reached by the selection, but the connection admits no port for it.
        (
            &reaching,
            moved(302, "https://media.example.test/file/x"),
            false,
            ErrorCode::Forbidden,
            "`https://media.example.test`, which the connection does not admit",
        ),
        (
            &reaching,
            moved(302, "http://media.example.test/file/x"),
            true,
            ErrorCode::Forbidden,
            "not an https URL",
        ),
        (
            &reaching,
            moved(302, "https://reader@media.example.test/file/x"),
            true,
            ErrorCode::Forbidden,
            "not an https URL",
        ),
        // A path on the API base: the engine holds no origin for it.
        (
            &reaching,
            moved(302, "/file/x"),
            true,
            ErrorCode::Forbidden,
            "not an https URL",
        ),
        (
            &unreaching,
            moved(302, "https://media.example.test/file/x"),
            true,
            ErrorCode::Forbidden,
            "reaches no other host",
        ),
        (
            &reaching,
            answer(303, &[], b""),
            true,
            ErrorCode::UpstreamProtocol,
            "without a location",
        ),
    ];
    for (engine, first, admit, code, named) in cases {
        let api = port(vec![first]);
        let media = port(vec![answer(200, &[], BYTES)]);
        let other = port(vec![answer(200, &[], BYTES)]);
        let hosts = if admit {
            admitted(&[(MEDIA, &media), (OTHER, &other)])
        } else {
            admitted(&[(OTHER, &other)])
        };
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
        assert_eq!(error.code, code, "{named}: {}", error.message);
        assert!(error.message.contains(named), "{named}: {}", error.message);
        assert!(media.calls().is_empty(), "{named}: sent to the media host");
        assert!(other.calls().is_empty(), "{named}: sent to another host");
    }
}

#[tokio::test]
async fn more_than_three_redirects_are_refused_and_a_path_resolves_on_its_own_origin() {
    let engine = engine(&[content(1024, &[MEDIA])]);
    let api = port(vec![moved(302, "https://media.example.test/hop/1")]);
    let media = port(vec![
        moved(302, "/hop/2"),
        moved(302, "/hop/3"),
        moved(302, "/hop/4"),
    ]);
    let error = engine
        .read_reaching(
            api.as_ref(),
            &admitted(&[(MEDIA, &media)]),
            "fixture",
            "file.content",
            json!({"id": "F1"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
    assert!(
        error.message.contains("more than three"),
        "{}",
        error.message
    );
    let paths: Vec<Vec<String>> = media.calls().into_iter().map(|(path, _)| path).collect();
    assert_eq!(
        paths,
        [["hop", "1"], ["hop", "2"], ["hop", "3"]].map(|p| p.map(String::from).to_vec())
    );
}

#[tokio::test]
async fn an_answer_outside_2xx_after_a_redirect_is_classified_as_any_read() {
    let engine = engine(&[content(1024, &[MEDIA])]);
    for (status, code) in [
        (404, ErrorCode::NotFound),
        (401, ErrorCode::Unauthorized),
        (403, ErrorCode::Forbidden),
        (503, ErrorCode::Unavailable),
    ] {
        let api = port(vec![moved(303, "https://media.example.test/file/x")]);
        let media = port(vec![answer(status, &[], b"{}")]);
        let error = engine
            .read_reaching(
                api.as_ref(),
                &admitted(&[(MEDIA, &media)]),
                "fixture",
                "file.content",
                json!({"id": "F1"}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, code, "{status}");
    }
}

#[tokio::test]
async fn json_and_text_selections_are_unchanged() {
    let engine = engine(&[
        json!({"id": "file.get", "operation_id": "getFile", "effect": "read"}),
        json!({"id": "file.log", "operation_id": "getLog", "effect": "read"}),
        content(1024, &[MEDIA]),
    ]);
    for declaration in engine.declarations(&[Effect::Read]) {
        if declaration.id != "file.content" {
            assert_eq!(declaration.output_schema["properties"]["body"], json!({}));
        }
    }
    let api = port(vec![answer(200, &[], br#"{"id":"F1","name":"fixture"}"#)]);
    let value = engine
        .read(api.as_ref(), "fixture", "file.get", json!({"id": "F1"}))
        .await
        .unwrap();
    assert_eq!(value["body"], json!({"id": "F1", "name": "fixture"}));
    let api = port(vec![answer(200, &[], b"line one\nline two\n")]);
    let value = engine
        .read(api.as_ref(), "fixture", "file.log", json!({"id": "F1"}))
        .await
        .unwrap();
    assert_eq!(value["body"], "line one\nline two\n");
    // A JSON or text read follows no redirect, even to a host a binary read of
    // the same connection reaches and the connection admits.
    for id in ["file.get", "file.log"] {
        let api = port(vec![moved(303, "https://media.example.test/file/x")]);
        let media = port(vec![answer(200, &[], BYTES)]);
        let error = engine
            .read_reaching(
                api.as_ref(),
                &admitted(&[(MEDIA, &media)]),
                "fixture",
                id,
                json!({"id": "F1"}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UpstreamProtocol, "{id}");
        assert!(media.calls().is_empty(), "{id}");
    }
    // A body a JSON read cannot read is still refused, never answered as bytes.
    let api = port(vec![answer(200, &[], BYTES)]);
    let error = engine
        .read(api.as_ref(), "fixture", "file.get", json!({"id": "F1"}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
}

#[test]
fn binary_members_are_refused_when_the_selection_loads() {
    let five: Vec<String> = (1..=5)
        .map(|n| format!("https://host{n}.example.test"))
        .collect();
    let with = |change: &dyn Fn(&mut Value)| {
        let mut value = content(1024, &[MEDIA]);
        change(&mut value);
        value
    };
    let refused = [
        with(&|v| {
            v.as_object_mut().unwrap().remove("binary");
        }),
        with(&|v| {
            v.as_object_mut().unwrap().remove("response");
        }),
        with(&|v| v["response"] = json!("text")),
        with(&|v| v["response"] = json!("json")),
        json!({"id": "file.create", "operation_id": "createFile", "effect": "write",
               "response": "binary", "binary": {"max_bytes": 1024}}),
        with(&|v| v["binary"]["max_bytes"] = json!(0)),
        with(&|v| v["binary"]["max_bytes"] = json!(BINARY_LIMIT + 1)),
        with(&|v| v["binary"]["hosts"] = json!(["https://Media.example.test"])),
        with(&|v| v["binary"]["hosts"] = json!(["http://media.example.test"])),
        with(&|v| v["binary"]["hosts"] = json!(["https://media.example.test/"])),
        with(&|v| v["binary"]["hosts"] = json!(["https://media.example.test:443"])),
        with(&|v| v["binary"]["hosts"] = json!([MEDIA, MEDIA])),
        with(&|v| v["binary"]["hosts"] = json!(five)),
        // A download names no operation, reaches a host, and binds nothing.
        {
            let mut v = download(&[FILES]);
            v["operation_id"] = json!("getContent");
            v
        },
        {
            let mut v = download(&[FILES]);
            v["binary"].as_object_mut().unwrap().remove("hosts");
            v
        },
        {
            let mut v = download(&[FILES]);
            v["withhold"] = json!(["token"]);
            v
        },
        {
            let mut v = download(&[FILES]);
            v["effect"] = json!("write");
            v
        },
        // Neither an operation nor a download.
        json!({"id": "nothing", "effect": "read"}),
    ];
    for prefix in ["files-pri/", "/files-pri", "/", "/a/../b/", "/a?b/"] {
        let mut v = download(&[FILES]);
        v["download"]["path_prefix"] = json!(prefix);
        let selection = selection(v);
        assert!(
            Engine::new(&bundle(), "/v1", &[selection]).is_err(),
            "path_prefix {prefix:?} loaded"
        );
    }
    for value in refused {
        let id = value["id"].as_str().unwrap().to_owned();
        let parsed: std::result::Result<Selection, _> = serde_json::from_value(value.clone());
        // A member the reader itself refuses is refused as surely as the engine would.
        let Ok(selection) = parsed else { continue };
        match Engine::new(&bundle(), "/v1", &[selection]) {
            Ok(_) => panic!("loaded: {value}"),
            Err(error) => assert!(
                error.message.contains(&format!("`{id}`")),
                "{}",
                error.message
            ),
        }
    }
    // The bound's ceiling, and an origin with its own port, load.
    engine(&[with(&|v| v["binary"]["max_bytes"] = json!(BINARY_LIMIT))]);
    engine(&[with(&|v| {
        v["binary"]["hosts"] = json!(["https://localhost:8443"])
    })]);
}

#[tokio::test]
async fn a_download_reads_a_url_on_a_reached_admitted_origin_under_its_prefix() {
    let engine = engine(&[download(&[FILES])]);
    assert_eq!(engine.effect("file.download"), Some(Effect::Read));
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    let declaration = engine.declarations(&[Effect::Read]).remove(0);
    assert_eq!(declaration.id, "file.download");
    assert_eq!(declaration.input_schema["required"], json!(["url"]));
    assert_eq!(
        engine.reached_hosts().into_iter().collect::<Vec<_>>(),
        [FILES]
    );
    let api = port(vec![]);
    let files = port(vec![answer(
        200,
        &[("content-type", "application/pdf")],
        BYTES,
    )]);
    let value = engine
        .read_reaching(
            api.as_ref(),
            &admitted(&[(FILES, &files)]),
            "fixture",
            "file.download",
            json!({"url": "https://files.example.test/files-pri/T0-F0/fixture%20one.pdf"}),
        )
        .await
        .unwrap();
    connectors_sdk::validate(&declaration.output_schema, &value).unwrap();
    assert_eq!(value["body"]["sha256"], hex_sha256(BYTES));
    assert_eq!(value["body"]["media_type"], "application/pdf");
    assert_eq!(
        value["provenance"]["resource"],
        "https://files.example.test/files-pri/"
    );
    assert_eq!(
        files.calls(),
        calls(&["files-pri", "T0-F0", "fixture one.pdf"], &[])
    );
    assert!(
        api.calls().is_empty(),
        "a download never reaches the API base"
    );

    for (url, code, named) in [
        (
            "https://other.example.test/files-pri/T0-F0/x.pdf",
            ErrorCode::Forbidden,
            "which this download does not reach",
        ),
        (
            "https://files.example.test/api/x.pdf",
            ErrorCode::InvalidInput,
            "is not under `/files-pri/`",
        ),
        (
            "http://files.example.test/files-pri/T0-F0/x.pdf",
            ErrorCode::InvalidInput,
            "not an https URL",
        ),
        (
            "https://files.example.test/files-pri/../api/x",
            ErrorCode::InvalidInput,
            "not an https URL",
        ),
        (
            "https://files.example.test/files-pri/%2e%2e/api/x",
            ErrorCode::InvalidInput,
            "not an https URL",
        ),
    ] {
        let error = engine
            .read_reaching(
                api.as_ref(),
                &admitted(&[(FILES, &files)]),
                "fixture",
                "file.download",
                json!({"url": url}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, code, "{url}: {}", error.message);
        assert!(error.message.contains(named), "{url}: {}", error.message);
    }
    // Reached by the selection, but not admitted by the connection.
    let error = engine
        .read_reaching(
            api.as_ref(),
            &NoHosts,
            "fixture",
            "file.download",
            json!({"url": "https://files.example.test/files-pri/T0-F0/x.pdf"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert!(error.message.contains("the connection does not admit"));
    // Only `url` is input.
    let error = engine
        .read_reaching(
            api.as_ref(),
            &admitted(&[(FILES, &files)]),
            "fixture",
            "file.download",
            json!({"url": "https://files.example.test/files-pri/T0-F0/x.pdf", "id": "F1"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(files.calls().len(), 1, "a refused download sent a request");
    assert!(api.calls().is_empty());
}

#[test]
fn an_origin_is_canonical_https_with_no_user_information() {
    for (text, canonical) in [
        ("https://files.slack.com", Some("https://files.slack.com")),
        (
            "HTTPS://Files.Slack.com/files-pri/x",
            Some("https://files.slack.com"),
        ),
        (
            "https://files.slack.com:443",
            Some("https://files.slack.com"),
        ),
        ("https://localhost:8443/x?y", Some("https://localhost:8443")),
        ("http://files.slack.com", None),
        ("https://", None),
        ("https://reader@files.slack.com", None),
        ("https://files.slack.com:0", None),
        ("https://files.slack.com:65536", None),
        ("https://files.slack.com:", None),
        ("https://[::1]:8443", None),
        ("files.slack.com", None),
    ] {
        assert_eq!(origin(text).as_deref(), canonical, "{text}");
    }
}
