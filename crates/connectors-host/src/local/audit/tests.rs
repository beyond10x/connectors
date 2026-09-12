use super::*;
use crate::local::mutations;
use serde_json::json;
use std::{
    os::unix::fs::PermissionsExt,
    sync::{Arc, Barrier, atomic::Ordering},
};

const NOW: i64 = 1_789_056_000_000;
pub(crate) fn fixture() -> (tempfile::TempDir, Store) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata.connection.execute_batch("INSERT INTO registry_instances VALUES ('alpha','adapter','config',0),('beta','adapter','config',0);
      INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1','{}');
      INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms) VALUES
      ('connection','alpha','profile','{}','scope','revision','fence','live',1,1),
      ('other','alpha','profile','{}','other-scope','revision','fence','live',1,1),
      ('beta-connection','beta','profile','{}','beta-scope','revision','fence','live',1,1);").unwrap();
    drop(metadata);
    let store = Store::new(root.path(), 100_000).unwrap();
    (root, store)
}
pub(crate) fn anchor() -> Anchor {
    Anchor {
        instance_id: "alpha".into(),
        kind: Kind::AdmittedExecution,
        activity: Some(Activity::Invoke),
        hop: Hop::Execution,
        stage: Stage::Admission,
        request_id: Some("request".into()),
        principal_ref: Some("caller".into()),
        operation_id: Some("operation".into()),
        connection_ref: Some("connection".into()),
        descriptor_revision: Some("descriptor".into()),
        recorded_at_ms: NOW,
        attempt_id: None,
    }
}
fn final_value() -> FinalObservation {
    FinalObservation {
        observation_id: Uuid::from_u128(7),
        outcome: Outcome::Success,
        code: None,
        recorded_at_ms: NOW + 1,
    }
}
fn receipt(ack: Acknowledgement) -> Admission {
    match ack {
        Acknowledgement::Execution(value) => value,
        _ => panic!("expected admitted anchor"),
    }
}
fn reference(ack: Acknowledgement) -> Reference {
    match ack {
        Acknowledgement::Execution(value) => value.reference().clone(),
        Acknowledgement::Refusal(value) => value,
    }
}
fn count(path: &Path) -> u32 {
    Metadata::inspect(path)
        .unwrap()
        .connection
        .query_row("SELECT count(*) FROM execution_audits", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn exact_private_identity_is_injective_bounded_and_strictly_decoded() {
    let pairs = [
        ("alpha", "same"),
        ("beta", "same"),
        ("a:b", "c"),
        ("a", "b:c"),
        ("é", "e\u{301}"),
        ("e\u{301}", "é"),
        ("a\0b", "c"),
    ];
    let mut encodings = std::collections::BTreeSet::new();
    for (instance, audit_ref) in pairs {
        let pair = Reference {
            instance: instance.into(),
            audit_ref: audit_ref.into(),
        };
        let encoded = pair.encode_private().unwrap();
        assert!(encodings.insert(encoded.clone()));
        assert_eq!(Reference::decode_private(&encoded).unwrap(), pair);
        assert!(Reference::decode_private(&(encoded.clone() + "=")).is_err());
        let mut raw =
            base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &encoded)
                .unwrap();
        raw.push(0);
        assert!(
            Reference::decode_private(&base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                raw
            ))
            .is_err()
        );
        assert!(Reference::decode_private(&encoded[..encoded.len() - 3]).is_err());
    }
    let pair = Reference {
        instance: "é".repeat(64),
        audit_ref: "x".repeat(128),
    };
    assert!(pair.encode_private().unwrap().len() <= 512);
    assert_eq!(
        Reference::decode_private(&pair.encode_private().unwrap()).unwrap(),
        pair
    );
    for pair in [
        Reference {
            instance: "x".repeat(129),
            audit_ref: "ok".into(),
        },
        Reference {
            instance: "ok".into(),
            audit_ref: "é".repeat(65),
        },
        Reference {
            instance: "".into(),
            audit_ref: "ok".into(),
        },
    ] {
        assert_eq!(pair.encode_private(), Err(Failure::InvalidInput));
    }
    for value in ["", "%%%", &"x".repeat(513)] {
        assert_eq!(Reference::decode_private(value), Err(Failure::InvalidInput));
    }
}

