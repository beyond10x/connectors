//! story:connection-follows-configuration-upgrade: revalidating a connection
//! whose binding names an older configuration revision of its instance moves
//! it to the configured revision without a credential entry, when the
//! authentication it was admitted under is unchanged.
use super::tests::{NOW, binding, fixture, prepared, publish_fixture};
use super::*;

const LATER: u64 = NOW + 1;

/// One change to a configured binding beside its configuration revision.
type Change = fn(&mut Binding);

fn upgraded() -> Binding {
    let mut upgraded = binding();
    upgraded.configuration_revision = "config-2".into();
    upgraded
}

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

fn observe(registry: &Registry, configured: &str, reference: &str, now: u64) -> ObservedConnection {
    registry
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

/// Every recorded fact an upgrade may change, read back from the record:
/// the connection's binding, fence, evidence, material and public revision,
/// whether its material is still valid, and its instance's revision.
#[derive(Debug, PartialEq)]
struct Recorded {
    binding: Binding,
    /// The configured revision whose upgrade the new provider refused.
    refused: Option<String>,
    fence: String,
    baseline: Option<String>,
    material: Option<String>,
    revision: String,
    invalid: Option<String>,
    instance_revision: String,
    acquisitions: i64,
}

impl std::fmt::Debug for Binding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&encode(self).map_err(|_| std::fmt::Error)?)
    }
}

fn recorded(root: &Path, reference: &str) -> Recorded {
    let metadata = Metadata::inspect(root).unwrap();
    let (binding, fence, baseline, material, revision): (
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    ) = metadata
        .connection
        .query_row(
            "SELECT binding,publication_fence,baseline,active_material,semantic_revision FROM registry_connections WHERE connection_ref=?1",
            [reference],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    let invalid: Option<String> = material.as_ref().and_then(|version| {
        metadata
            .connection
            .query_row(
                "SELECT invalid_reason FROM registry_materials WHERE version_id=?1",
                [version],
                |r| r.get(0),
            )
            .unwrap()
    });
    let instance_revision: String = metadata
        .connection
        .query_row(
            "SELECT configuration_revision FROM registry_instances WHERE instance_id='fixture-instance'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let acquisitions: i64 = metadata
        .connection
        .query_row("SELECT count(*) FROM registry_acquisitions", [], |r| {
            r.get(0)
        })
        .unwrap();
    let (binding, refused) = decode_binding(&binding).unwrap();
    Recorded {
        binding,
        refused,
        fence,
        baseline,
        material,
        revision,
        invalid,
        instance_revision,
        acquisitions,
    }
}

fn connected(registry: &Registry, subject: &str) -> (String, String, custody::Version) {
    let (_, candidate) = prepared(registry, subject, NOW);
    let version = candidate.version();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = observe(registry, "config-1", &reference, NOW).revision;
    (reference, revision, version)
}

#[test]
fn an_unchanged_authentication_follows_a_configuration_upgrade_without_reentry() {
    let (root, registry) = fixture();
    let (reference, revision, version) = connected(&registry, "one");
    let stale = observe(&registry, "config-2", &reference, LATER);
    assert!(stale.stale);
    assert_eq!(stale.state, State::Pending);

    registry
        .admit_revalidation(&upgraded(), &reference, &revision, LATER)
        .unwrap();
    // The capture names the material already in custody: no acquisition, no
    // credential source, no new custody version.
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    assert!(captured.version() == version);
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER)
        .unwrap();

    let observed = observe(&registry, "config-2", &reference, LATER);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    // The binding changed, so the public revision advances
    // (contracts/cli/v1alpha1/semantics.md, effect-relevant configuration).
    assert_ne!(observed.revision, revision);
    assert_eq!(
        registry.admit_revalidation(&upgraded(), &reference, &revision, LATER),
        Err(Failure::Conflict)
    );
    registry
        .admit_revalidation(&upgraded(), &reference, &observed.revision, LATER)
        .unwrap();
    let after = recorded(root.path(), &reference);
    assert_eq!(after.revision, observed.revision);
    assert_eq!(after.binding, upgraded());
    assert_eq!(after.instance_revision, "config-2");
    assert_eq!(after.acquisitions, 1);
    assert_eq!(after.invalid, None);

    // The record replays to the same projection in a fresh handle.
    drop(registry);
    let reopened = Registry::new(root.path());
    assert_eq!(
        observe(&reopened, "config-2", &reference, LATER).state,
        State::Ready
    );
    assert_eq!(recorded(root.path(), &reference), after);
}

#[test]
fn invocation_is_admitted_under_the_upgraded_revision_and_not_the_old_one() {
    let (_root, registry) = fixture();
    let (reference, revision, version) = connected(&registry, "one");
    let none = BTreeSet::new();
    // Before the upgrade only the admitted revision reads.
    assert_eq!(
        registry.admit_read(&upgraded(), &reference, &none, LATER),
        Err(Failure::UpgradeRequired)
    );
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER)
        .unwrap();

    registry
        .admit_read(&upgraded(), &reference, &none, LATER)
        .unwrap();
    let read = registry
        .capture_read(&upgraded(), &reference, &none, LATER, LATER + 1000)
        .unwrap();
    assert!(read.version() == version);
    registry.dispatch_read(read, LATER).unwrap();
    assert_eq!(
        registry.admit_read(&binding(), &reference, &none, LATER),
        Err(Failure::UpgradeRequired)
    );
    assert!(matches!(
        registry.capture_read(&binding(), &reference, &none, LATER, LATER + 1000),
        Err(Failure::UpgradeRequired)
    ));
}

