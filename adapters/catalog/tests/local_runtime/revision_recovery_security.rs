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
