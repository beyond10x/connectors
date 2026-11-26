use super::*;
use std::{sync::atomic::Ordering, time::Duration};

#[test]
fn production_finish_audit_recovers_a_lost_acknowledgement_on_the_real_clock() {
    let (_root, store) = audit::tests::fixture();
    let request = Uuid::new_v4().to_string();
    let mut facts = audit::tests::anchor();
    facts.request_id = Some(request.clone());
    let audit::Acknowledgement::Execution(admission) = store.anchor(&facts).unwrap() else {
        panic!("missing audit admission");
    };
    let reference = store.confirm(admission, &facts).unwrap();
    let mut value = Delivery {
        request_id: request.clone(),
        result: Some(serde_json::json!({"retained":"native result"})),
        error: None,
        mutation: MutationObservation {
            classification: Classification::Applied,
            attempt: Some(Attempt {
                instance: "alpha".into(),
                id: Uuid::new_v4(),
            }),
            original_request_id: Some(request),
            replayed: false,
            cause: Some(Cause {
                code: connectors_core::ErrorCode::Unavailable,
                stage: Stage::Response,
            }),
            standing: None,
        },
        source_audit: SourceAudit {
            instance: "alpha".into(),
            audit_ref: None,
            audit_status: AuditStatus::Unavailable,
        },
    };
    value.validate().unwrap();
    store.fault.store(2, Ordering::SeqCst);
    finish_audit(
        &store,
        reference,
        &mut value,
        Instant::now() + Duration::from_secs(60),
    );
    assert_eq!(value.source_audit.audit_status, AuditStatus::Complete);
}

#[test]
fn final_audit_recovery_preserves_every_live_business_result() {
    for (classification, applied_error) in [
        (Classification::Applied, false),
        (Classification::Applied, true),
        (Classification::Refused, false),
        (Classification::NotAttempted, false),
        (Classification::Unknown, false),
    ] {
        for (fault, expired, expected) in [
            (0, false, AuditStatus::Complete),
            (1, false, AuditStatus::Complete),
            (2, false, AuditStatus::Complete),
            (9, false, AuditStatus::Complete),
            (10, false, AuditStatus::Incomplete),
            (1, true, AuditStatus::Incomplete),
            (2, true, AuditStatus::Incomplete),
        ] {
            let (_root, store) = audit::tests::fixture();
            let request = Uuid::new_v4().to_string();
            let mut facts = audit::tests::anchor();
            facts.request_id = Some(request.clone());
            let audit::Acknowledgement::Execution(admission) = store.anchor(&facts).unwrap() else {
                panic!("missing audit admission");
            };
            let reference = store.confirm(admission, &facts).unwrap();
            let mut value = Delivery {
                request_id: request.clone(),
                result: (classification == Classification::Applied && !applied_error)
                    .then(|| serde_json::json!({"retained":"native result"})),
                error: match classification {
                    Classification::Applied if !applied_error => None,
                    Classification::Applied => Some(Failure {
                        code: FailureCode::Owner(Code::ServiceFailure),
                        service_code: Some(connectors_core::ErrorCode::UpstreamProtocol),
                        origin: crate::local::owner::Origin::Host,
                        store_version: None,
                        created: None,
                        retry_after_seconds: None,
                    }),
                    Classification::Unknown => Some(Code::OutcomeUnknown.into()),
                    _ => Some(Code::Forbidden.into()),
                },
                mutation: MutationObservation {
                    classification,
                    attempt: Some(Attempt {
                        instance: "alpha".into(),
                        id: Uuid::new_v4(),
                    }),
                    original_request_id: Some(request),
                    replayed: false,
                    cause: Some(Cause {
                        code: connectors_core::ErrorCode::Unavailable,
                        stage: Stage::Response,
                    }),
                    standing: None,
                },
                source_audit: SourceAudit {
                    instance: "alpha".into(),
                    audit_ref: None,
                    audit_status: AuditStatus::Unavailable,
                },
            };
            value.validate().unwrap();
            let mut before = serde_json::to_value(&value).unwrap();
            before.as_object_mut().unwrap().remove("source_audit");
            store.fault.store(fault, Ordering::SeqCst);
            // A frozen clock: the live cases keep their whole recovery window
            // however slowly storage runs, and the expired ones start past it.
            let start = Instant::now();
            let until = if expired {
                start
            } else {
                start + Duration::from_secs(1)
            };
            finish_audit_with_now(&store, reference.clone(), &mut value, until, || start);
            value.validate().unwrap();
            assert_eq!(value.source_audit.audit_status, expected);
            assert_eq!(
                value.source_audit.audit_ref.as_deref(),
                Some(reference.audit_ref.as_str())
            );
            let mut after = serde_json::to_value(&value).unwrap();
            after.as_object_mut().unwrap().remove("source_audit");
            assert_eq!(after, before);
            let record = store.observe(&reference).unwrap().unwrap();
            if let Some(observation) = record.final_observation {
                assert_eq!(
                    observation.outcome,
                    match classification {
                        Classification::Applied if !applied_error => audit::Outcome::Success,
                        Classification::Applied => audit::Outcome::Error,
                        Classification::Unknown => audit::Outcome::Unknown,
                        _ => audit::Outcome::Refused,
                    }
                );
            } else {
                assert!(matches!((fault, expired), (1, true) | (10, false)));
            }
        }
    }
}