#[test]
fn same_public_ref_is_independent_per_instance_and_gateway_and_leaf() {
    let (_root, store) = fixture();
    let mut gateway = anchor();
    gateway.hop = Hop::Gateway;
    let a = reference(store.insert(&gateway, "same".into()).unwrap());
    let mut leaf = anchor();
    leaf.instance_id = "beta".into();
    leaf.connection_ref = Some("beta-connection".into());
    let b = reference(store.insert(&leaf, "same".into()).unwrap());
    store.append(&a, &final_value()).unwrap();
    assert!(
        store
            .observe(&b)
            .unwrap()
            .unwrap()
            .final_observation
            .is_none()
    );
    let mut unknown = final_value();
    unknown.outcome = Outcome::Unknown;
    store.append(&b, &unknown).unwrap();
    assert_eq!(
        store.observe(&a).unwrap().unwrap().final_observation,
        Some(final_value())
    );
    assert_eq!(
        store.observe(&b).unwrap().unwrap().final_observation,
        Some(unknown)
    );
    assert!(matches!(
        store.insert(&anchor(), "same".into()),
        Err(Failure::Conflict)
    ));
    let missing = Reference {
        instance: "gamma".into(),
        audit_ref: "same".into(),
    };
    assert!(store.observe(&missing).unwrap().is_none());
}

#[test]
fn anchor_failure_or_lost_ack_never_returns_an_admission_receipt() {
    let (root, store) = fixture();
    store.fault.store(1, Ordering::SeqCst);
    assert!(matches!(
        store.insert(&anchor(), "rolled-back".into()),
        Err(Failure::MetadataUnavailable)
    ));
    assert_eq!(count(root.path()), 0);
    store.fault.store(2, Ordering::SeqCst);
    assert!(matches!(
        store.insert(&anchor(), "lost-ack".into()),
        Err(Failure::OutcomeUnknown)
    ));
    assert_eq!(count(root.path()), 1);
    let reopened = Store::new(root.path(), 100_000).unwrap();
    let stored = reopened
        .observe(&Reference {
            instance: "alpha".into(),
            audit_ref: "lost-ack".into(),
        })
        .unwrap()
        .unwrap();
    assert!(stored.final_observation.is_none());
    // Record is observation only: no API converts it to an Admission.
    let read = anchor();
    let admission = receipt(reopened.anchor(&read).unwrap());
    assert_eq!(admission.facts(), &read);
    reopened.confirm(admission, &read).unwrap();
}

#[test]
fn early_refusal_and_malformed_or_mismatched_facts_cannot_satisfy_the_gate() {
    let (root, store) = fixture();
    let early = Anchor {
        kind: Kind::EarlyRefusal,
        stage: Stage::Decoding,
        activity: None,
        principal_ref: None,
        request_id: None,
        operation_id: None,
        connection_ref: None,
        descriptor_revision: None,
        ..anchor()
    };
    assert!(matches!(
        store.anchor(&early).unwrap(),
        Acknowledgement::Refusal(_)
    ));
    let mut invalid = anchor();
    invalid.principal_ref = None;
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    invalid = anchor();
    invalid.operation_id = None;
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    invalid = anchor();
    invalid.activity = Some(Activity::Describe);
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    invalid.operation_id = None;
    let admitted = receipt(store.anchor(&invalid).unwrap());
    store.confirm(admitted, &invalid).unwrap();
    let mut admitted = receipt(store.anchor(&anchor()).unwrap());
    admitted.process = std::process::id().wrapping_add(1);
    assert_eq!(store.confirm(admitted, &anchor()), Err(Failure::Conflict));
    let admitted = receipt(store.anchor(&anchor()).unwrap());
    invalid = anchor();
    invalid.request_id = Some("different".into());
    assert_eq!(store.confirm(admitted, &invalid), Err(Failure::Conflict));
    let (_other_root, other_store) = fixture();
    let admitted = receipt(store.anchor(&anchor()).unwrap());
    // Initialize its audit schema without sharing the authority identity.
    other_store.anchor(&anchor()).unwrap();
    assert_eq!(
        other_store.confirm(admitted, &anchor()),
        Err(Failure::Conflict)
    );
    let admitted = receipt(store.anchor(&anchor()).unwrap());
    store.append(admitted.reference(), &final_value()).unwrap();
    assert_eq!(store.confirm(admitted, &anchor()), Err(Failure::Conflict));
    assert!(count(root.path()) > 0);
}

