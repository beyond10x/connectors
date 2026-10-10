//! Adversary cases for the engine lane of wave 20260930c: repeated and
//! required query parameters, and what the rebuilt bundles did to the revisions
//! an operator binds approvals to. Each case drives the engine or the shipped
//! binary from what the unit's own story and guide say.
use connectors_catalog::{bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_host::local::filesystem;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

type Call = (Vec<String>, Vec<(String, String)>);

struct Recorder {
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Recorder {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: b"{}".to_vec(),
        })
    }
}
fn recorder() -> Arc<Recorder> {
    Arc::new(Recorder {
        calls: Mutex::new(Vec::new()),
    })
}

/// A repeated string, and a repeated integer with no bound, as Jira's
/// `reconcileIssues` and Confluence's `space-id` are declared.
fn shapes() -> connectors_catalog::bundle::Bundle {
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "adversary", "version": "1"},
        "paths": {"/v1/messages": {"get": {
            "operationId": "listMessages",
            "parameters": [
                {"name": "labelIds", "in": "query", "schema": {"type": "array", "items": {"type": "string"}}},
                {"name": "ids", "in": "query", "schema": {"type": "array", "items": {"type": "integer"}}}
            ],
            "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
        }}}
    }))
    .unwrap();
    let source = ingest("adversary.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    connectors_catalog::bundle::Bundle {
        provider: "adversary".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}
fn engine(selection: Value) -> Engine {
    let selection: Selection = serde_json::from_value(selection).unwrap();
    Engine::new(&shapes(), "/v1", &[selection]).unwrap()
}
fn plain() -> Engine {
    engine(json!({"id": "messages.list", "operation_id": "listMessages", "effect": "read"}))
}

/// Story `catalog-repeated-query-parameters`, Acceptance: "the operation's
/// declared input schema gives the repeated parameter `type: array` with
/// scalar `items`". Decided in correction round 1: the type is a union of
/// `array` with the element's scalar forms, because callers already send one
/// comma-joined string (Jira `fields`, Confluence `space-id`; see
/// `docs/catalog-jira.md` and `docs/catalog-confluence.md`), and a bare
/// `type: array` would refuse them.
#[test]
fn adversary_repeated_parameter_declares_type_array() {
    let declared = &plain().declarations(&[Effect::Read])[0].input_schema["properties"];
    assert_eq!(
        declared["labelIds"]["type"],
        json!(["array", "string", "integer"]),
        "string elements: an array, or one string (a comma-joined list included) \
         kept for comma-joined callers: {declared}"
    );
    assert_eq!(
        declared["labelIds"]["items"],
        json!({"type": ["string", "integer"]})
    );
    assert_eq!(
        declared["ids"]["type"],
        json!(["array", "integer", "string"]),
        "integer elements: an array, one integer, or a comma-joined list of \
         integers kept for comma-joined callers: {declared}"
    );
    assert_eq!(declared["ids"]["pattern"], json!("^-?[0-9]+(,-?[0-9]+)*$"));
}

