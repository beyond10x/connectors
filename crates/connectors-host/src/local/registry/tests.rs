use super::lifecycle::custody_version;
use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    sync::{Arc, Barrier},
};

const NOW: u64 = 1_788_998_400_000;

#[test]
fn approval_target_is_passive_but_requires_exact_retained_admission() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let old = describe(&registry, &reference, NOW + 60_001);
    assert_eq!(old.state, State::Pending);
    let metadata = Metadata::inspect(root.path()).unwrap();
    let watermark: i64 = metadata
        .connection
        .query_row("SELECT last_seen_ms FROM registry_clock", [], |r| r.get(0))
        .unwrap();
    let path = root.path().join("metadata.sqlite3");
    drop(metadata);
    // Independent read-only observer; do not retain the metadata owner's
    // lifecycle lease while the tested coordinator opens its own handle.
    let observer =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let changes: i64 = observer
        .query_row("PRAGMA data_version", [], |r| r.get(0))
        .unwrap();
    // A system-clock registry must not sample that clock on this path. The
    // retained evidence is expired, but it still identifies this exact target.
    let passive = Registry::with_system_clock(root.path());
    assert_eq!(
        passive
            .approval_target(&binding(), &reference, &BTreeSet::new())
            .unwrap(),
        old.revision
    );
    let mut changed = binding();
    changed.provider_authority.push_str("/other");
    assert_eq!(
        passive.approval_target(&changed, &reference, &BTreeSet::new()),
        Err(Failure::Conflict)
    );
    assert_eq!(
        passive.approval_target(&binding(), &reference, &BTreeSet::from(["write".into()])),
        Err(Failure::InsufficientScope)
    );
    let after = Metadata::inspect(root.path()).unwrap();
    assert_eq!(
        after
            .connection
            .query_row("SELECT last_seen_ms FROM registry_clock", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        watermark
    );
    drop(after);
    assert_eq!(
        observer
            .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        changes
    );
    drop(observer);
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &old.revision,
            NOW + 60_001,
        )
        .unwrap();
    assert_eq!(
        passive.approval_target(&binding(), &reference, &BTreeSet::new()),
        Err(Failure::Revoked)
    );
}

#[test]
fn revalidation_profile_discovers_identity_without_persisting_clock_or_granting_admission() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let expired = NOW + 60_001;
    assert_eq!(
        describe(&registry, &reference, expired).state,
        State::Pending
    );
    let metadata = Metadata::inspect(root.path()).unwrap();
    let watermark: i64 = metadata
        .connection
        .query_row("SELECT last_seen_ms FROM registry_clock", [], |r| r.get(0))
        .unwrap();
    drop(metadata);
    let passive = Registry::with_system_clock(root.path());
    assert_eq!(
        passive.revalidation_profile("fixture-instance", "fixture-adapter", &reference, &revision),
        Ok(binding().profile.id)
    );
    assert_eq!(
        passive.revalidation_profile("other-instance", "fixture-adapter", &reference, &revision),
        Err(Failure::NotFound)
    );
    assert_eq!(
        passive.revalidation_profile("fixture-instance", "fixture-adapter", &reference, "wrong"),
        Err(Failure::Conflict)
    );
    let metadata = Metadata::inspect(root.path()).unwrap();
    assert_eq!(
        metadata
            .connection
            .query_row("SELECT last_seen_ms FROM registry_clock", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        watermark
    );
    drop(metadata);
    let revoked_revision = registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            expired,
        )
        .unwrap();
    assert_eq!(
        passive.revalidation_profile("fixture-instance", "fixture-adapter", &reference, &revision),
        Err(Failure::Conflict)
    );
    assert_eq!(
        passive.revalidation_profile(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revoked_revision
        ),
        Err(Failure::Revoked)
    );
}

