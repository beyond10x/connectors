//! story:connect-refused-while-instance-holds-connection: an instance holds at
//! most one published non-revoked connection (contracts/auth/connection/
//! v1alpha1/semantics.md §4.2). A connect is refused while the instance holds a
//! published Live or Stale connection and admitted again once it is revoked; a
//! connection allocated and never published does not count once its connect
//! ended. An instance that held more than one before the rule keeps them all
//! and is refused a new one until the extra ones are revoked. Every
//! observation reopens the metadata store, so each answer here is the one the
//! metadata authority recorded.
use super::tests::{NOW, binding, fixture, prepared, publish_fixture};
use super::*;

const LATER: u64 = NOW + 1;

fn revision(registry: &Registry, reference: &str, now: u64) -> String {
    registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            reference,
            now,
            true,
        )
        .unwrap()
        .revision
}

fn connected(registry: &Registry) -> (String, String) {
    let (_, candidate) = prepared(registry, "one", NOW);
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = revision(registry, &reference, NOW);
    (reference, revision)
}

fn revoke(registry: &Registry, reference: &str, now: u64) {
    let revision = revision(registry, reference, now);
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            reference,
            &revision,
            now,
        )
        .unwrap();
}

fn elsewhere() -> Binding {
    Binding {
        instance_id: "other-instance".into(),
        ..binding()
    }
}

#[test]
fn a_connect_is_refused_while_the_instance_holds_a_published_live_connection() {
    let (_root, registry) = fixture();
    let (reference, _) = connected(&registry);
    assert_eq!(
        registry.begin(&binding(), LATER).map(|_| ()),
        Err(Failure::InstanceConnected)
    );
    // Control: another instance still connects, so the refusal is the rule.
    registry.begin(&elsewhere(), LATER).unwrap();
    // Revoked connections are history and do not count.
    revoke(&registry, &reference, LATER);
    registry.begin(&binding(), LATER).unwrap();
}

#[test]
fn a_stale_connection_is_held_as_a_live_one_is() {
    let (root, registry) = fixture();
    let (reference, _) = connected(&registry);
    let captured = registry
        .capture_read(&binding(), &reference, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, NOW).unwrap();
    registry.refuse_read(dispatched, NOW).unwrap();
    let observed = Registry::new(root.path())
        .describe(
            "fixture-instance",
            "fixture-adapter",
            "config-1",
            &reference,
            LATER,
            true,
        )
        .unwrap();
    assert_eq!(observed.state, State::Stale);
    assert_eq!(
        registry.begin(&binding(), LATER).map(|_| ()),
        Err(Failure::InstanceConnected)
    );
    revoke(&registry, &reference, LATER);
    registry.begin(&binding(), LATER).unwrap();
}

