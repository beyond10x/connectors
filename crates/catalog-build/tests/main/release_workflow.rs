//! The release graph must overlap compilation and validation without bypassing publication gates.

use serde_norway::Value;
use std::collections::BTreeSet;

fn workflow() -> Value {
    serde_norway::from_str(include_str!("../../../../.github/workflows/release.yml"))
        .expect("release workflow must be valid YAML")
}

#[test]
fn native_builds_do_not_wait_for_validation() {
    let workflow = workflow();
    let build = &workflow["jobs"]["build"];
    assert!(
        build["needs"].is_null(),
        "native builds must start concurrently with checks and the sharded gate"
    );
    assert!(
        build["if"].is_null(),
        "both tag pushes and manual dispatch must exercise all native targets"
    );
    let targets: BTreeSet<_> = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("native build matrix")
        .iter()
        .map(|entry| entry["target"].as_str().expect("target"))
        .collect();
    assert_eq!(
        targets,
        BTreeSet::from([
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ])
    );
}

#[test]
fn publishing_joins_every_gate_and_refuses_manual_dispatch() {
    let workflow = workflow();
    let release = &workflow["jobs"]["release"];
    let dependencies: BTreeSet<_> = release["needs"]
        .as_sequence()
        .expect("explicit publication dependencies")
        .iter()
        .map(|dependency| dependency.as_str().expect("job id"))
        .collect();
    assert_eq!(dependencies, BTreeSet::from(["checks", "gate", "build"]));
    // Explicit result checks prevent failed, cancelled or skipped matrix legs from publishing.
    // Checking the event as well as the ref keeps manual dispatch build-only even on a tag.
    assert_eq!(
        release["if"].as_str(),
        Some("${{ always() && github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') && needs.checks.result == 'success' && needs.gate.result == 'success' && needs.build.result == 'success' }}")
    );
}

#[test]
fn every_job_uses_one_immutable_candidate_and_only_publication_can_write() {
    let workflow = workflow();
    assert_eq!(workflow["permissions"]["contents"].as_str(), Some("read"));
    for (name, job) in workflow["jobs"].as_mapping().expect("jobs") {
        let name = name.as_str().expect("job id");
        if name == "release" {
            assert_eq!(job["permissions"]["contents"].as_str(), Some("write"));
        } else {
            assert!(
                job["permissions"].is_null(),
                "{name} inherits read-only access"
            );
        }
        let checkout = job["steps"]
            .as_sequence()
            .expect("job steps")
            .iter()
            .find(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|action| action.starts_with("actions/checkout@"))
            })
            .expect("job checks out candidate");
        assert_eq!(
            checkout["with"]["ref"].as_str(),
            Some("${{ github.sha }}"),
            "{name}"
        );
        assert_eq!(
            checkout["with"]["persist-credentials"].as_bool(),
            Some(false),
            "{name}"
        );
    }
}