/// A settled write's stored outcome, replayed after the owner restarts: the
/// native result passes through the owner's own settlement projection, the
/// durable ledger and a fresh store handle, as `observe` reads it.
mod replay {
    use super::*;
    use crate::local::{metadata::Metadata, owner::Origin, runtime};
    use connectors_core::ErrorCode;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    struct Fixed;
    impl ledger::Clock for Fixed {
        fn now(&self) -> ledger::Result<ledger::ClockInterval> {
            Ok(ledger::ClockInterval {
                lower_unix_ms: 1_789_056_000_000,
                upper_unix_ms: 1_789_056_002_000,
            })
        }
    }
    fn root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        #[cfg(windows)]
        crate::local::filesystem::windows::owner_only(root.path());
        drop(Metadata::initialize(root.path()).unwrap());
        let mut metadata = Metadata::update_mutations(root.path()).unwrap();
        metadata
            .connection
            .execute_batch(
                "INSERT INTO registry_instances VALUES ('instance','adapter','config',0);",
            )
            .unwrap();
        let binding = crate::local::registry::fixture_binding("instance");
        metadata
            .connection
            .execute(
                "INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1',?1)",
                [serde_json::to_string(&binding.profile).unwrap()],
            )
            .unwrap();
        metadata.connection.execute(
            "INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms)
             VALUES ('connection','instance','profile',?1,'scope','revision','fence','live',1,1)",
            [serde_json::to_string(&binding).unwrap()],
        ).unwrap();
        metadata.persist().unwrap();
        root
    }
    fn candidate() -> ledger::Candidate {
        ledger::Candidate {
            namespace: ledger::Namespace {
                receiver_instance: "instance".into(),
                tenant: None,
                realm: None,
                caller: "caller".into(),
                executor: None,
                origin: ledger::Origin::Direct,
            },
            fingerprint: ledger::Fingerprint {
                operation: ledger::OperationRef {
                    instance: "instance".into(),
                    adapter: "adapter".into(),
                    operation: "operation".into(),
                },
                connection_ref: Some("connection".into()),
                connection_revision: Some("revision".into()),
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
            approval: ledger::Approval::NotRequired,
        }
    }
    /// Settle one refused attempt with `payload`, drop every handle, then read
    /// it back through a new store the way a restarted owner does.
    fn replay(payload: Value) -> (Classification, Failure) {
        let root = root();
        {
            let store = ledger::Store::new(root.path(), Fixed, ledger::Limits::default()).unwrap();
            let ledger::Preparation::Prepared(prepared) = store.prepare(&candidate()).unwrap()
            else {
                panic!("unexpected existing attempt");
            };
            let reference = prepared.reference();
            store.open_dispatch(prepared).unwrap().consume().unwrap();
            store
                .settle(reference, &ledger::Outcome::Refused(payload))
                .unwrap();
        }
        let store = ledger::Store::new(root.path(), NoClock, ledger::Limits::default()).unwrap();
        let mut retry = candidate();
        retry.request_id = "retry-request".into();
        let original = store.lookup(&retry).unwrap().unwrap();
        match replayed(&original).unwrap() {
            (classification, StoredOutcome::Failure { error }) => (classification, error),
            (_, StoredOutcome::Success { .. }) => panic!("a refusal replayed as success"),
        }
    }
    fn refused(failure: runtime::Failure) -> Value {
        let (classification, outcome) = native_outcome(runtime::WriteResult {
            effect: runtime::WriteEffect::Refused,
            result: Err(failure),
        });
        assert_eq!(classification, Classification::Refused);
        serde_json::to_value(StoredOutcome::from(outcome)).unwrap()
    }

    /// A clock whose interval is `width` ms wide from a fixed lower bound.
    struct Width(std::sync::Arc<std::sync::atomic::AtomicI64>);
    impl ledger::Clock for Width {
        fn now(&self) -> ledger::Result<ledger::ClockInterval> {
            Ok(ledger::ClockInterval {
                lower_unix_ms: 1_789_056_000_000,
                upper_unix_ms: 1_789_056_000_000 + self.0.load(Ordering::SeqCst),
            })
        }
    }

    /// The ledger settles on an interval as wide as the clock's own 8,000 ms
    /// cap (docs/local-mutation-ledger.md) and refuses 8,001 ms.
    #[test]
    fn ledger_clock_admits_8000_ms() {
        for (width, admitted) in [(8000, true), (8001, false)] {
            let root = root();
            let wide = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(2000));
            let store =
                ledger::Store::new(root.path(), Width(wide.clone()), ledger::Limits::default())
                    .unwrap();
            let ledger::Preparation::Prepared(prepared) = store.prepare(&candidate()).unwrap()
            else {
                panic!("unexpected existing attempt");
            };
            let reference = prepared.reference();
            store.open_dispatch(prepared).unwrap().consume().unwrap();
            wide.store(width, Ordering::SeqCst);
            let settled = store.settle(
                reference,
                &ledger::Outcome::Refused(serde_json::json!({"code":"conflict"})),
            );
            if admitted {
                settled.unwrap();
            } else {
                assert!(
                    matches!(settled, Err(ledger::Failure::ClockUnavailable)),
                    "{width}"
                );
            }
        }
    }

    #[test]
    fn a_stored_provider_refusal_replays_at_dispatch_with_its_next_action_after_a_restart() {
        for (failure, code, service_code, action) in [
            (
                runtime::Failure::ProviderForbidden,
                Code::Forbidden,
                None,
                "request_permission",
            ),
            (
                runtime::Failure::ProviderNotFound,
                Code::ServiceFailure,
                Some(ErrorCode::NotFound),
                "none",
            ),
        ] {
            let live = native_outcome(runtime::WriteResult {
                effect: runtime::WriteEffect::Refused,
                result: Err(failure),
            })
            .1
            .unwrap_err();
            assert_eq!(live.origin, Origin::Provider);
            assert_eq!(
                live.next_action(Classification::Refused),
                action,
                "{failure:?} live"
            );
            let stored = refused(failure);
            assert_eq!(stored["error"]["origin"], "provider", "{stored}");
            let (classification, error) = replay(stored);
            assert_eq!(classification, Classification::Refused);
            assert_eq!(error.code, FailureCode::Owner(code));
            assert_eq!(error.service_code, service_code);
            assert_eq!(error.origin, Origin::Provider);
            assert_eq!(
                error.next_action(Classification::Refused),
                action,
                "{failure:?} replayed"
            );
            assert_eq!(
                error.stage(classification),
                "dispatch",
                "{failure:?} replayed"
            );
        }
    }

    #[test]
    fn a_host_refusal_is_stored_without_an_origin_and_keeps_retry_status() {
        // A host-origin failure writes no `origin`, so the entry is the same
        // bytes a binary predating this change wrote and still reads.
        let stored = refused(runtime::Failure::Forbidden);
        assert_eq!(
            stored,
            serde_json::json!({"kind":"failure","error":{"code":"forbidden"}})
        );
        let (_, error) = replay(stored);
        assert_eq!(error.origin, Origin::Host);
        assert_eq!(error.next_action(Classification::Refused), "retry_status");
        let approval: Failure = ApprovalCode::ApprovalRefused.into();
        assert_eq!(
            approval.next_action(Classification::NotAttempted),
            "retry_status"
        );
        assert_eq!(approval.stage(Classification::NotAttempted), "admission");
    }

    /// semantics.md §3 on the owner path: the owner's ledger on a store a
    /// newer binary with another record model wrote is refused as
    /// `metadata_schema_newer`, and the owner's write failure keeps the
    /// store's release, the `admission` stage and `upgrade`, as the direct
    /// path answers; it survives the stored outcome's round trip.
    #[test]
    fn an_owner_write_on_a_store_a_newer_binary_wrote_keeps_its_release() {
        let root = root();
        Metadata::record_writer_for_tests(root.path(), "999.0.0", &"0".repeat(64)).unwrap();
        let refused = ledger::Store::new(root.path(), Fixed, ledger::Limits::default())
            .and_then(|store| store.prepare(&candidate()).map(drop))
            .unwrap_err();
        assert_eq!(
            refused,
            ledger::Failure::MetadataSchemaNewer(Some(crate::local::Release::new(999, 0, 0)))
        );
        let failure = execution::ledger_error(refused);
        assert_eq!(failure.code, FailureCode::Owner(Code::MetadataSchemaNewer));
        assert_eq!(failure.stage(Classification::NotAttempted), "admission");
        assert_eq!(failure.next_action(Classification::NotAttempted), "upgrade");
        let stored = serde_json::to_value(StoredOutcome::Failure { error: failure }).unwrap();
        assert_eq!(
            stored,
            serde_json::json!({"kind":"failure","error":{"code":"metadata_schema_newer",
                "store_version":"999.0.0"}})
        );
        let StoredOutcome::Failure { error } = serde_json::from_value(stored).unwrap() else {
            panic!("a failure read back as success");
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap()["store_version"],
            "999.0.0"
        );
    }

    #[test]
    fn a_ledger_entry_written_before_the_origin_existed_still_replays() {
        for payload in [
            serde_json::json!({"kind":"failure","error":{"code":"forbidden"}}),
            serde_json::json!({"kind":"failure","error":{"code":"service_failure","service_code":"not_found"}}),
            serde_json::json!({"kind":"failure","error":{"code":"approval_refused"}}),
        ] {
            let (classification, error) = replay(payload.clone());
            assert_eq!(classification, Classification::Refused, "{payload}");
            assert_eq!(error.origin, Origin::Host, "{payload}");
            assert_eq!(
                error.next_action(Classification::Refused),
                "retry_status",
                "{payload}"
            );
        }
    }
}

