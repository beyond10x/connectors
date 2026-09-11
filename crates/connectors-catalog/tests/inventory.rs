use connectors_catalog::inventory::{Location, extract};
use serde_json::json;

fn document() -> serde_json::Value {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "1"},
        "paths": {
            "/projects": {
                "parameters": [{"name": "tenant", "in": "header", "required": true}],
                "get": {
                    "operationId": "listProjects",
                    "parameters": [{"name": "page", "in": "query"}],
                    "responses": {
                        "200": {"content": {"application/json": {}}},
                        "default": {"content": {"application/json": {}}}
                    }
                },
                "post": {
                    "requestBody": {"content": {"application/json": {}, "text/plain": {}}},
                    "responses": {"201": {"content": {"application/json": {}}}}
                }
            },
            "/projects/{id}": {
                "get": {
                    "operationId": "readProject",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true},
                        {"name": "trace", "in": "cookie"}
                    ],
                    "responses": {"200": {"content": {"application/json": {}}}}
                }
            }
        }
    })
}

#[test]
fn the_same_document_yields_a_byte_identical_inventory_twice() {
    let first = serde_json::to_vec(&extract(&document())).expect("serialisable");
    let second = serde_json::to_vec(&extract(&document())).expect("serialisable");
    assert_eq!(first, second);
}

#[test]
fn an_operation_without_an_id_is_still_inventoried() {
    let inventory = extract(&document());
    let anonymous = inventory
        .operations
        .iter()
        .find(|o| o.method == "post")
        .expect("the POST is inventoried");
    assert!(anonymous.operation_id.is_none());
    assert_eq!(anonymous.designation(), "POST /projects");
    assert_eq!(
        anonymous.request_media_types,
        vec!["application/json", "text/plain"]
    );
}

#[test]
fn path_level_parameters_reach_the_operations_under_them() {
    let inventory = extract(&document());
    let list = inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some("listProjects"))
        .expect("listProjects is inventoried");
    let names: Vec<&str> = list.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["tenant", "page"]);
    assert_eq!(list.parameters[0].location, Location::Header);
    assert!(list.parameters[0].required);
    assert!(!list.parameters[1].required);
    let statuses: Vec<&str> = list.responses.iter().map(|r| r.status.as_str()).collect();
    assert_eq!(statuses, vec!["200", "default"]);
}

#[test]
fn every_unrepresentable_thing_is_named_with_its_operation() {
    let document = json!({
        "openapi": "3.1.0",
        "webhooks": {"onPush": {}},
        "paths": {
            "/a": {"get": {
                "operationId": "refBody",
                "requestBody": {"$ref": "#/components/requestBodies/Thing"},
                "responses": {}
            }},
            "/b": {"get": {
                "operationId": "oddLocation",
                "parameters": [{"name": "who", "in": "matrix"}],
                "responses": {}
            }},
            "/c": {"$ref": "./other.json#/paths/~1c"}
        }
    });
    let inventory = extract(&document);
    let (inventoried, gaps) = inventory.coverage();
    assert_eq!(inventoried, 2, "the $ref path item is not inventoried");
    // webhooks, the $ref body, the unknown parameter location, and the $ref path item.
    assert_eq!(gaps, 4);
    let reasons: Vec<(&str, &str)> = inventory
        .unsupported
        .iter()
        .map(|u| (u.designation.as_str(), u.reason.as_str()))
        .collect();
    assert!(reasons.contains(&("document.webhooks", "`webhooks` is a document member this build does not read")));
    assert!(reasons.iter().any(|(d, r)| *d == "refBody" && r.contains("request body is a $ref")));
    assert!(reasons.iter().any(|(d, r)| *d == "oddLocation" && r.contains("`matrix`")));
    assert!(reasons.iter().any(|(d, _)| *d == "/c"));
}

#[test]
fn coverage_reports_its_two_numbers_apart() {
    let inventory = extract(&document());
    let (inventoried, gaps) = inventory.coverage();
    assert_eq!(inventoried, 3);
    assert_eq!(gaps, 0);
}

#[test]
fn a_document_without_paths_is_empty_rather_than_an_error() {
    let inventory = extract(&json!({"openapi": "3.0.0"}));
    assert_eq!(inventory.coverage(), (0, 0));
}
