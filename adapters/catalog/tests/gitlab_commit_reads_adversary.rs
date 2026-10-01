//! Adversarial cases for the two GitLab commit-graph reads in the shipped
//! selection set, `commits.list` and `repository.compare`, driven through the
//! engine over the committed bundle with a recording transport. Each case
//! asserts what the pinned document (`adapters/gitlab/upstream/openapi_v3.yaml`)
//! or the story's acceptance says.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

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
fn reads(bodies: Vec<Value>) -> Reads {
    Reads {
        responses: Mutex::new(
            bodies
                .into_iter()
                .map(|body| HttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: serde_json::to_vec(&body).unwrap(),
                })
                .collect(),
        ),
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
fn properties(engine: &Engine, id: &str) -> serde_json::Map<String, Value> {
    engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == id)
        .unwrap_or_else(|| panic!("`{id}` is not a declared read"))
        .input_schema["properties"]
        .as_object()
        .unwrap()
        .clone()
}
/// Refused as `invalid_input` and nothing sent.
async fn refused(engine: &Engine, id: &str, input: Value) {
    let http = reads(vec![json!([])]);
    let outcome = engine.read(&http, "one", id, input.clone()).await;
    let calls = http.calls.lock().unwrap().clone();
    match outcome {
        Err(error) => {
            assert_eq!(error.code, ErrorCode::InvalidInput, "{id} {input}");
            assert!(calls.is_empty(), "{id} {input}: sent {calls:?}");
        }
        Ok(_) => panic!("{id} {input} was accepted and sent {calls:?}"),
    }
}
/// Accepted, and the one request's query, sorted.
async fn sent(engine: &Engine, id: &str, input: Value) -> Vec<(String, String)> {
    let http = reads(vec![json!([])]);
    engine
        .read(&http, "one", id, input.clone())
        .await
        .unwrap_or_else(|e| panic!("{id} {input}: {e:?}"));
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1, "{id} {input}");
    let mut query = calls[0].1.clone();
    query.sort();
    query
}

/// The declared input types follow the pinned document: `ref_name`, `since`,
/// `until`, `from` and `to` are strings; `first_parent` and `straight` are
/// booleans; `page` and `per_page` are integers.
#[test]
fn adversary_declared_types_follow_the_pinned_document() {
    let engine = engine();
    let string = json!({"type": ["string", "integer"]});
    let boolean = json!({"anyOf": [
        {"type": "boolean"},
        {"type": "string", "enum": ["true", "false"]}
    ]});
    let list = properties(&engine, "commits.list");
    for name in ["ref_name", "since", "until"] {
        assert_eq!(list[name], string, "commits.list {name}");
    }
    assert_eq!(list["first_parent"], boolean);
    for name in ["page", "per_page"] {
        assert_eq!(
            list[name]["anyOf"][0],
            json!({"type": "integer"}),
            "commits.list {name}"
        );
    }
    assert_eq!(list["per_page"]["minimum"], json!(1));
    assert_eq!(list["per_page"]["maximum"], json!(100));
    let compare = properties(&engine, "repository.compare");
    for name in ["from", "to"] {
        assert_eq!(compare[name], string, "repository.compare {name}");
    }
    assert_eq!(compare["straight"], boolean);
}

/// With keyset pagination GitLab does not read `page`, and the next-page
/// link travels in a header the provider never returns: the documented
/// walk ("stop on a short page") repeats page one while pages stay full.
/// The selection sends it on unchanged today; the guide says `commits.list`
/// supports offset paging only.
#[tokio::test]
async fn adversary_keyset_pagination_is_sent_until_the_selection_excludes_it() {
    let engine = engine();
    let query = sent(
        &engine,
        "commits.list",
        json!({"id": "org/project", "pagination": "keyset", "page": 2, "per_page": 100}),
    )
    .await;
    assert_eq!(
        query,
        [
            ("page".to_string(), "2".to_string()),
            ("pagination".to_string(), "keyset".to_string()),
            ("per_page".to_string(), "100".to_string()),
        ],
        "today `pagination=keyset` is sent; when story:catalog-selection-excludes-parameters \
         lands this flips to a refusal before any request"
    );
}

/// `order` is an enum in the pinned document (`default`, `topo`); a value
/// outside it is a malformed request the engine could refuse locally, but
/// today it is sent.
#[tokio::test]
async fn adversary_commits_list_order_outside_its_enum_is_sent_today() {
    let engine = engine();
    let query = sent(
        &engine,
        "commits.list",
        json!({"id": "org/project", "order": "newest"}),
    )
    .await;
    assert_eq!(
        query,
        [("order".to_string(), "newest".to_string())],
        "today an `order` outside its enum is sent; when \
         story:catalog-validates-enum-and-date-time lands this flips to a refusal"
    );
}

/// `since` and `until` are `format: date-time` in the pinned document; today a
/// value that is not one reaches GitLab unchanged.
#[tokio::test]
async fn adversary_commits_list_since_that_is_not_a_date_time_is_sent_today() {
    let engine = engine();
    for (since, text) in [
        (json!("yesterday"), "yesterday"),
        (json!(""), ""),
        (json!(20260901), "20260901"),
    ] {
        let query = sent(
            &engine,
            "commits.list",
            json!({"id": "org/project", "since": since}),
        )
        .await;
        assert_eq!(
            query,
            [("since".to_string(), text.to_string())],
            "today a `since` of {since} is sent; when \
             story:catalog-validates-enum-and-date-time lands this flips to a refusal"
        );
    }
}

