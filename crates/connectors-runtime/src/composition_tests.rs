use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use super::*;

#[test]
fn git_fetch_environment_override_is_atomic_and_secret_free() {
    assert_eq!(
        git_fetch_override_from_values(None, None, None, None).unwrap(),
        None
    );
    assert!(matches!(
        git_fetch_override_from_values(
            Some("https://git-fetch.example.test".to_owned()),
            None,
            Some("/etc/connectors-git-fetch/tls.crt".to_owned()),
            Some("/etc/connectors-git-fetch/tls.key".to_owned()),
        ),
        Err(HostedServerConfigError::Invalid)
    ));

    let placement = git_fetch_override_from_values(
        Some("https://git-fetch.example.test".to_owned()),
        Some("0.0.0.0:8443".to_owned()),
        Some("/etc/connectors-git-fetch/tls.crt".to_owned()),
        Some("/etc/connectors-git-fetch/tls.key".to_owned()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(placement.origin, "https://git-fetch.example.test");
    assert_eq!(placement.listen, "0.0.0.0:8443".parse().unwrap());
    assert_eq!(
        placement.certificate_file,
        PathBuf::from("/etc/connectors-git-fetch/tls.crt")
    );
    assert_eq!(
        placement.private_key_file,
        PathBuf::from("/etc/connectors-git-fetch/tls.key")
    );
}

#[test]
fn git_fetch_environment_override_refuses_an_invalid_listener() {
    assert!(matches!(
        git_fetch_override_from_values(
            Some("https://git-fetch.example.test".to_owned()),
            Some("not-a-listener".to_owned()),
            Some("/etc/connectors-git-fetch/tls.crt".to_owned()),
            Some("/etc/connectors-git-fetch/tls.key".to_owned()),
        ),
        Err(HostedServerConfigError::Invalid)
    ));
}

#[tokio::test]
async fn empty_personal_runtime_binds_and_cleans_without_a_credential_store() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let state_root = temporary.path().join("state");
    let runtime = PersonalRuntime::bind(None, &state_root).await.unwrap();
    assert_eq!(runtime.readiness()["ready"], true);
    assert_eq!(runtime.readiness()["event_reply_claims"], true);
    assert!(state_root.join("connectors.sock").exists());
    assert!(
        state_root.join("event-reply-claims.sqlite").exists(),
        "the local reply-claim journal exists from first boot"
    );
    assert!(!state_root.join("credentials.store").exists());
    runtime.serve_until(std::future::ready(())).await.unwrap();
    assert!(!state_root.join("connectors.sock").exists());
}

#[test]
fn working_tree_state_roots_are_refused() {
    assert!(matches!(
        validate_state_root(Path::new(env!("CARGO_MANIFEST_DIR"))),
        Err(RuntimeError::UnsafeStateRoot)
    ));
}

#[test]
fn a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("connectors.sqlite3");
    let store = hosted_state_store(
        None,
        Some(path.to_str().expect("a UTF-8 test path").to_owned()),
    )
    .expect("a named SQLite file is a complete answer to where hosted state lives");
    store.replace("connections", b"one", 64).unwrap();
    let read_back = store.read("connections", 64).unwrap();
    assert_eq!(read_back.as_deref(), Some(&b"one"[..]));
    assert!(path.exists(), "the bookkeeping went to the named file");
}

#[test]
fn naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning() {
    assert!(matches!(
        hosted_state_store(
            Some("postgres://localhost/connectors".to_owned()),
            Some("/var/lib/connectors/state.sqlite3".to_owned()),
        ),
        Err(RuntimeError::AmbiguousHostedState)
    ));
}

#[test]
fn naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere() {
    assert!(matches!(
        hosted_state_store(None, None),
        Err(RuntimeError::MissingHostedState)
    ));
}

/// A supervisor that interpolates an unset variable hands the process an empty string, not an
/// absent one. Treating that as a store would either dial an empty database URL or open a file
/// called nothing; both come up looking healthy with no bookkeeping in them.
#[test]
fn an_empty_variable_is_no_store_at_all() {
    assert!(matches!(
        hosted_state_store(Some(String::new()), Some("   ".to_owned())),
        Err(RuntimeError::MissingHostedState)
    ));
}

/// A path that cannot hold a database is a typo in a variable, and the person who typed it
/// needs to see which path the process tried rather than "state is unavailable".
#[test]
fn an_unopenable_sqlite_path_is_named_in_the_refusal() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("absent/connectors.sqlite3");
    let path = path.to_str().expect("a UTF-8 test path").to_owned();
    let Err(refusal) = hosted_state_store(None, Some(path.clone())) else {
        panic!("a SQLite path under a missing directory must be refused");
    };
    assert!(refusal.to_string().contains(&path), "{refusal}");
}

/// The sentence a person reads when the placement did not say where its state lives has to name
/// what to set. The previous one named only the database URL, and stayed on the screen long
/// after a second store existed.
#[test]
fn the_refusal_names_both_stores_a_deployment_may_choose() {
    let Err(refusal) = hosted_state_store(None, None) else {
        panic!("a placement that named no store must be refused");
    };
    let refusal = refusal.to_string();
    assert!(refusal.contains("CONNECTORS_DATABASE_URL"), "{refusal}");
    assert!(refusal.contains("CONNECTORS_SQLITE"), "{refusal}");
}
