use super::lifecycle::custody_version;
use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    sync::{Arc, Barrier},
};

const NOW: u64 = 1_788_998_400_000;

#[test]
fn production_clock_samples_after_locking_and_still_rejects_regression() {
    let (root, _) = fixture();
    let registry = Registry::with_system_clock(root.path());
    // A call timestamp older than a committed observation is harmless when the
    // physical clock is sampled inside this production transaction.
    registry
        .list(
            "fixture-instance",
            "fixture-adapter",
            "fixture-config",
            PageOptions {
                limit: 10,
                cursor: None,
            },
            0,
            false,
        )
        .unwrap();
    registry
        .list(
            "fixture-instance",
            "fixture-adapter",
            "fixture-config",
            PageOptions {
                limit: 10,
                cursor: None,
            },
            0,
            false,
        )
        .unwrap();
    let metadata = Metadata::update(root.path(), false).unwrap();
    metadata
        .connection
        .execute(
            "UPDATE registry_clock SET last_seen_ms=?1",
            [timestamp(connectors_sdk::now_ms() + 60_000).unwrap()],
        )
        .unwrap();
    drop(metadata);
    assert!(matches!(
        registry.list(
            "fixture-instance",
            "fixture-adapter",
            "fixture-config",
            PageOptions {
                limit: 10,
                cursor: None
            },
            0,
            false
        ),
        Err(Failure::MetadataUnavailable)
    ));
}

#[test]
fn lost_publication_acknowledgement_is_resolved_without_repeating_capture() {
    let (root, registry) = fixture();
    let (claim, candidate) = prepared(&registry, "one", NOW);
    let receipt = custody::WrittenVersion::fixture(candidate.version());
    let stored = registry.acknowledge(candidate, receipt, NOW).unwrap();
    registry
        .lose_next_commit
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(registry.publish(stored, NOW), Err(Failure::OutcomeUnknown));
    drop(registry);
    let registry = Registry::new(root.path());
    let status = registry
        .acquisition_status(
            "fixture-instance",
            "fixture-adapter",
            claim.acquisition_reference(),
            NOW + 1,
        )
        .unwrap();
    assert_eq!(status.state, "completed");
    let connection = status.connection.unwrap();
    assert_eq!(
        describe(&registry, &connection, NOW + 1).state,
        State::Ready
    );
    assert_eq!(registry.fail(&claim, NOW + 1), Err(Failure::Conflict));
    assert!(matches!(
        registry.consume(claim.acquisition, NOW + 1),
        Err(Failure::Conflict)
    ));
    let use_once = registry
        .capture_read(
            &binding(),
            &connection,
            &BTreeSet::new(),
            NOW + 1,
            NOW + 1000,
        )
        .unwrap();
    let replay = use_once.fixture_replay();
    let dispatched = registry.dispatch_read(use_once, NOW + 1).unwrap();
    assert!(matches!(
        registry.dispatch_read(replay, NOW + 1),
        Err(Failure::Conflict)
    ));
    registry.release_read(dispatched, NOW + 2).unwrap();
}