/// The stage says who refused and whether anything was sent; the next action
/// never invites a plain retry of a write that may have taken effect.
#[test]
fn a_write_failure_names_its_stage_and_never_suggests_retrying_a_possible_effect() {
    use crate::local::owner::Origin;
    let failure = |code: Code, origin: Origin| {
        let mut error = Error::from(code);
        error.origin = origin;
        Failure::from(error)
    };
    let approval: Failure = ApprovalCode::ApprovalRequired.into();
    for (error, classification, stage, action) in [
        // The host refused before anything was sent.
        (
            failure(Code::Timeout, Origin::Host),
            Classification::NotAttempted,
            "admission",
            "retry_status",
        ),
        (
            failure(Code::Forbidden, Origin::Host),
            Classification::NotAttempted,
            "admission",
            "retry_status",
        ),
        (
            approval,
            Classification::NotAttempted,
            "admission",
            "retry_status",
        ),
        // The provider answered the preflight.
        (
            failure(Code::Timeout, Origin::Provider),
            Classification::NotAttempted,
            "dispatch",
            "retry_explicitly",
        ),
        (
            failure(Code::Forbidden, Origin::Provider),
            Classification::NotAttempted,
            "dispatch",
            "request_permission",
        ),
        // A write that was sent, or may have been, is never retried plainly.
        (
            failure(Code::Timeout, Origin::Provider),
            Classification::Applied,
            "dispatch",
            "retry_status",
        ),
        (
            failure(Code::Capacity, Origin::Provider),
            Classification::Unknown,
            "dispatch",
            "retry_status",
        ),
        (
            failure(Code::OutcomeUnknown, Origin::Host),
            Classification::Unknown,
            "dispatch",
            "retry_status",
        ),
        (
            failure(Code::Forbidden, Origin::Host),
            Classification::Refused,
            "dispatch",
            "retry_status",
        ),
    ] {
        assert_eq!(
            error.stage(classification),
            stage,
            "{error:?} {classification:?}"
        );
        assert_eq!(
            error.next_action(classification),
            action,
            "{error:?} {classification:?}"
        );
    }
}

