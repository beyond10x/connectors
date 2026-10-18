//! story:connection-binding-excludes-operations (architecture-decision-record:
//! provider-definitions-and-runtimes D12): a saved connection is bound to its
//! provider authority, profile declaration, validated identity, the
//! credential-side configuration (`credential_revision`) and the provider
//! executable, not to the bundle, source or selection. A binding saved before
//! it records neither digest and keeps the configuration-revision rule until a
//! revalidation records them.
use super::tests::{NOW, binding, fixture, publish_fixture};
use super::*;

const LATER: u64 = NOW + 1;

fn saved() -> Binding {
    Binding {
        credential_revision: Some("credential-1".into()),
        executable: Some("a".repeat(64)),
        ..binding()
    }
}
/// Only the operation side moved: a rebuilt bundle or another selection.
fn operations() -> Binding {
    Binding {
        configuration_revision: "config-2".into(),
        ..saved()
    }
}
/// The CA file changed: the configuration and the credential side moved.
fn trust() -> Binding {
    Binding {
        configuration_revision: "config-2".into(),
        credential_revision: Some("credential-2".into()),
        ..saved()
    }
}
/// Only the provider executable was replaced.
fn executable() -> Binding {
    Binding {
        executable: Some("b".repeat(64)),
        ..saved()
    }
}

fn validated(subject: &str, now: u64) -> ValidatedBaseline {
    ValidatedBaseline {
        identity: ExternalIdentity {
            kind: "fixture-account".into(),
            subject: subject.into(),
        },
        granted_scopes: Some(BTreeSet::from(["read".into()])),
        credential_expires_at_ms: Some(now + 3_600_000),
        collected_at_ms: now,
        valid_until_ms: now + 60_000,
    }
}

fn configured(binding: &Binding) -> Configured<'_> {
    Configured {
        configuration_revision: &binding.configuration_revision,
        credential_revision: binding.credential_revision.as_deref(),
        executable: binding.executable.as_deref(),
    }
}

fn observe(registry: &Registry, binding: &Binding, reference: &str) -> ObservedConnection {
    registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            configured(binding),
            reference,
            LATER,
            true,
        )
        .unwrap()
}

