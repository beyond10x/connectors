use super::lifecycle::custody_version;
use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier};

pub(super) const NOW: u64 = 1_788_998_400_000;

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
    // The calls below open the store as a fresh process does, as a command run
    // directly does, not from a handle this thread already pooled; another test
    // thread can evict that handle from the shared pool anyway.
    crate::local::metadata::simulate_process(0x9a55_1001);
    let passive = Registry::with_system_clock(root.path());
    assert_eq!(
        passive
            .approval_target(&binding(), &reference, &BTreeSet::new())
            .unwrap(),
        old.revision
    );
    let mut changed = binding();
    changed.provider_authority.push_str("/other");
    // A changed provider authority is a changed connection identity: only a
    // new instance id helps (story:revision-change-keeps-a-recovery-path).
    assert_eq!(
        passive.approval_target(&changed, &reference, &BTreeSet::new()),
        Err(Failure::AuthenticationChanged)
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
    crate::local::metadata::exit_process();
    crate::local::metadata::simulate_process(0);
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
        Err(Failure::EvidenceExpired)
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

/// story:expired-evidence-invoke-advises-revalidate. A read on a connection
/// whose validation evidence expired while its credential is intact is refused
/// as `EvidenceExpired`, at admission and at capture, and revalidation alone
/// readmits it. A missing scope on current evidence stays `InsufficientScope`,
/// and a credential that itself expired or is known invalid stays `NotReady`.
#[test]
fn an_expired_evidence_read_is_refused_by_name_and_revalidation_readmits_it() {
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let none = BTreeSet::new();
    let write = BTreeSet::from(["write".to_owned()]);
    assert_eq!(
        registry.admit_read(&binding(), &reference, &write, NOW),
        Err(Failure::InsufficientScope)
    );
    let expired = NOW + 60_001;
    assert_eq!(
        describe(&registry, &reference, expired).state,
        State::Pending
    );
    assert_eq!(
        registry.admit_read(&binding(), &reference, &none, expired),
        Err(Failure::EvidenceExpired)
    );
    assert!(matches!(
        registry.capture_read(&binding(), &reference, &none, expired, expired + 1000),
        Err(Failure::EvidenceExpired)
    ));
    let capture = registry
        .capture_revalidation(&binding(), &reference, &revision, expired, expired + 30_000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(capture, expired).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(baseline("one", expired)), expired)
        .unwrap();
    registry
        .admit_read(&binding(), &reference, &none, expired)
        .unwrap();
    let read = registry
        .capture_read(&binding(), &reference, &none, expired, expired + 1000)
        .unwrap();
    registry.dispatch_read(read, expired).unwrap();
    // The credential's own expiry is not evidence a revalidation recollects.
    assert_eq!(
        registry.admit_read(&binding(), &reference, &none, NOW + 3_600_000),
        Err(Failure::NotReady)
    );
}

/// story:connection-lease-holds-unattended: a captured read carries its
/// evidence's own lifetime (collection to `valid_until`, here shorter than the
/// profile's bound), and a read whose evidence lapses between its capture and
/// its dispatch is refused as expired evidence, not as the read's own timeout.
/// A use bound by the read's deadline alone still expires as a timeout.
#[test]
fn a_read_whose_evidence_lapses_before_dispatch_is_refused_as_expired_evidence() {
    let (_root, registry) = fixture();
    let acquisition = registry.begin(&binding(), NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let mut short = baseline("one", NOW);
    short.valid_until_ms = NOW + 8_000;
    let candidate = registry.prepare(&claim, short, 12, NOW).unwrap();
    let reference = publish_fixture(&registry, candidate, NOW);
    let none = BTreeSet::new();
    let bounded = registry
        .capture_read(&binding(), &reference, &none, NOW, NOW + 1000)
        .unwrap();
    assert_eq!(bounded.evidence_lifetime_ms(), 8_000);
    assert!(matches!(
        registry.dispatch_read(bounded, NOW + 1000),
        Err(Failure::Expired)
    ));
    let at = NOW + 7_500;
    let read = registry
        .capture_read(&binding(), &reference, &none, at, at + 30_000)
        .unwrap();
    assert_eq!(read.expires_at_ms(), NOW + 8_000);
    assert!(matches!(
        registry.dispatch_read(read, NOW + 8_000),
        Err(Failure::EvidenceExpired)
    ));
}

/// Known invalid material is not expired evidence either: only a repair helps.
#[test]
fn an_invalid_credential_read_stays_not_ready_after_its_evidence_expired() {
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let none = BTreeSet::new();
    let captured = registry
        .capture_read(&binding(), &reference, &none, NOW, NOW + 1000)
        .unwrap();
    registry
        .invalidate_read(&captured, InvalidCredential::Invalid, NOW)
        .unwrap();
    for at in [NOW, NOW + 60_001] {
        assert_eq!(
            registry.admit_read(&binding(), &reference, &none, at),
            Err(Failure::NotReady),
            "{at}"
        );
    }
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
        // Known invalidity makes the connection Stale (story:ess-connector-
        // vocabulary-model decision 17); the late success below still cannot
        // publish over it.
        assert_eq!(describe(&registry, &reference, NOW).state, State::Stale);
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
            crate::local::metadata::set_lifecycle_lock_wait(
                crate::local::metadata::CONTENDED_LOCK_WAIT,
            );
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
    // The floor is the higher of the durable floor file and the recorded one.
    let floor = floor_file(root.path())
        .unwrap()
        .max(recorded_registry_floor(root.path()));
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
    // registry clock first. Unanswered, each optimistic attempt is refused
    // by its feed check and the observation is starved.
    let writer = Registry::new(root.path());
    let races = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = races.clone();
    super::super::metadata::set_relock_hook(Some(Box::new(move || {
        let race = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64;
        // More than one recording interval later, so the write changes the
        // recorded clock subject.
        writer
            .transaction(NOW + 60_001 + race, false, |_, _, _| Ok(()))
            .unwrap();
    })));
    let observed = registry.describe(
        "fixture-instance",
        "fixture-adapter",
        "config-1",
        &reference,
        NOW + 120_000,
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
    // The durable floor file holds the refused sample; the recorded floor,
    // within one interval of it, is not advanced.
    assert_eq!(
        floor_file(root.path()),
        Some(i64::try_from(refused_at).unwrap())
    );
    assert_eq!(recorded_registry_floor(root.path()), NOW as i64);
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
            crate::local::metadata::set_lifecycle_lock_wait(
                crate::local::metadata::CONTENDED_LOCK_WAIT,
            );
            start.wait();
            a.publish(stored, NOW)
        });
        let b = Registry::new(root.path());
        let start = barrier.clone();
        let dispatch = std::thread::spawn(move || {
            crate::local::metadata::set_lifecycle_lock_wait(
                crate::local::metadata::CONTENDED_LOCK_WAIT,
            );
            start.wait();
            b.dispatch_read(captured, NOW)
        });
        crate::local::metadata::set_lifecycle_lock_wait(
            crate::local::metadata::CONTENDED_LOCK_WAIT,
        );
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

pub(super) fn fixture() -> (tempfile::TempDir, Registry) {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    #[cfg(windows)]
    crate::local::filesystem::windows::owner_only(root.path());
    super::super::filesystem::directory(root.path(), false, true).unwrap();
    drop(Metadata::initialize(root.path()).unwrap());
    let registry = Registry::new(root.path());
    (root, registry)
}
pub(super) fn binding() -> Binding {
    Binding {
        credential_revision: None,
        executable: None,
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
            declaration: None,
            oauth2: None,
        },
        token_recipient: None,
    }
}
fn baseline(subject: &str, now: u64) -> ValidatedBaseline {
    ValidatedBaseline {
        binding: None,
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
pub(super) fn prepared(registry: &Registry, subject: &str, now: u64) -> (Claim, PreparedCandidate) {
    let acquisition = registry.begin(&binding(), now).unwrap();
    let claim = registry.consume(acquisition, now).unwrap();
    let prepared = registry
        .prepare(&claim, baseline(subject, now), 12, now)
        .unwrap();
    (claim, prepared)
}

/// Connections of one instance prepared as `begin` admitted them before
/// story:connect-refused-while-instance-holds-connection: every one is
/// allocated before any is published, so neither the host's check nor the
/// model's refuses the next. An instance that held several before the rule
/// keeps them, so the registry still serves this state.
pub(super) fn prepared_together(
    registry: &Registry,
    subjects: &[&str],
    now: u64,
) -> Vec<PreparedCandidate> {
    subjects
        .iter()
        .map(|subject| {
            let acquisition = registry.begin_unchecked(&binding(), now).unwrap();
            let claim = registry.consume(acquisition, now).unwrap();
            registry
                .prepare(&claim, baseline(subject, now), 12, now)
                .unwrap()
        })
        .collect()
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
    // The first connect is still in flight: only the profile record is under
    // test here, not the instance's one connection.
    reopened.begin_unchecked(&binding(), NOW + 1).unwrap();
    let metadata = Metadata::inspect(root.path()).unwrap();
    let profiles: i64 = metadata
        .connection
        .query_row("SELECT count(*) FROM registry_profiles", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(profiles, 1);
}
pub(super) fn publish_fixture(
    registry: &Registry,
    prepared: PreparedCandidate,
    now: u64,
) -> String {
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, now).unwrap();
    registry.publish(stored, now).unwrap()
}

/// The read-only binding's rule, one item backs one connection, is held in
/// the publishing commit: two connects of one profile that were both begun
/// and acknowledged before either published yield one connection, and the
/// second is refused as `CustodyReadOnly`. The platform path is unchanged.
/// `begin` now refuses the second connect while the first is in flight
/// (story:connect-refused-while-instance-holds-connection); the publishing
/// commit still holds the rule for two begun as before that rule.
#[test]
fn exclusive_publication_admits_one_connection_per_profile() {
    let (_root, registry) = fixture();
    let [first, second]: [PreparedCandidate; 2] =
        prepared_together(&registry, &["one", "one"], NOW)
            .try_into()
            .ok()
            .unwrap();
    let receipt = custody::WrittenVersion::fixture(first.version());
    let first = registry.acknowledge(first, receipt, NOW).unwrap();
    let receipt = custody::WrittenVersion::fixture(second.version());
    let second = registry.acknowledge(second, receipt, NOW).unwrap();
    registry.publish_with(first, NOW, true, None).unwrap();
    assert_eq!(
        registry.publish_with(second, NOW, true, None),
        Err(Failure::CustodyReadOnly)
    );

    let (_root, registry) = fixture();
    for prepared in prepared_together(&registry, &["one", "one"], NOW) {
        publish_fixture(&registry, prepared, NOW);
    }
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
                binding: None,
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
                crate::local::metadata::set_lifecycle_lock_wait(
                    crate::local::metadata::CONTENDED_LOCK_WAIT,
                );
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
    // Three connections of one instance, as one held them before the rule.
    for prepared in prepared_together(&registry, &["a", "b", "c"], NOW) {
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

// contracts/cli/v1alpha1/semantics.md:297-298 and docs/local-connection-registry.md:106-107:
// a continuation expires 300 s after the first page, and a later page's cursor
// keeps that original deadline rather than starting a new one.
#[test]
fn cursors_expire_at_the_original_five_minute_deadline() {
    let (_root, registry) = fixture();
    // Three connections of one instance, as one held them before the rule.
    for prepared in prepared_together(&registry, &["a", "b", "c"], NOW) {
        publish_fixture(&registry, prepared, NOW);
    }
    let page = |cursor: Option<&str>, now: u64| {
        registry.list(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            PageOptions { limit: 1, cursor },
            now,
            true,
        )
    };
    let first = page(None, NOW).unwrap();
    let second = page(first.next_cursor.as_deref(), NOW + ENTRY_MS - 1).unwrap();
    let cursor = second.next_cursor.as_deref().unwrap();
    assert!(matches!(
        page(Some(cursor), NOW + ENTRY_MS),
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

/// One repair attempt of `connection` through `prepare`.
fn repair_prepare(registry: &Registry, connection: &str, now: u64) -> Result<PreparedCandidate> {
    let revision = describe(registry, connection, now).revision;
    let attempt = registry
        .begin_repair(&binding(), connection, &revision, now)
        .unwrap();
    let claim = registry.consume(attempt, now).unwrap();
    registry.prepare(&claim, baseline("one", now), 12, now)
}

#[test]
fn a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity() {
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    // Far past eight undeleted versions, below the refresh bound of 64.
    for _ in 0..20 {
        repair_prepare(&registry, &connection, NOW + 1).unwrap();
    }
}

#[test]
fn a_reconnect_at_the_retention_bound_is_refused_capacity() {
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    RETAINED_BOUND.with(|bound| bound.set(10));
    for _ in 0..9 {
        repair_prepare(&registry, &connection, NOW + 1).unwrap();
    }
    assert!(matches!(
        repair_prepare(&registry, &connection, NOW + 1),
        Err(Failure::Capacity)
    ));
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
    // Beside the first connect, still in flight, as before the rule.
    let second = prepared_together(&registry, &["two"], NOW).remove(0);
    let connection = publish_fixture(&registry, second, NOW);
    let captured = registry
        .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    registry
        .invalidate_read(&captured, InvalidCredential::Missing, NOW + 1)
        .unwrap();
    // A lost keyring entry is one a reconnect fixes: the connection is Stale
    // (story:ess-connector-vocabulary-model decision 17).
    assert_eq!(
        describe(&Registry::new(root.path()), &connection, NOW + 1).state,
        State::Stale
    );
    assert!(registry.dispatch_read(captured, NOW + 1).is_err());
}

// `ExpireAcquisition` runs from Pending as well as Completing
// (`ess/domains/auth_bindings.yaml`). No other case expires an acquisition that
// was never consumed, so dropping Pending from that transition went unnoticed.
#[test]
fn expiry_sweep_retires_a_pending_acquisition_that_was_never_consumed() {
    let (root, registry) = fixture();
    let acquisition = registry.begin(&binding(), NOW).unwrap();
    assert_eq!(registry.expire(NOW + ENTRY_MS - 1).unwrap(), 0);
    assert_eq!(registry.expire(NOW + ENTRY_MS).unwrap(), 1);
    drop(registry);
    let reopened = Registry::new(root.path());
    assert!(matches!(
        reopened.consume(acquisition, NOW + ENTRY_MS),
        Err(Failure::Conflict)
    ));
    assert_eq!(reopened.expire(NOW + ENTRY_MS + 1).unwrap(), 0);
}

// auth/evidence semantics.md:204: at most 64 retained scopes
// (`ess/domains/credential_evidence.yaml` ESS-LIMIT; `scopes_valid`).
#[test]
fn retained_scopes_are_at_most_sixty_four() {
    let (_root, registry) = fixture();
    let acquisition = registry.begin(&binding(), NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let scopes = |count: usize| {
        let mut scopes = (1..count)
            .map(|index| format!("scope-{index}"))
            .collect::<BTreeSet<_>>();
        scopes.insert("read".into());
        scopes
    };
    let mut too_many = baseline("one", NOW);
    too_many.granted_scopes = Some(scopes(65));
    assert_eq!(
        registry.prepare(&claim, too_many, 12, NOW).map(|_| ()),
        Err(Failure::InvalidInput)
    );
    let mut bounded = baseline("one", NOW);
    bounded.granted_scopes = Some(scopes(64));
    registry.prepare(&claim, bounded, 12, NOW).unwrap();
}

// auth/evidence semantics.md:204: every retained scope is nonempty
// (`ess/domains/credential_evidence.yaml` ESS-LIMIT; `scopes_valid`).
#[test]
fn retained_scopes_are_nonempty() {
    let (_root, registry) = fixture();
    let acquisition = registry.begin(&binding(), NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let mut empty = baseline("one", NOW);
    empty.granted_scopes = Some(BTreeSet::from(["read".into(), String::new()]));
    assert_eq!(
        registry.prepare(&claim, empty, 12, NOW).map(|_| ()),
        Err(Failure::InvalidInput)
    );
}

// Acquisition semantics §4.3: at `now >= expires_at` known expiry prevents
// further exchange and publication; one millisecond earlier both remain open.
// The deadline is host time (`ess/domains/auth_bindings.yaml` ESS-LIMIT).
#[test]
fn acquisition_deadline_closes_consumption_and_publication_at_expiry() {
    // The registry clock never runs backwards, so the late attempt is checked last.
    let (_root, registry) = fixture();
    let late = registry.begin(&binding(), NOW).unwrap();
    // Beside the one in flight, as an instance from before the rule allowed.
    let acquisition = registry.begin_unchecked(&binding(), NOW).unwrap();
    let before = NOW + ENTRY_MS - 1;
    let claim = registry.consume(acquisition, before).unwrap();
    let prepared = registry
        .prepare(&claim, baseline("one", before), 12, before)
        .unwrap();
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, before).unwrap();
    assert!(matches!(
        registry.consume(late, NOW + ENTRY_MS),
        Err(Failure::Expired)
    ));
    assert!(matches!(
        registry.publish(stored, NOW + ENTRY_MS),
        Err(Failure::Expired)
    ));
}

// Connection semantics §4.2: the baseline is cleared on terminal local
// revocation. The ER lowering cannot clear an Optional field
// (`ess/domains/auth_bindings.yaml` ESS-LIMIT), so the owner does it.
#[test]
fn revocation_clears_the_baseline_with_the_active_material() {
    let (root, registry) = fixture();
    let (_claim, candidate) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &connection, NOW).revision;
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &connection,
            &revision,
            NOW + 1,
        )
        .unwrap();
    let metadata = Metadata::inspect(root.path()).unwrap();
    let (baseline, generation, material): (Option<String>, Option<String>, Option<String>) =
        metadata
            .connection
            .query_row(
                "SELECT baseline,active_generation,active_material FROM registry_connections WHERE connection_ref=?1",
                [&connection],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
    assert_eq!((baseline, generation, material), (None, None, None));
}

// Local connection registry: acknowledged deletion clears byte-size metadata
// and keeps identity/fence history. The ER lowering cannot clear an Optional
// field (`ess/domains/auth_bindings.yaml` ESS-LIMIT); the schema refuses a
// deletion that keeps the size.
#[test]
fn acknowledged_deletion_cannot_keep_the_byte_size() {
    let (root, registry) = fixture();
    let (claim, _candidate) = prepared(&registry, "one", NOW);
    registry.fail(&claim, NOW + 1).unwrap();
    let mut metadata = Metadata::update(root.path(), false).unwrap();
    assert!(
        metadata
            .connection
            .execute("UPDATE registry_materials SET deleted=1", [])
            .is_err(),
        "a deleted version kept its byte size"
    );
    assert_eq!(
        metadata
            .connection
            .execute(
                "UPDATE registry_materials SET deleted=1,byte_size=NULL WHERE retirement_fence IS NOT NULL",
                [],
            )
            .unwrap(),
        1
    );
    metadata.persist().unwrap();
}

// A read use whose expiry an admitted sweep has recorded leaves the SQL
// projection while its Entity Runtime history remains. A clock-only
// observation after that (status, list, the observation before revalidate or
// invoke) must still be admitted; it failed with MetadataUnavailable.
#[test]
fn observation_after_a_recorded_read_use_expiry_is_admitted() {
    // The first use is recorded, as a release up to 0.76.0 recorded it: a
    // store such a release wrote holds uses whose expiry is still to record.
    super::super::metadata::record_read_uses(true);
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let connection = publish_fixture(&registry, first, NOW);
    registry
        .capture_read(&binding(), &connection, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    // This capture's admitted sweep records the first use Expired.
    let _ = registry.capture_read(
        &binding(),
        &connection,
        &BTreeSet::new(),
        NOW + 2000,
        NOW + 3000,
    );
    let observed = registry.describe(
        "fixture-instance",
        "fixture-adapter",
        "config-1",
        &connection,
        NOW + 4000,
        true,
    );
    assert!(observed.is_ok(), "{:?}", observed.err());
}

/// story:settled-read-uses-leave-the-store. No path of a read use records it:
/// a read, a cancelled read, a reuse, a finished and a cancelled revalidation,
/// a refused read and an invalidated read leave no `ReadUse` event in the
/// store. With `record_read_uses` on, one read records its three events, as
/// releases up to 0.76.0 did.
#[test]
fn no_path_of_a_read_use_records_it() {
    use super::super::metadata::record_read_uses;
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let capture = |reference: &str, now: u64| {
        registry
            .capture_read(&binding(), reference, &BTreeSet::new(), now, now + 1000)
            .unwrap()
    };
    // A read: captured, dispatched, released.
    let dispatched = registry
        .dispatch_read(capture(&reference, NOW + 1), NOW + 1)
        .unwrap();
    registry.release_read(dispatched, NOW + 1).unwrap();
    // A cancelled read.
    registry
        .cancel_read(capture(&reference, NOW + 2), NOW + 2)
        .unwrap();
    // A reuse by another instance, and its release.
    let target = Binding {
        instance_id: "target-instance".into(),
        configuration_revision: "config-target".into(),
        ..binding()
    };
    let source = registry
        .capture_reuse(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &target,
            &reference,
            &revision,
            NOW + 3,
            NOW + 1003,
        )
        .unwrap();
    let dispatched = registry.dispatch_reuse(source, NOW + 3).unwrap();
    registry.release_read(dispatched, NOW + 3).unwrap();
    // A revalidation: captured, dispatched, finished.
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW + 4, NOW + 1004)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, NOW + 4).unwrap();
    registry
        .finish_revalidation(dispatched, Err(None), NOW + 4)
        .unwrap();
    // A cancelled revalidation.
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW + 5, NOW + 1005)
        .unwrap();
    registry.cancel_revalidation(captured, NOW + 5).unwrap();
    // A refused read and an invalidated read. Each makes the connection
    // Stale, so both are captured, and the first dispatched, before either ends.
    let refused = capture(&reference, NOW + 6);
    let invalid = capture(&reference, NOW + 6);
    let dispatched = registry.dispatch_read(refused, NOW + 6).unwrap();
    registry
        .invalidate_read(&invalid, InvalidCredential::Invalid, NOW + 6)
        .unwrap();
    registry.refuse_read(dispatched, NOW + 6).unwrap();
    let recorded = recorded_read_uses(root.path());
    assert_eq!(
        recorded, 0,
        "seven read uses, one per path, recorded {recorded} ReadUse events"
    );

    // The connection is Stale; the read below needs a new one.
    let stale = describe(&registry, &reference, NOW + 7).revision;
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &stale,
            NOW + 7,
        )
        .unwrap();
    let (_, candidate) = prepared(&registry, "one", NOW + 8);
    let again = publish_fixture(&registry, candidate, NOW + 8);
    record_read_uses(true);
    let dispatched = registry
        .dispatch_read(capture(&again, NOW + 9), NOW + 9)
        .unwrap();
    registry.release_read(dispatched, NOW + 9).unwrap();
    record_read_uses(false);
    assert_eq!(
        recorded_read_uses(root.path()),
        3,
        "a read with `record_read_uses` on records Captured, Dispatched and Released"
    );
}

/// story:settled-read-uses-leave-the-store, difference 4. A use is a row only
/// in the process that captured it, so a revoke another process runs (the CLI's
/// `connections revoke`) does not release it: the owner's dispatch of it finds
/// the use open and the connection revoked, and answers `Revoked`. An
/// undispatched revalidation likewise.
#[test]
fn a_revoke_by_another_process_refuses_the_owners_dispatch_as_revoked() {
    use super::super::metadata::{exit_process, simulate_process};
    let revoke = |registry: &Registry, reference: &str, revision: &str| {
        simulate_process(2);
        registry
            .revoke(
                "fixture-instance",
                "fixture-adapter",
                reference,
                revision,
                NOW + 1,
            )
            .unwrap();
        exit_process();
        simulate_process(1);
    };

    simulate_process(1);
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    revoke(&registry, &reference, &revision);
    let read = registry.dispatch_read(captured, NOW + 2).err();

    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
        .unwrap();
    revoke(&registry, &reference, &revision);
    let revalidation = registry.dispatch_revalidation(captured, NOW + 2).err();
    assert_eq!(
        (read, revalidation),
        (Some(Failure::Revoked), Some(Failure::Revoked)),
        "the dispatch of a read use and of a revalidation whose connection another process revoked"
    );
    exit_process();
    simulate_process(0);
}

/// story:settled-read-uses-leave-the-store, difference 2. The uses a process
/// holds are gone when it ends: the next process of the same id and another
/// process both see no use, and a dispatch of an ended process's use answers
/// `Conflict`, the answer for an unknown id.
#[test]
fn the_read_uses_of_an_ended_process_are_gone_with_it() {
    use super::super::metadata::{exit_process, simulate_process};
    let uses = |root: &Path| -> i64 {
        Metadata::inspect(root)
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM registry_uses", [], |row| row.get(0))
            .unwrap()
    };
    simulate_process(1);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let capture = || {
        registry
            .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
            .unwrap()
    };
    let first = capture();
    registry.dispatch_read(capture(), NOW).unwrap();
    exit_process();

    simulate_process(1);
    assert_eq!(uses(root.path()), 0, "the next process 1 sees a use");
    simulate_process(2);
    assert_eq!(uses(root.path()), 0, "process 2 sees a use");
    exit_process();
    simulate_process(1);
    assert_eq!(
        registry.dispatch_read(first, NOW + 1).err(),
        Some(Failure::Conflict)
    );
    exit_process();
    simulate_process(0);
}

// Revoking a connection is terminal for that connection only: the same
// identity connects again through a fresh acquisition.
#[test]
fn the_same_identity_connects_again_after_revoke() {
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, first, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            NOW + 1,
        )
        .unwrap();
    let (_claim, second) = prepared(&registry, "one", NOW + 2);
    let again = publish_fixture(&registry, second, NOW + 2);
    assert_ne!(again, reference);
}

// A second configured instance of the same adapter in one store: the new
// instance is declared beside the first and connects.
#[test]
fn a_second_instance_of_the_same_adapter_connects() {
    let (_root, registry) = fixture();
    let (_claim, first) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, first, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            NOW + 1,
        )
        .unwrap();
    let mut second = binding();
    second.instance_id = "second-instance".into();
    second.configuration_revision = "config-2".into();
    let acquisition = registry.begin(&second, NOW + 2);
    assert!(acquisition.is_ok(), "{:?}", acquisition.err());
}

/// Grows the recorded store to at least `events` events. Like the observed
/// store it holds few subjects and many records: forty runtime records that
/// each remember a 1 KiB description, whose suppression then flips until the
/// store holds `events` events.
fn seed_recorded_events(root: &Path, events: i64) {
    const SUBJECTS: i64 = 40;
    let descriptor = serde_json::json!({
        "version":"v1alpha1", "instance":"fixture", "adapter":"adapter",
        "revision":"descriptor", "configuration_schema":{"type":"object"},
        "operations":[{"id":"read", "description":"d".repeat(1024), "contract":"fixture/1",
            "profile":"pat", "input_schema":{"type":"object"},
            "output_schema":{"type":"object"}}]
    });
    let bootstrap: crate::local::runtime::Bootstrap = serde_json::from_value(serde_json::json!({
        "instance":"fixture", "adapter":"adapter", "protocol":"v1alpha1",
        "configuration_revision":"cfg", "provider_authority":"fixture-authority",
        "descriptor":descriptor.to_string(),
        "profiles":[{"id":"pat", "revision":"1", "purpose":"delegated_user",
            "subject":"user", "scheme":"http_bearer", "capability":"http-bearer",
            "minimum_scopes":[], "evidence_lifetime_ms":60000,
            "fields":[{"name":"token", "label":"Fictional credential", "max_bytes":100}]}],
        "requirements":[{"operation":"read", "profile":"pat", "scopes":[], "effect":"read"}]
    }))
    .unwrap();
    // A remembered bootstrap names the build that wrote it (ess/domains/cli.yaml).
    let description =
        crate::local::runtime::state::encode_cached(&bootstrap, Some("0.0.0-fixture")).unwrap();
    // Ten members a batch keeps each batch well inside the bridge deadline on
    // a loaded host.
    const BATCH: i64 = 10;
    for first in (0..SUBJECTS).step_by(BATCH as usize) {
        seed_batch(root, |metadata| {
            for member in first..first + BATCH {
                metadata
                    .connection
                    .execute(
                        "INSERT OR IGNORE INTO local_runtime_instances (instance_id,suppressed) VALUES (?1,0)",
                        params![format!("seed-{member}")],
                    )
                    .unwrap();
            }
            metadata.persist_runtime_state()?;
            metadata
                .connection
                .execute(
                    "UPDATE local_runtime_instances SET selection='seed',bootstrap=?1,observed_at_ms=0 WHERE instance_id LIKE 'seed-%' AND bootstrap IS NULL",
                    [&description],
                )
                .unwrap();
            metadata.persist_runtime_state()
        });
    }
    // A quarter of the records flip in each later batch. Entity Runtime's batch
    // execution slows with both its member count and the store's size, so a
    // wider batch outruns the bridge deadline on a grown store.
    const GROUPS: i64 = 4;
    let mut round = 0;
    while recorded_events(root) < events {
        seed_batch(root, |metadata| {
            metadata
                .connection
                .execute(
                    "UPDATE local_runtime_instances SET suppressed=1-suppressed WHERE instance_id LIKE 'seed-%' AND CAST(substr(instance_id,6) AS INTEGER)%?1=?2",
                    params![GROUPS, round % GROUPS],
                )
                .unwrap();
            metadata.persist_runtime_state()
        });
        round += 1;
    }
}

/// One seeding batch, run by `batch` in a fresh handle. Entity Runtime bounds a
/// batch at 30 s; on a host loaded enough to pass that bound the commit's
/// outcome is legitimately unknown, and the next action is to read the store
/// again and run the batch again, which is safe because every batch is
/// idempotent or only has to have run enough times. Anything else, or more
/// than `UNKNOWN_LIMIT` unknown outcomes in a row, is a failure.
fn seed_batch(
    root: &Path,
    mut batch: impl FnMut(&mut Metadata) -> std::result::Result<(), crate::local::Failure>,
) {
    const UNKNOWN_LIMIT: u32 = 3;
    for _ in 0..=UNKNOWN_LIMIT {
        let mut metadata = Metadata::update(root, true).unwrap();
        match batch(&mut metadata) {
            Ok(()) => return,
            Err(crate::local::Failure::OutcomeUnknown) => {}
            Err(other) => panic!("a seeding batch failed: {other:?}"),
        }
    }
    panic!(
        "{} seeding batches in a row ended with an unknown outcome: each passed Entity Runtime's 30 s batch deadline",
        UNKNOWN_LIMIT + 1
    );
}

pub(super) fn recorded_events(root: &Path) -> i64 {
    rusqlite::Connection::open_with_flags(
        root.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row("SELECT count(*) FROM connectors_er_events", [], |r| {
        r.get(0)
    })
    .unwrap()
}

/// Recorded entries whose subject is a `connectors.clock.LocalClockFloor`,
/// read from the wrapper blob each `er.recorded_entry` event names.
pub(super) fn recorded_clock_floor_events(root: &Path) -> i64 {
    rusqlite::Connection::open_with_flags(
        root.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row(
        "SELECT count(*) FROM connectors_er_events e JOIN connectors_er_blobs b \
         ON b.tenant_id=e.tenant_id AND b.digest=json_extract(e.data,'$.blob') \
         WHERE e.event_name='er.recorded_entry' \
         AND instr(CAST(b.bytes AS TEXT),'\"subject\":[\"connectors.clock.LocalClockFloor\",')>0",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// Recorded entries whose subject is a
/// `connectors.credential_evidence.ReadUse`, read as
/// `recorded_clock_floor_events` reads the clock's.
pub(super) fn recorded_read_uses(root: &Path) -> i64 {
    rusqlite::Connection::open_with_flags(
        root.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row(
        "SELECT count(*) FROM connectors_er_events e JOIN connectors_er_blobs b \
         ON b.tenant_id=e.tenant_id AND b.digest=json_extract(e.data,'$.blob') \
         WHERE e.event_name='er.recorded_entry' \
         AND instr(CAST(b.bytes AS TEXT),'\"subject\":[\"connectors.credential_evidence.ReadUse\",')>0",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// The registry clock floor Entity Runtime records, as a fresh open reads it.
fn recorded_registry_floor(root: &Path) -> i64 {
    super::super::metadata::simulate_process(u64::MAX - 1);
    Metadata::inspect(root)
        .unwrap()
        .connection
        .query_row(
            "SELECT last_seen_ms FROM registry_clock WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

const FLOOR_FILE: &str = "registry-clock.floor";

fn floor_file(root: &Path) -> Option<i64> {
    let bytes = std::fs::read(root.join(FLOOR_FILE)).ok()?;
    serde_json::from_slice::<serde_json::Value>(&bytes).ok()?["last_seen_ms"].as_i64()
}

fn write_floor_file(root: &Path, last_seen_ms: u64) {
    let path = root.join(FLOOR_FILE);
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({ "last_seen_ms": last_seen_ms })).unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    #[cfg(windows)]
    crate::local::filesystem::windows::owner_only(&path);
}

/// One registry transaction that does nothing but sample the clock.
fn tick(registry: &Registry, now: u64) -> Result<()> {
    registry.transaction(now, false, |_, _, _| Ok(()))
}

/// One read invoke whose steps each read the clock at their own millisecond,
/// as a production CLI and owner do: equal-value records cannot pass this.
fn read_invoke_with_distinct_step_clocks(
    root: &Path,
    registry: &Registry,
    reference: &str,
    now: u64,
) {
    super::super::metadata::simulate_process(now);
    drop(Metadata::inspect(root).unwrap());
    registry
        .admit_read(&binding(), reference, &BTreeSet::new(), now)
        .unwrap();
    super::super::metadata::simulate_process(1);
    let captured = registry
        .capture_read(&binding(), reference, &BTreeSet::new(), now + 1, now + 1000)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, now + 2).unwrap();
    registry.release_read(dispatched, now + 3).unwrap();
}

#[test]
fn read_invokes_record_the_registry_floor_once_per_interval_not_once_per_step() {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let before = recorded_clock_floor_events(root.path());
    // Five invokes, twenty distinct clock readings, all within one interval.
    for invoke in 0..5 {
        read_invoke_with_distinct_step_clocks(
            root.path(),
            &registry,
            &reference,
            NOW + 1 + invoke * 10,
        );
    }
    let within = recorded_clock_floor_events(root.path()) - before;
    assert_eq!(
        within, 0,
        "five read invokes inside one interval recorded {within} registry floors"
    );
    assert_eq!(recorded_registry_floor(root.path()), NOW as i64);
    assert_eq!(floor_file(root.path()), Some((NOW + 44) as i64));
    // Exactly at the interval the floor is not recorded; one past it is.
    tick(&registry, NOW + 60_000).unwrap();
    assert_eq!(recorded_registry_floor(root.path()), NOW as i64);
    tick(&registry, NOW + 60_001).unwrap();
    assert_eq!(recorded_registry_floor(root.path()), (NOW + 60_001) as i64);
    assert_eq!(recorded_clock_floor_events(root.path()) - before, 1);
}

#[test]
fn a_missing_floor_file_falls_back_to_the_recorded_floor_plus_the_interval() {
    let (root, registry) = fixture();
    tick(&registry, NOW).unwrap();
    tick(&registry, NOW + 10).unwrap();
    assert_eq!(recorded_registry_floor(root.path()), NOW as i64);
    let _ = std::fs::remove_file(root.path().join(FLOOR_FILE));
    let restarted = Registry::new(root.path());
    super::super::metadata::simulate_process(42);
    assert_eq!(
        tick(&restarted, NOW + 59_999),
        Err(Failure::MetadataUnavailable)
    );
    tick(&restarted, NOW + 60_000).unwrap();
    assert_eq!(floor_file(root.path()), Some((NOW + 60_000) as i64));
}

#[test]
fn a_corrupt_floor_file_falls_back_to_the_recorded_floor_plus_the_interval() {
    let (root, registry) = fixture();
    tick(&registry, NOW).unwrap();
    tick(&registry, NOW + 10).unwrap();
    let path = root.path().join(FLOOR_FILE);
    std::fs::write(&path, b"not a floor").unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    #[cfg(windows)]
    crate::local::filesystem::windows::owner_only(&path);
    assert_eq!(
        tick(&registry, NOW + 59_999),
        Err(Failure::MetadataUnavailable)
    );
    tick(&registry, NOW + 60_000).unwrap();
}

#[test]
fn a_rolled_back_floor_file_is_still_bounded_by_the_recorded_floor() {
    let (root, registry) = fixture();
    tick(&registry, NOW).unwrap();
    tick(&registry, NOW + 70_000).unwrap();
    tick(&registry, NOW + 80_000).unwrap();
    assert_eq!(recorded_registry_floor(root.path()), (NOW + 70_000) as i64);
    // The file is rolled back by hand to a value below the recorded floor.
    write_floor_file(root.path(), NOW);
    super::super::metadata::simulate_process(43);
    let restarted = Registry::new(root.path());
    assert_eq!(
        tick(&restarted, NOW + 69_999),
        Err(Failure::MetadataUnavailable)
    );
    // At most one interval below the last committed sample is admitted.
    tick(&restarted, NOW + 70_000).unwrap();
}

#[test]
fn a_cross_process_regression_is_refused() {
    let (root, registry) = fixture();
    tick(&registry, NOW).unwrap();
    super::super::metadata::simulate_process(44);
    tick(&registry, NOW + 5_000).unwrap();
    super::super::metadata::exit_process();
    super::super::metadata::simulate_process(45);
    let other = Registry::new(root.path());
    assert_eq!(tick(&other, NOW + 4_999), Err(Failure::MetadataUnavailable));
    tick(&other, NOW + 5_000).unwrap();
    assert_eq!(recorded_registry_floor(root.path()), NOW as i64);
}

/// The metadata transactions one `invoke` of a read operation
/// performs, in the process that performs each: the CLI's cached-description
/// reads, admission and owner handshake, then the owner's capture, dispatch
/// and release. Each invoke's CLI is a fresh process; the owner is not.
fn read_invoke(root: &Path, registry: &Registry, reference: &str, now: u64) -> u64 {
    let before = super::super::metadata::full_replays();
    super::super::metadata::simulate_process(now);
    drop(Metadata::inspect(root).unwrap());
    drop(Metadata::inspect(root).unwrap());
    registry
        .admit_read(&binding(), reference, &BTreeSet::new(), now)
        .unwrap();
    drop(Metadata::inspect(root).unwrap());
    super::super::metadata::simulate_process(1);
    let captured = registry
        .capture_read(&binding(), reference, &BTreeSet::new(), now, now + 1000)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, now).unwrap();
    registry.release_read(dispatched, now).unwrap();
    super::super::metadata::full_replays() - before
}

fn replay_fixture(events: i64) -> (tempfile::TempDir, Registry, String) {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    seed_recorded_events(root.path(), events);
    // The owner has served an invoke before the one under test.
    read_invoke(root.path(), &registry, &reference, NOW + 1);
    (root, registry, reference)
}

#[test]
fn a_read_invoke_against_a_grown_store_replays_it_at_most_once() {
    let (root, registry, reference) = replay_fixture(150);
    assert!(recorded_events(root.path()) >= 150);
    let replays = read_invoke(root.path(), &registry, &reference, NOW + 2);
    assert!(
        replays <= 1,
        "one read invoke replayed the whole store {replays} times"
    );
    // And again once the store has grown by that invoke's own records.
    let replays = read_invoke(root.path(), &registry, &reference, NOW + 3);
    assert!(
        replays <= 1,
        "the next read invoke replayed the whole store {replays} times"
    );
    // What the owner caught up on, after another process wrote, is exactly
    // what a complete replay in a fresh process reads.
    super::super::metadata::simulate_process(1);
    let held = Metadata::inspect(root.path()).unwrap().recorded_rows();
    super::super::metadata::simulate_process(u64::MAX);
    let replayed = Metadata::inspect(root.path()).unwrap().recorded_rows();
    assert!(!held.is_empty());
    assert_eq!(held, replayed);
}

/// A seeded store, reused from `saved` when it holds one so that two builds
/// are measured against the same bytes, and saved there otherwise.
fn saved_replay_fixture(
    events: i64,
    saved: Option<&Path>,
) -> (tempfile::TempDir, Registry, String) {
    let stored = saved.map(|saved| saved.join(events.to_string()));
    let Some(stored) = stored.filter(|stored| stored.join("reference").exists()) else {
        let (root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let reference = publish_fixture(&registry, candidate, NOW);
        seed_recorded_events(root.path(), events);
        if let Some(stored) = saved.map(|saved| saved.join(events.to_string())) {
            std::fs::create_dir_all(&stored).unwrap();
            rusqlite::Connection::open(root.path().join("metadata.sqlite3"))
                .unwrap()
                .execute(
                    "VACUUM INTO ?1",
                    [stored.join("metadata.sqlite3").to_str().unwrap()],
                )
                .unwrap();
            // The copy is written in rollback mode; metadata admits only WAL.
            rusqlite::Connection::open(stored.join("metadata.sqlite3"))
                .unwrap()
                .pragma_update(None, "journal_mode", "WAL")
                .unwrap();
            std::fs::write(stored.join("reference"), &reference).unwrap();
        }
        return (root, registry, reference);
    };
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    #[cfg(windows)]
    crate::local::filesystem::windows::owner_only(root.path());
    for name in ["metadata.sqlite3", "metadata.lock"] {
        let target = root.path().join(name);
        if name == "metadata.lock" {
            std::fs::write(&target, b"").unwrap();
        } else {
            std::fs::copy(stored.join(name), &target).unwrap();
        }
        #[cfg(unix)]
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
        #[cfg(windows)]
        crate::local::filesystem::windows::owner_only(&target);
    }
    let reference = std::fs::read_to_string(stored.join("reference")).unwrap();
    let registry = Registry::new(root.path());
    (root, registry, reference)
}

/// Wall time of one read invoke's metadata transactions at 600 recorded
/// events, or at the comma-separated sizes in `CONNECTORS_REPLAY_EVENTS`.
/// A measurement, not a check: run it explicitly with
/// `CONNECTORS_REPLAY_STORES=<dir> cargo test --release -p connectors-host --lib read_invoke_metadata_time -- --ignored --nocapture`;
/// the first run seeds and saves the stores in `<dir>`, later runs reuse them.
#[test]
#[ignore = "timing measurement; run explicitly"]
fn read_invoke_metadata_time() {
    let saved = std::env::var_os("CONNECTORS_REPLAY_STORES").map(std::path::PathBuf::from);
    let sizes = std::env::var("CONNECTORS_REPLAY_EVENTS").map_or_else(
        |_| vec![600],
        |sizes| {
            sizes
                .split(',')
                .map(|size| size.trim().parse().unwrap())
                .collect()
        },
    );
    for events in sizes {
        let (root, registry, reference) = saved_replay_fixture(events, saved.as_deref());
        let recorded = recorded_events(root.path());
        let mut timings = Vec::new();
        let mut replays = 0;
        let mut failed = 0;
        // The first invoke starts the owner; the next five are measured.
        for invoke in 0..6 {
            let started = std::time::Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                read_invoke(root.path(), &registry, &reference, NOW + 1 + invoke)
            }));
            let elapsed = started.elapsed().as_millis();
            println!("events={recorded} invoke={invoke} ms={elapsed} outcome={outcome:?}");
            if invoke == 0 {
                continue;
            }
            match outcome {
                Ok(count) => replays += count,
                Err(_) => failed += 1,
            }
            timings.push(elapsed);
        }
        timings.sort_unstable();
        println!(
            "events={recorded} invokes=5 failed={failed} replays={replays} median_ms={} min_ms={} max_ms={}",
            timings[2], timings[0], timings[4]
        );
    }
}

// ---------------------------------------------------------------------------
// Adversarial pass on story:settled-read-uses-leave-the-store (e08df9e9fd8).
// Each test acts as its own simulated process, so that ending it closes only
// its own handles and held uses.

/// Row count and content hash of every table of the store file.
fn adversary_d5_store_tables(root: &Path) -> std::collections::BTreeMap<String, (i64, u64)> {
    use rusqlite::types::ValueRef;
    use std::hash::{Hash, Hasher};
    let connection = rusqlite::Connection::open_with_flags(
        root.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let names = connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    let mut tables = std::collections::BTreeMap::new();
    for name in names {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{name}\""))
            .unwrap();
        let columns = statement.column_count();
        let mut rows = statement.query([]).unwrap();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut count = 0_i64;
        while let Some(row) = rows.next().unwrap() {
            count += 1;
            for index in 0..columns {
                match row.get_ref(index).unwrap() {
                    ValueRef::Null => 0_u8.hash(&mut hasher),
                    ValueRef::Integer(value) => {
                        1_u8.hash(&mut hasher);
                        value.hash(&mut hasher);
                    }
                    ValueRef::Real(value) => {
                        2_u8.hash(&mut hasher);
                        value.to_bits().hash(&mut hasher);
                    }
                    ValueRef::Text(value) => {
                        3_u8.hash(&mut hasher);
                        value.hash(&mut hasher);
                    }
                    ValueRef::Blob(value) => {
                        4_u8.hash(&mut hasher);
                        value.hash(&mut hasher);
                    }
                }
            }
        }
        tables.insert(name, (count, hasher.finish()));
    }
    tables
}

/// The read uses of the calling process's projection, as (dispatched, released) by id.
fn adversary_d5_uses(root: &Path) -> std::collections::BTreeMap<String, (bool, bool)> {
    let metadata = Metadata::inspect(root).unwrap();
    let mut statement = metadata
        .connection
        .prepare("SELECT use_id,dispatched,released FROM registry_uses")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap()
}

/// Read uses the store records in a state other than `Expired`.
fn adversary_d5_unexpired_recorded_uses(root: &Path) -> usize {
    Metadata::inspect(root)
        .unwrap()
        .recorded_rows()
        .iter()
        .filter(|row| {
            row.contains("entity: \"connectors.credential_evidence.ReadUse\"")
                && !row.contains("lifecycle_state: \"Expired\"")
        })
        .count()
}

/// Point 5, the Outcome: "A read invoke records nothing per read in the local
/// metadata store." Acceptance 1 counts two tables (events and blobs). Here
/// every table of the store file is compared, and the file's commit counter,
/// after one warm-up read: a read, a cancelled read, a reuse and a cancelled
/// revalidation, all inside one interval of the recorded clock floor.
#[test]
fn adversary_d5_read_paths_leave_every_table_of_the_store_file_unchanged() {
    use super::super::metadata::{exit_process, simulate_process};
    simulate_process(0xad5_0001);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let read = |now: u64| {
        registry
            .admit_read(&binding(), &reference, &BTreeSet::new(), now)
            .unwrap();
        let captured = registry
            .capture_read(&binding(), &reference, &BTreeSet::new(), now, now + 1000)
            .unwrap();
        let dispatched = registry.dispatch_read(captured, now).unwrap();
        registry.release_read(dispatched, now).unwrap();
    };
    // Whatever a first read writes once (an open checkpoint) is written here.
    read(NOW + 1);
    let observer = rusqlite::Connection::open_with_flags(
        root.path().join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let commits = |observer: &rusqlite::Connection| -> i64 {
        observer
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .unwrap()
    };
    let before_commits = commits(&observer);
    let mut before = adversary_d5_store_tables(root.path());
    assert!(
        before
            .get("connectors_er_events")
            .is_some_and(|table| table.0 > 0),
        "the comparison reads the store's tables: {before:?}"
    );
    let mut changes = Vec::new();
    let mut compare = |path: &str| {
        let after = adversary_d5_store_tables(root.path());
        for (table, now) in &after {
            if before.get(table) != Some(now) {
                changes.push(format!(
                    "{path}: table {table} was {:?} (rows, hash), is {now:?}",
                    before.get(table)
                ));
            }
        }
        before = after;
    };
    for invoke in 2..=4 {
        read(NOW + invoke);
    }
    compare("three reads");
    let captured = registry
        .capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            NOW + 5,
            NOW + 1005,
        )
        .unwrap();
    registry.cancel_read(captured, NOW + 5).unwrap();
    compare("a cancelled read");
    let target = Binding {
        instance_id: "target-instance".into(),
        configuration_revision: "config-target".into(),
        ..binding()
    };
    let source = registry
        .capture_reuse(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &target,
            &reference,
            &revision,
            NOW + 6,
            NOW + 1006,
        )
        .unwrap();
    let dispatched = registry.dispatch_reuse(source, NOW + 6).unwrap();
    registry.release_read(dispatched, NOW + 6).unwrap();
    compare("a reuse");
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW + 7, NOW + 1007)
        .unwrap();
    registry.cancel_revalidation(captured, NOW + 7).unwrap();
    compare("a cancelled revalidation");
    let after_commits = commits(&observer);
    drop(observer);
    exit_process();
    simulate_process(0);
    assert!(
        changes.is_empty(),
        "read paths changed the store file: {changes:#?}"
    );
    assert_eq!(
        after_commits, before_commits,
        "no table changed, yet a commit wrote the store file during the read paths"
    );
}

/// Point 3. A store an owner of 0.76.0 left: ten recorded, released uses and
/// two recorded uses still open (one Captured, one Dispatched), the owner
/// gone. The next owner (this change) reads the open ones as open until their
/// deadline and no longer; its first capture after the deadline records each
/// of the twelve Expired once, in bounded batches, while its own use is held
/// and not recorded; nothing is recorded after that.
#[test]
fn adversary_d5_the_next_owner_expires_the_recorded_uses_of_an_ended_0760_owner_once() {
    use super::super::metadata::{
        exit_process, record_read_uses, simulate_process, with_expiry_batches,
    };
    use super::use_and_retirement::TEST_READ_USE_BOUND;
    simulate_process(0xad5_0002);
    record_read_uses(true);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let capture =
        |now: u64| registry.capture_read(&binding(), &reference, &BTreeSet::new(), now, now + 1000);
    for invoke in 1..=10 {
        let dispatched = registry
            .dispatch_read(capture(NOW + invoke).unwrap(), NOW + invoke)
            .unwrap();
        registry.release_read(dispatched, NOW + invoke).unwrap();
    }
    let left_captured = capture(NOW + 11).unwrap();
    registry
        .dispatch_read(capture(NOW + 11).unwrap(), NOW + 11)
        .unwrap();
    record_read_uses(false);
    assert_eq!(adversary_d5_unexpired_recorded_uses(root.path()), 12);
    // The 0.76.0 owner ends; the next owner is another process.
    exit_process();
    simulate_process(0xad5_0003);
    TEST_READ_USE_BOUND.with(|bound| bound.set(Some(2)));
    let before_deadline = capture(NOW + 12).err();
    let read_uses = recorded_read_uses(root.path());
    let later = NOW + 10_000;
    let (first, expiry_batches) = with_expiry_batches(4, None, || capture(later));
    let unexpired = adversary_d5_unexpired_recorded_uses(root.path());
    let expired = recorded_read_uses(root.path()) - read_uses;
    let events = recorded_events(root.path());
    let second = capture(later);
    let third = capture(later).err();
    let dispatched = first.and_then(|first| registry.dispatch_read(first, later));
    let released = dispatched.and_then(|dispatched| registry.release_read(dispatched, later));
    let fourth = capture(later).map(|_| ());
    let left = registry.dispatch_read(left_captured, later).err();
    let events_after = recorded_events(root.path()) - events;
    let unexpired_after = adversary_d5_unexpired_recorded_uses(root.path());
    TEST_READ_USE_BOUND.with(|bound| bound.set(None));
    exit_process();
    simulate_process(0);
    assert_eq!(
        before_deadline,
        Some(Failure::Capacity),
        "the two open uses the ended owner recorded count as open before their deadline"
    );
    assert_eq!(
        (released, second.map(|_| ()), third, fourth, left),
        (
            Ok(()),
            Ok(()),
            Some(Failure::Capacity),
            Ok(()),
            Some(Failure::Conflict)
        ),
        "after the deadline: the first capture and its read, a second capture, a third past the \
         bound of 2, a fourth once one is released, and the dispatch of the ended owner's use"
    );
    assert_eq!(
        (unexpired, expired, expiry_batches),
        (0, 12, 2),
        "recorded uses not Expired after the first capture, Expired events it recorded, expiry \
         batches ahead of the final one"
    );
    assert_eq!(
        (events_after, unexpired_after),
        (0, 0),
        "events recorded and recorded uses not Expired after the first capture"
    );
}

/// Point 4. An observation builds its projection without the lifecycle lock
/// and takes the lock back before it persists. In that window this process
/// cancels a use the projection holds as open and captures another, which
/// records nothing, so the observation's feed check does not refuse it. Its
/// persist must neither revive the cancelled use nor drop the new one, with
/// and without a batch (the clock floor) to record.
#[test]
fn adversary_d5_an_observation_built_before_a_capture_neither_revives_nor_drops_a_use() {
    use super::super::metadata::{exit_process, set_relock_hook, simulate_process};
    use std::{cell::RefCell, rc::Rc};
    simulate_process(0xad5_0004);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let capture = |now: u64| {
        Registry::new(root.path())
            .capture_read(&binding(), &reference, &BTreeSet::new(), now, now + 50_000)
            .unwrap()
    };
    // In the window: cancel `old` and capture another use, both at `at`.
    let race = |old: ReadUse, at: u64| -> Rc<RefCell<Option<ReadUse>>> {
        let pending = RefCell::new(Some(old));
        let captured = Rc::new(RefCell::new(None));
        let slot = captured.clone();
        let writer = Registry::new(root.path());
        let reference = reference.clone();
        set_relock_hook(Some(Box::new(move || {
            if let Some(old) = pending.borrow_mut().take() {
                writer.cancel_read(old, at).unwrap();
                *slot.borrow_mut() = Some(
                    writer
                        .capture_read(&binding(), &reference, &BTreeSet::new(), at, at + 50_000)
                        .unwrap(),
                );
            }
        })));
        captured
    };

    // Without a batch: the observation is inside the clock interval.
    let first = capture(NOW + 1);
    let first_id = first.id.clone();
    let events = recorded_events(root.path());
    let slot = race(first, NOW + 2);
    let observed = registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &reference,
            NOW + 3,
            true,
        )
        .map(|_| ());
    set_relock_hook(None);
    let second = slot.borrow_mut().take();
    let unrecorded = recorded_events(root.path()) - events;
    let uses = adversary_d5_uses(root.path());
    let second_id = second.as_ref().map(|second| second.id.clone());
    let dispatched = second.map(|second| registry.dispatch_read(second, NOW + 4).map(|_| ()));

    // With a batch: the observation records a clock floor.
    let third = capture(NOW + 5);
    let third_id = third.id.clone();
    let slot = race(third, NOW + 6);
    let observed_with_batch = registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &reference,
            NOW + 60_001,
            true,
        )
        .map(|_| ());
    set_relock_hook(None);
    let fourth_id = slot.borrow_mut().take().map(|fourth| fourth.id);
    let recorded = recorded_events(root.path()) - events;
    let uses_with_batch = adversary_d5_uses(root.path());
    exit_process();
    simulate_process(0);

    assert_eq!(observed, Ok(()));
    let second_id = second_id.expect("the window's capture ran");
    assert_eq!(
        unrecorded, 0,
        "the window's cancel and capture recorded events: the observation was refused and \
         replayed under the lock, which is not the case under test"
    );
    assert_eq!(
        uses,
        [
            (first_id.clone(), (false, true)),
            (second_id.clone(), (false, false))
        ]
        .into_iter()
        .collect(),
        "after the observation: the cancelled use released, the window's use open"
    );
    assert_eq!(dispatched, Some(Ok(())), "the window's use dispatches");
    assert_eq!(observed_with_batch, Ok(()));
    let fourth_id = fourth_id.expect("the second window's capture ran");
    assert!(
        recorded > 0,
        "the second observation recorded its clock floor"
    );
    assert_eq!(
        uses_with_batch,
        [
            (first_id, (false, true)),
            (second_id, (true, false)),
            (third_id, (false, true)),
            (fourth_id, (false, false))
        ]
        .into_iter()
        .collect(),
        "after the observation with a batch: no use revived, none dropped"
    );
}

/// Point 2. Three threads of one process read at once while a fourth observes
/// (its projections are built without the lifecycle lock). Every read
/// succeeds, every use ends released exactly once, none is lost, doubled or
/// recorded.
#[test]
fn adversary_d5_reads_and_observations_of_one_process_at_once_leave_every_use_released() {
    use super::super::metadata::{exit_process, simulate_process};
    const PROCESS: u64 = 0xad5_0005;
    const READERS: usize = 3;
    const READS: usize = 3;
    simulate_process(PROCESS);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let barrier = Arc::new(Barrier::new(READERS + 1));
    let mut workers = Vec::new();
    for reader in 0..=READERS {
        let path = root.path().to_owned();
        let reference = reference.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || -> Result<()> {
            simulate_process(PROCESS);
            crate::local::metadata::set_lifecycle_lock_wait(
                crate::local::metadata::CONTENDED_LOCK_WAIT,
            );
            let registry = Registry::new(&path);
            barrier.wait();
            // Every step at one instant: the registry clock never runs back.
            let now = NOW + 1;
            if reader == READERS {
                for _ in 0..READS * 2 {
                    registry.admit_read(&binding(), &reference, &BTreeSet::new(), now)?;
                }
                return Ok(());
            }
            for _ in 0..READS {
                let captured = registry.capture_read(
                    &binding(),
                    &reference,
                    &BTreeSet::new(),
                    now,
                    now + 1000,
                )?;
                let dispatched = registry.dispatch_read(captured, now)?;
                registry.release_read(dispatched, now)?;
            }
            Ok(())
        }));
    }
    let outcomes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    let uses = adversary_d5_uses(root.path());
    let recorded = recorded_read_uses(root.path());
    drop(registry);
    exit_process();
    simulate_process(0);
    assert_eq!(outcomes, vec![Ok(()); READERS + 1]);
    assert_eq!(
        (
            uses.len(),
            uses.values()
                .filter(|state| **state == (true, true))
                .count(),
            recorded
        ),
        (READERS * READS, READERS * READS, 0),
        "uses the process holds, of them dispatched and released, ReadUse events recorded: {uses:?}"
    );
}

/// Point 1, and `contracts/cli/v1alpha1/owner.md` (the reuse paragraph this
/// change extends): "A `connections revoke` run by another process does not
/// release that use; its dispatch then answers `revoked`", and "A refused
/// dispatch cancels the read use". The owner holds a reuse capture and a read
/// in flight when another process revokes the connection. The reuse's dispatch
/// answers `Revoked` and leaves no open use; the provider's refusal of the
/// read in flight is taken, changes nothing of the revoked connection and
/// records no use.
#[test]
fn adversary_d5_a_revoke_by_another_process_refuses_the_reuse_and_ends_the_read_in_flight() {
    use super::super::metadata::{exit_process, simulate_process};
    simulate_process(0xad5_0006);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = describe(&registry, &reference, NOW).revision;
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let flying = registry.dispatch_read(captured, NOW).unwrap();
    let target = Binding {
        instance_id: "target-instance".into(),
        configuration_revision: "config-target".into(),
        ..binding()
    };
    let source = registry
        .capture_reuse(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &target,
            &reference,
            &revision,
            NOW,
            NOW + 1000,
        )
        .unwrap();
    // The CLI process: `connections revoke`.
    simulate_process(0xad5_0007);
    let revoked = registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            NOW + 1,
        )
        .unwrap();
    exit_process();
    simulate_process(0xad5_0006);
    let open_before = adversary_d5_uses(root.path())
        .values()
        .filter(|state| !state.1)
        .count();
    let reuse = registry.dispatch_reuse(source, NOW + 2).err();
    let refused = registry.refuse_read(flying, NOW + 2);
    let open_after = adversary_d5_uses(root.path())
        .values()
        .filter(|state| !state.1)
        .count();
    let observed = describe(&registry, &reference, NOW + 3);
    let recorded = recorded_read_uses(root.path());
    exit_process();
    simulate_process(0);
    assert_eq!(
        open_before, 2,
        "the other process's revoke released a use the owner holds"
    );
    assert_eq!(reuse, Some(Failure::Revoked));
    assert_eq!(refused, Ok(()));
    assert_eq!(open_after, 0, "the owner still holds an open use");
    assert_eq!(
        (observed.state, observed.revision, recorded),
        (State::Revoked, revoked, 0),
        "the revoked connection's state and revision, and ReadUse events recorded"
    );
}