/// The element type is known and declared as `items: integer`, so one scalar
/// for the parameter must be typed like it: an integer, or a string of
/// comma-separated integers (decided in correction round 1, for callers that
/// already send Confluence's `space-id` joined). A string that is no integer
/// list must not reach the provider.
#[tokio::test]
async fn adversary_a_scalar_for_a_repeated_integer_parameter_is_typed_like_its_elements() {
    // A comma-joined list of integers is the one scalar form kept: one pair,
    // as given.
    let http = recorder();
    plain()
        .read(
            http.as_ref(),
            "fixture",
            "messages.list",
            json!({"ids": "1,2"}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        [("ids".to_string(), "1,2".to_string())]
    );
    for ids in [json!("abc"), json!("1,x"), json!("1.5"), json!(true)] {
        let http = recorder();
        let result = plain()
            .read(
                http.as_ref(),
                "fixture",
                "messages.list",
                json!({ "ids": ids }),
            )
            .await;
        let calls = http.calls.lock().unwrap();
        assert!(
            matches!(&result, Err(error) if error.code == ErrorCode::InvalidInput)
                && calls.is_empty(),
            "{ids} was sent as {calls:?}"
        );
    }
}

/// The declaration and the engine must agree on a required repeated
/// parameter: the engine refuses `[]` as absent, so the declared schema must
/// not accept it.
#[tokio::test]
async fn adversary_a_required_repeated_parameter_declares_what_the_engine_refuses() {
    let required = engine(json!({
        "id": "messages.list", "operation_id": "listMessages", "effect": "read",
        "required": ["labelIds"]
    }));
    let input = json!({"labelIds": []});
    let schema = required.declarations(&[Effect::Read])[0]
        .input_schema
        .clone();
    let declared_valid = connectors_sdk::validate(&schema, &input).is_ok();
    let http = recorder();
    let refused = required
        .read(http.as_ref(), "fixture", "messages.list", input.clone())
        .await
        .is_err();
    assert!(
        !(declared_valid && refused),
        "the declared schema accepts {input} and the engine refuses it: {schema}"
    );
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The GitLab bundle as base `81b509745` committed it, rebuilt from the
/// current one: no parameter repeated, a repeated parameter's element type
/// dropped (the base read no type from an array schema), and no gaps.
fn base_gitlab(directory: &Path) {
    let mut gitlab = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    for operation in &mut gitlab.inventory.operations {
        for parameter in &mut operation.parameters {
            if parameter.repeated {
                parameter.repeated = false;
                parameter.value_type = None;
            }
        }
    }
    gitlab.inventory.unsupported.clear();
    // The base predates every cited amendment, the project search path and the
    // media types of the JSON-bodied writes: undo them and the record of their
    // amendment file.
    gitlab.source.amendments = None;
    for operation in &mut gitlab.inventory.operations {
        if operation.operation_id.as_deref() == Some("getApiV4ProjectsIdDashSearch") {
            operation.path = "/api/v4/projects/{id}/(-/)search".into();
        }
        if [
            "postApiV4ProjectsIdRepositoryCommits",
            "putApiV4ProjectsIdRepositoryFilesFilePath",
            "postApiV4Projects",
        ]
        .contains(&operation.operation_id.as_deref().unwrap_or_default())
        {
            operation.request_media_types = vec!["multipart/form-data".into()];
        }
    }
    let entry = bundle::write(directory, &gitlab, false).unwrap();
    // `git show 81b509745:adapters/catalog/generated/bundles/index.json`.
    assert_eq!(
        entry.bundle_sha256, "da458f3b69bf4634579d61482fa020733ef888c5d7de63cbb0e097b3685bd127",
        "the reconstruction is not the base bundle"
    );
}

/// The GitLab guide's site-form configuration, with a selection set of shipped
/// operations none of which has a repeated parameter: three reads and the
/// three guarded writes.
fn descriptor_revision(bundles: &Path, scratch: &Path) -> String {
    let shipped: Value =
        serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    let mut selections = shipped.clone();
    selections["operations"] = json!(
        shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["effect"] == "write"
                || ["project.get", "merge_request.get", "branch.get"]
                    .contains(&s["id"].as_str().unwrap()))
            .cloned()
            .collect::<Vec<_>>()
    );
    fs::create_dir_all(scratch).unwrap();
    fs::set_permissions(scratch, fs::Permissions::from_mode(0o700)).unwrap();
    let private_directory = scratch.join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let operations = private_directory.join("operations.json");
    private(&operations, &serde_json::to_vec(&selections).unwrap());
    let config = json!({
        "format": "connectors-catalog-local/2",
        "instance": "gitlab-sandbox",
        "provider": "gitlab",
        "bundle_directory": bundles,
        "api_base": "https://gitlab.example/api/v4",
        "auth": {
            "profile": "gitlab.pat",
            "header": "PRIVATE-TOKEN",
            "bearer": false,
            "label": "GitLab personal access token",
            "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
            "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
            "minimum_scopes": ["api"]
        },
        "operations_file": operations
    });
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(&config).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    let descriptor: Value =
        serde_json::from_str(bootstrap["descriptor"].as_str().unwrap()).unwrap();
    descriptor["revision"].as_str().unwrap().to_owned()
}

/// The descriptor revision is the instance's, derived from the configuration
/// revision, which digests the bundle's bytes: rebuilding the bundle moves it
/// for every GitLab, Jira and Confluence instance, including one whose
/// selections have no repeated parameter, and every write approval policy
/// bound to it must be issued again. `docs/local-catalog-provider.md`,
/// "Array and required query parameters", says so (correction round 1).
#[test]
fn adversary_an_instance_without_repeated_parameters_still_moves_its_descriptor_revision() {
    let scratch = tempfile::tempdir().unwrap();
    let base: PathBuf = scratch.path().join("base-bundles");
    base_gitlab(&base);
    let before = descriptor_revision(&base, &scratch.path().join("before"));
    let after = descriptor_revision(
        &root().join("generated/bundles").canonicalize().unwrap(),
        &scratch.path().join("after"),
    );
    assert_ne!(
        after, before,
        "the rebuilt bundle must move the instance's descriptor revision even where no \
         selection has a repeated parameter, as docs/local-catalog-provider.md \
         (\"Array and required query parameters\") tells operators; if it no longer \
         does, that paragraph is wrong"
    );
}
