use super::*;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn selection() -> Selection {
    Selection {
        configuration_revision: "cfg-1".into(),
        descriptor_revision: "desc-1".into(),
        executable_selection: "a".repeat(64),
        clock_configuration_sha256: "b".repeat(64),
    }
}
fn store(path: &Path) -> Store {
    Store::new(path, "instance", "adapter").unwrap()
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    fs::directory(root.path(), false, true).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    root
}
// This suite qualifies metadata/serialization only. The actual key coordinator
// owns custody qualification; a retained issuer does not assert an active key.
fn issuer(path: &Path, instance: &str) -> Uuid {
    let m = Metadata::update_approval_keys(path).unwrap();
    m.connection.execute("INSERT INTO registry_instances(instance_id,adapter_id,configuration_revision) VALUES (?1,'adapter','cfg-1')", [instance]).unwrap();
    let id = Uuid::new_v4();
    m.connection
        .execute(
            "INSERT INTO local_approval_issuers VALUES (?1,?2,?3,?4)",
            params![
                id.to_string(),
                instance,
                Uuid::new_v4().to_string(),
                Uuid::new_v4().to_string()
            ],
        )
        .unwrap();
    id
}
fn version(path: &Path) -> i64 {
    Metadata::inspect(path)
        .unwrap()
        .connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}
fn enabled(s: &Store) -> View {
    s.set_admitted(&selection(), vec!["item.write".into()], None)
        .unwrap()
}

#[test]
fn passive_status_and_missing_issuer_never_install_policy() {
    let root = fixture();
    let s = store(root.path());
    assert!(s.status().unwrap().is_none());
    assert_eq!(
        s.set_admitted(&selection(), vec![], None),
        Err(Failure::NotFound)
    );
    assert_eq!(version(root.path()), 3);
    assert!(!root.path().join(LOCK).exists());
    issuer(root.path(), "instance");
    assert!(s.status().unwrap().is_none());
    assert_eq!(version(root.path()), 7);
    assert!(!root.path().join(LOCK).exists());
}

#[test]
fn restart_preserves_identity_revision_principal_and_snapshot() {
    let root = fixture();
    let expected_issuer = issuer(root.path(), "instance");
    let s = store(root.path());
    let first = s
        .set_admitted(&selection(), vec!["z.write".into(), "a.write".into()], None)
        .unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(first.issuer_id, expected_issuer);
    assert_eq!(first.operations, ["a.write", "z.write"]);
    let snapshot = first.snapshot().unwrap();
    drop(s);
    let s = store(root.path());
    assert_eq!(s.status().unwrap(), Some(first.clone()));
    assert_eq!(s.status().unwrap().unwrap().snapshot().unwrap(), snapshot);
    assert_eq!(
        s.set_admitted(&selection(), vec![], None),
        Err(Failure::Conflict)
    );
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(2)),
        Err(Failure::Conflict)
    );
    let second = s.set_admitted(&selection(), vec![], Some(1)).unwrap();
    assert_eq!(second.policy_id, first.policy_id);
    assert_eq!(second.principal(), first.principal());
    assert_ne!(second.snapshot().unwrap(), snapshot);
    assert!(matches!(
        s.acquire(&selection(), "a.write"),
        Err(Failure::NotAdmitted)
    ));
    assert_eq!(version(root.path()), 8);
    drop(Metadata::initialize(root.path()).unwrap());
    assert_eq!(s.status().unwrap(), Some(second));
}

#[test]
fn exact_selections_and_operations_are_required_for_use() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    let first = enabled(&s);
    for change in 0..4 {
        let mut changed = selection();
        match change {
            0 => changed.configuration_revision = "cfg-2".into(),
            1 => changed.descriptor_revision = "desc-2".into(),
            2 => changed.executable_selection = "c".repeat(64),
            _ => changed.clock_configuration_sha256 = "c".repeat(64),
        }
        assert!(matches!(
            s.acquire(&changed, "item.write"),
            Err(Failure::Conflict | Failure::NotAdmitted)
        ));
    }
    assert!(matches!(
        s.acquire(&selection(), "another.write"),
        Err(Failure::NotAdmitted)
    ));
    let current = s.acquire(&selection(), "item.write").unwrap();
    assert_eq!(current.policy().unwrap(), &first);
    drop(current);
    // Key rotation changes no caller namespace or policy revision.
    let m = Metadata::update_approval_keys(root.path()).unwrap();
    m.connection
        .execute(
            "UPDATE local_approval_issuers SET revision=?1",
            [Uuid::new_v4().to_string()],
        )
        .unwrap();
    drop(m);
    assert_eq!(
        s.acquire(&selection(), "item.write")
            .unwrap()
            .policy()
            .unwrap()
            .principal(),
        first.principal()
    );
}

