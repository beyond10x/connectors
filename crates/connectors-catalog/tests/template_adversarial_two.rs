//! Second adversarial pass against `story:catalog-operation-template`. It attacks
//! the fixes, not the defects the first pass reported: chiefly the `location:name`
//! key convention, which is the implementation's own invention and is therefore
//! the part of the unit no document constrains.

use connectors_catalog::inventory::{self, Operation};
use connectors_catalog::template::Template;
use serde_json::json;
use std::collections::BTreeMap;

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The one operation a document yields, so every case asserts against a state the
/// crate's own ingest path produces rather than one hand-built here.
fn extracted(path: &str, declared: serde_json::Value) -> Operation {
    let document = json!({
        "openapi": "3.1.0",
        "paths": { path: {
            "get": { "operationId": "op", "parameters": declared, "responses": {} }
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

fn template(operation: Operation) -> Template {
    Template::from_operation(&operation).expect("the fixture operation is representable")
}

/// A refusal exists so a caller can act on it. When `id` is declared in two
/// locations and the higher-ranked one is required, supplying `id` refuses
/// `ValueAbsent("id")` — naming the key the caller has already supplied, whose
/// reason ("required parameter `id` has no value") is false, and which no
/// re-supply of that key can ever satisfy. The qualified spelling the caller needs
/// appears nowhere in the refusal.
#[test]
fn a_value_absent_refusal_names_a_key_the_caller_has_not_already_supplied() {
    let operation = extracted(
        "/things",
        json!([
            {"name": "trace", "in": "query", "required": false},
            {"name": "trace", "in": "header", "required": true}
        ]),
    );
    let supplied = values(&[("trace", "abc")]);
    let refusal = template(operation)
        .bind(&supplied, None)
        .expect_err("the header `trace` is required and a bare key cannot reach it");
    assert!(
        !supplied.contains_key(refusal.subject()),
        "the refusal names `{}`, which the caller supplied: `{}`. Re-supplying it \
         cannot change the outcome",
        refusal.subject(),
        refusal.reason()
    );
}

/// `bind`'s own documentation says a bare name reaching only the first location by
/// rank means "one value can never populate two parameters". `location:name` is
/// string concatenation into a namespace that already contains arbitrary strings,
/// so a query parameter literally named `query:trace` and one named `trace`
/// collide: the single key `query:trace` is the qualified spelling of one and the
/// bare name of the other, and both are emitted.
#[test]
fn one_supplied_value_never_populates_two_parameters() {
    let operation = extracted(
        "/things",
        json!([
            {"name": "trace", "in": "query", "required": false},
            {"name": "query:trace", "in": "query", "required": false}
        ]),
    );
    let binding = template(operation)
        .bind(&values(&[("query:trace", "V")]), None)
        .expect("one key, for the parameter literally named `query:trace`");
    assert_eq!(
        binding.query.len(),
        1,
        "one supplied value produced {} query pairs: {:?}",
        binding.query.len(),
        binding.query
    );
}

/// The Scope refuses a cookie "rather than silently dropped", and refuses a path
/// parameter the path templates nowhere for the same reason. A value supplied
/// under a bare name that `bind` accepts as declared, and that the precedence rule
/// then makes unreachable, is the same silent drop: it passes the undeclared-key
/// check, reaches no part of the request, and raises nothing.
#[test]
fn a_value_under_an_accepted_key_is_not_silently_dropped() {
    let operation = extracted(
        "/things",
        json!([
            {"name": "trace", "in": "query", "required": false},
            {"name": "trace", "in": "header", "required": false}
        ]),
    );
    let binding = template(operation)
        .bind(
            &values(&[("trace", "bare"), ("query:trace", "qualified")]),
            None,
        )
        .expect("both keys pass the undeclared-key check");
    let carried: Vec<&String> = binding
        .query
        .iter()
        .chain(binding.headers.iter())
        .map(|(_, value)| value)
        .collect();
    assert!(
        carried.iter().any(|value| value.as_str() == "bare"),
        "the value supplied under the accepted key `trace` reaches nothing and \
         nothing was refused: query {:?}, headers {:?}",
        binding.query,
        binding.headers
    );
}

/// `header_safe` rejects control bytes and DEL, which stops a header name from
/// ending its own line. It does not make the name a header name: RFC 9110
/// `field-name` is a `token`, and a name holding `:`, SP or `(` is not one. Such a
/// name is written into the line the same way and produces a field no parser
/// reads as the document meant.
#[test]
fn a_document_header_name_that_is_not_an_http_token_is_refused() {
    const TCHAR_SYMBOLS: &str = "!#$%&'*+-.^_`|~";
    let not_tokens = [
        "X-Trace: injected",
        "X Trace",
        "",
        "X-Trace,Other",
        "@X-Trace",
    ];
    let accepted: Vec<&str> = not_tokens
        .into_iter()
        .filter(|name| {
            let operation = extracted(
                "/things",
                json!([{"name": name, "in": "header", "required": false}]),
            );
            Template::from_operation(&operation).is_ok()
        })
        .collect();
    // Stated here rather than inlined, so the rule the case applies is visible.
    assert!(
        !TCHAR_SYMBOLS.contains(':') && !TCHAR_SYMBOLS.contains(' '),
        "the token grammar this case applies must exclude `:` and SP"
    );
    assert_eq!(
        accepted,
        Vec::<&str>::new(),
        "these document-declared header names are not HTTP tokens and were \
         accepted: {accepted:?}"
    );
}