/// story:local-invoke-admits-a-keyed-write-under-a-standing-grant, acceptance
/// 15: `standing::named` and the bytes of the observation and of `Failure`.
#[cfg(unix)]
#[test]
fn standing_is_named_for_an_attempt_and_absent_without_a_store() {
    use crate::local::standing;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    /// A private state directory below a temporary one.
    fn state() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
        (root, state)
    }
    let delivery = |attempt: Option<Uuid>, error: Option<Failure>| {
        let classification = match (&error, attempt) {
            (None, _) => Classification::Applied,
            (Some(_), Some(_)) => Classification::Refused,
            (Some(_), None) => Classification::NotAttempted,
        };
        Delivery {
            request_id: Uuid::new_v4().to_string(),
            result: error.is_none().then(|| json!({"status": 200})),
            error,
            mutation: MutationObservation {
                classification,
                attempt: attempt.map(|id| Attempt {
                    instance: "leaf".into(),
                    id,
                }),
                original_request_id: attempt.map(|_| "request-1".to_owned()),
                replayed: false,
                cause: None,
                standing: None,
            },
            source_audit: SourceAudit {
                instance: "leaf".into(),
                audit_ref: None,
                audit_status: AuditStatus::Unavailable,
            },
        }
    };
    let named = |mut value: Delivery, attempt: Uuid, state: &std::path::Path| {
        standing::named(&mut value.mutation, attempt, state);
        value.validate().unwrap();
        // The bytes the owner sends are the bytes the CLI reads back.
        let bytes = serde_json::to_value(&value).unwrap();
        let read: Delivery = serde_json::from_value(bytes.clone()).unwrap();
        assert_eq!(read.mutation.standing, value.mutation.standing);
        bytes
    };

    // One use bound to an attempt, one counted and bound to none.
    let (_root, directory) = state();
    let grant = standing::Grant {
        id: "g1".into(),
        instance: "leaf".into(),
        operation: "chat.postMessage".into(),
        target: vec![standing::TargetMember {
            pointer: "/body/channel".into(),
            bound: standing::Bound::Value(json!("C1")),
        }],
        input_keys: None,
        not_after_unix_ms: 1_789_056_600_000,
        max_per_hour: 2,
    };
    let uses = standing::Uses::create(&directory).unwrap();
    let take = || match uses.take("leaf", &[&grant], 1_789_056_002_000).unwrap() {
        standing::Taken::Slot(taken) => taken,
        standing::Taken::AtCap { .. } => panic!("at the cap"),
    };
    let (bound, unbound) = (Uuid::new_v4(), Uuid::new_v4());
    uses.bind(&take(), bound).unwrap();
    let _counted = take();

    // A bound attempt names its grant, on the success shape and on the
    // failure shape.
    let success = named(delivery(Some(bound), None), bound, &directory);
    assert_eq!(success["mutation"]["standing"], json!({"grant_id": "g1"}));
    assert!(success["result"].is_object());
    let failed = named(
        delivery(Some(bound), Some(Code::ServiceFailure.into())),
        bound,
        &directory,
    );
    assert_eq!(failed["mutation"]["standing"], json!({"grant_id": "g1"}));
    assert_eq!(failed["error"]["code"], "service_failure");

    // An attempt with no row has no member, not a null one.
    let other = named(delivery(Some(unbound), None), unbound, &directory);
    assert!(other["mutation"].get("standing").is_none(), "{other}");

    // With no store file the member is absent and none is created.
    let (_empty_root, empty) = state();
    let absent = named(delivery(Some(bound), None), bound, &empty);
    assert!(absent["mutation"].get("standing").is_none(), "{absent}");
    assert!(standing::Uses::existing(&empty).unwrap().is_none());
    assert!(!empty.join("standing-grants.sqlite3").exists());
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    // The store that exists is read, not written: a row count as before.
    assert_eq!(uses.rows().unwrap(), 2);

    // `rate_limited` carries `retry_after_seconds`; no other failure does.
    let limited = delivery(None, Some(Failure::rate_limited(1800)));
    limited.validate().unwrap();
    let limited = serde_json::to_value(&limited).unwrap();
    assert_eq!(
        limited["error"],
        json!({"code": "rate_limited", "retry_after_seconds": 1800})
    );
    assert_eq!(limited["mutation"]["classification"], "not_attempted");
    let read: Failure = serde_json::from_value(limited["error"].clone()).unwrap();
    assert_eq!(read.code, FailureCode::Approval(ApprovalCode::RateLimited));
    assert_eq!(read.retry_after_seconds, Some(1800));
    assert_eq!(read.stage(Classification::NotAttempted), "admission");
    assert_eq!(
        read.next_action(Classification::NotAttempted),
        "retry_status"
    );
    assert_eq!(
        serde_json::to_value(Failure::from(ApprovalCode::ApprovalRequired)).unwrap(),
        json!({"code": "approval_required"})
    );

    // What the owner never sends: a grant named without an attempt, a cap
    // without its number or with one outside 1 to 3,600, the number on
    // another code.
    let mut unattempted = delivery(None, Some(ApprovalCode::ApprovalRequired.into()));
    unattempted.mutation.standing = Some(Standing {
        grant_id: "g1".into(),
    });
    assert!(unattempted.validate().is_err());
    for refused in [
        Failure::from(ApprovalCode::RateLimited),
        Failure::rate_limited(0),
        Failure::rate_limited(3601),
        Failure {
            retry_after_seconds: Some(60),
            ..Failure::from(ApprovalCode::ApprovalRequired)
        },
    ] {
        assert!(delivery(None, Some(refused)).validate().is_err());
    }
}

