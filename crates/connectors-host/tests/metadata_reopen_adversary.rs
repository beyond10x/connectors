//! Adversarial cases for reopening a metadata store from an authority this
//! process already verified. A full replay in a fresh process is the oracle:
//! a store opened at a path the process has never opened is replayed in full,
//! exactly as every open did before authorities were kept.

use connectors_host::local::{
    metadata::Metadata,
    registry::{Binding, Purpose, Registry, StaticProfile, Subject},
};
use rusqlite::{Connection, OpenFlags, types::Value};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const NOW: u64 = 1_788_998_400_000;
const NAME: &str = "metadata.sqlite3";

fn binding(instance: &str) -> Binding {
    Binding {
        instance_id: instance.into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "config-1".into(),
        provider_authority: "https://fixture.invalid/fixed-authority".into(),
        profile: StaticProfile {
            id: "fixture-token".into(),
            revision: "profile-1".into(),
            purpose: Purpose::DelegatedUser,
            subject: Subject::User,
            minimum_scopes: BTreeSet::from(["read".to_owned()]),
            evidence_lifetime_ms: 60_000,
        },
    }
}

fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

/// An initialized store at `state` holding records of several subjects.
fn seeded(state: &Path) -> Registry {
    private_dir(state);
    drop(Metadata::initialize(state).unwrap());
    let registry = Registry::new(state);
    for (index, instance) in ["one", "two", "three"].into_iter().enumerate() {
        let acquisition = registry
            .begin(&binding(instance), NOW + index as u64)
            .unwrap();
        registry.consume(acquisition, NOW + index as u64).unwrap();
    }
    registry
}

