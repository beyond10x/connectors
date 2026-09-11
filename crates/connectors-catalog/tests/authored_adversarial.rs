//! Adversarial cases against the authored reader. Each asserts something the
//! story's Outcome or Acceptance claims about the authored path, not a behaviour
//! this file would prefer: the authored path is "not a reduced second engine",
//! the file "is expanded into an `inventory::Operation` and then into a
//! `template::Template`", and it "binds exactly as the equivalent OpenAPI
//! operation would".

use connectors_catalog::authored::{Document, read, read_file};
use connectors_catalog::inventory::extract;
use connectors_catalog::template::Template;
use connectors_core::ErrorCode;
use std::collections::BTreeMap;

/// A document that declares one parameter twice under one name and location.
/// Refusing it is a correct outcome, and so is accepting it and honouring what it
/// declares. Accepting it and dropping the `required = true` it wrote is neither.
const REPEATED_PARAMETER: &str = r#"
provider = "acme"

[[action]]
name = "listIssues"
method = "get"
path = "/issues"

[[action.parameter]]
name = "page"
location = "query"
required = true

[[action.parameter]]
name = "page"
location = "query"
"#;

/// The document, when the reader accepts it at all. A refusal is an acceptable
/// outcome for the cases below; what they deny is acceptance that then ignores
/// what the document said.
fn representable(document: &str) -> Option<Document> {
    read(document.as_bytes()).ok()
}

#[test]
fn a_repeated_parameter_does_not_lose_the_required_it_declares() {
    let Some(document) = representable(REPEATED_PARAMETER) else {
        return;
    };
    let action = &document.actions[0];
    assert!(
        action
            .operation
            .parameters
            .iter()
            .any(|parameter| parameter.name == "page" && parameter.required),
        "the published operation no longer declares `page` required: {:?}",
        action.operation.parameters
    );
    let bound = action.template.bind(&BTreeMap::new(), None);
    assert!(
        bound.is_err(),
        "the operation declares `page` required and the template built from that \
         operation bound a request without it: {bound:?}"
    );
}

#[test]
fn an_authored_operation_declares_each_parameter_once() {
    let Some(document) = representable(REPEATED_PARAMETER) else {
        return;
    };
    let parameters = &document.actions[0].operation.parameters;
    for (index, parameter) in parameters.iter().enumerate() {
        let twin = parameters
            .iter()
            .skip(index + 1)
            .any(|other| other.name == parameter.name && other.location == parameter.location);
        assert!(
            !twin,
            "`{}` is declared twice in one operation, which the inventory never \
             emits and the template silently collapses: {parameters:?}",
            parameter.name
        );
    }
}

#[test]
fn the_default_request_media_type_is_the_one_the_equivalent_import_picks() {
    let document = read(
        br#"
provider = "acme"

[[action]]
name = "createIssue"
method = "post"
path = "/issues"
request_media_types = ["application/xml", "application/json"]
"#,
    )
    .expect("the fixture document is representable");
    let authored = document.actions[0]
        .template
        .bind(&BTreeMap::new(), None)
        .expect("the action declares no parameter");

    let imported = extract(&serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "Acme", "version": "1"},
        "paths": {
            "/issues": {
                "post": {
                    "operationId": "createIssue",
                    "requestBody": {
                        "content": {"application/xml": {}, "application/json": {}}
                    }
                }
            }
        }
    }));
    assert_eq!(imported.coverage(), (1, 0));
    let from_openapi = Template::from_operation(&imported.operations[0])
        .expect("the imported operation is representable")
        .bind(&BTreeMap::new(), None)
        .expect("the operation declares no parameter");

    assert_eq!(
        authored, from_openapi,
        "the authored action does not bind as the equivalent imported operation does"
    );
}

#[test]
fn a_document_that_exists_but_cannot_be_read_is_not_reported_as_absent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("acme.actions.toml");
    std::fs::create_dir(&path).expect("create the path the refusal is about");
    assert!(path.exists(), "the path this refusal is about exists");

    let refused = read_file(&path).expect_err("a directory is not an authored document");
    assert_ne!(
        refused.code,
        ErrorCode::NotFound,
        "a path that exists is reported as not found, which no caller can tell \
         apart from an absent file: {refused:?}"
    );
}
