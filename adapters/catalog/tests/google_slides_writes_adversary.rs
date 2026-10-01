//! Adversary pass 1 against the guarded Google Slides writes: the claims of
//! `docs/catalog-google-slides.md` an agent follows before writing to a real
//! deck, driven through the engine against a scripted provider and through the
//! provider's own bootstrap and the host's connection registry. No network, no
//! credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_host::local::{filesystem, metadata::Metadata, registry, runtime::Bootstrap};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

const PROFILE: &str = "google.oauth";
const READ_SCOPE: &str = "https://www.googleapis.com/auth/presentations.readonly";
const WRITE_SCOPE: &str = "https://www.googleapis.com/auth/presentations";
const DECK: &str = "fixture-deck-1";
const REVISION: &str = "fixture-revision-2";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine() -> Engine {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-slides/operations.json")).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "google-slides").unwrap();
    Engine::new(&bundle, "", &selections).unwrap()
}
fn response(value: Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: [("content-type".to_owned(), "application/json".to_owned())].into(),
        body: serde_json::to_vec(&value).unwrap(),
    }
}

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
type Sent = Arc<Mutex<Vec<(Call, Value)>>>;
struct Send {
    sent: Sent,
    response: HttpResponse,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Send {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        assert_eq!(method, WriteMethod::Post);
        self.sent.lock().unwrap().push((
            (
                path.iter().map(|s| s.to_string()).collect(),
                query
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
            ),
            body.clone(),
        ));
        Ok(self.response)
    }
}

/// The guide: "Every other query parameter the projection declares for an
/// operation is accepted by name, including the document-wide `fields`", and
/// the projection declares `fields` on `slides.presentations.batchUpdate`, so
/// an agent may narrow the answer to its `replies`. Google then answers 200
/// with the replies only: every request applied atomically, and no
/// `presentationId` to compare. The postflight reads the absent field as a
/// difference, so a write Google confirmed is reported as `Unknown`. The guide
/// and the selection's description state that a `fields` value on
/// `batchUpdate` must keep `presentationId`; this case pins today's `Unknown`
/// for one that does not, and `Applied` for one that does.
#[tokio::test]
async fn a_batch_update_narrowed_without_presentation_id_is_reported_unknown_as_the_guide_warns() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-slides.md")).unwrap();
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("A `fields` value on `batchUpdate` must therefore keep `presentationId`"),
        "the guide no longer warns that `fields` on `batchUpdate` must keep `presentationId`"
    );
    let cases = [
        ("replies", json!({"replies": [{}]}), false),
        (
            "presentationId,replies",
            json!({"presentationId": DECK, "replies": [{}]}),
            true,
        ),
    ];
    for (fields, acknowledgement, applied) in cases {
        let input = json!({"presentationId": DECK, "fields": fields, "body": {
            "requests": [{"createSlide": {"objectId": "fixture-slide-3"}}],
            "writeControl": {"requiredRevisionId": REVISION}}});
        let http = Reads {
            responses: Mutex::new(VecDeque::from([response(
                json!({"presentationId": DECK, "revisionId": REVISION}),
            )])),
            calls: Mutex::new(Vec::new()),
        };
        let prepared = engine()
            .prepare(
                &http,
                "fixture-google-slides",
                "presentations.batchUpdate",
                input,
            )
            .await
            .expect("the current revision passes the preflight");
        // The preflight read is not narrowed: `values` maps `presentationId` only.
        assert_eq!(
            http.calls.lock().unwrap().as_slice(),
            [(
                vec!["v1".to_owned(), "presentations".to_owned(), DECK.to_owned()],
                vec![]
            )]
        );
        let sent: Sent = Arc::new(Mutex::new(Vec::new()));
        let outcome = prepared
            .execute(Box::new(Send {
                sent: sent.clone(),
                response: response(acknowledgement),
            }))
            .await;
        // The narrowing reached Google on the POST.
        assert_eq!(
            sent.lock().unwrap()[0].0.1,
            [("fields".to_owned(), fields.to_owned())]
        );
        match (outcome, applied) {
            (WriteOutcome::Applied(Ok(_)), true) => {}
            (WriteOutcome::Unknown(error), false) => assert!(
                error.message.contains("`/presentationId`"),
                "`{fields}`: unknown for another reason: {}",
                error.message
            ),
            (WriteOutcome::Applied(Ok(_)), false) => panic!(
                "`{fields}` without `presentationId` is now reported applied: the guide's \
                 warning that a `fields` value on `batchUpdate` must keep `presentationId` \
                 (docs/catalog-google-slides.md, Writes) no longer holds; update it"
            ),
            (
                WriteOutcome::Applied(Err(error))
                | WriteOutcome::Refused(error)
                | WriteOutcome::Unknown(error),
                _,
            ) => panic!(
                "`{fields}`: expected {}, got {:?} {}; see the guide's sentence that a \
                 `fields` value on `batchUpdate` must keep `presentationId`",
                if applied { "applied" } else { "unknown" },
                error.code,
                error.message
            ),
        }
    }
}

