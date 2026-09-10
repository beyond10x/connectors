use super::*;
use crate::local::keyring::custody::tests::Fixture;

fn fixture_store(f: &Fixture) -> Store {
    fs::directory(f.root.path(), false, true).expect("private fixture bus parent");
    assert_eq!(
        crate::local::keyring::inspect_at(Some(&f.socket)),
        crate::local::keyring::State::Available
    );
    let state = f.root.path().join("metadata");
    fs::directory(&state, true, true).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    Store::new(
        &state,
        Some(&f.socket),
        "fixture-instance",
        "fixture-adapter",
        "cfg-1",
    )
    .unwrap()
}

#[test]
fn passive_status_does_not_migrate_or_touch_custody() {
    let root = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let state = root.path().join("state");
    fs::directory(&state, true, true).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    let store = Store::new(
        &state,
        Some(&root.path().join("missing-bus")),
        "instance",
        "adapter",
        "cfg",
    )
    .unwrap();
    assert!(store.status().unwrap().is_none());
    assert!(!state.join(LOCK).exists());
    let m = Metadata::inspect(&state).unwrap();
    assert_eq!(
        m.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn unavailable_write_leaves_exact_candidate_and_no_duplicate_allocation() {
    let root = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    let state = root.path().join("state");
    fs::directory(&state, true, true).unwrap();
    drop(Metadata::initialize(&state).unwrap());
    let store = Store::new(
        &state,
        Some(&root.path().join("missing-bus")),
        "instance",
        "adapter",
        "cfg",
    )
    .unwrap();
    assert_eq!(store.init(None).unwrap_err(), Failure::CustodyUnavailable);
    let view = store.status().unwrap().unwrap();
    assert_eq!(view.keys.len(), 1);
    assert_eq!(view.keys[0].state, State::Candidate);
    assert_eq!(store.init(None).unwrap_err(), Failure::Conflict);
    assert_eq!(
        store.init(Some(view.revision)).unwrap_err(),
        Failure::Conflict
    );
    assert_eq!(
        store
            .recover(view.revision, view.keys[0].key_id)
            .unwrap_err(),
        Failure::CustodyUnavailable
    );
    let encoded = serde_json::to_string(&view).unwrap();
    let m = Metadata::inspect(&state).unwrap();
    let private:(String,String)=m.connection.query_row("SELECT custody_scope,material_version FROM local_approval_issuers JOIN local_approval_keys USING(issuer_id)",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert!(!encoded.contains(&private.0));
    assert!(!encoded.contains(&private.1));
    assert_eq!(
        m.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn durable_key_management_with_qualified_custody() {
    let f = Fixture::new();
    let store = fixture_store(&f);
    let initial = store.init(None).unwrap();
    let old = initial.active().unwrap().key_id;
    assert_eq!(initial.keys.len(), 1);
    assert_eq!(
        store.acquire_key().unwrap().key().unwrap().kid,
        old.to_string()
    );
    store
        .acquire_key()
        .unwrap()
        .with_signer(|_| Ok(()))
        .unwrap();
    assert_eq!(
        store.retire(initial.revision, old).unwrap_err(),
        Failure::Conflict
    );
    let rotated = store.rotate(initial.revision, old).unwrap();
    let new = rotated.active().unwrap().key_id;
    assert_ne!(new, old);
    assert_eq!(rotated.key(old).unwrap().state, State::Retired);
    assert_eq!(
        store.revoke(initial.revision, old).unwrap_err(),
        Failure::Conflict
    );
    assert_eq!(
        store.recover(rotated.revision, new).unwrap_err(),
        Failure::Conflict
    );
    let retired = store.retire(rotated.revision, old).unwrap();
    assert_eq!(retired.key(old).unwrap().state, State::Deleted);
    let revoked = store.revoke(retired.revision, new).unwrap();
    assert!(revoked.active().is_none());
    assert!(matches!(store.acquire_key(), Err(Failure::NotFound)));
    assert_eq!(store.init(None).unwrap_err(), Failure::Conflict);
    let fresh = store.init(Some(revoked.revision)).unwrap();
    assert_ne!(fresh.active().unwrap().key_id, new);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn restart_failed_rotation_recovery_and_revocation_fences() {
    let mut f = Fixture::new();
    let store = fixture_store(&f);
    let initial = store.init(None).unwrap();
    let old = initial.active().unwrap().key_id;
    f.stop();
    f.start(false);
    assert_eq!(
        store.status().unwrap().unwrap().active().unwrap().key_id,
        old
    );
    assert_eq!(
        store.rotate(initial.revision, old).unwrap_err(),
        Failure::CustodyUnavailable
    );
    let pending = store.current().unwrap();
    let candidate = pending.view.candidate().unwrap().key_id;
    assert_eq!(pending.view.active().unwrap().key_id, old);
    f.stop();
    f.start(true);
    store
        .acquire_key()
        .unwrap()
        .with_signer(|_| Ok(()))
        .unwrap();
    // There was no stored seed during the locked write. Recovery cannot invent it.
    assert_eq!(
        store.recover(pending.view.revision, candidate).unwrap_err(),
        Failure::CustodyUnavailable
    );
    let revoked = store.revoke(pending.view.revision, old).unwrap();
    assert_eq!(revoked.key(candidate).unwrap().state, State::Retired);
    assert_eq!(
        store.recover(pending.view.revision, candidate).unwrap_err(),
        Failure::Conflict
    );
    let cleared = store.retire(revoked.revision, candidate).unwrap();
    assert_eq!(cleared.key(candidate).unwrap().state, State::Deleted);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn key_use_excludes_rotation_and_concurrent_init_has_one_winner() {
    let f = Fixture::new();
    let store = fixture_store(&f);
    let gate = std::sync::Barrier::new(2);
    std::thread::scope(|s| {
        let a = s.spawn(|| {
            gate.wait();
            store.init(None)
        });
        let b = s.spawn(|| {
            gate.wait();
            store.init(None)
        });
        let (a, b) = (a.join().unwrap(), b.join().unwrap());
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert_eq!(a.err().or(b.err()), Some(Failure::Conflict));
    });
    let view = store.status().unwrap().unwrap();
    let key = view.active().unwrap().key_id;
    let use_key = store.acquire_key().unwrap();
    assert_eq!(
        store.rotate(view.revision, key).unwrap_err(),
        Failure::Conflict
    );
    assert_eq!(
        store.revoke(view.revision, key).unwrap_err(),
        Failure::Conflict
    );
    assert_eq!(store.status().unwrap().unwrap().revision, view.revision);
    drop(use_key);
    store.revoke(view.revision, key).unwrap();
    let changed = Store::new(
        &store.path,
        Some(&f.socket),
        "fixture-instance",
        "changed-adapter",
        "cfg-1",
    )
    .unwrap();
    assert_eq!(changed.status().unwrap_err(), Failure::Conflict);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn wrong_recovery_material_never_publishes() {
    let f = Fixture::new();
    let store = fixture_store(&f);
    let initial = store.init(None).unwrap();
    let old = initial.active().unwrap().key_id;
    let seed = Secret(vec![31; 32]);
    let (candidate, key) = {
        let _lease = Lease::acquire(&store.path, true).unwrap();
        store
            .stage(
                Some(initial.revision),
                Some(old),
                &public_key(&seed).unwrap(),
            )
            .unwrap()
    };
    let version = candidate.version(key).unwrap();
    let custody = custody::Store::open_at(candidate.scope, Some(&f.socket)).unwrap();
    custody
        .write_new_guarded(version, &Secret(vec![32; 32]), || Ok(()))
        .unwrap();
    assert_eq!(
        store.recover(candidate.view.revision, key).unwrap_err(),
        Failure::CustodyUnavailable
    );
    assert_eq!(
        store.status().unwrap().unwrap().active().unwrap().key_id,
        old
    );
}

#[test]
#[ignore = "auxiliary child entry; parent invokes exact phases"]
fn key_crash_child() {
    let phase = std::env::var("CONNECTORS_KEY_CRASH_PHASE").unwrap();
    let path = PathBuf::from(std::env::var_os("CONNECTORS_KEY_CRASH_STATE").unwrap());
    let socket = PathBuf::from(std::env::var_os("CONNECTORS_KEY_CRASH_SOCKET").unwrap());
    let store = Store::new(
        &path,
        Some(&socket),
        "fixture-instance",
        "fixture-adapter",
        "cfg-1",
    )
    .unwrap();
    let _lease = Lease::acquire(&path, true).unwrap();
    let current = store.current().unwrap();
    let old = current.view.active().unwrap().key_id;
    let mut seed = Secret(vec![0; 32]);
    SystemRandom::new().fill(&mut seed.0).unwrap();
    let (record, key) = store
        .stage(
            Some(current.view.revision),
            Some(old),
            &public_key(&seed).unwrap(),
        )
        .unwrap();
    if phase == "staged" {
        std::process::exit(73);
    }
    let version = record.version(key).unwrap();
    let custody = custody::Store::open_at(record.scope, Some(&socket)).unwrap();
    let receipt = custody
        .write_new_guarded(version, &seed, || {
            store
                .candidate_guard(record.view.revision, key, version)
                .map_err(|_| custody::Failure::Denied)
        })
        .unwrap();
    if phase == "stored" {
        std::process::exit(73);
    }
    assert_eq!(phase, "published");
    store.publish(record.view.revision, key, &receipt).unwrap();
    std::process::exit(73);
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn process_exits_keep_pre_and_post_publication_distinct() {
    for phase in ["staged", "stored", "published"] {
        let mut f = Fixture::new();
        let store = fixture_store(&f);
        let initial = store.init(None).unwrap();
        let old = initial.active().unwrap().key_id;
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "local::approval_keys::tests::key_crash_child",
            ])
            .env("CONNECTORS_KEY_CRASH_PHASE", phase)
            .env("CONNECTORS_KEY_CRASH_STATE", &store.path)
            .env("CONNECTORS_KEY_CRASH_SOCKET", &f.socket)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(73),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        f.stop();
        f.start(true);
        let current = store.status().unwrap().unwrap();
        if phase == "published" {
            let new = current.active().unwrap().key_id;
            assert_ne!(new, old);
            assert_eq!(current.key(old).unwrap().state, State::Retired);
            assert_eq!(
                store.recover(current.revision, new).unwrap_err(),
                Failure::Conflict
            );
        } else {
            assert_eq!(current.active().unwrap().key_id, old);
            let candidate = current.candidate().unwrap().key_id;
            if phase == "stored" {
                let recovered = store.recover(current.revision, candidate).unwrap();
                assert_eq!(recovered.active().unwrap().key_id, candidate);
                assert_eq!(recovered.key(old).unwrap().state, State::Retired);
            } else {
                assert_eq!(
                    store.recover(current.revision, candidate).unwrap_err(),
                    Failure::CustodyUnavailable
                );
            }
        }
        store
            .acquire_key()
            .unwrap()
            .with_signer(|_| Ok(()))
            .unwrap();
    }
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn unknown_storage_acknowledgement_requires_exact_reconfirmation() {
    let f = Fixture::new();
    let store = fixture_store(&f);
    let initial = store.init(None).unwrap();
    let old = initial.active().unwrap().key_id;
    let seed = Secret(vec![41; 32]);
    let (candidate, key) = {
        let _lease = Lease::acquire(&store.path, true).unwrap();
        store
            .stage(
                Some(initial.revision),
                Some(old),
                &public_key(&seed).unwrap(),
            )
            .unwrap()
    };
    let version = candidate.version(key).unwrap();
    let custody = custody::Store::open_at(candidate.scope, Some(&f.socket)).unwrap();
    custody.fail_synchronization(true);
    assert!(matches!(
        custody.write_new_guarded(version, &seed, || Ok(())),
        Err(custody::Failure::OutcomeUnknown)
    ));
    assert_eq!(
        store.status().unwrap().unwrap().active().unwrap().key_id,
        old
    );
    let recovered = store.recover(candidate.view.revision, key).unwrap();
    assert_eq!(recovered.active().unwrap().key_id, key);
    assert_eq!(recovered.key(old).unwrap().state, State::Retired);
    assert!(
        custody
            .write_new_guarded(version, &seed, || Ok(()))
            .is_err()
    );
    // SQL protects historical identity and disallows reactivation independently
    // of public parser checks; no deletion or identity reuse is available.
    let m = Metadata::update_approval_keys(&store.path).unwrap();
    assert!(
        m.connection
            .execute(
                "DELETE FROM local_approval_keys WHERE key_id=?1",
                [old.to_string()]
            )
            .is_err()
    );
    assert!(
        m.connection
            .execute(
                "UPDATE local_approval_keys SET state='active' WHERE key_id=?1",
                [old.to_string()]
            )
            .is_err()
    );
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn purpose_isolation_for_identical_private_uuid_tuples() {
    let f = Fixture::new();
    let authority = Uuid::new_v4();
    let allocation = Uuid::new_v4();
    let material = Uuid::new_v4();
    let credential = custody::Scope::new(authority, allocation).unwrap();
    let signing = custody::Scope::approval_signing(authority, allocation).unwrap();
    let credential_version = custody::Version::new(credential, material).unwrap();
    let signing_version = custody::Version::new(signing, material).unwrap();
    let a = custody::Store::open_at(credential, Some(&f.socket)).unwrap();
    let b = custody::Store::open_at(signing, Some(&f.socket)).unwrap();
    a.write_new_guarded(credential_version, &Secret(vec![61; 32]), || Ok(()))
        .unwrap();
    b.write_new_guarded(signing_version, &Secret(vec![62; 32]), || Ok(()))
        .unwrap();
    assert_eq!(a.read(credential_version).unwrap().0, vec![61; 32]);
    assert_eq!(b.read(signing_version).unwrap().0, vec![62; 32]);
    assert!(matches!(
        a.read(signing_version),
        Err(custody::Failure::Denied)
    ));
    assert!(matches!(
        b.read(credential_version),
        Err(custody::Failure::Denied)
    ));
    assert!(matches!(
        a.confirm_signing_key_guarded(credential_version, || Ok(()), |_| Ok(())),
        Err(custody::Failure::Denied)
    ));
    b.delete_guarded(signing_version, || Ok(())).unwrap();
    assert_eq!(a.read(credential_version).unwrap().0, vec![61; 32]);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable GNOME Secret Service"]
fn production_cli_key_journey() {
    use std::process::Command;
    let executable =
        std::env::var_os("CONNECTORS_TEST_CLI").expect("exact built CLI path required");
    let mut f = Fixture::new();
    let state = f.root.path().join("cli-state");
    let config = f.root.path().join("config/config.toml");
    let run = |args: &[&str]| {
        let output = Command::new(&executable)
            .args(["--output", "json", "--config"])
            .arg(&config)
            .arg("--state-dir")
            .arg(&state)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"].clone()
    };
    run(&["setup", "init"]);
    let original = std::fs::read_to_string(&config).unwrap();
    let configured = format!(
        "secret_service_socket={}\n{original}\n[adapters.forge]\ninstance_id='fixture-instance'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/adapter'\nsha256='{}'\nargs=[]\n",
        toml::Value::String(f.socket.to_str().unwrap().into()),
        "a".repeat(64)
    );
    std::fs::write(&config, configured).unwrap();
    assert!(
        run(&["approvals", "key-status", "--adapter", "forge"])
            .get("issuer")
            .is_none()
    );
    let initial = run(&["approvals", "key-init", "--adapter", "forge"]);
    let initial_key = initial["issuer"]["keys"][0]["key_id"].as_str().unwrap();
    let revision = initial["issuer"]["revision"].as_str().unwrap();
    f.stop();
    f.start(true);
    let after = run(&["approvals", "key-status", "--adapter", "forge"]);
    assert_eq!(initial, after);
    let local = Store::new(
        &state,
        Some(&f.socket),
        "fixture-instance",
        "fixture-adapter",
        "cfg-1",
    )
    .unwrap();
    local
        .acquire_key()
        .unwrap()
        .with_signer(|_| Ok(()))
        .unwrap();
    // Simulate a lost publication response after exact staged material storage;
    // the recovery itself crosses the production generated CLI/process boundary.
    let seed = Secret(vec![71; 32]);
    let (candidate, candidate_key) = {
        let _lease = Lease::acquire(&state, true).unwrap();
        local
            .stage(
                Some(Uuid::parse_str(revision).unwrap()),
                Some(Uuid::parse_str(initial_key).unwrap()),
                &public_key(&seed).unwrap(),
            )
            .unwrap()
    };
    custody::Store::open_at(candidate.scope, Some(&f.socket))
        .unwrap()
        .write_new_guarded(candidate.version(candidate_key).unwrap(), &seed, || Ok(()))
        .unwrap();
    let recovered = run(&[
        "approvals",
        "key-recover",
        "--adapter",
        "forge",
        "--expected-revision",
        &candidate.view.revision.to_string(),
        "--candidate",
        &candidate_key.to_string(),
    ]);
    let revision = recovered["issuer"]["revision"].as_str().unwrap();
    let recovered_key = candidate_key.to_string();
    let rotated = run(&[
        "approvals",
        "key-rotate",
        "--adapter",
        "forge",
        "--expected-revision",
        revision,
        "--expected-key",
        &recovered_key,
    ]);
    let next = rotated["issuer"]["keys"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["state"] == "active")
        .unwrap()["key_id"]
        .as_str()
        .unwrap();
    let retired = run(&[
        "approvals",
        "key-retire",
        "--adapter",
        "forge",
        "--expected-revision",
        rotated["issuer"]["revision"].as_str().unwrap(),
        "--key",
        initial_key,
    ]);
    let revoked = run(&[
        "approvals",
        "key-revoke",
        "--adapter",
        "forge",
        "--expected-revision",
        retired["issuer"]["revision"].as_str().unwrap(),
        "--expected-key",
        next,
    ]);
    assert!(
        revoked["issuer"]["keys"]
            .as_array()
            .unwrap()
            .iter()
            .all(|k| k["state"] != "active")
    );
    assert!(!state.join("owner.sock").exists());
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn uncertain_deletion_keeps_retirement_fence_and_exact_retry() {
    let f = Fixture::new();
    let store = fixture_store(&f);
    let initial = store.init(None).unwrap();
    let old = initial.active().unwrap().key_id;
    let rotated = store.rotate(initial.revision, old).unwrap();
    let active = rotated.active().unwrap().key_id;
    let record = {
        let _lease = Lease::acquire(&store.path, true).unwrap();
        store.prepare_retirement(rotated.revision, old).unwrap()
    };
    let version = record.version(old).unwrap();
    let custody = custody::Store::open_at(record.scope, Some(&f.socket)).unwrap();
    custody.fail_synchronization(true);
    assert!(matches!(
        custody.delete_guarded(version, || Ok(())),
        Err(custody::Failure::OutcomeUnknown)
    ));
    let status = store.status().unwrap().unwrap();
    assert_eq!(status.key(old).unwrap().state, State::Retiring);
    assert_eq!(status.active().unwrap().key_id, active);
    assert_eq!(
        store.recover(status.revision, old).unwrap_err(),
        Failure::Conflict
    );
    assert_eq!(
        store.retire(rotated.revision, old).unwrap_err(),
        Failure::Conflict
    );
    let finished = store.retire(status.revision, old).unwrap();
    assert_eq!(finished.key(old).unwrap().state, State::Deleted);
    assert_eq!(finished.active().unwrap().key_id, active);
    store
        .acquire_key()
        .unwrap()
        .with_signer(|_| Ok(()))
        .unwrap();
}
