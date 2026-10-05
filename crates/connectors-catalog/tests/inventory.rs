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
    assert!(reasons.contains(&(
        "document.webhooks",
        "`webhooks` is a document member this build does not read"
    )));
    assert!(
        reasons
            .iter()
            .any(|(d, r)| *d == "refBody" && r.contains("request body is a $ref"))
    );
    assert!(
        reasons
            .iter()
            .any(|(d, r)| *d == "oddLocation" && r.contains("`matrix`"))
    );
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

#[test]
fn an_operation_level_parameter_replaces_the_path_level_one_it_overrides() {
    // OpenAPI 3.1 section 4.8.9.1: a parameter at the operation overrides the one
    // the path item declares with the same name and location.
    let document = json!({
        "openapi": "3.1.0",
        "paths": {"/projects/{id}": {
            "parameters": [
                {"name": "id", "in": "path", "required": true},
                {"name": "page", "in": "query", "required": true}
            ],
            "get": {
                "operationId": "readProject",
                "parameters": [{"name": "page", "in": "query", "required": false}],
                "responses": {}
            }
        }}
    });
    let inventory = extract(&document);
    let operation = &inventory.operations[0];
    let declared: Vec<(&str, bool)> = operation
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p.required))
        .collect();
    assert_eq!(declared, vec![("id", true), ("page", false)]);
}

#[test]
fn a_parameter_in_another_location_is_not_an_override() {
    // Identity is name and location together: `id` in the path and `id` in the
    // query are two parameters, and neither replaces the other.
    let document = json!({
        "openapi": "3.1.0",
        "paths": {"/projects/{id}": {
            "parameters": [{"name": "id", "in": "path", "required": true}],
            "get": {
                "operationId": "readProject",
                "parameters": [{"name": "id", "in": "query", "required": false}],
                "responses": {}
            }
        }}
    });
    let operation = &extract(&document).operations[0];
    let declared: Vec<(&str, Location)> = operation
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p.location))
        .collect();
    assert_eq!(
        declared,
        vec![("id", Location::Path), ("id", Location::Query)]
    );
}

fn served(servers: serde_json::Value) -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": {"title": "Fixture", "version": "1"},
        "servers": servers,
        "paths": {"/pages/{id}": {"get": {"operationId": "readPage", "responses": {}}}}
    })
}

#[test]
fn a_path_is_recorded_below_its_servers_base_path() {
    // OpenAPI 3.x: an operation path is appended to the server URL, so a
    // document served under `/wiki/api/v2` sends `/wiki/api/v2/pages/{id}`.
    for url in [
        "https://{your-domain}/wiki/api/v2",
        "https://site.example/wiki/api/v2/",
        "//site.example/wiki/api/v2",
        "/wiki/api/v2",
    ] {
        let inventory = extract(&served(json!([{"url": url}])));
        assert_eq!(
            inventory.operations[0].path, "/wiki/api/v2/pages/{id}",
            "{url}"
        );
        assert!(inventory.unsupported.is_empty(), "{url}");
    }
}

#[test]
fn servers_at_the_root_leave_the_paths_as_written() {
    for servers in [
        json!([{"url": "https://{hostname}"}]),
        json!([{"url": "https://your-domain.example"}, {"url": "https://other.example/"}]),
        json!([]),
    ] {
        let inventory = extract(&served(servers.clone()));
        assert_eq!(inventory.operations[0].path, "/pages/{id}", "{servers}");
        assert!(inventory.unsupported.is_empty(), "{servers}");
    }
    let inventory = extract(&document());
    assert_eq!(inventory.operations[0].path, "/projects");
}

#[test]
fn servers_without_one_base_path_are_named_and_the_paths_kept() {
    for servers in [
        json!([{"url": "https://a.example/v1"}, {"url": "https://a.example/v2"}]),
        json!([{"url": "https://a.example/{version}"}]),
        json!([{"url": "v2"}]),
        json!([{"description": "no url"}]),
        json!({"url": "/v2"}),
    ] {
        let inventory = extract(&served(servers.clone()));
        assert_eq!(inventory.operations[0].path, "/pages/{id}", "{servers}");
        assert_eq!(inventory.unsupported.len(), 1, "{servers}");
        assert_eq!(inventory.unsupported[0].designation, "document.servers");
    }
}

