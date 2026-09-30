//! story:catalog-parameters-declare-their-type: a catalog parameter's declared
//! input type follows the type its pinned document gives it. Integers take a
//! JSON integer or its decimal string, booleans take `true` or `false`, strings
//! take any string. A value of another type is refused as `invalid_input`
//! before any request, on bounded and unbounded parameters alike.
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
fn reads() -> Reads {
    Reads {
        responses: Mutex::new(
            [HttpResponse {
                status: 200,
                headers: Default::default(),
                body: b"[]".to_vec(),
            }]
            .into(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine(provider: &str, base: &str) -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), provider).unwrap();
    Engine::new(&bundle, base, &selections).unwrap()
}
fn gitlab() -> Engine {
    engine("gitlab", "/api/v4")
}
fn confluence() -> Engine {
    engine("confluence", "/wiki")
}
fn jira() -> Engine {
    engine("jira", "/rest/api/3")
}

/// Every refusal here must come back as `invalid_input` with nothing sent.
async fn refused(engine: &Engine, id: &str, cases: Vec<Value>) -> Vec<String> {
    let mut escaped = Vec::new();
    for input in cases {
        let http = reads();
        let outcome = engine.read(&http, "one", id, input.clone()).await;
        let calls = http.calls.lock().unwrap().clone();
        if !calls.is_empty()
            || outcome.as_ref().err().map(|e| &e.code) != Some(&ErrorCode::InvalidInput)
        {
            escaped.push(format!("{id} {input}: {outcome:?}, sent {calls:?}"));
        }
    }
    escaped
}

/// The value one read sent for `name`, or why it sent nothing.
async fn sent(engine: &Engine, id: &str, input: Value, name: &str) -> Option<String> {
    let http = reads();
    engine
        .read(&http, "one", id, input.clone())
        .await
        .unwrap_or_else(|e| panic!("{id} {input} refused: {e:?}"));
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1, "{id} {input}");
    calls[0]
        .1
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

#[tokio::test]
async fn gitlab_per_page_of_another_type_is_refused_before_any_request() {
    let escaped = refused(
        &gitlab(),
        "issues.list",
        vec![
            json!({"id": "org/project", "per_page": true}),
            json!({"id": "org/project", "per_page": false}),
            json!({"id": "org/project", "per_page": "abc"}),
            json!({"id": "org/project", "per_page": "true"}),
        ],
    )
    .await;
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

#[tokio::test]
async fn confluence_limit_of_another_type_is_refused_before_any_request() {
    let engine = confluence();
    let mut escaped = Vec::new();
    for id in ["pages.changed", "space.pages", "page.comments"] {
        let base = if id == "pages.changed" {
            json!({})
        } else {
            json!({"id": "65538"})
        };
        let with = |limit: Value| {
            let mut input = base.clone();
            input["limit"] = limit;
            input
        };
        escaped.extend(
            refused(
                &engine,
                id,
                vec![with(json!(true)), with(json!(false)), with(json!("abc"))],
            )
            .await,
        );
    }
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

/// An unbounded integer has no bound check behind it; the declared type is the
/// only thing that refuses a boolean there.
#[tokio::test]
async fn unbounded_integers_refuse_booleans_and_non_decimal_strings() {
    let mut escaped = refused(
        &gitlab(),
        "issues.list",
        vec![
            json!({"id": "org/project", "closed_by_id": true}),
            json!({"id": "org/project", "page": true}),
            json!({"id": "org/project", "page": "2a"}),
            json!({"id": "org/project", "page": "1.5"}),
            json!({"id": "org/project", "page": ""}),
        ],
    )
    .await;
    escaped.extend(
        refused(
            &jira(),
            "issue.comments",
            vec![
                json!({"issueIdOrKey": "KEY-1", "maxResults": true}),
                json!({"issueIdOrKey": "KEY-1", "startAt": "first"}),
            ],
        )
        .await,
    );
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

#[tokio::test]
async fn booleans_refuse_anything_but_true_or_false() {
    let mut escaped = refused(
        &gitlab(),
        "issues.list",
        vec![
            json!({"id": "org/project", "with_labels_details": 1}),
            json!({"id": "org/project", "with_labels_details": "yes"}),
        ],
    )
    .await;
    escaped.extend(
        refused(
            &confluence(),
            "page.get",
            vec![
                json!({"id": "1001", "get-draft": 0}),
                json!({"id": "1001", "get-draft": "no"}),
            ],
        )
        .await,
    );
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

#[tokio::test]
async fn strings_take_integers_as_text_and_refuse_other_types() {
    let gitlab = gitlab();
    let escaped = refused(
        &gitlab,
        "issues.list",
        vec![
            json!({"id": "org/project", "state": true}),
            json!({"id": "org/project", "state": 1.5}),
            json!({"id": "org/project", "state": 3.0}),
            json!({"id": "org/project", "state": {"a": 1}}),
        ],
    )
    .await;
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
    assert_eq!(
        sent(
            &gitlab,
            "issues.list",
            json!({"id": "org/project", "state": 3}),
            "state"
        )
        .await
        .as_deref(),
        Some("3")
    );
}

#[tokio::test]
async fn integers_refuse_fractions_and_exponents() {
    let escaped = refused(
        &gitlab(),
        "issues.list",
        vec![
            json!({"id": "org/project", "page": 2.0}),
            json!({"id": "org/project", "page": 1e2}),
            json!({"id": "org/project", "per_page": 20.0}),
        ],
    )
    .await;
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

#[tokio::test]
async fn integers_as_numbers_or_decimal_strings_are_still_sent() {
    let gitlab = gitlab();
    for per_page in [json!(100), json!("100")] {
        assert_eq!(
            sent(
                &gitlab,
                "issues.list",
                json!({"id": "org/project", "per_page": per_page}),
                "per_page"
            )
            .await
            .as_deref(),
            Some("100")
        );
    }
    let confluence = confluence();
    for limit in [json!(25), json!("25")] {
        assert_eq!(
            sent(
                &confluence,
                "pages.changed",
                json!({"limit": limit}),
                "limit"
            )
            .await
            .as_deref(),
            Some("25")
        );
    }
    // An integer path parameter keeps its decimal string form too.
    let http = reads();
    confluence
        .read(&http, "one", "page.get", json!({"id": "1001"}))
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].0,
        ["api", "v2", "pages", "1001"]
    );
}

#[tokio::test]
async fn string_and_boolean_parameters_from_the_pinned_documents_keep_working() {
    let gitlab = gitlab();
    assert_eq!(
        sent(
            &gitlab,
            "issues.list",
            json!({"id": "org/project", "state": "opened"}),
            "state"
        )
        .await
        .as_deref(),
        Some("opened")
    );
    for (flag, text) in [(json!(true), "true"), (json!(false), "false")] {
        assert_eq!(
            sent(
                &gitlab,
                "issues.list",
                json!({"id": "org/project", "with_labels_details": flag}),
                "with_labels_details"
            )
            .await
            .as_deref(),
            Some(text)
        );
    }
    assert_eq!(
        sent(
            &confluence(),
            "page.get",
            json!({"id": "1001", "get-draft": true}),
            "get-draft"
        )
        .await
        .as_deref(),
        Some("true")
    );
    assert_eq!(
        sent(
            &jira(),
            "issues.search",
            json!({"jql": "project = KEY", "maxResults": 50}),
            "jql"
        )
        .await
        .as_deref(),
        Some("project = KEY")
    );
}

/// The descriptor a caller reads says the same thing the engine enforces: a
/// bounded page size is declared an integer, never a boolean.
#[test]
fn declared_page_sizes_do_not_admit_booleans() {
    for (engine, id, name) in [
        (gitlab(), "issues.list", "per_page"),
        (confluence(), "pages.changed", "limit"),
    ] {
        let declaration = engine
            .declarations(&[Effect::Read])
            .into_iter()
            .find(|d| d.id == id)
            .unwrap();
        let schema = &declaration.input_schema["properties"][name];
        assert!(
            connectors_sdk::validate(schema, &json!(true)).is_err(),
            "{id} `{name}` declares {schema}"
        );
        assert!(connectors_sdk::validate(schema, &json!(7)).is_ok());
        assert!(connectors_sdk::validate(schema, &json!("7")).is_ok());
    }
}
