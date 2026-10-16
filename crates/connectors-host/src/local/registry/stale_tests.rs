//! story:connection-stale-state: a connection whose stored credential the
//! provider refused is `Stale` (`connectors.auth_bindings.MarkConnectionStale`)
//! until a publication returns it to `Live`; revocation ends it from either.
//! Every observation reopens the metadata store, so each state read here is
//! the one the metadata authority recorded.
use super::tests::{NOW, binding, fixture, prepared, publish_fixture};
use super::*;

const LATER: u64 = NOW + 1;

fn validated(subject: &str, now: u64) -> ValidatedBaseline {
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

fn observe(root: &Path, configured: &str, reference: &str, now: u64) -> ObservedConnection {
    Registry::new(root)
        .describe(
            "fixture-instance",
            "fixture-adapter",
            configured,
            reference,
            now,
            true,
        )
        .unwrap()
}

fn state(root: &Path, reference: &str, now: u64) -> State {
    observe(root, "config-1", reference, now).state
}

fn connected(registry: &Registry) -> (String, String) {
    let (_, candidate) = prepared(registry, "one", NOW);
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &reference,
            NOW,
            true,
        )
        .unwrap()
        .revision;
    (reference, revision)
}

/// A read the provider refused as unauthorized, as the owner reports it.
fn refused_read(registry: &Registry, reference: &str, now: u64) -> Result<()> {
    let captured =
        registry.capture_read(&binding(), reference, &BTreeSet::new(), now, now + 1000)?;
    let dispatched = registry.dispatch_read(captured, now)?;
    registry.refuse_read(dispatched, now)
}

fn reconnect(
    registry: &Registry,
    reference: &str,
    revision: &str,
    subject: &str,
) -> Result<String> {
    let acquisition = registry.begin_repair(&binding(), reference, revision, LATER)?;
    let claim = registry.consume(acquisition, LATER)?;
    let candidate = registry.prepare(&claim, validated(subject, LATER), 12, LATER)?;
    let receipt = custody::WrittenVersion::fixture(candidate.version());
    let stored = registry.acknowledge(candidate, receipt, LATER)?;
    registry.publish(stored, LATER)
}

#[test]
fn a_refused_read_makes_the_connection_stale_until_a_reconnect_publishes() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    assert_eq!(state(root.path(), &reference, NOW), State::Ready);
    // A second read already in flight when the first refusal lands.
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let in_flight = registry.dispatch_read(captured, NOW).unwrap();

    refused_read(&registry, &reference, NOW).unwrap();
    let stale = observe(root.path(), "config-1", &reference, NOW);
    assert_eq!(stale.state, State::Stale);
    // Stale is the lifecycle, not a new public revision, and not the cached
    // metadata flag a description carries beside it.
    assert_eq!(stale.revision, revision);
    assert!(!stale.stale);
    assert_eq!(
        serde_json::to_value(stale.state).unwrap(),
        serde_json::json!("stale")
    );
    // Stale is not ready: no further read is admitted, and the refusal of the
    // read already in flight leaves it Stale.
    assert_eq!(
        refused_read(&registry, &reference, NOW),
        Err(Failure::NotReady)
    );
    registry.refuse_read(in_flight, NOW).unwrap();
    assert_eq!(state(root.path(), &reference, NOW), State::Stale);

    // Another identity on the reconnect is refused and changes nothing.
    assert_eq!(
        reconnect(&registry, &reference, &revision, "other"),
        Err(Failure::IdentityMismatch)
    );
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);

    assert_eq!(
        reconnect(&registry, &reference, &revision, "one").unwrap(),
        reference
    );
    let live = observe(root.path(), "config-1", &reference, LATER);
    assert_eq!(live.state, State::Ready);
    assert_eq!(live.revision, revision);

    // Revocation ends a Stale connection.
    refused_read(&registry, &reference, LATER).unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Revoked);
}

/// A reconnect that prepared a candidate and then failed publishes nothing:
/// the connection stays Stale, and the next reconnect still returns it to Live.
#[test]
fn a_failed_reconnect_leaves_the_connection_stale() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    refused_read(&registry, &reference, NOW).unwrap();
    let acquisition = registry
        .begin_repair(&binding(), &reference, &revision, LATER)
        .unwrap();
    let claim = registry.consume(acquisition, LATER).unwrap();
    registry
        .prepare(&claim, validated("one", LATER), 12, LATER)
        .unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
    registry.fail(&claim, LATER).unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
    reconnect(&registry, &reference, &revision, "one").unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Ready);
}

