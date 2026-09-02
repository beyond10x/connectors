//! Drift tests for the committed OpenAPI contract served at `/openapi.json` (S-067).
//!
//! The served document's envelope schemas are generated from the `protocol` payload structs;
//! the skeleton (paths, examples, MCP, error body) is authored, so these tests are what keep it
//! honest: every embedded request example must be accepted by the exact protocol type the
//! route deserializes with, every refusal example must name a code the protocol's closed
//! error vocabulary actually contains, and every documented route must exist in the real
//! router. A deliberately wrong example — an unknown field anywhere — must be refused, which
//! proves the `deny_unknown_fields` discipline is exercised rather than assumed.
//!
//! The public documentation page at `/docs` (S-068) is held to the same standard: every JSON
//! block it shows must be one of the document's own examples after JSON normalization, the
//! page must name no absolute origin the document does not, and it must trigger zero
//! external requests — one self-contained HTML answer.

use serde_json::Value;
use sha2::Digest as _;

use super::*;

/// The exact bytes `docs.rs` embeds and the route serves verbatim.
fn doc_json() -> &'static str {
    crate::hosted::docs::document_json()
}

/// The six envelope endpoints and how many distinct request examples each must carry —
/// one per method in the closed set, so a method the document forgets fails here.
const ENVELOPE_EXAMPLE_FLOORS: [(&str, usize); 6] = [
    ("/approvals", 1),
    ("/operations", 7),
    ("/connections", 8),
    ("/catalog", 2),
    ("/events", 3),
    ("/datasources", 4),
];

fn test_router() -> Router {
    router_with_client_discovery(
        Arc::new(Verifier),
        Arc::new(Backend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
        Some(Arc::new(SubscriptionCustody::new(Arc::new(
            connector_secrets::MemoryStore::new(),
        )))),
        ClientDiscovery::new(&url::Url::parse("https://identity.example.test/").unwrap()),
    )
}

fn document() -> Value {
    serde_json::from_str(doc_json()).expect("the committed OpenAPI document is JSON")
}

/// Every named example under one media-type `examples` object, as `(name, value)` pairs.
fn named_examples(container: &Value) -> Vec<(String, Value)> {
    container
        .get("content")
        .and_then(|content| content.get("application/json"))
        .and_then(|media| media.get("examples"))
        .and_then(Value::as_object)
        .map(|examples| {
            examples
                .iter()
                .map(|(name, example)| {
                    (
                        name.clone(),
                        example
                            .get("value")
                            .cloned()
                            .unwrap_or_else(|| panic!("example `{name}` carries a value")),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn request_examples(doc: &Value, path: &str) -> Vec<(String, Value)> {
    let body = &doc["paths"][path]["post"]["requestBody"];
    let examples = named_examples(body);
    assert!(
        !examples.is_empty(),
        "`{path}` documents at least one request example"
    );
    examples
}

/// Deserialize one request example with the exact type the route uses, then run the same
/// frame validation the route runs, so an example the server would refuse cannot ship.
fn assert_request_example_accepted(path: &str, name: &str, value: Value) {
    let refused = |stage: &str, error: String| -> ! {
        panic!("`{path}` request example `{name}` is refused by the protocol {stage}: {error}")
    };
    match path {
        "/approvals" => serde_json::from_value::<ApprovalRequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.to_string())),
        "/operations" => serde_json::from_value::<RequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.to_string())),
        "/connections" => serde_json::from_value::<ConnectionRequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.to_string())),
        "/catalog" => serde_json::from_value::<CatalogRequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.message)),
        "/events" => serde_json::from_value::<EventRequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.to_string())),
        "/datasources" => serde_json::from_value::<DatasourceRequestEnvelope>(value)
            .unwrap_or_else(|error| refused("type", error.to_string()))
            .validate()
            .unwrap_or_else(|error| refused("validation", error.to_string())),
        other => panic!("no protocol type is mapped for `{other}`"),
    }
}

