//! Swagger 2.0 projected into OpenAPI 3.1 and ingested through
//! `pipeline::run_derived`: the pinned Slack Web API document end to end, the
//! projection's rows on a small document, and a refusal by name for every
//! construct the projection has no exact row for.

use connectors_catalog::{Derivation, Dialect, bundle, derived, pipeline, swagger};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const SLACK: &str = "slack_web_openapi_v2_without_examples.json";
/// `slackapi/slack-api-specs` at `bc08db49625630e3585bf2f1322128ea04f2a7f3`.
const SLACK_SHA256: &str = "8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a";
const SLACK_BYTES: usize = 1_039_581;

fn slack_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/slack/upstream")
        .join(SLACK)
}

fn project(document: &Value) -> Result<swagger::Projection, swagger::Refusal> {
    swagger::project(document.to_string().as_bytes())
}

/// The smallest document the projection accepts, with one GET operation.
fn base() -> Value {
    json!({
        "swagger": "2.0",
        "info": {"title": "Fixture", "version": "1.0.0"},
        "host": "api.example.com",
        "basePath": "/api",
        "schemes": ["https"],
        "paths": {
            "/items.list": {
                "get": {
                    "operationId": "items_list",
                    "parameters": [
                        {"name": "cursor", "in": "query", "type": "string"}
                    ],
                    "responses": {"200": {"description": "ok"}}
                }
            }
        }
    })
}

#[test]
fn the_pinned_slack_document_projects_and_ingests_with_its_conversation_reads() {
    let bytes = std::fs::read(slack_path()).expect("the pinned Slack document");
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SLACK_SHA256);
    assert_eq!(bytes.len(), SLACK_BYTES);

    let projection = swagger::project(&bytes).expect("the pinned Slack document projects");
    assert_eq!(projection.record.projector, swagger::PROJECTOR);
    assert_eq!(projection.record.source_sha256, SLACK_SHA256);
    assert_eq!(projection.record.operation_count, 174);
    assert_eq!(projection.record.operations.len(), 174);
    assert_eq!(
        swagger::project(&bytes).expect("again"),
        projection,
        "the same bytes project to the same document and record"
    );

    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("slack.openapi.json");
    std::fs::write(&source, &projection.openapi).expect("write projection");
    let bundles = temp.path().join("bundles");
    let request = pipeline::Request {
        provider: "slack",
        source: &source,
        directory: &bundles,
        auth_profile: "slack.oauth",
        replace: false,
        amendments: None,
    };
    let run = pipeline::run_derived(&request, &slack_path()).expect("projected Slack ingests");
    assert_eq!(run.source.dialect, Dialect::V31);
    assert_eq!(run.source.openapi, swagger::OPENAPI);
    assert_eq!(
        run.source.derivation,
        Some(Derivation {
            from_file: SLACK.to_owned(),
            from_sha256: SLACK_SHA256.to_owned(),
            from_bytes: SLACK_BYTES,
            format: swagger::FORMAT.to_owned(),
            discovery_revision: None,
            projector: swagger::PROJECTOR.to_owned(),
        })
    );

    let bundle = bundle::load(&bundles, "slack").expect("the written bundle");
    assert_eq!(bundle.inventory.operations.len(), 174);
    let expected: [(&str, &[&str]); 3] = [
        (
            "/api/conversations.list",
            &["token", "exclude_archived", "types", "limit", "cursor"],
        ),
        (
            "/api/conversations.history",
            &[
                "token",
                "channel",
                "latest",
                "oldest",
                "inclusive",
                "limit",
                "cursor",
            ],
        ),
        (
            "/api/conversations.replies",
            &[
                "token",
                "channel",
                "ts",
                "latest",
                "oldest",
                "inclusive",
                "limit",
                "cursor",
            ],
        ),
    ];
    for (path, parameters) in expected {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|operation| operation.path == path && operation.method == "get")
            .unwrap_or_else(|| panic!("`GET {path}` is in the inventory"));
        let id = path.trim_start_matches("/api/").replace('.', "_");
        assert_eq!(operation.operation_id.as_deref(), Some(id.as_str()));
        let mut names: Vec<&str> = operation
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        let mut wanted = parameters.to_vec();
        names.sort_unstable();
        wanted.sort_unstable();
        assert_eq!(names, wanted, "`{path}` parameters");
        assert!(
            operation
                .parameters
                .iter()
                .all(|parameter| parameter.location
                    == connectors_catalog::inventory::Location::Query),
            "`{path}` parameters travel in the query"
        );
        let typed = |name: &str| {
            operation
                .parameters
                .iter()
                .find(|parameter| parameter.name == name)
                .and_then(|parameter| parameter.value_type)
        };
        use connectors_catalog::inventory::ValueType;
        assert_eq!(typed("limit"), Some(ValueType::Integer), "`{path}` limit");
        assert_eq!(typed("cursor"), Some(ValueType::String), "`{path}` cursor");
    }
    assert_eq!(
        run.coverage.unsupported, 0,
        "{:?}",
        bundle.inventory.unsupported
    );
}

