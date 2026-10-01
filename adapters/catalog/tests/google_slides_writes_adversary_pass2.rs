//! Adversary pass 2 against the guarded Google Slides writes: the corrected
//! guide's write-instance route, and the `batchUpdate` request kinds whose
//! effect the guide and the selection's description characterise. Driven
//! through the engine against a scripted provider and from the guide's own
//! JSON blocks. No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

const DECK: &str = "fixture-deck-1";
const REVISION: &str = "fixture-revision-2";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-google-slides.md")).unwrap()
}
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn selections() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("providers/google-slides/operations.json")).unwrap(),
    )
    .unwrap()
}
fn description(id: &str) -> String {
    selections()["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|operation| operation["id"] == id)
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn engine() -> Engine {
    let selections: Vec<Selection> =
        serde_json::from_value(selections()["operations"].clone()).unwrap();
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
/// A JSON block of the guide, by a string it contains.
fn guide_block(marker: &str) -> Value {
    let guide = guide();
    let block = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(marker))
        .unwrap_or_else(|| panic!("the guide has no JSON block with {marker}"));
    serde_json::from_str(block).unwrap()
}

struct Reads(Mutex<VecDeque<HttpResponse>>);
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, _path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
type Sent = Arc<Mutex<Vec<Value>>>;
struct Send {
    sent: Sent,
    response: HttpResponse,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Send {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        _path: &[&str],
        _query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        assert_eq!(method, WriteMethod::Post);
        self.sent.lock().unwrap().push(body.clone());
        Ok(self.response)
    }
}
/// One guarded `batchUpdate` of `requests` at the pinned revision, answered
/// `acknowledgement`; returns the outcome and the body that reached Google.
async fn batch_update(requests: Value, acknowledgement: Value) -> (WriteOutcome<Value>, Value) {
    let input = json!({"presentationId": DECK, "body": {
        "requests": requests, "writeControl": {"requiredRevisionId": REVISION}}});
    let http = Reads(Mutex::new(VecDeque::from([response(
        json!({"presentationId": DECK, "revisionId": REVISION}),
    )])));
    let prepared = engine()
        .prepare(
            &http,
            "fixture-google-slides",
            "presentations.batchUpdate",
            input,
        )
        .await
        .expect("the current revision passes the preflight");
    let sent: Sent = Arc::new(Mutex::new(Vec::new()));
    let outcome = prepared
        .execute(Box::new(Send {
            sent: sent.clone(),
            response: response(acknowledgement),
        }))
        .await;
    let body = sent.lock().unwrap().pop().expect("one POST");
    (outcome, body)
}

/// The corrected guide: "Then `connections connect` that instance with the
/// Google client file." Connecting from Google's downloaded client file is
/// `story:cli-oauth-loopback-acquisition` (a `depends_on` of this story), and
/// its flow refuses the file with `protected_entry_unavailable` before any
/// browser step unless the profile's `authorize_url` equals the file's
/// `auth_uri` byte for byte (`crates/connectors-host/src/local/oauth.rs:160`
/// on `impl/cli-oauth-loopback-acquisition` 839cb135c). Google's installed
/// client file carries `https://accounts.google.com/o/oauth2/auth`; that
/// branch's own guide says so and moved `docs/local-catalog-provider.md`'s
/// example to it. The write instance the guide prescribes is the read block
/// with the write values applied, so it inherits the read block's
/// `authorize_url`.
#[test]
fn the_documented_write_instance_can_connect_from_the_google_client_file() {
    const GOOGLE_CLIENT_FILE_AUTH_URI: &str = "https://accounts.google.com/o/oauth2/auth";
    let mut config = guide_block("\"provider\": \"google-slides\"");
    let values = guide_block("\"instance\": \"google-slides-write\"");
    config["instance"] = values["instance"].clone();
    config["auth"]["minimum_scopes"] = values["auth"]["minimum_scopes"].clone();
    config["auth"]["requested_scopes"] = values["auth"]["requested_scopes"].clone();
    assert!(
        flat(&guide()).contains("`connections connect` that instance with the Google client file"),
        "the guide no longer routes the write instance through the Google client file"
    );
    assert_eq!(
        config["auth"]["authorize_url"], GOOGLE_CLIENT_FILE_AUTH_URI,
        "the write instance's authorize_url differs from the auth_uri of Google's client file, \
         so `connections connect` with that file is refused before consent"
    );
}

/// The selection set tells an agent that a thumbnail's `contentUrl` gives
/// anyone holding it the requester's access and must not be shared
/// (`presentations.pages.getThumbnail`). The same set's `batchUpdate` accepts
/// `createImage`, `replaceImage` and `replaceAllShapesWithImage` with a URL,
/// which the pinned document says Google "saved with the image, and exposed
/// through the Image.source_url field" to every reader of the target deck. The
/// engine forwards such a URL verbatim and reports the write applied; neither
/// the `batchUpdate` description nor the guide says that an image URL is
/// published into the deck, so the credential is shared by the one route the
/// set offers for placing another deck's page into this one.
#[tokio::test]
async fn an_image_url_published_by_batch_update_is_named_where_the_thumbnail_warning_is() {
    let thumbnail = description("presentations.pages.getThumbnail");
    assert!(thumbnail.contains("treat it as a credential and do not log or share it"));
    let content_url = "https://lh7-rt.googleusercontent.com/slidesz/fixture-thumbnail-grant";
    let requests = json!([{"createImage": {"objectId": "fixture-image-1", "url": content_url,
        "elementProperties": {"pageObjectId": "p"}}}]);
    let (outcome, body) = batch_update(
        requests.clone(),
        json!({"presentationId": DECK, "replies": [{"createImage": {"objectId": "fixture-image-1"}}]}),
    )
    .await;
    assert!(matches!(outcome, WriteOutcome::Applied(Ok(_))));
    assert_eq!(
        body["requests"], requests,
        "the URL reached Google verbatim"
    );
    let batch = description("presentations.batchUpdate");
    let guide = flat(&guide());
    let names = |text: &str| text.contains("sourceUrl") || text.contains("source_url");
    assert!(
        names(&batch) || names(&guide),
        "neither the batchUpdate description nor the guide says an image URL is saved into the \
         deck as Image.sourceUrl, readable by every viewer, while getThumbnail's description \
         forbids sharing that URL"
    );
}

/// The guide says Google "applies them all or none" and the description says
/// `batchUpdate` applies `body.requests` "atomically". The pinned document's
/// `BatchUpdatePresentationResponse.commentUpdateState` says otherwise for
/// the comment requests the same document offers (`insertComment`,
/// `deleteComment`, `addCommentReply`, `updateCommentPost`): a 200 may carry
/// `ALL_FAILED_UNKNOWN_REASON`, the other requests applied and the comment
/// updates not. The engine reports that answer applied, and nothing tells the
/// caller to read `commentUpdateState`.
#[tokio::test]
async fn a_batch_whose_comment_updates_failed_is_not_reported_as_all_applied() {
    let document: Value = serde_json::from_slice(
        &fs::read(root().join("../google/upstream/slides/slides-api.json")).unwrap(),
    )
    .unwrap();
    let states =
        &document["schemas"]["BatchUpdatePresentationResponse"]["properties"]["commentUpdateState"];
    assert!(
        states["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("ALL_FAILED_UNKNOWN_REASON"))
    );
    assert!(flat(&guide()).contains("applies them all or none"));
    let (outcome, _) = batch_update(
        json!([{"createSlide": {"objectId": "fixture-slide-3"}},
               {"insertComment": {"objectId": "fixture-slide-3", "content": "fixture"}}]),
        json!({"presentationId": DECK, "replies": [{"createSlide": {"objectId": "fixture-slide-3"}}, {}],
               "commentUpdateState": "ALL_FAILED_UNKNOWN_REASON"}),
    )
    .await;
    let applied = matches!(outcome, WriteOutcome::Applied(Ok(_)));
    let documented = flat(&guide()).contains("commentUpdateState")
        || description("presentations.batchUpdate").contains("commentUpdateState");
    assert!(
        !applied || documented,
        "a 200 whose commentUpdateState is ALL_FAILED_UNKNOWN_REASON is reported applied, while \
         the guide says Google applies the requests all or none and never names commentUpdateState"
    );
}
