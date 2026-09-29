//! Security-review conformance cases for story:metadata-open-is-not-a-full-replay.
//!
//! The unit keeps a verified Entity Runtime authority per process and, on the
//! next open of the same store, reads only what was appended since. These cases
//! hold that reuse to the contract the full replay kept at base: whatever a
//! fresh open (a complete, digest-verified replay) refuses, an open served from
//! the held authority refuses too, and what a held authority accounts for after
//! catching up equals what a fresh replay reads.
use super::Failure;
use super::metadata::{Metadata, full_replays, simulate_process};
use rusqlite::params;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn store() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    super::filesystem::directory(root.path(), false, true).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    root
}

/// Records one runtime subject per persist, so the store holds several
/// subjects, each with its own recorded entries.
fn seed(root: &Path, instances: &[&str]) {
    for instance in instances {
        let mut metadata = Metadata::update(root, true).unwrap();
        metadata
            .connection
            .execute(
                "INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES (?1,0)",
                params![instance],
            )
            .unwrap();
        metadata.persist_runtime_state().unwrap();
    }
}

fn flip(root: &Path, instance: &str) {
    let mut metadata = Metadata::update(root, true).unwrap();
    metadata
        .connection
        .execute(
            "UPDATE local_runtime_instances SET suppressed=1-suppressed WHERE instance_id=?1",
            params![instance],
        )
        .unwrap();
    metadata.persist_runtime_state().unwrap();
}

fn physical(root: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(root.join("metadata.sqlite3")).unwrap()
}

fn events(root: &Path) -> Vec<(i64, String, String)> {
    let connection = physical(root);
    let mut statement = connection
        .prepare("SELECT global_seq,event_name,data FROM connectors_er_events ORDER BY global_seq")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

/// Every recorded event, altered in place one at a time: an open served from
/// the process's held authority must refuse exactly when a fresh open does.
/// At base every open was a fresh open, so this held by construction.
#[test]
fn a_held_authority_refuses_every_in_place_alteration_a_fresh_open_refuses() {
    let root = store();
    seed(root.path(), &["sec-a", "sec-b", "sec-c", "sec-d"]);
    flip(root.path(), "sec-c");
    let owner = 0x5ec_0001_u64;
    let mut fresh = 0x5ec_1000_u64;
    let mut admitted = Vec::new();
    let mut checked = 0;
    for (position, name, original) in events(root.path()) {
        simulate_process(owner);
        // The owner holds a verified authority for this store, and an
        // unaltered reopen is served from it without a full replay.
        drop(Metadata::inspect(root.path()).unwrap());
        let control = full_replays();
        drop(Metadata::inspect(root.path()).unwrap());
        if full_replays() != control {
            continue;
        }
        assert_eq!(
            physical(root.path())
                .execute(
                    "UPDATE connectors_er_events SET data='{}' WHERE global_seq=?1",
                    [position],
                )
                .unwrap(),
            1
        );
        let before = full_replays();
        let held = Metadata::inspect(root.path()).map(drop);
        let replayed = full_replays() - before;
        fresh += 1;
        simulate_process(fresh);
        let replay = Metadata::inspect(root.path()).map(drop);
        simulate_process(owner);
        physical(root.path())
            .execute(
                "UPDATE connectors_er_events SET data=?2 WHERE global_seq=?1",
                params![position, original],
            )
            .unwrap();
        // Another test's handle may have evicted the owner's from the shared
        // pool; then the owner's open was itself a fresh replay and says nothing.
        if replayed != 0 {
            continue;
        }
        checked += 1;
        if replay == Err(Failure::MetadataUnavailable) && held.is_ok() {
            admitted.push(format!("{position}:{name}"));
        }
    }
    assert!(checked > 0, "no open was served from a held authority");
    assert!(
        admitted.is_empty(),
        "a held authority admitted {} of {checked} in-place alterations a fresh open refuses: {admitted:?}",
        admitted.len()
    );
}

/// Another process writes one subject; the owner, holding an older authority,
/// then writes that subject and another in one batch. What the owner accounts
/// for afterwards equals a complete replay in a fresh process.
#[test]
fn an_owner_batch_over_a_subject_another_process_wrote_equals_a_fresh_replay() {
    let root = store();
    seed(root.path(), &["cu-a", "cu-b", "cu-c"]);
    let owner = 0x5ec_2001_u64;
    simulate_process(owner);
    drop(Metadata::inspect(root.path()).unwrap());
    simulate_process(0x5ec_2002);
    flip(root.path(), "cu-a");
    simulate_process(owner);
    let mut metadata = Metadata::update(root.path(), true).unwrap();
    metadata
        .connection
        .execute(
            "UPDATE local_runtime_instances SET suppressed=1-suppressed WHERE instance_id IN ('cu-a','cu-b')",
            [],
        )
        .unwrap();
    metadata.persist_runtime_state().unwrap();
    let held = metadata.recorded_rows();
    drop(metadata);
    // The owner's held handle keeps its sidecars open past every lifecycle
    // lock, as a CLI that exits holding one leaves them; they stay private
    // and a fresh process admits them and reads the committed batch.
    for suffix in ["-wal", "-shm"] {
        let sidecar = root.path().join(format!("metadata.sqlite3{suffix}"));
        let mode = std::fs::metadata(&sidecar).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{suffix}");
    }
    simulate_process(0x5ec_2003);
    let replayed = Metadata::inspect(root.path()).unwrap().recorded_rows();
    assert!(!held.is_empty());
    assert_eq!(held, replayed);
}