/// Point 4. Two persists may write only their own rows: an observation its
/// registry clock ("This check also covers additions and removals",
/// `persist_batch`), the runtime-state writer its runtime rows. Up to 0.76.0
/// both refused a persist whose SQL had added, changed or removed a read use.
/// Each attempt below is made once against a use that is recorded, as 0.76.0
/// recorded it (the control: the comparison still refuses it), and once
/// against a use the process holds.
#[test]
fn adversary_d5_a_persist_that_may_write_only_the_clock_or_runtime_rows_refuses_a_changed_use() {
    use super::super::Failure::MetadataUnavailable;
    use super::super::metadata::{exit_process, record_read_uses, simulate_process};
    let attempt = |recorded: bool, process: u64| {
        // A refused persist retires its handle, which keeps the lifecycle lock until
        // its worker ended: the next open waits for it.
        crate::local::metadata::set_lifecycle_lock_wait(
            crate::local::metadata::CONTENDED_LOCK_WAIT,
        );
        simulate_process(process);
        let (root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let reference = publish_fixture(&registry, candidate, NOW);
        record_read_uses(recorded);
        let captured = registry
            .capture_read(
                &binding(),
                &reference,
                &BTreeSet::new(),
                NOW + 1,
                NOW + 1001,
            )
            .unwrap();
        record_read_uses(false);
        // An observation, replayed outside the lock and relocked, whose SQL
        // gained a use.
        let mut observation = Metadata::update_observation(root.path()).unwrap();
        observation.relock_observation().unwrap();
        let added = observation
            .connection
            .execute(
                "INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms) \
                 SELECT ?1,connection_ref,generation_id,version_id,publication_fence,expires_at_ms \
                 FROM registry_uses WHERE use_id=?2",
                [&uuid::Uuid::new_v4().to_string(), &captured.id],
            )
            .unwrap();
        assert_eq!(
            added, 1,
            "the observation's projection holds the captured use"
        );
        let inserted = observation.persist_prepared_observation();
        drop(observation);
        // An observation replayed under the lock whose SQL released the use.
        let mut observation = Metadata::update(root.path(), false).unwrap();
        let changed = observation
            .connection
            .execute(
                "UPDATE registry_uses SET released=1 WHERE use_id=?1",
                [&captured.id],
            )
            .unwrap();
        assert_eq!(changed, 1);
        let released = observation.persist_prepared_observation();
        drop(observation);
        // The runtime-state writer (`runtime/state.rs`) whose SQL lost the use.
        let mut runtime = Metadata::update(root.path(), true).unwrap();
        let removed = runtime
            .connection
            .execute("DELETE FROM registry_uses WHERE use_id=?1", [&captured.id])
            .unwrap();
        assert_eq!(removed, 1);
        let deleted = runtime.persist_runtime_state();
        drop(runtime);
        // Refused persists change nothing: the use still dispatches.
        let dispatched = registry.dispatch_read(captured, NOW + 2).map(|_| ());
        exit_process();
        simulate_process(0);
        (inserted, released, deleted, dispatched)
    };
    let (_, released, deleted, dispatched) = attempt(true, 0xad5_0008);
    assert_eq!(
        (released, deleted, dispatched),
        (Err(MetadataUnavailable), Err(MetadataUnavailable), Ok(())),
        "control, a recorded use: the observation that released it, the runtime-state persist \
         that removed it, its dispatch afterwards"
    );
    assert_eq!(
        attempt(false, 0xad5_0009),
        (
            Err(MetadataUnavailable),
            Err(MetadataUnavailable),
            Err(MetadataUnavailable),
            Ok(())
        ),
        "a use the process holds: the observation that added one, the observation that released \
         it, the runtime-state persist that removed it, its dispatch afterwards"
    );
}

/// Design, `persist`: "a key that is not in the build snapshot is a new use:
/// it is admitted only where a recorded creation is admitted today
/// (`admitted_registry_use`, a row that is Captured, not dispatched and not
/// released)". A persist that is no admission, and an admission whose new use
/// is already dispatched, are refused and leave what the process held; an
/// admission of a captured use holds it.
#[test]
fn adversary_d5_only_an_admission_holds_a_new_use_and_only_a_captured_one() {
    use super::super::Failure::MetadataUnavailable;
    use super::super::metadata::{exit_process, simulate_process};
    crate::local::metadata::set_lifecycle_lock_wait(crate::local::metadata::CONTENDED_LOCK_WAIT);
    simulate_process(0xad5_000a);
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let captured = registry
        .capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            NOW + 1,
            NOW + 1001,
        )
        .unwrap();
    // A handle built under the lifecycle lock whose SQL gained a use.
    let with_new_use = |dispatched: bool| {
        let metadata = Metadata::update(root.path(), false).unwrap();
        let added = metadata
            .connection
            .execute(
                "INSERT INTO registry_uses (use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms,dispatched) \
                 SELECT ?1,connection_ref,generation_id,version_id,publication_fence,expires_at_ms,?3 \
                 FROM registry_uses WHERE use_id=?2",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), captured.id, dispatched],
            )
            .unwrap();
        assert_eq!(added, 1);
        metadata
    };
    let mut metadata = with_new_use(false);
    let no_admission = metadata.persist();
    drop(metadata);
    let mut metadata = with_new_use(true);
    let dispatched_admission = metadata.persist_admission();
    drop(metadata);
    let held_after_refusals = adversary_d5_uses(root.path()).len();
    let mut metadata = with_new_use(false);
    let admission = metadata.persist_admission();
    drop(metadata);
    let held_after_admission = adversary_d5_uses(root.path()).len();
    exit_process();
    simulate_process(0);
    assert_eq!(
        (no_admission, dispatched_admission, held_after_refusals),
        (Err(MetadataUnavailable), Err(MetadataUnavailable), 1),
        "a persist that is no admission, an admission of a dispatched use, uses held afterwards"
    );
    assert_eq!(
        (admission, held_after_admission),
        (Ok(()), 2),
        "an admission of a captured use, uses held afterwards"
    );
}