/// `from` and `to` are both required: either missing, or given as JSON null,
/// is refused before any request.
#[tokio::test]
async fn adversary_compare_without_from_or_to_is_refused() {
    let engine = engine();
    for input in [
        json!({"id": 7, "to": "main"}),
        json!({"id": 7}),
        json!({"id": 7, "from": null, "to": "main"}),
        json!({"id": 7, "from": "v0.1.0", "to": null}),
        json!({"from": "v0.1.0", "to": "main"}),
    ] {
        refused(&engine, "repository.compare", input).await;
    }
}

/// `repository.compare` is not paged: a caller who tries to page it is
/// refused rather than silently sent the same compare twice.
#[tokio::test]
async fn adversary_compare_does_not_take_page_parameters() {
    let engine = engine();
    for input in [
        json!({"id": 7, "from": "a", "to": "b", "page": 2}),
        json!({"id": 7, "from": "a", "to": "b", "per_page": 100}),
    ] {
        refused(&engine, "repository.compare", input).await;
    }
}

/// `per_page` at both bounds is sent verbatim; every form outside them, or
/// not an integer, is refused before any request.
#[tokio::test]
async fn adversary_commits_list_per_page_bounds() {
    let engine = engine();
    for (given, text) in [
        (json!(1), "1"),
        (json!(100), "100"),
        (json!("1"), "1"),
        (json!("100"), "100"),
    ] {
        let query = sent(
            &engine,
            "commits.list",
            json!({"id": "org/project", "per_page": given}),
        )
        .await;
        assert_eq!(query, [("per_page".to_string(), text.to_string())]);
    }
    for given in [
        json!(-1),
        json!("0"),
        json!("-0"),
        json!("+5"),
        json!(" 5"),
        json!(1.5),
        json!(true),
        json!(null),
        json!("1e2"),
        json!(18446744073709551616u128.to_string()),
    ] {
        refused(
            &engine,
            "commits.list",
            json!({"id": "org/project", "per_page": given}),
        )
        .await;
    }
}

/// Twin of the `order` case on an operation the base already shipped:
/// `projects.list` `order_by` is an enum in the pinned document, and the
/// engine (unchanged by this unit) sends any string. The enum gap predates
/// the unit.
#[tokio::test]
async fn adversary_base_twin_projects_list_order_by_outside_its_enum_is_sent_today() {
    let engine = engine();
    let query = sent(&engine, "projects.list", json!({"order_by": "newest"})).await;
    assert_eq!(
        query,
        [("order_by".to_string(), "newest".to_string())],
        "today an `order_by` outside its enum is sent; when \
         story:catalog-validates-enum-and-date-time lands this flips to a refusal"
    );
}

/// Twin of the `since` case on an operation the base already shipped:
/// `projects.list` `last_activity_after` is `format: date-time` in the pinned
/// document. The format gap predates the unit.
#[tokio::test]
async fn adversary_base_twin_projects_list_last_activity_after_not_a_date_time_is_sent_today() {
    let engine = engine();
    let query = sent(
        &engine,
        "projects.list",
        json!({"last_activity_after": "yesterday"}),
    )
    .await;
    assert_eq!(
        query,
        [("last_activity_after".to_string(), "yesterday".to_string())],
        "today a `last_activity_after` that is not a date-time is sent; when \
         story:catalog-validates-enum-and-date-time lands this flips to a refusal"
    );
}

/// The re-pinned GitLab bootstrap digest in `gateway_prefix.rs` is explained
/// by the two added selections alone: the documented site-form configuration,
/// loaded with the shipped selection set minus `commits.list` and
/// `repository.compare`, prints the bootstrap whose digest and revision the
/// base pinned (`3fe50e13…`, `f87354a4…`).
#[test]
fn adversary_bootstrap_without_the_two_reads_is_the_base_pin() {
    use connectors_host::local::filesystem;
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = std::fs::read_to_string(root.join("../../docs/local-catalog-provider.md")).unwrap();
    let mut config: Value = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .filter_map(|body| serde_json::from_str::<Value>(body).ok())
        .find(|config| config["provider"] == "gitlab")
        .expect("the guide documents a gitlab configuration");
    assert!(config.get("request_prefix").is_none());

    let mut selection: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let before = selection["operations"].as_array().unwrap().len();
    selection["operations"]
        .as_array_mut()
        .unwrap()
        .retain(|o| o["id"] != "commits.list" && o["id"] != "repository.compare");
    assert_eq!(
        selection["operations"].as_array().unwrap().len(),
        before - 2
    );

    let directory = tempfile::tempdir().unwrap();
    let private = directory.path().join("private");
    filesystem::directory(&private, true, true).unwrap();
    let write = |path: &Path, bytes: &[u8]| {
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    };
    let operations = private.join("operations.json");
    write(&operations, &serde_json::to_vec(&selection).unwrap());
    config["bundle_directory"] = json!(root.join("generated/bundles").canonicalize().unwrap());
    config["operations_file"] = json!(operations);
    config.as_object_mut().unwrap().remove("ca_file");
    let path = private.join("catalog.json");
    write(&path, &serde_json::to_vec(&config).unwrap());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
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
    assert_eq!(
        (
            hex::encode(Sha256::digest(&output.stdout)),
            bootstrap["configuration_revision"].as_str().unwrap()
        ),
        (
            "3fe50e13d4ffa3aebc78634b2e14a94fe66c235c38af5569a79fcbf972fd5a9d".to_string(),
            "f87354a4d17235768ef1745c5693296658c66e7fedd179fbe00aac2a58b5bb80"
        )
    );
}