#[test]
fn a_servers_override_is_named_and_not_recorded_under_the_document_base() {
    let own = json!([{"url": "https://site.example/wiki/rest/api"}]);
    for paths in [
        json!({"/user/current": {"servers": own, "get": {"operationId": "getCurrentUser"}}}),
        json!({"/user/current": {"get": {"operationId": "getCurrentUser", "servers": own}}}),
    ] {
        let mut document = served(json!([{"url": "https://site.example/wiki/api/v2"}]));
        document["paths"] = paths.clone();
        let inventory = extract(&document);
        assert!(inventory.operations.is_empty(), "{paths}");
        assert_eq!(inventory.unsupported.len(), 1, "{paths}");
        assert!(
            inventory.unsupported[0]
                .designation
                .contains("/user/current"),
            "{paths}"
        );
        assert!(
            inventory.unsupported[0]
                .reason
                .contains("declares its own `servers`"),
            "{paths}"
        );
    }
}

/// One operation whose query parameters cover every array shape this pass
/// reads, beside a scalar and a header array that must stay as they were.
fn array_document() -> serde_json::Value {
    let query = |name: &str, schema: serde_json::Value| json!({"name": name, "in": "query", "schema": schema});
    json!({
        "openapi": "3.1.0",
        "paths": {"/messages": {"get": {
            "operationId": "listMessages",
            "parameters": [
                query("labelIds", json!({"type": "array", "items": {"type": "string"}})),
                {"name": "eventTypes", "in": "query", "style": "form", "explode": true,
                 "schema": {"type": "array", "items": {"type": "integer"}}},
                query("nullable", json!({"type": ["array", "null"], "items": {"type": "boolean"}})),
                query("untyped", json!({"type": "array", "items": {"$ref": "#/components/schemas/Id"}})),
                query("q", json!({"type": "string"})),
                {"name": "joined", "in": "query", "style": "form", "explode": false,
                 "schema": {"type": "array", "items": {"type": "string"}}},
                {"name": "spaced", "in": "query", "style": "spaceDelimited",
                 "schema": {"type": "array", "items": {"type": "string"}}},
                {"name": "piped", "in": "query", "style": "pipeDelimited", "explode": true,
                 "schema": {"type": "array", "items": {"type": "string"}}},
                {"name": "trace", "in": "header", "schema": {"type": "array", "items": {"type": "string"}}}
            ],
            "responses": {}
        }}}
    })
}

#[test]
fn inventory_marks_form_explode_array_repeated() {
    use connectors_catalog::inventory::ValueType;
    let inventory = extract(&array_document());
    let parameters = &inventory.operations[0].parameters;
    let shape = |name: &str| {
        let parameter = parameters
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("`{name}` is inventoried"));
        (parameter.repeated, parameter.value_type)
    };
    // `style` and `explode` absent are OpenAPI's query defaults: form, exploded.
    assert_eq!(shape("labelIds"), (true, Some(ValueType::String)));
    assert_eq!(shape("eventTypes"), (true, Some(ValueType::Integer)));
    assert_eq!(shape("nullable"), (true, Some(ValueType::Boolean)));
    // Repeated, with elements of no type this model carries.
    assert_eq!(shape("untyped"), (true, None));
    assert_eq!(shape("q"), (false, Some(ValueType::String)));
    // Arrays outside the query are out of this pass: recorded as they were.
    assert_eq!(shape("trace"), (false, None));
    // A parameter that is not repeated serialises without the field, so a
    // bundle with no repeated parameter keeps its bytes; one that is says so.
    let serialised = serde_json::to_value(parameters).unwrap();
    let by_name = |name: &str| {
        serialised
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(
        by_name("q"),
        json!({"name": "q", "location": "query", "required": false, "type": "string"})
    );
    assert_eq!(by_name("labelIds")["repeated"], json!(true));
    assert_eq!(by_name("labelIds")["type"], json!("string"));
    let back: Vec<connectors_catalog::inventory::Parameter> =
        serde_json::from_value(serialised).unwrap();
    assert_eq!(&back, parameters);
}

