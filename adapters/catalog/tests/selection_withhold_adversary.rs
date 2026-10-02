//! Adversary pass against `withhold` (story:catalog-selection-excludes-parameters):
//! every route by which a withheld parameter could still reach the wire, the
//! load checks in orders the unit's own cases do not take, and the acceptance
//! driven through the adapter process rather than the engine alone. No network,
//! no credential.
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, Secret, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

type Call = (Vec<String>, Vec<(String, String)>);

/// Records every GET and answers from a queue; an empty queue panics, so an
/// unexpected read is loud.
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

/// The one-use write capability: records the query and body it was sent.
type Written = Arc<Mutex<Vec<(Vec<(String, String)>, Value)>>>;
struct Write {
    sent: Written,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        _method: WriteMethod,
        _path: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            body.clone(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&json!({"mode": "pinned"})).unwrap(),
        })
    }
}

// ---------------------------------------------------------------------------
// The shipped GitLab selection set.

fn shipped_gitlab() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn gitlab() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped_gitlab()).unwrap()
}

/// Refused as `invalid_input` with no request of any kind.
async fn read_refused(engine: &Engine, id: &str, input: Value) {
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

/// Every spelling of the withheld names a caller could reach for, and every
/// value of `pagination` (offset included: the guide says either name is
/// refused, whatever it carries).
#[tokio::test]
async fn adversary_no_spelling_or_value_of_the_keyset_parameters_reaches_commits_list() {
    let engine = gitlab();
    for key in [
        "pagination",
        "Pagination",
        "PAGINATION",
        " pagination",
        "pagination ",
        "query:pagination",
        "query:page_token",
        "pagination[]",
        "page_token",
        "page-token",
        "pageToken",
        "PAGE_TOKEN",
        "page_token[]",
    ] {
        let mut input = json!({"id": "org/project", "page": 2, "per_page": 100});
        input[key] = json!("keyset");
        read_refused(&engine, "commits.list", input).await;
    }
    for value in [
        json!("keyset"),
        json!("offset"),
        json!("none"),
        json!(""),
        json!(null),
        json!(["keyset"]),
        json!(1),
    ] {
        read_refused(
            &engine,
            "commits.list",
            json!({"id": "org/project", "pagination": value}),
        )
        .await;
    }
    // A value of another parameter cannot carry the pair into the query: the
    // engine hands `ref_name` to the transport as one value, which encodes it.
    let http = reads(vec![json!([])]);
    engine
        .read(
            &http,
            "one",
            "commits.list",
            json!({"id": "org/project", "ref_name": "main&pagination=keyset"}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(
        calls[0].1,
        [("ref_name".to_string(), "main&pagination=keyset".to_string())]
    );
}

// ---------------------------------------------------------------------------
// Through the adapter process: what `operations describe` reads, and what
// `operations invoke` answers.

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The GitLab configuration the guide shows, made loadable here, with an
/// `api_base` nothing listens on so any request that leaves fails as
/// `unavailable` rather than reaching a network.
fn gitlab_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/local-catalog-provider.md")).unwrap();
    let mut config = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .filter_map(|body| serde_json::from_str::<Value>(body).ok())
        .find(|config| config["provider"] == "gitlab")
        .expect("the guide documents a GitLab configuration");
    config["bundle_directory"] = json!(root().join("generated/bundles").canonicalize().unwrap());
    config["operations_file"] = json!(
        root()
            .join("providers/gitlab/operations.json")
            .canonicalize()
            .unwrap()
    );
    config["api_base"] = json!("https://localhost:9/api/v4");
    config.as_object_mut().unwrap().remove("ca_file");
    config
}

struct Process {
    _directory: tempfile::TempDir,
    adapter: Adapter,
    bootstrap: Bootstrap,
}
fn process() -> Process {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(&gitlab_config()).unwrap());
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
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .canonicalize()
        .unwrap();
    let adapter = Adapter {
        private_protocol: None,
        permissions: Default::default(),
        instance_id: bootstrap.instance.clone(),
        adapter_id: "catalog".into(),
        configuration_revision: bootstrap.configuration_revision.clone(),
        protocol: "v1alpha1".into(),
        startup: Startup::OnDemand,
        restart: Restart::Never,
        executable: Executable {
            sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
            path: binary,
            args: vec!["--local-config".into(), path.to_str().unwrap().into()],
        },
    };
    Process {
        _directory: directory,
        adapter,
        bootstrap,
    }
}

/// The descriptor the adapter binary publishes for the shipped GitLab
/// configuration, which is what `operations describe` shows.
#[test]
fn adversary_the_published_descriptor_lists_no_keyset_parameter() {
    let descriptor = process().bootstrap.descriptor().unwrap();
    let list = descriptor
        .operations
        .iter()
        .find(|o| o.id == "commits.list")
        .expect("commits.list is published");
    let properties = list.input_schema["properties"].as_object().unwrap();
    for name in ["pagination", "page_token"] {
        assert!(!properties.contains_key(name), "published {name}");
    }
    for name in ["id", "ref_name", "page", "per_page"] {
        assert!(properties.contains_key(name), "lost {name}");
    }
    assert_eq!(list.input_schema["additionalProperties"], json!(false));
}

/// The acceptance through the adapter process: a keyset input is
/// `invalid_input`, while the same input without it goes on to a request
/// (here `unavailable`, because nothing listens), so the refusal is the
/// withheld name's and not the fixture's.
#[test]
fn adversary_a_keyset_input_to_the_adapter_process_is_invalid_input() {
    let process = process();
    let revision = process.bootstrap.descriptor().unwrap().revision;
    let mut child = Child::spawn(&process.adapter).unwrap();
    let secret = Secret(serde_json::to_vec(&json!({"token": "fixture-token"})).unwrap());
    let deadline = connectors_sdk::now_ms() + 30_000;
    let invoke = |child: &mut Child, input: Value| {
        child.invoke(
            "commits.list",
            &revision,
            "one",
            &secret,
            &serde_json::to_vec(&input).unwrap(),
            deadline,
        )
    };
    for input in [
        json!({"id": "org/project", "pagination": "keyset"}),
        json!({"id": "org/project", "page_token": "abc123"}),
    ] {
        match invoke(&mut child, input.clone()) {
            Err(Failure::InvalidInput) => {}
            other => panic!("{input}: {other:?}"),
        }
    }
    match invoke(&mut child, json!({"id": "org/project", "page": 2})) {
        Err(Failure::InvalidInput) => panic!("a plain page read was refused as invalid_input"),
        Err(_) => {}
        Ok(output) => panic!("reached a provider: {}", String::from_utf8_lossy(&output)),
    }
}

// ---------------------------------------------------------------------------
// A fixture document for the shapes the shipped set does not carry: a name in
// two locations, a repeated parameter, a guarded write.

fn fixture() -> Bundle {
    let string = json!({"type": "string"});
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "withhold", "version": "1"},
        "paths": {
            "/v1/items": {"get": {
                "operationId": "listItems",
                "parameters": [
                    {"name": "mode", "in": "query", "schema": string},
                    {"name": "mode", "in": "header", "schema": string},
                    {"name": "tags", "in": "query",
                     "schema": {"type": "array", "items": {"type": "string"}}},
                    {"name": "limit", "in": "query", "schema": {"type": "integer"}}
                ],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }},
            "/v1/twins/{x}": {"get": {
                "operationId": "getTwin",
                "parameters": [
                    {"name": "x", "in": "path", "required": true, "schema": string},
                    {"name": "x", "in": "query", "schema": string}
                ],
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }},
            "/v1/things/{id}": {
                "get": {
                    "operationId": "getThing",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": string},
                        {"name": "mode", "in": "query", "schema": string}
                    ],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                },
                "put": {
                    "operationId": "putThing",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": string},
                        {"name": "mode", "in": "query", "schema": string}
                    ],
                    "requestBody": {"required": true,
                                    "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            }
        }
    }))
    .unwrap();
    Bundle {
        provider: "withhold".into(),
        source: ingest("withhold.json", &bytes).unwrap(),
        inventory: inventory::extract(&serde_json::from_slice(&bytes).unwrap()),
        auth_profile: "fixture.token".into(),
    }
}
fn written(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}
fn load(selection: Value) -> Result<Engine> {
    Engine::new(&fixture(), "/v1", &[written(selection)])
}
fn declared(engine: &Engine, effect: Effect) -> serde_json::Map<String, Value> {
    engine.declarations(&[effect])[0].input_schema["properties"]
        .as_object()
        .unwrap()
        .clone()
}

