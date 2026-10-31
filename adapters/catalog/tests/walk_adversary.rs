//! Adversary pass on story:catalog-operation-declares-its-walk. A declared walk is a promise that
//! every page it builds is a page the operation admits: each page's input (the caller's, with the
//! walk's `query`, and from the second page on the position as `parameter` and no `first_only`)
//! is validated against the same admitted input schema an invoke validates against
//! (`connectors_sdk::validate`, `Engine::invoke`). The loader checks that `query` and
//! `first_only` name query parameters of the operation, and nothing about whether the pages they
//! shape are still admitted: these cases load walks whose pages the operation refuses.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection, walk::Walk};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn file(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(
            root()
                .join("providers")
                .join(provider)
                .join("operations.json"),
        )
        .unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}

fn committed(provider: &str) -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), provider).unwrap()
}

/// Load `provider`'s shipped selection file with `id`'s walk changed by `change`. When the
/// engine refuses it, the refusal; when it loads, a panic naming what the operation's own input
/// schema answers to the walk's page built from `caller` (the first page, or with `position`
/// a later one), so the red output shows the page the loaded walk cannot send.
fn refusal(
    provider: &str,
    id: &str,
    change: impl Fn(&mut Value),
    caller: Value,
    position: Option<Value>,
) -> String {
    let mut selections = file(provider);
    let at = selections.iter().position(|s| s.id == id).unwrap();
    let mut selection = serde_json::to_value(&selections[at]).unwrap();
    change(&mut selection["walk"]);
    selections[at] = serde_json::from_value(selection).unwrap();
    let engine = match Engine::new(&committed(provider), "/", &selections) {
        Err(error) => return error.message,
        Ok(engine) => engine,
    };
    let operation = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == id)
        .unwrap();
    let walk = Walk::declared(&operation.input_schema).unwrap().unwrap();
    let page = walk.page_input(&caller, position.as_ref());
    let answer = connectors_sdk::validate(&operation.input_schema, &page)
        .map_err(|error| format!("{:?}: {}", error.code, error.message));
    panic!("`{id}`: the walk loads; its page {page} is answered {answer:?}");
}

/// Every page of a walk sends its `query`, so a value the operation's schema refuses makes every
/// page `invalid_input`. `include_boundary_indicators` is a boolean (it takes `true`, `false`
/// and their strings); the loader checks only that `"yes"` is a scalar.
#[test]
fn a_walk_query_value_of_another_type_is_refused_when_the_selection_loads() {
    let refused = refusal(
        "zendesk",
        "ticket.audits",
        |w| w["query"] = json!({"include_boundary_indicators": "yes"}),
        json!({"ticket_id": 101, "page[size]": 2}),
        None,
    );
    assert!(refused.contains("include_boundary_indicators"), "{refused}");
}

/// The same for a value outside the selection's own bounds: `tickets.incremental` bounds
/// `per_page` to 1..=1000.
#[test]
fn a_walk_query_value_outside_the_selection_bounds_is_refused_when_the_selection_loads() {
    let refused = refusal(
        "zendesk",
        "tickets.incremental",
        |w| w["query"] = json!({"per_page": 5000}),
        json!({"start_time": 1_758_000_000u64}),
        None,
    );
    assert!(refused.contains("per_page"), "{refused}");
}

/// `first_only` parameters are left out from the second page on, so naming one the selection
/// requires (`users.history.list` requires `startHistoryId` on every call) makes every page after
/// the first `invalid_input`. The loader checks only that it is a query parameter.
#[test]
fn a_first_only_parameter_the_selection_requires_is_refused_when_the_selection_loads() {
    let refused = refusal(
        "google-gmail",
        "users.history.list",
        |w| w["first_only"] = json!(["startHistoryId"]),
        json!({"userId": "me", "startHistoryId": "4711"}),
        Some(json!("page-token-2")),
    );
    assert!(refused.contains("startHistoryId"), "{refused}");
}
