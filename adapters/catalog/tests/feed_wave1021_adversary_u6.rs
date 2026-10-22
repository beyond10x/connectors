//! Adversarial pass over unit U6 of wave 20261021 (`c1866ffa0..caf75e173`): the deferred
//! judgement of a watermark a read cannot resume, the seen set kept at the kept instant only,
//! and the documents U6 wrote about both. The cases drive the shipped GitLab feed
//! (`providers/gitlab/operations.json`) and the fixture feeds under `tests/feed/` with
//! in-process answers; no live provider is called. The case on a Jira first read started again
//! is withdrawn: `jira.md` and `confluence-v1.md` no longer claim that such a read gets past a
//! full re-read window.
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, fs, path::Path, sync::Mutex};

const INSTANCE: &str = "adversary-instance";
const GITLAB_INSTANCE: &str = "gitlab-feed-instance";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn json_file(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}

fn respond(status: u16, body: &Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(body).unwrap(),
    }
}

/// Answers replayed in order; every path read is recorded.
struct Replay {
    answers: Mutex<VecDeque<(u16, Value)>>,
    paths: Mutex<Vec<Vec<String>>>,
}

impl Replay {
    fn new(answers: Vec<(u16, Value)>) -> Self {
        Self {
            answers: Mutex::new(answers.into()),
            paths: Mutex::new(Vec::new()),
        }
    }

    fn reads(&self) -> usize {
        self.paths.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Replay {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.paths
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
        let (status, body) = self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(respond(status, &body))
    }
}

// ---- the generic time fixture ------------------------------------------------------------------

fn time_feed() -> Value {
    json_file("tests/feed/time.operations.json")["feed"].clone()
}

fn fixture_engine(feed: &Value) -> Engine {
    let bytes = fs::read(root().join("tests/feed/api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let bundle = Bundle {
        provider: "feed-fixture".into(),
        source: ingest("api.json", &bytes).unwrap(),
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    };
    let feed: Declaration = serde_json::from_value(feed.clone()).unwrap();
    Engine::with_feed(&bundle, "/v1", &[], Some(&feed)).unwrap()
}

fn room(id: &str) -> Value {
    json!({"room": {"id": id, "title": "Room", "type": "room", "access": "open"}})
}

fn message(id: &str, at: &str) -> Value {
    json!({"id": id, "version": "1", "created": at, "edited": at, "text": "body"})
}

// ---- the shipped GitLab feed -------------------------------------------------------------------

fn gitlab_engine() -> Engine {
    let feed: Declaration =
        serde_json::from_value(json_file("providers/gitlab/operations.json")["feed"].clone())
            .unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::with_feed(&bundle, "/api/v4", &[], Some(&feed)).unwrap()
}

fn project(id: u64) -> Value {
    let mut project = json_file("tests/feed/gitlab/project.json");
    project["id"] = json!(id);
    project
}

fn merge_request(iid: u64, at: &str) -> Value {
    let mut request = json_file("tests/feed/gitlab/merge_request.json");
    request["id"] = json!(50_000 + iid);
    request["iid"] = json!(iid);
    request["updated_at"] = json!(at);
    request
}

// ---- the cases ---------------------------------------------------------------------------------

/// U6's Ceilings row (`contracts/catalog/v1alpha1/semantics.md:186`) said a page's watermark
/// passes 16 KiB only by "the pairs of items sharing its kept instant". The watermark also
/// carries the container id, which the items input bounds only by `minLength: 1` (`src/feed.rs`,
/// the items input schema). The row and the ceiling's message now name it: a first read of a
/// room with a 13,000-byte id whose listing is empty holds no pair at all, and is `unavailable`
/// for its container id, after the lookup and the listing.
#[tokio::test]
async fn adversary_u6_a_container_id_alone_can_pass_the_watermark_ceiling_as_the_row_says() {
    let engine = fixture_engine(&time_feed());
    let container = "r".repeat(13_000);
    let http = Replay::new(vec![
        (200, room(&container)),
        (200, json!({"messages": []})),
    ]);
    let answer = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": container, "limit": 10}),
        )
        .await
        .map(|page| page["items"].as_array().map(Vec::len))
        .map_err(|error| (error.code, error.message));
    assert_eq!(
        answer,
        Err((
            ErrorCode::Unavailable,
            "the watermark would exceed its ceiling; the container id is too long or too many \
             items share one instant"
                .to_owned()
        ))
    );
    assert_eq!(http.reads(), 2, "the lookup and the listing");
}

/// The family (`contracts/datasources/feed/v1alpha1/semantics.md:161-163`, table at :206): a
/// watermark a binding "recognises as foreign, malformed or no longer resumable is refused as
/// `stale_cursor`, and the consumer starts again with a first read". U6's Watermark scope row
/// (`contracts/catalog/v1alpha1/semantics.md:183`) names one such watermark, one holding
/// `max_limit` pairs without `page`, and answers it `unavailable`. The shipped GitLab feed issues
/// that watermark itself when 100 merge requests share one `updated_at`; every read from it is
/// refused from the watermark alone, whatever GitLab answers.
#[tokio::test]
async fn adversary_u6_a_watermark_the_gitlab_feed_issued_and_cannot_resume_is_stale_cursor() {
    let engine = gitlab_engine();
    let at = "2026-10-06T09:00:00.000Z";
    let full: Vec<Value> = (1..=100).map(|iid| merge_request(iid, at)).collect();
    let http = Replay::new(vec![(200, project(1001)), (200, json!(full))]);
    let first = engine
        .read(
            &http,
            GITLAB_INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 100}),
        )
        .await
        .unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 100, "control");
    assert_eq!(first["complete"], json!(false), "control");

    // GitLab answers the first read the resume is judged on with the same 100.
    let http = Replay::new(vec![(200, project(1001)), (200, json!(full))]);
    let resumed = engine
        .read(
            &http,
            GITLAB_INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 100, "watermark": first["next_watermark"]}),
        )
        .await
        .map(|page| page["items"].as_array().unwrap().len())
        .map_err(|error| (error.code, error.message));
    assert!(
        matches!(resumed, Err((ErrorCode::StaleCursor, _))),
        "a watermark this binding issued and recognises as no longer resumable answered \
         {resumed:?} after {} provider reads",
        http.reads()
    );
}

