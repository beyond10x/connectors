//! Adversary case for story:websearch-contract-states-host-deadline: the published tool
//! profile must not promise a longer call than the websearch contract now states.
use serde_json::Value;

/// `semantics.md` now states a call ends at 30 s in the CLI and 20 s at the service, and that
/// the transport's 160 s ceiling never applies. The tool profile is the source of the
/// published `generated/tool-contract.yaml`; its `bounds.timeout_ms.max` is what a tool
/// consumer is told a call may take.
#[test]
fn the_tool_profile_bounds_no_call_past_the_cli_deadline() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/spec/tool-profiles.json");
    let profiles: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let operations = profiles["operations"].as_object().unwrap();
    assert_eq!(operations.len(), 3);
    for (id, operation) in operations {
        let max = operation["profile"]["bounds"]["timeout_ms"]["max"]
            .as_u64()
            .unwrap();
        assert!(
            max <= 30_000,
            "{id}: tool profile promises timeout_ms.max {max}, the contract says a call ends at 30 s"
        );
    }
}
