//! story:invoke-definition-file: an invoked definition's instance is held
//! beside the operator's configuration, never in it, and never in place of an
//! instance the operator configured.
use super::*;
use crate::local::config::{Executable, Permissions};
use std::collections::BTreeSet;

fn paths() -> (tempfile::TempDir, Paths) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    Config::initialize(&paths).unwrap();
    (root, paths)
}

fn entry(root: &std::path::Path, connector: &str, digest: char) -> Entry {
    Entry {
        digest: digest.to_string().repeat(64),
        adapter: Adapter {
            credential_revision: Some("credential-1".into()),
            instance_id: alias(connector),
            adapter_id: "catalog".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: Executable {
                path: root.join("connectors-catalog-provider"),
                sha256: "a".repeat(64),
                args: vec!["--local-config".into(), "/fixture/native.json".into()],
            },
            permissions: Permissions {
                profiles: BTreeSet::from(["forge.pat".to_owned()]),
                operations: BTreeSet::from(["project.get".to_owned()]),
                custody: Default::default(),
            },
        },
    }
}

#[test]
fn a_recorded_definition_is_the_instance_definition_of_its_connector() {
    let (root, paths) = paths();
    let operator = std::fs::read(&paths.config).unwrap();
    record(&paths, "forge", &entry(root.path(), "forge", 'b')).unwrap();
    let config = Config::read_at(&paths).unwrap();
    let selected = config
        .select(Selector::Instance {
            connector: "forge",
            instance: INSTANCE,
        })
        .unwrap();
    assert_eq!(selected.alias, "forge.definition");
    assert_eq!(selected.adapter.instance_id, "forge.definition");
    assert_eq!(config.names("forge.definition"), Some(("forge", INSTANCE)));
    // Only a read beside the state sees it; the operator's file is untouched.
    assert!(Config::read(&paths.config).unwrap().adapters.is_empty());
    assert_eq!(std::fs::read(&paths.config).unwrap(), operator);
}

#[test]
fn a_held_definition_is_never_written_to_the_operators_file() {
    let (root, paths) = paths();
    record(&paths, "forge", &entry(root.path(), "forge", 'b')).unwrap();
    let config = Config::read_at(&paths).unwrap();
    assert!(config.adapters.contains_key("forge.definition"));
    let text = config.to_text().unwrap();
    assert!(!text.contains("forge.definition"), "{text}");
    assert!(config.legacy_aliases().is_empty());
}

#[test]
fn recording_one_definition_keeps_every_other() {
    let (root, paths) = paths();
    record(&paths, "forge", &entry(root.path(), "forge", 'b')).unwrap();
    record(&paths, "other", &entry(root.path(), "other", 'c')).unwrap();
    record(&paths, "forge", &entry(root.path(), "forge", 'd')).unwrap();
    let held = entries(&paths).unwrap();
    assert_eq!(held.keys().collect::<Vec<_>>(), ["forge", "other"]);
    assert_eq!(held["forge"].digest, "d".repeat(64));
    assert_eq!(held["other"].digest, "c".repeat(64));
}

#[test]
fn an_instance_the_operator_configured_is_not_replaced() {
    let (root, paths) = paths();
    // The operator's entry already uses the definition's instance id.
    let mut config = Config::load(&paths.config).unwrap();
    config.format = "connectors-local/2".into();
    let mut operators = entry(root.path(), "forge", 'b').adapter;
    operators.configuration_revision = "operators".into();
    config.adapters.insert("mine".into(), operators);
    std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
    let operator = Config::read(&paths.config).unwrap();
    assert!(collides(&operator, "forge"));
    assert!(!collides(&operator, "other"));

    record(&paths, "forge", &entry(root.path(), "forge", 'c')).unwrap();
    let config = Config::read_at(&paths).unwrap();
    assert!(!config.adapters.contains_key("forge.definition"));
    assert!(config.definitions.is_empty());
    assert_eq!(config.adapters["mine"].configuration_revision, "operators");
}

#[test]
fn an_entry_of_another_shape_is_neither_recorded_nor_held() {
    let (root, paths) = paths();
    let mut wrong = entry(root.path(), "forge", 'b');
    wrong.adapter.instance_id = "forge.other".into();
    assert!(record(&paths, "forge", &wrong).is_err());
    let mut wrong = entry(root.path(), "forge", 'b');
    wrong.adapter.permissions.custody.insert(
        "project.get".into(),
        crate::local::config::CustodyTarget {
            adapter: "elsewhere".into(),
            profile: "forge.pat".into(),
        },
    );
    assert!(record(&paths, "forge", &wrong).is_err());
    assert!(entries(&paths).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn an_entries_file_that_is_not_owner_only_is_not_held() {
    use std::os::unix::fs::PermissionsExt;
    let (root, paths) = paths();
    record(&paths, "forge", &entry(root.path(), "forge", 'b')).unwrap();
    let file = paths.state.join(DIRECTORY).join(ENTRIES);
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(paths.state.join(DIRECTORY))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(entries(&paths).is_err());
    assert!(
        !Config::read_at(&paths)
            .unwrap()
            .adapters
            .contains_key("forge.definition")
    );
}