/// A refusal of a credential a publication has since replaced is not a
/// refusal of the connection's credential.
#[test]
fn a_refusal_of_a_replaced_credential_leaves_the_connection_live() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, NOW).unwrap();
    reconnect(&registry, &reference, &revision, "one").unwrap();
    registry.refuse_read(dispatched, LATER).unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Ready);
}

/// Decision 17: each revalidation outcome a reconnect fixes makes the
/// connection Stale; an unavailable provider does not.
#[test]
fn revalidation_invalid_insufficient_and_missing_make_the_connection_stale() {
    type Outcome = fn(&Registry, Revalidation) -> Result<()>;
    let outcomes: [(&str, Outcome); 5] = [
        ("invalid", |registry, captured| {
            let dispatched = registry.dispatch_revalidation(captured, NOW)?;
            registry.finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), NOW)
        }),
        ("insufficient", |registry, captured| {
            let dispatched = registry.dispatch_revalidation(captured, NOW)?;
            registry.finish_revalidation(
                dispatched,
                Err(Some(InvalidCredential::Insufficient)),
                NOW,
            )
        }),
        ("missing", |registry, captured| {
            registry.missing_revalidation(captured, NOW)
        }),
        ("another identity", |registry, captured| {
            let dispatched = registry.dispatch_revalidation(captured, NOW)?;
            assert_eq!(
                registry.finish_revalidation(dispatched, Ok(validated("other", NOW)), NOW),
                Err(Failure::IdentityMismatch)
            );
            Ok(())
        }),
        ("unavailable", |registry, captured| {
            let dispatched = registry.dispatch_revalidation(captured, NOW)?;
            registry.finish_revalidation(dispatched, Err(None), NOW)
        }),
    ];
    for (outcome, finish) in outcomes {
        let (root, registry) = fixture();
        let (reference, revision) = connected(&registry);
        let captured = registry
            .capture_revalidation(&binding(), &reference, &revision, NOW, NOW + 1000)
            .unwrap();
        finish(&registry, captured).unwrap();
        let expected = if outcome == "unavailable" {
            State::Ready
        } else {
            State::Stale
        };
        assert_eq!(state(root.path(), &reference, NOW), expected, "{outcome}");
    }
}

/// A material the owner found missing while it read the credential is
/// Stale too: a reconnect is the fix.
#[test]
fn a_missing_credential_on_a_read_makes_the_connection_stale() {
    let (root, registry) = fixture();
    let (reference, _) = connected(&registry);
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    registry
        .invalidate_read(&captured, InvalidCredential::Missing, NOW)
        .unwrap();
    assert_eq!(state(root.path(), &reference, NOW), State::Stale);
}

/// A Stale connection is not ready, so it admits no revalidation: only a
/// reconnect's publication returns it to Live (coordinator decision F1 on
/// this story, contract connection section 4.1 row 3).
#[test]
fn a_stale_connection_admits_no_revalidation() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    refused_read(&registry, &reference, NOW).unwrap();
    assert_eq!(state(root.path(), &reference, NOW), State::Stale);
    assert_eq!(
        registry.admit_revalidation(&binding(), &reference, &revision, LATER),
        Err(Failure::NotReady)
    );
    assert!(matches!(
        registry.capture_revalidation(&binding(), &reference, &revision, LATER, LATER + 1000),
        Err(Failure::NotReady)
    ));
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
}

/// The provider's refusal is about the credential, not about the use: a use
/// another capture already swept as expired still makes the connection Stale.
#[test]
fn a_refusal_of_a_swept_use_still_makes_the_connection_stale() {
    let (root, registry) = fixture();
    let (reference, _) = connected(&registry);
    let expires = NOW + 1000;
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, expires)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, NOW).unwrap();
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
    registry.refuse_read(dispatched, expires).unwrap();
    assert_eq!(state(root.path(), &reference, expires), State::Stale);
}

/// A revalidation refused under a newer configuration revision records only
/// the refused revision: the credential still works under its own.
#[test]
fn a_refused_upgrade_does_not_make_the_connection_stale() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    let mut upgraded = binding();
    upgraded.configuration_revision = "config-2".into();
    let captured = registry
        .capture_revalidation(&upgraded, &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), LATER)
        .unwrap();
    assert_eq!(state(root.path(), &reference, LATER), State::Ready);
    // A Stale connection admits no upgrade either: it stays Stale.
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry);
    refused_read(&registry, &reference, LATER).unwrap();
    assert!(matches!(
        registry.capture_revalidation(&upgraded, &reference, &revision, LATER, LATER + 1000),
        Err(Failure::NotReady)
    ));
    assert_eq!(state(root.path(), &reference, LATER), State::Stale);
}

#[path = "stale_adversary_tests.rs"]
mod adversary;