/// A published connection saved under `binding`; its reference and public
/// revision.
fn connect(registry: &Registry, binding: &Binding, subject: &str) -> (String, String) {
    let acquisition = registry.begin(binding, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let candidate = registry
        .prepare(&claim, validated(subject, NOW), 12, NOW)
        .unwrap();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = observe(registry, binding, &reference).revision;
    (reference, revision)
}

/// The recorded binding, refusal, public revision and material invalidity.
fn recorded(root: &Path, reference: &str) -> (Binding, Kept, String, Option<String>) {
    let metadata = Metadata::inspect(root).unwrap();
    let (binding, revision, material): (String, String, Option<String>) = metadata
        .connection
        .query_row(
            "SELECT binding,semantic_revision,active_material FROM registry_connections WHERE connection_ref=?1",
            [reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    let invalid = material.and_then(|version| {
        metadata
            .connection
            .query_row(
                "SELECT invalid_reason FROM registry_materials WHERE version_id=?1",
                [version],
                |r| r.get(0),
            )
            .unwrap()
    });
    let (binding, kept) = decode_stored(&binding).unwrap();
    (binding, kept, revision, invalid)
}

fn revalidate(
    registry: &Registry,
    configured: &Binding,
    reference: &str,
    revision: &str,
    subject: &str,
) -> Result<()> {
    let captured =
        registry.capture_revalidation(configured, reference, revision, LATER, LATER + 1000)?;
    let dispatched = registry.dispatch_revalidation(captured, LATER)?;
    registry.finish_revalidation(dispatched, Ok(validated(subject, LATER)), LATER)
}

fn read(registry: &Registry, configured: &Binding, reference: &str) -> Result<()> {
    let scopes = BTreeSet::from(["read".to_owned()]);
    registry.admit_read(configured, reference, &scopes, LATER)?;
    let captured = registry.capture_read(configured, reference, &scopes, LATER, LATER + 1000)?;
    registry.cancel_read(captured, LATER)
}

#[test]
fn the_classification_follows_the_credential_side_and_the_executable() {
    assert_eq!(follow(&saved(), &saved()), Follow::Same);
    assert_eq!(follow(&saved(), &operations()), Follow::Operations);
    assert_eq!(follow(&saved(), &executable()), Follow::Executable);
    let both = Binding {
        configuration_revision: "config-2".into(),
        ..executable()
    };
    assert_eq!(follow(&saved(), &both), Follow::Executable);
    assert_eq!(follow(&saved(), &trust()), Follow::Changed);
    let authority = Binding {
        provider_authority: "https://other.invalid/".into(),
        ..operations()
    };
    assert_eq!(follow(&saved(), &authority), Follow::Changed);
    // Saved before the change: today's rule, the configuration revision.
    assert_eq!(follow(&binding(), &saved()), Follow::Same);
    assert_eq!(follow(&binding(), &operations()), Follow::Changed);
    // Nothing configured to compare the credential side with.
    let unstated = Binding {
        credential_revision: None,
        ..operations()
    };
    assert_eq!(follow(&saved(), &unstated), Follow::Changed);
}

/// The acceptance rule at the registry: a selection-only change lists the
/// connection `ready`, not stale, and admits a read and an approval target
/// with no revalidation; the record replays the same in a fresh handle.
#[test]
fn an_operation_only_change_leaves_the_connection_ready_and_readable() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let observed = observe(&registry, &operations(), &reference);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    assert_eq!(observed.revision, revision);
    read(&registry, &operations(), &reference).unwrap();
    assert_eq!(
        registry.approval_target(&operations(), &reference, &BTreeSet::new()),
        Ok(revision.clone())
    );
    let before = recorded(root.path(), &reference);
    assert_eq!(before.0, saved());
    drop(registry);
    let reopened = Registry::new(root.path());
    let observed = observe(&reopened, &operations(), &reference);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    read(&reopened, &operations(), &reference).unwrap();
    assert_eq!(recorded(root.path(), &reference), before);
}

/// The next revalidation records the new revision: no credential source, the
/// same material, and the instance moves with it.
#[test]
fn an_operation_only_change_is_recorded_by_the_next_revalidation() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    revalidate(&registry, &operations(), &reference, &revision, "one").unwrap();
    let (binding, kept, _, invalid) = recorded(root.path(), &reference);
    assert_eq!(binding, operations());
    assert_eq!(kept, Kept::default());
    assert_eq!(invalid, None);
    assert_eq!(
        observe(&registry, &operations(), &reference).state,
        State::Ready
    );
}

/// An operation-only change is still the instance's connection: a reconnect
/// is admitted, and a second connect names it.
#[test]
fn an_operation_only_change_admits_repair_and_refuses_a_second_connect() {
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    registry
        .begin_repair(&operations(), &reference, &revision, LATER)
        .unwrap();
    let (_root, registry) = fixture();
    connect(&registry, &saved(), "one");
    assert_eq!(
        registry.begin(&operations(), LATER).err(),
        Some(Failure::InstanceConnected)
    );
}

/// A changed CA file moves the credential side: today's rule, `pending` and
/// stale, the read refused until `connections revalidate` follows it.
#[test]
fn a_trust_change_keeps_the_explicit_revalidation() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let observed = observe(&registry, &trust(), &reference);
    assert_eq!(observed.state, State::Pending);
    assert!(observed.stale);
    assert_eq!(
        read(&registry, &trust(), &reference),
        Err(Failure::UpgradeRequired)
    );
    revalidate(&registry, &trust(), &reference, &revision, "one").unwrap();
    assert_eq!(recorded(root.path(), &reference).0, trust());
    read(&registry, &trust(), &reference).unwrap();
}