/// A connect still in flight holds the instance until it ends, so two
/// concurrent connects cannot both publish; one that failed or expired before
/// publication holds nothing and never blocks the next connect.
#[test]
fn only_a_connect_in_flight_blocks_beside_a_published_one() {
    let (_root, registry) = fixture();
    let acquisition = registry.begin(&binding(), NOW).unwrap();
    assert_eq!(
        registry.begin(&binding(), NOW).map(|_| ()),
        Err(Failure::Conflict)
    );
    let claim = registry.consume(acquisition, NOW).unwrap();
    registry.fail(&claim, NOW).unwrap();
    registry.begin(&binding(), NOW).unwrap();

    // Abandoned without a recorded failure: it ends at its deadline, before
    // any expiry sweep recorded it.
    let (_root, registry) = fixture();
    registry.begin(&binding(), NOW).unwrap();
    assert_eq!(
        registry.begin(&binding(), NOW + ENTRY_MS - 1).map(|_| ()),
        Err(Failure::Conflict)
    );
    registry.begin(&binding(), NOW + ENTRY_MS).unwrap();

    // The in-flight connect that publishes holds the instance from then on.
    let (_root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    publish_fixture(&registry, candidate, NOW);
    assert_eq!(
        registry.begin(&binding(), NOW + ENTRY_MS).map(|_| ()),
        Err(Failure::InstanceConnected)
    );
}

/// Two connections published before the rule, as `begin` admitted them then:
/// both stay usable with their references, and a new connect is refused until
/// both are revoked.
#[test]
fn an_instance_holding_two_connections_keeps_them_and_connects_after_both_are_revoked() {
    let (_root, registry) = fixture();
    let (_, first) = prepared(&registry, "one", NOW);
    let acquisition = registry.begin_unchecked(&binding(), NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let second = registry
        .prepare(
            &claim,
            ValidatedBaseline {
                identity: ExternalIdentity {
                    kind: "fixture-account".into(),
                    subject: "two".into(),
                },
                granted_scopes: Some(BTreeSet::from(["read".into()])),
                credential_expires_at_ms: Some(NOW + 3_600_000),
                collected_at_ms: NOW,
                valid_until_ms: NOW + 60_000,
            },
            12,
            NOW,
        )
        .unwrap();
    let first = publish_fixture(&registry, first, NOW);
    let second = publish_fixture(&registry, second, NOW);
    for reference in [&first, &second] {
        let observed = registry
            .describe(
                "fixture-instance",
                "fixture-adapter",
                "config-1",
                reference,
                LATER,
                true,
            )
            .unwrap();
        assert_eq!(observed.state, State::Ready, "{reference}");
    }
    // No reconnect replaces two: only revoking the extra one clears it.
    assert_eq!(
        registry.begin(&binding(), LATER).map(|_| ()),
        Err(Failure::InstanceConnectionChanged)
    );
    revoke(&registry, &first, LATER);
    assert_eq!(
        registry.begin(&binding(), LATER).map(|_| ()),
        Err(Failure::InstanceConnected)
    );
    revoke(&registry, &second, LATER);
    registry.begin(&binding(), LATER).unwrap();
}

/// A held connection saved under an older configuration revision is refused
/// as a repair of it is: it names the revalidation that makes it the
/// configured one, never a revocation of a connection that can follow the
/// upgrade. Once it followed, the connect names a reconnect. (A held
/// connection that repair refuses as changed names a revocation:
/// `upgrade_tests`.)
#[test]
fn a_held_connection_under_an_older_revision_names_its_revalidation() {
    let (_root, registry) = fixture();
    let (reference, _) = connected(&registry);
    let mut configured = binding();
    configured.configuration_revision = "config-2".into();
    let revision = revision(&registry, &reference, LATER);
    assert_eq!(
        registry
            .begin_repair(&configured, &reference, &revision, LATER)
            .map(|_| ()),
        Err(Failure::UpgradeRequired)
    );
    assert_eq!(
        registry.begin(&configured, LATER).map(|_| ()),
        Err(Failure::UpgradeRequired)
    );
    let captured = registry
        .capture_revalidation(&configured, &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    let baseline = ValidatedBaseline {
        identity: ExternalIdentity {
            kind: "fixture-account".into(),
            subject: "one".into(),
        },
        granted_scopes: Some(BTreeSet::from(["read".into()])),
        credential_expires_at_ms: Some(LATER + 3_600_000),
        collected_at_ms: LATER,
        valid_until_ms: LATER + 60_000,
    };
    registry
        .finish_revalidation(dispatched, Ok(baseline), LATER)
        .unwrap();
    assert_eq!(
        registry.begin(&configured, LATER).map(|_| ()),
        Err(Failure::InstanceConnected)
    );
    revoke(&registry, &reference, LATER);
    registry.begin(&configured, LATER).unwrap();
}

/// The model refuses the allocation itself (`AllocateConnection`'s
/// `instance-connected` outcome) once the host supplies the held connection:
/// an allocation that passed no host check beside a published connection is
/// refused by the metadata authority and commits nothing.
#[test]
fn the_model_refuses_an_allocation_beside_a_held_connection() {
    let (root, registry) = fixture();
    let (reference, _) = connected(&registry);
    assert_eq!(
        registry.begin_unchecked(&binding(), LATER).map(|_| ()),
        Err(Failure::MetadataUnavailable)
    );
    let metadata = Metadata::inspect(root.path()).unwrap();
    let rows: i64 = metadata
        .connection
        .query_row(
            "SELECT count(*) FROM registry_connections WHERE instance_id='fixture-instance'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(metadata);
    assert_eq!(rows, 1);
    revoke(&registry, &reference, LATER);
    registry.begin_unchecked(&binding(), LATER).unwrap();
}

/// What a command that names no connection selects: the instance's one
/// published non-revoked connection; none while a connect is only in flight
/// or after revocation; two say only that one must be named.
#[test]
fn held_names_the_published_non_revoked_connections_of_the_instance() {
    let (_root, registry) = fixture();
    let (_, first) = prepared(&registry, "one", NOW);
    assert!(registry.held("fixture-instance", NOW).unwrap().is_empty());
    let acquisition = registry.begin_unchecked(&binding(), NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let second = registry
        .prepare(
            &claim,
            ValidatedBaseline {
                identity: ExternalIdentity {
                    kind: "fixture-account".into(),
                    subject: "two".into(),
                },
                granted_scopes: Some(BTreeSet::from(["read".into()])),
                credential_expires_at_ms: Some(NOW + 3_600_000),
                collected_at_ms: NOW,
                valid_until_ms: NOW + 60_000,
            },
            12,
            NOW,
        )
        .unwrap();
    let first = publish_fixture(&registry, first, NOW);
    assert_eq!(
        registry.held("fixture-instance", NOW).unwrap(),
        std::slice::from_ref(&first)
    );
    assert!(registry.held("other-instance", NOW).unwrap().is_empty());
    let second = publish_fixture(&registry, second, NOW);
    let mut both = vec![first.clone(), second.clone()];
    both.sort();
    assert_eq!(registry.held("fixture-instance", NOW).unwrap(), both);
    revoke(&registry, &first, LATER);
    assert_eq!(
        registry.held("fixture-instance", LATER).unwrap(),
        std::slice::from_ref(&second)
    );
    revoke(&registry, &second, LATER);
    assert!(registry.held("fixture-instance", LATER).unwrap().is_empty());
}
