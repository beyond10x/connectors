//! Compiled compatibility facts, available without opening installation state.

use serde_json::{json, Value};

/// Report the calling binary's version and the formats its existing owners support.
///
/// The frontend supplies its package version because the console library's package version
/// need not identify the binary that linked it. Catalog metadata comes from embedded bytes;
/// credential and session versions come from their readers and writers, without opening either.
#[must_use]
pub fn run(cli_version: &str) -> Value {
    let catalog = catalog::reader::embedded();
    let [legacy, prepared] = connector_secrets::file::supported_format_versions();

    json!({
        "cli_version": cli_version,
        "catalog": {
            "schema_version": catalog.schema_version(),
            "digest": catalog.digest(),
        },
        "credential_store": {
            "read_versions": [legacy, prepared],
            "write_versions": [legacy, prepared],
            "initial_write_version": legacy,
            "prepared_write_version": prepared,
            "transition": format!(
                "Ordinary writes retain v{legacy} until a prepared-transaction write changes \
                 the store to v{prepared}; later writes retain v{prepared}."
            ),
        },
        "session_metadata_version": connectors_client::SESSION_METADATA_VERSION,
        "installation": {
            "command": "task install",
            "context": "Run from a Connectors source checkout with Rust and Task installed.",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_calling_binary_and_owning_formats_supply_the_reported_versions() {
        let report = run("calling-binary-test-version");
        assert_eq!(report["cli_version"], "calling-binary-test-version");
        let catalog = catalog::reader::embedded();
        assert_eq!(
            report["catalog"]["schema_version"],
            catalog.schema_version()
        );
        assert_eq!(report["catalog"]["digest"], catalog.digest());
        let formats = connector_secrets::file::supported_format_versions();
        assert_eq!(report["credential_store"]["read_versions"], json!(formats));
        assert_eq!(report["credential_store"]["write_versions"], json!(formats));
        assert_eq!(
            report["session_metadata_version"],
            connectors_client::SESSION_METADATA_VERSION
        );
    }
}
