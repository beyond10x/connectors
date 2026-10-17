//! Adversary pass on story:connection-stale-state. Each case drives the
//! registry the way the owner does (`owner/supervisor.rs` worker,
//! `owner/transport.rs` launch) and asserts what the Connection lifecycle
//! (`contracts/auth/connection/v1alpha1/semantics.md` sections 4.1 and 4.2) and
//! the unit's own stale_tests.rs say.
use super::*;

/// Every reader of a Stale connection agrees on what Stale means. A
/// connection made Stale by a revalidation's `invalid` admits no read; one
/// made Stale by a refused read reports the same `stale` state, so it must
/// admit no read either (section 4.1 row 3: Stale is not ready, and business
/// use requires global ready). The consumer launch captures through the same
/// `capture_read` (`owner/transport.rs` launch).
#[test]
fn adversary_a_stale_connection_admits_no_read_whichever_refusal_made_it_stale() {
    // Stale through revalidation `invalid`.
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, NOW).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), NOW)
        .unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
    let by_revalidation = registry.admit_read(&binding(), &reference, &BTreeSet::new(), LATER);
    assert_eq!(by_revalidation, Err(Failure::NotReady));

    // Stale through a refused read: the same state, so the same admission.
    let (root, registry) = fixture();
    let (reference, _) = connected(&registry);
    refused_read(&registry, &reference, NOW).unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
    assert_eq!(
        registry.admit_read(&binding(), &reference, &BTreeSet::new(), LATER),
        by_revalidation,
        "a Stale connection admitted a read"
    );
    assert!(
        registry
            .capture_read(
                &binding(),
                &reference,
                &BTreeSet::new(),
                LATER,
                LATER + 1000
            )
            .is_err(),
        "a Stale connection's refused credential was captured for dispatch"
    );
}

/// A refusal whose use another capture already swept as expired is still a
/// provider refusal. `release_read` answers `Ok` for the same swept use; the
/// owner propagates `refuse_read`'s error with `?`, so a `Conflict` here turns
/// the provider's `invalid_credential` into the host's `lifecycle_conflict`.
#[test]
fn adversary_a_refusal_of_a_swept_use_is_not_a_lifecycle_conflict() {
    let (_root, registry) = fixture();
    let (reference, _) = connected(&registry);
    let expires = NOW + 1000;
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, expires)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, NOW).unwrap();
    // Another invoke captures at the first one's expiry and sweeps its use.
    let other = registry
        .capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            expires,
            expires + 1000,
        )
        .unwrap();
    registry.cancel_read(other, expires).unwrap();
    assert_eq!(registry.refuse_read(dispatched, expires), Ok(()));
}
