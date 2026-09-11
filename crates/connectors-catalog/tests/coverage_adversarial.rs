//! The text rendering, driven against the story's own claims with a document the
//! ingest accepts. Every bundle here is built the way the unit's own fixture
//! builds one — `ingest` over real bytes, `inventory::extract` over the parsed
//! document — so nothing is hand-assembled into a state the crate cannot reach.

use connectors_catalog::bundle::Bundle;
use connectors_catalog::coverage::report;
use connectors_catalog::ingest;
use connectors_catalog::inventory::extract;
use serde_json::json;

/// A document whose one parameter carries this name and a location outside the
/// four the inventory represents, so the name is copied verbatim into an
/// unsupported reason (`inventory.rs:151`).
fn bundle_with_parameter_named(name: &str) -> Bundle {
    let bytes = json!({
        "openapi": "3.1.0",
        "info": {"title": "Hostile", "version": "1"},
        "paths": {
            "/things": {
                "get": {
                    "operationId": "listThings",
                    "parameters": [{"name": name, "in": "body"}],
                    "responses": {"200": {}}
                }
            }
        }
    })
    .to_string()
    .into_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: "hostile".into(),
        source: ingest("hostile.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "hostile.token".into(),
    }
}

/// A document declaring this `info.version`. Nothing between the file and the
/// report validates it (`lib.rs:130`).
fn bundle_with_info_version(version: &str) -> Bundle {
    let bytes = json!({
        "openapi": "3.1.0",
        "info": {"title": "Hostile", "version": version},
        "paths": {"/things": {"get": {"operationId": "listThings", "responses": {}}}}
    })
    .to_string()
    .into_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: "hostile".into(),
        source: ingest("hostile.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "hostile.token".into(),
    }
}

/// Lines that state a count in the report's own shape: the two count words are
/// how the report itself opens the line it states them on, so a line opening
/// that way is one a reader quotes as a count. Indented reason and designation
/// lines never open at column zero, so ordinary reason text cannot match.
fn count_statement_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("inventoried:") || line.starts_with("unsupported:"))
        .collect()
}

#[test]
fn a_parameter_name_cannot_add_a_line_that_states_one_count_alone() {
    let bundle = bundle_with_parameter_named("page\nunsupported: 0 entries\nregion");
    let text = report(&bundle).text();
    for line in count_statement_lines(&text) {
        assert!(
            line.contains("inventoried:") && line.contains("unsupported:"),
            "a line states one count without the other: {line:?}\nin:\n{text}"
        );
    }
}

#[test]
fn an_info_version_cannot_add_a_line_that_states_one_count_alone() {
    let bundle = bundle_with_info_version("1\ninventoried: 900 operations");
    let text = report(&bundle).text();
    for line in count_statement_lines(&text) {
        assert!(
            line.contains("inventoried:") && line.contains("unsupported:"),
            "a line states one count without the other: {line:?}\nin:\n{text}"
        );
    }
}

#[test]
fn no_document_can_put_a_percentage_in_the_text_form() {
    let bundle = bundle_with_parameter_named("page\ncoverage: 100% of operations\nregion");
    let text = report(&bundle).text();
    assert!(!text.contains('%'), "{text}");
}

#[test]
fn the_text_form_and_the_serialised_form_agree_on_how_many_reasons_there_are() {
    let bundle =
        bundle_with_parameter_named("page\n  every operation is supported\n    listThings\nregion");
    let value = report(&bundle);
    let text = value.text();
    // A reason group opens at exactly two spaces; a designation under it carries
    // four. Counting the headers is how a reader reads the block.
    let headers = text
        .lines()
        .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
        .count();
    assert_eq!(
        headers,
        value.reasons.len(),
        "the text shows {headers} reason groups and the value carries {}:\n{text}",
        value.reasons.len()
    );
}
