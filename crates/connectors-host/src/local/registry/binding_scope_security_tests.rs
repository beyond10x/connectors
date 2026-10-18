//! Security review of story:connection-binding-excludes-operations
//! (architecture-decision-record:provider-definitions-and-runtimes D12) at the
//! registry: integrity conformance checks of the binding's credential side,
//! its executable digest and their record across a full metadata replay.
use super::*;
#[cfg(unix)]
use rusqlite::{Connection, OpenFlags};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
const FLOOR: &str = "registry-clock.floor";
#[cfg(unix)]
const STORE: &str = "metadata.sqlite3";

/// Both the provider executable and the credential side moved.
fn executable_and_trust() -> Binding {
    Binding {
        executable: Some("b".repeat(64)),
        ..trust()
    }
}

/// A logical copy of the store at `from` into `to`, a path this process has
/// never opened, so the next open replays the ER journal in full.
#[cfg(unix)]
fn copy_store(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    std::fs::set_permissions(to, std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = to.join(STORE);
    Connection::open_with_flags(from.join(STORE), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .execute("VACUUM INTO ?1", [target.to_str().unwrap()])
        .unwrap();
    Connection::open(&target)
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    std::fs::write(to.join("metadata.lock"), b"").unwrap();
    let mut names = vec![STORE, "metadata.lock"];
    if from.join(FLOOR).exists() {
        std::fs::copy(from.join(FLOOR), to.join(FLOOR)).unwrap();
        names.push(FLOOR);
    }
    for name in names {
        std::fs::set_permissions(to.join(name), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}

/// Invariant 3: an executable-only change whose validation the new provider
/// refuses (`invalid`) changes nothing: no invalidation, no refusal record, no
/// revision move, and the connection still reads under its saved binding.
#[test]
fn security_an_executable_only_refusal_changes_nothing() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let before = recorded(root.path(), &reference);
    let captured = registry
        .capture_revalidation(&executable(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), LATER)
        .unwrap();
    assert_eq!(recorded(root.path(), &reference), before);
    read(&registry, &saved(), &reference).unwrap();
}

/// Invariant 1: an executable change that comes with a credential-side change
/// is a credential-side change: stale, and refused until an explicit
/// revalidation, at the preflight as at the capture.
#[test]
fn security_an_executable_and_credential_side_change_keeps_the_explicit_rule() {
    let (_root, registry) = fixture();
    let (reference, _) = connect(&registry, &saved(), "one");
    assert_eq!(follow(&saved(), &executable_and_trust()), Follow::Changed);
    let observed = observe(&registry, &executable_and_trust(), &reference);
    assert_eq!(observed.state, State::Pending);
    assert!(observed.stale);
    assert_eq!(
        read(&registry, &executable_and_trust(), &reference),
        Err(Failure::UpgradeRequired)
    );
}

/// Invariant 5, restated for the coordinator's decision of 2026-10-18 (writes
/// follow an executable-only change as reads do): an approval target names a
/// connection whose executable alone changed, and records nothing; nothing is
/// sent under it before a validation under the new executable, since its
/// capture still refuses as `ExecutableChanged`.
#[test]
fn security_an_approval_target_names_an_executable_change_but_nothing_is_sent_unvalidated() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let before = recorded(root.path(), &reference);
    assert_eq!(
        registry.approval_target(&executable(), &reference, &BTreeSet::new()),
        Ok(revision)
    );
    assert_eq!(recorded(root.path(), &reference), before);
    let scopes = BTreeSet::from(["read".to_owned()]);
    assert_eq!(
        registry
            .capture_read(&executable(), &reference, &scopes, LATER, LATER + 1000)
            .err(),
        Some(Failure::ExecutableChanged)
    );
}

/// Invariant 4: the digests a revalidation recorded survive a full replay of
/// the ER journal in a fresh path, and the journal itself carries them.
#[cfg(unix)]
#[test]
fn security_a_full_replay_keeps_the_recorded_digests() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    revalidate(&registry, &executable(), &reference, &revision, "one").unwrap();
    assert_eq!(recorded(root.path(), &reference).0, executable());
    drop(registry);
    let fresh = tempfile::tempdir().unwrap();
    let state = fresh.path().join("state");
    copy_store(root.path(), &state);
    let replayed = Registry::new(&state);
    assert_eq!(
        replayed
            .recorded_binding("fixture-instance", "fixture-adapter", &reference)
            .unwrap(),
        executable()
    );
    let journal: Vec<String> = {
        let connection =
            Connection::open_with_flags(state.join(STORE), OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let mut statement = connection
            .prepare(
                "SELECT CAST(data AS TEXT) FROM connectors_er_events \
                 UNION ALL SELECT CAST(bytes AS TEXT) FROM connectors_er_blobs",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, Option<String>>(0))
            .unwrap()
            .filter_map(|row| row.unwrap())
            .collect()
    };
    let digest = "b".repeat(64);
    assert!(
        journal.iter().any(|event| event.contains(&digest)),
        "no ER event carries the followed executable digest"
    );
    assert!(
        journal.iter().any(|event| event.contains("credential-1")),
        "no ER event carries the credential-side digest"
    );
}

// Security review, round 2 (story:connection-binding-excludes-operations,
// contracts/cli/v1alpha1/semantics.md "Launching for a description"): the
// registry's launch condition, `Registry::launch_admits`, which both the CLI
// and the owner check before a read command starts the configured adapter.

/// A connection published under `binding` whose credential states no expiry,
/// so the wall clock `launch_admits` reads never makes it expired; its
/// evidence lifetime has passed (`pending`), which the condition admits.
fn security2_connect(registry: &Registry, binding: &Binding, subject: &str) -> (String, String) {
    let acquisition = registry.begin(binding, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let baseline = ValidatedBaseline {
        credential_expires_at_ms: None,
        ..validated(subject, NOW)
    };
    let candidate = registry.prepare(&claim, baseline, 12, NOW).unwrap();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = registry
        .describe(
            &binding.instance_id,
            &binding.adapter_id,
            configured(binding),
            &reference,
            LATER,
            true,
        )
        .unwrap()
        .revision;
    (reference, revision)
}

fn security2_admits(registry: &Registry, configured_as: &Binding, named: Option<&str>) -> bool {
    registry
        .launch_admits(
            &configured_as.instance_id,
            &configured_as.adapter_id,
            configured(configured_as),
            named,
        )
        .unwrap()
}

/// Control: a usable connection admits a launch under a selection-only and an
/// executable-only change, named or not; nothing else in this file is
/// meaningful without it.
#[test]
fn security2_a_current_usable_connection_admits_a_launch() {
    let (_root, registry) = fixture();
    let (reference, _) = security2_connect(&registry, &saved(), "one");
    assert!(security2_admits(&registry, &operations(), None));
    assert!(security2_admits(&registry, &operations(), Some(&reference)));
    assert!(security2_admits(&registry, &executable(), None));
}

/// Invariant 2: revoked, Stale, invalidated (identity refused) and expired
/// connections admit no launch.
#[test]
fn security2_an_unusable_connection_admits_no_launch() {
    let (_root, registry) = fixture();
    let (reference, revision) = security2_connect(&registry, &saved(), "one");
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    assert!(!security2_admits(&registry, &operations(), None), "revoked");
    assert!(
        !security2_admits(&registry, &operations(), Some(&reference)),
        "revoked, named"
    );

    let (_root, registry) = fixture();
    let (reference, revision) = security2_connect(&registry, &saved(), "one");
    refuse(&registry, &saved(), &reference, &revision);
    assert!(!security2_admits(&registry, &operations(), None), "stale");

    let (_root, registry) = fixture();
    let (reference, revision) = security2_connect(&registry, &saved(), "one");
    assert_eq!(
        revalidate(&registry, &saved(), &reference, &revision, "other"),
        Err(Failure::IdentityMismatch)
    );
    assert!(
        !security2_admits(&registry, &operations(), None),
        "identity refused"
    );

    // `connect` records a credential expiry one hour after NOW, which the
    // wall clock has passed.
    let (_root, registry) = fixture();
    connect(&registry, &saved(), "one");
    assert!(!security2_admits(&registry, &operations(), None), "expired");
}

/// Invariant 2: a binding with no credential-side digest under a moved
/// configuration revision, and a CA (credential-side) change, admit no launch;
/// the same legacy binding under its own revision does.
#[test]
fn security2_a_connection_not_current_under_the_entry_admits_no_launch() {
    let (_root, registry) = fixture();
    security2_connect(&registry, &binding(), "one");
    let moved = Binding {
        configuration_revision: "config-2".into(),
        credential_revision: Some("credential-1".into()),
        executable: Some("b".repeat(64)),
        ..binding()
    };
    assert!(!security2_admits(&registry, &moved, None), "legacy, moved");
    assert!(security2_admits(&registry, &binding(), None), "control");

    let (_root, registry) = fixture();
    security2_connect(&registry, &saved(), "one");
    assert!(!security2_admits(&registry, &trust(), None), "trust change");
}

/// Invariant 2: a connection of another instance, named by a client, admits no
/// launch for this one.
#[test]
fn security2_another_instances_connection_admits_no_launch_for_this_one() {
    let (_root, registry) = fixture();
    let other = Binding {
        instance_id: "other-instance".into(),
        ..saved()
    };
    let (reference, _) = security2_connect(&registry, &other, "one");
    assert!(
        security2_admits(&registry, &other, Some(&reference)),
        "control"
    );
    assert!(!security2_admits(
        &registry,
        &operations(),
        Some(&reference)
    ));
    assert!(!security2_admits(&registry, &operations(), None));
}

/// Invariant 4, restated after review (round 2) for the contract as narrowed:
/// a changed provider authority moves the credential-side digest of an
/// implementation that states one (the catalog's covers the API base), and a
/// connection whose digest moved admits no launch. The condition does not
/// compare the authority itself, which an observation does not know.
#[test]
fn security2_a_changed_provider_authority_admits_no_launch() {
    let (_root, registry) = fixture();
    let (reference, _) = security2_connect(&registry, &saved(), "one");
    let moved = Binding {
        configuration_revision: "config-2".into(),
        provider_authority: "https://other.invalid/authority".into(),
        credential_revision: Some("credential-moved".into()),
        ..saved()
    };
    assert_eq!(follow(&saved(), &moved), Follow::Changed);
    assert!(
        !security2_admits(&registry, &moved, Some(&reference)),
        "a connection under another provider authority admits a launch"
    );
}