#[test]
fn a_changed_authority_or_profile_refuses_the_upgrade_and_publishes_nothing() {
    let changes: [(&str, Change); 6] = [
        ("provider authority", |b| {
            b.provider_authority.push_str("/other")
        }),
        // A changed scheme, capability or entry field is a new profile
        // declaration and so a new profile revision.
        ("profile revision", |b| {
            b.profile.revision = "profile-2".into()
        }),
        ("profile id", |b| b.profile.id = "fixture-other".into()),
        ("purpose and subject", |b| {
            b.profile.purpose = Purpose::AppLevel;
            b.profile.subject = Subject::App;
        }),
        ("minimum scopes", |b| {
            b.profile.minimum_scopes.insert("write".into());
        }),
        ("evidence lifetime", |b| {
            b.profile.evidence_lifetime_ms = 30_000
        }),
    ];
    for (name, change) in changes {
        let (root, registry) = fixture();
        let (reference, revision, _) = connected(&registry, "one");
        let mut target = upgraded();
        change(&mut target);
        let before = recorded(root.path(), &reference);
        assert_eq!(
            registry.admit_revalidation(&target, &reference, &revision, LATER),
            Err(Failure::BindingChanged),
            "{name}"
        );
        assert!(
            matches!(
                registry.capture_revalidation(&target, &reference, &revision, LATER, LATER + 1000),
                Err(Failure::BindingChanged)
            ),
            "{name}"
        );
        assert_eq!(recorded(root.path(), &reference), before, "{name}");
        assert_eq!(
            observe(&registry, "config-1", &reference, LATER).state,
            State::Ready,
            "{name}"
        );
    }
}

#[test]
fn a_different_identity_refuses_the_upgrade_and_keeps_the_credential() {
    let (root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    let before = recorded(root.path(), &reference);
    // The refusal is recorded against the configured revision and changes
    // nothing else (story:pending-connection-refusal-names-its-remedy).
    let mut refused = recorded(root.path(), &reference);
    refused.refused = Some("config-2".into());
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    assert_eq!(
        registry.finish_revalidation(dispatched, Ok(validated("different", LATER)), LATER),
        Err(Failure::UpgradeIdentityMismatch)
    );
    assert_eq!(recorded(root.path(), &reference), refused);
    assert_eq!(
        observe(&registry, "config-1", &reference, LATER).state,
        State::Ready
    );
    // A credential the new configuration's provider refuses is not proved
    // invalid under the configuration it was admitted under either.
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), LATER)
        .unwrap();
    assert_eq!(recorded(root.path(), &reference), refused);
    assert!(before.refused.is_none());
}

#[test]
fn a_revoked_connection_is_not_upgraded() {
    let (root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    // A revocation that lands while the new provider validates wins.
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    let revoked = registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    let before = recorded(root.path(), &reference);
    assert_eq!(
        registry.finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER),
        Err(Failure::Revoked)
    );
    for expected in [&revision, &revoked] {
        assert!(matches!(
            registry.capture_revalidation(&upgraded(), &reference, expected, LATER, LATER + 1000),
            Err(Failure::Revoked)
        ));
    }
    let after = recorded(root.path(), &reference);
    assert_eq!(after, before);
    assert_eq!(after.binding, binding());
    assert_eq!(after.instance_revision, "config-1");
}

#[test]
fn other_connections_of_the_instance_keep_their_revision_until_revalidated() {
    let (root, registry) = fixture();
    let (first, first_revision, _) = connected(&registry, "one");
    let (second, second_revision, _) = connected(&registry, "two");
    let captured = registry
        .capture_revalidation(&upgraded(), &first, &first_revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER)
        .unwrap();

    let kept = recorded(root.path(), &second);
    assert_eq!(kept.binding, binding());
    assert_eq!(kept.instance_revision, "config-2");
    let stale = observe(&registry, "config-2", &second, LATER);
    assert!(stale.stale);
    assert_eq!(stale.state, State::Pending);
    assert_eq!(
        registry.admit_read(&upgraded(), &second, &BTreeSet::new(), LATER),
        Err(Failure::UpgradeRequired)
    );
    // The kept revision replays from the record in a fresh handle.
    drop(registry);
    let registry = Registry::new(root.path());
    assert_eq!(recorded(root.path(), &second), kept);

    let captured = registry
        .capture_revalidation(&upgraded(), &second, &second_revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(validated("two", LATER)), LATER)
        .unwrap();
    for reference in [&first, &second] {
        assert_eq!(recorded(root.path(), reference).binding, upgraded());
        assert_eq!(
            observe(&registry, "config-2", reference, LATER).state,
            State::Ready
        );
    }
    // A new connection of the instance is now admitted under the new revision.
    registry.begin(&upgraded(), LATER).unwrap();
}