#[test]
fn invalid_publication_cannot_replace_a_valid_policy() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    let first = enabled(&s);
    for ops in [
        vec!["dup".into(), "dup".into()],
        vec!["\nsecret".into()],
        vec!["a".repeat(257)],
        (0..257).map(|n| format!("write.{n}")).collect(),
    ] {
        assert_eq!(
            s.set_admitted(&selection(), ops, Some(1)),
            Err(Failure::InvalidInput)
        );
    }
    for revision in [0, -1, MAX_REVISION + 1] {
        assert_eq!(
            s.set_admitted(&selection(), vec![], Some(revision)),
            Err(Failure::InvalidInput)
        );
    }
    let mut changed = selection();
    changed.configuration_revision = "changed".into();
    assert_eq!(
        s.set_admitted(&changed, vec![], Some(1)),
        Err(Failure::Conflict)
    );
    assert_eq!(s.status().unwrap(), Some(first));
}

#[test]
fn failed_sql_write_preserves_policy_but_lost_ack_requires_observation() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    let first = enabled(&s);
    {
        let m = Metadata::update_approval_policy(root.path()).unwrap();
        m.connection.execute_batch("CREATE TRIGGER test_refuse_policy BEFORE UPDATE ON local_approval_policies BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
    }
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(1)),
        Err(Failure::MetadataUnavailable)
    );
    assert_eq!(s.status().unwrap(), Some(first.clone()));
    {
        let m = Metadata::update_approval_policy(root.path()).unwrap();
        m.connection
            .execute_batch("DROP TRIGGER test_refuse_policy;")
            .unwrap();
    }
    assert_eq!(
        s.publish(&selection(), vec![], Some(1), || Err(Failure::Conflict)),
        Err(Failure::OutcomeUnknown)
    );
    let retained = store(root.path()).status().unwrap().unwrap();
    assert_eq!(retained.policy_id, first.policy_id);
    assert_eq!(retained.revision, 2);
    assert!(retained.operations.is_empty());
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(1)),
        Err(Failure::Conflict)
    );
}

#[test]
fn durable_guards_retain_identity_revision_and_instance_binding() {
    let root = fixture();
    issuer(root.path(), "instance");
    let other = issuer(root.path(), "other");
    let s = store(root.path());
    enabled(&s);
    let m = Metadata::update_approval_policy(root.path()).unwrap();
    for statement in [
        "DELETE FROM local_approval_policies",
        "UPDATE local_approval_policies SET revision=1",
        "UPDATE local_approval_policies SET revision=3",
        "UPDATE local_approval_policies SET owner_uid=owner_uid+1,revision=2",
    ] {
        assert!(m.connection.execute(statement, []).is_err(), "{statement}");
    }
    assert!(
        m.connection
            .execute(
                "UPDATE local_approval_policies SET issuer_id=?1,revision=2",
                [other.to_string()]
            )
            .is_err()
    );
    assert!(
        m.connection
            .execute(
                "INSERT INTO local_approval_policies VALUES (?1,'other',?2,1,?3,'{}','[]')",
                params![
                    Uuid::new_v4().to_string(),
                    s.issuer(&m.connection, None).unwrap().to_string(),
                    fs::uid()
                ]
            )
            .is_err()
    );
}

#[test]
fn concurrent_initial_publication_has_one_winner() {
    let root = fixture();
    issuer(root.path(), "instance");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let start = barrier.clone();
        let path = root.path();
        let a = scope.spawn(move || {
            start.wait();
            store(path).set_admitted(&selection(), vec![], None)
        });
        barrier.wait();
        let b = store(root.path()).set_admitted(&selection(), vec![], None);
        [a.join().unwrap(), b]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(Failure::Conflict))
            .count(),
        1
    );
    assert_eq!(store(root.path()).status().unwrap().unwrap().revision, 1);
}

