//! story:catalog-link-header-next-page: a read whose selection sets `link_next`
//! answers the `Link` response header's `rel="next"` URL as `next`, beside
//! `status`, `body` and `provenance`, and no other header. The URL is answered
//! only on the instance's own origin (scheme, host and port of its API base)
//! and under the API base's path, the account path a Basecamp endpoint names
//! included; any other `rel="next"` is refused as `upstream_protocol`. A page
//! without `rel="next"` answers no `next`, so a walk that follows it ends
//! there. A read without `link_next` reads no header at all, as before.
//!
//! The provider is a scripted port that answers each request in turn; the
//! engine runs over a Basecamp-shaped bundle whose paths start after the
//! account, as a lowered definition's do. No network.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
};

/// The instance's API base: its endpoint names account 12345.
const BASE: &str = "https://3.example.com/12345/";
const NEXT: &str = "https://3.example.com/12345/projects.json?page=2";

fn document() -> Vec<u8> {
    let page = json!({"name": "page", "in": "query", "schema": {"type": "integer"}});
    let json_answer = json!({"200": {"description": "ok", "content": {"application/json": {}}}});
    serde_json::to_vec(&json!({
        "openapi": "3.1.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/projects.json": {
                "get": {"operationId": "ListProjects", "parameters": [page],
                    "responses": json_answer},
                "post": {"operationId": "CreateProject", "parameters": [page],
                    "requestBody": {"content": {"application/json": {"schema":
                        {"type": "object"}}}},
                    "responses": json_answer}
            },
            "/events.txt": {
                "get": {"operationId": "ListEventsText", "parameters": [page],
                    "responses": {"200": {"description": "ok",
                        "content": {"text/plain": {}}}}}
            }
        }
    }))
    .unwrap()
}

