//! Drift tests for the committed OpenAPI contract served at `/openapi.json` (S-067).
//!
//! The document is an authored artifact, not a projection, so these tests are what keep it
//! honest: every embedded request example must be accepted by the exact protocol type the
//! route deserializes with, every refusal example must name a code the protocol's closed
//! error vocabulary actually contains, and every documented route must exist in the real
//! router. A deliberately wrong example — an unknown field anywhere — must be refused, which
//! proves the `deny_unknown_fields` discipline is exercised rather than assumed.

use serde_json::Value;
use sha2::Digest as _;

use super::*;

/// The exact bytes `docs.rs` embeds and the route serves verbatim.
const DOC: &str = include_str!("../docs/openapi.json");

/// The five envelope endpoints and how many distinct request examples each must carry —
/// one per method in the closed set, so a method the document forgets fails here.
const ENVELOPE_EXAMPLE_FLOORS: [(&str, usize); 5] = [
    ("/operations", 7),
    ("/connections", 8),
    ("/catalog", 2),
    ("/events", 3),
    ("/datasources", 4),
];

fn test_router() -> Router {
    router(
        Arc::new(Verifier),
        Arc::new(Backend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
    )
}

fn document() -> Value {
    serde_json::from_str(DOC).expect("the committed OpenAPI document is JSON")
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
        DOC.as_bytes(),
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
        assert!(
            methods.len() >= floor,
            "`{path}` documents an example for each of its {floor} methods, found {methods:?}"
        );
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
                        // The catalog code is an open string on the wire; the hosted route
                        // produces exactly these four, and the document's enum must agree.
                        let documented = doc["components"]["schemas"]["catalog.error"]
                            ["properties"]["code"]["enum"]
                            .as_array()
                            .expect("the document closes the catalog error codes")
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<BTreeSet<_>>();
                        assert_eq!(
                            documented,
                            BTreeSet::from([
                                "protocol",
                                "invalid_input",
                                "not_found",
                                "not_granted"
                            ]),
                            "the documented catalog codes are the ones the hosted route produces"
                        );
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
