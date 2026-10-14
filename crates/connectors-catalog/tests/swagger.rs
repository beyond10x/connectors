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

/// The rows a document with shared parameters and responses needs, as the
/// pinned Opsgenie document writes them: a `#/parameters/` reference is the
/// declared parameter, inlined; a `#/responses/` reference is the declared
/// response, inlined; response headers are OpenAPI 3.1 headers; a `csv` or
/// `multi` array query parameter is a `form` array, unexploded or exploded;
/// operation extensions are kept; a Swagger `discriminator` names its
/// property; `nullable: true` beside one type adds `null` to it, and
/// `nullable: false` adds nothing. A document `parameters` or `responses` entry
/// no operation references is not projected, and is listed.
#[test]
fn the_shared_parameter_response_and_header_rows_project_exactly() {
    let document = json!({
        "swagger": "2.0",
        "info": {"title": "Fixture", "version": "1.0.0"},
        "host": "api.example.com",
        "schemes": ["https"],
        "produces": ["application/json"],
        "parameters": {
            "Identifier": {"name": "identifier", "in": "path", "required": true,
                           "type": "string", "description": "The id."},
            "Order": {"name": "order", "in": "query", "required": false, "type": "string",
                      "enum": ["asc", "desc"], "default": "desc"},
            "Unused": {"name": "unused", "in": "query", "type": "string"},
            "MisplacedSchema": {"type": "object", "properties": {"a": {"type": "string"}}}
        },
        "responses": {
            "NotFound": {"description": "Not found", "schema": {"$ref": "#/definitions/Error"},
                         "headers": {"X-Request-ID": {"description": "Request id",
                                                      "type": "string"}}},
            "Unused": {"description": "Never referenced"}
        },
        "paths": {
            "/v1/items/{identifier}": {
                "get": {
                    "operationId": "getItem",
                    "x-collapse-params": "GetItemRequest",
                    "parameters": [
                        {"$ref": "#/parameters/Identifier"},
                        {"$ref": "#/parameters/Order"},
                        {"name": "expand", "in": "query", "type": "array",
                         "items": {"type": "string", "enum": ["a", "b"]},
                         "collectionFormat": "csv"},
                        {"name": "tag", "in": "query", "type": "array",
                         "items": {"type": "string"}, "collectionFormat": "multi"},
                        {"name": "keys", "in": "query", "type": "array",
                         "items": {"type": "integer"}}
                    ],
                    "responses": {
                        "200": {"description": "Item", "schema": {"$ref": "#/definitions/Item"},
                                "headers": {"X-Paging-Next": {"description": "Next page",
                                                              "type": "string"}}},
                        "404": {"$ref": "#/responses/NotFound"}
                    }
                }
            }
        },
        "definitions": {
            "Error": {"type": "object", "properties": {"message": {"type": "string"}}},
            "Item": {
                "type": "object",
                "discriminator": "kind",
                "required": ["kind"],
                "properties": {
                    "kind": {"type": "string"},
                    "enabled": {"type": "boolean", "nullable": true},
                    "count": {"type": "integer", "nullable": false}
                }
            },
            "Special": {"allOf": [{"$ref": "#/definitions/Item"}],
                        "x-discriminator-value": "special"}
        }
    });
    let projection = project(&document).expect("the fixture projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).expect("JSON");
    let error = json!({"$ref": "#/components/schemas/Error"});
    assert_eq!(
        openapi["paths"]["/v1/items/{identifier}"]["get"],
        json!({
            "operationId": "getItem",
            "x-collapse-params": "GetItemRequest",
            "parameters": [
                {"name": "identifier", "in": "path", "required": true,
                 "description": "The id.", "schema": {"type": "string"}},
                {"name": "order", "in": "query", "required": false,
                 "schema": {"type": "string", "enum": ["asc", "desc"], "default": "desc"}},
                {"name": "expand", "in": "query", "style": "form", "explode": false,
                 "schema": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}}},
                {"name": "tag", "in": "query", "style": "form", "explode": true,
                 "schema": {"type": "array", "items": {"type": "string"}}},
                {"name": "keys", "in": "query", "style": "form", "explode": false,
                 "schema": {"type": "array", "items": {"type": "integer"}}}
            ],
            "responses": {
                "200": {
                    "description": "Item",
                    "headers": {"X-Paging-Next": {"description": "Next page",
                                                  "schema": {"type": "string"}}},
                    "content": {"application/json": {"schema":
                        {"$ref": "#/components/schemas/Item"}}}
                },
                "404": {
                    "description": "Not found",
                    "headers": {"X-Request-ID": {"description": "Request id",
                                                 "schema": {"type": "string"}}},
                    "content": {"application/json": {"schema": error}}
                }
            }
        })
    );
    assert_eq!(
        openapi["components"]["schemas"]["Item"],
        json!({
            "type": "object",
            "discriminator": {"propertyName": "kind"},
            "required": ["kind"],
            "properties": {
                "kind": {"type": "string"},
                "enabled": {"type": ["boolean", "null"]},
                "count": {"type": "integer"}
            }
        })
    );
    assert_eq!(
        openapi["components"]["schemas"]["Special"]["x-discriminator-value"],
        "special"
    );
    assert_eq!(
        projection.record.ignored_keys,
        [
            "/parameters/MisplacedSchema",
            "/parameters/Unused",
            "/responses/Unused"
        ]
    );
    // The projection ingests: the shared parameters reach the inventory.
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("fixture.openapi.json");
    std::fs::write(&source, &projection.openapi).expect("write projection");
    let record = connectors_catalog::ingest_file(&source).expect("ingests");
    assert_eq!(record.dialect, Dialect::V31);
}