fn bundle() -> Bundle {
    let bytes = document();
    let source = ingest("fixture.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "fixture".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

/// A selection as a selection file states it.
fn selection(value: Value) -> Selection {
    serde_json::from_value(value).expect("the selection reads")
}

fn list(link_next: bool) -> Selection {
    let mut value = json!({"id": "projects.list", "operation_id": "ListProjects",
        "effect": "read"});
    if link_next {
        value["link_next"] = json!(true);
    }
    selection(value)
}

fn engine(link_next: bool) -> Engine {
    Engine::new(&bundle(), "/", &[list(link_next)])
        .and_then(|engine| engine.with_link_origin(BASE))
        .expect("the engine loads")
}

type Call = (Vec<String>, Vec<(String, String)>);
struct Port {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Port {
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

fn port(responses: Vec<HttpResponse>) -> Port {
    Port {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(Vec::new()),
    }
}

/// A `200` with a bare JSON array, as Basecamp lists answer, and `headers`.
fn page(headers: &[(&str, &str)]) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
        body: serde_json::to_vec(&json!([{"id": 1}, {"id": 2}])).unwrap(),
    }
}

fn keys(answer: &Value) -> Vec<&str> {
    answer
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

/// Every other header the page carries, none of which an answer may show.
const OTHER: [(&str, &str); 4] = [
    ("x-request-id", "fixture-request-id"),
    ("set-cookie", "session=fixture-session"),
    ("etag", "W/\"fixture-etag\""),
    ("x-total-count", "41"),
];

#[tokio::test]
async fn a_link_next_read_answers_rel_next_as_next_and_no_other_header() {
    let engine = engine(true);
    let link = format!(
        "<https://3.example.com/12345/projects.json?page=1>; rel=\"first\", <{NEXT}>; rel=\"next\", \
         <https://3.example.com/12345/projects.json?page=9>; rel=\"last\""
    );
    let mut headers = vec![("link", link.as_str())];
    headers.extend(OTHER);
    let provider = port(vec![
        page(&headers),
        page(&[
            (
                "link",
                "<https://3.example.com/12345/projects.json?page=1>; rel=\"prev\"",
            ),
            OTHER[0],
        ]),
        page(&OTHER),
    ]);
    let first = engine
        .read(&provider, "work", "projects.list", json!({}))
        .await
        .expect("the first page is answered");
    assert_eq!(keys(&first), ["body", "next", "provenance", "status"]);
    assert_eq!(first["next"], NEXT);
    assert_eq!(first["body"], json!([{"id": 1}, {"id": 2}]));
    let shown = first.to_string();
    for (name, value) in OTHER {
        assert!(
            !shown.contains(name) && !shown.contains(value),
            "the answer shows `{name}`: {shown}"
        );
    }
    assert!(!shown.contains("page=9") && !shown.contains("page=1"));

    // The walk sends the next page's `page`; a page with no `rel="next"`
    // (only `prev`) and one with no `Link` at all answer no `next`.
    for _ in 0..2 {
        let later = engine
            .read(&provider, "work", "projects.list", json!({"page": 2}))
            .await
            .expect("a later page is answered");
        assert_eq!(keys(&later), ["body", "provenance", "status"], "{later}");
    }
    assert_eq!(
        *provider.calls.lock().unwrap(),
        [
            (vec!["projects.json".to_string()], vec![]),
            (
                vec!["projects.json".to_string()],
                vec![("page".to_string(), "2".to_string())]
            ),
            (
                vec!["projects.json".to_string()],
                vec![("page".to_string(), "2".to_string())]
            ),
        ]
    );

    let declared = engine.declarations(&[Effect::Read]);
    assert_eq!(
        declared[0].output_schema["properties"]["next"],
        json!({"type": "string"})
    );
    assert!(
        !declared[0].output_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("next"))
    );
}

#[tokio::test]
async fn rel_next_is_read_in_any_case_and_quoting_on_the_canonical_origin() {
    let engine = engine(true);
    for (link, next) in [
        (format!("<{NEXT}>; rel=next"), NEXT.to_string()),
        (format!("<{NEXT}> ; REL = \"Next\""), NEXT.to_string()),
        (format!("<{NEXT}>; rel=\"next last\""), NEXT.to_string()),
        (
            "<https://3.EXAMPLE.com:443/12345/projects.json?page=2>; rel=\"next\"".to_string(),
            "https://3.EXAMPLE.com:443/12345/projects.json?page=2".to_string(),
        ),
        // One line repeated, as a host joins two equal `Link` lines.
        (
            format!("<{NEXT}>; rel=\"next\", <{NEXT}>; rel=\"next\""),
            NEXT.to_string(),
        ),
        // A comma inside the URL is the URL's.
        (
            "<https://3.example.com/12345/projects.json?page=2&ids=1,2>; title=\"a, b\"; rel=\"next\""
                .to_string(),
            "https://3.example.com/12345/projects.json?page=2&ids=1,2".to_string(),
        ),
    ] {
        let provider = port(vec![page(&[("Link", link.as_str())])]);
        let answer = engine
            .read(&provider, "work", "projects.list", json!({}))
            .await
            .unwrap_or_else(|error| panic!("{link}: refused {error:?}"));
        assert_eq!(answer["next"], json!(next), "{link}");
    }
}

#[tokio::test]
async fn a_rel_next_off_the_instance_origin_or_account_path_is_refused() {
    let engine = engine(true);
    for link in [
        // Another host, another port, another scheme.
        "<https://evil.example/12345/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com:8443/12345/projects.json?page=2>; rel=\"next\"",
        "<http://3.example.com/12345/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com@evil.example/12345/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com.evil.example/12345/projects.json?page=2>; rel=\"next\"",
        // Another account, the account itself, a path climbing out of it.
        "<https://3.example.com/99999/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/123456/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/12345?page=2>; rel=\"next\"",
        "<https://3.example.com/12345/../99999/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/12345/%2e%2E/99999/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/12345/.%2E/99999/projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/12345\\..\\99999/projects.json?page=2>; rel=\"next\"",
        // Not an absolute URL, two different next pages, no link at all.
        "</12345/projects.json?page=2>; rel=\"next\"",
        "<projects.json?page=2>; rel=\"next\"",
        "<https://3.example.com/12345/projects.json?page=2>; rel=\"next\", <https://3.example.com/12345/projects.json?page=3>; rel=\"next\"",
        "https://3.example.com/12345/projects.json?page=2; rel=\"next\"",
        "<https://3.example.com/12345/projects.json?page=2; rel=\"next\"",
    ] {
        let provider = port(vec![page(&[("link", link)])]);
        let error = engine
            .read(&provider, "work", "projects.list", json!({}))
            .await
            .expect_err(link);
        assert_eq!(error.code, ErrorCode::UpstreamProtocol, "{link}");
        assert!(
            !error.message.contains("evil") && !error.message.contains("99999"),
            "the refusal repeats no part of the URL: {}",
            error.message
        );
        assert_eq!(provider.calls.lock().unwrap().len(), 1, "{link}");
    }

    let long = format!(
        "<https://3.example.com/12345/projects.json?page=2&pad={}>; rel=\"next\"",
        "a".repeat(16 * 1024)
    );
    let provider = port(vec![page(&[("link", long.as_str())])]);
    let error = engine
        .read(&provider, "work", "projects.list", json!({}))
        .await
        .expect_err("a next URL over 16 KiB");
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);

    // An engine told no API base answers no `next` it cannot hold to one.
    let unplaced = Engine::new(&bundle(), "/", &[list(true)]).unwrap();
    let link = format!("<{NEXT}>; rel=\"next\"");
    let provider = port(vec![page(&[("link", link.as_str())])]);
    let error = unplaced
        .read(&provider, "work", "projects.list", json!({}))
        .await
        .expect_err("no API base");
    assert_eq!(error.code, ErrorCode::UpstreamProtocol);
}

#[tokio::test]
async fn a_read_without_link_next_reads_no_link_header() {
    let engine = engine(false);
    let link = format!(
        "<https://evil.example/projects.json?page=2>; rel=\"next\", <{NEXT}>; rel=\"next\""
    );
    let provider = port(vec![page(&[("link", link.as_str())])]);
    let answer = engine
        .read(&provider, "work", "projects.list", json!({}))
        .await
        .expect("the read is answered as before");
    assert_eq!(keys(&answer), ["body", "provenance", "status"]);
    let declared = engine.declarations(&[Effect::Read]);
    assert!(
        declared[0].output_schema["properties"]
            .get("next")
            .is_none()
    );
}

#[test]
fn link_next_on_a_write_or_a_text_read_is_refused_when_the_selection_loads() {
    for (value, words) in [
        (
            json!({"id": "project.create", "operation_id": "CreateProject",
                "effect": "write", "link_next": true}),
            "on a write",
        ),
        (
            json!({"id": "events.text", "operation_id": "ListEventsText",
                "effect": "read", "link_next": true}),
            "does not answer JSON",
        ),
        (
            json!({"id": "projects.text", "operation_id": "ListProjects",
                "effect": "read", "response": "text", "link_next": true}),
            "does not answer JSON",
        ),
        (
            json!({"id": "projects.raw", "operation_id": "ListProjects",
                "effect": "read", "response": "binary", "link_next": true}),
            "does not answer JSON",
        ),
    ] {
        let error = match Engine::new(&bundle(), "/", &[selection(value.clone())]) {
            Ok(_) => panic!("loaded: {value}"),
            Err(error) => error,
        };
        assert!(
            error.message.contains("link_next") && error.message.contains(words),
            "{value}: {}",
            error.message
        );
    }
}
