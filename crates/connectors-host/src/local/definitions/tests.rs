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
    assert!(collides(&operator, "forge", INSTANCE));
    assert!(!collides(&operator, "other", INSTANCE));

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

const MOVED_INSTANCE: &str = "definition-0123456789abcdef";

/// task:definition-moved-authority-new-instance: a definition whose instance
/// `definition` cannot hold its binding is recorded under its moved instance,
/// which every command selects by that name, and the instance `definition` is
/// no longer configured, so what it holds is left behind and revocable.
#[test]
fn a_moved_definition_is_held_under_its_own_instance_alone() {
    let (_root, paths) = paths();
    record(&paths, "forge", &entry(&paths, "forge", 'b')).unwrap();
    let mut moved = entry(&paths, "forge", 'c');
    moved.adapter.instance_id = alias_of("forge", MOVED_INSTANCE);
    record(&paths, "forge", &moved).unwrap();
    let config = Config::read_at(&paths).unwrap();
    let selected = config
        .select(Selector::Instance {
            connector: "forge",
            instance: MOVED_INSTANCE,
        })
        .unwrap();
    assert_eq!(selected.alias, "forge.definition-0123456789abcdef");
    assert_eq!(selected.instance, MOVED_INSTANCE);
    assert_eq!(
        config.names("forge.definition-0123456789abcdef"),
        Some(("forge", MOVED_INSTANCE))
    );
    assert!(
        config
            .select(Selector::Instance {
                connector: "forge",
                instance: INSTANCE,
            })
            .is_none()
    );
    assert!(!config.adapters.contains_key("forge.definition"));
    // An operator's instance of that name is never replaced.
    assert!(!collides(&config, "forge", INSTANCE));
    assert!(collides(&config, "forge", MOVED_INSTANCE));
}

#[test]
fn only_definition_or_a_moved_instance_name_is_recorded() {
    let (_root, paths) = paths();
    for instance in [
        "definition-0123456789ABCDEF",
        "definition-0123456789abcde",
        "definition-0123456789abcdef0",
        "definition-0123456789abcdeg",
        "definitions",
        "definition-",
        "other",
    ] {
        let mut wrong = entry(&paths, "forge", 'b');
        wrong.adapter.instance_id = alias_of("forge", instance);
        assert!(record(&paths, "forge", &wrong).is_err(), "{instance}");
    }
    // Another connector's moved alias is not this connector's.
    let mut wrong = entry(&paths, "forge", 'b');
    wrong.adapter.instance_id = alias_of("other", MOVED_INSTANCE);
    assert!(record(&paths, "forge", &wrong).is_err());
}

/// A profile `forge.pat` declared as the catalog provider declares one, with
/// this evidence lifetime and header.
fn declared(lifetime: u64, header: &str) -> registry::StaticProfile {
    let mut profile = runtime::Profile {
        id: "forge.pat".into(),
        revision: String::new(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "header".into(),
        capability: header.into(),
        minimum_scopes: BTreeSet::from(["api".to_owned()]),
        evidence_lifetime_ms: lifetime,
        fields: Vec::new(),
        acquisition: None,
    };
    profile.revision = connectors_core::digest(&serde_json::to_value(&profile).unwrap());
    profile.registry_profile()
}

fn bound(authority: &str, profile: registry::StaticProfile) -> registry::Binding {
    registry::Binding {
        instance_id: alias("forge"),
        adapter_id: "catalog".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: authority.into(),
        profile,
        credential_revision: Some("credential-1".into()),
        executable: None,
    }
}

/// The moved instance is a function of what makes a new instance id
/// necessary, the connector, the provider authority and the profile's
/// authentication, and of nothing else: repeated runs name the same instance,
/// and a lifetime-only change, which the registry admits under the same id,
/// keeps it.
#[test]
fn the_moved_instance_follows_the_authority_and_the_authentication_alone() {
    let at = |authority: &str| bound(authority, declared(60_000, "PRIVATE-TOKEN"));
    let first = moved("forge", &at("https://gitlab-moved.example.com/api/v4"));
    let digest = first.strip_prefix("definition-").unwrap();
    assert!(
        digest.len() == 16
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{first}"
    );
    assert!(connectors_core::valid_id(&alias_of("forge", &first)));
    assert_eq!(
        first,
        moved("forge", &at("https://gitlab-moved.example.com/api/v4"))
    );
    let mut revised = at("https://gitlab-moved.example.com/api/v4");
    revised.configuration_revision = "cfg-2".into();
    revised.credential_revision = Some("credential-2".into());
    assert_eq!(first, moved("forge", &revised));
    let lifetime = bound(
        "https://gitlab-moved.example.com/api/v4",
        declared(300_000, "PRIVATE-TOKEN"),
    );
    assert_eq!(first, moved("forge", &lifetime));

    for other in [
        moved("forge", &at("https://gitlab.example.com/api/v4")),
        moved("other", &at("https://gitlab-moved.example.com/api/v4")),
        moved(
            "forge",
            &bound(
                "https://gitlab-moved.example.com/api/v4",
                declared(60_000, "AUTHORIZATION"),
            ),
        ),
    ] {
        assert_ne!(first, other);
    }
}

/// Security review of task:definition-moved-authority-new-instance,
/// invariants 2 and 4: over every combination of authority, purpose, subject,
/// scheme, header, scopes and evidence lifetime, two bindings share a moved
/// instance exactly when the registry admits a connection of one under the
/// other (same provider authority, same authentication of the recorded
/// profile): a moved instance never holds two authorities or two
/// authentications, and is never itself the dead end it replaces.
#[test]
fn security_review_a_moved_instance_is_shared_exactly_by_bindings_the_registry_admits_together() {
    let mut bindings = Vec::new();
    for authority in [
        "https://gitlab.example.com/api/v4",
        "https://gitlab-moved.example.com/api/v4",
    ] {
        for purpose in [
            registry::Purpose::DelegatedUser,
            registry::Purpose::ServiceAccount,
        ] {
            for subject in [registry::Subject::User, registry::Subject::App] {
                for scheme in ["header", "bearer"] {
                    for header in ["PRIVATE-TOKEN", "AUTHORIZATION"] {
                        for scope in ["api", "read_api"] {
                            for lifetime in [60_000, 300_000] {
                                let mut profile = runtime::Profile {
                                    id: "forge.pat".into(),
                                    revision: String::new(),
                                    purpose,
                                    subject,
                                    scheme: scheme.into(),
                                    capability: header.into(),
                                    minimum_scopes: BTreeSet::from([scope.to_owned()]),
                                    evidence_lifetime_ms: lifetime,
                                    fields: Vec::new(),
                                    acquisition: None,
                                };
                                profile.revision = connectors_core::digest(
                                    &serde_json::to_value(&profile).unwrap(),
                                );
                                bindings.push(bound(authority, profile.registry_profile()));
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(bindings.len(), 128);
    let named: Vec<String> = bindings.iter().map(|b| moved("forge", b)).collect();
    for (a, admitted) in bindings.iter().enumerate() {
        // What the registry records: the profile without its declaration.
        let mut recorded = admitted.profile.clone();
        recorded.declaration = None;
        for (b, configured) in bindings.iter().enumerate() {
            let together = admitted.provider_authority == configured.provider_authority
                && recorded.same_authentication(&configured.profile);
            assert_eq!(named[a] == named[b], together, "bindings {a} and {b}");
        }
    }
}
