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

fn entry(paths: &Paths, connector: &str, digest: char) -> Entry {
    let digest = digest.to_string().repeat(64);
    let native = paths.state.join(DIRECTORY).join(&digest).join(NATIVE);
    Entry {
        digest,
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
                path: paths.state.join(format!(
                    "connectors-catalog-provider{}",
                    std::env::consts::EXE_SUFFIX
                )),
                sha256: "a".repeat(64),
                args: vec!["--local-config".into(), native.to_str().unwrap().into()],
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
    let (_root, paths) = paths();
    let operator = std::fs::read(&paths.config).unwrap();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
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
    let (_root, paths) = paths();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
    let config = Config::read_at(&paths).unwrap();
    assert!(config.adapters.contains_key("forge.definition"));
    let text = config.to_text().unwrap();
    assert!(!text.contains("forge.definition"), "{text}");
    assert!(config.legacy_aliases().is_empty());
}

#[test]
fn recording_one_definition_keeps_every_other() {
    let (_root, paths) = paths();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
    record(&paths, "other", &entry(&paths, "other", 'c')).unwrap();
    record(&paths, "forge", &entry(&paths, "forge", 'd')).unwrap();
    let held = entries(&paths).unwrap();
    assert_eq!(held.keys().collect::<Vec<_>>(), ["forge", "other"]);
    assert_eq!(held["forge"].digest, "d".repeat(64));
    assert_eq!(held["other"].digest, "c".repeat(64));
}

#[test]
fn an_instance_the_operator_configured_is_not_replaced() {
    let (_root, paths) = paths();
    // The operator's entry already uses the definition's instance id.
    let mut config = Config::load(&paths.config).unwrap();
    config.format = "connectors-local/2".into();
    let mut operators = entry(&paths, "forge", 'b').adapter;
    operators.configuration_revision = "operators".into();
    config.adapters.insert("mine".into(), operators);
    std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
    let operator = Config::read(&paths.config).unwrap();
    assert!(collides(&operator, "forge"));
    assert!(!collides(&operator, "other"));

    record(&paths, "forge", &entry(&paths, "forge", 'c')).unwrap();
    let config = Config::read_at(&paths).unwrap();
    assert!(!config.adapters.contains_key("forge.definition"));
    assert!(config.definitions.is_empty());
    assert_eq!(config.adapters["mine"].configuration_revision, "operators");
}

#[test]
fn an_entry_of_another_shape_is_neither_recorded_nor_held() {
    let (_root, paths) = paths();
    let mut wrong = entry(&paths, "forge", 'b');
    wrong.adapter.instance_id = "forge.other".into();
    assert!(record(&paths, "forge", &wrong).is_err());
    let mut wrong = entry(&paths, "forge", 'b');
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
    let (_root, paths) = paths();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
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

/// Adversary, round 2: `admitted` compares the recorded `--local-config` path
/// with `paths.state` as text. The same state directory named
/// `<root>/./state` (admitted by `validate_path`, the same directory for every
/// other command) refuses the whole entries file, so every held definition
/// silently disappears from `Config::read_at` and no definition can be recorded.
#[cfg(unix)]
#[test]
fn adversary_an_entry_is_held_whichever_spelling_names_the_state_directory() {
    let (root, paths) = paths();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
    let dotted = Paths {
        config: paths.config.clone(),
        state: std::path::PathBuf::from(format!("{}/./state", root.path().display())),
    };
    assert_eq!(
        entries(&dotted).map(|held| held.keys().cloned().collect::<Vec<_>>()),
        Ok(vec!["forge".to_owned()])
    );
    assert!(
        Config::read_at(&dotted)
            .unwrap()
            .adapters
            .contains_key("forge.definition")
    );
}

/// story:invoke-definition-reuse-credential, security review round 1 F1: the
/// binding compares the provider authority and the profile declaration, which
/// a token endpoint is not part of. Only a definition instance whose native
/// configuration sends the credential to its API base's origin, and nowhere
/// else, receives a reused credential.
#[cfg(unix)]
#[test]
fn only_a_definition_whose_profile_names_no_url_receives_a_reuse() {
    let (_root, paths) = paths();
    let recorded = entry(&paths, "forge", 'b');
    record(&paths, "forge", &recorded).unwrap();
    let directory = paths.state.join(DIRECTORY).join(&recorded.digest);
    fs::directory(&directory, true, true).unwrap();
    let native = |value: serde_json::Value| {
        let path = directory.join(NATIVE);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .unwrap();
        let config = Config::read_at(&paths).unwrap();
        receives_reuse(&paths, &config, "forge.definition").unwrap()
    };
    let base = |auth: serde_json::Value| {
        serde_json::json!({
            "format": "connectors-catalog-local/2",
            "instance": "forge.definition",
            "provider": "forge",
            "bundle_directory": directory,
            "api_base": "https://gitlab.example.com/api/v4",
            "auth": auth,
            "operations_file": directory.join("operations.json"),
        })
    };
    // A header token: sent to the API alone.
    assert!(native(base(
        serde_json::json!({"profile": "forge.pat", "scheme": "header"})
    )));
    // Any token endpoint, even on the API's own origin: the source may send its
    // secret elsewhere (security review round 2), so no reuse.
    for other in [
        "https://GitLab.example.com/oauth/token",
        "https://collector.example.net/oauth/token",
        "https://gitlab.example.com:8443/oauth/token",
        "http://gitlab.example.com/oauth/token",
        "https://user@gitlab.example.com/oauth/token",
    ] {
        assert!(
            !native(base(
                serde_json::json!({"profile": "forge.oauth", "token_url": other})
            )),
            "{other}"
        );
    }
    // Any trust input or admitted host the binding does not compare: no reuse.
    for key in ["hosts", "ca_file", "token_ca_file"] {
        let mut value = base(serde_json::json!({"profile": "forge.pat"}));
        value[key] = serde_json::json!("https://gitlab.example.com");
        assert!(!native(value), "{key}");
    }
    // An instance the operator configured is no target.
    let config = Config::read_at(&paths).unwrap();
    assert!(!receives_reuse(&paths, &config, "forge.main").unwrap());
}
