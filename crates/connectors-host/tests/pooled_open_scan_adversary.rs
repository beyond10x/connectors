//! Adversary cases for story:metadata-invoke-cost-flat-in-store-size (wave
//! 20261007c, U2): "a process holding a verified handle on the same store file
//! skips PRAGMA quick_check and the foreign-key scan on later opens".
//!
//! This test binary is one real OS process, as the owner is: the first open
//! leaves its verified Entity Runtime handle in the process pool, so every
//! later open of the same file in this process is a pooled open. Between the
//! two opens the durable file is damaged outside SQLite. Every open before
//! this unit ran `PRAGMA quick_check(1)` and refused a file failing it as
//! `MetadataUnavailable`.
use connectors_host::local::{Failure, metadata::Metadata};
use rusqlite::Connection;
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const NAME: &str = "metadata.sqlite3";

fn store() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    fs::create_dir_all(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    // A verified open whose handle stays in this process's pool.
    drop(Metadata::inspect(&state).unwrap());
    // Control: a second, pooled, open of the unchanged store answers.
    drop(Metadata::inspect(&state).unwrap());
    (root, state)
}

/// Keeps every sidecar private, so a refusal below is never the sidecar
/// admission refusing a file this test's connection created.
fn private_sidecars(state: &Path) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let path = state.join(format!("{NAME}{suffix}"));
        if path.exists() {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

fn quick_check(file: &Path) -> String {
    let Ok(connection) = Connection::open(file) else {
        return "unopenable".into();
    };
    connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0))
        .unwrap_or_else(|error| error.to_string())
}

/// Moves every committed page into the file; returns the page size and every
/// b-tree (name, root page) but those the host reads on every open.
fn checkpointed_btrees(file: &Path) -> (u64, Vec<(String, u64)>) {
    let other = Connection::open(file).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (busy, log): (i64, i64) = other
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
    let page_size: i64 = other
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap();
    let mut statement = other
        .prepare(
            "SELECT name, rootpage FROM sqlite_schema WHERE rootpage > 1 \
             AND tbl_name NOT IN ('local_authority','schema_migrations','connectors_er_authority') \
             ORDER BY name",
        )
        .unwrap();
    let btrees = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .unwrap()
        .map(|row| {
            let (name, root) = row.unwrap();
            (name, root as u64)
        })
        .collect();
    (page_size as u64, btrees)
}

/// A byte copy of the store in `from` in the new state directory `to`: a file
/// no handle of this process has opened.
fn copied(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    fs::set_permissions(to, fs::Permissions::from_mode(0o700)).unwrap();
    fs::copy(from.join(NAME), to.join(NAME)).unwrap();
    fs::write(to.join("metadata.lock"), b"").unwrap();
    for name in [NAME, "metadata.lock"] {
        fs::set_permissions(to.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

/// For each b-tree of the durable file in turn, on its own fresh store: its
/// root page's type byte is overwritten with 0x00 (no b-tree page type),
/// bypassing SQLite, while this process holds its verified handle. The file
/// then fails `PRAGMA quick_check`.
///
/// Today's behaviour, both ways (story:metadata-invoke-cost-flat-in-store-size):
/// a pooled open in the process that verified the store does not repeat the
/// page scan and accepts a file damaged in a b-tree its catch-up does not read
/// (every one outside the `connectors_er_` tables), as a provider-tracked handle accepts
/// a raw edit made while it is held; a fresh open of the same bytes runs the
/// scan and refuses them as `MetadataUnavailable`.
#[test]
fn a_damaged_page_is_accepted_by_a_pooled_open_and_refused_by_a_fresh_one() {
    let (_probe_root, probe) = store();
    let (_, btrees) = checkpointed_btrees(&probe.join(NAME));
    assert!(!btrees.is_empty());
    let mut pooled_refused = Vec::new();
    let mut fresh_accepted = Vec::new();
    let mut pooled_accepted = 0;
    for (name, _) in &btrees {
        let (root, state) = store();
        let file = state.join(NAME);
        let (page_size, here) = checkpointed_btrees(&file);
        let root_page = here.iter().find(|(n, _)| n == name).unwrap().1;
        let mut handle = fs::OpenOptions::new().write(true).open(&file).unwrap();
        handle
            .seek(SeekFrom::Start((root_page - 1) * page_size))
            .unwrap();
        handle.write_all(&[0x00]).unwrap();
        handle.sync_all().unwrap();
        drop(handle);
        private_sidecars(&state);
        let check = quick_check(&file);
        assert_ne!(check, "ok", "damaging {name} fails quick_check");
        // The pooled open still reads the Entity Runtime tables it catches up
        // from, so damage there can be refused; the rest is never read by it.
        let read_by_catch_up = name.starts_with("connectors_er_")
            || name.starts_with("sqlite_autoindex_connectors_er_");
        if !read_by_catch_up {
            if Metadata::inspect(&state).is_err() {
                pooled_refused.push(name.clone());
            } else {
                pooled_accepted += 1;
            }
        }
        let fresh = root.path().join("fresh");
        copied(&state, &fresh);
        if Metadata::inspect(&fresh).map(drop) != Err(Failure::MetadataUnavailable) {
            fresh_accepted.push(format!("{name} (quick_check: {check})"));
        }
    }
    assert!(
        fresh_accepted.is_empty(),
        "a fresh open accepted a store failing PRAGMA quick_check, damaged at: {fresh_accepted:#?}"
    );
    assert!(
        pooled_accepted > 0,
        "no b-tree outside the catch-up's tables was damaged"
    );
    assert!(
        pooled_refused.is_empty(),
        "story:metadata-invoke-cost-flat-in-store-size: a pooled open skips the page scan and \
         accepts these damaged stores today; it refused {pooled_refused:#?}. If the pooled \
         shortcut was removed, restore the refusal this case asserted before: every open \
         refuses a store failing PRAGMA quick_check as MetadataUnavailable."
    );
}
