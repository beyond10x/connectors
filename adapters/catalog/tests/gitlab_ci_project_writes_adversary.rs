//! Adversary pass against story:parity-gitlab-project-writes: the shipped
//! `file.update` guard, driven through the engine against a scripted GitLab.
//! No network, no credential.
//!
//! `file.update`'s preflight reads the branch (`GET .../repository/branches/
//! {branch}`), but its proof after the write reads `GET .../repository/commits/
//! {branch}`, which GitLab resolves as a git revision. Git resolves a short
//! name `refs/tags/<name>` before `refs/heads/<name>` (gitrevisions(7)), so
//! when a tag shares the branch's name the read after the write answers the
//! tag's commit, not the branch head the write produced. The guard then proves
//! the write by an object it did not write.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

/// The commit the caller pinned: the branch head when it was approved.
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
/// Another actor's commit, pushed onto the branch after the preflight read and
/// before the write.
const MOVED: &str = "1111111111111111111111111111111111111111";
/// The commit the write produced, on top of `MOVED`.
const WRITTEN: &str = "2222222222222222222222222222222222222222";
/// The commit the same-named tag points at, an unrelated child of `SHA`.
const TAGGED: &str = "3333333333333333333333333333333333333333";
/// A name that is both a branch and a tag, as GitLab permits.
const NAME: &str = "v1";

fn json_answer(status: u16, body: &Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(body).unwrap(),
    }
}
fn commit(id: &str, parent: &str) -> Value {
    json!({"id": id, "short_id": &id[..8], "title": "fixture", "parent_ids": [parent],
           "message": "fixture"})
}
fn branch(name: &str, head: &str, parent: &str) -> Value {
    json!({"name": name, "commit": commit(head, parent), "merged": false, "protected": false,
           "default": false, "can_push": true})
}

/// A GitLab whose state the test sets: answers keyed by the route after the
/// project, recorded in order.
struct Gitlab {
    routes: Mutex<BTreeMap<String, Value>>,
    calls: Mutex<Vec<String>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Gitlab {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        let start = path.iter().position(|s| *s == "repository").unwrap_or(0);
        let route = path[start..].join("/");
        self.calls.lock().unwrap().push(route.clone());
        Ok(match self.routes.lock().unwrap().get(&route) {
            Some(body) => json_answer(200, body),
            None => json_answer(404, &json!({"message": "404 Not Found"})),
        })
    }
}

type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Value)>>>;
struct Write {
    sent: Sent,
    answer: HttpResponse,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        _query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            body.clone(),
        ));
        Ok(self.answer)
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine() -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &selections).unwrap()
}

/// The branch `v1` is at `SHA` when the preflight reads it. Another actor then
/// pushes `MOVED` onto it, and the update lands on top of that (`WRITTEN`,
/// parent `MOVED`): exactly the lost-update race the guard exists to report,
/// which its description promises leaves the effect unknown. A tag `v1` at
/// `TAGGED` (parent `SHA`) also exists, so `commits/v1` resolves to the tag,
/// whose first parent is `SHA`, and the write is reported applied.
#[tokio::test]
async fn adversary_file_update_proof_is_not_satisfied_by_a_same_named_tag() {
    let engine = engine();
    let gitlab = Gitlab {
        routes: Mutex::new(BTreeMap::from([(
            format!("repository/branches/{NAME}"),
            branch(NAME, SHA, MOVED),
        )])),
        calls: Mutex::new(Vec::new()),
    };
    let input = json!({"id": "org/project", "file_path": "docs/guide.md", "sha": SHA,
                       "body": {"branch": NAME, "content": "changed",
                                "commit_message": "update file"}});
    let prepared = engine
        .prepare(&gitlab, "fixture", "file.update", input)
        .await
        .expect("the preflight sees the branch at the pinned sha");
    // Between the preflight and the write: the branch moves, the write lands
    // on the moved head, and git resolves `v1` to the tag first.
    {
        let mut routes = gitlab.routes.lock().unwrap();
        routes.insert(
            format!("repository/branches/{NAME}"),
            branch(NAME, WRITTEN, MOVED),
        );
        routes.insert(format!("repository/commits/{NAME}"), commit(TAGGED, SHA));
        routes.insert(
            format!("repository/tags/{NAME}"),
            json!({"name": NAME,
            "commit": commit(TAGGED, SHA)}),
        );
    }
    let sent: Sent = Arc::default();
    let outcome = prepared
        .execute_reading(
            Box::new(Write {
                sent: sent.clone(),
                answer: json_answer(200, &json!({"file_path": "docs/guide.md", "branch": NAME})),
            }),
            Some(&gitlab),
        )
        .await;
    assert_eq!(sent.lock().unwrap().len(), 1);
    let calls = gitlab.calls.lock().unwrap().clone();
    match outcome {
        WriteOutcome::Unknown(_) => {}
        WriteOutcome::Applied(answer) => panic!(
            "file.update written onto a moved branch (head {WRITTEN}, parent {MOVED}) reported \
             applied, proved by reads {calls:?}: {answer:?}"
        ),
        WriteOutcome::Refused(error) => panic!("refused: {error:?}"),
    }
}