#[tokio::test]
async fn openapi_json_is_served_verbatim_with_a_content_hash_etag() {
    let response = test_router()
        .oneshot(
            Request::get("/openapi.json")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("the contract answer carries an ETag")
        .to_owned();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("bounded body");
    assert_eq!(
        body.as_ref(),
        doc_json().as_bytes(),
        "the served document is the committed artifact, byte for byte"
    );
    let digest = sha2::Sha256::digest(body.as_ref());
    let mut expected = String::with_capacity(66);
    expected.push('"');
    for byte in digest {
        expected.push_str(&format!("{byte:02x}"));
    }
    expected.push('"');
    assert_eq!(etag, expected, "the ETag is the content hash of the bytes");
}

#[test]
fn every_documented_request_example_is_accepted_by_its_protocol_type() {
    let doc = document();
    for (path, floor) in ENVELOPE_EXAMPLE_FLOORS {
        let examples = request_examples(&doc, path);
        let methods: BTreeSet<String> = examples
            .iter()
            .filter_map(|(_, value)| value["request"]["method"].as_str().map(str::to_owned))
            .collect();
        if path == "/approvals" {
            assert!(
                examples.len() >= floor,
                "`{path}` documents its issue request"
            );
        } else {
            assert!(
                methods.len() >= floor,
                "`{path}` documents an example for each of its {floor} methods, found {methods:?}"
            );
        }
        for (name, value) in examples {
            assert_request_example_accepted(path, &name, value);
        }
    }
}

#[test]
fn every_documented_refusal_example_names_a_real_error_code() {
    let doc = document();
    let mut operation_codes = BTreeSet::new();
    let mut refusals = 0usize;
    for (path, _) in ENVELOPE_EXAMPLE_FLOORS {
        let responses = doc["paths"][path]["post"]["responses"]
            .as_object()
            .unwrap_or_else(|| panic!("`{path}` documents its responses"));
        for (status, response) in responses {
            for (name, value) in named_examples(response) {
                if value["status"].as_str() != Some("error") {
                    continue;
                }
                refusals += 1;
                let error = value["error"].clone();
                let code_string = error["code"].as_str().map(str::to_owned);
                let refused = |what: String| -> ! {
                    panic!("`{path}` {status} refusal example `{name}` names no real error: {what}")
                };
                // The closed serde enums are the proof: an invented code cannot deserialize.
                match path {
                    "/operations" => {
                        serde_json::from_value::<OperationError>(error)
                            .unwrap_or_else(|error| refused(error.to_string()));
                        operation_codes
                            .insert(code_string.unwrap_or_else(|| refused("no code".to_owned())))
                    }
                    "/connections" => serde_json::from_value::<ConnectionError>(error)
                        .map(|_| true)
                        .unwrap_or_else(|error| refused(error.to_string())),
                    "/events" => serde_json::from_value::<EventError>(error)
                        .map(|_| true)
                        .unwrap_or_else(|error| refused(error.to_string())),
                    "/datasources" => serde_json::from_value::<DatasourceError>(error)
                        .map(|_| true)
                        .unwrap_or_else(|error| refused(error.to_string())),
                    "/catalog" => {
                        let error =
                            serde_json::from_value::<protocol::catalog::CatalogError>(error)
                                .unwrap_or_else(|error| refused(error.to_string()));
                        // The catalog code is an open string on the wire (the generated
                        // schema says so); the hosted route produces exactly these four,
                        // and the documented refusal examples must agree.
                        assert!(
                            doc["components"]["schemas"]["catalog.responseEnvelope"]
                                .pointer("/properties/code/enum")
                                .is_none(),
                            "the generated catalog schema keeps the wire-open code"
                        );
                        let documented = ["protocol", "invalid_input", "not_found", "not_granted"]
                            .into_iter()
                            .collect::<BTreeSet<_>>();
                        documented.contains(error.code.as_str())
                            || refused(format!("`{}` is outside the documented enum", error.code))
                    }
                    other => panic!("no error type is mapped for `{other}`"),
                };
            }
        }
    }
    assert!(refusals >= 10, "the document keeps its refusal examples");
    for code in [
        "not_granted",
        "approval_required",
        "invalid_input",
        "result_too_large",
        "unavailable",
        "stale_authority",
    ] {
        assert!(
            operation_codes.contains(code),
            "the operation refusal examples keep covering `{code}`"
        );
    }
}

#[test]
fn a_request_example_with_an_unknown_field_is_refused() {
    let doc = document();
    let (_, example) = request_examples(&doc, "/operations")
        .into_iter()
        .next()
        .expect("at least one operation request example");
    serde_json::from_value::<RequestEnvelope>(example.clone())
        .expect("the untouched example is accepted");

    let mut smuggled_envelope = example.clone();
    smuggled_envelope["ambient_secret"] = serde_json::json!("must-refuse");
    assert!(
        serde_json::from_value::<RequestEnvelope>(smuggled_envelope).is_err(),
        "an unknown envelope field must be refused, or the drift suite proves nothing"
    );

    let mut smuggled_params = example;
    smuggled_params["request"]["params"]["ambient_secret"] = serde_json::json!("must-refuse");
    assert!(
        serde_json::from_value::<RequestEnvelope>(smuggled_params).is_err(),
        "an unknown params field must be refused, or the drift suite proves nothing"
    );
}

#[tokio::test]
async fn every_documented_route_exists_in_the_real_router() {
    let doc = document();
    let paths = doc["paths"]
        .as_object()
        .expect("the document declares paths");
    for (path, item) in paths {
        for method in item.as_object().expect("path item").keys() {
            let request = match method.as_str() {
                "get" => Request::get(path).body(Body::empty()).expect("request"),
                "post" => Request::post(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from("{}"))
                    .expect("request"),
                "put" => Request::put(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from("{}"))
                    .expect("request"),
                "delete" => Request::delete(path)
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::empty())
                    .expect("request"),
                other => panic!("`{path}` documents unmapped method `{other}`"),
            };
            let response = test_router().oneshot(request).await.expect("infallible");
            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "`{method} {path}` is documented but absent from the router"
            );
        }
    }
    // The control: an undocumented route must answer 404, so the assertion above can fail.
    let response = test_router()
        .oneshot(
            Request::get("/absent-route")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn the_document_pins_the_exact_wire_contract_identities_and_audience() {
    let doc = document();
    let schemas = &doc["components"]["schemas"];
    for (schema, contract) in [
        ("approval.requestEnvelope", protocol::approval::CONTRACT),
        ("operation.requestEnvelope", protocol::operation::CONTRACT),
        (
            "connection.request_envelope",
            protocol::connection::CONTRACT,
        ),
        ("catalog.requestEnvelope", protocol::catalog::CONTRACT),
        ("event.request_envelope", protocol::event::CONTRACT),
        (
            "datasource.request_envelope",
            protocol::datasource::CONTRACT,
        ),
    ] {
        assert_eq!(
            schemas[schema]["properties"]["protocol"]["const"].as_str(),
            Some(contract),
            "`{schema}` pins the exact wire contract identity"
        );
    }
    let bearer = doc["components"]["securitySchemes"]["identityAccessToken"]["description"]
        .as_str()
        .expect("the bearer scheme carries a description");
    assert!(
        bearer.contains(CONNECTORS_AUDIENCE),
        "the bearer scheme names the identity audience"
    );
    for scope in [
        "connectors.catalog.read",
        "connectors.invoke",
        "connectors.connections.manage",
        "connectors.connections.self",
        "connectors.events.read",
        "connectors.events.self",
    ] {
        assert!(
            bearer.contains(scope),
            "the bearer scheme names the `{scope}` scope"
        );
    }
}

/// Fetch the public documentation page through the real router, unauthenticated.
async fn docs_page() -> String {
    let response = test_router()
        .oneshot(Request::get("/docs").body(Body::empty()).expect("request"))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("bounded body");
    String::from_utf8(body.to_vec()).expect("the page is UTF-8")
}

/// Reverse the page's HTML escaping; `&amp;` must be undone last or it re-escapes.
fn unescape_html(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Every JSON block the page marks as lifted from the document, as
/// `(path, kind, name, parsed value)`. The page wraps each one in an element carrying
/// `data-example="<path> <kind> <name>"`; the content is HTML-escaped JSON.
fn page_examples(page: &str) -> Vec<(String, String, String, Value)> {
    const MARKER: &str = "data-example=\"";
    let mut examples = Vec::new();
    let mut rest = page;
    while let Some(start) = rest.find(MARKER) {
        let attribute = &rest[start + MARKER.len()..];
        let (spec, after) = attribute
            .split_once('"')
            .expect("the example attribute closes");
        let mut parts = spec.split(' ');
        let path = parts.next().expect("the example names its path").to_owned();
        let kind = parts.next().expect("the example names its kind").to_owned();
        let name = parts.next().expect("the example names its name").to_owned();
        assert!(
            parts.next().is_none(),
            "the example marker is exactly `<path> <kind> <name>`, got `{spec}`"
        );
        let content = &after[after.find('>').expect("the example tag closes") + 1..];
        let (body, tail) = content
            .split_once("</")
            .expect("the example element closes");
        let value = serde_json::from_str(&unescape_html(body)).unwrap_or_else(|error| {
            panic!("`{path}` {kind} example `{name}` on the page is not JSON: {error}")
        });
        examples.push((path, kind, name, value));
        rest = tail;
    }
    examples
}

/// Resolve one example the page claims to show back to the document's own value.
fn documented_example(doc: &Value, path: &str, kind: &str, name: &str) -> Value {
    let operation = &doc["paths"][path]["post"];
    let container = match kind {
        "request" => &operation["requestBody"],
        "response" => operation["responses"]
            .get("200")
            .or_else(|| operation["responses"].get("201"))
            .expect("an envelope endpoint documents a success response"),
        other => panic!("the page marks unknown example kind `{other}`"),
    };
    let value = container["content"]["application/json"]["examples"][name]["value"].clone();
    assert!(
        !value.is_null(),
        "the page shows `{path}` {kind} example `{name}`, which the document does not carry"
    );
    value
}

#[tokio::test]
async fn the_docs_page_is_served_unauthenticated_as_html() {
    let response = test_router()
        .oneshot(Request::get("/docs").body(Body::empty()).expect("request"))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/html; charset=utf-8")
    );
    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("the page answer carries an ETag")
        .to_owned();
    let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("bounded body");
    let digest = sha2::Sha256::digest(body.as_ref());
    let mut expected = String::with_capacity(66);
    expected.push('"');
    for byte in digest {
        expected.push_str(&format!("{byte:02x}"));
    }
    expected.push('"');
    assert_eq!(etag, expected, "the ETag is the content hash of the bytes");
    let page = String::from_utf8(body.to_vec()).expect("the page is UTF-8");
    assert!(
        page.starts_with("<!doctype html>"),
        "the answer is one HTML document"
    );
}

#[tokio::test]
async fn every_docs_page_example_is_the_documents_example_after_json_normalization() {
    let doc = document();
    let page = docs_page().await;
    let examples = page_examples(&page);
    // Floor: the five envelope endpoints (request + response each), the MCP
    // initialize/tools_call requests and initialize response, and the datasource
    // read-verb blocks.
    assert!(
        examples.len() >= 17,
        "the page keeps its example blocks, found {}",
        examples.len()
    );
    let mut paths = BTreeSet::new();
    for (path, kind, name, shown) in examples {
        let documented = documented_example(&doc, &path, &kind, &name);
        assert_eq!(
            shown, documented,
            "`{path}` {kind} example `{name}` on the page drifted from the document"
        );
        paths.insert(path);
    }
    for path in [
        "/operations",
        "/connections",
        "/catalog",
        "/events",
        "/datasources",
        "/mcp",
    ] {
        assert!(
            paths.contains(path),
            "the page shows an example for `{path}`"
        );
    }
}

#[tokio::test]
async fn the_docs_page_makes_zero_external_requests() {
    let doc = document();
    let page = docs_page().await;
    // The only absolute origins the page may name are the ones the contract itself
    // declares (server URLs; an identity origin, were the document to name one). Today
    // every documented server URL is relative and no origin is named, so the allowed
    // set is empty and the page must carry no absolute http(s) URL at all.
    let allowed: Vec<&str> = doc["servers"]
        .as_array()
        .expect("the document declares servers")
        .iter()
        .filter_map(|server| server["url"].as_str())
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"))
        .collect();
    for scheme in ["http://", "https://"] {
        let mut rest = page.as_str();
        while let Some(start) = rest.find(scheme) {
            let tail = &rest[start..];
            let url = tail
                .split(|character: char| {
                    character.is_whitespace() || matches!(character, '"' | '\'' | '<' | ')')
                })
                .next()
                .unwrap_or(tail);
            assert!(
                allowed.iter().any(|origin| url.starts_with(origin)),
                "the page names a foreign absolute URL: `{url}`"
            );
            rest = &tail[scheme.len()..];
        }
    }
    for fetching in [
        "<script", "<img", "<link", "<iframe", "<object", "<embed", "<video", "<audio", "src=",
        "srcset=", "@import", "url(",
    ] {
        assert!(
            !page.contains(fetching),
            "the page must not carry `{fetching}`: one self-contained document, zero requests"
        );
    }
}

#[tokio::test]
async fn the_docs_page_links_the_contract_and_renders_its_version() {
    let doc = document();
    let page = docs_page().await;
    assert!(
        page.contains("href=\"openapi.json\""),
        "the page links the machine-readable contract relatively"
    );
    let version = doc["info"]["version"]
        .as_str()
        .expect("the document declares its version");
    assert!(
        page.contains(&format!("<span class=\"version\">{version}</span>")),
        "the page renders the document's version"
    );
    assert!(
        page.contains(CONNECTORS_AUDIENCE),
        "the page names the identity audience the document pins"
    );
    assert!(
        page.contains("/v1/access-token"),
        "the page shows the token mint step"
    );
}

#[tokio::test]
async fn the_docs_page_refusal_table_carries_every_documented_code() {
    let doc = document();
    let page = docs_page().await;
    // The closed vocabularies surface as generated enums inside the response
    // envelopes; the catalog code is wire-open, so its row carries the four codes
    // the documented refusal examples name. Same extraction the page renderer uses.
    let rows = crate::hosted::docs::refusal_rows_for_tests(&doc);
    assert_eq!(
        rows.len(),
        5,
        "the refusal table stays one row per envelope endpoint"
    );
    for (family, codes) in rows {
        assert!(!codes.is_empty(), "`{family}` documents at least one code");
        for code in codes {
            assert!(
                page.contains(&format!("<code>{code}</code>")),
                "the refusal table misses `{code}` from `{family}`"
            );
        }
    }
}

#[tokio::test]
async fn every_documented_mcp_request_example_is_answered_by_the_live_transport() {
    let doc = document();
    for (name, value) in request_examples(&doc, "/mcp") {
        let response = test_router()
            .oneshot(
                Request::post("/mcp")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, "Bearer access")
                    .body(Body::from(serde_json::to_vec(&value).expect("encode")))
                    .expect("request"),
            )
            .await
            .expect("infallible");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "MCP example `{name}` is answered"
        );
        let body = axum::body::to_bytes(response.into_body(), OPERATION_MAX_FRAME_BYTES)
            .await
            .expect("bounded body");
        let frame: Value = serde_json::from_slice(&body).expect("a JSON-RPC response");
        assert_eq!(frame["jsonrpc"].as_str(), Some("2.0"));
        assert!(
            frame.get("result").is_some(),
            "MCP example `{name}` produces a result, got {frame}"
        );
    }
}
