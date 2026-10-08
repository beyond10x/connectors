//! A metadata store written by the previous Entity Runtime and Eventlog pins
//! opens, reads and accepts writes under the current ones, and the current
//! pins add nothing to it that the previous ones would refuse.
//!
//! `fixtures/metadata-store-er-0.26.0/` is the store [`seed`] wrote under
//! Entity Runtime 0.26.0 and Eventlog `6983cc25`, the pins connectors 0.31.0
//! released with; its README says how. A fresh store seeded the same way
//! under the current pins is the oracle: the fixture must answer exactly as it.

use connectors_host::local::{
    metadata::{Checkpoints, Metadata},
    registry::{Binding, Purpose, Registry, StaticProfile, Subject},
};
use rusqlite::{Connection, OpenFlags};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

pub const NOW: u64 = 1_788_998_400_000;
pub const NAME: &str = "metadata.sqlite3";
pub const FLOOR: &str = "registry-clock.floor";
pub const REFERENCES: &str = "references";
/// Instances [`seed`] begins an acquisition for; it consumes all but the last.
pub const INSTANCES: [&str; 3] = ["one", "two", "three"];

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/metadata-store-er-0.26.0")
}

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

/// Initializes a store at `state` and begins one acquisition per instance,
/// consuming all but the last. Returns each instance's acquisition reference.
pub fn seed(state: &Path) -> Vec<(String, String)> {
    private_dir(state);
    drop(Metadata::initialize(state).unwrap());
    let registry = Registry::new(state);
    let mut references = Vec::new();
    for (index, instance) in INSTANCES.into_iter().enumerate() {
        let now = NOW + index as u64;
        let acquisition = registry.begin(&binding(instance), now).unwrap();
        references.push((instance.to_owned(), acquisition.reference().to_owned()));
        if index + 1 < INSTANCES.len() {
            registry.consume(acquisition, now).unwrap();
        }
    }
    drop(Metadata::inspect(state).unwrap());
    references
}

