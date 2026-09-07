use super::*;
use connector_secrets::MemoryStore;
use protocol::local_setup::SecretValue;
use std::collections::BTreeMap;

fn fixture() -> (tempfile::TempDir, PersonalSetup, Arc<MemoryStore>) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("connectors.toml");
    let original = format!(
        "# retained operator comment\n[owner]\ntenant_id = \"fixture\"\nagent_id = \"fixture\"\nagent_revision = 1\nauthority_snapshot_id = \"fixture\"\nauthority_snapshot_sha256 = \"{}\"\n",
        "a".repeat(64)
    );
    std::fs::write(&path, original).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let config = PersonalConfig::read(&path).unwrap();
    let store = Arc::new(MemoryStore::new());
    let setup = PersonalSetup::new(path, config.owner_context(), store.clone(), "memory".into());
    (root, setup, store)
}

fn request(value: &str) -> EnrollRequest {
    EnrollRequest {
        provider: "gitlab".into(),
        instance: None,
        credential: "gitlab.token".into(),
        endpoints: BTreeMap::from([("origin".into(), "https://gitlab.example".into())]),
        usernames: BTreeMap::new(),
        allow_writes: false,
        operator_network: false,
        source: CredentialSource::Pasted {
            value: SecretValue::new(value.into()),
        },
    }
}

#[tokio::test]
async fn enrollment_rotates_only_the_selected_credential_and_keeps_backups_value_free() {
    let (_root, setup, store) = fixture();
    let original = std::fs::read(&setup.config_path).unwrap();
    let first = setup.enroll(request("first-fixture-value")).await.unwrap();
    assert_eq!(first["reload_required"], true);
    assert_eq!(
        std::fs::read(first["backup"].as_str().unwrap()).unwrap(),
        original
    );
    let config = PersonalConfig::read(&setup.config_path).unwrap();
    let provider = catalog::provider(catalog::ProviderKey::id("gitlab")).unwrap();
    let declared = provider
        .auth
        .iter()
        .find(|value| value.name == request("").credential)
        .unwrap();
    let reference = integration_catalog::credential_address(
        "fixture",
        provider.authority.unwrap(),
        &config.catalog[0],
        declared.leaf,
    )
    .unwrap();
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "first-fixture-value"
    );
    let second = setup
        .enroll(request("rotated-fixture-value"))
        .await
        .unwrap();
    assert_eq!(second["added_to_existing_identity"], true);
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "rotated-fixture-value"
    );
    let state = setup.auth_status().await.unwrap().to_string();
    assert!(!state.contains("fixture-value"));
    for path in [
        setup.config_path.clone(),
        PathBuf::from(first["backup"].as_str().unwrap()),
        PathBuf::from(second["backup"].as_str().unwrap()),
    ] {
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("fixture-value"));
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o077,
            0
        );
    }
}

#[tokio::test]
async fn invalid_configuration_cannot_change_custody_or_grant_ceiling() {
    let (_root, setup, store) = fixture();
    setup
        .enroll(request("retained-fixture-value"))
        .await
        .unwrap();
    let prior = std::fs::read(&setup.config_path).unwrap();
    let mut write = request("replacement-fixture-value");
    write.allow_writes = true;
    assert_eq!(setup.enroll(write).await, Err(SetupError::Configuration));
    let mut destination = request("replacement-fixture-value");
    destination
        .endpoints
        .insert("origin".into(), "https://other.example".into());
    assert_eq!(
        setup.enroll(destination).await,
        Err(SetupError::Configuration)
    );
    let mut secret_field = request("replacement-fixture-value");
    secret_field
        .endpoints
        .insert("password".into(), "must-not-be-config".into());
    assert_eq!(
        setup.enroll(secret_field).await,
        Err(SetupError::InvalidInput)
    );
    assert_eq!(std::fs::read(&setup.config_path).unwrap(), prior);
    let config = PersonalConfig::read(&setup.config_path).unwrap();
    let provider = catalog::provider(catalog::ProviderKey::id("gitlab")).unwrap();
    let declared = provider
        .auth
        .iter()
        .find(|value| value.name == request("").credential)
        .unwrap();
    let reference = integration_catalog::credential_address(
        "fixture",
        provider.authority.unwrap(),
        &config.catalog[0],
        declared.leaf,
    )
    .unwrap();
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "retained-fixture-value"
    );
}

#[tokio::test]
async fn configuration_owner_swap_and_untrusted_secret_files_are_refused() {
    let (root, setup, _) = fixture();
    let value = root.path().join("credential");
    std::fs::write(&value, "fixture-secret").unwrap();
    std::fs::set_permissions(&value, std::fs::Permissions::from_mode(0o644)).unwrap();
    let mut enrollment = request("unused");
    enrollment.source = CredentialSource::File {
        path: value.clone(),
    };
    assert_eq!(
        setup.enroll(enrollment).await,
        Err(SetupError::Configuration)
    );
    std::fs::set_permissions(&value, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = root.path().join("symlink");
    std::os::unix::fs::symlink(value, &link).unwrap();
    let mut enrollment = request("unused");
    enrollment.source = CredentialSource::File { path: link };
    assert_eq!(
        setup.enroll(enrollment).await,
        Err(SetupError::Configuration)
    );
    let prior = std::fs::read_to_string(&setup.config_path).unwrap();
    std::fs::write(
        &setup.config_path,
        prior.replace("tenant_id = \"fixture\"", "tenant_id = \"other\""),
    )
    .unwrap();
    assert_eq!(
        setup.enroll(request("unused")).await,
        Err(SetupError::Configuration)
    );
}
