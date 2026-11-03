//! Durable open checkpoints (Entity Runtime 0.29.0) on the local metadata
//! store: a store `setup init` creates has them, and an open from one never
//! answers before the whole store was verified, so a raw edit of the file made
//! while no handle was open is refused before the first command after it
//! (`decision-blocker:checkpoint-offline-edit-detection`, option B).
use super::{Checkpoints, Metadata, NAME, er, exit_process, simulate_process};
use crate::local::{Failure, filesystem as fs};
use entity_eventlog::OpenVerification;
use rusqlite::{Connection, OpenFlags, params};
use std::{
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn store() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    fs::directory(&path, true, true).unwrap();
    (root, path)
}

fn continuity(file: &Path) -> bool {
    let connection = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    er::checkpoints_enabled(&connection).unwrap()
}

/// One recorded business write in process `process`, which then exits.
fn write_in(path: &Path, process: u64, round: u64) {
    simulate_process(process);
    let mut metadata = Metadata::update(path, true).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES (?1,0)",
            params![format!("probe-{process}-{round}")],
        )
        .unwrap();
    metadata.persist_runtime_state().unwrap();
    drop(metadata);
    exit_process();
}

/// Moves every committed page into the database file through SQLite, which
/// changes no row, then alters bytes of the oldest recorded blob in the file
/// itself, bypassing SQLite: the edit no SQLite connection can observe.
fn edit_oldest_blob(file: &Path) {
    let connection = Connection::open(file).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (busy, log): (i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        if busy == 0 && log == 0 {
            break;
        }
        assert!(Instant::now() < deadline, "the WAL never emptied");
        std::thread::sleep(Duration::from_millis(20));
    }
    let blob: Vec<u8> = connection
        .query_row(
            "SELECT bytes FROM connectors_er_blobs ORDER BY rowid LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection.close().unwrap();
    assert!(blob.len() >= 32, "a blob of {} bytes", blob.len());
    let window = &blob[..32];
    let bytes = std::fs::read(file).unwrap();
    let found = bytes
        .windows(window.len())
        .enumerate()
        .filter(|(_, candidate)| *candidate == window)
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    assert!(!found.is_empty(), "the blob's bytes are not in the file");
    let mut handle = std::fs::OpenOptions::new().write(true).open(file).unwrap();
    // Every copy: a page SQLite freed may hold the same bytes as the live one.
    for start in found {
        let offset = start + 16;
        handle.seek(SeekFrom::Start(offset as u64)).unwrap();
        handle.write_all(&[bytes[offset] ^ 0x01]).unwrap();
    }
    handle.sync_all().unwrap();
}

/// A store `setup init` creates has durable open checkpoints, and once a
/// handle has persisted its observation, the next process opens from it.
#[test]
fn a_store_setup_init_creates_has_durable_open_checkpoints() {
    let (_root, path) = store();
    simulate_process(10);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    assert!(continuity(&path.join(NAME)), "no continuity table");
    write_in(&path, 11, 0);
    simulate_process(12);
    let opened = Metadata::inspect(&path).unwrap();
    assert!(opened.opened_from_checkpoint());
    drop(opened);
    exit_process();
    // The command that enables an existing store answers that it is enabled.
    simulate_process(13);
    assert_eq!(
        Metadata::enable_checkpoints(&path),
        Ok(Checkpoints::AlreadyEnabled)
    );
    exit_process();
}

/// A raw edit made while no handle is open is outside what a checkpoint open
/// detects; the first command after it, run directly, refuses the store.
#[test]
fn an_offline_raw_edit_is_refused_by_the_first_direct_command() {
    let (_root, path) = store();
    simulate_process(20);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    for round in 0..3 {
        write_in(&path, 21, round);
    }
    let file = path.join(NAME);
    assert_ne!(er::provider_open(&file), Ok(OpenVerification::Complete));
    edit_oldest_blob(&file);
    // The hazard: the provider's checkpoint open alone accepts the edit.
    assert!(er::provider_open(&file).is_ok());
    simulate_process(22);
    assert_eq!(
        Metadata::inspect(&path).map(drop),
        Err(Failure::MetadataUnavailable)
    );
    exit_process();
    simulate_process(23);
    assert_eq!(
        Metadata::update(&path, false).map(drop),
        Err(Failure::MetadataUnavailable)
    );
    exit_process();
}

/// The owner verifies the whole store when it starts: an edit made while it
/// was stopped is refused at its start, before it serves anything.
#[test]
fn an_offline_raw_edit_is_refused_when_the_owner_starts() {
    let (_root, path) = store();
    simulate_process(30);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    // The owner runs, serves writes from itself and from a command, stops.
    simulate_process(31);
    drop(Metadata::start_owner(&path).unwrap());
    write_in(&path, 32, 0);
    simulate_process(31);
    drop(Metadata::start_owner(&path).unwrap());
    exit_process();
    let file = path.join(NAME);
    assert_ne!(er::provider_open(&file), Ok(OpenVerification::Complete));
    edit_oldest_blob(&file);
    assert!(er::provider_open(&file).is_ok());
    simulate_process(31);
    assert_eq!(
        Metadata::start_owner(&path).map(drop),
        Err(Failure::MetadataUnavailable)
    );
    exit_process();
}

/// A fresh open verifies the store once: one provider open and one complete
/// read. A reopen in the process that verified it, the owner's included,
/// verifies nothing again (story:cold-owner-start-verifies-once).
#[test]
fn a_fresh_open_verifies_once_and_a_reopen_in_its_process_not_again() {
    let (_root, path) = store();
    simulate_process(50);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    write_in(&path, 51, 0);
    let counts = || (super::fresh_opens(), super::full_replays());
    for process in [52, 53] {
        simulate_process(process);
        let before = counts();
        drop(Metadata::start_owner(&path).unwrap());
        let started = counts();
        assert_eq!(
            (started.0 - before.0, started.1 - before.1),
            (1, 1),
            "an owner start in process {process}"
        );
        drop(Metadata::inspect(&path).unwrap());
        drop(Metadata::update(&path, false).unwrap());
        assert_eq!(counts(), started, "a reopen in process {process}");
        exit_process();
    }
}

/// An open while another thread of the same process uses its verified handle
/// waits for that handle instead of verifying the store again, as the owner's
/// background recovery and a request's admission did when they met
/// (story:cold-owner-start-verifies-once); the thread using a handle opens
/// another afresh at once, never waiting for itself.
#[test]
fn an_open_waits_for_the_handle_another_thread_uses_instead_of_verifying_again() {
    let (_root, path) = store();
    simulate_process(60);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    write_in(&path, 61, 0);
    simulate_process(62);
    drop(Metadata::start_owner(&path).unwrap());
    // As long as a large store's verification: the bound of the wait.
    let bound = Duration::from_secs(20);
    er::assume_open_took(&path.join(NAME), bound);
    let held = Metadata::inspect(&path).unwrap();
    let other = {
        let path = path.clone();
        std::thread::spawn(move || {
            simulate_process(62);
            let started = Instant::now();
            drop(Metadata::inspect(&path).unwrap());
            (er::fresh_opens(), started.elapsed())
        })
    };
    let used = Duration::from_millis(300);
    std::thread::sleep(used);
    drop(held);
    let (opens, waited) = other.join().unwrap();
    assert_eq!(opens, 0, "the other thread verified the store again");
    assert!(
        waited >= used && waited < bound,
        "the other thread waited {waited:?}"
    );

    let held = Metadata::inspect(&path).unwrap();
    let before = er::fresh_opens();
    let started = Instant::now();
    drop(Metadata::inspect(&path).unwrap());
    assert_eq!(
        er::fresh_opens() - before,
        1,
        "a nested open of the same thread"
    );
    assert!(
        started.elapsed() < bound,
        "the thread waited for its own handle"
    );
    drop(held);
    exit_process();
}

/// An inspection never takes the lifecycle lock again once it released it, so
/// a locked open that meets the handle an inspection uses waits for it instead
/// of verifying the store again: the owner's maintenance inspects the store
/// every few seconds (`mutations.rs`), and a read's locked opens in the owner
/// (its read-use grant, its audit write) met that hold and verified a 200 MB
/// store a second time (story:cold-owner-start-verifies-once).
#[test]
fn a_locked_open_waits_for_the_handle_an_inspection_uses_instead_of_verifying_again() {
    let (_root, path) = store();
    simulate_process(90);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    write_in(&path, 91, 0);
    simulate_process(92);
    drop(Metadata::start_owner(&path).unwrap());
    // As long as a large store's verification: the bound of the wait.
    let bound = Duration::from_secs(20);
    er::assume_open_took(&path.join(NAME), bound);
    let held = Metadata::inspect(&path).unwrap();
    let locked = {
        let path = path.clone();
        std::thread::spawn(move || {
            simulate_process(92);
            let started = Instant::now();
            let opened = Metadata::update(&path, false).map(drop);
            (opened, er::fresh_opens(), started.elapsed())
        })
    };
    let used = Duration::from_millis(300);
    std::thread::sleep(used);
    drop(held);
    let (opened, opens, waited) = locked.join().unwrap();
    exit_process();
    assert_eq!(opened, Ok(()), "the locked open");
    assert_eq!(opens, 0, "the locked open verified the store again");
    assert!(
        waited >= used && waited < bound,
        "the locked open waited {waited:?}"
    );
}

/// Adversary, D417 U1 (story:cold-owner-start-verifies-once). An unlocked
/// observation (`Registry::transaction_observation`, a read on one owner
/// request thread) replays with the process's verified handle after releasing
/// the lifecycle lock, then relocks. A locked open on another request thread of
/// the same process takes the free lock and, in `take_or_wait`, waits for the
/// handle the observation holds; the observation cannot return it before it
/// relocks, and it cannot relock while the locked open holds the lock. Before
/// the wait the locked open verified afresh at once and the observation
/// relocked after it; now one of the two waits out its bound.
#[test]
fn a_locked_open_never_waits_for_a_handle_whose_user_waits_for_its_lock() {
    let (_root, path) = store();
    simulate_process(80);
    drop(Metadata::initialize(&path).unwrap());
    exit_process();
    write_in(&path, 81, 0);
    simulate_process(82);
    drop(Metadata::start_owner(&path).unwrap());
    // A large store's last fresh open: the bound of the locked open's wait.
    er::assume_open_took(&path.join(NAME), Duration::from_secs(10));
    let lock_wait = Duration::from_secs(3);
    super::set_lifecycle_lock_wait(lock_wait);
    // The observation replayed with the verified handle and released the lock.
    let mut observation = Metadata::update_observation(&path).unwrap();
    let locked = {
        let path = path.clone();
        std::thread::spawn(move || {
            simulate_process(82);
            super::set_lifecycle_lock_wait(lock_wait);
            let started = Instant::now();
            let opened = Metadata::update(&path, false).map(drop);
            (opened, started.elapsed())
        })
    };
    // The locked open has taken the lock and is in its open.
    std::thread::sleep(Duration::from_secs(1));
    let relocked = observation.relock_observation();
    drop(observation);
    let (opened, took) = locked.join().unwrap();
    exit_process();
    assert_eq!(opened, Ok(()), "the locked open");
    assert_eq!(
        relocked,
        Ok(()),
        "the observation could not relock: the locked open held the lock while it waited for the observation's handle"
    );
    assert!(
        took < lock_wait,
        "the locked open took {took:?}, waiting for a handle that could not return"
    );
}

/// The authority a CLI names in its owner greeting is read without replaying
/// the store, and is the store's authority; a missing store has none.
#[test]
fn the_greeting_authority_is_read_without_a_replay() {
    let (_root, path) = store();
    simulate_process(40);
    drop(Metadata::initialize(&path).unwrap());
    let expected = Metadata::inspect(&path).unwrap().authority().unwrap();
    exit_process();
    simulate_process(41);
    let replays = super::full_replays();
    assert_eq!(Metadata::authority_at(&path), Ok(expected));
    assert_eq!(super::full_replays(), replays);
    exit_process();
    let (_empty, missing) = store();
    assert_eq!(
        Metadata::authority_at(&missing),
        Err(Failure::MetadataUnavailable)
    );
}

/// Integrity conformance for the wait an open makes for a handle another
/// thread of its process uses (security review of
/// story:cold-owner-start-verifies-once): however long the store's last fresh
/// open took, the wait ends at the caller's own metadata deadline, and an open
/// of a different store never waits for that handle.
#[test]
fn a_wait_for_a_used_handle_ends_at_the_callers_deadline_and_never_spans_stores() {
    let (_root, path) = store();
    let (_other_root, other) = store();
    simulate_process(91_701);
    drop(Metadata::initialize(&path).unwrap());
    drop(Metadata::initialize(&other).unwrap());
    exit_process();
    write_in(&path, 91_702, 0);
    write_in(&other, 91_703, 0);
    simulate_process(91_704);
    drop(Metadata::start_owner(&path).unwrap());
    let bound = Duration::from_secs(20);
    er::assume_open_took(&path.join(NAME), bound);
    er::assume_open_took(&other.join(NAME), bound);
    let held = Metadata::inspect(&path).unwrap();

    let deadline = Duration::from_millis(500);
    let waiter = {
        let path = path.clone();
        std::thread::spawn(move || {
            simulate_process(91_704);
            let started = Instant::now();
            let opened = super::deadline::within(Instant::now() + deadline, || {
                Metadata::inspect(&path).map(drop)
            });
            (opened, started.elapsed())
        })
    };
    let across = {
        let other = other.clone();
        std::thread::spawn(move || {
            simulate_process(91_704);
            let started = Instant::now();
            let opened = Metadata::inspect(&other).map(drop);
            (opened, er::fresh_opens(), started.elapsed())
        })
    };
    let (across_opened, across_opens, across_waited) = across.join().unwrap();
    let (opened, waited) = waiter.join().unwrap();
    drop(held);
    exit_process();
    assert!(
        waited < deadline + Duration::from_secs(2),
        "an open waited {waited:?} for a used handle past its {deadline:?} deadline ({opened:?})"
    );
    assert_eq!(across_opened, Ok(()));
    assert_eq!(across_opens, 1, "the other store was not opened afresh");
    assert!(
        across_waited < Duration::from_secs(10),
        "an open of another store waited {across_waited:?} for this store's handle"
    );
}
