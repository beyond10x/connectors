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
