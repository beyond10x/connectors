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