/// The pinned Opsgenie document (`opsgenie/opsgenie-oas` at
/// `71c55343d03076e5544c484ed32b37c3fbc54037`) projects. It declares no
/// `consumes` on the document, and on one operation only (the attachment
/// upload), so each of its 64 other operations with a `body` or `formData`
/// parameter — every one a write — has a request body without a media type and
/// is listed unprojected. The upload is listed for its `formData` file, and six
/// policy operations for the `schema` written on their `teamId` query
/// parameter; one of those six shares `listAlertPolicies` with
/// `GET /v1/policies`, which projects. The other 99 project, among them the
/// alert and incident reads the catalog ships.
#[test]
fn the_pinned_opsgenie_document_projects_with_its_alert_and_incident_reads() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/opsgenie/upstream/swagger.json"),
    )
    .expect("the pinned Opsgenie document");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "60901e17d0d4ba28b8d13d3d78f6a5d329b93008c3c1710dfe6a0b6c786ef071"
    );
    let projection = swagger::project(&bytes).expect("the pinned Opsgenie document projects");
    let record = &projection.record;
    assert_eq!(record.operation_count, 170);
    assert_eq!(record.operations.len(), 99);
    assert_eq!(record.unprojected_operations.len(), 71);
    let mut others = Vec::new();
    for unprojected in &record.unprojected_operations {
        if unprojected.reason != swagger::NO_MEDIA_TYPE {
            others.push((
                unprojected.pointer.as_str(),
                unprojected.operation_id.as_str(),
                unprojected.reason.as_str(),
            ));
            continue;
        }
        let method = unprojected.pointer.rsplit('/').next().expect("a method");
        assert!(
            ["post", "put", "patch", "delete"].contains(&method),
            "`{}` at `{}` is a write",
            unprojected.operation_id,
            unprojected.pointer
        );
    }
    others.sort_unstable();
    let schema = swagger::SCHEMA_ON_NON_BODY;
    assert_eq!(
        others,
        [
            (
                "/paths/~1v2~1alerts~1{identifier}~1attachments/post",
                "addAttachment",
                swagger::FILE_FORM_PARAMETER
            ),
            (
                "/paths/~1v2~1policies~1alert/get",
                "listAlertPolicies",
                schema
            ),
            (
                "/paths/~1v2~1policies~1notification/get",
                "listNotificationPolicies",
                schema
            ),
            (
                "/paths/~1v2~1policies~1{policyId}/delete",
                "deletePolicy",
                schema
            ),
            ("/paths/~1v2~1policies~1{policyId}/get", "getPolicy", schema),
            (
                "/paths/~1v2~1policies~1{policyId}~1disable/post",
                "disablePolicy",
                schema
            ),
            (
                "/paths/~1v2~1policies~1{policyId}~1enable/post",
                "enablePolicy",
                schema
            ),
        ]
    );
    assert!(record.operations.iter().any(|o| o == "listAlertPolicies"));
    for id in ["listAlerts", "getAlert", "ListIncidents", "getIncident"] {
        assert!(
            record.operations.iter().any(|o| o == id),
            "`{id}` is not projected"
        );
    }
}