/// The family (`contracts/datasources/feed/v1alpha1/semantics.md:205`): an unknown container and
/// one the connection cannot read answer `not_found` "indistinguishably". GitLab shows project
/// 1001 and answers its merge request list 404; an unknown project's lookup answers 404.
#[tokio::test]
async fn adversary_u6_a_merge_request_list_gitlab_answers_404_reads_as_an_unknown_project() {
    let engine = gitlab_engine();
    let unknown = Replay::new(vec![(404, json!({"message": "404 Project Not Found"}))]);
    let unknown = engine
        .read(
            &unknown,
            GITLAB_INSTANCE,
            ITEMS,
            json!({"container": "1002", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(unknown.code, ErrorCode::NotFound, "control");

    let listing = Replay::new(vec![
        (200, project(1001)),
        (404, json!({"message": "404 Not found"})),
    ]);
    let refused = engine
        .read(
            &listing,
            GITLAB_INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (&refused.code, &refused.message),
        (&unknown.code, &unknown.message),
        "a project whose merge request list GitLab refuses is told apart from an unknown one"
    );
}

/// U6's Time row (`contracts/catalog/v1alpha1/semantics.md:181`): "without `page` a resume whose
/// one answer is full and holds no unseen item is `unavailable`" (`src/feed.rs:1505`). 101
/// messages changed within one minute, read at minute precision through a parameter without
/// `page`: the first read returns 100, and the resume, whose one answer of 100 re-reads only
/// those, is `unavailable`, never an empty incomplete page under the same watermark. No case
/// U6 added reaches that branch.
#[tokio::test]
async fn adversary_u6_a_full_answer_of_held_items_without_page_is_unavailable() {
    let mut feed = time_feed();
    feed["items"]["position"]["value"]["precision"] = json!("minute");
    let engine = fixture_engine(&feed);
    let messages: Vec<Value> = (0..101)
        .map(|n| {
            message(
                &format!("m{n:03}"),
                &format!("2026-10-06T09:00:{:02}.{:03}Z", n / 2, (n % 2) * 500),
            )
        })
        .collect();
    let first = Replay::new(vec![
        (200, room("room")),
        (200, json!({"messages": messages[..100].to_vec()})),
    ]);
    let first = engine
        .read(
            &first,
            INSTANCE,
            ITEMS,
            json!({"container": "room", "limit": 100}),
        )
        .await
        .unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 100, "control");

    // The provider re-reads the minute, oldest first, 100 an answer.
    let resume = Replay::new(vec![
        (200, room("room")),
        (200, json!({"messages": messages[..100].to_vec()})),
    ]);
    let resumed = engine
        .read(
            &resume,
            INSTANCE,
            ITEMS,
            json!({"container": "room", "limit": 100, "watermark": first["next_watermark"]}),
        )
        .await
        .map(|page| {
            (
                page["items"].as_array().unwrap().len(),
                page["complete"].clone(),
                page["next_watermark"] == first["next_watermark"],
            )
        })
        .map_err(|error| (error.code, error.message));
    assert_eq!(
        resumed,
        Err((
            ErrorCode::Unavailable,
            "more items share one minute than one provider page holds".to_owned()
        ))
    );
}
