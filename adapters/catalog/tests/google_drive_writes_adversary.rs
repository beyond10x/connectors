//! Adversary pass 1 against the guarded Google Drive metadata writes: the
//! guide, the selection descriptions an agent reads and the declared inputs,
//! driven through the engine against a scripted provider that answers as Drive
//! does. No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

/// Path segments and query of one request.
type Call = (Vec<String>, Vec<(String, String)>);

struct Scripted {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
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
fn json_response(status: u16, body: &Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: [("content-type".to_owned(), "application/json".to_owned())].into(),
        body: serde_json::to_vec(body).unwrap(),
    }
}
fn scripted(status: u16, body: &Value) -> Scripted {
    Scripted {
        responses: Mutex::new(VecDeque::from([json_response(status, body)])),
        calls: Mutex::new(Vec::new()),
    }
}

/// The one write a prepared request sends, as method, segments, query and body.
type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Vec<(String, String)>, Value)>>>;
struct Recorder {
    sent: Sent,
    answer: Value,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Recorder {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        segments: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            segments.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            body.clone(),
        ));
        Ok(json_response(200, &self.answer))
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/google-drive/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn drive() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "google-drive").unwrap();
    Engine::new(&bundle, "/drive/v3", &shipped()).unwrap()
}
fn guide() -> String {
    std::fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap()
}
/// The guide's `## Writes` section, up to the next second-level heading.
fn writes_section(guide: &str) -> String {
    let start = guide
        .find("\n## Writes\n")
        .expect("the guide's Writes section");
    let rest = &guide[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    rest[..end].to_owned()
}

/// `files.update` declares `supportsAllDrives` and forwards it on the PATCH,
/// so a caller updating a shared-drive file sends it. The update's preflight is
/// the `files.get` of that same file, and `files.get` declares the same
/// parameter with the same meaning ("Whether the requesting application
/// supports both My Drives and shared drives", the pinned Discovery document);
/// the guide says the preflight is "one `files.get` for `fileId` with the same
/// `fields`". The guard maps only `fileId` and `fields` into the preflight
/// (`operations.json`), so the preflight of a shared-drive update asks Drive
/// for the file as an application that does not support shared drives, while
/// the PATCH it guards says it does. The preflight has to read the file the
/// PATCH writes, under the same shared-drive mode.
#[tokio::test]
async fn files_update_preflight_reads_under_the_shared_drive_mode_of_the_patch() {
    let http = scripted(200, &json!({"id": "fixture-shared-1", "version": "7"}));
    let input = json!({"fileId": "fixture-shared-1", "version": "7", "fields": "id,version",
                       "supportsAllDrives": true, "body": {"name": "renamed"}});
    let prepared = drive()
        .prepare(&http, "fixture-google-drive", "files.update", input)
        .await
        .expect("the pin matches");
    let sent: Sent = Arc::default();
    let _ = prepared
        .execute(Box::new(Recorder {
            sent: sent.clone(),
            answer: json!({"id": "fixture-shared-1", "version": "8"}),
        }))
        .await;
    let sent = sent.lock().unwrap();
    let (_, _, patch_query, _) = &sent[0];
    let flag = ("supportsAllDrives".to_owned(), "true".to_owned());
    assert!(
        patch_query.contains(&flag),
        "the PATCH carries the flag: {patch_query:?}"
    );
    // Today's state, pinned: a guard value is required (`lib.rs` refuses an
    // absent one), so an optional `supportsAllDrives` cannot reach the
    // preflight, and the guide says `files.update` on a shared-drive file is
    // not supported yet. When optional preflight values land, map the flag in
    // `operations.json` and flip this assertion to `contains`.
    let calls = http.calls.lock().unwrap();
    let (_, preflight_query) = &calls[0];
    assert!(
        !preflight_query.contains(&flag),
        "the preflight now carries supportsAllDrives: story:catalog-guard-optional-preflight-values \
         has landed; flip this case to require it and drop the guide's shared-drive limit: \
         {preflight_query:?}"
    );
    assert_eq!(
        preflight_query,
        &[("fields".to_owned(), "id,version".to_owned())],
        "the preflight's query until story:catalog-guard-optional-preflight-values"
    );
    let guide = guide();
    let writes = writes_section(&guide);
    assert!(
        writes.contains("Shared-drive files are not supported by `files.update` yet")
            && writes.contains("story:catalog-guard-optional-preflight-values"),
        "the guide's Writes section must state the shared-drive limit and name \
         story:catalog-guard-optional-preflight-values"
    );
}

/// The engine sends an update's `addParents` and `removeParents` query values
/// and a body's `trashed` and `inheritedPermissionsDisabled` members to Drive
/// unchanged: one `files.update` moves the file to another folder, trashes it
/// and removes every inherited grant on it. The approval an issuer is shown
/// carries instance, operation, connection and descriptor revision only
/// (`crates/connectors-host/src/local/owner/approval_issuance.rs`), so the words
/// an agent and an issuer read are the selection description ("Change one
/// file's metadata") and the guide's Writes section ("Three writes change Drive
/// metadata"; "Metadata only"). Those have to say that `files.update` also
/// moves, trashes and changes access, since the guard pins `version` only.
#[tokio::test]
async fn files_update_says_it_moves_trashes_and_changes_access() {
    let http = scripted(200, &json!({"id": "fixture-doc-1", "version": "7"}));
    let body = json!({"trashed": true, "inheritedPermissionsDisabled": true});
    let input = json!({"fileId": "fixture-doc-1", "version": "7", "fields": "id,version",
                       "addParents": "fixture-elsewhere", "removeParents": "fixture-here",
                       "body": body});
    let prepared = drive()
        .prepare(&http, "fixture-google-drive", "files.update", input)
        .await
        .expect("the pin matches");
    let sent: Sent = Arc::default();
    let _ = prepared
        .execute(Box::new(Recorder {
            sent: sent.clone(),
            answer: json!({"id": "fixture-doc-1", "version": "8"}),
        }))
        .await;
    {
        let sent = sent.lock().unwrap();
        let (_, _, query, sent_body) = &sent[0];
        assert!(query.contains(&("addParents".to_owned(), "fixture-elsewhere".to_owned())));
        assert!(query.contains(&("removeParents".to_owned(), "fixture-here".to_owned())));
        assert_eq!(*sent_body, body, "the body reaches Drive as supplied");
    }
    let description = drive()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "files.update")
        .unwrap()
        .description;
    let writes = writes_section(&guide());
    let missing: Vec<&str> = [
        "addParents",
        "removeParents",
        "trashed",
        "inheritedPermissionsDisabled",
    ]
    .into_iter()
    .filter(|name| !writes.contains(&format!("`{name}`")) && !description.contains(name))
    .collect();
    assert!(
        missing.is_empty(),
        "files.update moves, trashes and changes access, and neither its description \
         ({description:?}) nor the guide's Writes section names {missing:?}"
    );
}

/// The guide: "an update without `fields` is refused as `invalid_input` before
/// any request", and the selection: "fields ... must include version". An
/// agent reads the input schema from `connectors operations describe`; that
/// schema has to require what the runtime refuses without.
#[test]
fn files_update_declares_fields_required() {
    let update = drive()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "files.update")
        .unwrap();
    let required: Vec<String> =
        serde_json::from_value(update.input_schema["required"].clone()).unwrap();
    // The selection declares `required: ["fields"]`
    // (story:catalog-selection-required-parameters), so the schema an agent
    // reads requires what the runtime refuses without.
    assert_eq!(
        required,
        ["body", "fields", "fileId", "version"],
        "files.update's declared input schema must require `fields` \
         (story:catalog-selection-required-parameters)"
    );
    assert!(
        !guide().contains("does not yet mark `fields` required"),
        "the guide still says files.update's schema does not require `fields`"
    );
}
