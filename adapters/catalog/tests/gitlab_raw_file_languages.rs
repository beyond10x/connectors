//! The GitLab reads `file.raw` and `project.languages` in the shipped selection
//! set, driven through the engine over the committed bundle with a recording
//! transport. `file.raw` reads a repository file's raw body as text
//! (`connectors.transport.TextBodyOutcome`, `ess/domains/transport.yaml`): its
//! exact UTF-8 text with `ref` sent, a body that is not UTF-8 refused
//! `upstream_protocol`, and a body over the 4 MiB provider response bound
//! refused `capacity` on its raw bytes. `project.languages` returns the
//! language map with its percentages unchanged. Bodies are synthetic.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, RESPONSE_LIMIT, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

/// UTF-8 text a normalising reader would change: a byte-order mark, CRLF and
/// LF line endings, multi-byte characters, a NUL and no final newline.
const TEXT: &[u8] = "\u{feff}# Guide\r\nna\u{ef}ve caf\u{e9} \u{2014} \u{2713}\n\0tail".as_bytes();
/// Latin-1 bytes, which are not UTF-8.
const LATIN1: &[u8] = b"caf\xe9 \xff\xfe\n";

struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
fn answer(media_type: &str, body: Vec<u8>) -> Reads {
    Reads {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status: 200,
            headers: [("content-type".to_owned(), media_type.to_owned())].into(),
            body,
        }])),
        calls: Mutex::new(Vec::new()),
    }
}
fn shipped() -> Vec<Selection> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped()).unwrap()
}
fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}
fn raw_input() -> Value {
    json!({"id": "org/project", "file_path": "docs/guide.md", "ref": "feature/text"})
}

/// Both are declared reads with the source's parameters, neither is a write.
#[test]
fn file_raw_and_project_languages_are_declared_reads() {
    let engine = engine();
    let reads = engine.declarations(&[Effect::Read]);
    for (id, parameters, required) in [
        (
            "file.raw",
            &["id", "file_path", "ref", "lfs"][..],
            &["file_path", "id"][..],
        ),
        ("project.languages", &["id"][..], &["id"][..]),
    ] {
        let declaration = reads
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not a declared read"));
        let properties = declaration.input_schema["properties"].as_object().unwrap();
        let mut declared: Vec<&str> = properties.keys().map(String::as_str).collect();
        declared.sort();
        let mut expected = parameters.to_vec();
        expected.sort();
        assert_eq!(declared, expected, "`{id}` parameters");
        let mut needed: Vec<&str> = declaration.input_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        needed.sort();
        assert_eq!(needed, required, "`{id}` required");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
    }
    let writes = engine.declarations(&[Effect::Write]);
    for id in ["file.raw", "project.languages"] {
        assert!(!writes.iter().any(|o| o.id == id), "`{id}` is a write");
    }
}

/// A UTF-8 file comes back as exactly its bytes, as a string, from the raw
/// route at the `ref` the caller named.
#[tokio::test]
async fn file_raw_returns_the_exact_text_at_the_named_ref() {
    let engine = engine();
    let http = answer("text/plain; charset=utf-8", TEXT.to_vec());
    let result = engine
        .read(&http, "one", "file.raw", raw_input())
        .await
        .unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"].as_str().unwrap().as_bytes(), TEXT);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        strings(&[
            "projects",
            "org/project",
            "repository",
            "files",
            "docs/guide.md",
            "raw"
        ])
    );
    assert_eq!(
        calls[0].1,
        vec![("ref".to_owned(), "feature/text".to_owned())]
    );
}

/// The declared `application/octet-stream` does not make a text body JSON or
/// binary: whatever media type the answer names, the bytes are the text.
#[tokio::test]
async fn file_raw_reads_an_octet_stream_answer_as_text() {
    let engine = engine();
    let http = answer(
        "application/octet-stream",
        b"{\"not\": \"parsed\"}".to_vec(),
    );
    let result = engine
        .read(&http, "one", "file.raw", raw_input())
        .await
        .unwrap();
    assert_eq!(result["body"], json!("{\"not\": \"parsed\"}"));
    let empty = answer("application/octet-stream", Vec::new());
    let result = engine
        .read(&empty, "one", "file.raw", raw_input())
        .await
        .unwrap();
    assert_eq!(result["body"], json!(""));
}

/// A file that is not UTF-8 is refused `upstream_protocol` by name, never
/// answered with replacement characters or base64.
#[tokio::test]
async fn file_raw_refuses_a_body_that_is_not_utf8_by_name() {
    let engine = engine();
    let http = answer("application/octet-stream", LATIN1.to_vec());
    let error = engine
        .read(&http, "one", "file.raw", raw_input())
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
    assert_eq!(
        error.message,
        "provider returned a text body that is not UTF-8"
    );
}

/// The provider response bound holds on the raw bytes: one byte over 4 MiB is
/// refused `capacity`, before any decoding, even when the bytes are not UTF-8;
/// exactly 4 MiB is read.
#[tokio::test]
async fn file_raw_refuses_a_body_over_the_provider_bound_as_capacity() {
    let engine = engine();
    let over = answer("text/plain", vec![b'a'; RESPONSE_LIMIT + 1]);
    let error = engine
        .read(&over, "one", "file.raw", raw_input())
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);

    let mut bytes = vec![b'a'; RESPONSE_LIMIT];
    bytes.push(0xff);
    let over_and_invalid = answer("application/octet-stream", bytes);
    let error = engine
        .read(&over_and_invalid, "one", "file.raw", raw_input())
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);

    let at = answer("text/plain", vec![b'a'; RESPONSE_LIMIT]);
    let result = engine
        .read(&at, "one", "file.raw", raw_input())
        .await
        .unwrap();
    assert_eq!(result["body"].as_str().unwrap().len(), RESPONSE_LIMIT);
}

/// `project.languages` returns the language map, each name with its
/// percentage, unchanged, from the languages route.
#[tokio::test]
async fn project_languages_returns_each_language_with_its_percentage() {
    let engine = engine();
    let languages = json!({"Rust": 71.42, "Shell": 20.0, "Dockerfile": 8.58});
    let http = answer("application/json", serde_json::to_vec(&languages).unwrap());
    let result = engine
        .read(
            &http,
            "one",
            "project.languages",
            json!({"id": "org/project"}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"], languages);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec![(strings(&["projects", "org/project", "languages"]), vec![])]
    );
}
