//! `setup owner-stop` (contracts/cli/v1alpha1/owner.md, "Stopping an owner"):
//! asks the running owner of the state directory, of any build, to drain and
//! exit, and waits a bounded time for it. After an upgrade this is the command
//! the `stop_owner` next action names. The CLI never signals a process.
use super::{failure, owner_failure};
use connectors_cli_contract::HandlerReply;
use connectors_host::local::{
    config::Paths,
    owner::{self, OwnerStop},
};
use serde_json::{Value, json};
use std::time::Duration;

/// How long the command waits for the owner to answer and to exit.
const BOUND: Duration = Duration::from_secs(30);
/// Test builds may shorten the bound through the environment; a release build
/// has no override.
#[cfg(debug_assertions)]
fn bound() -> Duration {
    std::env::var("CONNECTORS_TEST_OWNER_STOP_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(BOUND)
}
#[cfg(not(debug_assertions))]
fn bound() -> Duration {
    BOUND
}

pub(super) fn execute(paths: &Paths) -> Result<Value, HandlerReply> {
    match owner::stop_owner(paths, bound()) {
        Ok(OwnerStop::Stopped { host_incarnation }) => Ok(json!({
            "disposition": "stopped",
            "state_path": paths.state,
            "host_incarnation": host_incarnation,
        })),
        Ok(OwnerStop::NoOwner) => Ok(json!({
            "disposition": "no_owner",
            "state_path": paths.state,
        })),
        Err(error) => Err(match error.code {
            // The state directory, its socket or its lock belongs to another user.
            owner::Code::Forbidden => failure("forbidden", "stop", "check_configuration", false),
            // The lock is held but no owner answered the greeting or the request.
            owner::Code::Unavailable => failure("unavailable", "stop", "retry_explicitly", false),
            // It accepted the request and still runs at the bound.
            owner::Code::Timeout => failure("timeout", "stop", "retry_explicitly", false),
            _ => owner_failure(error),
        }),
    }
}