/// Only the executable replaced: `ready`, never stale; the read is refused by
/// name for the owner to validate once under the new executable, and that
/// validation, answering the recorded identity, follows it.
#[test]
fn an_executable_only_change_follows_after_validation_confirms_the_identity() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let observed = observe(&registry, &executable(), &reference);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    let scopes = BTreeSet::from(["read".to_owned()]);
    // The preflight admits it: the owner validates before it captures.
    registry
        .admit_read(&executable(), &reference, &scopes, LATER)
        .unwrap();
    assert_eq!(
        registry
            .capture_read(&executable(), &reference, &scopes, LATER, LATER + 1000)
            .err(),
        Some(Failure::ExecutableChanged)
    );
    revalidate(&registry, &executable(), &reference, &revision, "one").unwrap();
    assert_eq!(recorded(root.path(), &reference).0, executable());
    // An executable-only follow keeps the public revision an approval binds.
    assert_eq!(recorded(root.path(), &reference).2, revision);
    read(&registry, &executable(), &reference).unwrap();
}

/// A new executable that answers another identity is refused as
/// `identity_mismatch`, and nothing changes: the binding, the refusal record,
/// the public revision and the credential's validity are as before.
#[test]
fn an_executable_only_change_answering_another_identity_changes_nothing() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let before = recorded(root.path(), &reference);
    assert_eq!(
        revalidate(&registry, &executable(), &reference, &revision, "other"),
        Err(Failure::UpgradeIdentityMismatch)
    );
    assert_eq!(recorded(root.path(), &reference), before);
    read(&registry, &saved(), &reference).unwrap();
    let observed = observe(&registry, &executable(), &reference);
    assert_eq!(observed.state, State::Ready);
}

/// A connection saved before this change records no credential side: any
/// configuration revision change stales it as before, and the revalidation
/// that follows records both digests, after which an operation-only change
/// leaves it ready.
#[test]
fn a_connection_saved_before_the_change_keeps_todays_rule_until_revalidated() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &binding(), "one");
    let observed = observe(&registry, &operations(), &reference);
    assert_eq!(observed.state, State::Pending);
    assert!(observed.stale);
    assert_eq!(
        read(&registry, &operations(), &reference),
        Err(Failure::UpgradeRequired)
    );
    revalidate(&registry, &operations(), &reference, &revision, "one").unwrap();
    assert_eq!(recorded(root.path(), &reference).0, operations());
    let later = Binding {
        configuration_revision: "config-3".into(),
        ..operations()
    };
    let observed = observe(&registry, &later, &reference);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    read(&registry, &later, &reference).unwrap();
}

/// A legacy connection revalidated under its own configuration revision
/// records the digests too, and an identity it does not answer any longer
/// invalidates it as before.
#[test]
fn a_connection_saved_before_the_change_records_the_digests_on_revalidation() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &binding(), "one");
    revalidate(&registry, &saved(), &reference, &revision, "one").unwrap();
    assert_eq!(recorded(root.path(), &reference).0, saved());
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &binding(), "one");
    assert_eq!(
        revalidate(&registry, &saved(), &reference, &revision, "other"),
        Err(Failure::IdentityMismatch)
    );
    assert_eq!(
        recorded(root.path(), &reference).3.as_deref(),
        Some("invalid")
    );
}

// Adversary pass 1 (story:connection-binding-excludes-operations).

/// A revalidation under `configured` whose provider refuses the stored
/// credential (`invalid`), as the owner's `connections revalidate` records an
/// `InvalidCredential` answer (owner/supervisor.rs `Task::Revalidate`).
fn refuse(registry: &Registry, configured: &Binding, reference: &str, revision: &str) {
    let captured = registry
        .capture_revalidation(configured, reference, revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), LATER)
        .unwrap();
}