/// A name declared both as a query and as an optional header parameter is
/// withheld from both: nothing declares it, an input carrying it is refused,
/// and a read without it sends no `mode`.
#[tokio::test]
async fn adversary_a_name_in_query_and_header_is_withheld_from_both() {
    let engine = load(json!({
        "id": "items.list", "operation_id": "listItems", "effect": "read",
        "withhold": ["mode"]
    }))
    .unwrap();
    assert!(!declared(&engine, Effect::Read).contains_key("mode"));
    read_refused(&engine, "items.list", json!({"mode": "keyset"})).await;
    read_refused(&engine, "items.list", json!({"header:mode": "keyset"})).await;
    let http = reads(vec![json!([])]);
    engine
        .read(&http, "one", "items.list", json!({"limit": 5}))
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        [("limit".to_string(), "5".to_string())]
    );
}

/// A repeated query parameter, withheld: neither its array form nor its
/// single-value form is admitted.
#[tokio::test]
async fn adversary_a_withheld_repeated_parameter_is_refused_in_every_form() {
    let engine = load(json!({
        "id": "items.list", "operation_id": "listItems", "effect": "read",
        "withhold": ["tags"]
    }))
    .unwrap();
    assert!(!declared(&engine, Effect::Read).contains_key("tags"));
    for tags in [json!(["a", "b"]), json!("a,b"), json!([]), json!("a")] {
        read_refused(&engine, "items.list", json!({"tags": tags})).await;
    }
}

