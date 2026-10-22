//! Adversarial pass over wave 20261021, against the shipped GitLab feed
//! (`providers/gitlab/operations.json`): a caller-written watermark is opened before the merge
//! request list decides admission. The family requires "admission of the principal, operation,
//! connection and container precedes any disclosure, including whether a watermark is valid"
//! (`contracts/datasources/feed/v1alpha1/semantics.md`), and the catalog binding's Watermark
//! scope row answers a container the listing refuses "as the Errors row says, whatever the
//! watermark" (`contracts/catalog/v1alpha1/semantics.md` §3.3). In-process answers only; no live
//! provider is called.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const INSTANCE: &str = "gitlab-feed-instance";

struct Reads {
    responses: Mutex<VecDeque<(u16, Value)>>,
    paths: Mutex<Vec<Vec<String>>>,
}

impl Reads {
    fn new(responses: Vec<(u16, Value)>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            paths: Mutex::new(Vec::new()),
        }
    }

    fn paths(&self) -> Vec<Vec<String>> {
        self.paths.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.paths
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
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

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
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

fn project(id: u64) -> Value {
    let mut project: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/feed/gitlab/project.json")).unwrap(),
    )
    .unwrap();
    project["id"] = json!(id);
    project
}

/// A watermark in the engine's own format for project 1001, as any caller can write one: one
/// instant, and `seen` pairs at it.
fn forged(seen: usize) -> String {
    let at = "2026-10-06T09:00:00.000Z";
    let pairs: Vec<Value> = (1..=seen).map(|iid| json!([iid.to_string(), at])).collect();
    let mark = json!({
        "format": "catalog-feed/1",
        "profile": shipped()["feed"]["profile"],
        "instance": INSTANCE,
        "container": "1001",
        "time": at,
        "seen": pairs,
    });
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

/// GitLab shows project 1001 and refuses its merge request list with a 403, which the binding
/// answers `not_found`, exactly as an unknown project. A watermark holding `max_limit` pairs at
/// one instant must not answer otherwise: today it is `unavailable` ("more items share one
/// instant than one provider page holds") before the list is read, which tells the caller the
/// project is there.
#[tokio::test]
async fn adversary_a_full_seen_set_does_not_tell_a_refused_project_from_an_unknown_one() {
    let engine = engine();
    let max_limit = shipped()["feed"]["max_limit"].as_u64().unwrap() as usize;
    let read = |watermark: Option<String>| {
        let engine = &engine;
        async move {
            let http = Reads::new(vec![
                (200, project(1001)),
                (403, json!({"message": "403 Forbidden"})),
            ]);
            let mut input = json!({"container": "1001", "limit": 10});
            if let Some(watermark) = watermark {
                input["watermark"] = json!(watermark);
            }
            let answer = engine.read(&http, INSTANCE, ITEMS, input).await;
            (answer, http.paths())
        }
    };
    let (reference, _) = read(None).await;
    let reference = reference.unwrap_err();
    assert_eq!(reference.code, ErrorCode::NotFound, "control");
    // Control: a watermark one pair short of the ceiling answers as the reference.
    let (short, _) = read(Some(forged(max_limit - 1))).await;
    let short = short.unwrap_err();
    assert_eq!(
        (&short.code, &short.message),
        (&reference.code, &reference.message),
        "control"
    );

    let (answer, paths) = read(Some(forged(max_limit))).await;
    let answer = answer.unwrap_err();
    assert_eq!(
        (&answer.code, &answer.message),
        (&reference.code, &reference.message),
        "a refused project is told apart by a forged watermark; provider reads {paths:?}"
    );
}
