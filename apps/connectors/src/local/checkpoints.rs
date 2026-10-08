//! `setup checkpoints-enable` (contracts/cli/v1alpha1/semantics.md §3): enables
//! Entity Runtime durable open checkpoints on the existing store, one-way for
//! connectors 0.32.0 and earlier.
use super::failure;
use connectors_cli_contract::{HandlerReply, Invocation};
use connectors_host::local::{config::Paths, metadata::Checkpoints, owner};
use serde_json::{Value, json};

/// The one confirmation `--confirm` admits, and the change every result states.
const ONE_WAY: &str = "one-way";
/// The newest connectors release that cannot open a store carrying them.
const NEWEST_INCOMPATIBLE_RELEASE: &str = "0.32.0";

pub(super) fn execute(call: &Invocation<'_>, paths: &Paths) -> Result<Value, HandlerReply> {
    // Refused before any lock is taken or the store is opened.
    if call.input.get("confirm").and_then(Value::as_str) != Some(ONE_WAY) {
        return Err(failure(
            "confirmation_required",
            "arguments",
            "retry_explicitly",
            true,
        ));
    }
    let disposition = match owner::enable_checkpoints(paths) {
        Ok(Checkpoints::Enabled) => "enabled",
        Ok(Checkpoints::AlreadyEnabled) => "already_enabled",
        Err(error) => {
            return Err(match error.code {
                owner::Code::LifecycleConflict => {
                    failure("lifecycle_conflict", "admission", "stop_owner", false)
                }
                // Enabling an enabled store changes nothing: running it again
                // answers `already_enabled` or enables.
                owner::Code::OutcomeUnknown => {
                    failure("outcome_unknown", "publication", "retry_explicitly", false)
                }
                _ => failure("metadata_unavailable", "observation", "retry_status", false),
            });
        }
    };
    Ok(json!({
        "disposition": disposition,
        "state_path": paths.state,
        "change": ONE_WAY,
        "newest_incompatible_release": NEWEST_INCOMPATIBLE_RELEASE,
    }))
}
