//! Compare the advertised transition with actual durable file-store operations.

use std::os::unix::fs::PermissionsExt;

use connector_secrets::{
    CredentialRef, CredentialScope, FileStore, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretStore, SecretTransactionGeneration, SecretTransactionId,
};

#[tokio::test]
async fn reported_versions_match_ordinary_prepared_reopened_and_later_writes() {
    let scratch = tempfile::tempdir().unwrap();
    std::fs::set_permissions(scratch.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = scratch.path().join("credentials.store");
    let report = connectors_console::upgrade::run("embedded-test-caller");
    let reference = CredentialRef::new("tenant-test", "com.example.api", "api", "key").unwrap();
    let store = FileStore::open(&path).unwrap();
    store
        .put(&reference, &Secret::new("SENTINEL-NOT-A-REAL-SECRET-first"))
        .await
        .unwrap();
    let header = |version: &serde_json::Value| {
        format!(
            "# codewandler-connector-secrets file store, v{}",
            version.as_str().unwrap()
        )
    };
    let assert_header = |version: &serde_json::Value| {
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert_eq!(bytes.lines().next().unwrap(), header(version));
    };
    assert_header(&report["credential_store"]["initial_write_version"]);
    drop(store);
    let store = FileStore::open(&path).unwrap();
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "SENTINEL-NOT-A-REAL-SECRET-first"
    );
    let generation = SecretTransactionGeneration::from_protocol_bytes(1_u64.to_be_bytes()).unwrap();
    let transaction = SecretTransactionId::new(generation, [42; 24]);
    let mut batch =
        SecretBatch::new(CredentialScope::new("tenant-test", "com.example.api").unwrap());
    batch
        .put(
            reference.clone(),
            Secret::new("SENTINEL-NOT-A-REAL-SECRET-second"),
        )
        .unwrap();
    store
        .prepare(
            transaction,
            SecretProposalDigest::from_protocol_bytes([42; 32]),
            &batch,
        )
        .await
        .unwrap();
    assert_header(&report["credential_store"]["prepared_write_version"]);
    drop(store);
    let store = FileStore::open(&path).unwrap();
    store.commit(transaction).await.unwrap();
    store.reclaim(generation).await.unwrap();
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "SENTINEL-NOT-A-REAL-SECRET-second"
    );
    store
        .put(&reference, &Secret::new("SENTINEL-NOT-A-REAL-SECRET-third"))
        .await
        .unwrap();
    assert_header(&report["credential_store"]["prepared_write_version"]);
    drop(store);
    let store = FileStore::open(&path).unwrap();
    assert_eq!(
        store.get(&reference).await.unwrap().expose_secret(),
        "SENTINEL-NOT-A-REAL-SECRET-third"
    );
    assert_eq!(
        report["credential_store"]["read_versions"],
        serde_json::json!(["1", "2"])
    );
    assert_eq!(
        report["credential_store"]["write_versions"],
        serde_json::json!(["1", "2"])
    );
    assert_eq!(
        report,
        connectors_console::upgrade::run("embedded-test-caller")
    );
}
