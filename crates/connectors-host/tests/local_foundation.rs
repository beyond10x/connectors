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
                Config::initialize(&Paths { config, state })
            })
        })
        .collect();
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(
        results
            .iter()
            .all(|r| r.is_ok() || *r == Err(Failure::ConfigurationExists)),
        "{results:?}"
    );
    Metadata::inspect(&paths.state).unwrap();
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
