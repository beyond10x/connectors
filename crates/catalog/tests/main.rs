//! **One integration-test binary for this crate.**
//!
//! Every `.rs` file directly under `tests/` is its own crate, which Cargo links into its own
//! executable carrying the entire dependency graph. The files under `tests/main/` are therefore
//! modules of this single test target. Run one of them with
//! `cargo test -p catalog --test main <module>::`.
//!
//! The `#[path]` attribute on every declaration is load-bearing: this file is a crate root, and a
//! crate root resolves a bare `mod x;` in its **own** directory (`tests/`), never in `tests/main/`.

#[path = "main/consumer_api.rs"]
mod consumer_api;
#[path = "main/pack_table.rs"]
mod pack_table;

#[test]
fn source_fidelity_catalog_describes_translated_input_and_output_from_the_document() {
    for suffix in ["list", "create", "update", "delete"] {
        let id = format!("gitlab-pipeline-schedule-{suffix}");
        let operation = catalog::operations()
            .find(|operation| operation.id == id)
            .unwrap();
        assert_eq!(
            operation.request_semantics,
            catalog::RequestSemantics::OpenApi30JsonV1
        );
        let input: serde_json::Value = serde_json::from_str(operation.input_schema).unwrap();
        assert_eq!(
            input["properties"]["id"]["oneOf"],
            serde_json::json!([{"type":"string"},{"type":"integer"}])
        );
        if suffix == "delete" {
            assert!(operation.output_schema.is_none());
        } else {
            let response: serde_json::Value = serde_json::from_str(
                operation
                    .output_schema
                    .expect("declared successful response"),
            )
            .unwrap();
            assert_eq!(response["type"], "object");
            assert!(response["properties"]["inputs"].get("items").is_none());
        }
    }
    assert_eq!(
        catalog::operations()
            .find(|operation| operation.id == "gitlab-user-get")
            .unwrap()
            .request_semantics,
        catalog::RequestSemantics::LegacyV1
    );
}
