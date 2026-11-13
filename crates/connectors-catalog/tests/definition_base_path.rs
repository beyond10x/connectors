//! story:catalog-path-prefix-from-config: a `base_url` whose path ends in a
//! variable `config` declares (`https://x.example/{accountId}`) carries that
//! path for every operation. The lowering drops the base URL's path and its
//! parameter from each operation that starts with it, so no caller passes the
//! value; the instance endpoint's path supplies it. A reached operation that
//! does not start with that path is refused as `outside_base_path`, and a
//! variable `config` does not declare is still `undeclared_variable`.
//! Synthetic source, Basecamp-shaped; no network.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::{Value, json};
use std::io::Write as _;

/// Every path but two is under `/{accountId}`, as every Basecamp path is.
fn openapi() -> Vec<u8> {
    let account = json!({"name": "accountId", "in": "path", "required": true,
        "schema": {"type": "string", "pattern": "^[0-9]+$"}});
    let ok = json!({"200": {"description": "ok", "content": {"application/json":
        {"schema": {"type": "object"}}}}});
    serde_json::to_vec(&json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture projects", "version": "1"},
        "paths": {
            "/{accountId}/projects.json": {
                "get": {"operationId": "ListProjects", "summary": "List projects",
                    "parameters": [account, {"name": "status", "in": "query",
                        "schema": {"type": "string"}}],
                    "responses": ok},
                "post": {"operationId": "CreateProject", "summary": "Create a project",
                    "parameters": [account],
                    "requestBody": {"content": {"application/json": {"schema":
                        {"type": "object", "properties": {"name": {"type": "string"}}}}}},
                    "responses": ok}
            },
            "/{accountId}/projects/{projectId}": {
                "get": {"operationId": "GetProject", "summary": "Get a project",
                    "parameters": [account, {"name": "projectId", "in": "path",
                        "required": true, "schema": {"type": "integer"}}],
                    "responses": ok}
            },
            "/authorization.json": {
                "get": {"operationId": "GetAuthorization", "summary": "Who holds the token",
                    "responses": ok}
            },
            "/{bucketId}/things.json": {
                "get": {"operationId": "ListThings", "summary": "List things",
                    "parameters": [{"name": "bucketId", "in": "path", "required": true,
                        "schema": {"type": "string"}}],
                    "responses": ok}
            }
        }
    }))
    .unwrap()
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// `base_url` and `config` as given, a bearer profile and `operations`.
fn definition(base_url: &str, config: &str, operations: &str) -> String {
    format!(
        "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/projects/openapi.json.gz
  sha256: {}
base_url: {base_url}
{config}auth:
  projects.token: {{scheme: bearer, label: Fixture token, identity: {{path: my/profile.json, kind: projects.person, subject: /id}}}}
operations:
{operations}",
        sha256(&openapi())
    )
}

const READS: &str = "  projects.list: {op: ListProjects}\n  project.get: {op: GetProject}\n";
const ACCOUNT: &str = "config:\n  accountId: {type: string, default: \"12345\"}\n";

struct Lowering {
    temp: tempfile::TempDir,
    outcome: Result<definition::Lowered, Refusal>,
}

impl Lowering {
    fn lowered(&self) -> &definition::Lowered {
        self.outcome.as_ref().expect("the definition lowers")
    }
    fn bundle(&self) -> connectors_catalog::bundle::Bundle {
        connectors_catalog::bundle::load(&self.temp.path().join("out"), "projects").unwrap()
    }
    fn declaration(&self) -> Value {
        serde_json::from_slice(&std::fs::read(&self.lowered().connector).unwrap()).unwrap()
    }
    fn refusal(self) -> Refusal {
        let out = self.temp.path().join("out");
        let refusal = self.outcome.expect_err("the definition is refused");
        assert!(
            !out.exists(),
            "a refused definition writes nothing to the output directory"
        );
        refusal
    }
}

fn lower(text: &str) -> Lowering {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    let upstream = root.join("upstream/projects");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::write(upstream.join("openapi.json.gz"), gzip(&openapi())).unwrap();
    let connectors = root.join("connectors");
    std::fs::create_dir_all(&connectors).unwrap();
    let file = connectors.join("projects.yaml");
    std::fs::write(&file, text).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let outcome = definition::lower(&Request {
        definition: &file,
        root: &root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    });
    Lowering { temp, outcome }
}