#[test]
fn concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence() {
    for _ in 0..8 {
        let (root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let connection = publish_fixture(&registry, candidate, NOW);
        let revision = describe(&registry, &connection, NOW).revision;
        let repair = registry
            .begin_repair(&binding(), &connection, &revision, NOW)
            .unwrap();
        let claim = registry.consume(repair, NOW).unwrap();
        let candidate = registry
            .prepare(&claim, baseline("one", NOW), 12, NOW)
            .unwrap();
        let receipt = custody::WrittenVersion::fixture(candidate.version());
        let stored = registry.acknowledge(candidate, receipt, NOW).unwrap();
        let captured = registry
            .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let a = Registry::new(root.path());
        let start = barrier.clone();
        let publish = std::thread::spawn(move || {
            start.wait();
            a.publish(stored, NOW)
        });
        let b = Registry::new(root.path());
        let start = barrier.clone();
        let dispatch = std::thread::spawn(move || {
            start.wait();
            b.dispatch_read(captured, NOW)
        });
        barrier.wait();
        registry
            .revoke(
                "fixture-instance",
                "fixture-adapter",
                &connection,
                &revision,
                NOW,
            )
            .unwrap();
        assert!(matches!(
            publish.join().unwrap(),
            Ok(_) | Err(Failure::Conflict | Failure::Revoked)
        ));
        // A read admitted before revoke may already be in flight; later dispatch
        // cannot inherit that fact or roll back the terminal connection state.
        assert!(matches!(
            dispatch.join().unwrap(),
            Ok(_) | Err(Failure::Conflict | Failure::Revoked)
        ));
        assert_eq!(
            describe(&Registry::new(root.path()), &connection, NOW).state,
            State::Revoked
        );
    }
}

fn fixture() -> (tempfile::TempDir, Registry) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    super::super::filesystem::directory(root.path(), false, true).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    let registry = Registry::new(root.path());
    (root, registry)
}
fn binding() -> Binding {
    Binding {
        instance_id: "fixture-instance".into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "config-1".into(),
        provider_authority: "https://fixture.invalid/fixed-authority".into(),
        profile: StaticProfile {
            id: "fixture-token".into(),
            revision: "profile-1".into(),
            purpose: Purpose::DelegatedUser,
            subject: Subject::User,
            minimum_scopes: BTreeSet::from(["read".to_owned()]),
            evidence_lifetime_ms: 60_000,
        },
    }
}
fn baseline(subject: &str, now: u64) -> ValidatedBaseline {
    ValidatedBaseline {
        identity: ExternalIdentity {
            kind: "fixture-account".into(),
            subject: subject.into(),
        },
        granted_scopes: Some(BTreeSet::from(["read".into()])),
        credential_expires_at_ms: Some(now + 3_600_000),
        collected_at_ms: now,
        valid_until_ms: now + 60_000,
    }
}
fn prepared(registry: &Registry, subject: &str, now: u64) -> (Claim, PreparedCandidate) {
    let acquisition = registry.begin(&binding(), now).unwrap();
    let claim = registry.consume(acquisition, now).unwrap();
    let prepared = registry
        .prepare(&claim, baseline(subject, now), 12, now)
        .unwrap();
    (claim, prepared)
}
fn publish_fixture(registry: &Registry, prepared: PreparedCandidate, now: u64) -> String {
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, now).unwrap();
    registry.publish(stored, now).unwrap()
}
fn describe(registry: &Registry, connection: &str, now: u64) -> ObservedConnection {
    registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            connection,
            now,
            true,
        )
        .unwrap()
}

#[test]
fn allocation_custody_and_publication_are_distinct_and_restartable() {
    let (root, registry) = fixture();
    let (claim, prepared) = prepared(&registry, "one", NOW);
    let acquisition = claim.acquisition_reference().to_owned();
    assert!(
        registry
            .list(
                "fixture-instance",
                "fixture-adapter",
                "config-1",
                PageOptions {
                    limit: 100,
                    cursor: None
                },
                NOW,
                true
            )
            .unwrap()
            .connections
            .is_empty()
    );
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, NOW).unwrap();
    let restarted = Registry::new(root.path());
    assert!(
        restarted
            .list(
                "fixture-instance",
                "fixture-adapter",
                "config-1",
                PageOptions {
                    limit: 100,
                    cursor: None
                },
                NOW,
                true
            )
            .unwrap()
            .connections
            .is_empty()
    );
    assert_eq!(
        restarted
            .acquisition_status("fixture-instance", "fixture-adapter", &acquisition, NOW)
            .unwrap()
            .state,
        "pending"
    );
    let connection = registry.publish(stored, NOW).unwrap();
    assert_eq!(
        describe(&restarted, &connection, NOW + 1).state,
        State::Ready
    );
    let status = restarted
        .acquisition_status("fixture-instance", "fixture-adapter", &acquisition, NOW + 1)
        .unwrap();
    assert_eq!(status.state, "completed");
    assert_eq!(status.connection.as_deref(), Some(connection.as_str()));
    assert_eq!(
        describe(&restarted, &connection, NOW + 60_000).state,
        State::Pending
    );
    assert!(matches!(
        restarted.describe(
            "another-instance",
            "fixture-adapter",
            "config-1",
            &connection,
            NOW + 60_000,
            true
        ),
        Err(Failure::NotFound)
    ));
    assert!(matches!(
        restarted.describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &connection,
            NOW + 1,
            true
        ),
        Err(Failure::MetadataUnavailable)
    ));
}

