use connectors_mcp::lease::{DataLease, GateDecision as D, LeaseGate, Timestamp};
use std::{cell::Cell, rc::Rc};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn stamp(ms: i64) -> Timestamp {
    Timestamp(
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .unwrap()
            .format(&Rfc3339)
            .unwrap(),
    )
}
fn lease(issued: i64, expiry: i64, sequence: i64) -> DataLease {
    DataLease {
        issued_at: stamp(issued),
        effective_expiry: stamp(expiry),
        sequence,
    }
}
type ClockSample = Rc<Cell<Option<(i64, i64)>>>;

fn clock(lower: i64, upper: i64) -> (ClockSample, impl FnMut() -> Option<(i64, i64)>) {
    let sample = Rc::new(Cell::new(Some((lower, upper))));
    let source = sample.clone();
    (sample, move || source.get())
}

#[test]
fn delay_and_uncertainty_consume_the_original_lease_without_receipt_grace() {
    let (sample, source) = clock(500, 700);
    let mut gate = LeaseGate::admit(lease(0, 2000, 1), source).ok().unwrap();
    sample.set(Some((1998, 1999)));
    assert_eq!(gate.decision(), D::Allow);
    sample.set(Some((1999, 2000)));
    assert_eq!(gate.decision(), D::Expired);
    assert!(gate.lease().is_none());
    sample.set(Some((2000, 2000)));
    assert_eq!(gate.renew(lease(2000, 4000, 2)), D::Expired);
    sample.set(Some((2100, 2100)));
    assert_eq!(gate.decision(), D::Expired);
}

#[test]
fn malformed_future_zero_and_overlong_leases_never_admit() {
    for candidate in [
        lease(0, 2001, 1),
        lease(0, 0, 1),
        lease(1, 2000, 1),
        lease(0, 2000, 0),
        lease(0, 2000, -1),
        DataLease {
            issued_at: Timestamp("invalid".into()),
            ..lease(0, 2000, 1)
        },
        DataLease {
            effective_expiry: Timestamp("1970-01-01T00:00:02.000000001Z".into()),
            ..lease(0, 2000, 1)
        },
    ] {
        let (_, source) = clock(0, 0);
        assert!(matches!(
            LeaseGate::admit(candidate, source),
            Err(D::Unverified)
        ));
    }
}

#[test]
fn renewal_requires_live_prior_authority_and_data_activity_does_not_renew() {
    let (sample, source) = clock(0, 0);
    let mut gate = LeaseGate::admit(lease(0, 2000, 1), source).ok().unwrap();
    sample.set(Some((1000, 1050)));
    assert_eq!(gate.renew(lease(1000, 3000, 2)), D::Allow);
    assert_eq!(gate.lease(), Some(&lease(1000, 3000, 2)));
    for moment in [1500, 1900, 2100, 2500, 2999] {
        sample.set(Some((moment, moment)));
        assert_eq!(gate.decision(), D::Allow);
    }
    sample.set(Some((3000, 3000)));
    assert_eq!(gate.decision(), D::Expired);
}

#[test]
fn replay_and_reordered_issuance_close_instead_of_refreshing_authority() {
    for replay in [lease(1500, 3500, 1), lease(500, 2500, 2)] {
        let (sample, source) = clock(1000, 1000);
        let mut gate = LeaseGate::admit(lease(1000, 3000, 1), source).ok().unwrap();
        sample.set(Some((1500, 1500)));
        assert_eq!(gate.renew(replay), D::Unverified);
        assert!(gate.lease().is_none());
        assert_eq!(gate.renew(lease(1500, 3500, 3)), D::Unverified);
    }
}

#[test]
fn unavailable_regressed_reversed_or_too_uncertain_clock_never_recovers_a_gate() {
    for broken in [None, Some((99, 99)), Some((102, 101)), Some((100, 2100))] {
        let (sample, source) = clock(100, 100);
        let mut gate = LeaseGate::admit(lease(100, 2100, 1), source).ok().unwrap();
        sample.set(broken);
        assert_eq!(gate.decision(), D::Unverified);
        assert!(gate.lease().is_none());
        sample.set(Some((200, 200)));
        assert_eq!(gate.renew(lease(200, 2200, 2)), D::Unverified);
    }
}

#[test]
fn drain_caps_future_renewals_and_denies_without_extra_grace() {
    let (sample, source) = clock(0, 0);
    let mut gate = LeaseGate::admit(lease(0, 2000, 1), source).ok().unwrap();
    gate.begin_drain(stamp(4000)).unwrap();
    sample.set(Some((1000, 1000)));
    assert_eq!(gate.renew(lease(1000, 3000, 2)), D::Allow);
    sample.set(Some((2500, 2500)));
    assert_eq!(gate.renew(lease(2500, 4500, 3)), D::Allow);
    assert_eq!(gate.lease(), Some(&lease(2500, 4000, 3)));
    gate.begin_drain(stamp(9000)).unwrap(); // a later request cannot extend the first drain
    sample.set(Some((3999, 3999)));
    assert_eq!(gate.decision(), D::Allow);
    sample.set(Some((4000, 4000)));
    assert_eq!(gate.decision(), D::Revoked);
    assert_eq!(gate.renew(lease(4000, 6000, 4)), D::Revoked);
}

#[test]
fn unsupported_short_drain_leaves_existing_authority_unchanged() {
    let (sample, source) = clock(100, 120);
    let original = lease(100, 2100, 1);
    let mut gate = LeaseGate::admit(original.clone(), source).ok().unwrap();
    assert_eq!(
        gate.begin_drain(stamp(2119)).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(gate.lease(), Some(&original));
    sample.set(Some((200, 200)));
    assert_eq!(gate.decision(), D::Allow);
    sample.set(Some((2100, 2100)));
    assert_eq!(gate.decision(), D::Expired);
}

#[test]
fn first_revocation_wins_over_later_expiry_and_valid_renewal() {
    let (sample, source) = clock(0, 0);
    let mut gate = LeaseGate::admit(lease(0, 2000, 1), source).ok().unwrap();
    assert_eq!(gate.revoke(), D::Revoked);
    sample.set(Some((3000, 3000)));
    assert_eq!(gate.decision(), D::Revoked);
    assert_eq!(gate.renew(lease(3000, 5000, 2)), D::Revoked);
    assert!(gate.lease().is_none());
}

#[test]
fn timestamp_precision_is_not_rounded_to_the_clock_interface_resolution() {
    let (sample, source) = clock(1, 1);
    let original = DataLease {
        issued_at: Timestamp("1970-01-01T00:00:00.000000001Z".into()),
        effective_expiry: Timestamp("1970-01-01T00:00:02.000000001Z".into()),
        sequence: 1,
    };
    let mut gate = LeaseGate::admit(original.clone(), source).ok().unwrap();
    assert_eq!(gate.lease(), Some(&original));
    sample.set(Some((2000, 2000)));
    assert_eq!(gate.decision(), D::Allow);
    sample.set(Some((2001, 2001)));
    assert_eq!(gate.decision(), D::Expired);
}
