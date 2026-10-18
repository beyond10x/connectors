//! story:connection-follows-configuration-upgrade: revalidating a connection
//! whose binding names an older configuration revision of its instance moves
//! it to the configured revision without a credential entry, when the
//! authentication it was admitted under is unchanged.
use super::tests::{NOW, binding, fixture, prepared, prepared_together, publish_fixture};
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

/// Connections of one instance, as one held them before
/// story:connect-refused-while-instance-holds-connection.
fn connected_together(
    registry: &Registry,
    subjects: &[&str],
) -> Vec<(String, String, custody::Version)> {
    prepared_together(registry, subjects, NOW)
        .into_iter()
        .map(|candidate| {
            let version = candidate.version();
            let reference = publish_fixture(registry, candidate, NOW);
            let revision = observe(registry, "config-1", &reference, NOW).revision;
            (reference, revision, version)
        })
        .collect()
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
        // story:revision-change-keeps-a-recovery-path: a changed identity
        // names a new instance id, as connect under this one refuses it too.
        // Another profile id is no change of this profile's authentication.
        // A lifetime change no declaration proves stays another authentication.
        let refused = if name == "profile id" {
            Failure::BindingChanged
        } else {
            Failure::AuthenticationChanged
        };
        let before = recorded(root.path(), &reference);
        assert_eq!(
            registry.admit_revalidation(&target, &reference, &revision, LATER),
            Err(refused),
            "{name}"
        );
        assert_eq!(
            registry
                .capture_revalidation(&target, &reference, &revision, LATER, LATER + 1000)
                .map(|_| ())
                .err(),
            Some(refused),
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

/// A new connection under another revision of its instance is refused when it
/// changes the provider authority or the declaration of a profile the instance
/// already holds: both still need a new instance id. A connection only begun
/// counts, and so does a revoked one. Another profile id is admitted.
#[test]
fn a_new_connection_that_changes_authority_or_profile_needs_a_new_instance_id() {
    let refused: [(&str, Change); 5] = [
        ("provider authority", |b| {
            b.provider_authority.push_str("/other")
        }),
        ("profile revision", |b| {
            b.profile.revision = "profile-2".into()
        }),
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
    for (name, change) in refused {
        let (_root, registry) = fixture();
        registry.begin(&binding(), NOW).unwrap();
        let mut target = upgraded();
        change(&mut target);
        assert_eq!(
            registry.begin(&target, LATER).err(),
            Some(Failure::AuthenticationChanged),
            "begun: {name}"
        );
        assert!(
            registry.needs_new_instance(&target, LATER).unwrap(),
            "begun, a new instance id: {name}"
        );

        let (_root, registry) = fixture();
        let (reference, revision, _) = connected(&registry, "one");
        registry
            .revoke(
                "fixture-instance",
                "fixture-adapter",
                &reference,
                &revision,
                LATER,
            )
            .unwrap();
        assert_eq!(
            registry.begin(&target, LATER + 1).err(),
            Some(Failure::AuthenticationChanged),
            "revoked: {name}"
        );
        assert!(
            registry.needs_new_instance(&target, LATER + 1).unwrap(),
            "revoked, a new instance id: {name}"
        );
    }
    // The authentication check admits these; the instance's first connect is
    // still in flight, which `begin` alone refuses
    // (story:connect-refused-while-instance-holds-connection).
    let (_root, registry) = fixture();
    registry.begin(&binding(), NOW).unwrap();
    let mut other = upgraded();
    other.profile.id = "fixture-other".into();
    other.profile.revision = "profile-2".into();
    assert_eq!(
        registry.begin(&other, LATER).map(|_| ()),
        Err(Failure::Conflict)
    );
    registry.begin_unchecked(&other, LATER).unwrap();
    registry.begin_unchecked(&upgraded(), LATER).unwrap();
    // Neither another profile id nor the unchanged binding needs a new
    // instance id (task:definition-moved-authority-new-instance).
    assert!(!registry.needs_new_instance(&other, LATER).unwrap());
    assert!(!registry.needs_new_instance(&upgraded(), LATER).unwrap());
    assert!(!registry.needs_new_instance(&binding(), LATER).unwrap());
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
    let mut together = connected_together(&registry, &["one", "two"]).into_iter();
    let (first, first_revision, _) = together.next().unwrap();
    let (second, second_revision, _) = together.next().unwrap();
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
    // A new connection of the instance is admitted under the new revision
    // once both it holds are revoked (story:connect-refused-while-instance-
    // holds-connection); until then the connect names their revocation.
    assert_eq!(
        registry.begin(&upgraded(), LATER).map(|_| ()),
        Err(Failure::InstanceConnectionChanged)
    );
    for reference in [&first, &second] {
        let revision = observe(&registry, "config-2", reference, LATER).revision;
        registry
            .revoke(
                "fixture-instance",
                "fixture-adapter",
                reference,
                &revision,
                LATER,
            )
            .unwrap();
    }
    registry.begin(&upgraded(), LATER).unwrap();
}

// Adversary: the upgrade moves its instance's revision, so the metadata record
// must re-record every other connection of the instance with the revision it
// keeps. A revoked connection is a terminal record. An instance that once had a
// connection revoked must still let its live connections follow an upgrade.
#[test]
fn adversary_a_revoked_sibling_does_not_block_the_upgrade_of_a_live_connection() {
    let (root, registry) = fixture();
    let mut together = connected_together(&registry, &["one", "two"]).into_iter();
    let (live, live_revision, _) = together.next().unwrap();
    let (gone, gone_revision, _) = together.next().unwrap();
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
    // An approval reads the wall clock to see expiry: this fixture's
    // credential (connected at NOW, expiring an hour later) has expired, so
    // a revalidation would refuse it and only a new connection helps.
    assert_eq!(
        registry.approval_target(&upgraded(), &reference, &none),
        Err(Failure::BindingChanged)
    );
    // A credential current at the wall clock still follows the upgrade.
    let (_current_root, current) = fixture();
    let wall = connectors_sdk::now_ms();
    let (_, candidate) = prepared(&current, "one", wall);
    let fresh = publish_fixture(&current, candidate, wall);
    assert_eq!(
        current.approval_target(&upgraded(), &fresh, &none),
        Err(Failure::UpgradeRequired)
    );
    let mut changed = upgraded();
    changed.profile.revision = "profile-2".into();
    assert_eq!(
        registry.admit_read(&changed, &reference, &none, LATER),
        Err(Failure::AuthenticationChanged)
    );
    assert!(matches!(
        registry.capture_read(&changed, &reference, &none, LATER, LATER + 1000),
        Err(Failure::AuthenticationChanged)
    ));
    assert_eq!(
        registry.approval_target(&changed, &reference, &none),
        Err(Failure::AuthenticationChanged)
    );
}

/// story:pending-connection-refusal-names-its-remedy: `create_connection` is a
/// remedy only if a connection can be made under the configured revision while
/// an older connection of the instance has not followed it, as after an upgrade
/// the new provider refused. Publishing the new connection moves the instance,
/// and its approval keys then answer under the configured revision; the older
/// connection keeps its own revision and can still follow the upgrade, which
/// removes its recorded refusal.
///
/// story:connect-refused-while-instance-holds-connection: a connect beside the
/// older connection is now refused naming its revocation, since no reconnect
/// replaces it. The new connection here was begun while the older one was
/// still in flight, as before that rule, so the rest of this record (both
/// connections, the older one following later) is the one an instance from
/// before the rule can still reach.
#[test]
fn a_new_connection_under_the_configured_revision_moves_the_instance() {
    use crate::local::approval_keys;
    let (root, registry) = fixture();
    let (_, old_candidate) = prepared(&registry, "one", NOW);
    let acquisition = registry.begin_unchecked(&upgraded(), NOW).unwrap();
    let old = publish_fixture(&registry, old_candidate, NOW);
    let old_revision = observe(&registry, "config-1", &old, NOW).revision;
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
    assert_eq!(
        registry.begin(&upgraded(), LATER).map(|_| ()),
        Err(Failure::InstanceConnectionChanged)
    );

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
    let (old, old_revision, _) = connected(&registry, "one");
    let before = recorded(root.path(), &old);
    assert_eq!(before.instance_revision, "config-1");
    // The instance holds `old`, which can follow the upgrade, so a connect
    // beside it names that revalidation; the connect under the configured
    // revision follows its revocation
    // (story:connect-refused-while-instance-holds-connection).
    assert_eq!(
        registry.begin(&upgraded(), LATER).map(|_| ()),
        Err(Failure::UpgradeRequired)
    );
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &old,
            &old_revision,
            LATER,
        )
        .unwrap();
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

/// Adversary pass 2 (semantics.md: the instance's recorded revision "moves to
/// the configured one only when a connection is published under it"). A
/// connect begun under revision 1 before the configuration moved, published
/// after a connection under revision 2 moved the instance, moves the instance
/// back to revision 1: the configured revision's approval keys then refuse
/// again although a connection under it is published.
#[test]
fn adversary_a_connect_published_under_an_older_revision_does_not_move_the_instance_back() {
    use crate::local::approval_keys;
    let (root, registry) = fixture();
    // Three connects of the instance in flight at once, as before
    // story:connect-refused-while-instance-holds-connection: `begin` now
    // refuses the second while the first is in flight, so only an instance
    // from before that rule reaches the publications below.
    let (_, one) = prepared(&registry, "one", NOW);
    assert_eq!(
        registry.begin(&binding(), LATER).map(|_| ()),
        Err(Failure::Conflict)
    );
    // A connect under revision 1 is in progress when the instance is
    // reconfigured at revision 2.
    let acquisition = registry.begin_unchecked(&binding(), LATER).unwrap();
    let old_claim = registry.consume(acquisition, LATER).unwrap();
    let old_prepared = registry
        .prepare(&old_claim, validated("two", LATER), 12, LATER)
        .unwrap();
    // A connect under revision 2 publishes and moves the instance.
    let acquisition = registry.begin_unchecked(&upgraded(), LATER).unwrap();
    publish_fixture(&registry, one, LATER);
    let claim = registry.consume(acquisition, LATER).unwrap();
    let prepared = registry
        .prepare(&claim, validated("three", LATER), 12, LATER)
        .unwrap();
    let new = publish_fixture(&registry, prepared, LATER);
    assert_eq!(recorded(root.path(), &new).instance_revision, "config-2");
    // The older connect completes afterwards.
    let _ = publish_fixture(&registry, old_prepared, LATER);
    let keys = approval_keys::Store::new(
        root.path(),
        None,
        "fixture-instance",
        "fixture-adapter",
        "config-2",
    )
    .unwrap()
    .status();
    assert_eq!(
        recorded(root.path(), &new).instance_revision,
        "config-2",
        "a connection published under an older revision moved the instance back ({keys:?})"
    );
}

/// Adversary pass 2 (ESS auth_bindings: "An upgrade that succeeds removes it;
/// a refusal under another revision replaces it"). The refusal is recorded for
/// the credential the provider refused; a repair under the connection's own
/// revision replaces that credential and keeps the refusal. When the
/// configuration moves to the refused revision again, the read names a new
/// connection, while a revalidation of the repaired credential upgrades it.
#[test]
fn adversary_a_repair_clears_the_refusal_recorded_for_the_replaced_credential() {
    let (root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    // The new provider refuses the credential's scope under revision 2.
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
    assert_eq!(
        recorded(root.path(), &reference).refused.as_deref(),
        Some("config-2")
    );
    // The configuration is rolled back to revision 1, the credential repaired
    // with a broader one for the same identity.
    let at = LATER + 10;
    let acquisition = registry
        .begin_repair(&binding(), &reference, &revision, at)
        .unwrap();
    let claim = registry.consume(acquisition, at).unwrap();
    let prepared = registry
        .prepare(&claim, validated("one", at), 12, at)
        .unwrap();
    publish_fixture(&registry, prepared, at);
    // Revision 2 is configured again.
    let at = at + 10;
    let read = registry.admit_read(&upgraded(), &reference, &none, at);
    let revision = observe(&registry, "config-2", &reference, at).revision;
    let captured = registry
        .capture_revalidation(&upgraded(), &reference, &revision, at, at + 1000)
        .unwrap();
    let dispatched = registry.dispatch_revalidation(captured, at).unwrap();
    let revalidation = registry.finish_revalidation(dispatched, Ok(validated("one", at)), at);
    assert!(
        read == Err(Failure::UpgradeRequired) || revalidation.is_err(),
        "the read names a new connection ({read:?}) for a repaired credential the \
         new provider never refused; revalidation upgrades it ({revalidation:?})"
    );
}

/// Adversary pass 2 (semantics.md: `revalidate_connection` "only when ... its
/// credential is neither known invalid nor expired"; otherwise
/// `create_connection` for reads, approvals and launches). An approval target
/// for a stale connection whose credential has expired names revalidation;
/// revalidation refuses it `NotReady` (`repair_connection`) and repair refuses
/// the changed binding (`IdentityMismatch`, `repair_connection` again).
#[test]
fn adversary_an_approval_for_a_stale_expired_connection_does_not_name_revalidation() {
    let (_root, registry) = fixture();
    let (reference, revision, _) = connected(&registry, "one");
    let none = BTreeSet::new();
    // Past the credential's expiry (validated: NOW + 3_600_000).
    let expired = NOW + 3_600_000;
    let approval = registry.approval_target(&upgraded(), &reference, &none);
    let revalidation = registry.admit_revalidation(&upgraded(), &reference, &revision, expired);
    let repair = registry
        .begin_repair(&upgraded(), &reference, &revision, expired)
        .map(|_| ());
    assert!(
        approval != Err(Failure::UpgradeRequired) || revalidation.is_ok(),
        "approval names revalidation ({approval:?}), revalidation refuses \
         ({revalidation:?}), repair refuses ({repair:?})"
    );
}

/// story:revision-change-keeps-a-recovery-path: the profile `profile.id` an
/// adapter declares, digested into its revision as every shipped adapter does
/// (`adapters/catalog/src/local.rs`), with this evidence lifetime and these
/// minimum scopes.
fn declared(lifetime: u64, scopes: &[&str], label: &str) -> StaticProfile {
    let mut profile = crate::local::runtime::Profile {
        id: "fixture-token".into(),
        revision: String::new(),
        purpose: Purpose::DelegatedUser,
        subject: Subject::User,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: scopes.iter().map(|scope| (*scope).to_owned()).collect(),
        evidence_lifetime_ms: lifetime,
        fields: vec![crate::local::runtime::EntryField {
            name: "token".into(),
            label: label.into(),
            max_bytes: 1024,
        }],
        acquisition: None,
    };
    profile.revision = connectors_core::digest(&serde_json::to_value(&profile).unwrap());
    profile.registry_profile()
}

fn declared_binding(configuration: &str, profile: StaticProfile) -> Binding {
    Binding {
        configuration_revision: configuration.into(),
        profile,
        ..binding()
    }
}

/// A connection published under `admitted`; its reference and public revision.
fn connected_under(registry: &Registry, admitted: &Binding) -> (String, String) {
    let acquisition = registry.begin(admitted, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let candidate = registry
        .prepare(&claim, validated("one", NOW), 12, NOW)
        .unwrap();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = observe(registry, "config-1", &reference, NOW).revision;
    (reference, revision)
}

/// The observed defect: raising the adapter's evidence lifetime moves its
/// profile revision, and the connection was refused by revalidate
/// (`BindingChanged`, `create_connection`), connect (`Conflict`,
/// `retry_status`) and repair (`IdentityMismatch`, `repair_connection`), so no
/// next action led anywhere. The lifetime is not the connection's identity,
/// but a connection stays bound to the declaration record it was admitted
/// under, so it is not upgraded in place: revalidate, a read and a repair name
/// a new connection, connect under the same instance id admits it and moves
/// the instance, and the old connection stays revocable.
///
/// story:connect-refused-while-instance-holds-connection: the instance holds
/// the old connection, which no reconnect replaces, so the connect names its
/// revocation; once revoked, the connect is admitted and moves the instance.
/// Read, connect and revoke each name a step that leads on.
#[test]
fn an_evidence_lifetime_change_is_followed_by_a_new_connection() {
    let admitted = declared_binding("config-1", declared(60_000, &["read"], "Token"));
    let configured = declared_binding("config-2", declared(300_000, &["read"], "Token"));
    assert_ne!(admitted.profile.revision, configured.profile.revision);
    assert!(admitted.profile.same_authentication(&configured.profile));
    let none = BTreeSet::new();

    let (root, registry) = fixture();
    let (reference, revision) = connected_under(&registry, &admitted);
    assert_eq!(
        registry.admit_read(&configured, &reference, &none, LATER),
        Err(Failure::BindingChanged)
    );
    assert_eq!(
        registry.admit_revalidation(&configured, &reference, &revision, LATER),
        Err(Failure::BindingChanged)
    );
    assert_eq!(
        registry
            .begin_repair(&configured, &reference, &revision, LATER)
            .map(|_| ()),
        Err(Failure::BindingChanged)
    );
    assert_eq!(
        registry.begin(&configured, LATER).map(|_| ()),
        Err(Failure::InstanceConnectionChanged)
    );
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();

    let acquisition = registry.begin(&configured, LATER + 1).unwrap();
    let claim = registry.consume(acquisition, LATER + 1).unwrap();
    let candidate = registry
        .prepare(&claim, validated("one", LATER + 1), 12, LATER + 1)
        .unwrap();
    let fresh = publish_fixture(&registry, candidate, LATER + 1);
    let observed = observe(&registry, "config-2", &fresh, LATER + 1);
    assert_eq!(observed.state, State::Ready);
    assert!(!observed.stale);
    registry
        .admit_read(&configured, &fresh, &none, LATER + 1)
        .unwrap();
    assert_eq!(recorded(root.path(), &fresh).instance_revision, "config-2");
    // The lifetime alone is not proved by a declaration-less profile.
    let mut unproved = configured.profile.clone();
    unproved.declaration = None;
    assert!(!admitted.profile.same_authentication(&unproved));
    // The same instance id holds it: no new one is needed.
    assert!(!registry.needs_new_instance(&configured, LATER + 2).unwrap());
}

/// A change to the connection's identity, the provider authority or the
/// profile's authentication (here its minimum scopes, or an entry field with
/// the lifetime changed beside it), refuses every command under the same
/// instance id by one name, `AuthenticationChanged`: only a new instance id
/// helps, so none of them names another command that refuses again.
#[test]
fn an_identity_change_names_a_new_instance_id_on_every_command() {
    let admitted = declared_binding("config-1", declared(60_000, &["read"], "Token"));
    let changes: [(&str, Binding); 3] = [
        ("provider authority", {
            let mut configured = declared_binding("config-2", declared(60_000, &["read"], "Token"));
            configured.provider_authority.push_str("/other");
            configured
        }),
        (
            "minimum scopes",
            declared_binding("config-2", declared(60_000, &["read", "write"], "Token")),
        ),
        (
            "entry field beside the lifetime",
            declared_binding("config-2", declared(300_000, &["read"], "Other token")),
        ),
    ];
    let none = BTreeSet::new();
    for (name, configured) in changes {
        let (root, registry) = fixture();
        let (reference, revision) = connected_under(&registry, &admitted);
        let before = recorded(root.path(), &reference);
        assert_eq!(
            registry.admit_read(&configured, &reference, &none, LATER),
            Err(Failure::AuthenticationChanged),
            "read: {name}"
        );
        assert_eq!(
            registry.admit_revalidation(&configured, &reference, &revision, LATER),
            Err(Failure::AuthenticationChanged),
            "revalidate: {name}"
        );
        assert_eq!(
            registry
                .begin_repair(&configured, &reference, &revision, LATER)
                .map(|_| ()),
            Err(Failure::AuthenticationChanged),
            "repair: {name}"
        );
        assert_eq!(
            registry.begin(&configured, LATER).err(),
            Some(Failure::AuthenticationChanged),
            "connect: {name}"
        );
        assert_eq!(recorded(root.path(), &reference), before, "{name}");
    }
}

/// story:revision-change-keeps-a-recovery-path: a changed provider authority
/// or profile authentication moves the adapter entry to a new instance id, and
/// the connection left under the earlier one stays revocable through that
/// entry while no entry configures the earlier id; one another entry still
/// configures, or one of another adapter, is not this entry's to revoke.
#[test]
fn a_connection_left_under_an_unconfigured_instance_id_stays_revocable() {
    let (root, registry) = fixture();
    let (reference, revision) = connected_under(&registry, &binding());
    let moved = "fixture-instance-2";
    let both = BTreeSet::from(["fixture-instance".to_owned(), moved.to_owned()]);
    let only_moved = BTreeSet::from([moved.to_owned()]);
    assert_eq!(
        registry.revoke(moved, "fixture-adapter", &reference, &revision, LATER),
        Err(Failure::NotFound)
    );
    assert_eq!(
        registry.revoke_left_behind(
            moved,
            "fixture-adapter",
            &both,
            &reference,
            &revision,
            LATER
        ),
        Err(Failure::NotFound)
    );
    assert_eq!(
        registry.revoke_left_behind(
            moved,
            "other-adapter",
            &only_moved,
            &reference,
            &revision,
            LATER
        ),
        Err(Failure::NotFound)
    );
    let before = recorded(root.path(), &reference);
    assert!(before.material.is_some());
    let revoked = registry
        .revoke_left_behind(
            moved,
            "fixture-adapter",
            &only_moved,
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    let after = recorded(root.path(), &reference);
    assert_eq!(after.revision, revoked);
    assert_eq!(after.material, None);
    assert_eq!(
        observe(&registry, "config-1", &reference, LATER + 1).state,
        State::Revoked
    );
    // Repeating it answers the revocation already recorded.
    assert_eq!(
        registry.revoke_left_behind(
            moved,
            "fixture-adapter",
            &only_moved,
            &reference,
            &revision,
            LATER + 2
        ),
        Ok(revoked)
    );
}

/// Security review of task:definition-moved-authority-new-instance,
/// invariant 5 (nothing reaches a connection of another connector): the
/// connection a moved definition leaves under `forge.definition` is revocable
/// through the moved entry of `forge`, and through no instance of another
/// connector. Every invoked definition's entry has the adapter `catalog`, and
/// `connections revoke` passes every configured instance id
/// (apps/connectors/src/local.rs), so the adapter scope alone spans
/// connectors.
#[test]
fn a_left_behind_definition_connection_is_not_revocable_through_another_connector() {
    let (_root, registry) = fixture();
    let mut left = binding();
    left.instance_id = "forge.definition".into();
    left.adapter_id = "catalog".into();
    let acquisition = registry.begin(&left, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let candidate = registry
        .prepare(&claim, validated("one", NOW), 12, NOW)
        .unwrap();
    let reference = publish_fixture(&registry, candidate, NOW);
    let revision = registry
        .describe(
            "forge.definition",
            "catalog",
            "config-1",
            &reference,
            NOW,
            true,
        )
        .unwrap()
        .revision;
    // `forge` moved to its own new instance; `other` is another connector's
    // invoked definition. Neither configures `forge.definition` any longer.
    let configured = BTreeSet::from([
        "forge.definition-0123456789abcdef".to_owned(),
        "other.definition".to_owned(),
    ]);
    assert_eq!(
        registry.revoke_left_behind(
            "other.definition",
            "catalog",
            &configured,
            &reference,
            &revision,
            LATER
        ),
        Err(Failure::NotFound),
        "a connection of `forge` was revoked through the connector `other`"
    );
    // A connector whose id only starts with `forge` is another connector.
    assert_eq!(
        registry.revoke_left_behind(
            "forge-file.definition",
            "catalog",
            &configured,
            &reference,
            &revision,
            LATER
        ),
        Err(Failure::NotFound)
    );
    // The moved entry of `forge` reaches it.
    assert!(
        registry
            .revoke_left_behind(
                "forge.definition-0123456789abcdef",
                "catalog",
                &configured,
                &reference,
                &revision,
                LATER
            )
            .is_ok()
    );
}