/// A JSON block of the guide, by a string it contains.
fn guide_block(marker: &str) -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-slides.md")).unwrap();
    let block = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(marker))
        .unwrap_or_else(|| panic!("the guide has no JSON block with {marker}"));
    serde_json::from_str(block).unwrap()
}
/// The guide's documented `google-slides` configuration, changed by `adjust`,
/// printed as the provider's bootstrap.
fn bootstrap(directory: &Path, name: &str, adjust: impl FnOnce(&mut Value)) -> Bootstrap {
    let mut config = guide_block("\"provider\": \"google-slides\"");
    config["bundle_directory"] = json!(canonical("generated/bundles"));
    config["operations_file"] = json!(canonical("providers/google-slides/operations.json"));
    adjust(&mut config);
    let path = directory.join(name);
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bootstrap inspection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    bootstrap
}
fn canonical(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}

/// The guide's route to a write-capable connection: a second instance,
/// `google-slides-write`, whose configuration is the read configuration with
/// the guide's write values applied, connected afresh with `connections
/// connect`. Beside an existing read-only connection, that instance starts a
/// fresh acquisition. The guide's reason repair cannot widen the read
/// connection holds: the write scopes change the configuration revision (the
/// provider digests `auth`, `adapters/catalog/src/local.rs:703`) and the
/// profile, both part of the binding, and the same instance reconfigured for
/// writes is refused a new acquisition while the read one exists.
#[test]
fn the_documented_write_instance_starts_a_fresh_acquisition() {
    let scratch = tempfile::tempdir().unwrap();
    let directory = scratch.path().join("private");
    filesystem::directory(&directory, true, true).unwrap();
    let values = guide_block("\"instance\": \"google-slides-write\"");
    assert_eq!(values["auth"]["minimum_scopes"], json!([WRITE_SCOPE]));
    assert_eq!(
        values["auth"]["requested_scopes"],
        json!(["openid", WRITE_SCOPE])
    );
    let write_values = |config: &mut Value| {
        config["auth"]["minimum_scopes"] = values["auth"]["minimum_scopes"].clone();
        config["auth"]["requested_scopes"] = values["auth"]["requested_scopes"].clone();
    };
    let read = bootstrap(&directory, "read.json", |config| {
        assert_eq!(config["auth"]["minimum_scopes"], json!([READ_SCOPE]));
    })
    .binding(PROFILE)
    .unwrap();
    let write = bootstrap(&directory, "write.json", |config| {
        config["instance"] = values["instance"].clone();
        write_values(config);
    })
    .binding(PROFILE)
    .unwrap();
    let widened = bootstrap(&directory, "widened.json", write_values)
        .binding(PROFILE)
        .unwrap();

    let state = scratch.path().join("state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    filesystem::directory(&state, false, true).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    let registry = registry::Registry::new(&state);
    let now = connectors_sdk::now_ms();
    // The operator's existing read-only connection.
    registry.begin(&read, now).unwrap();
    // The documented route: the write instance starts its own acquisition.
    assert_eq!(write.instance_id, "google-slides-write");
    assert_ne!(write.instance_id, read.instance_id);
    registry
        .begin(&write, now + 1)
        .unwrap_or_else(|failure| panic!("the write instance was refused: {failure:?}"));
    // Not the route: the read instance itself reconfigured for writes.
    assert_eq!(widened.instance_id, read.instance_id);
    let read = serde_json::to_value(&read).unwrap();
    let widened_value = serde_json::to_value(&widened).unwrap();
    for field in ["configuration_revision", "profile"] {
        assert_ne!(read[field], widened_value[field], "`{field}` unchanged");
    }
    assert!(
        registry.begin(&widened, now + 2).is_err(),
        "the read instance reconfigured for writes started a second acquisition"
    );
}