#[test]
fn the_rows_project_a_small_document_exactly() {
    let document = json!({
        "swagger": "2.0",
        "info": {
            "title": "Fixture",
            "version": "2.1.0",
            "description": "A fixture.",
            "contact": {"name": "Team", "url": "https://example.com/support"},
            "license": {"name": "MIT"}
        },
        "host": "api.example.com",
        "basePath": "/api",
        "schemes": ["https"],
        "externalDocs": {"description": "Docs", "url": "https://example.com/docs"},
        "tags": [],
        "securityDefinitions": {
            "oauth": {
                "type": "oauth2",
                "flow": "accessCode",
                "authorizationUrl": "https://example.com/oauth/authorize",
                "scopes": {"items:read": "items:read", "items:write": "items:write"}
            }
        },
        "paths": {
            "/items.get": {
                "get": {
                    "operationId": "items_get",
                    "description": "Fetch one item.",
                    "tags": ["items"],
                    "externalDocs": {"url": "https://example.com/items.get"},
                    "consumes": ["application/x-www-form-urlencoded"],
                    "produces": ["application/json"],
                    "security": [{"oauth": ["items:read"]}],
                    "parameters": [
                        {"name": "token", "in": "header", "type": "string", "required": true,
                         "description": "Auth token."},
                        {"name": "limit", "in": "query", "type": "integer"},
                        {"name": "exact", "in": "query", "type": "boolean", "required": false}
                    ],
                    "responses": {
                        "200": {
                            "description": "Item",
                            "schema": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["ok"],
                                "properties": {
                                    "ok": {"$ref": "#/definitions/ok_true"},
                                    "pair": {"items": [{"type": "integer"}, {"type": "null"}]},
                                    "name": {"type": ["string", "null"], "x-examples": ["a"]}
                                }
                            }
                        },
                        "default": {"description": "Error"}
                    }
                }
            },
            "/items.set/{id}": {
                "post": {
                    "operationId": "items_set",
                    "consumes": ["application/x-www-form-urlencoded", "application/json"],
                    "produces": ["application/json"],
                    "parameters": [
                        {"name": "id", "in": "path", "type": "string", "required": true},
                        {"name": "name", "in": "formData", "type": "string", "required": true,
                         "description": "New name."},
                        {"name": "rank", "in": "formData", "type": "number"}
                    ],
                    "responses": {
                        "200": {"description": "Done", "schema": {"$ref": "#/definitions/ok_true"}}
                    }
                }
            }
        },
        "definitions": {
            "ok_true": {"type": "boolean", "enum": [true], "title": "ok"}
        }
    });
    let projection = project(&document).expect("the fixture projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).expect("JSON");
    assert_eq!(
        openapi,
        json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Fixture",
                "version": "2.1.0",
                "description": "A fixture.",
                "contact": {"name": "Team", "url": "https://example.com/support"},
                "license": {"name": "MIT"}
            },
            "servers": [{"url": "https://api.example.com/api"}],
            "externalDocs": {"description": "Docs", "url": "https://example.com/docs"},
            "tags": [],
            "paths": {
                "/items.get": {
                    "get": {
                        "operationId": "items_get",
                        "description": "Fetch one item.",
                        "tags": ["items"],
                        "externalDocs": {"url": "https://example.com/items.get"},
                        "x-swagger-security": [{"oauth": ["items:read"]}],
                        "parameters": [
                            {"name": "token", "in": "header", "required": true,
                             "description": "Auth token.", "schema": {"type": "string"}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer"}},
                            {"name": "exact", "in": "query", "required": false,
                             "schema": {"type": "boolean"}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Item",
                                "content": {"application/json": {"schema": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "required": ["ok"],
                                    "properties": {
                                        "ok": {"$ref": "#/components/schemas/ok_true"},
                                        "pair": {"prefixItems": [{"type": "integer"}, {"type": "null"}]},
                                        "name": {"type": ["string", "null"], "x-examples": ["a"]}
                                    }
                                }}}
                            },
                            "default": {"description": "Error"}
                        }
                    }
                },
                "/items.set/{id}": {
                    "post": {
                        "operationId": "items_set",
                        "parameters": [
                            {"name": "id", "in": "path", "required": true,
                             "schema": {"type": "string"}}
                        ],
                        "requestBody": {
                            "required": true,
                            "content": {"application/x-www-form-urlencoded": {"schema": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string", "description": "New name."},
                                    "rank": {"type": "number"}
                                },
                                "required": ["name"]
                            }}}
                        },
                        "responses": {
                            "200": {
                                "description": "Done",
                                "content": {"application/json": {"schema": {
                                    "$ref": "#/components/schemas/ok_true"
                                }}}
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {"ok_true": {"type": "boolean", "enum": [true], "title": "ok"}}
            }
        })
    );
    let record = &projection.record;
    assert_eq!(record.info_version, "2.1.0");
    assert_eq!(record.operation_count, 2);
    assert_eq!(record.operations, ["items_get", "items_set"]);
    assert_eq!(
        record.excluded_media_types,
        [swagger::ExcludedMediaType {
            operation_id: "items_set".to_owned(),
            media_type: "application/json".to_owned(),
            reason: swagger::FORM_ONLY.to_owned(),
        }]
    );
    assert_eq!(
        record.ignored_keys,
        ["/paths/~1items.get/get/consumes", "/securityDefinitions"]
    );
    // The record and the document are both canonical: sorted keys, one final newline.
    assert!(projection.openapi.ends_with(b"}\n"));
    assert!(projection.record_bytes().ends_with(b"}\n"));
}

