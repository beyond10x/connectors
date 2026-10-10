//! The four GitLab repository-browsing reads in the shipped selection set,
//! `repository.tree`, `commit.get`, `commit.diff` and `branches.list`, driven
//! through the engine over the committed bundle with a recording transport:
//! their declarations, the exact request each sends, the paged ones walked to
//! a short page, the per_page bound and the withheld keyset cursor refused
//! before any request, and a commit GitLab does not find refused by name.
//! Fixtures are hand-written in the pinned document's shapes
//! (`APIEntitiesTreeObject`, `APIEntitiesCommitDetail`, `APIEntitiesDiff`,
//! `APIEntitiesBranch`) with synthetic ids.
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
fn entry(name: &str, kind: &str, path: &str) -> Value {
    json!({
        "id": format!("7ee0{:0>36}", name.len()),
        "name": name,
        "type": kind,
        "path": path,
        "mode": if kind == "tree" { "040000" } else { "100644" }
    })
}
fn commit(id: &str) -> Value {
    json!({
        "id": id,
        "short_id": &id[..8],
        "parent_ids": ["c0ffee0100000000"],
        "title": format!("fixture commit {id}"),
        "message": format!("fixture commit {id}\n"),
        "author_name": "Fixture Author",
        "author_email": "author@example.test",
        "authored_date": "2026-10-01T07:59:00.000+00:00",
        "committed_date": "2026-10-01T08:00:00.000+00:00",
        "created_at": "2026-10-01T08:00:00.000+00:00",
        "web_url": format!("https://gitlab.example.test/org/project/-/commit/{id}")
    })
}
fn diff(path: &str) -> Value {
    json!({
        "diff": "@@ -1 +1 @@\n-old\n+new\n",
        "collapsed": false, "too_large": false,
        "new_path": path, "old_path": path,
        "a_mode": "100644", "b_mode": "100644",
        "new_file": false, "renamed_file": false, "deleted_file": false,
        "generated_file": false
    })
}
fn branch(name: &str) -> Value {
    json!({
        "name": name,
        "commit": commit("c0ffee0300000000"),
        "merged": false, "protected": name == "main",
        "developers_can_push": false, "developers_can_merge": false,
        "can_push": true, "default": name == "main",
        "web_url": format!("https://gitlab.example.test/org/project/-/tree/{name}")
    })
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

/// All four are declared reads with the source parameters the story names,
/// none is a write, and the paging ones expose offset paging only: the keyset
/// `pagination` and `page_token` are left out of the declaration.
#[test]
fn repository_browsing_reads_are_declared_reads_with_their_source_parameters() {
    let engine = engine();
    let reads = engine.declarations(&[Effect::Read]);
    for (id, parameters, required, absent) in [
        (
            "repository.tree",
            &["id", "ref", "path", "recursive", "page", "per_page"][..],
            &["id"][..],
            &["pagination", "page_token"][..],
        ),
        (
            "commit.get",
            &["id", "sha", "stats"][..],
            &["id", "sha"][..],
            &[][..],
        ),
        (
            "commit.diff",
            &["id", "sha", "unidiff", "page", "per_page"][..],
            &["id", "sha"][..],
            &[][..],
        ),
        (
            "branches.list",
            &["id", "search", "regex", "sort", "page", "per_page"][..],
            &["id"][..],
            &["page_token"][..],
        ),
    ] {
        let declaration = reads
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not a declared read"));
        let properties = declaration.input_schema["properties"].as_object().unwrap();
        for parameter in parameters {
            assert!(properties.contains_key(*parameter), "`{id}` `{parameter}`");
        }
        for parameter in absent {
            assert!(
                !properties.contains_key(*parameter),
                "`{id}` declares withheld `{parameter}`"
            );
        }
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
    for id in [
        "repository.tree",
        "commit.get",
        "commit.diff",
        "branches.list",
    ] {
        assert!(!writes.iter().any(|o| o.id == id), "`{id}` is a write");
    }
}

/// `repository.tree` sends `ref`, `path` and `recursive` to the tree route and
/// returns each entry unchanged; a walk at `per_page=2` reads two pages and
/// stops on the short second one.
#[tokio::test]
async fn repository_tree_sends_ref_path_recursive_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([
        entry("src", "tree", "crates/src"),
        entry("lib.rs", "blob", "crates/src/lib.rs"),
    ]);
    let second = json!([entry("Cargo.toml", "blob", "crates/Cargo.toml")]);
    let http = reads(vec![first.clone(), second.clone()]);
    let (items, pages) = walk(
        &engine,
        &http,
        "repository.tree",
        json!({"id": "org/project", "ref": "v0.3.0", "path": "crates",
               "recursive": true, "per_page": 2}),
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
            segments(&["projects", "org/project", "repository", "tree"])
        );
        assert_eq!(
            sorted(&call.1),
            pairs(&[
                ("ref", "v0.3.0"),
                ("path", "crates"),
                ("recursive", "true"),
                ("page", page),
                ("per_page", "2"),
            ]),
            "page {page}"
        );
    }
}

