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
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
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
