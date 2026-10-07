//! story:gitlab-feed-binding: the shipped GitLab selection file binds `datasource.feed/v1alpha1`
//! as data (`adapters/catalog/contracts/feed/v1alpha1/gitlab.md`), and the catalog engine reads
//! it over the committed GitLab bundle. A container is a project the token's user is a member
//! of; an item is one of its merge requests. Every answer here is a GitLab record in the shape
//! of `tests/feed/gitlab/*.json`; the requests the engine sends are recorded and checked. The
//! family's suite runs against the same declaration in
//! `crates/connectors-build/src/gitlab_feed_conformance.rs`.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Effect, Engine,
    feed::{CONTAINERS, Declaration, FEED_CONTRACT, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const PROFILE: &str = "gitlab-merge-requests/1";
const INSTANCE: &str = "gitlab-feed-instance";

type Call = (Vec<String>, Vec<(String, String)>);

struct Reads {
    responses: Mutex<VecDeque<(u16, Value)>>,
    calls: Mutex<Vec<Call>>,
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let mut query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        query.sort();
        self.calls
            .lock()
            .unwrap()
            .push((path.iter().map(|s| s.to_string()).collect(), query));
        let (status, body) = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn answers(responses: Vec<(u16, Value)>) -> Reads {
    Reads {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(Vec::new()),
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recorded(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join("tests/feed/gitlab").join(name)).unwrap())
        .unwrap()
}

fn shipped() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("providers/gitlab/operations.json")).unwrap())
        .unwrap()
}

