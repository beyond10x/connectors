//! Jira attachment content through the catalog engine: the shipped `attachment.content` selects
//! the pinned `getAttachmentContent` as a binary read. With `redirect` false Jira answers the
//! content itself (200); by default it answers 303 to a download URL (the pinned document's
//! responses), which is followed only to `https://api.media.atlassian.com`, and only through a
//! port the connection admits. Every id, byte and URL is synthetic; no network.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, NoHosts, ResponseKind, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

const BASE: &str = "/rest/api/3";
const MEDIA: &str = "https://api.media.atlassian.com";
/// An attachment's content: not UTF-8, so only a binary read can answer it.
const BYTES: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00 fixture jira attachment";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/jira/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    Engine::new(&bundle, BASE, &shipped()).unwrap()
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
fn media_hosts(media: &Arc<Port>) -> BTreeMap<String, Arc<dyn AuthenticatedHttp>> {
    let port: Arc<dyn AuthenticatedHttp> = media.clone();
    BTreeMap::from([(MEDIA.to_owned(), port)])
}
fn assert_bytes(value: &Value) {
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"]["media_type"], "image/png");
    assert_eq!(value["body"]["length"], BYTES.len());
    assert_eq!(value["body"]["sha256"], hex::encode(Sha256::digest(BYTES)));
    assert_eq!(
        STANDARD
            .decode(value["body"]["content_base64"].as_str().unwrap())
            .unwrap(),
        BYTES
    );
    assert_eq!(
        value["provenance"]["resource"],
        "/rest/api/3/attachment/content/{id}"
    );
}

#[test]
fn the_attachment_content_is_a_binary_read_of_the_pinned_operation() {
    let selections = shipped();
    let selection = selections
        .iter()
        .find(|s| s.id == "attachment.content")
        .unwrap();
    assert_eq!(selection.operation_id, "getAttachmentContent");
    assert_eq!(selection.response, Some(ResponseKind::Binary));
    let binary = selection.binary.as_ref().unwrap();
    assert_eq!(binary.hosts, [MEDIA]);
    assert_eq!(binary.max_bytes, connectors_catalog_provider::BINARY_LIMIT);
    let engine = engine();
    assert_eq!(engine.effect("attachment.content"), Some(Effect::Read));
    let declaration = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|d| d.id == "attachment.content")
        .unwrap();
    assert_eq!(declaration.input_schema["required"], json!(["id"]));
    assert!(
        declaration.input_schema["properties"]
            .get("redirect")
            .is_some()
    );
    // The pinned document states the 200 (with `redirect` false) and the 303
    // with its `Location`, and no media type for either.
    let document: Value = serde_json::from_slice(
        &fs::read(root().join("../atlassian/upstream/jira-platform-v3.json")).unwrap(),
    )
    .unwrap();
    let responses = &document["paths"]["/rest/api/3/attachment/content/{id}"]["get"]["responses"];
    assert!(
        responses["200"]["description"]
            .as_str()
            .unwrap()
            .contains("`redirect` is set to `false`")
    );
    assert!(
        responses["303"]["description"]
            .as_str()
            .unwrap()
            .contains("`Location` header")
    );
}

#[tokio::test]
async fn with_redirect_false_the_content_is_answered_directly() {
    let engine = engine();
    let api = port(vec![answer(200, &[("content-type", "image/png")], BYTES)]);
    let value = engine
        .read(
            api.as_ref(),
            "fixture-jira",
            "attachment.content",
            json!({"id": "10001", "redirect": false}),
        )
        .await
        .unwrap();
    assert_bytes(&value);
    assert_eq!(
        api.calls.lock().unwrap().clone(),
        vec![(
            vec![
                "attachment".to_owned(),
                "content".to_owned(),
                "10001".to_owned()
            ],
            vec![("redirect".to_owned(), "false".to_owned())]
        )]
    );
}

#[tokio::test]
async fn the_303_is_followed_to_the_admitted_media_host_only() {
    let engine = engine();
    let location = "https://api.media.atlassian.com/file/0f0f0f0f-fixture/binary?token=fixture-grant&client=fixture-client&dl=true";
    let api = port(vec![answer(303, &[("location", location)], b"")]);
    let media = port(vec![answer(200, &[("content-type", "image/png")], BYTES)]);
    let value = engine
        .read_reaching(
            api.as_ref(),
            &media_hosts(&media),
            "fixture-jira",
            "attachment.content",
            json!({"id": "10001"}),
        )
        .await
        .unwrap();
    assert_bytes(&value);
    assert_eq!(
        media.calls.lock().unwrap().clone(),
        vec![(
            vec![
                "file".to_owned(),
                "0f0f0f0f-fixture".to_owned(),
                "binary".to_owned()
            ],
            vec![
                ("token".to_owned(), "fixture-grant".to_owned()),
                ("client".to_owned(), "fixture-client".to_owned()),
                ("dl".to_owned(), "true".to_owned()),
            ]
        )]
    );

    // Not admitted by the connection: refused, nothing sent there.
    let api = port(vec![answer(303, &[("location", location)], b"")]);
    let error = engine
        .read_reaching(
            api.as_ref(),
            &NoHosts,
            "fixture-jira",
            "attachment.content",
            json!({"id": "10001"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert!(error.message.contains("the connection does not admit"));

    // Another host, even one the connection's ports include: refused by name.
    let api = port(vec![answer(
        303,
        &[("location", "https://fixture.example.test/file/x")],
        b"",
    )]);
    let other = port(vec![answer(200, &[], BYTES)]);
    let mut hosts = media_hosts(&media);
    let other_port: Arc<dyn AuthenticatedHttp> = other.clone();
    hosts.insert("https://fixture.example.test".to_owned(), other_port);
    let error = engine
        .read_reaching(
            api.as_ref(),
            &hosts,
            "fixture-jira",
            "attachment.content",
            json!({"id": "10001"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert!(
        error
            .message
            .contains("`https://fixture.example.test`, which this read does not reach"),
        "{}",
        error.message
    );
    assert!(other.calls.lock().unwrap().is_empty());
    assert_eq!(media.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn an_attachment_over_two_mebibytes_is_refused_as_capacity() {
    let engine = engine();
    let large = vec![0u8; connectors_catalog_provider::BINARY_LIMIT as usize + 1];
    let api = port(vec![answer(200, &[("content-type", "image/png")], &large)]);
    let error = engine
        .read(
            api.as_ref(),
            "fixture-jira",
            "attachment.content",
            json!({"id": "10001", "redirect": false}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);
    assert!(error.message.contains("max_bytes"), "{}", error.message);
}