#[test]
fn inventory_records_unsupported_array_style() {
    let inventory = extract(&array_document());
    let parameters = &inventory.operations[0].parameters;
    for name in ["joined", "spaced", "piped"] {
        let parameter = parameters
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("`{name}` stays inventoried"));
        // Kept as the scalar it was read as before: a caller that sends the
        // joined string itself is not refused by this pass.
        assert!(!parameter.repeated, "{name}");
        assert_eq!(parameter.value_type, None, "{name}");
    }
    let reasons: Vec<(&str, &str)> = inventory
        .unsupported
        .iter()
        .map(|u| (u.designation.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(reasons.len(), 3, "{reasons:?}");
    for (name, shape) in [
        ("joined", "style `form`, explode false"),
        ("spaced", "style `spaceDelimited`, explode false"),
        ("piped", "style `pipeDelimited`, explode true"),
    ] {
        assert!(
            reasons.iter().any(|(d, r)| *d == "listMessages"
                && r.contains(&format!("`{name}`"))
                && r.contains(shape)),
            "{name}: {reasons:?}"
        );
    }
}

#[test]
fn an_overridden_array_parameter_records_no_gap_for_the_declaration_it_replaced() {
    let document = json!({
        "openapi": "3.1.0",
        "paths": {"/messages": {
            "parameters": [{"name": "labelIds", "in": "query", "explode": false,
                            "schema": {"type": "array", "items": {"type": "string"}}}],
            "get": {
                "operationId": "listMessages",
                "parameters": [{"name": "labelIds", "in": "query",
                                "schema": {"type": "array", "items": {"type": "string"}}}],
                "responses": {}
            }
        }}
    });
    let inventory = extract(&document);
    assert!(
        inventory.unsupported.is_empty(),
        "{:?}",
        inventory.unsupported
    );
    assert!(inventory.operations[0].parameters[0].repeated);
}

/// Parameters declared once under `components.parameters` and referenced by
/// operations and path items, as vendor documents commonly write them.
fn referenced_document() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "components": {"parameters": {
            "ThingId": {"name": "thing_id", "in": "path", "required": true,
                        "schema": {"type": "integer", "format": "int64"}},
            "Since": {"name": "start_time", "in": "query", "required": true,
                      "schema": {"type": "integer"}},
            "Alias": {"$ref": "#/components/parameters/Since"},
            "a/b": {"name": "escaped", "in": "query", "schema": {"type": "string"}},
            "LoopA": {"$ref": "#/components/parameters/LoopB"},
            "LoopB": {"$ref": "#/components/parameters/LoopA"},
            "Page": {"name": "page", "in": "query", "style": "deepObject", "explode": true,
                     "schema": {"type": "object", "properties": {
                         "size": {"type": "integer", "minimum": 1},
                         "after": {"type": "string"}}}}
        }},
        "paths": {
            "/things/{thing_id}": {
                "parameters": [{"$ref": "#/components/parameters/ThingId"}],
                "get": {
                    "operationId": "showThing",
                    "parameters": [{"$ref": "#/components/parameters/a~1b"}],
                    "responses": {}
                }
            },
            "/things": {"get": {
                "operationId": "listThings",
                "parameters": [
                    {"$ref": "#/components/parameters/Alias"},
                    {"$ref": "#/components/parameters/Page"}
                ],
                "responses": {}
            }},
            "/broken": {"get": {
                "operationId": "brokenRefs",
                "parameters": [
                    {"$ref": "#/components/parameters/Missing"},
                    {"$ref": "#/components/parameters/LoopA"},
                    {"$ref": "./other.yaml#/components/parameters/ThingId"},
                    {"$ref": "#/components/schemas/ThingId"}
                ],
                "responses": {}
            }}
        }
    })
}