fn child(path: &Path, action: &str) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local::approval_policy::tests::policy_child",
            "--ignored",
            "--nocapture",
        ])
        .env("CONNECTORS_POLICY_TEST_PATH", path)
        .env("CONNECTORS_POLICY_TEST_ACTION", action)
        .spawn()
        .unwrap()
}
#[test]
#[ignore = "child fixture; launched by process serialization tests"]
fn policy_child() {
    let path = PathBuf::from(std::env::var_os("CONNECTORS_POLICY_TEST_PATH").unwrap());
    let s = store(&path);
    match std::env::var("CONNECTORS_POLICY_TEST_ACTION")
        .unwrap()
        .as_str()
    {
        "blocked" => assert_eq!(
            s.set_admitted(&selection(), vec![], Some(1)),
            Err(Failure::Conflict)
        ),
        "crash-after-commit" => {
            let _ = s.publish(&selection(), vec![], Some(1), || std::process::exit(71));
            panic!("must exit before acknowledgement");
        }
        "hold" => {
            let guard = s.acquire(&selection(), "item.write").unwrap();
            std::fs::write(path.join("child-ready"), b"ready").unwrap();
            std::thread::sleep(Duration::from_secs(30));
            drop(guard);
        }
        _ => panic!("unknown child scenario"),
    }
}

#[test]
fn held_use_blocks_cross_process_revocation_until_release() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    enabled(&s);
    let held = s.acquire(&selection(), "item.write").unwrap();
    let other = s.acquire(&selection(), "item.write").unwrap();
    assert!(child(root.path(), "blocked").wait().unwrap().success());
    assert_eq!(s.status().unwrap().unwrap().revision, 1);
    drop(other);
    drop(held);
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(1))
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn child_exit_releases_lease_and_committed_unacknowledged_revision_survives() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    let first = enabled(&s);
    let mut holder = child(root.path(), "hold");
    let until = Instant::now() + Duration::from_secs(5);
    while !root.path().join("child-ready").exists() {
        if Instant::now() >= until {
            let _ = holder.kill();
            let _ = holder.wait();
            panic!("child never held use");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    holder.kill().unwrap();
    holder.wait().unwrap();
    assert_eq!(
        child(root.path(), "crash-after-commit")
            .wait()
            .unwrap()
            .code(),
        Some(71)
    );
    let retained = store(root.path()).status().unwrap().unwrap();
    assert_eq!(retained.policy_id, first.policy_id);
    assert_eq!(retained.revision, 2);
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(1)),
        Err(Failure::Conflict)
    );
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(2))
            .unwrap()
            .revision,
        3
    );
}

#[test]
fn status_and_snapshot_exclude_private_authority_and_custody_coordinates() {
    let root = fixture();
    issuer(root.path(), "instance");
    let view = enabled(&store(root.path()));
    let m = Metadata::inspect(root.path()).unwrap();
    let scope: String = m
        .connection
        .query_row(
            "SELECT custody_scope FROM local_approval_issuers",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let public = serde_json::to_string(&view).unwrap();
    assert!(!public.contains(&scope));
    assert!(!public.contains(&m.authority().unwrap().to_string()));
    assert!(view.principal().len() <= 128);
}

#[test]
fn inherited_policy_use_is_not_valid_in_a_forked_process() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    enabled(&s);
    let held = s.acquire(&selection(), "item.write").unwrap();
    // SAFETY: the child only checks getpid against a captured integer and exits
    // with _exit; it never allocates, locks, runs destructors or calls SQLite.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        let refused = held.lease.check() == Err(Failure::Conflict);
        // SAFETY: terminate the fork child without running inherited destructors.
        unsafe { libc::_exit(if refused { 0 } else { 1 }) };
    }
    let mut status = 0;
    // SAFETY: wait for our exact child and write to a live status integer.
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), 0);
    assert!(held.policy().is_ok());
}

#[test]
fn exhausted_revision_never_wraps_or_reuses_identity() {
    let root = fixture();
    issuer(root.path(), "instance");
    let s = store(root.path());
    enabled(&s);
    {
        let m = Metadata::update_approval_policy(root.path()).unwrap();
        // Seed the terminal counter directly in this fixture, retaining the
        // production trigger afterwards; no runtime escape permits this jump.
        let trigger: String = m
            .connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='local_approval_policy_revision'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        m.connection
            .execute_batch("DROP TRIGGER local_approval_policy_revision")
            .unwrap();
        m.connection
            .execute(
                "UPDATE local_approval_policies SET revision=?1",
                [MAX_REVISION],
            )
            .unwrap();
        m.connection.execute_batch(&trigger).unwrap();
    }
    let retained = s.status().unwrap();
    assert_eq!(
        s.set_admitted(&selection(), vec![], Some(MAX_REVISION)),
        Err(Failure::Capacity)
    );
    assert_eq!(s.status().unwrap(), retained);
}