/// story:local-invoke-admits-a-keyed-write-under-a-standing-grant: the two
/// rechecks of the grant path, through `execute` itself on a live child of
/// this test binary. `execution::seam` gives the credential in place of the
/// custody read, and its hook revokes the grant between two steps as
/// `approvals revoke-standing` does: under `standing-grants.lock` and the
/// shared key lease, with the instance's active key.
#[cfg(unix)]
mod recheck {
    use super::super::execution::seam;
    use super::*;
    use crate::local::{
        approvals, clock, clock::fixture as clock_fixture, config, keyring::custody, metadata,
        owner::lifecycle, registry, runtime, standing,
    };
    use base64::{
        Engine,
        engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    };
    use connectors_sdk::Secret;
    use ring::{
        rand::{SecureRandom, SystemRandom},
        signature::{Ed25519KeyPair, KeyPair},
    };
    use std::{
        cell::Cell,
        collections::BTreeSet,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        rc::Rc,
        sync::{Mutex, atomic::AtomicBool},
    };

    /// Every wait of these tests: a bound only a hang reaches, inside the
    /// 60 s a test may run.
    const WAIT: Duration = Duration::from_secs(50);

    /// One instance `fixture` with a live child, a saved connection, a
    /// published policy, an active approval key generated here and one grant
    /// of `write` for `/value` true, once an hour.
    struct Site {
        child: runtime::Child,
        /// The two clock sources. Each drops as many of its next requests as
        /// a test stores in `drops`, and is silent while `respond` is false.
        clocks: [clock_fixture::Server; 2],
        paths: Paths,
        /// Where the child records `prepare`, `send` and `destroy`.
        events: PathBuf,
        grants: PathBuf,
        connection: String,
        schema: String,
        grant_id: String,
        seed: [u8; 32],
        kid: String,
        _root: tempfile::TempDir,
    }

    fn site() -> Site {
        metadata::set_lifecycle_lock_wait(WAIT);
        clock::set_test_query_timeout(WAIT);
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let events = root.path().join("events");
        std::fs::create_dir(&events).unwrap();
        let grants = root.path().join("fixture.grants");
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        config::Config::initialize(&paths).unwrap();
        let clocks = [7, 8].map(|seed| {
            clock_fixture::Server::dropping(clock_fixture::Fixture {
                seed,
                ..Default::default()
            })
        });
        let mut config = config::Config::load(&paths.config).unwrap();
        config.format = "connectors-local/2".into();
        config.secret_service_socket = Some(root.path().join("absent-bus"));
        config.approval_clock = Some(clock::ApprovalClock::Quorum(clock::Configuration {
            format: "roughtime-quorum/1".into(),
            quorum: 2,
            max_rate_error_ppm: 100,
            sources: clocks
                .iter()
                .zip([7, 8])
                .map(|(server, seed)| clock::Source {
                    address: server.address.clone(),
                    public_key: STANDARD.encode(clock_fixture::key(seed)),
                    version: clock::RoughtimeVersion::Draft0x8000000c,
                })
                .collect(),
        }));
        let (path, sha256) = crate::local::test_executable::path_and_sha256();
        let adapter = config::Adapter {
            standing_grants: Some(grants.clone()),
            credential_revision: None,
            instance_id: "fixture".into(),
            adapter_id: "fixture".into(),
            configuration_revision: "fixture-config".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: config::Executable {
                path,
                sha256,
                args: vec![
                    "--exact".into(),
                    "local::runtime::process::write_tests::fixture".into(),
                    "--".into(),
                    "write-mode=owned".into(),
                    format!("write-root={}", events.display()),
                ],
            },
            permissions: config::Permissions {
                profiles: BTreeSet::from(["fixture".into()]),
                operations: BTreeSet::from(["write".into(), "read".into()]),
                custody: Default::default(),
            },
        };
        config.adapters.insert("fixture".into(), adapter.clone());
        std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();

        // The child and its description, cached as the owner caches it.
        let child = runtime::Child::spawn_within(&adapter, WAIT).unwrap();
        let bootstrap = child.bootstrap().clone();
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
        let schema = crate::local::owner::schema(&bootstrap, "write").unwrap();

        // A saved connection whose validation is fresh on the system clock.
        let now = connectors_sdk::now_ms();
        let registry = registry::Registry::new(&paths.state);
        let acquisition = registry
            .begin(&bootstrap.binding("fixture").unwrap(), now)
            .unwrap();
        let claim = registry.consume(acquisition, now).unwrap();
        let candidate = registry
            .prepare(
                &claim,
                registry::ValidatedBaseline {
                    binding: None,
                    identity: registry::ExternalIdentity {
                        kind: "fixture".into(),
                        subject: "owner".into(),
                    },
                    granted_scopes: Some(BTreeSet::new()),
                    credential_expires_at_ms: None,
                    collected_at_ms: now,
                    valid_until_ms: now + 60_000,
                },
                12,
                now,
            )
            .unwrap();
        let receipt = custody::WrittenVersion::fixture(candidate.version());
        let stored = registry.acknowledge(candidate, receipt, now).unwrap();
        let connection = registry.publish(stored, now).unwrap();

        // The issuer and its active key, for a pair generated here: the rows
        // `approval_keys::Store` writes, a candidate first and its
        // publication second. The seed never enters a custody; the tests
        // sign with it directly.
        let mut seed = [0u8; 32];
        SystemRandom::new().fill(&mut seed).unwrap();
        let pair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let (issuer, kid) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
        let mut store = metadata::Metadata::update_approval_keys(&paths.state).unwrap();
        store
            .connection
            .execute(
                "INSERT INTO local_approval_issuers VALUES (?1,'fixture',?2,?3)",
                rusqlite::params![
                    issuer,
                    Uuid::new_v4().to_string(),
                    Uuid::new_v4().to_string()
                ],
            )
            .unwrap();
        store
            .connection
            .execute(
                "INSERT INTO local_approval_keys VALUES (?1,?2,?3,?4,'candidate')",
                rusqlite::params![
                    kid,
                    issuer,
                    URL_SAFE_NO_PAD.encode(pair.public_key().as_ref()),
                    Uuid::new_v4().to_string()
                ],
            )
            .unwrap();
        store.persist().unwrap();
        drop(store);
        let mut store = metadata::Metadata::update_approval_keys(&paths.state).unwrap();
        store
            .connection
            .execute(
                "UPDATE local_approval_keys SET state='active' WHERE key_id=?1 AND state='candidate'",
                [&kid],
            )
            .unwrap();
        store
            .connection
            .execute(
                "UPDATE local_approval_issuers SET revision=?1 WHERE issuer_id=?2",
                rusqlite::params![Uuid::new_v4().to_string(), issuer],
            )
            .unwrap();
        store.persist().unwrap();
        drop(store);
        // The lock file of the key lease, which the store's first exclusive
        // operation leaves; a shared lease does not create it.
        drop(
            crate::local::approval_keys::Lease::acquire(&paths.state, "approval-issuer.lock", true)
                .unwrap(),
        );
        issuance::policy_set(
            &paths,
            "fixture",
            br#"{"operations":["write"]}"#,
            None,
            Instant::now() + WAIT,
        )
        .unwrap();

        // The grant, signed as `approvals grant-standing` signs it; this is
        // also what creates the use store.
        let reading =
            standing::reading(&config.approval_clock.as_ref().unwrap().acquire().unwrap()).unwrap();
        let request = serde_json::to_vec(&serde_json::json!({
            "operation": "write",
            "target": [{"pointer": "/value", "value": true}],
            "not_after_unix_ms": reading.upper_unix_ms + 3_600_000,
            "max_per_hour": 1,
        }))
        .unwrap();
        let grant_id = {
            let lock = standing::lock(&paths.state).unwrap();
            let keys = issuance::key_store(&paths, &config, &adapter)
                .unwrap()
                .acquire_key()
                .unwrap();
            standing::grant(
                &lock,
                &grants,
                &approvals::Signer::from_seed(Secret(seed.to_vec()), kid.clone()).unwrap(),
                keys.key().unwrap(),
                "fixture",
                reading,
                &request,
            )
            .unwrap()
        };
        Site {
            child,
            clocks,
            paths,
            events,
            grants,
            connection,
            schema,
            grant_id,
            seed,
            kid,
            _root: root,
        }
    }

