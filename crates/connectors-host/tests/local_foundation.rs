use connectors_host::local::{
    Failure,
    config::{Config, Paths},
    filesystem,
    metadata::Metadata,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("local-foundation-")
        .tempdir()
        .unwrap()
}

fn paths(root: &tempfile::TempDir) -> Paths {
    Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap()
}

#[test]
fn exclusive_setup_persists_private_metadata_and_refuses_overwrite() {
    let root = root();
    let paths = paths(&root);
    Config::initialize(&paths).unwrap();
    assert!(Config::load(&paths.config).unwrap().adapters.is_empty());
    Metadata::inspect(&paths.state).unwrap();
    let bytes = fs::read(&paths.config).unwrap();
    assert_eq!(
        Config::initialize(&paths),
        Err(Failure::ConfigurationExists)
    );
    assert_eq!(fs::read(&paths.config).unwrap(), bytes);
    assert_eq!(
        fs::metadata(&paths.config).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&paths.state).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let db = rusqlite::Connection::open(paths.state.join("metadata.sqlite3")).unwrap();
    let mode: String = db
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    let authority: String = db
        .query_row("SELECT authority_id FROM local_authority", [], |r| r.get(0))
        .unwrap();
    drop(db);
    drop(Metadata::initialize(&paths.state).unwrap());
    let db = rusqlite::Connection::open(paths.state.join("metadata.sqlite3")).unwrap();
    let after: String = db
        .query_row("SELECT authority_id FROM local_authority", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, authority);
}

#[test]
fn unavailable_or_unrecognized_metadata_is_never_reinitialized() {
    let root = root();
    let paths = paths(&root);
    assert!(matches!(
        Metadata::inspect(&paths.state),
        Err(Failure::MetadataUnavailable)
    ));
    assert!(!paths.state.exists());
    Config::initialize(&paths).unwrap();
    let db = rusqlite::Connection::open(paths.state.join("metadata.sqlite3")).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    assert!(matches!(
        Metadata::initialize(&paths.state),
        Err(Failure::MetadataUnavailable)
    ));
    assert!(matches!(
        Metadata::inspect(&paths.state),
        Err(Failure::MetadataUnavailable)
    ));
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

#[test]
fn concurrent_initialization_has_one_authority_and_one_configuration() {
    for _ in 0..16 {
        concurrent_initialization_round();
    }
}

fn concurrent_initialization_round() {
    let root = root();
    let paths = paths(&root);
    filesystem::directory(paths.config.parent().unwrap(), true, true).unwrap();
    filesystem::directory(&paths.state, true, true).unwrap();
    // Exercise independently opened database handles, as independent CLI owners do.
    let barrier = Arc::new(Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let barrier = barrier.clone();
            let config = paths.config.clone();
            let state = paths.state.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let started = Instant::now();
                (
                    Config::initialize(&Paths { config, state }),
                    started.elapsed(),
                )
            })
        })
        .collect();
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(
        results.iter().filter(|(r, _)| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(
        results.iter().all(|(r, elapsed)| r.is_ok()
            || *r == Err(Failure::ConfigurationExists)
            || (*r == Err(Failure::MetadataUnavailable) && *elapsed >= Duration::from_secs(2))),
        "{results:?}"
    );
    Metadata::inspect(&paths.state).unwrap();
    // Setup's bounded lock can expire under storage/scheduling contention.
    // Once the winner is done, every explicit retry must observe its config;
    // an early unavailable response remains a failure above.
    for (result, _) in results {
        if result.is_err() {
            assert_eq!(
                Config::initialize(&paths),
                Err(Failure::ConfigurationExists)
            );
        }
    }
}

#[test]
fn contending_metadata_open_refuses_at_its_bound_and_recovers_after_release() {
    let root = root();
    let paths = paths(&root);
    Config::initialize(&paths).unwrap();
    let configuration = fs::read(&paths.config).unwrap();
    let held = Metadata::inspect(&paths.state).unwrap();
    let state = paths.state.clone();
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let started = Instant::now();
        send.send((Metadata::initialize(&state).map(|_| ()), started.elapsed()))
            .unwrap();
    });
    let (result, elapsed) = receive.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(result, Err(Failure::MetadataUnavailable));
    assert!(elapsed >= Duration::from_secs(2));
    worker.join().unwrap();
    drop(held);
    Metadata::initialize(&paths.state).unwrap();
    Metadata::inspect(&paths.state).unwrap();
    assert_eq!(fs::read(&paths.config).unwrap(), configuration);
}

#[test]
fn symlinks_hardlinks_and_broad_permissions_are_refused() {
    let root = root();
    let paths = paths(&root);
    Config::initialize(&paths).unwrap();
    let link = root.path().join("linked-parent");
    symlink(paths.config.parent().unwrap(), &link).unwrap();
    assert!(Config::load(&link.join("config.toml")).is_err());
    let parent = filesystem::directory(root.path(), false, false).unwrap();
    assert!(
        filesystem::private_file_at(&parent, std::ffi::OsStr::new("linked-parent/config.toml"))
            .is_err()
    );
    let hardlink = paths.config.with_file_name("hardlink.toml");
    fs::hard_link(&paths.config, &hardlink).unwrap();
    assert!(Config::load(&paths.config).is_err());
    fs::remove_file(hardlink).unwrap();
    fs::set_permissions(&paths.config, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Config::load(&paths.config).is_err());
    fs::set_permissions(&paths.config, fs::Permissions::from_mode(0o400)).unwrap();
    Config::load(&paths.config).unwrap();
    fs::set_permissions(
        paths.config.parent().unwrap(),
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    assert!(Config::load(&paths.config).is_err());
}

#[test]
fn sidecar_symlink_is_refused_without_touching_its_target() {
    let root = root();
    let paths = paths(&root);
    Config::initialize(&paths).unwrap();
    let target = root.path().join("untouched");
    fs::write(&target, b"sentinel").unwrap();
    let sidecar = paths.state.join("metadata.sqlite3-wal");
    assert!(!sidecar.exists());
    symlink(&target, &sidecar).unwrap();
    assert!(matches!(
        Metadata::inspect(&paths.state),
        Err(Failure::MetadataUnavailable)
    ));
    assert_eq!(fs::read(&target).unwrap(), b"sentinel");
}

#[test]
fn owner_and_migration_tampering_refuse_inspection() {
    for statement in [
        "UPDATE local_authority SET owner_uid=owner_uid+1",
        "UPDATE schema_migrations SET digest='changed'",
    ] {
        let root = root();
        let paths = paths(&root);
        Config::initialize(&paths).unwrap();
        let db = rusqlite::Connection::open(paths.state.join("metadata.sqlite3")).unwrap();
        db.execute(statement, []).unwrap();
        assert!(matches!(
            Metadata::inspect(&paths.state),
            Err(Failure::MetadataUnavailable)
        ));
    }
}