#[test]
fn approval_target_refuses_known_invalid_material_without_custody_access() {
    for (known_invalid, expected) in [
        (false, Failure::MetadataUnavailable),
        (true, Failure::NotReady),
    ] {
        let (root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let reference = publish_fixture(&registry, candidate, NOW);
        if known_invalid {
            // Positive invalidity uses the owning read-use transition.
            let captured = registry
                .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
                .unwrap();
            registry
                .invalidate_read(&captured, InvalidCredential::Invalid, NOW)
                .unwrap();
        } else {
            // The original active/deleted contradiction cannot replace the
            // retained material through the derived projection.
            let mut metadata = Metadata::update(root.path(), false).unwrap();
            metadata
                .connection
                .execute("UPDATE registry_materials SET deleted=1,byte_size=NULL,retirement_fence='7c9a4817-95ef-46ae-8363-ac98e1fe042d',retired_at_ms=1788998400000,delete_not_before_ms=1788998400000", [])
                .unwrap();
            assert!(metadata.persist().is_err());
            drop(metadata);
            assert!(
                registry
                    .approval_target(&binding(), &reference, &BTreeSet::new())
                    .is_ok()
            );
            // Independent physical corruption must remain unavailable at the
            // same passive boundary, rather than turning into absence.
            let physical =
                rusqlite::Connection::open(root.path().join("metadata.sqlite3")).unwrap();
            assert_eq!(
                physical
                    .execute(
                        "UPDATE connectors_er_events SET data='{}' WHERE global_seq=(SELECT max(global_seq) FROM connectors_er_events WHERE event_name='er.recorded_entry')",
                        [],
                    )
                    .unwrap(),
                1
            );
        }
        assert_eq!(
            registry.approval_target(&binding(), &reference, &BTreeSet::new()),
            Err(expected)
        );
    }
}

#[test]
fn revalidation_after_expiry_preserves_material_and_recovers_unknown_acknowledgement() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let version = candidate.version();
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let expired = NOW + 60_001;
    assert_eq!(
        describe(&registry, &reference, expired).state,
        State::Pending
    );
    assert_eq!(
        registry.admit_read(&binding(), &reference, &BTreeSet::new(), expired),
        Err(Failure::NotReady)
    );
    let capture = registry
        .capture_revalidation(&binding(), &reference, &revision, expired, expired + 30_000)
        .unwrap();
    assert!(capture.version() == version);
    let dispatched = registry.dispatch_revalidation(capture, expired).unwrap();
    registry
        .lose_next_commit
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        registry.finish_revalidation(dispatched, Ok(baseline("one", expired)), expired),
        Err(Failure::OutcomeUnknown)
    );
    drop(registry);
    let registry = Registry::new(root.path());
    let observed = describe(&registry, &reference, expired);
    assert_eq!(observed.state, State::Ready);
    assert_eq!(observed.revision, revision);
    let read = registry
        .capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            expired,
            expired + 1000,
        )
        .unwrap();
    assert!(read.version() == version);
    registry.dispatch_read(read, expired).unwrap();
    assert_eq!(
        describe(&registry, &reference, NOW + 3_600_000).state,
        State::ReauthorizationRequired
    );
}

#[test]
fn revalidation_orders_competing_recollection_repair_revoke_and_pending_reads() {
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let capture = || {
        registry
            .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
            .unwrap()
    };
    let first = registry.dispatch_revalidation(capture(), NOW).unwrap();
    let second = registry.dispatch_revalidation(capture(), NOW).unwrap();
    let read = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let repair = registry
        .begin_repair(&binding(), &reference, &revision, NOW)
        .unwrap();
    registry
        .finish_revalidation(first, Ok(baseline("one", NOW)), NOW)
        .unwrap();
    assert_eq!(
        registry.finish_revalidation(second, Ok(baseline("one", NOW)), NOW),
        Err(Failure::Conflict)
    );
    assert!(matches!(
        registry.dispatch_read(read, NOW),
        Err(Failure::Conflict)
    ));
    assert!(matches!(
        registry.consume(repair, NOW),
        Err(Failure::Conflict)
    ));
    let pending = registry.dispatch_revalidation(capture(), NOW).unwrap();
    let repair = registry
        .begin_repair(&binding(), &reference, &revision, NOW)
        .unwrap();
    let claim = registry.consume(repair, NOW).unwrap();
    let replacement = registry
        .prepare(&claim, baseline("one", NOW), 12, NOW)
        .unwrap();
    publish_fixture(&registry, replacement, NOW);
    assert_eq!(
        registry.finish_revalidation(pending, Ok(baseline("one", NOW)), NOW),
        Err(Failure::Conflict)
    );
    let pending = registry.dispatch_revalidation(capture(), NOW).unwrap();
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            NOW,
        )
        .unwrap();
    assert_eq!(
        registry.finish_revalidation(pending, Ok(baseline("one", NOW)), NOW),
        Err(Failure::Revoked)
    );
}

