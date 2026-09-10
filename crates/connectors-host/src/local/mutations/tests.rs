use super::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc, Barrier, Mutex,
    atomic::{AtomicUsize, Ordering},
};

const NOW: i64 = 1_789_056_000_000;
#[derive(Clone)]
struct TestClock(Arc<Mutex<Result<ClockInterval>>>);
impl Clock for TestClock {
    fn now(&self) -> Result<ClockInterval> {
        *self.0.lock().unwrap()
    }
}
impl TestClock {
    fn set(&self, lower: i64, upper: i64) {
        *self.0.lock().unwrap() = Ok(ClockInterval {
            lower_unix_ms: lower,
            upper_unix_ms: upper,
        });
    }
    fn unavailable(&self) {
        *self.0.lock().unwrap() = Err(Failure::ClockUnavailable);
    }
}
fn fixture() -> (tempfile::TempDir, Store<TestClock>, Candidate) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    let metadata = Metadata::update_mutations(root.path()).unwrap();
    // Synthetic admitted-registry coordinates, not provider/custody evidence.
    metadata.connection.execute_batch("INSERT INTO registry_instances VALUES ('instance','adapter','config',0),('second','adapter','config',0);
      INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1','{}');
      INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms) VALUES
      ('connection','instance','profile','{}','scope','revision','fence','live',1,1),
      ('other','instance','profile','{}','other-scope','other-revision','other-fence','live',1,1),
      ('second-connection','second','profile','{}','second-scope','revision','fence','live',1,1);").unwrap();
    drop(metadata);
    let clock = TestClock(Arc::new(Mutex::new(Ok(ClockInterval {
        lower_unix_ms: NOW,
        upper_unix_ms: NOW + 2000,
    }))));
    let store = Store::new(root.path(), clock, Limits::default()).unwrap();
    (root, store, candidate())
}
fn candidate() -> Candidate {
    Candidate {
        namespace: Namespace {
            receiver_instance: "instance".into(),
            tenant: None,
            realm: None,
            caller: "caller".into(),
            executor: None,
            origin: Origin::Direct,
        },
        fingerprint: Fingerprint {
            operation: OperationRef {
                instance: "instance".into(),
                adapter: "adapter".into(),
                operation: "operation".into(),
            },
            connection_ref: "connection".into(),
            connection_revision: "revision".into(),
            contract_ref: "operations/v1alpha1".into(),
            profile: "mutation".into(),
            descriptor_revision: "descriptor".into(),
            configuration_revision: "config".into(),
            canonicalization_version: "adapter-v1-canonical-json".into(),
            input_digest: "a".repeat(64),
            route: None,
        },
        caller_key: Some("key".into()),
        request_id: "original-request".into(),
        approval: Approval::NotRequired,
    }
}
fn prepared(store: &Store<TestClock>, candidate: &Candidate) -> Prepared {
    match store.prepare(candidate).unwrap() {
        Preparation::Prepared(p) => p,
        Preparation::Existing(_) => panic!("unexpected existing attempt"),
    }
}
fn count(root: &Path, table: &str) -> usize {
    let metadata = Metadata::inspect(root).unwrap();
    metadata
        .connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
            r.get::<_, u32>(0)
        })
        .unwrap() as usize
}
fn existing(store: &Store<TestClock>, candidate: &Candidate) -> Observation {
    store.lookup(candidate).unwrap().unwrap()
}