/// A byte-identical logical copy of the store at `from` into the new
/// directory `to`, which no handle of this process has ever opened.
fn copy_store(from: &Path, to: &Path) {
    private_dir(to);
    let target = to.join(NAME);
    Connection::open_with_flags(from.join(NAME), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .execute("VACUUM INTO ?1", [target.to_str().unwrap()])
        .unwrap();
    Connection::open(&target)
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    fs::write(to.join("metadata.lock"), b"").unwrap();
    for name in [NAME, "metadata.lock"] {
        fs::set_permissions(to.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn events(path: &Path) -> Vec<(i64, String, String)> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT global_seq,stream_type,event_name FROM connectors_er_events ORDER BY global_seq",
        )
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn event_count(path: &Path) -> i64 {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .query_row("SELECT count(*) FROM connectors_er_events", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// A stored event altered in place after this process verified the store is
/// refused on reopen whenever a full replay of the same bytes refuses it.
#[test]
fn a_reopen_refuses_every_altered_event_a_full_replay_refuses() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    drop(seeded(&state));
    let store = state.join(NAME);
    let mut accepted = Vec::new();
    let mut refused_by_replay = 0;
    for (index, (seq, stream_type, name)) in events(&store).into_iter().enumerate() {
        // This process verifies the unaltered store and keeps its authority.
        drop(Metadata::inspect(&state).unwrap());
        let physical = Connection::open(&store).unwrap();
        let original: Value = physical
            .query_row(
                "SELECT data FROM connectors_er_events WHERE global_seq=?1",
                [seq],
                |row| row.get(0),
            )
            .unwrap();
        physical
            .execute(
                "UPDATE connectors_er_events SET data='{}' WHERE global_seq=?1",
                [seq],
            )
            .unwrap();
        let fresh = root.path().join(format!("fresh-{index}"));
        copy_store(&state, &fresh);
        let replayed = Metadata::inspect(&fresh).map(drop);
        let reopened = Metadata::inspect(&state).map(drop);
        if replayed.is_err() {
            refused_by_replay += 1;
            if reopened.is_ok() {
                accepted.push(format!("{seq} {stream_type} {name}"));
            }
        }
        physical
            .execute(
                "UPDATE connectors_er_events SET data=?1 WHERE global_seq=?2",
                rusqlite::params![original, seq],
            )
            .unwrap();
        drop(physical);
        fs::remove_dir_all(&fresh).unwrap();
    }
    assert!(
        refused_by_replay > 1,
        "a full replay refused {refused_by_replay} alterations"
    );
    assert!(
        accepted.is_empty(),
        "a reopen accepted {} of {refused_by_replay} alterations a full replay refuses: {accepted:#?}",
        accepted.len()
    );
}

/// The same for a stored blob whose bytes are replaced by as many zero bytes.
#[test]
fn a_reopen_refuses_every_altered_blob_a_full_replay_refuses() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    drop(seeded(&state));
    let store = state.join(NAME);
    let rows: Vec<i64> = {
        let connection =
            Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut statement = connection
            .prepare("SELECT rowid FROM connectors_er_blobs ORDER BY rowid")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    assert!(!rows.is_empty());
    let mut accepted = Vec::new();
    let mut refused_by_replay = 0;
    for (index, rowid) in rows.into_iter().enumerate() {
        drop(Metadata::inspect(&state).unwrap());
        let physical = Connection::open(&store).unwrap();
        let original: Value = physical
            .query_row(
                "SELECT bytes FROM connectors_er_blobs WHERE rowid=?1",
                [rowid],
                |row| row.get(0),
            )
            .unwrap();
        physical
            .execute(
                "UPDATE connectors_er_blobs SET bytes=zeroblob(length(bytes)) WHERE rowid=?1",
                [rowid],
            )
            .unwrap();
        let fresh = root.path().join(format!("fresh-{index}"));
        copy_store(&state, &fresh);
        let replayed = Metadata::inspect(&fresh).map(drop);
        let reopened = Metadata::inspect(&state).map(drop);
        if replayed.is_err() {
            refused_by_replay += 1;
            if reopened.is_ok() {
                accepted.push(rowid);
            }
        }
        physical
            .execute(
                "UPDATE connectors_er_blobs SET bytes=?1 WHERE rowid=?2",
                rusqlite::params![original, rowid],
            )
            .unwrap();
        drop(physical);
        fs::remove_dir_all(&fresh).unwrap();
    }
    assert!(
        refused_by_replay > 1,
        "a full replay refused {refused_by_replay} alterations"
    );
    assert!(
        accepted.is_empty(),
        "a reopen accepted {} of {refused_by_replay} blob alterations a full replay refuses: {accepted:?}",
        accepted.len()
    );
}

/// The state directory is replaced by a restored copy while this process
/// holds an authority for the old one: the next write lands in the store
/// that is now at the path, as it does in a process that never opened it.
#[test]
fn a_write_after_the_store_is_replaced_at_its_path_lands_in_the_new_store() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let registry = seeded(&state);
    drop(Metadata::inspect(&state).unwrap());
    let backup = root.path().join("backup");
    copy_store(&state, &backup);
    // The live store moves on after the backup was taken.
    let acquisition = registry.begin(&binding("four"), NOW + 10).unwrap();
    registry.consume(acquisition, NOW + 10).unwrap();
    drop(Metadata::inspect(&state).unwrap());
    // Restore: the old state directory is moved aside, the backup takes its place.
    let aside: PathBuf = root.path().join("state.old");
    fs::rename(&state, &aside).unwrap();
    fs::rename(&backup, &state).unwrap();
    let restored = event_count(&state.join(NAME));
    let old = event_count(&aside.join(NAME));
    assert!(old > restored);

    let registry = Registry::new(&state);
    let acquisition = registry.begin(&binding("five"), NOW + 20).unwrap();
    registry.consume(acquisition, NOW + 20).unwrap();
    assert_eq!(
        event_count(&aside.join(NAME)),
        old,
        "the write went to the store that was moved away from the path"
    );
    assert!(
        event_count(&state.join(NAME)) > restored,
        "the store at the path did not receive the write"
    );
}

/// Two handles write the same store at once, each through its own authority,
/// both advancing the shared registry clock. Afterwards every acquisition reads
/// back at the path exactly as a full replay of the same bytes reads it.
#[test]
fn concurrent_writers_leave_a_reopen_equal_to_a_full_replay() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    drop(seeded(&state));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writers = (0..2)
        .map(|writer| {
            let state = state.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let registry = Registry::new(&state);
                barrier.wait();
                let mut references = Vec::new();
                for round in 0..8u64 {
                    let instance = format!("w{writer}-{round}");
                    let now = NOW + 100 + round;
                    let Ok(acquisition) = registry.begin(&binding(&instance), now) else {
                        continue;
                    };
                    references.push((instance, acquisition.reference().to_owned()));
                    if round % 2 == 0 {
                        let _ = registry.consume(acquisition, now);
                    }
                }
                references
            })
        })
        .collect::<Vec<_>>();
    let references = writers
        .into_iter()
        .flat_map(|writer| writer.join().unwrap())
        .collect::<Vec<_>>();
    assert!(
        references.len() > 4,
        "{} acquisitions began",
        references.len()
    );
    let fresh = root.path().join("fresh");
    copy_store(&state, &fresh);
    let held = Registry::new(&state);
    let replayed = Registry::new(&fresh);
    let now = NOW + 1_000;
    let view = |registry: &Registry, instance: &str, reference: &str| {
        registry
            .acquisition_status(instance, "fixture-adapter", reference, now)
            .map(|status| {
                (
                    status.state,
                    status.reason,
                    status.connection,
                    status.expires_at_ms,
                )
            })
    };
    for (instance, reference) in &references {
        assert_eq!(
            view(&held, instance, reference),
            view(&replayed, instance, reference),
            "{instance}"
        );
    }
}
