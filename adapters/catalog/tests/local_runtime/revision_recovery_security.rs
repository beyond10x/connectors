//! Security review of story:connection-binding-excludes-operations through
//! the production CLI, owner and catalog provider child: the story's
//! acceptance statement, and the owner's own validation of a connection whose
//! provider executable alone changed (owner/supervisor.rs `Task::Invoke`).
use super::*;

fn describe_operation(scenario: &Scenario, operation: &str) -> Output {
    scenario.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        operation,
    ])
}

/// The story's acceptance statement, verbatim: a connection saved under S1
/// lists `ready` after only its selection changes to S2, "and that added read
/// succeeds with no `connections revalidate`, no credential source and no
/// prompt".
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security_the_added_read_succeeds_after_a_selection_only_change_without_revalidation() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let validations = scenario.validations();
    let revision = scenario.revision.borrow().clone();
    scenario.select(&S2);
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "ready", "{listed}");
    let described = describe_operation(&scenario, "project.languages");
    assert!(
        described.status.success(),
        "the added read cannot be invoked without a revalidation: operations describe answered {}",
        String::from_utf8_lossy(&described.stderr)
    );
    let read = success(scenario.invoke("project.languages", json!({"id": "org/project"})));
    assert!(read["result"].is_object(), "{read}");
    assert_eq!(scenario.describe()["summary"]["revision"], revision);
    assert_eq!(scenario.validations(), validations);
}

/// Invariant 3 through the owner's read path: only the executable replaced,
/// its description cached by a revalidation the new provider refused. A read
/// then validates the stored credential under the new executable itself; while
/// the provider answers another identity it is refused as `identity_mismatch`
/// and the connection is unchanged; once the recorded identity answers, one
/// validation follows it and later reads validate nothing.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security_a_read_follows_an_executable_only_change_only_after_the_identity_answers() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let before = scenario.describe();
    scenario.replace_executable(1);
    scenario
        .provider
        .identity_shift
        .store(100, Ordering::SeqCst);
    refusal(scenario.run(Recovery::Revalidate), "identity_mismatch");
    let described = describe_operation(&scenario, "project.get");
    assert!(
        described.status.success(),
        "no description is cached after the refused revalidation: {}",
        String::from_utf8_lossy(&described.stderr)
    );
    let validations = scenario.validations();
    refusal(
        scenario.invoke("project.get", json!({"id": "org/project"})),
        "identity_mismatch",
    );
    assert_eq!(scenario.validations(), validations + 1);
    let after = scenario.describe();
    assert_eq!(after["summary"]["revision"], before["summary"]["revision"]);
    assert_eq!(after["summary"]["state"], "ready", "{after}");
    assert_eq!(after["external_identity"], before["external_identity"]);
    scenario.provider.identity_shift.store(0, Ordering::SeqCst);
    let validations = scenario.validations();
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
    assert_eq!(scenario.validations(), validations + 1);
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
    assert_eq!(scenario.validations(), validations + 1);
}

// Security review, round 2: the launch for a description
// (contracts/cli/v1alpha1/semantics.md, "Launching for a description").

fn security2_cached_source(scenario: &Scenario) -> Value {
    success(
        scenario
            .cli
            .run(&["adapters", "describe", "--adapter", "gitlab"]),
    )["source"]
        .clone()
}

/// A revoked connection admits no launch: after a selection-only change,
/// `operations describe` answers `description_unavailable` and caches nothing.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security2_a_revoked_connection_starts_nothing_for_a_description() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let revision = scenario.revision.borrow().clone();
    success(scenario.cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "gitlab",
        "--connection",
        &scenario.reference,
        "--expected-revision",
        &revision,
    ]));
    scenario.select(&S2);
    refusal(
        describe_operation(&scenario, "project.languages"),
        "description_unavailable",
    );
    assert_eq!(security2_cached_source(&scenario), "configuration");
}

/// An operator's `adapters stop` is not resumed by a read command's launch:
/// after a selection-only change the instance stays suppressed and no
/// description is cached.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security2_a_description_launch_does_not_resume_a_stopped_instance() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let coordinates = scenario.cli.status();
    success(scenario.cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "gitlab",
        "--expected-revision",
        coordinates["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        coordinates["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        coordinates["child_incarnation"].as_str().unwrap(),
    ]));
    assert_eq!(scenario.cli.status()["state"], "suppressed");
    scenario.select(&S2);
    let described = describe_operation(&scenario, "project.languages");
    assert!(
        !described.status.success(),
        "a stopped instance served a launched description"
    );
    assert_eq!(scenario.cli.status()["state"], "suppressed");
    assert_eq!(security2_cached_source(&scenario), "configuration");
}

/// contracts/cli/v1alpha1/owner.md, restated after review (round 2): passive
/// commands contact only an existing socket, and `operations describe` starts
/// the owner only to launch for a description. With no owner running and a
/// connection that is not current (its trust roots changed), it starts none.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security2_operations_describe_starts_no_owner_without_a_current_connection() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    scenario.cli.shutdown();
    assert!(!scenario.cli.paths.state.join("owner.sock").exists());
    scenario.trust_another_root();
    let described = describe_operation(&scenario, "project.get");
    assert!(!described.status.success());
    assert!(
        !scenario.cli.paths.state.join("owner.sock").exists(),
        "operations describe started the owner (answered success = {})",
        described.status.success()
    );
}