fn engine() -> Engine {
    let feed: Declaration = serde_json::from_value(shipped()["feed"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::with_feed(&bundle, "/api/v4", &[], Some(&feed)).unwrap()
}

/// A project as GitLab answers it, with the fields the binding reads replaced.
fn project(id: u64, name: &str, visibility: &str) -> Value {
    let mut project = recorded("project.json");
    project["id"] = json!(id);
    project["name_with_namespace"] = json!(name);
    project["visibility"] = json!(visibility);
    project
}

/// A merge request as GitLab lists it, with the fields the binding reads replaced.
fn merge_request(iid: u64, created: &str, updated: &str, description: Value) -> Value {
    let mut record = recorded("merge_request.json");
    record["iid"] = json!(iid);
    record["id"] = json!(50_000 + iid);
    record["created_at"] = json!(created);
    record["updated_at"] = json!(updated);
    record["description"] = description;
    record
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

/// The shipped declaration names GitLab's own operations and is carried by the committed
/// bundle: the engine adds exactly the family's two reads, under the binding's profile. The
/// profile states what GitLab lets it observe (decided 2026-10-06): no deletions, the one kind
/// word `project`, the update time as the revision, and every project private, so the
/// declaration carries no visibility rule and no `url`.
#[test]
fn the_shipped_gitlab_selection_declares_the_feed_family_under_its_profile() {
    let feed = &shipped()["feed"];
    assert_eq!(feed["profile"], PROFILE);
    assert_eq!(
        feed["capabilities"],
        json!({"deletions": "not-observed", "kind": "fixed-word",
               "revision": "update-time", "visibility": "all-private"})
    );
    assert_eq!(feed["containers"]["kind_word"], "project");
    assert!(feed["containers"].get("kind").is_none());
    assert!(feed["containers"].get("visibility").is_none());
    assert_eq!(
        feed["containers"]["cursor"],
        json!({"parameter": "id_after", "last": "/id"})
    );
    assert!(feed["items"].get("url").is_none());
    assert!(feed["items"].get("deleted").is_none());
    assert_eq!(feed["items"]["revision"], feed["items"]["updated_at"]);
    assert_eq!(feed["containers"]["operation_id"], "getApiV4Projects");
    assert_eq!(
        feed["containers"]["lookup"]["operation_id"],
        "getApiV4ProjectsId"
    );
    assert_eq!(
        feed["items"]["operation_id"],
        "getApiV4ProjectsIdMergeRequests"
    );
    let engine = engine();
    let declared = engine.declarations(&[Effect::Read]);
    let ids: Vec<&str> = declared.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, [CONTAINERS, ITEMS]);
    for operation in &declared {
        assert_eq!(operation.contract, FEED_CONTRACT);
        assert_eq!(operation.profile, PROFILE);
    }
    assert_eq!(
        declared[1].input_schema["properties"]["limit"]["maximum"],
        json!(100)
    );
    assert!(engine.declarations(&[Effect::Write]).is_empty());
}

/// `feed.containers` lists the projects the user is a member of, oldest id first, and
/// continues after the last project id a full page answered (`id_after`); a page shorter than
/// the limit is the end. A project is `project`, named with its namespace, and listed `private`
/// whatever GitLab's `visibility` says: a public project may keep its merge requests to its
/// members, which one field cannot tell.
#[tokio::test]
async fn containers_are_member_projects_continued_after_the_last_project_id() {
    let engine = engine();
    let http = answers(vec![
        (
            200,
            json!([
                project(11, "Group / Open", "public"),
                project(12, "Group / Inside", "internal"),
            ]),
        ),
        (
            200,
            json!([
                project(15, "Group / Closed", "private"),
                project(19, "Group / Odd", "secret")
            ]),
        ),
        (200, json!([])),
    ]);
    let first = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap();
    assert_eq!(
        first["containers"],
        json!([
            {"id": "11", "name": "Group / Open", "kind": "project", "visibility": "private"},
            {"id": "12", "name": "Group / Inside", "kind": "project", "visibility": "private"},
        ])
    );
    assert_eq!(first["complete"], json!(false));
    let cursor = first["next_cursor"].as_str().expect("a cursor").to_owned();
    let second = engine
        .read(
            &http,
            INSTANCE,
            CONTAINERS,
            json!({"limit": 2, "cursor": cursor}),
        )
        .await
        .unwrap();
    let visibility: Vec<(&str, &str)> = second["containers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["id"].as_str().unwrap(), c["visibility"].as_str().unwrap()))
        .collect();
    assert_eq!(visibility, [("15", "private"), ("19", "private")]);
    assert_eq!(second["complete"], json!(false));
    let third = engine
        .read(
            &http,
            INSTANCE,
            CONTAINERS,
            json!({"limit": 2, "cursor": second["next_cursor"]}),
        )
        .await
        .unwrap();
    assert_eq!(third["containers"], json!([]));
    assert_eq!(third["complete"], json!(true));
    assert_eq!(third["next_cursor"], Value::Null);
    let listing = |extra: &[(&str, &str)]| {
        let mut query = vec![
            ("membership", "true"),
            ("order_by", "id"),
            ("sort", "asc"),
            ("per_page", "2"),
        ];
        query.extend_from_slice(extra);
        (segments(&["projects"]), pairs(&query))
    };
    assert_eq!(
        *http.calls.lock().unwrap(),
        [
            listing(&[]),
            listing(&[("id_after", "12")]),
            listing(&[("id_after", "19")]),
        ]
    );
}

/// A full page whose last project carries no id cannot be continued; the listing says so
/// rather than claiming it is complete.
#[tokio::test]
async fn a_full_listing_page_without_a_last_id_is_unavailable_not_complete() {
    let engine = engine();
    let mut last = project(12, "Group / Inside", "internal");
    last.as_object_mut().unwrap().remove("id");
    let http = answers(vec![(
        200,
        json!([project(11, "Group / Open", "public"), last]),
    )]);
    let error = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

/// `feed.items` reads the project first, then its merge requests in every state and scope,
/// oldest `updated_at` first. A first read sends no `updated_after`; the next read sends the
/// last returned merge request's `updated_at` exactly as GitLab wrote it, millisecond and all,
/// and skips the merge requests it already returned at that instant. The item is the merge
/// request's `iid`; its revision is its `updated_at`; its body is its Markdown description.
#[tokio::test]
async fn items_are_merge_requests_resumed_from_the_last_updated_at() {
    let engine = engine();
    let at = "2026-10-06T09:00:00.081Z";
    let http = answers(vec![
        (200, project(1001, "Group / Project", "private")),
        (
            200,
            json!([
                merge_request(1, "2026-10-06T08:00:00.000Z", at, json!("First")),
                merge_request(2, "2026-10-06T08:30:00.000Z", at, Value::Null),
            ]),
        ),
        (200, project(1001, "Group / Project", "private")),
        (
            200,
            json!([
                merge_request(1, "2026-10-06T08:00:00.000Z", at, json!("First")),
                merge_request(2, "2026-10-06T08:30:00.000Z", at, Value::Null),
                merge_request(
                    3,
                    "2026-10-06T09:00:01.000Z",
                    "2026-10-06T09:00:01.500Z",
                    json!("Third")
                ),
            ]),
        ),
    ]);
    let first = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(
        first["items"][0],
        json!({
            "id": "1",
            "revision": at,
            "created_at": "2026-10-06T08:00:00.000Z",
            "updated_at": at,
            "author": {"id": "7", "display_name": "Member One"},
            "body": {"representation": "text/markdown", "bytes": 5, "content": "First",
                     "truncated": false, "truncation": []},
            "url": null,
            "parent": null,
            "deleted": false,
        })
    );
    assert_eq!(first["items"][1]["body"]["content"], json!(""));
    assert_eq!(first["complete"], json!(false));
    assert_eq!(first["provenance"]["profile"], json!(PROFILE));
    let second = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 2, "watermark": first["next_watermark"]}),
        )
        .await
        .unwrap();
    let ids: Vec<&str> = second["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["3"]);
    assert_eq!(second["complete"], json!(true));
    let list = |extra: &[(&str, &str)]| {
        let mut query = vec![
            ("order_by", "updated_at"),
            ("sort", "asc"),
            ("state", "all"),
            ("scope", "all"),
        ];
        query.extend_from_slice(extra);
        (
            segments(&["projects", "1001", "merge_requests"]),
            pairs(&query),
        )
    };
    let lookup = (segments(&["projects", "1001"]), Vec::new());
    assert_eq!(
        *http.calls.lock().unwrap(),
        [
            lookup.clone(),
            list(&[("per_page", "2")]),
            lookup,
            list(&[("per_page", "4"), ("updated_after", at)]),
        ]
    );
}

/// A project GitLab does not show this token is `not_found`, and nothing else is read.
#[tokio::test]
async fn a_project_gitlab_does_not_show_is_not_found() {
    let engine = engine();
    let http = answers(vec![(404, json!({"message": "404 Project Not Found"}))]);
    let error = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "77", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(http.calls.lock().unwrap().len(), 1);
}

/// GitLab deletes a merge request outright; the profile observes no deletions, so no item is
/// ever a tombstone, whatever its state. A closed or merged merge request is an ordinary item.
#[tokio::test]
async fn closed_and_merged_merge_requests_are_items_and_none_is_a_tombstone() {
    let engine = engine();
    let mut closed = merge_request(
        4,
        "2026-10-01T08:00:00.000Z",
        "2026-10-02T08:00:00.000Z",
        json!("x"),
    );
    closed["state"] = json!("closed");
    let mut merged = merge_request(
        5,
        "2026-10-01T08:00:00.000Z",
        "2026-10-03T08:00:00.000Z",
        json!("y"),
    );
    merged["state"] = json!("merged");
    let http = answers(vec![
        (200, project(1001, "Group / Project", "public")),
        (200, json!([closed, merged])),
    ]);
    let page = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 10}),
        )
        .await
        .unwrap();
    for item in page["items"].as_array().unwrap() {
        assert_eq!(item["deleted"], json!(false));
        assert!(item["body"].is_object());
    }
    assert_eq!(page["complete"], json!(true));
}
