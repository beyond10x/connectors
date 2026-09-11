//! Adversarial cases against `story:catalog-operation-template`. Each case drives
//! the implementation from the story's own Outcome/Scope/Acceptance text, or from
//! a document `inventory::extract` accepts, rather than from the shape the
//! implementation happens to have.

use connectors_catalog::inventory::{self, Location, Operation, Parameter};
use connectors_catalog::template::{Binding, Refusal, Template};
use serde_json::json;
use std::collections::BTreeMap;

fn parameter(name: &str, location: Location, required: bool) -> Parameter {
    Parameter {
        name: name.to_owned(),
        location,
        required,
    }
}

fn operation() -> Operation {
    Operation {
        method: "get".into(),
        path: "/projects/{id}/issues".into(),
        operation_id: Some("listIssues".into()),
        parameters: vec![
            parameter("id", Location::Path, true),
            parameter("page", Location::Query, false),
            parameter("tenant", Location::Header, true),
        ],
        request_media_types: vec!["application/json".into()],
        responses: Vec::new(),
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The one operation a document yields, so a case can show the state it asserts
/// against is one the crate's own ingest path produces.
fn extracted(path: &str, shared: serde_json::Value, own: serde_json::Value) -> Operation {
    let document = json!({
        "openapi": "3.1.0",
        "paths": { path: {
            "parameters": shared,
            "get": { "operationId": "listIssues", "parameters": own, "responses": {} }
        }}
    });
    let inventory = inventory::extract(&document);
    assert_eq!(
        inventory.unsupported,
        Vec::new(),
        "the fixture document must be one this pass represents without gaps"
    );
    inventory
        .operations
        .into_iter()
        .next()
        .expect("one operation")
}

/// RFC 3986 section 5.2.4, the rule every client applies before a request leaves.
/// A template whose path survives it unchanged cannot have its structure moved by
/// a value.
fn remove_dot_segments(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.join("/")
}

/// Acceptance: "a value containing a slash or a space is encoded rather than
/// changing the path structure". A dot-segment needs neither: `.` is unreserved,
/// so `..` passes the encoder untouched and the client resolves the segment away.
#[test]
fn a_dot_segment_value_cannot_move_the_path_the_operation_identifies() {
    let declared = remove_dot_segments("/projects/PLACEHOLDER/issues");
    match template_of(operation()).bind(&values(&[("id", ".."), ("tenant", "acme")]), None) {
        // Refusing the value is one correct answer.
        Err(_) => {}
        Ok(binding) => {
            let resolved = remove_dot_segments(&binding.path);
            assert_eq!(
                resolved.split('/').count(),
                declared.split('/').count(),
                "bound path `{}` resolves to `{resolved}`, which is not the path the \
                 operation identifies",
                binding.path
            );
        }
    }
}

/// The module claims "What the caller's values cannot be made to say safely is
/// refused by name rather than encoded away into a request that means something
/// else." A CRLF in a header value says something else: it ends the header and
/// starts another one.
#[test]
fn a_header_value_holding_a_line_break_is_not_carried_verbatim() {
    match template_of(operation()).bind(
        &values(&[("id", "42"), ("tenant", "acme\r\nX-Internal-Admin: true")]),
        None,
    ) {
        Err(_) => {}
        Ok(binding) => {
            let injected: Vec<&(String, String)> = binding
                .headers
                .iter()
                .filter(|(_, value)| value.contains('\r') || value.contains('\n'))
                .collect();
            assert!(
                injected.is_empty(),
                "a header value carries a raw line break into the request: {injected:?}"
            );
        }
    }
}

/// OpenAPI 3.1 §4.8.9.1: an operation-level parameter overrides the path-level
/// one with the same name and location. `extract` keeps both, and the template
/// emits both, so the query carries the pair twice.
#[test]
fn an_operation_level_parameter_replaces_the_path_level_one_it_overrides() {
    let operation = extracted(
        "/projects/{id}/issues",
        json!([
            {"name": "id", "in": "path", "required": true},
            {"name": "page", "in": "query", "required": false}
        ]),
        json!([{"name": "page", "in": "query", "required": true}]),
    );
    let binding = template_of(operation)
        .bind(&values(&[("id", "42"), ("page", "2")]), None)
        .expect("every declared parameter has a value");
    assert_eq!(
        binding.query_string(),
        "page=2",
        "the overridden path-level `page` is emitted alongside the override: {:?}",
        binding.query
    );
}

/// The same override, seen through `required`: the operation relaxes a path-level
/// requirement, and the path-level copy still refuses.
#[test]
fn an_operation_level_override_of_required_is_honoured() {
    let operation = extracted(
        "/projects/{id}/issues",
        json!([
            {"name": "id", "in": "path", "required": true},
            {"name": "page", "in": "query", "required": true}
        ]),
        json!([{"name": "page", "in": "query", "required": false}]),
    );
    let binding = template_of(operation)
        .bind(&values(&[("id", "42")]), None)
        .expect("the operation-level declaration makes `page` optional");
    assert!(binding.query.is_empty(), "{:?}", binding.query);
}

/// A parameter is identified by name *and* location, so `id` in the path and `id`
/// in the query are two parameters and a legal document. The bind map is keyed by
/// name alone, so the path value silently populates a query parameter the caller
/// gave no value for — which the suite's own
/// `an_optional_parameter_without_a_value_is_simply_absent` says must not happen.
#[test]
fn a_name_shared_across_two_locations_does_not_leak_one_value_into_both() {
    let operation = extracted(
        "/projects/{id}/issues",
        json!([]),
        json!([
            {"name": "id", "in": "path", "required": true},
            {"name": "id", "in": "query", "required": false}
        ]),
    );
    let binding = template_of(operation)
        .bind(&values(&[("id", "42")]), None)
        .expect("the path parameter has a value; the query one is optional");
    assert_eq!(binding.path, "/projects/42/issues");
    assert!(
        binding.query.is_empty(),
        "the optional query `id` was never supplied a value, yet it is emitted: {:?}",
        binding.query
    );
}

/// The Scope refuses "a path placeholder the operation declares no parameter
/// for"; the converse is unchecked. A declared path parameter the path does not
/// template is accepted, and the value the caller supplied for it reaches no part
/// of the request — the silent drop the Scope's cookie sentence rules out.
#[test]
fn a_path_parameter_with_no_placeholder_is_not_silently_dropped() {
    let operation = extracted(
        "/projects/{id}/issues",
        json!([]),
        json!([
            {"name": "id", "in": "path", "required": true},
            {"name": "scope", "in": "path", "required": true}
        ]),
    );
    let template = Template::from_operation(&operation);
    let Ok(template) = template else {
        // Refusing at construction is one correct answer.
        return;
    };
    let Ok(binding) = template.bind(&values(&[("id", "42"), ("scope", "all")]), None) else {
        return;
    };
    assert!(
        carries(&binding, "all"),
        "`scope` was declared, a value was supplied, and it reaches nothing: {binding:?}"
    );
}

/// An unterminated `{` is a malformed path, not an undeclared parameter. Here the
/// parameter it names *is* declared, so the refusal's own reason is false.
#[test]
fn an_unterminated_placeholder_is_not_reported_as_an_undeclared_parameter() {
    let mut operation = operation();
    operation.path = "/projects/{id".into();
    let refusal = Template::from_operation(&operation)
        .expect_err("a path with an unterminated `{` is not bindable");
    assert_ne!(
        refusal,
        Refusal::PlaceholderUndeclared("id".into()),
        "`id` is declared as a path parameter, yet the refusal reads `{}`",
        refusal.reason()
    );
}

fn template_of(operation: Operation) -> Template {
    Template::from_operation(&operation).expect("the fixture operation is representable")
}

fn carries(binding: &Binding, value: &str) -> bool {
    binding.path.contains(value)
        || binding.query.iter().any(|(_, v)| v == value)
        || binding.headers.iter().any(|(_, v)| v == value)
}
