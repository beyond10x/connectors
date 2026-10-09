//! The executable composition with an engine: an existing PostgreSQL
//! configuration keeps its bytes and revision, a MySQL configuration advertises
//! its own profile and authority, and the private runtime connects, revalidates
//! and reads through the MySQL wire fixture on loopback.
#[path = "mysql/fixture.rs"]
mod fixture;

use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, path::PathBuf, process::Command};

fn private_directory() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    filesystem::directory(&directory, true, true).unwrap();
    (root, directory)
}
fn write_private(path: &Path, document: &Value) {
    fs::write(path, serde_json::to_vec(document).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
/// The executable's own bootstrap for one configuration document, or `None`
/// when it refuses the document.
fn bootstrap(directory: &Path, document: &Value) -> Option<Bootstrap> {
    let path = directory.join("sql.json");
    write_private(&path, document);
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-sql"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    if !output.status.success() {
        return None;
    }
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    Some(bootstrap)
}
fn document(engine: Option<&str>, port: u16) -> Value {
    let mut document = json!({
        "format":"connectors-sql-local/1",
        "instance":"fixture-sql",
        "host":"127.0.0.1",
        "port":port,
        "database":"fixture",
        "user":"reader",
        "allow_plaintext":true,
        "ca_file":null
    });
    if let Some(engine) = engine {
        document["engine"] = json!(engine);
    }
    document
}
fn profiles(bootstrap: &Bootstrap) -> Vec<String> {
    bootstrap
        .descriptor()
        .unwrap()
        .operations
        .iter()
        .map(|operation| operation.profile.clone())
        .collect()
}

#[test]
fn an_existing_postgresql_configuration_without_an_engine_is_unchanged() {
    let (_root, directory) = private_directory();
    let existing = bootstrap(&directory, &document(None, 5432)).unwrap();
    // The configuration revision is the digest of exactly the effective
    // document every earlier release computed: no engine member appears in it.
    let earlier = json!({"format":"connectors-sql-local/1","instance":"fixture-sql",
        "host":"127.0.0.1","port":5432,"database":"fixture","user":"reader",
        "allow_plaintext":true,"ca_digest":null});
    assert_eq!(
        existing.configuration_revision,
        connectors_core::digest(&earlier)
    );
    assert_eq!(
        existing.provider_authority,
        "postgresql://127.0.0.1:5432/fixture"
    );
    let profile = existing.profile("postgres.password").unwrap();
    assert_eq!(profile.fields[0].label, "PostgreSQL password");
    assert_eq!(existing.profiles.len(), 1);
    assert_eq!(
        profiles(&existing),
        ["postgresql-native-text", "postgresql-native-text"]
    );
    // Naming the default engine is the same configuration, not a new one.
    let explicit = bootstrap(&directory, &document(Some("postgresql"), 5432)).unwrap();
    assert_eq!(
        explicit.configuration_revision,
        existing.configuration_revision
    );
    assert_eq!(explicit.descriptor, existing.descriptor);
}

#[test]
fn a_mysql_configuration_bootstraps_its_own_profile_and_authority() {
    let (_root, directory) = private_directory();
    let mysql = bootstrap(&directory, &document(Some("mysql"), 3306)).unwrap();
    let effective = json!({"format":"connectors-sql-local/1","instance":"fixture-sql",
        "engine":"mysql","host":"127.0.0.1","port":3306,"database":"fixture","user":"reader",
        "allow_plaintext":true,"ca_digest":null});
    assert_eq!(
        mysql.configuration_revision,
        connectors_core::digest(&effective)
    );
    assert_eq!(mysql.provider_authority, "mysql://127.0.0.1:3306/fixture");
    assert_eq!(mysql.profiles.len(), 1);
    let profile = mysql.profile("mysql.password").unwrap();
    assert_eq!(profile.scheme, "session_authority");
    assert_eq!(profile.capability, "session-authority");
    assert!(profile.minimum_scopes.is_empty());
    assert_eq!(profile.fields[0].name, "password");
    assert_eq!(profile.fields[0].label, "MySQL password");
    assert!(
        mysql
            .requirements
            .iter()
            .all(|r| r.profile == "mysql.password")
    );
    assert_eq!(profiles(&mysql), ["mysql-native-text", "mysql-native-text"]);
    // An engine the adapter does not implement is refused, not defaulted.
    for engine in ["sqlite", "MySQL", ""] {
        assert!(
            bootstrap(&directory, &document(Some(engine), 3306)).is_none(),
            "{engine}"
        );
    }
    // The port is still required; the engine does not loosen validation.
    let mut portless = document(Some("mysql"), 3306);
    portless.as_object_mut().unwrap().remove("port");
    assert!(bootstrap(&directory, &portless).is_none());
}

fn selection(directory: &Path, port: u16) -> Adapter {
    let config = directory.join("sql.json");
    write_private(&config, &document(Some("mysql"), port));
    let bootstrap = bootstrap(directory, &document(Some("mysql"), port)).unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-sql"))
        .canonicalize()
        .unwrap();
    Adapter {
        private_protocol: None,
        permissions: Default::default(),
        instance_id: "fixture-sql".into(),
        adapter_id: "sql".into(),
        configuration_revision: bootstrap.configuration_revision,
        protocol: "v1alpha1".into(),
        startup: Startup::OnDemand,
        restart: Restart::Never,
        executable: Executable {
            sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
            path: binary,
            args: vec!["--local-config".into(), config.to_str().unwrap().into()],
        },
    }
}
fn password(value: &str) -> Secret {
    Secret(serde_json::to_vec(&json!({ "password": value })).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}

#[test]
fn mysql_connects_revalidates_and_reads_through_the_private_runtime() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(fixture::start(fixture::Script::default()));
    let (_root, directory) = private_directory();
    let mut child = Child::spawn(&selection(&directory, server.port)).unwrap();
    assert_eq!(
        server.log.lock().unwrap().sessions,
        0,
        "spawning opens no session"
    );
    for _ in 0..2 {
        let baseline = child
            .validate("mysql.password", &password(fixture::PASSWORD), deadline())
            .unwrap();
        assert_eq!(baseline.identity.kind, "mysql.user");
        assert_eq!(baseline.identity.subject, "reader@fixture");
        assert!(baseline.granted_scopes.is_none());
        assert!(baseline.credential_expires_at_ms.is_none());
    }
    assert_eq!(server.log.lock().unwrap().logins.len(), 2);
    // A rejected password is the server's refusal, and the PostgreSQL profile
    // is not this connection's profile.
    assert!(
        child
            .validate("mysql.password", &password("wrong-password"), deadline())
            .is_err()
    );
    assert!(matches!(
        child.validate(
            "postgres.password",
            &password(fixture::PASSWORD),
            deadline()
        ),
        Err(Failure::Unsupported)
    ));
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            "query.read",
            &revision,
            "one",
            &password(fixture::PASSWORD),
            &serde_json::to_vec(&json!({"query":"SELECT 42, NULL","limit":1})).unwrap(),
            deadline(),
        )
        .unwrap();
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["rows"], json!([["42", null]]));
    let log = server.log.lock().unwrap();
    assert_eq!(log.logins.len(), 4);
    assert_eq!(log.statements[0], "SET SESSION TRANSACTION READ ONLY");
}