// Adversary: the upgrade moves its instance's revision, so the metadata record
// must re-record every other connection of the instance with the revision it
// keeps. A revoked connection is a terminal record. An instance that once had a
// connection revoked must still let its live connections follow an upgrade.
#[test]
fn adversary_a_revoked_sibling_does_not_block_the_upgrade_of_a_live_connection() {
    let (root, registry) = fixture();
    let (live, live_revision, _) = connected(&registry, "one");
    let (gone, gone_revision, _) = connected(&registry, "two");
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &gone,
            &gone_revision,
            NOW,
        )
        .unwrap();
    let captured = registry
        .capture_revalidation(&upgraded(), &live, &live_revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    assert_eq!(
        registry.finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER),
        Ok(())
    );
    drop(registry);
    let reopened = Registry::new(root.path());
    assert_eq!(recorded(root.path(), &live).binding, upgraded());
    assert_eq!(
        observe(&reopened, "config-2", &live, LATER).state,
        State::Ready
    );
}

/// story:pending-connection-refusal-names-its-remedy: a connection saved under
/// another configuration revision is `pending` and no status change is pending
/// for it, so its read and approval refusals name the step that clears it:
/// revalidation while only the revision differs, a new connection when the
/// authentication changed too. Never the plain conflict a retry repeats.
#[test]
fn a_stale_connection_names_the_step_that_clears_it() {
    let (_root, registry) = fixture();
    let (reference, _, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    let stale = observe(&registry, "config-2", &reference, LATER);
    assert!(stale.stale);
    assert_eq!(stale.state, State::Pending);
    assert_eq!(
        registry.admit_read(&upgraded(), &reference, &none, LATER),
        Err(Failure::UpgradeRequired)
    );
    assert!(matches!(
        registry.capture_read(&upgraded(), &reference, &none, LATER, LATER + 1000),
        Err(Failure::UpgradeRequired)
    ));
    assert_eq!(
        registry.approval_target(&upgraded(), &reference, &none),
        Err(Failure::UpgradeRequired)
    );
    let mut changed = upgraded();
    changed.profile.revision = "profile-2".into();
    assert_eq!(
        registry.admit_read(&changed, &reference, &none, LATER),
        Err(Failure::BindingChanged)
    );
    assert!(matches!(
        registry.capture_read(&changed, &reference, &none, LATER, LATER + 1000),
        Err(Failure::BindingChanged)
    ));
    assert_eq!(
        registry.approval_target(&changed, &reference, &none),
        Err(Failure::BindingChanged)
    );
}

/// story:pending-connection-refusal-names-its-remedy: `create_connection` is a
/// remedy only if a connection can be made under the configured revision while
/// an older connection of the instance has not followed it, as after an upgrade
/// the new provider refused. Publishing the new connection moves the instance,
/// and its approval keys then answer under the configured revision; the older
/// connection keeps its own revision and can still follow the upgrade, which
/// removes its recorded refusal.
#[test]
fn a_new_connection_under_the_configured_revision_moves_the_instance() {
    use crate::local::approval_keys;
    let (root, registry) = fixture();
    let (old, old_revision, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    let mut refused = recorded(root.path(), &old);
    refused.refused = Some("config-2".into());
    // The new provider refuses the credential: only the refusal is recorded,
    // and the read under the configured revision names a new connection.
    let captured = registry
        .capture_revalidation(&upgraded(), &old, &old_revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(
            dispatched,
            Err(Some(InvalidCredential::Insufficient)),
            LATER,
        )
        .unwrap();
    assert_eq!(recorded(root.path(), &old), refused);
    assert_eq!(
        registry.admit_read(&upgraded(), &old, &none, LATER),
        Err(Failure::BindingChanged)
    );
    assert_eq!(
        registry.approval_target(&upgraded(), &old, &none),
        Err(Failure::BindingChanged)
    );
    // Revalidation stays admitted: the provider side may have changed since.
    registry
        .admit_revalidation(&upgraded(), &old, &old_revision, LATER)
        .unwrap();
    let keys = |configuration: &str| {
        approval_keys::Store::new(
            root.path(),
            None,
            "fixture-instance",
            "fixture-adapter",
            configuration,
        )
        .unwrap()
        .status()
    };
    assert_eq!(
        keys("config-2").unwrap_err(),
        approval_keys::Failure::BindingChanged
    );

    let acquisition = registry.begin(&upgraded(), LATER).unwrap();
    let claim = registry.consume(acquisition, LATER).unwrap();
    let candidate = registry
        .prepare(&claim, validated("two", LATER), 12, LATER)
        .unwrap();
    let new = publish_fixture(&registry, candidate, LATER);
    assert_eq!(recorded(root.path(), &new).instance_revision, "config-2");
    assert_eq!(
        observe(&registry, "config-2", &new, LATER).state,
        State::Ready
    );
    assert!(keys("config-2").unwrap().is_none());
    let kept = recorded(root.path(), &old);
    assert_eq!(kept.binding, binding());
    assert_eq!(kept.refused.as_deref(), Some("config-2"));
    assert!(observe(&registry, "config-2", &old, LATER).stale);
    // The record replays to the same projection in a fresh handle.
    drop(registry);
    let registry = Registry::new(root.path());
    assert_eq!(recorded(root.path(), &old), kept);
    let captured = registry
        .capture_revalidation(&upgraded(), &old, &old_revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER)
        .unwrap();
    let upgraded_record = recorded(root.path(), &old);
    assert_eq!(upgraded_record.binding, upgraded());
    assert!(upgraded_record.refused.is_none());
}

/// Adversary (story:pending-connection-refusal-names-its-remedy): a stale
/// connection whose credential is known invalid is refused on read as
/// `UpgradeRequired` (`revalidate_connection`), but revalidation admits no
/// invalid credential (`NotReady`, `repair_connection`) and repair refuses the
/// changed binding (`IdentityMismatch`, `repair_connection` again). The named
/// remedy never clears it; only a new connection does.
#[test]
fn adversary_a_stale_invalid_connection_is_not_sent_to_a_revalidation_that_refuses_it() {
    let (_root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    // Under its own revision the provider answers the credential invalid.
    let captured = registry
        .capture_revalidation(&binding(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(dispatched, Err(Some(InvalidCredential::Invalid)), LATER)
        .unwrap();
    // The instance is then configured at config-2.
    let revision = observe(&registry, "config-2", &reference, LATER).revision;
    let read = registry.admit_read(&upgraded(), &reference, &none, LATER);
    let revalidation = registry.admit_revalidation(&upgraded(), &reference, &revision, LATER);
    let repair = registry
        .begin_repair(&upgraded(), &reference, &revision, LATER)
        .map(|_| ());
    assert!(
        read != Err(Failure::UpgradeRequired) || revalidation.is_ok(),
        "read names revalidation ({read:?}), revalidation refuses ({revalidation:?}), \
         repair refuses ({repair:?})"
    );
}

/// Adversary (story:pending-connection-refusal-names-its-remedy): semantics.md
/// says the instance's recorded revision moves "when a revalidation upgrades a
/// connection or a new connection is made under it", and the commit says "as
/// an upgrading revalidation does", which moves it only on a validated
/// success. An acquisition that is begun under the configured revision and
/// then fails publishes no connection and must not move the instance.
#[test]
fn adversary_a_failed_connect_under_the_configured_revision_does_not_move_the_instance() {
    let (root, registry) = fixture();
    let (old, _, _) = connected(&registry, "one");
    let before = recorded(root.path(), &old);
    assert_eq!(before.instance_revision, "config-1");
    let acquisition = registry.begin(&upgraded(), LATER).unwrap();
    let claim = registry.consume(acquisition, LATER).unwrap();
    // The new provider refuses the entered credential: nothing is published.
    registry.fail(&claim, LATER).unwrap();
    assert_eq!(
        recorded(root.path(), &old).instance_revision,
        "config-1",
        "a connect that published nothing moved the instance's configuration revision"
    );
}

/// Adversary (story acceptance: "a pending connection whose revalidation
/// cannot succeed; invoke ... answer a refusal whose next_action is the step
/// that clears it"). After the new provider refused the upgrade, revalidation
/// answers `create_connection` (supervisor.rs), yet the read still names
/// `revalidate_connection` through `UpgradeRequired`.
#[test]
fn adversary_after_a_refused_upgrade_the_read_does_not_name_revalidation_again() {
    let (_root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, LATER, LATER + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, LATER).unwrap();
    registry
        .finish_revalidation(
            dispatched,
            Err(Some(InvalidCredential::Insufficient)),
            LATER,
        )
        .unwrap();
    assert_ne!(
        registry.admit_read(&upgraded(), &reference, &none, LATER),
        Err(Failure::UpgradeRequired),
        "the read names the revalidation the new provider already refused"
    );
}