#[test]
fn every_construct_without_an_exact_row_is_refused_by_name() {
    type Edit = fn(&mut Value);
    let cases: Vec<(&str, Edit, &str, &str)> = vec![
        (
            "a version other than 2.0",
            |d| d["swagger"] = json!("3.0"),
            "/swagger",
            "2.0",
        ),
        (
            "no host",
            |d| {
                d.as_object_mut().unwrap().remove("host");
            },
            "/host",
            "required",
        ),
        (
            "no schemes",
            |d| {
                d.as_object_mut().unwrap().remove("schemes");
            },
            "/schemes",
            "required",
        ),
        (
            "a websocket scheme",
            |d| d["schemes"] = json!(["wss"]),
            "/schemes/0",
            "wss",
        ),
        (
            "a basePath with a final slash",
            |d| d["basePath"] = json!("/api/"),
            "/basePath",
            "/",
        ),
        (
            "an unknown document key",
            |d| d["parameters"] = json!({}),
            "/parameters",
            "rule table",
        ),
        (
            "path-level parameters",
            |d| d["paths"]["/items.list"]["parameters"] = json!([]),
            "/paths/~1items.list/parameters",
            "rule table",
        ),
        (
            "an operation-level schemes",
            |d| d["paths"]["/items.list"]["get"]["schemes"] = json!(["https"]),
            "/paths/~1items.list/get/schemes",
            "rule table",
        ),
        (
            "a parameter reference",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"$ref": "#/parameters/cursor"})
            },
            "/paths/~1items.list/get/parameters/0/$ref",
            "rule table",
        ),
        (
            "an array parameter",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0]["type"] = json!("array");
                d["paths"]["/items.list"]["get"]["parameters"][0]["items"] =
                    json!({"type": "string"});
            },
            "/paths/~1items.list/get/parameters/0/items",
            "rule table",
        ),
        (
            "a file parameter",
            |d| d["paths"]["/items.list"]["get"]["parameters"][0]["type"] = json!("file"),
            "/paths/~1items.list/get/parameters/0/type",
            "file",
        ),
        (
            "an Authorization header parameter",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"name": "Authorization", "in": "header", "type": "string"})
            },
            "/paths/~1items.list/get/parameters/0/name",
            "Authorization",
        ),
        (
            "a path parameter the template does not name",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"name": "id", "in": "path", "type": "string", "required": true})
            },
            "/paths/~1items.list/get/parameters/0/name",
            "id",
        ),
        (
            "formData without a form media type",
            |d| {
                d["paths"]["/items.list"]["get"]["consumes"] = json!(["application/json"]);
                d["paths"]["/items.list"]["get"]["parameters"][0]["in"] = json!("formData");
            },
            "/paths/~1items.list/get/consumes",
            "formData",
        ),
        (
            "a response schema with no produces",
            |d| {
                d["paths"]["/items.list"]["get"]["responses"]["200"]["schema"] =
                    json!({"type": "object"})
            },
            "/paths/~1items.list/get/produces",
            "required",
        ),
        (
            "response headers",
            |d| d["paths"]["/items.list"]["get"]["responses"]["200"]["headers"] = json!({}),
            "/paths/~1items.list/get/responses/200/headers",
            "rule table",
        ),
        (
            "a security scheme the document does not declare",
            |d| d["paths"]["/items.list"]["get"]["security"] = json!([{"oauth": []}]),
            "/paths/~1items.list/get/security/0/oauth",
            "oauth",
        ),
        (
            "a schema discriminator",
            |d| d["definitions"] = json!({"a": {"type": "object", "discriminator": "kind"}}),
            "/definitions/a/discriminator",
            "rule table",
        ),
        (
            "a boolean exclusiveMaximum",
            |d| {
                d["definitions"] =
                    json!({"a": {"type": "integer", "maximum": 3, "exclusiveMaximum": true}})
            },
            "/definitions/a/exclusiveMaximum",
            "rule table",
        ),
        (
            "a sibling of $ref",
            |d| {
                d["definitions"] = json!({
                    "a": {"type": "string"},
                    "b": {"$ref": "#/definitions/a", "description": "x"}
                })
            },
            "/definitions/b/description",
            "$ref",
        ),
        (
            "an unresolved $ref",
            |d| d["definitions"] = json!({"b": {"$ref": "#/definitions/missing"}}),
            "/definitions/b/$ref",
            "missing",
        ),
        (
            "a duplicate operationId",
            |d| {
                d["paths"]["/items.other"] = d["paths"]["/items.list"].clone();
            },
            "/paths/~1items.other/get/operationId",
            "items_list",
        ),
    ];
    for (case, edit, pointer, named) in cases {
        let mut document = base();
        edit(&mut document);
        let refusal = project(&document).expect_err(case);
        assert_eq!(refusal.pointer, pointer, "{case}: {refusal}");
        let text = refusal.to_string();
        assert!(text.contains(pointer), "{case}: `{text}` names its pointer");
        assert!(text.contains(named), "{case}: `{text}` names `{named}`");
    }
    assert!(project(&base()).is_ok(), "the base itself is accepted");
}