/// A connection whose credential the provider refused (Stale) is repaired by
/// `connections reconnect`. Before D12 the executable was not in the binding,
/// so a new provider executable left that repair admitted, and a connection
/// saved before D12 still has it. One saved after D12 must keep it too: D12
/// says an executable-only change does not stale the connection, not that it
/// takes away its repair (the only remaining step would be a revocation).
#[test]
fn adv_a_stale_connection_stays_repairable_after_an_executable_only_change() {
    // Control: a connection saved before D12 (no digests recorded).
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &binding(), "one");
    refuse(&registry, &binding(), &reference, &revision);
    let legacy_executable = Binding {
        executable: Some("b".repeat(64)),
        ..binding()
    };
    let revision = observe(&registry, &legacy_executable, &reference).revision;
    assert_eq!(
        registry
            .begin_repair(&legacy_executable, &reference, &revision, LATER)
            .err(),
        None,
        "control: a pre-D12 connection is repairable after an executable change"
    );

    // The same steps on a connection saved after D12.
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    refuse(&registry, &saved(), &reference, &revision);
    let observed = observe(&registry, &executable(), &reference);
    assert_eq!(observed.state, State::Stale);
    assert_eq!(
        registry
            .begin_repair(&executable(), &reference, &observed.revision, LATER)
            .err(),
        None,
        "a Stale connection saved after D12 cannot be reconnected once only the provider executable changed"
    );
}

/// Only the executable changed, and `connections revalidate` under the new
/// executable is refused by the provider as an invalid credential. The
/// registry records nothing (`finish_revalidation`, `Follow::Executable`), so
/// the connection still lists `ready`; a reconnect is then refused as
/// `UpgradeRequired` (`revalidate_connection`), which repeats the same
/// refusal: no step but a revocation leaves the loop.
#[test]
fn adv_a_credential_refused_under_a_new_executable_leaves_a_repair() {
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    refuse(&registry, &executable(), &reference, &revision);
    let observed = observe(&registry, &executable(), &reference);
    let repair = registry
        .begin_repair(&executable(), &reference, &observed.revision, LATER)
        .err();
    assert_eq!(
        repair, None,
        "after the new executable refused the credential, the connection lists {:?} and reconnect is refused as {repair:?}",
        observed.state
    );
}

/// Only the executable changed and it answers another identity: the
/// revalidation is refused as `UpgradeIdentityMismatch`, which the owner maps
/// to `identity_mismatch` with `next_action = create_connection`
/// (owner.rs `From<registry::Failure>`, `reconnect`). The connect that action
/// names is then refused as `UpgradeRequired` (`revalidate_connection`),
/// which repeats the refused revalidation: the two next actions loop.
#[test]
fn adv_an_executable_identity_mismatch_does_not_send_connect_back_to_revalidation() {
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    assert_eq!(
        revalidate(&registry, &executable(), &reference, &revision, "other"),
        Err(Failure::UpgradeIdentityMismatch)
    );
    let connect = registry.begin(&executable(), LATER).err();
    assert_ne!(
        connect,
        Some(Failure::UpgradeRequired),
        "create_connection after an executable identity mismatch answers revalidate_connection again"
    );
}

#[path = "binding_scope_security_tests.rs"]
mod security;

// Adversary pass 2 (story:connection-binding-excludes-operations, the launch
// for a description: contracts/cli/v1alpha1/semantics.md, "Launching for a
// description").

fn adv2_admits(
    registry: &Registry,
    instance: &str,
    configured_binding: &Binding,
    reference: Option<&str>,
) -> Result<bool> {
    registry.launch_admits(
        instance,
        "fixture-adapter",
        configured(configured_binding),
        reference,
    )
}

