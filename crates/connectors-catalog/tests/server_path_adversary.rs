//! Adversarial cases for story:catalog-confluence-reads: the server-path rule
//! `inventory::extract` gained. OpenAPI 3.x lets a path item and an operation
//! declare their own `servers`, which override the document's for that path or
//! operation. An inventory that records every path below the document's server
//! path must either honour such an override or name it unsupported; recording
//! the path below the document's base sends the request somewhere the document
//! does not say it lives.
use connectors_catalog::inventory::extract;
use serde_json::{Value, json};

fn document(paths: Value) -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {"title": "Fixture", "version": "1"},
        "servers": [{"url": "https://{your-domain}/wiki/api/v2",
                     "variables": {"your-domain": {"default": "site.example"}}}],
        "paths": paths
    })
}

fn path_of(document: &Value, operation_id: &str) -> Option<String> {
    extract(document)
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some(operation_id))
        .map(|o| o.path.clone())
}

fn named_unsupported(document: &Value, needle: &str) -> bool {
    extract(document)
        .unsupported
        .iter()
        .any(|gap| gap.designation.contains(needle))
}

#[test]
fn a_path_item_servers_override_is_honoured_or_named() {
    let document = document(json!({
        "/pages": {"get": {"operationId": "getPages", "responses": {}}},
        "/user/current": {
            "servers": [{"url": "https://{your-domain}/wiki/rest/api",
                         "variables": {"your-domain": {"default": "site.example"}}}],
            "get": {"operationId": "getCurrentUser", "responses": {}}
        }
    }));
    assert_eq!(
        path_of(&document, "getPages").as_deref(),
        Some("/wiki/api/v2/pages")
    );
    let recorded = path_of(&document, "getCurrentUser");
    assert!(
        recorded.as_deref() == Some("/wiki/rest/api/user/current")
            || named_unsupported(&document, "/user/current"),
        "a path-level server override was ignored: recorded {recorded:?}, not named unsupported"
    );
}

#[test]
fn an_operation_servers_override_is_honoured_or_named() {
    let document = document(json!({
        "/user/current": {
            "get": {
                "operationId": "getCurrentUser",
                "servers": [{"url": "https://{your-domain}/wiki/rest/api",
                             "variables": {"your-domain": {"default": "site.example"}}}],
                "responses": {}
            }
        }
    }));
    let recorded = path_of(&document, "getCurrentUser");
    assert!(
        recorded.as_deref() == Some("/wiki/rest/api/user/current")
            || named_unsupported(&document, "/user/current"),
        "an operation-level server override was ignored: recorded {recorded:?}, not named unsupported"
    );
}

/// Shapes the unit's own tests do not name: a templated port, an uppercase
/// scheme, a query and a fragment on the server URL. Each must land under
/// `/wiki/api/v2` and raise nothing.
#[test]
fn authority_shapes_do_not_move_the_base() {
    for url in [
        "https://{host}:{port}/wiki/api/v2/",
        "HTTPS://Site.Example/wiki/api/v2?x=1#f",
        "{scheme}://site.example/wiki/api/v2",
    ] {
        let document = json!({
            "openapi": "3.0.3",
            "info": {"title": "Fixture", "version": "1"},
            "servers": [{"url": url}],
            "paths": {"/pages": {"get": {"operationId": "getPages", "responses": {}}}}
        });
        let inventory = extract(&document);
        assert_eq!(inventory.operations[0].path, "/wiki/api/v2/pages", "{url}");
        assert!(inventory.unsupported.is_empty(), "{url}");
    }
}