/// A property written as a `$ref` with a `description` beside it, the way
/// Opsgenie documents a member whose type is a definition, projects as the
/// reference alone: Swagger 2.0 ignores a sibling of `$ref`, and the
/// description is an annotation that constrains nothing, so dropping it keeps
/// the document's meaning, and the record lists it as ignored. Any other
/// sibling of a property's `$ref`, and a `description` beside a `$ref`
/// anywhere but a property, is still refused.
#[test]
fn a_described_property_reference_projects_as_the_reference_and_is_listed() {
    let mut document = base();
    document["definitions"] = json!({
        "Team": {"type": "string"},
        "Payload": {
            "type": "object",
            "properties": {
                "team": {"$ref": "#/definitions/Team", "description": "the team"},
                "name": {"type": "string", "description": "kept"}
            }
        }
    });
    let projection = project(&document).expect("projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
    assert_eq!(
        openapi["components"]["schemas"]["Payload"]["properties"],
        json!({
            "team": {"$ref": "#/components/schemas/Team"},
            "name": {"type": "string", "description": "kept"}
        })
    );
    assert!(
        projection
            .record
            .ignored_keys
            .contains(&"/definitions/Payload/properties/team/description".to_owned()),
        "{:?}",
        projection.record.ignored_keys
    );

    for (sibling, at) in [
        (
            json!({"$ref": "#/definitions/Team", "maxLength": 3}),
            "/definitions/Payload/properties/team/maxLength",
        ),
        (
            json!({"$ref": "#/definitions/Team", "description": "x", "maxLength": 3}),
            "/definitions/Payload/properties/team/",
        ),
        (
            json!({"$ref": "#/definitions/Team", "description": 3}),
            "/definitions/Payload/properties/team/description",
        ),
    ] {
        let mut document = base();
        document["definitions"] = json!({
            "Team": {"type": "string"},
            "Payload": {"type": "object", "properties": {"team": sibling}}
        });
        let refusal = project(&document).expect_err("refused");
        assert!(
            refusal.pointer.starts_with(at),
            "{} is not under {at}",
            refusal.pointer
        );
    }
    let mut document = base();
    document["definitions"] = json!({
        "Team": {"type": "string"},
        "Payload": {"type": "array", "items": {"$ref": "#/definitions/Team", "description": "x"}}
    });
    assert_eq!(
        project(&document).expect_err("refused").pointer,
        "/definitions/Payload/items/description"
    );
}

/// An operation whose request body has no declared media type — a `body` or
/// `formData` parameter with no `consumes` on the operation and none on the
/// document, or an operation `consumes` that is empty — is not projected: the
/// projection never assumes `application/json`. It is listed in the record
/// with its JSON pointer and the reason, a path whose every operation is
/// unprojected is left out, and a derived bundle counts each one among its
/// unsupported operations. The operations beside it project as before.
#[test]
fn an_operation_whose_request_body_has_no_media_type_is_listed_and_counted_not_projected() {
    let mut document = base();
    document["definitions"] = json!({"Item": {"type": "object"}});
    document["paths"]["/items.create"] = json!({"post": {
        "operationId": "items_create",
        "parameters": [{"name": "body", "in": "body", "required": true,
                        "schema": {"$ref": "#/definitions/Item"}}],
        "responses": {"200": {"description": "ok"}}
    }});
    document["paths"]["/items.list"]["post"] = json!({
        "operationId": "items_rename",
        "parameters": [{"name": "name", "in": "formData", "type": "string"}],
        "responses": {"200": {"description": "ok"}}
    });
    document["paths"]["/items.delete"] = json!({"delete": {
        "consumes": [],
        "parameters": [{"name": "body", "in": "body",
                        "schema": {"$ref": "#/definitions/Item"}}],
        "responses": {"200": {"description": "ok"}}
    }});
    let projection = project(&document).expect("the document projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).expect("JSON");
    assert_eq!(
        openapi["paths"],
        json!({"/items.list": {"get": {
            "operationId": "items_list",
            "parameters": [{"name": "cursor", "in": "query", "schema": {"type": "string"}}],
            "responses": {"200": {"description": "ok"}}
        }}}),
        "nothing is projected for an operation without a request media type"
    );
    assert!(
        !String::from_utf8_lossy(&projection.openapi).contains("application/json"),
        "no media type is assumed"
    );
    let record = &projection.record;
    assert_eq!(record.operation_count, 4);
    assert_eq!(record.operations, ["items_list"]);
    let unprojected = |id: &str, pointer: &str| swagger::UnprojectedOperation {
        operation_id: id.to_owned(),
        pointer: pointer.to_owned(),
        reason: swagger::NO_MEDIA_TYPE.to_owned(),
    };
    assert_eq!(
        record.unprojected_operations,
        [
            unprojected("DELETE /items.delete", "/paths/~1items.delete/delete"),
            unprojected("items_create", "/paths/~1items.create/post"),
            unprojected("items_rename", "/paths/~1items.list/post"),
        ]
    );
    assert_eq!(swagger::NO_MEDIA_TYPE, "request body without a media type");

    // Derived into a bundle, each unprojected operation is an unsupported one.
    let temp = tempfile::tempdir().expect("temporary directory");
    let from = temp.path().join("fixture.swagger.json");
    std::fs::write(&from, document.to_string()).expect("write swagger");
    let source = temp.path().join("fixture.openapi.json");
    std::fs::write(&source, &projection.openapi).expect("write projection");
    let bundles = temp.path().join("bundles");
    let request = pipeline::Request {
        provider: "fixture",
        source: &source,
        directory: &bundles,
        auth_profile: "fixture.token",
        replace: false,
        amendments: None,
    };
    let run = pipeline::run_derived(&request, &from).expect("the projection ingests");
    assert_eq!(run.coverage.unsupported, 3);
    let bundle = bundle::load(&bundles, "fixture").expect("the written bundle");
    assert_eq!(bundle.inventory.operations.len(), 1);
    let mut gaps: Vec<(&str, &str)> = bundle
        .inventory
        .unsupported
        .iter()
        .map(|gap| (gap.designation.as_str(), gap.reason.as_str()))
        .collect();
    gaps.sort_unstable();
    assert_eq!(
        gaps,
        [
            ("DELETE /items.delete", swagger::NO_MEDIA_TYPE),
            ("items_create", swagger::NO_MEDIA_TYPE),
            ("items_rename", swagger::NO_MEDIA_TYPE),
        ]
    );

    // A declared media type that is not a form type beside formData is still
    // refused: that body has a media type, and no exact projection under it.
    let mut declared = base();
    declared["consumes"] = json!(["application/json"]);
    declared["paths"]["/items.list"]["get"]["parameters"][0]["in"] = json!("formData");
    assert_eq!(
        project(&declared).expect_err("refused").pointer,
        "/paths/~1items.list/get/consumes"
    );
}

/// A `formData` parameter of `type: file` — Swagger 2.0's upload, whose
/// OpenAPI 3 spelling is a binary encoding the projection has no exact row for —
/// leaves its operation unprojected: listed with its pointer and the reason,
/// counted unsupported in a derived bundle, its path left out when nothing
/// else on it projects. The other operations project as before, and a `file`
/// parameter anywhere but `formData` is still refused (see
/// `every_construct_without_an_exact_row_is_refused_by_name`).
#[test]
fn an_operation_with_a_file_form_parameter_is_listed_and_counted_not_projected() {
    let mut document = base();
    document["paths"]["/items.upload"] = json!({"post": {
        "operationId": "items_upload",
        "consumes": ["multipart/form-data"],
        "parameters": [
            {"name": "file", "in": "formData", "required": true, "type": "file"},
            {"name": "note", "in": "formData", "type": "string"}
        ],
        "responses": {"200": {"description": "ok"}}
    }});
    let projection = project(&document).expect("the document projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).expect("JSON");
    assert_eq!(
        openapi["paths"]
            .as_object()
            .expect("paths")
            .keys()
            .collect::<Vec<_>>(),
        ["/items.list"]
    );
    assert!(
        !String::from_utf8_lossy(&projection.openapi).contains("multipart/form-data"),
        "no request body is projected for the upload"
    );
    let record = &projection.record;
    assert_eq!(record.operation_count, 2);
    assert_eq!(record.operations, ["items_list"]);
    assert_eq!(
        record.unprojected_operations,
        [swagger::UnprojectedOperation {
            operation_id: "items_upload".to_owned(),
            pointer: "/paths/~1items.upload/post".to_owned(),
            reason: swagger::FILE_FORM_PARAMETER.to_owned(),
        }]
    );
    assert_eq!(
        swagger::FILE_FORM_PARAMETER,
        "formData parameter of type file"
    );

    let temp = tempfile::tempdir().expect("temporary directory");
    let from = temp.path().join("fixture.swagger.json");
    std::fs::write(&from, document.to_string()).expect("write swagger");
    let source = temp.path().join("fixture.openapi.json");
    std::fs::write(&source, &projection.openapi).expect("write projection");
    let bundles = temp.path().join("bundles");
    let request = pipeline::Request {
        provider: "fixture",
        source: &source,
        directory: &bundles,
        auth_profile: "fixture.token",
        replace: false,
        amendments: None,
    };
    let run = pipeline::run_derived(&request, &from).expect("the projection ingests");
    assert_eq!(run.coverage.unsupported, 1);

    // A refusal elsewhere in the same operation still stands.
    let mut broken = document.clone();
    broken["paths"]["/items.upload"]["post"]["parameters"][1]["type"] = json!("date");
    assert_eq!(
        project(&broken).expect_err("refused").pointer,
        "/paths/~1items.upload/post/parameters/1/type"
    );
}

/// A `query`, `header`, `path` or `formData` parameter written with a
/// `schema` (which Swagger 2.0 defines for `body` only, and which then leaves
/// the parameter without the `type` its location requires) has no exact
/// projection; its operation is listed with its pointer and the reason, counted
/// unsupported, and the operations beside it project. Any other unknown key on
/// such a parameter is still refused by name.
#[test]
fn an_operation_with_a_schema_on_a_non_body_parameter_is_listed_not_projected() {
    let mut document = base();
    document["paths"]["/items.list"]["delete"] = json!({
        "operationId": "items_clear",
        "parameters": [
            {"name": "team", "in": "query", "required": false,
             "description": "a team", "schema": {"$ref": "#/parameters/Team"}}
        ],
        "responses": {"200": {"description": "ok"}}
    });
    let projection = project(&document).expect("the document projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).expect("JSON");
    assert_eq!(
        openapi["paths"]["/items.list"]
            .as_object()
            .expect("path")
            .keys()
            .collect::<Vec<_>>(),
        ["get"]
    );
    let record = &projection.record;
    assert_eq!(record.operations, ["items_list"]);
    assert_eq!(
        record.unprojected_operations,
        [swagger::UnprojectedOperation {
            operation_id: "items_clear".to_owned(),
            pointer: "/paths/~1items.list/delete".to_owned(),
            reason: swagger::SCHEMA_ON_NON_BODY.to_owned(),
        }]
    );
    assert_eq!(
        swagger::SCHEMA_ON_NON_BODY,
        "non-body parameter written with a schema"
    );

    let mut extra = document.clone();
    extra["paths"]["/items.list"]["delete"]["parameters"][0]["minimum"] = json!(1);
    extra["paths"]["/items.list"]["delete"]["parameters"][0]["collection"] = json!(1);
    assert_eq!(
        project(&extra).expect_err("refused").pointer,
        "/paths/~1items.list/delete/parameters/0/collection"
    );
}

/// `operationId` is unique among the operations the projection writes. An
/// operation listed unprojected is not in the output, so its id collides with
/// nothing there: the Opsgenie document names `listAlertPolicies` twice, once
/// on an operation that projects and once on one listed for its own reason.
/// The record keeps both designations apart by pointer. Two projected
/// operations with one id are still refused
/// (`every_construct_without_an_exact_row_is_refused_by_name`).
#[test]
fn an_operation_id_shared_with_an_unprojected_operation_does_not_refuse() {
    let mut document = base();
    document["paths"]["/items.other"] = json!({"get": {
        "operationId": "items_list",
        "parameters": [{"name": "team", "in": "query", "schema": {"type": "string"}}],
        "responses": {"200": {"description": "ok"}}
    }});
    let projection = project(&document).expect("the document projects");
    let record = &projection.record;
    assert_eq!(record.operations, ["items_list"]);
    assert_eq!(
        record.unprojected_operations,
        [swagger::UnprojectedOperation {
            operation_id: "items_list".to_owned(),
            pointer: "/paths/~1items.other/get".to_owned(),
            reason: swagger::SCHEMA_ON_NON_BODY.to_owned(),
        }]
    );
    // Listed first in the document, the unprojected one does not claim the id.
    let mut first = base();
    let listed = document["paths"]["/items.other"].clone();
    let projected = first["paths"]["/items.list"].clone();
    first["paths"] = json!({"/items.a": listed, "/items.b": projected});
    assert_eq!(
        project(&first).expect("projects").record.operations,
        ["items_list"]
    );
}

/// The rows for the document's shared parameters and responses, response
/// headers, array query parameters, operation extensions, `discriminator` and
/// `nullable` changed the table, and a change to any row is a new projector
/// version. Every committed Swagger 2.0 projection record names the current
/// one, so none was left recording the version before the change.
#[test]
fn the_projector_version_moved_with_its_rows_and_every_record_names_it() {
    assert_eq!(swagger::PROJECTOR, "swagger2-openapi/2");
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let mut swagger_records = Vec::new();
    for adapter in std::fs::read_dir(&adapters).expect("adapters") {
        let generated = adapter.expect("entry").path().join("generated");
        let Ok(entries) = std::fs::read_dir(&generated) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            if !path.to_string_lossy().ends_with(".projection.json") {
                continue;
            }
            let record: Value =
                serde_json::from_slice(&std::fs::read(&path).expect("record")).expect("json");
            let projector = record["projector"].as_str().expect("a projector");
            if projector.starts_with("swagger2-openapi/") {
                assert_eq!(projector, swagger::PROJECTOR, "{}", path.display());
                swagger_records.push(path);
            }
        }
    }
    swagger_records.sort();
    let names: Vec<_> = swagger_records
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "opsgenie.openapi.projection.json",
            "slack-web.openapi.projection.json"
        ]
    );
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
            |d| d["webhooks"] = json!({}),
            "/webhooks",
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
            "a parameter reference the document does not declare",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"$ref": "#/parameters/cursor"})
            },
            "/paths/~1items.list/get/parameters/0/$ref",
            "cursor",
        ),
        (
            "a parameter reference outside `#/parameters/`",
            |d| {
                d["parameters"] =
                    json!({"cursor": {"name": "cursor", "in": "query", "type": "string"}});
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"$ref": "#/definitions/cursor"})
            },
            "/paths/~1items.list/get/parameters/0/$ref",
            "#/parameters/",
        ),
        (
            "a sibling of a parameter reference",
            |d| {
                d["parameters"] =
                    json!({"cursor": {"name": "cursor", "in": "query", "type": "string"}});
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"$ref": "#/parameters/cursor", "required": true})
            },
            "/paths/~1items.list/get/parameters/0/required",
            "$ref",
        ),
        (
            "a referenced document parameter that is not a parameter object",
            |d| {
                d["parameters"] = json!({"cursor": {"type": "object", "properties": {}}});
                d["paths"]["/items.list"]["get"]["parameters"][0] =
                    json!({"$ref": "#/parameters/cursor"})
            },
            "/parameters/cursor/properties",
            "rule table",
        ),
        (
            "an array parameter sent pipe-delimited",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0]["type"] = json!("array");
                d["paths"]["/items.list"]["get"]["parameters"][0]["items"] =
                    json!({"type": "string"});
                d["paths"]["/items.list"]["get"]["parameters"][0]["collectionFormat"] =
                    json!("pipes");
            },
            "/paths/~1items.list/get/parameters/0/collectionFormat",
            "pipes",
        ),
        (
            "an array parameter without items",
            |d| d["paths"]["/items.list"]["get"]["parameters"][0]["type"] = json!("array"),
            "/paths/~1items.list/get/parameters/0/items",
            "required",
        ),
        (
            "an array path parameter",
            |d| {
                d["paths"]["/items.list/{id}"] = json!({"get": {
                    "parameters": [{"name": "id", "in": "path", "required": true,
                                    "type": "array", "items": {"type": "string"}}],
                    "responses": {"200": {"description": "ok"}}}});
            },
            "/paths/~1items.list~1{id}/get/parameters/0/type",
            "array",
        ),
        (
            "an array of arrays",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0]["type"] = json!("array");
                d["paths"]["/items.list"]["get"]["parameters"][0]["items"] =
                    json!({"type": "array"});
            },
            "/paths/~1items.list/get/parameters/0/items/type",
            "array",
        ),
        (
            "items on a scalar parameter",
            |d| {
                d["paths"]["/items.list"]["get"]["parameters"][0]["items"] =
                    json!({"type": "string"})
            },
            "/paths/~1items.list/get/parameters/0/items",
            "array",
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
            "a response header key outside the table",
            |d| {
                d["paths"]["/items.list"]["get"]["responses"]["200"]["headers"] =
                    json!({"X-Next": {"type": "string", "collectionFormat": "csv"}})
            },
            "/paths/~1items.list/get/responses/200/headers/X-Next/collectionFormat",
            "rule table",
        ),
        (
            "a response header without a type",
            |d| {
                d["paths"]["/items.list"]["get"]["responses"]["200"]["headers"] =
                    json!({"X-Next": {"description": "next"}})
            },
            "/paths/~1items.list/get/responses/200/headers/X-Next/type",
            "required",
        ),
        (
            "a Content-Type response header",
            |d| {
                d["paths"]["/items.list"]["get"]["responses"]["200"]["headers"] =
                    json!({"Content-Type": {"type": "string"}})
            },
            "/paths/~1items.list/get/responses/200/headers/Content-Type",
            "Content-Type",
        ),
        (
            "a response reference the document does not declare",
            |d| {
                d["paths"]["/items.list"]["get"]["responses"]["200"] =
                    json!({"$ref": "#/responses/Ok"})
            },
            "/paths/~1items.list/get/responses/200/$ref",
            "Ok",
        ),
        (
            "a sibling of a response reference",
            |d| {
                d["responses"] = json!({"Ok": {"description": "ok"}});
                d["paths"]["/items.list"]["get"]["responses"]["200"] =
                    json!({"$ref": "#/responses/Ok", "description": "x"})
            },
            "/paths/~1items.list/get/responses/200/description",
            "$ref",
        ),
        (
            "an operation key outside the table that is not an extension",
            |d| d["paths"]["/items.list"]["get"]["callbacks"] = json!({}),
            "/paths/~1items.list/get/callbacks",
            "rule table",
        ),
        (
            "a security scheme the document does not declare",
            |d| d["paths"]["/items.list"]["get"]["security"] = json!([{"oauth": []}]),
            "/paths/~1items.list/get/security/0/oauth",
            "oauth",
        ),
        (
            "a schema discriminator that is not a property name",
            |d| {
                d["definitions"] =
                    json!({"a": {"type": "object", "discriminator": {"propertyName": "kind"}}})
            },
            "/definitions/a/discriminator",
            "string",
        ),
        (
            "nullable without one type",
            |d| d["definitions"] = json!({"a": {"nullable": true}}),
            "/definitions/a/nullable",
            "type",
        ),
        (
            "nullable that is not a boolean",
            |d| d["definitions"] = json!({"a": {"type": "string", "nullable": "yes"}}),
            "/definitions/a/nullable",
            "boolean",
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
