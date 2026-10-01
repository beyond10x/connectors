//! The two GitLab commit-graph reads in the shipped selection set,
//! `commits.list` and `repository.compare`, driven through the engine over the
//! committed bundle with a recording transport: their declarations, the exact
//! request each sends, a two-page `commits.list` walk, and a compare that hit
//! GitLab's time limit told apart from an empty compare.
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
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn segments(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}
/// One commit as GitLab lists it, with the fields the consumer reads.
fn commit(id: &str, parents: &[&str]) -> Value {
    json!({
        "id": id,
        "short_id": &id[..8],
        "parent_ids": parents,
        "created_at": "2026-09-10T08:00:00.000+00:00",
        "committed_date": "2026-09-10T08:00:00.000+00:00",
        "authored_date": "2026-09-10T07:59:00.000+00:00",
        "title": format!("fixture commit {id}"),
        "message": format!("fixture commit {id}\n"),
        "author_name": "Fixture Author",
        "author_email": "author@example.test",
        "web_url": format!("https://gitlab.example.test/org/project/-/commit/{id}")
    })
}

/// Both operations are declared reads, carry the source parameters the story
/// names in their input schema, and `repository.compare` requires `from` and
/// `to` as the pinned source does.
#[test]
fn commit_reads_are_declared_reads_with_their_source_parameters() {
    let engine = engine();
    let reads = engine.declarations(&[Effect::Read]);
    for (id, parameters, required) in [
        (
            "commits.list",
            &[
                "id",
                "ref_name",
                "since",
                "until",
                "first_parent",
                "page",
                "per_page",
            ][..],
            &["id"][..],
        ),
        (
            "repository.compare",
            &["id", "from", "to", "straight"][..],
            &["from", "id", "to"][..],
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
        let mut declared: Vec<&str> = declaration.input_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        declared.sort();
        assert_eq!(declared, required, "`{id}` required");
        assert_eq!(
            declaration.output_schema["required"],
            json!(["status", "body", "provenance"]),
            "`{id}` output"
        );
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
    }
    let writes = engine.declarations(&[Effect::Write]);
    assert!(
        !writes
            .iter()
            .any(|o| o.id == "commits.list" || o.id == "repository.compare")
    );
}

/// `commits.list` sends its filters verbatim to the repository commits route
/// and returns each commit unchanged; a walk at `per_page=2` reads two pages
/// and stops on the short second one.
#[tokio::test]
async fn commits_list_sends_its_filters_and_walks_to_a_short_page() {
    let engine = engine();
    let first = json!([
        commit("c0ffee0300000000", &["c0ffee0200000000"]),
        commit(
            "c0ffee0200000000",
            &["c0ffee0100000000", "beef000100000000"]
        ),
    ]);
    let second = json!([commit("c0ffee0100000000", &[])]);
    let http = reads(vec![first.clone(), second.clone()]);
    let input = json!({
        "id": "org/project", "ref_name": "main",
        "since": "2026-09-01T00:00:00+02:00", "until": "2026-09-30T23:59:59Z",
        "first_parent": true, "per_page": 2
    });
    let mut items = Vec::new();
    let mut page = 1;
    loop {
        let mut input = input.clone();
        input["page"] = json!(page);
        let result = engine
            .read(&http, "one", "commits.list", input)
            .await
            .unwrap();
        assert_eq!(result["status"], 200);
        let body = result["body"].as_array().unwrap().clone();
        let short = body.len() < 2;
        items.extend(body);
        if short {
            break;
        }
        page += 1;
        assert!(page <= 3, "walk did not stop");
    }
    assert_eq!(page, 2);
    let mut expected = first.as_array().unwrap().clone();
    expected.extend(second.as_array().unwrap().clone());
    assert_eq!(items, expected);
    assert_eq!(
        items[1]["parent_ids"],
        json!(["c0ffee0100000000", "beef000100000000"])
    );

    let calls = http.calls.lock().unwrap().clone();
    let path = segments(&["projects", "org/project", "repository", "commits"]);
    let query = |page: &str| {
        let mut query = pairs(&[
            ("ref_name", "main"),
            ("since", "2026-09-01T00:00:00+02:00"),
            ("until", "2026-09-30T23:59:59Z"),
            ("first_parent", "true"),
            ("page", page),
            ("per_page", "2"),
        ]);
        query.sort();
        query
    };
    assert_eq!(calls.len(), 2);
    for (call, page) in calls.iter().zip(["1", "2"]) {
        assert_eq!(call.0, path);
        let mut sent = call.1.clone();
        sent.sort();
        assert_eq!(sent, query(page), "page {page}");
    }
}

/// `commits.list` holds `per_page` to GitLab's cap like the other list reads.
#[tokio::test]
async fn commits_list_refuses_per_page_outside_one_to_one_hundred() {
    let engine = engine();
    for per_page in [json!(0), json!(101), json!("101")] {
        let http = reads(vec![]);
        let error = engine
            .read(
                &http,
                "one",
                "commits.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "per_page={per_page}");
        assert!(http.calls.lock().unwrap().is_empty());
    }
}

/// `repository.compare` sends `from`, `to` and `straight` to the compare route,
/// and refuses a call without `to` before any request.
#[tokio::test]
async fn repository_compare_sends_from_to_and_straight() {
    let engine = engine();
    let http = reads(vec![json!({"commits": [], "compare_timeout": false})]);
    engine
        .read(
            &http,
            "one",
            "repository.compare",
            json!({"id": 7, "from": "v0.2.0", "to": "c0ffee03", "straight": true}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        segments(&["projects", "7", "repository", "compare"])
    );
    let mut sent = calls[0].1.clone();
    sent.sort();
    assert_eq!(
        sent,
        pairs(&[("from", "v0.2.0"), ("straight", "true"), ("to", "c0ffee03")])
    );

    let http = reads(vec![]);
    let error = engine
        .read(
            &http,
            "one",
            "repository.compare",
            json!({"id": 7, "from": "v0.2.0"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(http.calls.lock().unwrap().is_empty());
}

/// What a caller receives for three compares: one with commits, one that is
/// genuinely empty, and one GitLab cut short. The body is GitLab's own,
/// unchanged, so the timed-out compare carries `compare_timeout: true` beside
/// its (empty) `commits`, and the empty compare carries `false`: a caller
/// reads `body.compare_timeout`, never the length of `body.commits`, to tell
/// "no commits between these refs" from "GitLab did not finish".
#[tokio::test]
async fn compare_timeout_is_distinguishable_from_an_empty_compare() {
    let engine = engine();
    let full = json!({
        "commit": commit("c0ffee0300000000", &["c0ffee0200000000"]),
        "commits": [
            commit("c0ffee0200000000", &["c0ffee0100000000"]),
            commit("c0ffee0300000000", &["c0ffee0200000000"])
        ],
        "diffs": [],
        "compare_timeout": false,
        "compare_same_ref": false,
        "web_url": "https://gitlab.example.test/org/project/-/compare/v0.1.0...v0.3.0"
    });
    let empty = json!({
        "commit": null, "commits": [], "diffs": [],
        "compare_timeout": false, "compare_same_ref": false,
        "web_url": "https://gitlab.example.test/org/project/-/compare/v0.3.0...v0.3.0"
    });
    let timed_out = json!({
        "commit": null, "commits": [], "diffs": [],
        "compare_timeout": true, "compare_same_ref": false,
        "web_url": "https://gitlab.example.test/org/project/-/compare/v0.1.0...main"
    });
    let http = reads(vec![full.clone(), empty.clone(), timed_out.clone()]);
    let mut received = Vec::new();
    for to in ["v0.3.0", "v0.3.0", "main"] {
        received.push(
            engine
                .read(
                    &http,
                    "one",
                    "repository.compare",
                    json!({"id": "org/project", "from": "v0.1.0", "to": to}),
                )
                .await
                .unwrap(),
        );
    }
    let [full_result, empty_result, timed_out_result] = received.try_into().unwrap();

    assert_eq!(full_result["body"], full);
    assert_eq!(
        full_result["body"]["commits"][1]["parent_ids"],
        json!(["c0ffee0200000000"])
    );
    assert_eq!(empty_result["body"], empty);
    assert_eq!(timed_out_result["body"], timed_out);
    for result in [&full_result, &empty_result, &timed_out_result] {
        assert_eq!(result["status"], 200);
    }
    // Same status, same empty `commits`; only `compare_timeout` differs.
    assert_eq!(
        empty_result["body"]["commits"],
        timed_out_result["body"]["commits"]
    );
    assert_eq!(empty_result["body"]["compare_timeout"], json!(false));
    assert_eq!(timed_out_result["body"]["compare_timeout"], json!(true));
}