/// `commit.get` sends the sha as its own path segment, with `stats`, and
/// returns GitLab's commit unchanged.
#[tokio::test]
async fn commit_get_sends_the_sha_and_stats() {
    let engine = engine();
    let mut body = commit("c0ffee0300000000");
    body["stats"] = json!({"additions": 1, "deletions": 1, "total": 2});
    body["status"] = Value::Null;
    let http = reads(vec![body.clone()]);
    let result = engine
        .read(
            &http,
            "one",
            "commit.get",
            json!({"id": 7, "sha": "c0ffee0300000000", "stats": true}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"], body);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        segments(&["projects", "7", "repository", "commits", "c0ffee0300000000"])
    );
    assert_eq!(sorted(&calls[0].1), pairs(&[("stats", "true")]));
}

/// A ref name holding `/` stays one path segment: the commit route is never
/// widened by the value, whatever it contains.
#[tokio::test]
async fn commit_get_keeps_a_slashed_ref_in_one_segment() {
    let engine = engine();
    let http = reads(vec![commit("c0ffee0300000000")]);
    engine
        .read(
            &http,
            "one",
            "commit.get",
            json!({"id": "org/project", "sha": "release/v0.3"}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(
        calls[0].0,
        segments(&[
            "projects",
            "org/project",
            "repository",
            "commits",
            "release/v0.3"
        ])
    );
    assert!(calls[0].1.is_empty());
}

/// GitLab answers `404 Commit Not Found` for a sha the project does not hold;
/// `commit.get` and `commit.diff` refuse it as `not_found` after one request.
#[tokio::test]
async fn a_commit_gitlab_does_not_find_is_refused_as_not_found() {
    let engine = engine();
    for id in ["commit.get", "commit.diff"] {
        let http = answers(vec![(404, json!({"message": "404 Commit Not Found"}))]);
        let error = engine
            .read(
                &http,
                "one",
                id,
                json!({"id": "org/project", "sha": "deadbeef00000000"}),
            )
            .await
            .expect_err("a refusal");
        assert_eq!(error.code, ErrorCode::NotFound, "`{id}`");
        assert_eq!(http.calls.lock().unwrap().len(), 1, "`{id}`");
    }
}

/// `commit.diff` sends `unidiff` to the commit's diff route and walks its
/// paged diff list to a short page.
#[tokio::test]
async fn commit_diff_sends_unidiff_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([diff("a.rs"), diff("b.rs")]);
    let second = json!([diff("c.rs")]);
    let http = reads(vec![first.clone(), second.clone()]);
    let (items, pages) = walk(
        &engine,
        &http,
        "commit.diff",
        json!({"id": "org/project", "sha": "c0ffee0300000000",
               "unidiff": true, "per_page": 2}),
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
            segments(&[
                "projects",
                "org/project",
                "repository",
                "commits",
                "c0ffee0300000000",
                "diff"
            ])
        );
        assert_eq!(
            sorted(&call.1),
            pairs(&[("unidiff", "true"), ("page", page), ("per_page", "2")]),
            "page {page}"
        );
    }
}

/// `branches.list` sends `search` and `sort` to the branches route and walks
/// to a short page; each branch carries its head commit.
#[tokio::test]
async fn branches_list_sends_its_filters_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([branch("main"), branch("release/v0.3")]);
    let second = json!([branch("renovate/x")]);
    let http = reads(vec![first.clone(), second.clone()]);
    let (items, pages) = walk(
        &engine,
        &http,
        "branches.list",
        json!({"id": 7, "search": "re", "sort": "name_asc", "per_page": 2}),
    )
    .await;
    assert_eq!(pages, 2);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["commit"]["id"], "c0ffee0300000000");
    assert_eq!(items[0]["default"], true);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    for (call, page) in calls.iter().zip(["1", "2"]) {
        assert_eq!(
            call.0,
            segments(&["projects", "7", "repository", "branches"])
        );
        assert_eq!(
            sorted(&call.1),
            pairs(&[
                ("search", "re"),
                ("sort", "name_asc"),
                ("page", page),
                ("per_page", "2"),
            ]),
            "page {page}"
        );
    }
}

/// The three paged reads hold `per_page` to GitLab's cap of 100, and refuse
/// the keyset cursor they withhold, each before any request.
#[tokio::test]
async fn paged_browsing_reads_refuse_out_of_range_per_page_and_keyset_paging() {
    let engine = engine();
    let base = |id: &str| match id {
        "commit.diff" => json!({"id": "org/project", "sha": "c0ffee0300000000"}),
        _ => json!({"id": "org/project"}),
    };
    for id in ["repository.tree", "commit.diff", "branches.list"] {
        for per_page in [json!(0), json!(101), json!("101")] {
            let http = reads(vec![]);
            let mut input = base(id);
            input["per_page"] = per_page.clone();
            let error = engine.read(&http, "one", id, input).await.unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {per_page}");
            assert!(http.calls.lock().unwrap().is_empty(), "`{id}` {per_page}");
        }
    }
    for (id, parameter, value) in [
        ("repository.tree", "pagination", "keyset"),
        ("repository.tree", "page_token", "7ee0000000000000"),
        ("branches.list", "page_token", "main"),
    ] {
        let http = reads(vec![]);
        let mut input = base(id);
        input[parameter] = json!(value);
        let error = engine.read(&http, "one", id, input).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {parameter}");
        assert!(http.calls.lock().unwrap().is_empty(), "`{id}` {parameter}");
    }
}