#[test]
fn identity_scope_and_expiry_failures_preserve_the_usable_generation() {
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    let original = describe(&registry, &connection, NOW);
    let attempt = registry
        .begin_repair(&binding(), &connection, &original.revision, NOW + 1)
        .unwrap();
    let claim = registry.consume(attempt, NOW + 1).unwrap();
    assert!(matches!(
        registry.prepare(&claim, baseline("different", NOW + 1), 12, NOW + 1),
        Err(Failure::IdentityMismatch)
    ));
    let mut missing_scope = baseline("one", NOW + 1);
    missing_scope.granted_scopes = Some(BTreeSet::new());
    assert!(matches!(
        registry.prepare(&claim, missing_scope, 12, NOW + 1),
        Err(Failure::InsufficientScope)
    ));
    registry.fail(&claim, NOW + 1).unwrap();
    assert_eq!(
        describe(&registry, &connection, NOW + 2).state,
        State::Ready
    );
    assert_eq!(
        describe(&registry, &connection, NOW + 2).revision,
        original.revision
    );
    assert!(matches!(
        registry.capture_read(
            &binding(),
            &connection,
            &BTreeSet::from(["write".into()]),
            NOW + 2,
            NOW + 1000
        ),
        Err(Failure::InsufficientScope)
    ));
    assert_eq!(
        registry
            .describe(
                "fixture-instance",
                "fixture-adapter",
                "config-1",
                &connection,
                NOW + 3,
                false
            )
            .unwrap()
            .state,
        State::CustodyUnavailable
    );
    assert_eq!(
        describe(&registry, &connection, NOW + 3_600_000).state,
        State::ReauthorizationRequired
    );
}