/// `connect`, with a credential that states no expiry: `launch_admits` reads
/// the wall clock (as `approval_target` does), which is past the fixture's
/// `NOW` plus the one-hour expiry `validated` states. Its evidence is expired,
/// which the contract still admits (`pending`).
fn adv2_connect(registry: &Registry, binding: &Binding, subject: &str) -> (String, String) {
    let acquisition = registry.begin(binding, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let baseline = ValidatedBaseline {
        credential_expires_at_ms: None,
        ..validated(subject, NOW)
    };
    let candidate = registry.prepare(&claim, baseline, 12, NOW).unwrap();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = observe(registry, binding, &reference).revision;
    (reference, revision)
}

/// Each class the contract names, one fixture each: a current usable
/// connection admits the launch under the saved, an operation-only and an
/// executable-only entry; a changed credential side, a connection saved
/// without the digest under another revision, a Stale one, a revoked one, an
/// unknown name and a connection of another instance admit none.
#[test]
fn adv2_launch_admits_only_a_current_usable_connection_of_the_instance() {
    let (_root, registry) = fixture();
    let (reference, _) = adv2_connect(&registry, &saved(), "one");
    for entry in [saved(), operations(), executable()] {
        assert_eq!(
            adv2_admits(&registry, "fixture-instance", &entry, Some(&reference)),
            Ok(true),
            "{}",
            entry.configuration_revision
        );
        assert_eq!(
            adv2_admits(&registry, "fixture-instance", &entry, None),
            Ok(true)
        );
    }
    assert_eq!(
        adv2_admits(&registry, "fixture-instance", &trust(), None),
        Ok(false),
        "a changed credential side is not current"
    );
    assert_eq!(
        adv2_admits(&registry, "other-instance", &saved(), Some(&reference)),
        Ok(false),
        "a connection named under another instance"
    );
    assert_eq!(
        adv2_admits(&registry, "fixture-instance", &saved(), Some("missing")),
        Ok(false)
    );

    let (_root, registry) = fixture();
    adv2_connect(&registry, &binding(), "one");
    let legacy_next = Binding {
        configuration_revision: "config-2".into(),
        ..binding()
    };
    assert_eq!(
        adv2_admits(&registry, "fixture-instance", &legacy_next, None),
        Ok(false),
        "no digest recorded, another configuration revision"
    );

    let (_root, registry) = fixture();
    let (reference, revision) = adv2_connect(&registry, &saved(), "one");
    refuse(&registry, &saved(), &reference, &revision);
    assert_eq!(observe(&registry, &saved(), &reference).state, State::Stale);
    assert_eq!(
        adv2_admits(&registry, "fixture-instance", &operations(), None),
        Ok(false),
        "a Stale connection"
    );

    let (_root, registry) = fixture();
    let (reference, revision) = adv2_connect(&registry, &saved(), "one");
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    assert_eq!(
        adv2_admits(
            &registry,
            "fixture-instance",
            &operations(),
            Some(&reference)
        ),
        Ok(false),
        "a revoked connection"
    );
}

/// Restated after review (round 2, finding F2) for the contract as narrowed:
/// an identity refused under an executable-only change records nothing and
/// changes nothing, so it does not stop a credential-free launch for a
/// description, while the connection's reads keep refusing until a validation
/// answers the recorded identity.
#[test]
fn adv2_an_identity_refused_under_a_new_executable_admits_a_launch_but_no_read() {
    let (_root, registry) = fixture();
    let (reference, revision) = adv2_connect(&registry, &saved(), "one");
    assert_eq!(
        revalidate(&registry, &executable(), &reference, &revision, "other"),
        Err(Failure::UpgradeIdentityMismatch)
    );
    let selection_too = Binding {
        configuration_revision: "config-2".into(),
        ..executable()
    };
    assert_eq!(
        adv2_admits(
            &registry,
            "fixture-instance",
            &selection_too,
            Some(&reference)
        ),
        Ok(true)
    );
    let scopes = BTreeSet::from(["read".to_owned()]);
    assert_eq!(
        registry
            .capture_read(&selection_too, &reference, &scopes, LATER, LATER + 1000)
            .err(),
        Some(Failure::ExecutableChanged)
    );
}