#[test]
fn concurrent_finalization_has_one_immutable_winner_and_exact_retry() {
    let (root, store) = fixture();
    let reference = reference(store.anchor(&anchor()).unwrap());
    let barrier = Arc::new(Barrier::new(8));
    let results = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|index| {
                let barrier = barrier.clone();
                let reference = &reference;
                let store = &store;
                scope.spawn(move || {
                    let mut value = final_value();
                    value.observation_id = Uuid::from_u128(100 + index);
                    barrier.wait();
                    (value.clone(), store.append(reference, &value))
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results.iter().filter(|(_, result)| result.is_ok()).count(),
        1
    );
    let winner = results
        .iter()
        .find(|(_, result)| result.is_ok())
        .unwrap()
        .0
        .clone();
    let original = store.observe(&reference).unwrap().unwrap();
    for _ in 0..3 {
        assert_eq!(store.append(&reference, &winner).unwrap(), original);
    }
    let variations: [fn(&mut FinalObservation); 4] = [
        |v| v.observation_id = Uuid::new_v4(),
        |v| v.outcome = Outcome::Error,
        |v| v.code = Some("changed".into()),
        |v| v.recorded_at_ms += 1,
    ];
    for variation in variations {
        let mut value = winner.clone();
        variation(&mut value);
        assert_eq!(store.append(&reference, &value), Err(Failure::Conflict));
    }
    assert_eq!(count(root.path()), 1);
    assert_eq!(
        Store::new(root.path(), 1)
            .unwrap()
            .observe(&reference)
            .unwrap(),
        Some(original)
    );
}

#[test]
fn failed_final_append_preserves_live_answer_and_unknown_ack_recovers_exactly() {
    let (_root, store) = fixture();
    let reference = reference(store.anchor(&anchor()).unwrap());
    let known_live_result = json!({"effect":"applied","iid":7});
    let value = final_value();
    store.fault.store(1, Ordering::SeqCst);
    assert_eq!(
        store.append(&reference, &value),
        Err(Failure::MetadataUnavailable)
    );
    assert_eq!(known_live_result["effect"], "applied");
    assert_eq!(value, final_value());
    assert!(
        store
            .observe(&reference)
            .unwrap()
            .unwrap()
            .final_observation
            .is_none()
    );
    store.fault.store(2, Ordering::SeqCst);
    assert_eq!(
        store.append(&reference, &value),
        Err(Failure::OutcomeUnknown)
    );
    let original = store.observe(&reference).unwrap().unwrap();
    assert_eq!(original.final_observation, Some(value.clone()));
    assert_eq!(store.append(&reference, &value).unwrap(), original);
    assert_eq!(known_live_result["effect"], "applied");
}

#[test]
fn capacity_is_atomic_per_instance_without_eviction_or_append_restriction() {
    let (root, _) = fixture();
    let store = Store::new(root.path(), 4).unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let results = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let barrier = barrier.clone();
                let store = &store;
                scope.spawn(move || {
                    barrier.wait();
                    // Eight threads contend for four slots against one SQLite file. This
                    // test asserts that capacity is atomic, not how the store behaves when
                    // its busy timeout is exhausted — and under a loaded machine running
                    // the rest of the suite beside it, a thread can exhaust that timeout
                    // and return MetadataUnavailable, which is neither of the two answers
                    // capacity has. Retry that one failure mode so the assertion below
                    // measures what it is about. See story:host-suite-load-sensitivity.
                    let mut attempt = 0;
                    loop {
                        match store.anchor(&anchor()) {
                            Err(Failure::MetadataUnavailable) if attempt < 16 => {
                                attempt += 1;
                                std::thread::sleep(Duration::from_millis(50));
                            }
                            outcome => break outcome,
                        }
                    }
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    let mut refs = Vec::new();
    for result in results {
        match result {
            Ok(value) => refs.push(reference(value)),
            Err(Failure::Capacity) => {}
            Err(value) => panic!("unexpected {value:?}"),
        }
    }
    assert_eq!(refs.len(), 4);
    for reference in &refs {
        store.append(reference, &final_value()).unwrap();
    }
    assert!(matches!(store.anchor(&anchor()), Err(Failure::Capacity)));
    let mut beta = anchor();
    beta.instance_id = "beta".into();
    beta.connection_ref = None;
    store.anchor(&beta).unwrap();
    assert_eq!(count(root.path()), 5);
    let smaller = Store::new(root.path(), 1).unwrap();
    for reference in &refs {
        assert!(
            smaller
                .observe(reference)
                .unwrap()
                .unwrap()
                .final_observation
                .is_some()
        );
    }
    assert!(Store::new(root.path(), 0).is_err());
    assert!(Store::new(root.path(), 100_001).is_err());
}

#[test]
fn byte_bounds_safe_shapes_and_aggregate_budget_are_enforced() {
    let (_root, store) = fixture();
    let fields: [fn(&mut Anchor, String); 6] = [
        |a, v| a.instance_id = v,
        |a, v| a.request_id = Some(v),
        |a, v| a.principal_ref = Some(v),
        |a, v| a.connection_ref = Some(v),
        |a, v| a.descriptor_revision = Some(v),
        |a, v| a.operation_id = Some(v),
    ];
    for (index, change) in fields.into_iter().enumerate() {
        let limit = if index == 5 { 256 } else { 128 };
        for value in [
            String::new(),
            "x".repeat(limit + 1),
            "é".repeat(limit / 2 + 1),
        ] {
            let mut a = anchor();
            change(&mut a, value);
            assert!(matches!(store.anchor(&a), Err(Failure::InvalidInput)));
        }
    }
    let reference = reference(store.anchor(&anchor()).unwrap());
    for code in [
        "x".repeat(65),
        "é".into(),
        "unsafe\nline".into(),
        String::new(),
    ] {
        let mut value = final_value();
        value.code = Some(code);
        assert_eq!(store.append(&reference, &value), Err(Failure::InvalidInput));
    }
    let mut value = final_value();
    value.code = Some("x".repeat(64));
    store.append(&reference, &value).unwrap();
    let mut huge = anchor();
    huge.request_id = Some("\0".repeat(128));
    huge.principal_ref = Some("\0".repeat(128));
    huge.operation_id = Some("\0".repeat(256));
    huge.descriptor_revision = Some("\0".repeat(128));
    assert!(matches!(store.anchor(&huge), Err(Failure::Capacity)));
    let mut malformed = anchor();
    malformed.recorded_at_ms = -1;
    assert!(matches!(
        store.anchor(&malformed),
        Err(Failure::InvalidInput)
    ));
    let mut value = final_value();
    value.observation_id = Uuid::nil();
    assert_eq!(store.append(&reference, &value), Err(Failure::InvalidInput));
    value = final_value();
    value.recorded_at_ms = -1;
    assert_eq!(store.append(&reference, &value), Err(Failure::InvalidInput));
    let mut raw = serde_json::to_value(anchor()).unwrap();
    raw["credential"] = json!("never accepted");
    assert!(serde_json::from_value::<Anchor>(raw).is_err());
    let mut raw = serde_json::to_value(final_value()).unwrap();
    raw["provider_body"] = json!("never accepted");
    assert!(serde_json::from_value::<FinalObservation>(raw).is_err());
}

#[derive(Clone, Copy)]
struct TestClock;
impl mutations::Clock for TestClock {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        Ok(mutations::ClockInterval {
            lower_unix_ms: NOW,
            upper_unix_ms: NOW,
        })
    }
}
fn candidate() -> mutations::Candidate {
    mutations::Candidate {
        namespace: mutations::Namespace {
            receiver_instance: "alpha".into(),
            tenant: None,
            realm: None,
            caller: "caller".into(),
            executor: None,
            origin: mutations::Origin::Direct,
        },
        fingerprint: mutations::Fingerprint {
            operation: mutations::OperationRef {
                instance: "alpha".into(),
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
        request_id: "request".into(),
        approval: mutations::Approval::NotRequired,
    }
}

#[test]
fn retained_cross_owner_references_are_checked_and_never_cascade_deleted() {
    let (root, store) = fixture();
    let ledger =
        mutations::Store::new(root.path(), TestClock, mutations::Limits::default()).unwrap();
    let candidate = candidate();
    let prepared = match ledger.prepare(&candidate).unwrap() {
        mutations::Preparation::Prepared(value) => value,
        _ => panic!(),
    };
    let attempt = prepared.reference();
    let mut a = anchor();
    a.attempt_id = Some(attempt.attempt_id);
    let admission = receipt(store.anchor(&a).unwrap());
    let reference = store.confirm(admission, &a).unwrap();
    ledger.open_dispatch(prepared).unwrap().consume().unwrap();
    let known = ledger
        .settle(
            attempt,
            &mutations::Outcome::Applied(json!({"applied":true})),
        )
        .unwrap();
    store.append(&reference, &final_value()).unwrap();
    let original = store.observe(&reference).unwrap();
    let mut invalid = a.clone();
    invalid.instance_id = "beta".into();
    invalid.connection_ref = Some("beta-connection".into());
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    invalid = a.clone();
    invalid.connection_ref = Some("other".into());
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    invalid = a.clone();
    invalid.attempt_id = Some(Uuid::new_v4());
    assert!(matches!(store.anchor(&invalid), Err(Failure::InvalidInput)));
    // Same-instance attempt-only correlation is permitted; it still grants no use.
    a.connection_ref = None;
    store.anchor(&a).unwrap();
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata.connection.execute("UPDATE registry_connections SET state='revoked',revoked_at_ms=1 WHERE connection_ref='connection'",[]).unwrap();
    assert!(
        metadata
            .connection
            .execute(
                "DELETE FROM mutation_attempts WHERE attempt_id=?1",
                [attempt.attempt_id.to_string()]
            )
            .is_err()
    );
    assert!(
        metadata
            .connection
            .execute(
                "DELETE FROM registry_connections WHERE connection_ref='connection'",
                []
            )
            .is_err()
    );
    // Model the owning index's completed retirement while retaining attempts.
    metadata
        .connection
        .execute(
            "UPDATE mutation_keys SET state='expired' WHERE reservation_id=?1",
            [known.reservation_id.unwrap().to_string()],
        )
        .unwrap();
    drop(metadata);
    assert_eq!(store.observe(&reference).unwrap(), original);
    assert_eq!(
        ledger.observe(attempt).unwrap().state,
        mutations::State::Completed
    );
}

#[test]
fn passive_old_schema_inspection_never_installs_audit_and_migration_preserves_history() {
    let (root, store) = fixture();
    let reference = Reference {
        instance: "alpha".into(),
        audit_ref: "absent".into(),
    };
    assert_eq!(store.observe(&reference), Err(Failure::MetadataUnavailable));
    let metadata = Metadata::inspect(root.path()).unwrap();
    let authority = metadata.authority().unwrap();
    let migrations: Vec<(i64, String)> = metadata
        .connection
        .prepare("SELECT version,digest FROM schema_migrations ORDER BY version")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(migrations.len(), 3);
    drop(metadata);
    drop(Metadata::initialize(root.path()).unwrap());
    assert_eq!(store.observe(&reference), Err(Failure::MetadataUnavailable));
    store.anchor(&anchor()).unwrap();
    let metadata = Metadata::inspect(root.path()).unwrap();
    assert_eq!(metadata.authority().unwrap(), authority);
    let current: Vec<(i64, String)> = metadata
        .connection
        .prepare("SELECT version,digest FROM schema_migrations ORDER BY version")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(&current[..3], &migrations);
    assert_eq!(current.len(), 5);
    drop(metadata);
    assert!(store.observe(&reference).unwrap().is_none());
    let ledger =
        mutations::Store::new(root.path(), TestClock, mutations::Limits::default()).unwrap();
    assert!(matches!(
        ledger.prepare(&candidate()).unwrap(),
        mutations::Preparation::Prepared(_)
    ));
    drop(Metadata::initialize(root.path()).unwrap());
    assert!(store.observe(&reference).unwrap().is_none());
}

#[test]
fn corrupted_or_unavailable_metadata_is_not_absence_or_recovered_acknowledgement() {
    let (root, store) = fixture();
    let reference = reference(store.anchor(&anchor()).unwrap());
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata
        .connection
        .execute(
            "UPDATE execution_audits SET connection_ref='other' WHERE audit_ref=?1",
            [&reference.audit_ref],
        )
        .unwrap();
    drop(metadata);
    assert_eq!(store.observe(&reference), Err(Failure::MetadataUnavailable));
    assert_eq!(
        store.append(&reference, &final_value()),
        Err(Failure::MetadataUnavailable)
    );
    std::fs::set_permissions(
        root.path().join("metadata.sqlite3"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        store.observe(&Reference {
            instance: "alpha".into(),
            audit_ref: "missing".into()
        }),
        Err(Failure::MetadataUnavailable)
    );
}

#[test]
fn abrupt_process_exits_preserve_acknowledgement_boundaries_without_resend() {
    for phase in [
        "anchor-before",
        "anchor-after",
        "final-before",
        "final-after",
    ] {
        let (root, store) = fixture();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local::audit::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("CONNECTORS_AUDIT_CRASH_ROOT", root.path())
            .env("CONNECTORS_AUDIT_CRASH_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let reference = Reference {
            instance: "alpha".into(),
            audit_ref: "crash".into(),
        };
        let observed = store.observe(&reference).unwrap();
        if phase == "anchor-before" {
            assert!(observed.is_none());
        } else {
            assert_eq!(
                observed.unwrap().final_observation,
                if phase == "final-after" {
                    Some(final_value())
                } else {
                    None
                }
            );
        }
        let effects = root.path().join("simulated-effect");
        assert_eq!(effects.exists(), phase.starts_with("final"));
        if phase.starts_with("final") {
            store.append(&reference, &final_value()).unwrap();
            store.append(&reference, &final_value()).unwrap();
            assert_eq!(std::fs::read_to_string(effects).unwrap(), "one");
        }
    }
}

#[test]
#[ignore = "subprocess entry point exercised by abrupt_process_exits"]
fn crash_child() {
    let root = PathBuf::from(std::env::var_os("CONNECTORS_AUDIT_CRASH_ROOT").unwrap());
    let phase = std::env::var("CONNECTORS_AUDIT_CRASH_PHASE").unwrap();
    let store = Store::new(&root, 100_000).unwrap();
    if phase.starts_with("anchor") {
        store.fault.store(
            if phase.ends_with("before") { 7 } else { 8 },
            Ordering::SeqCst,
        );
    }
    let admission = receipt(store.insert(&anchor(), "crash".into()).unwrap());
    let reference = store.confirm(admission, &anchor()).unwrap();
    let file = std::fs::File::create_new(root.join("simulated-effect")).unwrap();
    use std::io::Write;
    (&file).write_all(b"one").unwrap();
    file.sync_all().unwrap();
    store.fault.store(
        if phase.ends_with("before") { 7 } else { 8 },
        Ordering::SeqCst,
    );
    store.append(&reference, &final_value()).unwrap();
    panic!("fault did not exit");
}

#[test]
fn recovering_append_reuses_exact_fields_and_reads_a_lost_acknowledgement() {
    for (fault, appends, reads) in [(1, 2, 1), (2, 1, 1), (9, 2, 2)] {
        let (root, store) = fixture();
        let reference = reference(store.anchor(&anchor()).unwrap());
        let observation = final_value();
        store.fault.store(fault, Ordering::SeqCst);
        // A recovery budget generous enough that a busy scheduler cannot consume it.
        // These tests assert what recovery does, not how fast it is; the one test that
        // does assert expiry passes an already-elapsed instant. See
        // story:host-suite-load-sensitivity.
        let record = store
            .append_recovering(
                &reference,
                &observation,
                Instant::now() + Duration::from_secs(60),
            )
            .unwrap();
        assert_eq!(record.final_observation, Some(observation.clone()));
        assert_eq!(
            *store.appends.lock().unwrap(),
            vec![observation.clone(); appends]
        );
        assert_eq!(store.reads.load(Ordering::SeqCst), reads);
        assert_eq!(count(root.path()), 1);
        let restored = Store::new(root.path(), 100_000).unwrap();
        assert_eq!(restored.observe(&reference).unwrap(), Some(record.clone()));
        assert_eq!(restored.append(&reference, &observation).unwrap(), record);
    }
}

#[test]
fn recovering_append_never_substitutes_a_different_final_observation() {
    for field in 0..4 {
        let (_root, store) = fixture();
        let reference = reference(store.anchor(&anchor()).unwrap());
        let original = store.append(&reference, &final_value()).unwrap();
        let mut changed = final_value();
        match field {
            0 => changed.observation_id = Uuid::new_v4(),
            1 => changed.outcome = Outcome::Unknown,
            2 => changed.code = Some("outcome_unknown".into()),
            3 => changed.recorded_at_ms += 1,
            _ => unreachable!(),
        }
        store.appends.lock().unwrap().clear();
        assert_eq!(
            store.append_recovering(
                &reference,
                &changed,
                Instant::now() + Duration::from_secs(60)
            ),
            Err(Failure::Conflict)
        );
        assert_eq!(*store.appends.lock().unwrap(), vec![changed]);
        assert_eq!(store.reads.load(Ordering::SeqCst), 1);
        assert_eq!(store.observe(&reference).unwrap(), Some(original));
    }
}

#[test]
fn expired_recovery_budget_performs_no_extra_storage_calls() {
    for (fault, failure) in [
        (1, Failure::MetadataUnavailable),
        (2, Failure::OutcomeUnknown),
    ] {
        let (root, store) = fixture();
        let reference = reference(store.anchor(&anchor()).unwrap());
        let observation = final_value();
        store.fault.store(fault, Ordering::SeqCst);
        assert_eq!(
            store.append_recovering(&reference, &observation, Instant::now()),
            Err(failure)
        );
        assert_eq!(*store.appends.lock().unwrap(), vec![observation.clone()]);
        assert_eq!(store.reads.load(Ordering::SeqCst), 0);
        let restored = Store::new(root.path(), 100_000).unwrap();
        assert_eq!(
            restored
                .observe(&reference)
                .unwrap()
                .unwrap()
                .final_observation,
            (fault == 2).then(|| observation.clone())
        );
        assert_eq!(
            restored
                .append_recovering(
                    &reference,
                    &observation,
                    Instant::now() + Duration::from_secs(60)
                )
                .unwrap()
                .final_observation,
            Some(observation)
        );
    }
}

#[test]
fn recovering_append_requires_a_readable_exact_original_anchor() {
    let (root, store) = fixture();
    let reference = reference(store.anchor(&anchor()).unwrap());
    let other = Reference {
        instance: "beta".into(),
        audit_ref: reference.audit_ref.clone(),
    };
    assert_eq!(
        store.append_recovering(
            &other,
            &final_value(),
            Instant::now() + Duration::from_secs(60)
        ),
        Err(Failure::NotFound)
    );
    assert_eq!(store.appends.lock().unwrap().len(), 1);
    assert!(
        store
            .observe(&reference)
            .unwrap()
            .unwrap()
            .final_observation
            .is_none()
    );
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata
        .connection
        .execute("UPDATE execution_audits SET record_json='{}'", [])
        .unwrap();
    drop(metadata);
    store.appends.lock().unwrap().clear();
    store.reads.store(0, Ordering::SeqCst);
    assert_eq!(
        store.append_recovering(
            &reference,
            &final_value(),
            Instant::now() + Duration::from_secs(60)
        ),
        Err(Failure::MetadataUnavailable)
    );
    assert_eq!(store.appends.lock().unwrap().len(), 1);
    assert_eq!(store.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn recovering_append_stops_after_one_failed_retry() {
    let (_root, store) = fixture();
    let reference = reference(store.anchor(&anchor()).unwrap());
    let observation = final_value();
    store.fault.store(10, Ordering::SeqCst);
    assert_eq!(
        store.append_recovering(
            &reference,
            &observation,
            Instant::now() + Duration::from_secs(60)
        ),
        Err(Failure::MetadataUnavailable)
    );
    assert_eq!(*store.appends.lock().unwrap(), vec![observation; 2]);
    assert_eq!(store.reads.load(Ordering::SeqCst), 2);
    assert!(
        store
            .observe(&reference)
            .unwrap()
            .unwrap()
            .final_observation
            .is_none()
    );
}