/// Copies the single store file `file`, and the clock floor beside it, into
/// the new state directory `to`, as a restore would: WAL mode, a lock file,
/// private permissions, owned by the user running this test. No handle of
/// this process has opened `to` before.
///
/// A store records the uid that created it in `local_authority.owner_uid`, and
/// `Metadata` refuses a store another uid owns as `MetadataUnavailable` before
/// any runtime reads it. The fixture records the uid of the machine that wrote
/// it, so the copy is handed to the uid that owns `to`, which this process
/// created: a user restoring their own store, whatever uid runs the test.
fn restore(file: &Path, floor: &Path, to: &Path) {
    private_dir(to);
    let owner = fs::metadata(to).unwrap().uid();
    let target = to.join(NAME);
    Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .execute("VACUUM INTO ?1", [target.to_str().unwrap()])
        .unwrap();
    let restored = Connection::open(&target).unwrap();
    restored.pragma_update(None, "journal_mode", "WAL").unwrap();
    assert_eq!(
        restored
            .execute(
                "UPDATE local_authority SET owner_uid = ?1 WHERE singleton = 1",
                [owner],
            )
            .unwrap(),
        1,
        "the restored store has no ownership record"
    );
    drop(restored);
    fs::write(to.join("metadata.lock"), b"").unwrap();
    fs::copy(floor, to.join(FLOOR)).unwrap();
    for name in [NAME, "metadata.lock", FLOOR] {
        fs::set_permissions(to.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn fixture_references() -> Vec<(String, String)> {
    fs::read_to_string(fixture().join(REFERENCES))
        .unwrap()
        .lines()
        .map(|line| {
            let (instance, reference) = line.split_once(' ').unwrap();
            (instance.to_owned(), reference.to_owned())
        })
        .collect()
}

type View = Result<(String, Option<String>, Option<String>, u64), String>;

/// What the registry answers about each acquisition, while pending and after
/// every acquisition has expired.
fn views(state: &Path, references: &[(String, String)]) -> Vec<(String, u64, View)> {
    let registry = Registry::new(state);
    let mut views = Vec::new();
    for now in [NOW + 1_000, NOW + 86_400_000] {
        for (instance, reference) in references {
            let view = registry
                .acquisition_status(instance, "fixture-adapter", reference, now)
                .map(|status| {
                    (
                        status.state.to_owned(),
                        status.reason,
                        status.connection,
                        status.expires_at_ms,
                    )
                })
                .map_err(|failure| format!("{failure:?}"));
            views.push((instance.clone(), now, view));
        }
    }
    views
}

/// Every schema object in the store file, by type and name.
fn schema(file: &Path) -> BTreeSet<(String, String)> {
    let connection = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut statement = connection
        .prepare("SELECT type, name FROM sqlite_master")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn event_count(file: &Path) -> i64 {
    Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .query_row("SELECT count(*) FROM connectors_er_events", [], |row| {
            row.get(0)
        })
        .unwrap()
}

/// The previous pins' store opens and every acquisition reads back exactly as
/// it does in a store the current pins wrote through the same steps.
#[test]
fn a_store_the_previous_pins_wrote_reads_as_one_the_current_pins_write() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    restore(&fixture().join(NAME), &fixture().join(FLOOR), &state);
    let oracle = root.path().join("oracle");
    let current = seed(&oracle);

    drop(Metadata::inspect(&state).unwrap());
    let previous = fixture_references();
    assert_eq!(
        previous
            .iter()
            .map(|(instance, _)| instance)
            .collect::<Vec<_>>(),
        INSTANCES.iter().collect::<Vec<_>>()
    );
    let strip = |views: Vec<(String, u64, View)>| {
        views
            .into_iter()
            .map(|(instance, now, view)| {
                let view = view.unwrap_or_else(|failure| {
                    panic!("{instance} at {now} refused: {failure}");
                });
                // Connection references are minted per store; the rest is not.
                (instance, now, view.0, view.1, view.2.is_some(), view.3)
            })
            .collect::<Vec<_>>()
    };
    let read = strip(views(&state, &previous));
    assert_eq!(read, strip(views(&oracle, &current)));
    assert!(read.iter().any(|view| view.2 == "pending"));
    assert!(read.iter().any(|view| view.2 == "failed"));
    assert_eq!(
        event_count(&state.join(NAME)),
        event_count(&oracle.join(NAME))
    );
}

/// A write the current pins make to the previous pins' store lands, a fresh
/// full replay of the result reads it and every earlier acquisition, and the
/// store holds no schema object the previous pins did not write: no Eventlog
/// continuity table, journal or trigger, which Eventlog 0.7.0 and Entity
/// Runtime 0.28.0 and earlier refuse to open.
#[test]
fn a_write_to_a_previous_pins_store_adds_nothing_the_previous_pins_refuse() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    restore(&fixture().join(NAME), &fixture().join(FLOOR), &state);
    let before = event_count(&state.join(NAME));
    let registry = Registry::new(&state);
    let acquisition = registry.begin(&binding("four"), NOW + 10).unwrap();
    let mut references = fixture_references();
    references.push(("four".to_owned(), acquisition.reference().to_owned()));
    registry.consume(acquisition, NOW + 10).unwrap();
    drop(Metadata::inspect(&state).unwrap());
    assert!(event_count(&state.join(NAME)) > before);

    let replayed = root.path().join("replayed");
    restore(&state.join(NAME), &state.join(FLOOR), &replayed);
    let read = views(&replayed, &references);
    assert_eq!(read, views(&state, &references));
    for (instance, now, view) in &read {
        let (state, ..) = view
            .as_ref()
            .unwrap_or_else(|failure| panic!("{instance} at {now} refused: {failure}"));
        let expected = if *now < NOW + 86_400_000 {
            "pending"
        } else {
            "failed"
        };
        assert_eq!(state, expected, "{instance} at {now}");
    }

    let written = schema(&state.join(NAME));
    assert_eq!(written, schema(&fixture().join(NAME)));
    let continuity = written
        .iter()
        .filter(|(kind, name)| {
            kind == "trigger"
                || name.contains("capture_continuity")
                || name.contains("capture_journal")
        })
        .collect::<Vec<_>>();
    assert!(continuity.is_empty(), "{continuity:?}");
}

/// The previous pins' store gets durable open checkpoints only through the
/// explicit enable, which installs Eventlog's continuity and persists a
/// checkpoint: a copy of the enabled store opens from it in a process that
/// never opened the store, and reads as before. A second enable changes
/// nothing.
#[test]
fn the_previous_pins_store_is_enabled_and_then_opens_from_its_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    restore(&fixture().join(NAME), &fixture().join(FLOOR), &state);
    let previous = fixture_references();
    assert_eq!(
        Metadata::enable_checkpoints(&state),
        Ok(Checkpoints::Enabled)
    );
    let written = schema(&state.join(NAME));
    assert!(written.contains(&(
        "table".to_owned(),
        "connectors_er_capture_continuity".to_owned()
    )));
    assert!(written.iter().any(|(kind, _)| kind == "trigger"));
    assert_eq!(
        Metadata::enable_checkpoints(&state),
        Ok(Checkpoints::AlreadyEnabled)
    );
    assert_eq!(schema(&state.join(NAME)), written);

    // This process holds handles on `state`; a byte copy at a new path is
    // opened afresh. `VACUUM INTO` may renumber rows, which a checkpoint
    // rightly does not survive, so the committed pages are moved into the
    // file and the file is copied as it is.
    let copied = root.path().join("copied");
    copy_file(&state, &copied);
    let opened = Metadata::inspect(&copied).unwrap();
    assert!(opened.opened_from_checkpoint());
    drop(opened);
    // Both stores read with the same clock floor, so they answer alike.
    let read = views(&copied, &previous);
    assert!(read.iter().any(|(_, _, view)| view.is_ok()), "{read:?}");
    assert_eq!(read, views(&state, &previous));
}

/// Copies the store in `from` byte for byte into the new state directory `to`,
/// after SQLite has moved every committed page into the database file.
fn copy_file(from: &Path, to: &Path) {
    let connection = Connection::open(from.join(NAME)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let (busy, log): (i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        if busy == 0 && log == 0 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the WAL never emptied"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    drop(connection);
    private_dir(to);
    fs::copy(from.join(NAME), to.join(NAME)).unwrap();
    fs::write(to.join("metadata.lock"), b"").unwrap();
    fs::copy(from.join(FLOOR), to.join(FLOOR)).unwrap();
    for name in [NAME, "metadata.lock", FLOOR] {
        fs::set_permissions(to.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
}