    /// The keyed write `{"value":true}` through `execute`. The grant is
    /// revoked when `execute` reaches the step `revoke_at`; the second value
    /// says how often it was.
    fn write(site: &mut Site, revoke_at: seam::Step) -> (Delivery, u32) {
        write_within(site, WAIT, Some(revoke_at))
    }

    /// [`write`] with its deadline `wait` after the call, and the grant
    /// revoked at `revoke_at` or, with `None`, never.
    fn write_within(
        site: &mut Site,
        wait: Duration,
        revoke_at: Option<seam::Step>,
    ) -> (Delivery, u32) {
        let revoked = Rc::new(Cell::new(0));
        let hook = {
            let revoked = revoked.clone();
            let paths = Paths {
                config: site.paths.config.clone(),
                state: site.paths.state.clone(),
            };
            let (grants, grant_id) = (site.grants.clone(), site.grant_id.clone());
            let (seed, kid) = (site.seed, site.kid.clone());
            move |step: seam::Step| {
                if Some(step) != revoke_at {
                    return;
                }
                let config = config::Config::load(&paths.config).unwrap();
                let adapter = config.adapters["fixture"].clone();
                let reading =
                    standing::reading(&config.approval_clock.as_ref().unwrap().acquire().unwrap())
                        .unwrap();
                let lock = standing::lock(&paths.state).unwrap();
                let keys = issuance::key_store(&paths, &config, &adapter)
                    .unwrap()
                    .acquire_key()
                    .unwrap();
                standing::revoke(
                    &lock,
                    &grants,
                    &approvals::Signer::from_seed(Secret(seed.to_vec()), kid.clone()).unwrap(),
                    keys.key().unwrap(),
                    reading,
                    &grant_id,
                )
                .unwrap();
                revoked.set(revoked.get() + 1);
            }
        };
        seam::set(Some(seam::Seam {
            credential: b"fictional-only".to_vec(),
            at: Box::new(hook),
        }));
        let owned = Mutex::new(lifecycle::Control::default());
        let (stopped, pool) = (AtomicBool::new(false), std::sync::Weak::new());
        let delivery = execute(
            &site.paths,
            "fixture",
            Invocation {
                request: issuance::Request {
                    connection: &site.connection,
                    operation: "write",
                    schema: &site.schema,
                    revision: "fixture-private-descriptor",
                    input: r#"{"value":true}"#,
                },
                key: Some("key-1"),
                proof: None,
                until: Instant::now() + wait,
            },
            &mut site.child,
            Control {
                lifecycle: &owned,
                epoch: 0,
                stopped: &stopped,
                pool: &pool,
            },
        );
        seam::set(None);
        let delivery = delivery.unwrap();
        delivery.validate().unwrap();
        (delivery, revoked.get())
    }

    /// How often the child recorded `event`.
    fn count(site: &Site, event: &str) -> usize {
        std::fs::read_to_string(site.events.join(event))
            .unwrap_or_default()
            .lines()
            .count()
    }

