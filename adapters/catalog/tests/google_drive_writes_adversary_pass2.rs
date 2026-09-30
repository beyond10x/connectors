//! Adversary pass 2 against the guarded Google Drive metadata writes: the
//! corrected descriptions and guide read against the pinned Discovery document
//! and driven through the engine against a scripted provider. No network, no
//! credential.
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

struct Scripted {
    responses: Mutex<VecDeque<HttpResponse>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
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
fn scripted(answers: &[Value]) -> Scripted {
    Scripted {
        responses: Mutex::new(answers.iter().map(|a| json_response(200, a)).collect()),
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
fn writes_section() -> String {
    let guide = guide();
    let start = guide
        .find("\n## Writes\n")
        .expect("the guide's Writes section");
    let rest = &guide[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
    rest[..end].to_owned()
}
/// The `operations describe` description an agent reads for one write.
fn description(id: &str) -> String {
    drive()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
        .description
}
/// Prepare and execute one write; the requests the provider saw, in order.
async fn send(
    id: &str,
    input: Value,
    preflight: &[Value],
) -> (
    std::result::Result<(), connectors_core::Error>,
    Vec<(WriteMethod, Vec<String>, Vec<(String, String)>, Value)>,
) {
    let http = scripted(preflight);
    let sent: Sent = Arc::default();
    let prepared = match drive()
        .prepare(&http, "fixture-google-drive", id, input)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => return (Err(error), Vec::new()),
    };
    let _ = prepared
        .execute(Box::new(Recorder {
            sent: sent.clone(),
            answer: json!({"id": "fixture-new-1", "version": "8"}),
        }))
        .await;
    let sent = sent.lock().unwrap().clone();
    (Ok(()), sent)
}

/// The pinned Discovery document, `File.parents`: "If not specified as part of
/// a create request, the file is placed directly in the user's My Drive folder.
/// If not specified as part of a copy request, the file inherits any
/// discoverable parent of the source file." The engine sends a copy whose body
/// names no parent, so the copy lands in the source's folder and takes that
/// folder's inherited access. The description says only "the body names the
/// copy and its parents" and the guide's table says `body.parents` "places the
/// copy"; an agent that omits `parents` expecting My Drive, as a create gives,
/// writes into a folder it never named, and the approval shows no folder at all.
#[tokio::test]
async fn files_copy_says_where_a_copy_without_parents_lands() {
    let body = json!({"name": "fixture copy"});
    let (prepared, sent) = send(
        "files.copy",
        json!({"fileId": "fixture-source-1", "body": body}),
        &[],
    )
    .await;
    prepared.expect("a copy without parents is prepared");
    assert_eq!(sent.len(), 1, "exactly one write is sent");
    assert_eq!(sent[0].3, body, "the body reaches Drive without a parent");
    let description = description("files.copy");
    let writes = writes_section();
    let says = [
        "discoverable parent",
        "parent of the source",
        "source's parent",
        "source's folder",
        "folder of the source",
        "same folder as the source",
    ]
    .iter()
    .any(|phrase| description.contains(phrase) || writes.contains(phrase));
    assert!(
        says,
        "a copy without body.parents lands in the source file's folder (pinned Discovery, \
         File.parents), and neither the files.copy description ({description:?}) nor the \
         guide's Writes section says so"
    );
}

/// The pinned Discovery document, `File.name`: "This isn't necessarily unique
/// within a folder." A create or copy that names a file after one already in
/// the target folder makes a second file of that name; nothing is refused or
/// replaced, and a later `files.list` by `name` finds both. The guide says a
/// repeated create "makes another file" but never that a name does not identify
/// one, and the copy description says "the body names the copy".
#[test]
fn drive_writes_say_a_name_is_not_unique_in_a_folder() {
    let writes = writes_section();
    let says = [
        "not necessarily unique",
        "not unique",
        "isn't unique",
        "same name",
        "duplicate",
    ]
    .iter()
    .any(|phrase| writes.contains(phrase));
    assert!(
        says,
        "Drive keeps two files of one name in one folder (pinned Discovery, File.name), \
         and the guide's Writes section does not say a create or copy can make one"
    );
}

/// The corrected description and guide list what each write does "beyond
/// naming and describing a file". The engine forwards every other writable
/// member as well, and the pinned Discovery document gives three more that
/// change who may read, edit or download a file:
///
/// * `body.contentRestrictions` with `readOnly`: "a new revision of the file
///   may not be added, comments may not be added or modified, and the title of
///   the file may not be modified"; with `ownerRestricted` only the owner can
///   lift it (`ContentRestriction`);
/// * `body.downloadRestrictions`: "Download restrictions applied on the file";
/// * `copyComments` on `files.copy`: "Whether to copy the open (unresolved)
///   comments associated with the file", other people's words carried into a
///   copy that may sit under different access.
///
/// A list presented as what the write can do, which omits a lock on the file
/// and a download restriction, tells an issuer the input is safe to approve
/// once the listed members are absent.
#[tokio::test]
async fn drive_writes_name_the_lock_download_and_comment_effects_they_forward() {
    let restrictions = json!({
        "contentRestrictions": [{"readOnly": true, "ownerRestricted": true, "reason": "fixture"}],
        "downloadRestrictions": {"itemDownloadRestriction": {"restrictedForReaders": true}}
    });
    let (prepared, sent) = send(
        "files.update",
        json!({"fileId": "fixture-doc-1", "version": "7", "fields": "id,version",
               "body": restrictions}),
        &[json!({"id": "fixture-doc-1", "version": "7"})],
    )
    .await;
    prepared.expect("the pin matches");
    assert_eq!(
        sent[0].3, restrictions,
        "the lock reaches Drive as supplied"
    );
    let (prepared, sent) = send(
        "files.copy",
        json!({"fileId": "fixture-source-1", "copyComments": true, "body": {}}),
        &[],
    )
    .await;
    prepared.expect("a copy with comments is prepared");
    assert!(
        sent[0]
            .2
            .contains(&("copyComments".to_owned(), "true".to_owned())),
        "copyComments reaches Drive"
    );
    let writes = writes_section();
    let mut missing = Vec::new();
    for (id, names) in [
        (
            "files.update",
            &["contentRestrictions", "downloadRestrictions"][..],
        ),
        (
            "files.create",
            &["contentRestrictions", "downloadRestrictions"][..],
        ),
        (
            "files.copy",
            &[
                "contentRestrictions",
                "downloadRestrictions",
                "copyComments",
            ][..],
        ),
    ] {
        let description = description(id);
        for name in names {
            if !description.contains(name) && !writes.contains(name) {
                missing.push(format!("{id}: {name}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the writes forward these and neither the description nor the guide's Writes \
         section names them: {missing:?}"
    );
}