/// `cargo test -p connectors-host` (`.github/workflows/macos-host.yml`,
/// `windows-host.yml`) runs the tests of this binary as threads of one process.
/// The helpers `read_invoke`, `read_invoke_with_distinct_step_clocks` and
/// `measured_invoke` act as the owner, process 1, from capture to release, and
/// the tests this change added end process 1 and process 2 (`exit_process`).
/// Up to 0.76.0 that closed the other thread's idle handles, which it opened
/// again. This test holds a use as those helpers do while another thread does
/// what `a_revoke_by_another_process_refuses_the_owners_dispatch_as_revoked`,
/// `the_read_uses_of_an_ended_process_are_gone_with_it` and
/// `read_invokes_inside_one_clock_interval_record_nothing` do in their last
/// lines: the use must still dispatch. The control records the use, as 0.76.0
/// did.
#[test]
fn adversary_d5_a_test_that_ends_process_1_does_not_end_the_uses_of_another_test_thread() {
    use super::super::metadata::{exit_process, record_read_uses, simulate_process};
    let dispatch_beside_another_test = |recorded: bool| {
        let (_root, registry) = fixture();
        let (_, candidate) = prepared(&registry, "one", NOW);
        let reference = publish_fixture(&registry, candidate, NOW);
        // As `read_invoke` (the owner's part of an invoke).
        simulate_process(1);
        record_read_uses(recorded);
        let captured = registry
            .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
            .unwrap();
        record_read_uses(false);
        // The end of another test, on that test's thread.
        std::thread::spawn(|| {
            simulate_process(1);
            exit_process();
            simulate_process(0);
        })
        .join()
        .unwrap();
        let dispatched = registry.dispatch_read(captured, NOW).map(|_| ());
        simulate_process(0);
        dispatched
    };
    assert_eq!(
        dispatch_beside_another_test(true),
        Ok(()),
        "control: a recorded use dispatches after another test thread ended process 1"
    );
    assert_eq!(
        dispatch_beside_another_test(false),
        Ok(()),
        "a held use dispatches after another test thread ended process 1"
    );
}