    /// The rows of the use store: grant, state and attempt of each use.
    fn uses(site: &Site) -> Vec<(String, String, Option<String>)> {
        let connection = rusqlite::Connection::open_with_flags(
            site.paths.state.join("standing-grants.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = connection
            .prepare("SELECT grant_id, state, attempt_id FROM grant_uses")
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn refused_as_approval_required(delivery: &Delivery) {
        let error = delivery.error.as_ref().expect("the write was not refused");
        assert_eq!(
            error.code,
            FailureCode::Approval(ApprovalCode::ApprovalRequired)
        );
        assert_eq!(error.retry_after_seconds, None);
        assert!(delivery.result.is_none());
        assert_eq!(
            delivery.mutation.classification,
            Classification::NotAttempted
        );
    }

    /// The first recheck: the grant is revoked after the write was admitted
    /// and before its attempt is recorded. The write is refused as a write no
    /// grant covers, nothing is sent, no attempt exists and the use is given
    /// back. The hook runs only once the grant admitted the write, and the
    /// use it took is the row of the last assertion.
    #[test]
    fn a_grant_revoked_after_admission_refuses_the_write_and_gives_the_use_back() {
        let mut site = site();
        let (delivery, revoked) = write(&mut site, seam::Step::Admitted);
        assert_eq!(revoked, 1);
        refused_as_approval_required(&delivery);
        assert!(delivery.mutation.attempt.is_none());
        assert!(delivery.mutation.standing.is_none());
        // The preflight ran; the provider request did not.
        assert_eq!((count(&site, "prepare"), count(&site, "send")), (1, 0));
        assert_eq!(
            uses(&site),
            [(site.grant_id.clone(), "released".to_owned(), None)]
        );
    }

    /// The second recheck: the grant is revoked after its use was bound to
    /// the attempt and before the gate. The write is refused the same way and
    /// nothing is sent; the attempt stays, names the grant, and its use stays
    /// counted.
    #[test]
    fn a_grant_revoked_after_the_bind_refuses_the_write_and_keeps_the_use_counted() {
        let mut site = site();
        let (delivery, revoked) = write(&mut site, seam::Step::Bound);
        assert_eq!(revoked, 1);
        refused_as_approval_required(&delivery);
        let attempt = delivery.mutation.attempt.as_ref().unwrap().id;
        assert_eq!(
            delivery
                .mutation
                .standing
                .as_ref()
                .map(|s| s.grant_id.as_str()),
            Some(site.grant_id.as_str())
        );
        assert_eq!((count(&site, "prepare"), count(&site, "send")), (1, 0));
        assert_eq!(
            uses(&site),
            [(
                site.grant_id.clone(),
                "bound".to_owned(),
                Some(attempt.to_string())
            )]
        );
    }

    // story:an-approved-write-survives-one-refused-clock-acquisition: the
    // second attempt of the clock in the write path and in recovery
    // (contracts/service/clock.md, "Second attempt"). Each test sets the
    // query deadline below after `site()`, which acquires under `WAIT`.

    /// The query deadline of the tests below, on the test's thread.
    const QUERY: Duration = Duration::from_millis(500);

    /// How often each source was asked so far.
    fn asked(site: &Site) -> [usize; 2] {
        [0, 1].map(|i| site.clocks[i].requests.load(Ordering::SeqCst))
    }
    /// How often each source was asked since `before`.
    fn asked_since(site: &Site, before: [usize; 2]) -> [usize; 2] {
        let now = asked(site);
        [0, 1].map(|i| now[i] - before[i])
    }
    /// Each source drops its next request.
    fn drop_next(site: &Site) {
        for source in &site.clocks {
            source.drops.store(1, Ordering::SeqCst);
        }
    }
    /// The query deadline of the cases whose first source is closed: long
    /// enough that a loaded runner's work fits inside the deadlines it sets,
    /// and never waited for.
    const LONG_QUERY: Duration = Duration::from_secs(10);
    /// Takes the first source out of the site; the configuration still
    /// names its port. Once the source is dropped that port is closed and
    /// each request to it is refused at once (port unreachable on loopback),
    /// so an attempt ends below quorum, with no reply unverified, as soon as
    /// the second source answers. No attempt waits for the query deadline,
    /// and the time left after the first attempt is the deadline less the
    /// work before it: a slow runner leaves less time, never more.
    fn take_first_source(site: &mut Site) -> clock_fixture::Server {
        std::mem::replace(&mut site.clocks[0], clock_fixture::Server::silent())
    }
    /// [`take_first_source`], closed now.
    fn close_first_source(site: &mut Site) {
        drop(take_first_source(site));
    }
    fn refused_as_unavailable_before_any_attempt(delivery: &Delivery) {
        let error = delivery.error.as_ref().expect("the write was not refused");
        assert_eq!(error.code, FailureCode::Owner(Code::Unavailable));
        assert!(delivery.result.is_none());
        assert_eq!(
            delivery.mutation.classification,
            Classification::NotAttempted
        );
    }

    /// Every source drops the first request of the write. The first reading
    /// is taken from its second attempt, and the write is admitted under the
    /// grant and sent once.
    #[test]
    fn a_granted_write_is_dispatched_once_when_the_sources_drop_the_first_request_of_the_write() {
        let mut site = site();
        clock::set_test_query_timeout(QUERY);
        let before = asked(&site);
        drop_next(&site);
        let (delivery, revoked) = write_within(&mut site, WAIT, None);
        assert_eq!(revoked, 0);
        assert!(delivery.error.is_none(), "{:?}", delivery.error);
        assert_eq!(delivery.mutation.classification, Classification::Applied);
        assert!(delivery.mutation.attempt.is_some());
        assert_eq!(
            delivery
                .mutation
                .standing
                .as_ref()
                .map(|s| s.grant_id.as_str()),
            Some(site.grant_id.as_str())
        );
        assert_eq!((count(&site, "prepare"), count(&site, "send")), (1, 1));
        // Two attempts of the first reading, one of the second.
        assert_eq!(asked_since(&site, before), [3, 3]);
    }

    /// The reserve of the write's first reading: the first source is closed,
    /// and with 2 x `LONG_QUERY` to the deadline less than that is left after
    /// the first attempt, where a second needs 2 x `LONG_QUERY` + 1 s (one
    /// deadline, and the reserve before the second reading). No second
    /// attempt starts, and the refusal is today's, not a timeout: the work of
    /// the write has the whole 20 s. With a 500 ms query deadline that was
    /// waited for and 2 s to the deadline, the work had 1.5 s, and a loaded
    /// macOS runner answered `Timeout` (pull request 313).
    #[test]
    fn a_write_refused_inside_the_reserve_answers_as_today() {
        let mut site = site();
        clock::set_test_query_timeout(LONG_QUERY);
        close_first_source(&mut site);
        let before = asked(&site);
        let (delivery, _) = write_within(&mut site, 2 * LONG_QUERY, None);
        refused_as_unavailable_before_any_attempt(&delivery);
        assert!(delivery.mutation.attempt.is_none());
        assert_eq!(asked_since(&site, before), [0, 1]);
        assert_eq!((count(&site, "prepare"), count(&site, "send")), (0, 0));
    }

    /// The keyed write of [`write`], as recovery is asked for it.
    fn keyed_write(site: &Site) -> issuance::Request<'_> {
        issuance::Request {
            connection: &site.connection,
            operation: "write",
            schema: &site.schema,
            revision: "fixture-private-descriptor",
            input: r#"{"value":true}"#,
        }
    }
    /// One `Prepared` attempt of that write under `key-1`, made with the two
    /// functions recovery itself uses (`candidate`, `store`) and the ledger's
    /// `prepare`. Nothing was sent for it.
    fn prepared(site: &Site) -> ledger::Candidate {
        let first = issuance::resolve(
            &site.paths,
            "fixture",
            &keyed_write(site),
            Instant::now() + WAIT,
        )
        .unwrap();
        let candidate = candidate(&first, Some("key-1"), &Uuid::new_v4().to_string()).unwrap();
        let preparation = store(&site.paths, NoClock)
            .unwrap()
            .prepare(&candidate)
            .unwrap();
        assert!(matches!(preparation, ledger::Preparation::Prepared(_)));
        assert!(still_prepared(site, &candidate));
        candidate
    }
    /// Whether the ledger still holds the attempt of `candidate` as `Prepared`.
    fn still_prepared(site: &Site, candidate: &ledger::Candidate) -> bool {
        let original = store(&site.paths, NoClock)
            .unwrap()
            .lookup(candidate)
            .unwrap()
            .expect("the attempt exists");
        original.state == ledger::State::Prepared
    }
    /// `recover_observation` on this thread, its deadline `wait` after the
    /// call, with no worker and no child for the instance.
    fn recover(site: &Site, wait: Duration) -> Delivery {
        recover_observation(
            &site.paths,
            "fixture",
            &keyed_write(site),
            "key-1",
            Instant::now() + wait,
            crate::local::owner::supervisor::Quiescent::for_test("fixture", &mut None),
        )
        .unwrap()
    }
    /// The cause recovery names when it could not settle the attempt.
    fn unavailable_at_observation(delivery: &Delivery) -> bool {
        delivery.mutation.cause.as_ref().is_some_and(|cause| {
            cause.code == connectors_core::ErrorCode::Unavailable
                && cause.stage == Stage::Observation
        })
    }

    /// Recovery of a `Prepared` write needs one clock for the settlement
    /// time. Every source drops its first request; the clock is taken from
    /// the second attempt and the attempt is settled.
    #[test]
    fn recovery_settles_a_prepared_write_when_the_sources_drop_the_first_request() {
        let site = site();
        clock::set_test_query_timeout(QUERY);
        let candidate = prepared(&site);
        let before = asked(&site);
        drop_next(&site);
        let delivery = recover(&site, WAIT);
        assert!(
            !unavailable_at_observation(&delivery),
            "recovery did not settle the attempt"
        );
        assert!(!still_prepared(&site, &candidate));
        assert_eq!(asked_since(&site, before), [2, 2]);
        assert_eq!(count(&site, "send"), 0);
    }

    /// The reserve of recovery: the first source is closed, and with
    /// `LONG_QUERY` + 0.9 s to the deadline less than that is left after the
    /// first attempt, where a second needs `LONG_QUERY` + 1 s. No second
    /// attempt starts, and the answer is today's. A reserve of zero would
    /// admit it while the work before took under 0.9 s. Recovery's other work
    /// has the whole 10.9 s; with query deadlines that were waited for it had
    /// 1.2 s, then 2.4 s, and answered `Timeout` in one run.
    #[test]
    fn recovery_refused_inside_the_reserve_answers_as_today() {
        let mut site = site();
        clock::set_test_query_timeout(LONG_QUERY);
        let candidate = prepared(&site);
        close_first_source(&mut site);
        let before = asked(&site);
        let delivery = recover(&site, LONG_QUERY + Duration::from_millis(900));
        assert!(unavailable_at_observation(&delivery));
        assert!(still_prepared(&site, &candidate));
        assert_eq!(asked_since(&site, before), [0, 1]);
    }

    // The adversary's cases for the reserve of each caller, in a file of
    // their own beside this one.
    mod second_attempt_adversary {
        include!("second_attempt_adversary_tests.rs");
    }
}