#[test]
fn revalidation_preserves_transient_failures_but_cannot_resurrect_known_invalidity() {
    for identity_change in [false, true] {
        let (_root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let reference = publish_fixture(&registry, candidate, NOW);
        let revision = describe(&registry, &reference, NOW).revision;
        let capture = || {
            registry
                .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
                .unwrap()
        };
        let attempt = registry.dispatch_revalidation(capture(), NOW).unwrap();
        registry
            .finish_revalidation(attempt, Err(None), NOW)
            .unwrap();
        assert_eq!(describe(&registry, &reference, NOW).state, State::Ready);
        let late_success = registry.dispatch_revalidation(capture(), NOW).unwrap();
        let attempt = registry.dispatch_revalidation(capture(), NOW).unwrap();
        if identity_change {
            assert_eq!(
                registry.finish_revalidation(attempt, Ok(baseline("different", NOW)), NOW),
                Err(Failure::IdentityMismatch)
            );
        } else {
            registry
                .finish_revalidation(attempt, Err(Some(InvalidCredential::Invalid)), NOW)
                .unwrap();
        }
        assert_eq!(
            describe(&registry, &reference, NOW).state,
            State::ReauthorizationRequired
        );
        assert_eq!(
            registry.finish_revalidation(late_success, Ok(baseline("one", NOW)), NOW),
            Err(Failure::NotReady)
        );
        assert!(matches!(
            registry.capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000),
            Err(Failure::NotReady)
        ));
    }
}

#[test]
fn revalidation_capture_and_completion_are_one_use_and_keep_original_deadline() {
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let capture = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
        .unwrap();
    let replay = capture.clone();
    let dispatched = registry.dispatch_revalidation(capture, NOW).unwrap();
    assert!(matches!(
        registry.dispatch_revalidation(replay, NOW),
        Err(Failure::Conflict)
    ));
    let replay = dispatched.clone();
    registry
        .finish_revalidation(dispatched, Err(None), NOW)
        .unwrap();
    assert_eq!(
        registry.finish_revalidation(replay, Ok(baseline("one", NOW)), NOW),
        Err(Failure::Conflict)
    );
    let capture = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(capture, NOW).unwrap();
    assert_eq!(
        registry.finish_revalidation(dispatched, Ok(baseline("one", NOW + 1000)), NOW + 1000),
        Err(Failure::Expired)
    );
    assert_eq!(
        describe(&registry, &reference, NOW + 1000).state,
        State::Ready
    );
}

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
    let mut metadata = Metadata::update(root.path(), false).unwrap();
    metadata
        .connection
        .execute(
            "UPDATE registry_clock SET last_seen_ms=?1",
            [timestamp(connectors_sdk::now_ms() + 60_000).unwrap()],
        )
        .unwrap();
    metadata.persist().unwrap();
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
fn concurrent_observations_replay_current_clock_without_losing_a_floor() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let barrier = Arc::new(Barrier::new(4));
    let mut workers = Vec::new();
    for _ in 0..4 {
        let path = root.path().to_owned();
        let reference = reference.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            Registry::with_system_clock(&path)
                .describe(
                    "fixture-instance",
                    "fixture-adapter",
                    "config-1",
                    &reference,
                    0,
                    true,
                )
                .unwrap()
                .observed_at_ms
        }));
    }
    let observed = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    let floor: i64 = Metadata::inspect(root.path())
        .unwrap()
        .connection
        .query_row("SELECT last_seen_ms FROM registry_clock", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(floor >= i64::try_from(*observed.iter().max().unwrap()).unwrap());
}

#[test]
fn prepared_same_millisecond_observation_refuses_stale_registry_state() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;

    // This handle has already verified and closed the physical source, then
    // replayed ER outside the lifecycle lock. A business mutation can land
    // before it reacquires that lock.
    let mut prepared = Metadata::update_observation(root.path()).unwrap();
    let observed_floor: i64 = prepared
        .connection
        .query_row("SELECT last_seen_ms FROM registry_clock", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(observed_floor, i64::try_from(NOW).unwrap());
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            NOW,
        )
        .unwrap();

    prepared.relock_observation().unwrap();
    prepared
        .connection
        .execute("UPDATE registry_clock SET last_seen_ms=last_seen_ms", [])
        .unwrap();
    assert_eq!(
        prepared.persist_prepared_observation(),
        Err(super::super::Failure::ConcurrentRevision)
    );
    drop(prepared);
    let current = Metadata::inspect(root.path()).unwrap();
    let state: String = current
        .connection
        .query_row(
            "SELECT state FROM registry_connections WHERE connection_ref=?1",
            [&reference],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "revoked");
}