/// The bundle's operation `operation_id`: its path and its parameters' names.
fn operation(
    bundle: &connectors_catalog::bundle::Bundle,
    operation_id: &str,
) -> (String, Vec<String>) {
    let found = bundle
        .inventory
        .operations
        .iter()
        .find(|operation| operation.operation_id.as_deref() == Some(operation_id))
        .unwrap_or_else(|| panic!("the bundle carries no `{operation_id}`"));
    (
        found.path.clone(),
        found.parameters.iter().map(|p| p.name.clone()).collect(),
    )
}

#[test]
fn a_configured_trailing_path_variable_leaves_each_operation_and_its_parameter() {
    let lowering = lower(&definition("https://x.example/{accountId}", ACCOUNT, READS));
    let bundle = lowering.bundle();
    assert_eq!(
        operation(&bundle, "ListProjects"),
        ("/projects.json".to_owned(), vec!["status".to_owned()])
    );
    assert_eq!(
        operation(&bundle, "GetProject"),
        (
            "/projects/{projectId}".to_owned(),
            vec!["projectId".to_owned()]
        )
    );
    let declaration = lowering.declaration();
    assert_eq!(declaration["default_endpoint"], "https://x.example/12345");
    assert_eq!(declaration["endpoint_path"], "/{accountId}");
    assert_eq!(declaration.get("api_path"), None);
}

#[test]
fn without_a_default_the_endpoint_is_the_instance_s_and_the_path_is_still_dropped() {
    let config = "config:\n  accountId: {type: string}\n";
    let lowering = lower(&definition("https://x.example/{accountId}", config, READS));
    assert_eq!(
        operation(&lowering.bundle(), "ListProjects").0,
        "/projects.json"
    );
    let declaration = lowering.declaration();
    assert_eq!(declaration.get("default_endpoint"), None);
    assert_eq!(declaration["endpoint_path"], "/{accountId}");
}

#[test]
fn a_selected_operation_outside_the_base_path_is_refused() {
    for op in ["GetAuthorization", "ListThings"] {
        let operations = format!("{READS}  other: {{op: {op}}}\n");
        let refusal = lower(&definition(
            "https://x.example/{accountId}",
            ACCOUNT,
            &operations,
        ))
        .refusal();
        assert_eq!(refusal.name(), "outside_base_path", "{op}: {refusal}");
        assert!(refusal.to_string().contains("`other`"), "{op}: {refusal}");
    }
}

#[test]
fn a_guard_read_outside_the_base_path_is_refused() {
    let operations = format!(
        "{READS}  project.create:\n    op: CreateProject\n    effect: write\n    \
         body: {{name: string}}\n    guard:\n      before: {{read: GetAuthorization}}\n"
    );
    let refusal = lower(&definition(
        "https://x.example/{accountId}",
        ACCOUNT,
        &operations,
    ))
    .refusal();
    assert_eq!(refusal.name(), "outside_base_path", "{refusal}");
}

#[test]
fn an_undeclared_trailing_variable_is_still_refused_as_undeclared() {
    let refusal = lower(&definition("https://x.example/{accountId}", "", READS)).refusal();
    assert_eq!(refusal, Refusal::UndeclaredVariable("accountId".to_owned()));
}

#[test]
fn a_base_url_whose_path_ends_in_a_literal_keeps_every_path() {
    let config = "config:\n  host: {type: string, default: x.example}\n";
    let lowering = lower(&definition(
        "https://{host}/v1",
        config,
        "  things.list: {op: ListThings}\n",
    ));
    assert_eq!(
        operation(&lowering.bundle(), "ListThings"),
        (
            "/{bucketId}/things.json".to_owned(),
            vec!["bucketId".to_owned()]
        )
    );
    assert_eq!(
        operation(&lowering.bundle(), "ListProjects").0,
        "/{accountId}/projects.json"
    );
    let declaration = lowering.declaration();
    assert_eq!(declaration.get("endpoint_path"), None);
    assert_eq!(declaration["api_path"], "/v1");
}