#[test]
fn a_source_that_is_not_the_projection_is_refused_before_the_directory_is_touched() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let from = temp.path().join("fixture.swagger.json");
    std::fs::write(&from, base().to_string()).expect("write swagger");
    let mut projected: Value =
        serde_json::from_slice(&project(&base()).unwrap().openapi).expect("JSON");
    projected["info"]["title"] = json!("Edited");
    let source = temp.path().join("fixture.openapi.json");
    std::fs::write(&source, projected.to_string()).expect("write source");
    let bundles = temp.path().join("bundles");
    let request = pipeline::Request {
        provider: "fixture",
        source: &source,
        directory: &bundles,
        auth_profile: "fixture.token",
        replace: false,
        amendments: None,
    };
    let failure = pipeline::run_derived(&request, &from).expect_err("must refuse");
    assert_eq!(failure.step, pipeline::Step::Ingest);
    assert!(failure.to_string().contains("fixture.swagger.json"));
    assert!(!bundles.exists());

    // And one that is the projection, byte for byte, is accepted.
    std::fs::write(&source, project(&base()).unwrap().openapi).expect("write source");
    let run = pipeline::run_derived(&request, &from).expect("the projection ingests");
    let derivation = run.source.derivation.expect("derivation");
    assert_eq!(derivation.format, swagger::FORMAT);
    assert_eq!(derivation.discovery_revision, None);
}

#[test]
fn a_swagger_refusal_reaches_the_pipeline_named() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let mut document = base();
    document["paths"]["/items.list"]["get"]["schemes"] = json!(["https"]);
    let from = temp.path().join("bad.swagger.json");
    std::fs::write(&from, document.to_string()).expect("write swagger");
    let source = temp.path().join("bad.openapi.json");
    std::fs::write(&source, b"{}").expect("write source");
    let request = pipeline::Request {
        provider: "fixture",
        source: &source,
        directory: temp.path(),
        auth_profile: "fixture.token",
        replace: false,
        amendments: None,
    };
    let failure = pipeline::run_derived(&request, &from).expect_err("must refuse");
    assert_eq!(failure.step, pipeline::Step::Ingest);
    let text = failure.to_string();
    assert!(
        text.contains("swagger document refused at `/paths/~1items.list/get/schemes`"),
        "{text}"
    );
}

#[test]
fn the_derived_dispatch_reads_swagger_and_discovery_alike() {
    let swagger = derived::project(base().to_string().as_bytes()).expect("swagger projects");
    assert_eq!(swagger.format(), swagger::FORMAT);
    assert_eq!(
        swagger.openapi(),
        project(&base()).unwrap().openapi.as_slice()
    );
    let refusal = derived::project(br#"{"kind": "nope"}"#).expect_err("not discovery");
    assert!(
        refusal
            .to_string()
            .starts_with("discovery document refused")
    );
}