#[test]
fn competing_repairs_have_one_winner_and_old_dispatch_is_fenced() {
    let (root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    let revision = describe(&registry, &connection, NOW).revision;
    let old_use = registry
        .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let mut candidates = Vec::new();
    for _ in 0..2 {
        let attempt = registry
            .begin_repair(&binding(), &connection, &revision, NOW + 1)
            .unwrap();
        let claim = registry.consume(attempt, NOW + 1).unwrap();
        let prepared = registry
            .prepare(&claim, baseline("one", NOW + 1), 12, NOW + 1)
            .unwrap();
        let receipt = custody::WrittenVersion::fixture(prepared.version());
        candidates.push(registry.acknowledge(prepared, receipt, NOW + 1).unwrap());
    }
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = candidates
        .into_iter()
        .map(|candidate| {
            let path = root.path().to_owned();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                Registry::new(&path).publish(candidate, NOW + 2)
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(Failure::Conflict))
            .count(),
        1
    );
    assert_eq!(describe(&registry, &connection, NOW + 3).revision, revision);
    assert!(matches!(
        registry.dispatch_read(old_use, NOW + 3),
        Err(Failure::Conflict)
    ));
}

#[test]
fn terminal_revoke_invalidates_repair_and_dispatch_without_custody() {
    let (root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    let revision = describe(&registry, &connection, NOW).revision;
    let before = registry
        .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let acquired = registry
        .begin_repair(&binding(), &connection, &revision, NOW + 1)
        .unwrap();
    let claim = registry.consume(acquired, NOW + 1).unwrap();
    let candidate = registry
        .prepare(&claim, baseline("one", NOW + 1), 12, NOW + 1)
        .unwrap();
    let receipt = custody::WrittenVersion::fixture(candidate.version());
    let stored = registry.acknowledge(candidate, receipt, NOW + 1).unwrap();
    let revoked = registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &connection,
            &revision,
            NOW + 2,
        )
        .unwrap();
    assert_ne!(revoked, revision);
    assert!(registry.publish(stored, NOW + 3).is_err());
    assert!(registry.dispatch_read(before, NOW + 3).is_err());
    let restarted = Registry::new(root.path());
    assert_eq!(
        describe(&restarted, &connection, NOW + 3).state,
        State::Revoked
    );
    assert_eq!(
        restarted
            .revoke(
                "fixture-instance",
                "fixture-adapter",
                &connection,
                &revision,
                NOW + 4
            )
            .unwrap(),
        revoked
    );
    assert!(matches!(
        restarted.begin_repair(&binding(), &connection, &revoked, NOW + 4),
        Err(Failure::Revoked)
    ));
}

#[test]
fn cursors_are_scoped_bounded_and_invalidated_by_metadata_change() {
    let (_root, registry) = fixture();
    for subject in ["a", "b", "c"] {
        let (_, prepared) = prepared(&registry, subject, NOW);
        publish_fixture(&registry, prepared, NOW);
    }
    let first = registry
        .list(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            PageOptions {
                limit: 2,
                cursor: None,
            },
            NOW,
            true,
        )
        .unwrap();
    let cursor = first.next_cursor.as_deref().unwrap();
    assert_eq!(first.connections.len(), 2);
    assert!(matches!(
        registry.list(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            PageOptions {
                limit: 1,
                cursor: Some(cursor)
            },
            NOW,
            true
        ),
        Err(Failure::StaleCursor)
    ));
    let last = registry
        .list(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            PageOptions {
                limit: 2,
                cursor: Some(cursor),
            },
            NOW,
            true,
        )
        .unwrap();
    assert_eq!(last.connections.len(), 1);
    assert!(last.next_cursor.is_none());
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &first.connections[0].reference,
            &first.connections[0].revision,
            NOW + 1,
        )
        .unwrap();
    assert!(matches!(
        registry.list(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            PageOptions {
                limit: 2,
                cursor: Some(cursor)
            },
            NOW + 1,
            true
        ),
        Err(Failure::StaleCursor)
    ));
}

#[test]
fn abandoned_candidates_require_acknowledged_retirement_and_full_retention() {
    let (_root, registry) = fixture();
    let (claim, candidate) = prepared(&registry, "one", NOW);
    registry.write_allowed(&candidate, NOW).unwrap();
    assert!(registry.retirements(NOW).unwrap().is_empty());
    registry.fail(&claim, NOW + 1).unwrap();
    assert!(registry.write_allowed(&candidate, NOW + 1).is_err());
    assert!(registry.retirements(NOW + RETENTION_MS).unwrap().is_empty());
    let retired = registry.retirements(NOW + RETENTION_MS + 1).unwrap();
    assert_eq!(retired.len(), 1);
    assert!(retired[0].version() == candidate.version());
}

#[test]
fn receipt_identity_and_known_missing_material_cannot_be_substituted() {
    let (root, registry) = fixture();
    let (_, first) = prepared(&registry, "one", NOW);
    let wrong = registry
        .transaction(NOW, false, |_, authority, _| {
            custody_version(authority, &new_id(), &new_id())
        })
        .unwrap();
    assert!(matches!(
        registry.acknowledge(first, custody::WrittenVersion::fixture(wrong), NOW),
        Err(Failure::Conflict)
    ));
    let (_, second) = prepared(&registry, "two", NOW);
    let connection = publish_fixture(&registry, second, NOW);
    let captured = registry
        .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    registry
        .invalidate_read(&captured, InvalidCredential::Missing, NOW + 1)
        .unwrap();
    assert_eq!(
        describe(&Registry::new(root.path()), &connection, NOW + 1).state,
        State::ReauthorizationRequired
    );
    assert!(registry.dispatch_read(captured, NOW + 1).is_err());
}
