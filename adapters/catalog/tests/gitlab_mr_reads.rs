//! The two GitLab merge-request list reads in the shipped selection set,
//! `merge_request.diffs` and `merge_request.discussions`, driven through the
//! engine over the committed bundle with a recording transport: their
//! declarations, the exact request each sends, a walk to a short page with
//! GitLab's body unchanged, the per_page bound refused before any request,
//! and a merge request GitLab does not find, or does not let the token read,
//! refused by name. Fixtures are hand-written in the pinned document's shapes
//! (`APIEntitiesDiff`, `APIEntitiesDiscussion` with `APIEntitiesNote`) with
//! synthetic ids.
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
fn answers(responses: Vec<(u16, Value)>) -> Reads {
    Reads {
        responses: Mutex::new(
            responses
                .into_iter()
                .map(|(status, body)| HttpResponse {
                    status,
                    headers: Default::default(),
                    body: serde_json::to_vec(&body).unwrap(),
                })
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn reads(bodies: Vec<Value>) -> Reads {
    answers(bodies.into_iter().map(|body| (200, body)).collect())
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
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = list
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    pairs.sort();
    pairs
}
fn segments(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}
fn sorted(query: &[(String, String)]) -> Vec<(String, String)> {
    let mut query = query.to_vec();
    query.sort();
    query
}
/// One file's diff, shaped as the pinned `APIEntitiesDiff`.
fn diff(path: &str, new_file: bool) -> Value {
    json!({
        "diff": if new_file { "@@ -0,0 +1 @@\n+new\n" } else { "@@ -1 +1 @@\n-old\n+new\n" },
        "collapsed": false, "too_large": false,
        "new_path": path, "old_path": path,
        "a_mode": if new_file { "0" } else { "100644" }, "b_mode": "100644",
        "new_file": new_file, "renamed_file": false, "deleted_file": false,
        "generated_file": false
    })
}
/// One discussion, shaped as the pinned `APIEntitiesDiscussion`: a resolvable
/// thread on a diff line, or an individual note.
fn discussion(id: &str, resolvable: bool, resolved: bool, bodies: &[&str]) -> Value {
    let notes: Vec<Value> = bodies
        .iter()
        .enumerate()
        .map(|(n, body)| {
            json!({"id": 900 + n, "type": if resolvable { json!("DiffNote") } else { Value::Null },
                   "body": body, "author": {"id": 11, "username": "fixture-user",
                   "name": "Fixture User", "state": "active"},
                   "created_at": "2026-10-10T08:00:00.000Z",
                   "updated_at": "2026-10-10T08:00:00.000Z",
                   "system": false, "noteable_id": 501, "noteable_type": "MergeRequest",
                   "project_id": 7, "resolvable": resolvable, "resolved": resolved,
                   "noteable_iid": 7})
        })
        .collect();
    json!({"id": id, "individual_note": !resolvable, "resolvable": resolvable,
           "resolved": resolved, "notes": notes})
}

/// Reads one paged list through `id` until a page shorter than `per_page`,
/// returning every item and the number of pages read.
async fn walk(engine: &Engine, http: &Reads, id: &str, input: Value) -> (Vec<Value>, usize) {
    let per_page = input["per_page"].as_u64().unwrap() as usize;
    let mut items = Vec::new();
    let mut page = 1;
    loop {
        let mut input = input.clone();
        input["page"] = json!(page);
        let result = engine.read(http, "one", id, input).await.unwrap();
        assert_eq!(result["status"], 200);
        let body = result["body"].as_array().unwrap().clone();
        let short = body.len() < per_page;
        items.extend(body);
        if short {
            return (items, page);
        }
        page += 1;
        assert!(page <= 5, "walk did not stop");
    }
}

/// Both are declared reads, not writes, from the source operations the story
/// names, with every parameter the pinned source declares and the path
/// parameters required.
#[test]
fn merge_request_list_reads_are_declared_reads_with_their_source_parameters() {
    let engine = engine();
    let selections = shipped();
    let reads = engine.declarations(&[Effect::Read]);
    for (id, operation_id, parameters, required) in [
        (
            "merge_request.diffs",
            "getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs",
            &["id", "merge_request_iid", "page", "per_page", "unidiff"][..],
            &["id", "merge_request_iid"][..],
        ),
        (
            "merge_request.discussions",
            "getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions",
            &["id", "noteable_id", "page", "per_page"][..],
            &["id", "noteable_id"][..],
        ),
    ] {
        let selection = selections
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not shipped"));
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        let declaration = reads
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not a declared read"));
        let mut properties: Vec<&str> = declaration.input_schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        properties.sort();
        let mut expected = parameters.to_vec();
        expected.sort();
        assert_eq!(properties, expected, "`{id}` parameters");
        let mut declared: Vec<&str> = declaration.input_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        declared.sort();
        assert_eq!(declared, required, "`{id}` required");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
    }
    let writes = engine.declarations(&[Effect::Write]);
    for id in ["merge_request.diffs", "merge_request.discussions"] {
        assert!(!writes.iter().any(|o| o.id == id), "`{id}` is a write");
    }
}

/// `merge_request.diffs` sends `unidiff` to the merge request's diffs route
/// and walks its per-file diffs to a short page, each returned unchanged:
/// the per-file changes `gitlab.mr.changes` gives.
#[tokio::test]
async fn merge_request_diffs_sends_unidiff_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([diff("src/lib.rs", false), diff("src/new.rs", true)]);
    let second = json!([diff("README.md", false)]);
    let http = reads(vec![first.clone(), second.clone()]);
    let (items, pages) = walk(
        &engine,
        &http,
        "merge_request.diffs",
        json!({"id": "org/project", "merge_request_iid": 7, "unidiff": true, "per_page": 2}),
    )
    .await;
    assert_eq!(pages, 2);
    let mut expected = first.as_array().unwrap().clone();
    expected.extend(second.as_array().unwrap().clone());
    assert_eq!(items, expected);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    for (call, page) in calls.iter().zip(["1", "2"]) {
        assert_eq!(
            call.0,
            segments(&["projects", "org/project", "merge_requests", "7", "diffs"])
        );
        assert_eq!(
            sorted(&call.1),
            pairs(&[("unidiff", "true"), ("page", page), ("per_page", "2")]),
            "page {page}"
        );
    }
}

/// `merge_request.discussions` lists every discussion of merge request
/// `noteable_id` (its IID) with its notes, resolvable state and id, the id
/// `merge_request.discussion.get`, `.reply` and `.resolve` take, and walks to
/// a short page.
#[tokio::test]
async fn merge_request_discussions_lists_threads_with_notes_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([
        discussion(
            "6a9c1750b37d513a43987b574953fceb50b03ce7",
            true,
            false,
            &["Why 3 retries?", "Matches the provider's limit."]
        ),
        discussion(
            "87805b7c09016a7058e91bdbe7b29d1f284a39e6",
            false,
            false,
            &["Looks good."]
        ),
    ]);
    let second = json!([discussion(
        "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678",
        true,
        true,
        &["Rename this."]
    )]);
    let http = reads(vec![first.clone(), second.clone()]);
    let (items, pages) = walk(
        &engine,
        &http,
        "merge_request.discussions",
        json!({"id": 7, "noteable_id": 12, "per_page": 2}),
    )
    .await;
    assert_eq!(pages, 2);
    let mut expected = first.as_array().unwrap().clone();
    expected.extend(second.as_array().unwrap().clone());
    assert_eq!(items, expected);
    assert_eq!(
        items[0]["notes"][1]["body"],
        "Matches the provider's limit."
    );
    assert_eq!(items[2]["resolved"], true);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    for (call, page) in calls.iter().zip(["1", "2"]) {
        assert_eq!(
            call.0,
            segments(&["projects", "7", "merge_requests", "12", "discussions"])
        );
        assert_eq!(
            sorted(&call.1),
            pairs(&[("page", page), ("per_page", "2")]),
            "page {page}"
        );
    }
}

/// Both hold `per_page` to GitLab's cap of 100 and take only integers there,
/// and need the merge request's IID; each refusal comes before any request.
#[tokio::test]
async fn merge_request_list_reads_refuse_bad_input_before_any_request() {
    let engine = engine();
    let base = |id: &str| match id {
        "merge_request.diffs" => json!({"id": "org/project", "merge_request_iid": 7}),
        _ => json!({"id": "org/project", "noteable_id": 7}),
    };
    for id in ["merge_request.diffs", "merge_request.discussions"] {
        for per_page in [json!(0), json!(101), json!("101"), json!(true)] {
            let http = reads(vec![]);
            let mut input = base(id);
            input["per_page"] = per_page.clone();
            let error = engine.read(&http, "one", id, input).await.unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {per_page}");
            assert!(http.calls.lock().unwrap().is_empty(), "`{id}` {per_page}");
        }
        let http = reads(vec![]);
        let error = engine
            .read(&http, "one", id, json!({"id": "org/project"}))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` without IID");
        assert!(http.calls.lock().unwrap().is_empty(), "`{id}` without IID");
    }
}

/// GitLab answers `404 Not found` for a merge request the project does not
/// hold and `403 Forbidden` for one the token may not read; both reads refuse
/// them by name after one request.
#[tokio::test]
async fn a_merge_request_gitlab_refuses_is_refused_by_name() {
    let engine = engine();
    let input = |id: &str| match id {
        "merge_request.diffs" => json!({"id": "org/project", "merge_request_iid": 999}),
        _ => json!({"id": "org/project", "noteable_id": 999}),
    };
    for id in ["merge_request.diffs", "merge_request.discussions"] {
        for (status, message, code) in [
            (404, "404 Not found", ErrorCode::NotFound),
            (403, "403 Forbidden", ErrorCode::Forbidden),
        ] {
            let http = answers(vec![(status, json!({"message": message}))]);
            let error = engine
                .read(&http, "one", id, input(id))
                .await
                .expect_err("a refusal");
            assert_eq!(error.code, code, "`{id}` {status}");
            assert_eq!(http.calls.lock().unwrap().len(), 1, "`{id}` {status}");
        }
    }
}