#[test]
fn an_observation_raced_on_every_unlocked_replay_still_completes() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    // Every time the observation has replayed ER outside the lifecycle lock
    // and is about to take it back, a serialized writer advances the
    // registry clock first. Unanswered, each optimistic attempt loses its
    // clock revision and the observation is starved.
    let writer = Registry::new(root.path());
    let races = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = races.clone();
    super::super::metadata::set_relock_hook(Some(Box::new(move || {
        let race = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64;
        // A later millisecond, so the write changes the clock subject.
        writer
            .transaction(NOW + 1 + race, false, |_, _, _| Ok(()))
            .unwrap();
    })));
    let observed = registry.describe(
        "fixture-instance",
        "fixture-adapter",
        "config-1",
        &reference,
        NOW + 100,
        true,
    );
    super::super::metadata::set_relock_hook(None);
    assert!(observed.is_ok(), "{:?}", observed.err());
    // One optimistic attempt lost the race; the retry replayed under the
    // lock and so never released it.
    assert_eq!(races.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn refused_business_action_still_advances_the_restart_clock_floor() {
    let (root, registry) = fixture();
    registry.begin(&binding(), NOW).unwrap();
    let refused_at = NOW + 50_000;
    assert_eq!(
        registry
            .list(
                "fixture-instance",
                "wrong-adapter",
                "fixture-config",
                PageOptions {
                    limit: 10,
                    cursor: None,
                },
                refused_at,
                false,
            )
            .err(),
        Some(Failure::Conflict)
    );
    drop(registry);
    let floor: i64 = Metadata::inspect(root.path())
        .unwrap()
        .connection
        .query_row("SELECT last_seen_ms FROM registry_clock", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(floor, i64::try_from(refused_at).unwrap());
    let restarted = Registry::new(root.path());
    assert!(matches!(
        restarted.list(
            "fixture-instance",
            "fixture-adapter",
            "fixture-config",
            PageOptions {
                limit: 10,
                cursor: None,
            },
            refused_at - 1,
            false,
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
        let publication = publish.join().unwrap();
        assert!(
            matches!(
                publication,
                Ok(_) | Err(Failure::Conflict | Failure::Revoked)
            ),
            "{publication:?}"
        );
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

#[test]
fn repeated_binding_after_reopen_keeps_the_original_profile_declaration() {
    let (root, registry) = fixture();
    registry.begin(&binding(), NOW).unwrap();
    drop(registry);

    let metadata = Metadata::inspect(root.path()).unwrap();
    let declaration: String = metadata
        .connection
        .query_row("SELECT declaration FROM registry_profiles", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(declaration, encode(&binding().profile).unwrap());
    drop(metadata);

    let reopened = Registry::new(root.path());
    reopened.begin(&binding(), NOW + 1).unwrap();
    let metadata = Metadata::inspect(root.path()).unwrap();
    let profiles: i64 = metadata
        .connection
        .query_row("SELECT count(*) FROM registry_profiles", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(profiles, 1);
}
fn publish_fixture(registry: &Registry, prepared: PreparedCandidate, now: u64) -> String {
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, now).unwrap();
    registry.publish(stored, now).unwrap()
}

#[test]
fn publication_preserves_unobserved_scope_and_credential_expiry() {
    let (root, registry) = fixture();
    let mut unscoped = binding();
    unscoped.profile.minimum_scopes.clear();
    let acquired = registry.begin(&unscoped, NOW).unwrap();
    let claim = registry.consume(acquired, NOW).unwrap();
    let candidate = registry
        .prepare(
            &claim,
            ValidatedBaseline {
                identity: ExternalIdentity {
                    kind: "fixture-account".into(),
                    subject: "one".into(),
                },
                granted_scopes: None,
                credential_expires_at_ms: None,
                collected_at_ms: NOW,
                valid_until_ms: NOW + 60_000,
            },
            12,
            NOW,
        )
        .unwrap();
    let connection = publish_fixture(&registry, candidate, NOW);
    let reopened = Registry::new(root.path());
    let observed = describe(&reopened, &connection, NOW + 1);
    assert_eq!(observed.state, State::Ready);
    let metadata = Metadata::inspect(root.path()).unwrap();
    let encoded: String = metadata
        .connection
        .query_row(
            "SELECT baseline FROM registry_connections WHERE connection_ref=?1",
            [&connection],
            |row| row.get(0),
        )
        .unwrap();
    let snapshot: EvidenceSnapshot = decode(&encoded).unwrap();
    assert_eq!(encoded, encode(&snapshot).unwrap());
    assert_eq!(snapshot.granted_scopes, None);
    assert_eq!(snapshot.credential_expires_at, None);
    assert_eq!(snapshot.checks.len(), 4);
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
    let metadata = Metadata::inspect(root.path()).unwrap();
    let encoded: String = metadata
        .connection
        .query_row(
            "SELECT baseline FROM registry_connections WHERE connection_ref=?1",
            [&connection],
            |row| row.get(0),
        )
        .unwrap();
    let snapshot: EvidenceSnapshot = decode(&encoded).unwrap();
    assert_eq!(encoded, encode(&snapshot).unwrap());
    assert!(snapshot.granted_scopes.is_some());
    assert!(snapshot.credential_expires_at.is_some());
    drop(metadata);
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
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(Failure::Conflict))
            .count(),
        1,
        "{results:?}"
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
    let (root, registry) = fixture();
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
    drop(registry);
    let registry = Registry::new(root.path());
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
