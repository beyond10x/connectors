use super::*;
use connectors_host::local::{
    config::{Adapter, Executable, Permissions},
    registry,
};
use std::{
    collections::BTreeSet,
    os::unix::fs::{PermissionsExt, symlink},
};

struct Fixture {
    _root: tempfile::TempDir,
    paths: Paths,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        let adapter = Adapter {
            instance_id: "instance".into(),
            adapter_id: "adapter".into(),
            configuration_revision: "config-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: Executable {
                path: root.path().join("absent-provider"),
                sha256: "a".repeat(64),
                args: vec![],
            },
            permissions: Permissions {
                profiles: BTreeSet::from(["token".into()]),
                operations: BTreeSet::from(["read".into(), "absent".into()]),
            },
        };
        config.adapters.insert("selected".into(), adapter.clone());
        std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: adapter.instance_id.clone(),
            adapter: adapter.adapter_id.clone(),
            revision: "descriptor-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![connectors_core::Operation {
                id: "read".into(),
                description: "fixture".into(),
                contract: "operations/v1alpha1".into(),
                profile: "resource".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
            }],
        };
        let bootstrap = runtime::Bootstrap {
            instance: adapter.instance_id.clone(),
            adapter: adapter.adapter_id.clone(),
            protocol: adapter.protocol.clone(),
            configuration_revision: adapter.configuration_revision.clone(),
            provider_authority: "https://fixture.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: "token".into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![runtime::EntryField {
                    name: "token".into(),
                    label: "Token".into(),
                    max_bytes: 1024,
                }],
                acquisition: None,
            }],
            requirements: vec![runtime::Requirement {
                operation: "read".into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            }],
        };
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
        let fixture = Self { _root: root, paths };
        fixture.write(fixture.value());
        fixture
    }
    fn value(&self) -> serde_json::Value {
        json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,"concurrent_requests":16,"request_milliseconds":120000},"exposures":[{"adapter_alias":"selected","operation_ref":"read","connection_ref":"connection","families":["tools"],"enabled":true,"approval_file":"/nonexistent/proof.json"}]})
    }
    fn write(&self, value: serde_json::Value) {
        let path = companion(&self.paths);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn revision(&self) -> String {
        let config = Config::load(&self.paths.config).unwrap();
        revision(
            &load(&self.paths).unwrap(),
            &config.adapters["selected"],
            &owner::cached(&self.paths, "selected").unwrap(),
        )
    }
    fn request(&self) -> owner::approval_issuance::Request<'static> {
        owner::approval_issuance::Request {
            connection: "connection",
            operation: "read",
            schema: "schema",
            revision: "descriptor-1",
            input: "{}",
        }
    }
    fn check(&self, revision: &str) -> owner::Result<()> {
        ProjectionPolicy.admit(&self.paths, "selected", &self.request(), revision)
    }
}

#[test]
fn current_private_selection_needs_neither_a_provider_nor_an_approval_file() {
    let fixture = Fixture::new();
    fixture.check(&fixture.revision()).unwrap();
    assert!(!fixture._root.path().join("absent-provider").exists());
}
#[test]
fn withdrawal_and_enablement_changes_invalidate_the_old_projection() {
    let fixture = Fixture::new();
    let old = fixture.revision();
    let mut value = fixture.value();
    value["exposures"][0]["enabled"] = json!(false);
    fixture.write(value);
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::StaleDescription
    );
    assert_eq!(
        fixture.check(&fixture.revision()).unwrap_err().code,
        Code::Forbidden
    );
    let mut value = fixture.value();
    value["exposures"] = json!([]);
    fixture.write(value);
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::StaleDescription
    );
    assert_eq!(
        fixture.check(&fixture.revision()).unwrap_err().code,
        Code::NotFound
    );
}
#[test]
fn host_policy_and_target_scope_precede_stale_projection_disclosure() {
    let fixture = Fixture::new();
    let request = owner::approval_issuance::Request {
        connection: "another",
        ..fixture.request()
    };
    assert_eq!(
        ProjectionPolicy
            .admit(&fixture.paths, "selected", &request, "stale")
            .unwrap_err()
            .code,
        Code::Forbidden
    );
    let mut config = Config::load(&fixture.paths.config).unwrap();
    config
        .adapters
        .get_mut("selected")
        .unwrap()
        .permissions
        .operations
        .clear();
    std::fs::write(&fixture.paths.config, toml::to_string(&config).unwrap()).unwrap();
    std::fs::remove_file(companion(&fixture.paths)).unwrap();
    assert_eq!(fixture.check("stale").unwrap_err().code, Code::NotGranted);
}
#[test]
fn unsafe_missing_or_malformed_companion_is_not_an_empty_projection() {
    let fixture = Fixture::new();
    let old = fixture.revision();
    let path = companion(&fixture.paths);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::InvalidConfiguration
    );
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::InvalidConfiguration
    );
    symlink(&fixture.paths.config, &path).unwrap();
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::InvalidConfiguration
    );
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"{}").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        fixture.check(&old).unwrap_err().code,
        Code::InvalidConfiguration
    );
}

#[test]
fn discovery_revision_matches_execution_with_an_unadvertised_cached_operation() {
    let fixture = Fixture::new();
    let config = Config::load(&fixture.paths.config).unwrap();
    let adapter = &config.adapters["selected"];
    let mut original = owner::cached(&fixture.paths, "selected").unwrap();
    let mut descriptor = original.descriptor().unwrap();
    let mut hidden = descriptor.operations[0].clone();
    hidden.id = "hidden".into();
    descriptor.operations.push(hidden);
    original.descriptor = serde_json::to_string(&descriptor).unwrap();
    let mut requirement = original.requirements[0].clone();
    requirement.operation = "hidden".into();
    original.requirements.push(requirement);
    original.validate_for(adapter.private_protocol()).unwrap();
    runtime::state::State::new(&fixture.paths.state)
        .remember(&adapter.selection(), &original)
        .unwrap();
    let revision = ProjectionPolicy
        .metadata_revision(&fixture.paths, "selected", adapter, &original)
        .unwrap();
    fixture.check(&revision).unwrap();
    descriptor.operations.retain(|entry| entry.id != "hidden");
    let mut filtered = original.clone();
    filtered.descriptor = serde_json::to_string(&descriptor).unwrap();
    filtered
        .requirements
        .retain(|entry| entry.operation != "hidden");
    let incorrect = ProjectionPolicy
        .metadata_revision(&fixture.paths, "selected", adapter, &filtered)
        .unwrap();
    assert_ne!(revision, incorrect);
    assert_eq!(
        fixture.check(&incorrect).unwrap_err().code,
        Code::StaleDescription
    );
    let mut companion = fixture.value();
    companion["exposures"][0]["enabled"] = json!(false);
    fixture.write(companion);
    let current = ProjectionPolicy
        .metadata_revision(&fixture.paths, "selected", adapter, &original)
        .unwrap();
    assert_ne!(current, revision);
    assert_eq!(
        fixture.check(&revision).unwrap_err().code,
        Code::StaleDescription
    );
    assert_eq!(fixture.check(&current).unwrap_err().code, Code::Forbidden);
}
