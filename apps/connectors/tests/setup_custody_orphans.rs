//! `setup custody-orphans` (contracts/cli/v1alpha1/semantics.md §3, "Custody
//! orphans"), under a disposable Secret Service: it lists the custody items the
//! store of the state directory does not name, grouped by authority, and deletes
//! only the items of the one authority `--mode remove --authority` names.
#![cfg(target_os = "linux")]
use connectors_custody::{
    Backend, Material, Purpose, SIGNING_SEED_BYTES, Scope, Version,
    secret_service::{fixture::Fixture, items_at},
};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

fn put(keyring: &Fixture, scope: Scope) {
    let version = Version::new(scope, Uuid::new_v4()).unwrap();
    let length = if scope.purpose() == Purpose::ApprovalSigning {
        SIGNING_SEED_BYTES
    } else {
        16
    };
    keyring
        .backend(scope)
        .unwrap()
        .write_new(
            version,
            &Material::new(vec![7u8; length]),
            Instant::now() + Duration::from_secs(10),
            || Ok(()),
        )
        .unwrap();
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn result(output: Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    envelope["result"].clone()
}

/// A refusal: a failing exit, `invalid_input` naming `authority`.
fn refused_authority(output: Output) {
    assert!(!output.status.success(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["error"]["data"]["code"], "invalid_input");
    assert_eq!(envelope["error"]["data"]["field"], "authority");
}

/// A store with a private Secret Service holding two items of the store's own
/// authority, two of `first` (a credential and a signing key) and one of `second`.
struct World {
    keyring: Fixture,
    root: tempfile::TempDir,
    own: Uuid,
    first: Uuid,
    second: Uuid,
}

fn world() -> World {
    let keyring = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    result(run(root.path(), &["setup", "init"]));
    let config = root.path().join("config/config.toml");
    fs::write(
        &config,
        format!(
            "format='connectors-local/4'\nowner_uid={}\nsecret_service_socket='{}'\n",
            connectors_host::local::filesystem::uid(),
            keyring.socket.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let own = connectors_host::local::metadata::Metadata::authority_at(&root.path().join("state"))
        .unwrap();
    let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
    put(&keyring, Scope::new(own, Uuid::new_v4()).unwrap());
    put(
        &keyring,
        Scope::approval_signing(own, Uuid::new_v4()).unwrap(),
    );
    put(&keyring, Scope::new(first, Uuid::new_v4()).unwrap());
    put(
        &keyring,
        Scope::approval_signing(first, Uuid::new_v4()).unwrap(),
    );
    put(&keyring, Scope::new(second, Uuid::new_v4()).unwrap());
    World {
        keyring,
        root,
        own,
        first,
        second,
    }
}

fn count(world: &World, authority: Uuid) -> usize {
    items_at(Some(&world.keyring.socket))
        .unwrap()
        .iter()
        .filter(|item| item.authority == authority.to_string())
        .count()
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn the_listing_groups_foreign_authorities_and_says_it_cannot_see_other_stores() {
    let world = world();
    let listed = result(run(world.root.path(), &["setup", "custody-orphans"]));
    assert_eq!(listed["mode"], "list");
    let groups = listed["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 3, "{listed}");
    let row = |authority: Uuid, purpose: &str| {
        groups
            .iter()
            .find(|g| g["authority"] == authority.to_string() && g["purpose"] == purpose)
            .unwrap_or_else(|| panic!("{authority} {purpose}: {listed}"))["count"]
            .clone()
    };
    assert_eq!(row(world.first, "credential"), 1);
    assert_eq!(row(world.first, "approval-signing"), 1);
    assert_eq!(row(world.second, "credential"), 1);
    assert!(
        groups
            .iter()
            .all(|g| g["authority"] != world.own.to_string())
    );
    assert!(
        listed["notice"]
            .as_str()
            .unwrap()
            .contains("another store on this machine that this command cannot see")
    );
    assert_eq!(items_at(Some(&world.keyring.socket)).unwrap().len(), 5);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn remove_without_an_authority_deletes_nothing() {
    let world = world();
    refused_authority(run(
        world.root.path(),
        &["setup", "custody-orphans", "--mode", "remove"],
    ));
    assert_eq!(items_at(Some(&world.keyring.socket)).unwrap().len(), 5);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn remove_deletes_only_the_named_foreign_authority() {
    let world = world();
    let removed = result(run(
        world.root.path(),
        &[
            "setup",
            "custody-orphans",
            "--mode",
            "remove",
            "--authority",
            &world.first.to_string(),
        ],
    ));
    assert_eq!(removed["removed"].as_array().unwrap().len(), 2, "{removed}");
    assert!(removed["failed"].as_array().unwrap().is_empty());
    assert_eq!(count(&world, world.first), 0);
    assert_eq!(count(&world, world.second), 1);
    assert_eq!(count(&world, world.own), 2);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn remove_refuses_the_stores_own_authority() {
    let world = world();
    refused_authority(run(
        world.root.path(),
        &[
            "setup",
            "custody-orphans",
            "--mode",
            "remove",
            "--authority",
            &world.own.to_string(),
        ],
    ));
    assert_eq!(items_at(Some(&world.keyring.socket)).unwrap().len(), 5);
}