#[test]
fn a_local_parameter_reference_is_read_as_the_parameter_it_names() {
    use connectors_catalog::inventory::{Parameter, ValueType};
    let inventory = extract(&referenced_document());
    let operation = |id: &str| {
        inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(id))
            .unwrap()
    };
    assert_eq!(
        operation("showThing").parameters,
        vec![
            Parameter {
                name: "thing_id".into(),
                location: Location::Path,
                required: true,
                value_type: Some(ValueType::Integer),
                repeated: false,
            },
            // `a~1b` is the JSON pointer spelling of the component `a/b`.
            Parameter {
                name: "escaped".into(),
                location: Location::Query,
                required: false,
                value_type: Some(ValueType::String),
                repeated: false,
            },
        ]
    );
    // Followed through a reference to a reference.
    let since = &operation("listThings").parameters[0];
    assert_eq!((since.name.as_str(), since.required), ("start_time", true));
    assert!(
        inventory
            .unsupported
            .iter()
            .all(|u| u.designation == "brokenRefs"),
        "{:?}",
        inventory.unsupported
    );
}

#[test]
fn a_reference_that_names_nothing_loops_or_leaves_the_document_is_named() {
    let inventory = extract(&referenced_document());
    let broken = inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some("brokenRefs"))
        .unwrap();
    assert!(broken.parameters.is_empty());
    let reasons: Vec<&str> = inventory
        .unsupported
        .iter()
        .filter(|u| u.designation == "brokenRefs")
        .map(|u| u.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        vec![
            "parameter $ref `#/components/parameters/Missing` names no parameter",
            "parameter $ref chain is longer than 8 references or loops",
            "parameter is a $ref this pass does not resolve",
            "parameter is a $ref this pass does not resolve",
        ]
    );
}

#[test]
fn a_deep_object_query_parameter_is_one_parameter_per_property() {
    use connectors_catalog::inventory::ValueType;
    let inventory = extract(&referenced_document());
    let list = inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some("listThings"))
        .unwrap();
    let shapes: Vec<(&str, Option<ValueType>, bool)> = list
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p.value_type, p.required))
        .collect();
    // In property-name order, each its own scalar, none required.
    assert_eq!(
        shapes,
        vec![
            ("start_time", Some(ValueType::Integer), true),
            ("page[after]", Some(ValueType::String), false),
            ("page[size]", Some(ValueType::Integer), false),
        ]
    );
    assert!(
        list.parameters
            .iter()
            .all(|p| p.location == Location::Query && !p.repeated)
    );
}

#[test]
fn a_deep_object_this_pass_cannot_expand_is_named_and_kept_as_one_value() {
    let document = json!({
        "openapi": "3.0.3",
        "paths": {"/things": {
            "parameters": [{"name": "filter", "in": "query", "style": "deepObject",
                            "schema": {"type": "object", "properties": {"x": {"type": "integer"}}}}],
            "get": {
                "operationId": "listThings",
                "parameters": [
                    {"name": "nested", "in": "query", "style": "deepObject",
                     "schema": {"type": "object", "properties": {"inner": {"type": "object"}}}},
                    {"name": "page", "in": "query", "style": "deepObject",
                     "schema": {"type": "object", "properties": {"size": {"type": "integer"}}}},
                    {"name": "page[size]", "in": "query", "schema": {"type": "integer"}}
                ],
                "responses": {}
            },
            "post": {
                "operationId": "createThing",
                "parameters": [{"name": "filter", "in": "query", "schema": {"type": "string"}}],
                "responses": {}
            }
        }}
    });
    let inventory = extract(&document);
    let operation = |id: &str| {
        inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(id))
            .unwrap()
    };
    let names: Vec<&str> = operation("listThings")
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, vec!["filter[x]", "nested", "page", "page[size]"]);
    // The operation's own scalar `filter` replaces the path item's deepObject,
    // so it is neither expanded nor named.
    let names: Vec<&str> = operation("createThing")
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, vec!["filter"]);
    let reasons: Vec<(&str, &str)> = inventory
        .unsupported
        .iter()
        .map(|u| (u.designation.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(reasons.len(), 2, "{reasons:?}");
    assert!(reasons.iter().any(|(d, r)| *d == "listThings"
        && r.contains("`nested`")
        && r.contains("does not expand")));
    assert!(reasons.iter().any(|(d, r)| *d == "listThings"
        && r.contains("`page`")
        && r.contains("named like another declared parameter")));
}