/// The query twin of a path parameter cannot be withheld on its own: the name
/// reaches the path parameter, which is required. Refused at load rather than
/// silently dropping the path parameter as well; and a case variant of a
/// declared name is not a declared name.
#[test]
fn adversary_withhold_load_refusals_the_unit_does_not_take() {
    for (case, selection) in [
        (
            "path twin",
            json!({"id": "twin.get", "operation_id": "getTwin", "effect": "read",
                   "withhold": ["x"]}),
        ),
        (
            "case variant",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "withhold": ["Mode"]}),
        ),
        (
            "qualified spelling",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "withhold": ["query:mode"]}),
        ),
    ] {
        let error = load(selection)
            .err()
            .unwrap_or_else(|| panic!("`{case}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{case}");
        assert!(
            error.message.contains("withholds"),
            "{case}: {}",
            error.message
        );
    }
}

fn guard(values: Value, preflight: Value, postflight: Value) -> Value {
    json!({
        "preflight": {"operation_id": "getThing", "values": values, "checks": preflight},
        "postflight": {"checks": postflight}
    })
}

/// A guard reads input through its preflight `values` and its postflight
/// checks as well as its preflight checks. Either would declare the withheld
/// name again as a guard reference, so either is refused at load.
#[test]
fn adversary_every_guard_reference_to_a_withheld_name_is_refused_at_load() {
    let literal = json!([{"pointer": "/state", "expect": {"literal": "open"}}]);
    for (case, guard) in [
        (
            "preflight value",
            guard(
                json!({"id": "id", "mode": "mode"}),
                literal.clone(),
                json!([]),
            ),
        ),
        (
            "preflight value as the probe's path",
            guard(json!({"id": "mode"}), literal.clone(), json!([])),
        ),
        (
            "postflight check",
            guard(
                json!({"id": "id"}),
                literal.clone(),
                json!([{"pointer": "/mode", "expect": {"input": "mode"}}]),
            ),
        ),
        (
            "nested reference",
            guard(
                json!({"id": "id"}),
                json!([{"pointer": "/mode", "expect": {"input": "mode.value"}}]),
                json!([]),
            ),
        ),
    ] {
        let error = load(json!({
            "id": "thing.put", "operation_id": "putThing", "effect": "write",
            "withhold": ["mode"], "guard": guard
        }))
        .err()
        .unwrap_or_else(|| panic!("`{case}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{case}");
        assert!(
            error.message.contains("withholds"),
            "{case}: {}",
            error.message
        );
    }
}

/// A guarded write that withholds `mode`: an input carrying it is refused
/// before the preflight read; a guard reading `mode` under another spelling
/// or from the body only compares, and the write's query never carries
/// `mode`; and a body key of the same name stays in the body.
#[tokio::test]
async fn adversary_a_withheld_write_parameter_reaches_neither_the_preflight_nor_the_write() {
    let engine = load(json!({
        "id": "thing.put", "operation_id": "putThing", "effect": "write",
        "withhold": ["mode"], "body_keys": ["mode", "name"],
        "guard": guard(
            json!({"id": "id", "mode": "query:mode"}),
            json!([{"pointer": "/mode", "expect": {"input": "body.mode"}}]),
            json!([{"pointer": "/mode", "expect": {"input": "query:mode"}}]),
        )
    }))
    .unwrap();
    let declaration = declared(&engine, Effect::Write);
    assert!(!declaration.contains_key("mode"), "{declaration:?}");

    // Carrying the withheld name: refused, and not even the preflight is read.
    let http = reads(vec![json!({"mode": "pinned"})]);
    let error = engine
        .prepare(
            &http,
            "one",
            "thing.put",
            json!({"id": "a", "mode": "keyset", "query:mode": "pinned",
                   "body": {"mode": "pinned"}}),
        )
        .await
        .err()
        .expect("an input carrying the withheld name was prepared");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(http.calls.lock().unwrap().is_empty());

    // Without it: the probe carries the guard's value (the probe's own
    // parameter), and the write's query carries nothing.
    let http = reads(vec![json!({"mode": "pinned"})]);
    let prepared = engine
        .prepare(
            &http,
            "one",
            "thing.put",
            json!({"id": "a", "query:mode": "pinned", "body": {"mode": "pinned", "name": "n"}}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0],
        (
            vec!["things".to_string(), "a".to_string()],
            vec![("mode".to_string(), "pinned".to_string())]
        )
    );
    let sent: Written = Arc::default();
    match prepared
        .execute(Box::new(Write { sent: sent.clone() }))
        .await
    {
        WriteOutcome::Applied(Ok(_)) => {}
        other => panic!(
            "{:?}",
            match other {
                WriteOutcome::Applied(r) => r.err(),
                WriteOutcome::Refused(e) | WriteOutcome::Unknown(e) => Some(e),
            }
        ),
    }
    let sent = sent.lock().unwrap().clone();
    assert_eq!(
        sent,
        [(Vec::new(), json!({"mode": "pinned", "name": "n"}))],
        "the write's query must not carry the withheld `mode`"
    );
}
