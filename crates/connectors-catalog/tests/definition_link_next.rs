//! story:catalog-link-header-next-page: a paging style with `link_next: true`
//! asks that a read's next page be the `Link` response header's `rel="next"`
//! URL. The lowering carries `link_next` to the selection of every read that
//! takes the style, and to nothing else; the catalog provider answers that URL
//! as the read's `next`. A style that also names `next`, and a style that a
//! write or a read not answering JSON takes, is refused as `malformed` by name.
//! Synthetic source, Basecamp-shaped; no network.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::{Value, json};
use std::io::Write as _;

/// Basecamp's lists take `page`; one read and one write take nothing pageable.
fn openapi() -> Vec<u8> {
    let account = json!({"name": "accountId", "in": "path", "required": true,
        "schema": {"type": "string", "pattern": "^[0-9]+$"}});
    let page = json!({"name": "page", "in": "query", "schema": {"type": "integer"}});
    let ok = json!({"200": {"description": "ok", "content": {"application/json":
        {"schema": {"type": "array"}}}}});
    let text = json!({"200": {"description": "ok", "content": {"text/plain":
        {"schema": {"type": "string"}}}}});
    serde_json::to_vec(&json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture projects", "version": "1"},
        "paths": {
            "/{accountId}/projects.json": {
                "get": {"operationId": "ListProjects", "summary": "List projects",
                    "parameters": [account, page], "responses": ok},
                "post": {"operationId": "CreateProject", "summary": "Create a project",
                    "parameters": [account, page],
                    "requestBody": {"content": {"application/json": {"schema":
                        {"type": "object", "properties": {"name": {"type": "string"}}}}}},
                    "responses": ok}
            },
            "/{accountId}/projects/{projectId}.json": {
                "get": {"operationId": "GetProject", "summary": "Get a project",
                    "parameters": [account, {"name": "projectId", "in": "path",
                        "required": true, "schema": {"type": "integer"}}],
                    "responses": ok}
            },
            "/{accountId}/events.txt": {
                "get": {"operationId": "ListEventsText", "summary": "Events as text",
                    "parameters": [account, page], "responses": text}
            },
            "/{accountId}/my/profile.json": {
                "get": {"operationId": "GetMyProfile", "summary": "Who holds the token",
                    "parameters": [account], "responses": ok}
            }
        }
    }))
    .unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bytes))
}

fn definition(paging: &str, operations: &str) -> String {
    format!(
        "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/projects/openapi.json.gz
  sha256: {}
base_url: https://3.example.com/{{accountId}}
config:
  accountId: {{type: string}}
auth:
  projects.token: {{scheme: bearer, label: Fixture token, identity: {{path: my/profile.json, kind: projects.person, subject: /id}}}}
paging:
{paging}operations:
{operations}",
        sha256(&openapi())
    )
}

fn lower(text: &str) -> Result<Value, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(root.join("upstream/projects")).unwrap();
    std::fs::create_dir_all(root.join("connectors")).unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&openapi()).unwrap();
    std::fs::write(
        root.join("upstream/projects/openapi.json.gz"),
        encoder.finish().unwrap(),
    )
    .unwrap();
    let file = root.join("connectors/projects.yaml");
    std::fs::write(&file, text).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: &file,
        root: &root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    })?;
    Ok(serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap())
}

fn entry<'a>(selection: &'a Value, id: &str) -> &'a Value {
    selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("no entry `{id}`"))
}

#[test]
fn a_read_taking_a_link_next_style_carries_link_next_and_no_other_read_does() {
    let selection = lower(&definition(
        "  pages: {request: page, link_next: true}\n",
        "  projects.list: {op: ListProjects}\n  project.get: {op: GetProject}\n",
    ))
    .expect("the definition lowers");
    assert_eq!(entry(&selection, "projects.list")["link_next"], json!(true));
    assert!(
        entry(&selection, "project.get").get("link_next").is_none(),
        "a read without `page` takes no style: {selection}"
    );

    // A style without it lowers as before: no read carries `link_next`.
    let selection = lower(&definition(
        "  pages: {request: page, next: \"header:x-next-page\"}\n",
        "  projects.list: {op: ListProjects}\n",
    ))
    .expect("the definition lowers");
    assert!(
        entry(&selection, "projects.list")
            .get("link_next")
            .is_none(),
        "`next` names no header the read exposes: {selection}"
    );
    let selection = lower(&definition(
        "  pages: {request: page, link_next: false}\n",
        "  projects.list: {op: ListProjects}\n",
    ))
    .expect("the definition lowers");
    assert!(
        entry(&selection, "projects.list")
            .get("link_next")
            .is_none(),
        "false is as if unstated: {selection}"
    );
}

fn malformed(text: &str) -> String {
    match lower(text) {
        Err(Refusal::Malformed(message)) => message,
        other => panic!("not refused as malformed: {other:?}"),
    }
}

#[test]
fn a_link_next_style_beside_next_or_on_a_write_or_a_text_read_is_refused() {
    let message = malformed(&definition(
        "  pages: {request: page, next: /next, link_next: true}\n",
        "  projects.list: {op: ListProjects}\n",
    ));
    assert!(
        message.contains("link_next") && message.contains("`next`"),
        "{message}"
    );
    let message = malformed(&definition(
        "  pages: {request: page, link_next: true}\n",
        "  project.create: {op: CreateProject, effect: write}\n",
    ));
    assert!(
        message.contains("project.create") && message.contains("link_next"),
        "{message}"
    );
    let message = malformed(&definition(
        "  pages: {request: page, link_next: true}\n",
        "  events.text: {op: ListEventsText}\n",
    ));
    assert!(
        message.contains("events.text") && message.contains("link_next"),
        "{message}"
    );
}
