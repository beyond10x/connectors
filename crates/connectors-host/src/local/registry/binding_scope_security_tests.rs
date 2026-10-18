//! Security review of story:connection-binding-excludes-operations
//! (architecture-decision-record:provider-definitions-and-runtimes D12) at the
//! registry: integrity conformance checks of the binding's credential side,
//! its executable digest and their record across a full metadata replay.
use super::*;
use rusqlite::{Connection, OpenFlags};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const FLOOR: &str = "registry-clock.floor";
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

/// Invariant 5: an approval target never follows an executable-only change
/// that no validation under the new executable confirmed.
#[test]
fn security_an_approval_target_refuses_an_unvalidated_executable_change() {
    let (_root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved(), "one");
    let target = registry.approval_target(&executable(), &reference, &BTreeSet::new());
    assert!(
        matches!(
            target,
            Err(Failure::UpgradeRequired | Failure::BindingChanged)
        ),
        "an approval target admitted an executable-only change: {target:?} (revision {revision})"
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