#[test]
fn concurrent_duplicate_prepare_has_one_live_receipt_and_original_correlation() {
    let (root, store, candidate) = fixture();
    let store = Arc::new(store);
    let barrier = Arc::new(Barrier::new(8));
    let results: Vec<_> = std::thread::scope(|scope| {
        let tasks: Vec<_> = (0..8)
            .map(|index| {
                let barrier = barrier.clone();
                let store = &store;
                let mut c = candidate.clone();
                c.request_id = format!("request-{index}");
                scope.spawn(move || {
                    barrier.wait();
                    store.prepare(&c).unwrap()
                })
            })
            .collect();
        tasks.into_iter().map(|t| t.join().unwrap()).collect()
    });
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Preparation::Prepared(_)))
            .count(),
        1
    );
    let owner = results
        .into_iter()
        .find_map(|r| match r {
            Preparation::Prepared(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let reference = owner.reference();
    assert_eq!(count(root.path(), "mutation_attempts"), 1);
    assert_eq!(count(root.path(), "mutation_keys"), 1);
    assert_eq!(existing(&store, &candidate).reference, reference);
    let receipt = store.open_dispatch(owner).unwrap();
    let calls = AtomicUsize::new(0);
    assert_eq!(receipt.consume().unwrap(), reference);
    calls.fetch_add(1, Ordering::SeqCst);
    let outcome = Outcome::Applied(json!({"iid": 7}));
    let completed = store.settle(reference, &outcome).unwrap();
    let mut replay = candidate.clone();
    replay.request_id = "retry-request".into();
    replay.approval = Approval::Required {
        reference: "spent-or-expired-proof".into(),
    };
    match store.prepare(&replay).unwrap() {
        Preparation::Existing(value) => assert_eq!(value, completed),
        _ => panic!("duplicate gained a handle"),
    }
    assert_ne!(completed.request_id, replay.request_id);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn every_fingerprint_coordinate_conflicts_including_exact_route_revisions() {
    let (_root, store, mut candidate) = fixture();
    candidate.namespace.origin = Origin::Federated {
        gateway_instance: "gateway".into(),
    };
    candidate.fingerprint.route = Some(Route {
        gateway_instance: "gateway".into(),
        route_id: "route".into(),
        route_revision: "r1".into(),
    });
    let _owner = prepared(&store, &candidate);
    let changes: [fn(&mut Fingerprint); 11] = [
        |f| f.operation.adapter.push('2'),
        |f| f.operation.operation.push('2'),
        |f| f.connection_ref.push('2'),
        |f| f.connection_revision.push('2'),
        |f| f.contract_ref.push('2'),
        |f| f.profile.push('2'),
        |f| f.descriptor_revision.push('2'),
        |f| f.configuration_revision.push('2'),
        |f| f.input_digest = "b".repeat(64),
        |f| f.route.as_mut().unwrap().route_id.push('2'),
        |f| f.route.as_mut().unwrap().route_revision.push('2'),
    ];
    for change in changes {
        let mut changed = candidate.clone();
        change(&mut changed.fingerprint);
        assert_eq!(store.lookup(&changed), Err(Failure::Conflict));
        assert!(matches!(store.prepare(&changed), Err(Failure::Conflict)));
    }
    let mut unsupported = candidate.clone();
    unsupported.fingerprint.canonicalization_version = "future-version".into();
    assert_eq!(store.lookup(&unsupported), Err(Failure::InvalidInput));
    unsupported.fingerprint = candidate.fingerprint.clone();
    unsupported
        .fingerprint
        .route
        .as_mut()
        .unwrap()
        .gateway_instance = "untrusted-gateway".into();
    assert_eq!(store.lookup(&unsupported), Err(Failure::InvalidInput));
}

#[test]
fn namespace_is_injective_with_explicit_nulls_origin_and_opaque_keys() {
    let (root, store, candidate) = fixture();
    let mut variants = vec![candidate.clone()];
    for field in [
        "tenant", "realm", "executor", "caller", "origin", "receiver", "key",
    ] {
        let mut changed = candidate.clone();
        match field {
            "tenant" => changed.namespace.tenant = Some("default".into()),
            "realm" => changed.namespace.realm = Some("default".into()),
            "executor" => changed.namespace.executor = Some("executor".into()),
            "caller" => changed.namespace.caller = "other".into(),
            "origin" => {
                changed.namespace.origin = Origin::Federated {
                    gateway_instance: "gateway".into(),
                };
                changed.fingerprint.route = Some(Route {
                    gateway_instance: "gateway".into(),
                    route_id: "route".into(),
                    route_revision: "1".into(),
                });
            }
            "receiver" => {
                changed.namespace.receiver_instance = "second".into();
                changed.fingerprint.operation.instance = "second".into();
                changed.fingerprint.connection_ref = "second-connection".into();
            }
            _ => changed.caller_key = Some(" key\0\n".into()),
        }
        variants.push(changed);
    }
    let mut refs = std::collections::HashSet::new();
    for c in &variants {
        refs.insert(prepared(&store, c).reference().attempt_id);
    }
    assert_eq!(refs.len(), variants.len());
    assert_eq!(count(root.path(), "mutation_keys"), variants.len());
    let mut a = candidate.clone();
    a.namespace.tenant = Some("a|b".into());
    a.namespace.realm = Some("c".into());
    let mut b = candidate.clone();
    b.namespace.tenant = Some("a".into());
    b.namespace.realm = Some("b|c".into());
    assert_ne!(a.encode().unwrap().0, b.encode().unwrap().0);
}

#[test]
fn atomic_prepare_faults_never_return_a_handle_or_leave_a_half_reservation() {
    for fault in [1, 2, 3] {
        let (root, store, candidate) = fixture();
        store.fault.store(fault, Ordering::SeqCst);
        assert!(matches!(
            store.prepare(&candidate),
            Err(Failure::MetadataUnavailable | Failure::OutcomeUnknown)
        ));
        let rows = usize::from(fault == 2);
        assert_eq!(count(root.path(), "mutation_attempts"), rows);
        assert_eq!(count(root.path(), "mutation_keys"), rows);
        let clock = store.clock.clone();
        drop(store);
        let reopened = Store::new(root.path(), clock, Limits::default()).unwrap();
        if let Some(original) = reopened.lookup(&candidate).unwrap() {
            assert_eq!(original.state, State::Prepared);
            let result = reopened.recover(original.reference).unwrap();
            assert_eq!(result.state, State::Aborted);
            assert!(matches!(
                reopened.prepare(&candidate),
                Ok(Preparation::Existing(_))
            ));
        }
    }
}

#[test]
fn abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt() {
    let (_root, store, mut candidate) = fixture();
    for round in 0..24 {
        candidate.caller_key = Some(format!("key-{round}"));
        let owner = prepared(&store, &candidate);
        let reference = owner.reference();
        let barrier = Barrier::new(2);
        let (gate, abort) = std::thread::scope(|scope| {
            let gate = scope.spawn(|| {
                barrier.wait();
                store.open_dispatch(owner)
            });
            let abort = scope.spawn(|| {
                barrier.wait();
                store.abort(reference, json!({"cause":"cancelled"}))
            });
            (gate.join().unwrap(), abort.join().unwrap())
        });
        assert_ne!(gate.is_ok(), abort.is_ok());
        let expected = if let Ok(receipt) = gate {
            assert_eq!(receipt.consume().unwrap(), reference);
            State::Indeterminate
        } else {
            State::Aborted
        };
        assert_eq!(store.recover(reference).unwrap().state, expected);
        assert!(matches!(
            store.prepare(&candidate),
            Ok(Preparation::Existing(_))
        ));
    }
}

#[test]
fn ambiguous_gate_is_unknown_and_ambiguous_terminal_preserves_known_live_answer() {
    for fault in [1, 2] {
        let (root, store, candidate) = fixture();
        let owner = prepared(&store, &candidate);
        let reference = owner.reference();
        store.fault.store(fault, Ordering::SeqCst);
        assert!(store.open_dispatch(owner).is_err());
        let clock = store.clock.clone();
        drop(store);
        let store = Store::new(root.path(), clock, Limits::default()).unwrap();
        assert_eq!(
            store.recover(reference).unwrap().state,
            if fault == 1 {
                State::Aborted
            } else {
                State::Indeterminate
            }
        );
    }
    for fault in [1, 2] {
        let (root, store, candidate) = fixture();
        let owner = prepared(&store, &candidate);
        let reference = owner.reference();
        store.open_dispatch(owner).unwrap().consume().unwrap();
        let live_answer = Outcome::Applied(json!({"iid":9}));
        store.fault.store(fault, Ordering::SeqCst);
        assert!(store.settle(reference, &live_answer).is_err());
        assert!(matches!(live_answer, Outcome::Applied(_)));
        let clock = store.clock.clone();
        drop(store);
        let store = Store::new(root.path(), clock, Limits::default()).unwrap();
        let after = store.recover(reference).unwrap();
        assert_eq!(
            after.state,
            if fault == 1 {
                State::Indeterminate
            } else {
                State::Completed
            }
        );
        assert_eq!(existing(&store, &candidate), after);
    }
}

#[test]
fn terminal_first_fact_is_immutable_and_unknown_never_expires() {
    let (_root, store, candidate) = fixture();
    let owner = prepared(&store, &candidate);
    let reference = owner.reference();
    assert_eq!(
        store.settle(reference, &Outcome::Applied(json!({}))),
        Err(Failure::Conflict)
    );
    store.open_dispatch(owner).unwrap().consume().unwrap();
    let answer = Outcome::Unknown(json!({"cause":"lost_response"}));
    let unknown = store.settle(reference, &answer).unwrap();
    assert_eq!(store.settle(reference, &answer).unwrap(), unknown);
    assert_eq!(
        store.settle(reference, &Outcome::Applied(json!({}))),
        Err(Failure::Conflict)
    );
    assert_eq!(store.abort(reference, json!({})), Err(Failure::Conflict));
    store.clock.set(i64::MAX, i64::MAX);
    assert!(
        !store
            .expire(&candidate, reference, unknown.reservation_id.unwrap())
            .unwrap()
    );
    assert_eq!(existing(&store, &candidate), unknown);
    assert_eq!(store.recover(reference).unwrap(), unknown);
}

#[test]
fn retention_uses_trusted_bounds_and_cas_cannot_retire_a_replacement() {
    let (root, store, candidate) = fixture();
    let owner = prepared(&store, &candidate);
    let reference = owner.reference();
    let aborted = store
        .abort(reference, json!({"cause":"cancelled"}))
        .unwrap();
    assert!(matches!(store.open_dispatch(owner), Err(Failure::Conflict)));
    let expiry = NOW + 2000 + RETENTION_MS;
    assert_eq!(aborted.settled_at_ms, Some(NOW + 2000));
    assert_eq!(aborted.replay_expires_at_ms, Some(expiry));
    let id = aborted.reservation_id.unwrap();
    store.clock.set(expiry - 1, expiry + 1999);
    assert!(!store.expire(&candidate, reference, id).unwrap());
    store.clock.unavailable();
    assert_eq!(
        store.expire(&candidate, reference, id),
        Err(Failure::ClockUnavailable)
    );
    assert_eq!(existing(&store, &candidate), aborted);
    let clock = store.clock.clone();
    drop(store);
    let store = Store::new(root.path(), clock, Limits::default()).unwrap();
    store.clock.set(NOW, NOW + 2000);
    assert_eq!(
        store.expire(&candidate, reference, id),
        Err(Failure::ClockUnavailable)
    );
    store.clock.set(expiry, expiry + 2000);
    assert!(store.expire(&candidate, reference, id).unwrap());
    assert!(store.lookup(&candidate).unwrap().is_none());
    let replacement = prepared(&store, &candidate);
    assert_ne!(replacement.reference(), reference);
    assert!(!store.expire(&candidate, reference, id).unwrap());
    assert_eq!(
        existing(&store, &candidate).reference,
        replacement.reference()
    );
    assert_eq!(store.observe(reference).unwrap().state, State::Aborted);
}

#[test]
fn malformed_or_missing_clock_never_settles_a_known_result_or_shortens_retention() {
    for (lower, upper) in [
        (-1, 0),
        (NOW, NOW - 1),
        (NOW, NOW + 4001),
        (i64::MAX - 1, i64::MAX),
    ] {
        let (_root, store, candidate) = fixture();
        let owner = prepared(&store, &candidate);
        let reference = owner.reference();
        store.open_dispatch(owner).unwrap().consume().unwrap();
        store.clock.set(lower, upper);
        assert_eq!(
            store.settle(reference, &Outcome::Refused(json!({"code":"conflict"}))),
            Err(Failure::ClockUnavailable)
        );
        assert_eq!(existing(&store, &candidate).state, State::Dispatching);
        store.clock.unavailable();
        assert_eq!(
            store.recover(reference).unwrap().state,
            State::Indeterminate
        );
    }
}

#[test]
fn capacity_and_corrupt_or_unavailable_reads_do_not_become_absence() {
    let (root, mut store, candidate) = fixture();
    store.limits.attempts_per_instance = 1;
    let owner = prepared(&store, &candidate);
    let mut next = candidate.clone();
    next.caller_key = Some("next".into());
    assert!(matches!(store.prepare(&next), Err(Failure::Capacity)));
    assert!(matches!(
        store.prepare(&candidate),
        Ok(Preparation::Existing(_))
    ));
    store.limits.result_bytes = 64;
    assert_eq!(
        store.abort(owner.reference(), json!("x".repeat(65))),
        Err(Failure::Capacity)
    );
    assert_eq!(existing(&store, &candidate).state, State::Prepared);
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata
        .connection
        .execute("UPDATE mutation_keys SET fingerprint='corrupt'", [])
        .unwrap();
    drop(metadata);
    assert_eq!(
        store.observe(owner.reference()),
        Err(Failure::MetadataUnavailable)
    );
    assert_eq!(count(root.path(), "mutation_attempts"), 1);
    let missing = Store::new(
        &root.path().join("missing"),
        store.clock.clone(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        missing.lookup(&candidate),
        Err(Failure::MetadataUnavailable)
    );
    assert!(!root.path().join("missing").exists());
}

#[test]
fn connection_fences_authority_and_process_identity_refuse_stale_handles() {
    for change in [
        "UPDATE registry_connections SET publication_fence='new' WHERE connection_ref='connection'",
        "UPDATE registry_connections SET public=0 WHERE connection_ref='connection'",
        "UPDATE registry_connections SET semantic_revision='new' WHERE connection_ref='connection'",
        "UPDATE registry_instances SET configuration_revision='new' WHERE instance_id='instance'",
        "UPDATE registry_connections SET state='revoked',revoked_at_ms=2 WHERE connection_ref='connection'",
    ] {
        let (root, store, candidate) = fixture();
        let owner = prepared(&store, &candidate);
        let reference = owner.reference();
        let metadata = Metadata::update(root.path(), false).unwrap();
        metadata.connection.execute_batch(change).unwrap();
        drop(metadata);
        assert!(matches!(
            store.open_dispatch(owner),
            Err(Failure::BindingChanged)
        ));
        assert_eq!(store.observe(reference).unwrap().state, State::Prepared);
    }
    let (_root, store, candidate) = fixture();
    let mut owner = prepared(&store, &candidate);
    owner.process = std::process::id().wrapping_add(1);
    assert!(matches!(store.open_dispatch(owner), Err(Failure::Conflict)));
    let mut c = candidate.clone();
    c.caller_key = Some("second".into());
    let owner = prepared(&store, &c);
    let reference = owner.reference();
    let mut receipt = store.open_dispatch(owner).unwrap();
    receipt.process = std::process::id().wrapping_add(1);
    assert_eq!(receipt.consume(), Err(Failure::Conflict));
    assert_eq!(
        store.observe(AttemptRef {
            authority: Uuid::new_v4(),
            ..reference
        }),
        Err(Failure::Conflict)
    );
}

#[test]
fn unkeyed_attempts_still_require_the_gate_and_retain_outcome() {
    let (root, store, mut candidate) = fixture();
    candidate.caller_key = None;
    let a = prepared(&store, &candidate);
    let b = prepared(&store, &candidate);
    assert_ne!(a.reference(), b.reference());
    let reference = a.reference();
    store.open_dispatch(a).unwrap().consume().unwrap();
    let result = store
        .settle(reference, &Outcome::Refused(json!({"code":"denied"})))
        .unwrap();
    assert_eq!(result.state, State::Failed);
    assert_eq!(result.reservation_id, None);
    assert_eq!(count(root.path(), "mutation_keys"), 0);
    assert_eq!(store.recover(reference).unwrap(), result);
}

#[test]
fn version_three_inspection_is_read_only_and_admitted_upgrade_preserves_authority() {
    let (root, store, candidate) = fixture();
    let authority;
    let old_digests;
    {
        let metadata = Metadata::update(root.path(), false).unwrap();
        authority = metadata.authority().unwrap();
        old_digests = metadata
            .connection
            .prepare(
                "SELECT version,digest FROM schema_migrations WHERE version<=3 ORDER BY version",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        metadata.connection.execute_batch("DROP TABLE mutation_keys; DROP TABLE mutation_attempts; DROP TABLE mutation_clock; DELETE FROM schema_migrations WHERE version=4; PRAGMA user_version=3;").unwrap();
    }
    assert_eq!(store.lookup(&candidate), Err(Failure::MetadataUnavailable));
    // Ordinary setup/re-entry and admitted read metadata operations retain v3.
    drop(Metadata::initialize(root.path()).unwrap());
    drop(Metadata::update(root.path(), true).unwrap());
    {
        let metadata = Metadata::inspect(root.path()).unwrap();
        assert_eq!(metadata.authority().unwrap(), authority);
        assert_eq!(
            metadata
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
    let owner = prepared(&store, &candidate);
    assert_eq!(owner.reference().authority, authority);
    let metadata = Metadata::inspect(root.path()).unwrap();
    let after = metadata
        .connection
        .prepare("SELECT version,digest FROM schema_migrations WHERE version<=3 ORDER BY version")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(after, old_digests);
    assert_eq!(
        metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(metadata.connection.query_row("SELECT semantic_revision FROM registry_connections WHERE connection_ref='connection'",[],|r|r.get::<_,String>(0)).unwrap(),"revision");
}

#[test]
fn abrupt_process_exit_preserves_each_durable_boundary_without_resend() {
    for stage in ["prepared", "gate", "effect", "terminal"] {
        let (root, store, candidate) = fixture();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local::mutations::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("CONNECTORS_MUTATION_CRASH_PATH", root.path())
            .env("CONNECTORS_MUTATION_CRASH_STAGE", stage)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(23),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let original = existing(&store, &candidate);
        let recovered = store.recover(original.reference).unwrap();
        let expected = match stage {
            "prepared" => State::Aborted,
            "terminal" => State::Completed,
            _ => State::Indeterminate,
        };
        assert_eq!(recovered.state, expected);
        assert!(matches!(
            store.prepare(&candidate),
            Ok(Preparation::Existing(_))
        ));
        let effect = root.path().join("test-owned-effect");
        assert_eq!(effect.exists(), matches!(stage, "effect" | "terminal"));
        if effect.exists() {
            assert_eq!(std::fs::read(effect).unwrap(), b"one");
        }
        assert_eq!(count(root.path(), "mutation_attempts"), 1);
    }
}

#[test]
#[ignore = "subprocess entry point, exercised by abrupt_process_exit"]
fn crash_child() {
    let Some(path) = std::env::var_os("CONNECTORS_MUTATION_CRASH_PATH") else {
        return;
    };
    let stage = std::env::var("CONNECTORS_MUTATION_CRASH_STAGE").unwrap();
    // Use the same typed candidate but the parent's existing authority. This
    // test process has no provider or credential access.
    let candidate = candidate();
    let clock = TestClock(Arc::new(Mutex::new(Ok(ClockInterval {
        lower_unix_ms: NOW,
        upper_unix_ms: NOW + 2000,
    }))));
    let path = PathBuf::from(path);
    let store = Store::new(&path, clock, Limits::default()).unwrap();
    let owner = prepared(&store, &candidate);
    let reference = owner.reference();
    if stage == "prepared" {
        std::process::exit(23);
    }
    let receipt = store.open_dispatch(owner).unwrap();
    if stage == "gate" {
        std::process::exit(23);
    }
    receipt.consume().unwrap();
    use std::io::Write;
    let mut effect = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path.join("test-owned-effect"))
        .unwrap();
    effect.write_all(b"one").unwrap();
    effect.sync_all().unwrap();
    if stage == "effect" {
        std::process::exit(23);
    }
    store
        .settle(reference, &Outcome::Applied(json!({"effect":"one"})))
        .unwrap();
    std::process::exit(23);
}
