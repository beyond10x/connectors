//! story:invoke-definition-reuse-credential: which stored connections a new
//! connection of another instance may copy its credential from, and the
//! capture of the one version a copy reads. A candidate is a published, Live,
//! not Stale connection of another instance whose evidence is current under its
//! configured entry, bound to the same adapter, provider authority and profile
//! authentication as the target; nothing else is offered or captured, and a
//! capture leaves the source as it was.
use super::tests::{NOW, binding, fixture};
use super::*;

const ADAPTER: &str = "fixture-adapter";
const SOURCE: &str = "source-instance";

fn source() -> Binding {
    Binding {
        instance_id: SOURCE.into(),
        ..binding()
    }
}

fn target() -> Binding {
    Binding {
        instance_id: "target-instance".into(),
        configuration_revision: "config-target".into(),
        ..binding()
    }
}

fn validated(subject: &str, now: u64) -> ValidatedBaseline {
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

/// A published connection under `binding`, and its public revision.
fn connected(registry: &Registry, binding: &Binding, subject: &str) -> (String, String) {
    let acquisition = registry.begin(binding, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let prepared = registry
        .prepare(&claim, validated(subject, NOW), 12, NOW)
        .unwrap();
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, NOW).unwrap();
    let reference = registry.publish(stored, NOW).unwrap();
    let revision = observed(registry, binding, &reference, NOW).revision;
    (reference, revision)
}

fn observed(
    registry: &Registry,
    binding: &Binding,
    reference: &str,
    now: u64,
) -> ObservedConnection {
    registry
        .describe(
            &binding.instance_id,
            ADAPTER,
            binding.configuration_revision.as_str(),
            reference,
            now,
            true,
        )
        .unwrap()
}

fn offered(
    registry: &Registry,
    instance: &str,
    configured: &str,
    target: &Binding,
    now: u64,
) -> Vec<String> {
    registry
        .reuse_candidates(instance, ADAPTER, configured, target, now)
        .unwrap()
        .into_iter()
        .map(|candidate| candidate.reference)
        .collect()
}

#[test]
fn a_ready_connection_of_another_instance_under_the_same_authority_and_profile_is_offered() {
    let (_root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    let candidates = registry
        .reuse_candidates(SOURCE, ADAPTER, "config-1", &target(), NOW + 1)
        .unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.reference, reference);
    assert_eq!(candidate.instance, SOURCE);
    assert_eq!(candidate.revision, revision);
    assert_eq!(candidate.provider_authority, source().provider_authority);
    assert_eq!(
        candidate.identity,
        ExternalIdentity {
            kind: "fixture-account".into(),
            subject: "one".into()
        }
    );
}

#[test]
fn another_authority_profile_adapter_or_the_target_instance_itself_is_never_offered() {
    let (_root, registry) = fixture();
    connected(&registry, &source(), "one");
    let mut moved = target();
    moved.provider_authority = "https://other.invalid/api".into();
    assert!(offered(&registry, SOURCE, "config-1", &moved, NOW + 1).is_empty());
    let mut profile = target();
    profile.profile.revision = "profile-2".into();
    assert!(offered(&registry, SOURCE, "config-1", &profile, NOW + 1).is_empty());
    let mut scopes = target();
    scopes.profile.minimum_scopes = BTreeSet::from(["write".to_owned()]);
    assert!(offered(&registry, SOURCE, "config-1", &scopes, NOW + 1).is_empty());
    let mut adapter = target();
    adapter.adapter_id = "other-adapter".into();
    assert!(offered(&registry, SOURCE, "config-1", &adapter, NOW + 1).is_empty());
    // The target's own instance holds no candidate for itself.
    let own = Binding {
        instance_id: SOURCE.into(),
        ..target()
    };
    assert!(offered(&registry, SOURCE, "config-1", &own, NOW + 1).is_empty());
    // A source instance asked under another adapter id sees nothing.
    assert!(
        registry
            .reuse_candidates(SOURCE, "other-adapter", "config-1", &target(), NOW + 1)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn revoked_stale_expired_and_moved_connections_are_never_offered() {
    let (_root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    // Not current under the source's configured entry.
    assert!(offered(&registry, SOURCE, "config-2", &target(), NOW + 1).is_empty());
    // Stale: the provider refused its credential.
    let captured = registry
        .capture_read(
            &source(),
            &reference,
            &BTreeSet::new(),
            NOW + 1,
            NOW + 1_000,
        )
        .unwrap();
    let dispatched = registry.dispatch_read(captured, NOW + 1).unwrap();
    registry.refuse_read(dispatched, NOW + 1).unwrap();
    assert_eq!(
        observed(&registry, &source(), &reference, NOW + 2).state,
        State::Stale
    );
    assert!(offered(&registry, SOURCE, "config-1", &target(), NOW + 2).is_empty());
    assert_eq!(
        registry
            .capture_reuse(
                SOURCE,
                ADAPTER,
                "config-1",
                &target(),
                &reference,
                &revision,
                NOW + 2,
                NOW + 10_000
            )
            .map(|_| ())
            .err(),
        Some(Failure::NotFound)
    );
    // Evidence no longer current: revalidate it first. (The registry clock
    // never moves back, so this runs on its own store.)
    let (_root, registry) = fixture();
    connected(&registry, &source(), "one");
    assert!(offered(&registry, SOURCE, "config-1", &target(), NOW + 60_001).is_empty());
    // Revoked.
    let (_root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    registry
        .revoke(SOURCE, ADAPTER, &reference, &revision, NOW + 1)
        .unwrap();
    assert!(offered(&registry, SOURCE, "config-1", &target(), NOW + 2).is_empty());
    assert_eq!(
        registry
            .capture_reuse(
                SOURCE,
                ADAPTER,
                "config-1",
                &target(),
                &reference,
                &revision,
                NOW + 2,
                NOW + 10_000
            )
            .map(|_| ())
            .err(),
        Some(Failure::Revoked)
    );
}

#[test]
fn a_capture_holds_the_current_version_once_and_leaves_the_source_unchanged() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    let before = observed(&registry, &source(), &reference, NOW + 1);
    let captured = registry
        .capture_reuse(
            SOURCE,
            ADAPTER,
            "config-1",
            &target(),
            &reference,
            &revision,
            NOW + 1,
            NOW + 10_000,
        )
        .unwrap();
    assert_eq!(captured.identity().subject, "one");
    // The capture names the source's published custody version.
    let metadata = Metadata::inspect(root.path()).unwrap();
    let (scope, material): (String, String) = metadata
        .connection
        .query_row(
            "SELECT scope_id,active_material FROM registry_connections WHERE connection_ref=?1",
            [&reference],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let authority = metadata.authority().unwrap();
    drop(metadata);
    assert!(
        captured.version()
            == super::lifecycle::custody_version(authority, &scope, &material).unwrap()
    );
    let dispatched = registry
        .dispatch_read(captured.into_read(), NOW + 2)
        .unwrap();
    registry.release_read(dispatched, NOW + 3).unwrap();
    let after = observed(&registry, &source(), &reference, NOW + 3);
    assert_eq!(
        (&after.revision, after.state, &after.identity, after.stale),
        (
            &before.revision,
            before.state,
            &before.identity,
            before.stale
        )
    );
    assert_eq!(after.state, State::Ready);
    // A capture names the revision it was offered under, and only a candidate.
    assert_eq!(
        registry
            .capture_reuse(
                SOURCE,
                ADAPTER,
                "config-1",
                &target(),
                &reference,
                "other-revision",
                NOW + 3,
                NOW + 10_000
            )
            .map(|_| ())
            .err(),
        Some(Failure::Conflict)
    );
    let mut moved = target();
    moved.provider_authority = "https://other.invalid/api".into();
    assert_eq!(
        registry
            .capture_reuse(
                SOURCE,
                ADAPTER,
                "config-1",
                &moved,
                &reference,
                &revision,
                NOW + 3,
                NOW + 10_000
            )
            .map(|_| ())
            .err(),
        Some(Failure::NotFound)
    );
    assert_eq!(
        registry
            .capture_reuse(
                "target-instance",
                ADAPTER,
                "config-1",
                &target(),
                &reference,
                &revision,
                NOW + 3,
                NOW + 10_000
            )
            .map(|_| ())
            .err(),
        Some(Failure::NotFound)
    );
}

/// A capture cancelled before any dispatch also leaves the source as it was,
/// and its use is released: it cannot be dispatched afterwards.
#[test]
fn a_cancelled_capture_is_released_and_cannot_be_dispatched() {
    let (_root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    let captured = registry
        .capture_reuse(
            SOURCE,
            ADAPTER,
            "config-1",
            &target(),
            &reference,
            &revision,
            NOW + 1,
            NOW + 10_000,
        )
        .unwrap();
    let replay = captured.read_replay();
    registry.cancel_read(captured.into_read(), NOW + 2).unwrap();
    assert_eq!(
        registry.dispatch_read(replay, NOW + 3).map(|_| ()).err(),
        Some(Failure::Conflict)
    );
    assert_eq!(
        observed(&registry, &source(), &reference, NOW + 3).state,
        State::Ready
    );
}

/// Adversary round 1 F1: the copy's publishing commit requires the source to
/// be what was captured. A source revoked, made Stale or republished while its
/// copy was validated publishes no copy; an unchanged one publishes it.
#[test]
fn a_copy_publishes_only_while_its_source_is_what_was_captured() {
    type Move = fn(&Registry, &str, &str);
    let revoke: Move = |registry, reference, revision| {
        registry
            .revoke(SOURCE, ADAPTER, reference, revision, NOW + 3)
            .unwrap();
    };
    let stale: Move = |registry, reference, _| {
        let captured = registry
            .capture_read(&source(), reference, &BTreeSet::new(), NOW + 3, NOW + 1_000)
            .unwrap();
        let dispatched = registry.dispatch_read(captured, NOW + 3).unwrap();
        registry.refuse_read(dispatched, NOW + 3).unwrap();
    };
    let unchanged: Move = |_, _, _| {};
    for (moved, expected) in [
        (revoke, Some(Failure::Revoked)),
        (stale, Some(Failure::Conflict)),
        (unchanged, None),
    ] {
        let (_root, registry) = fixture();
        let (reference, revision) = connected(&registry, &source(), "one");
        let captured = registry
            .capture_reuse(
                SOURCE,
                ADAPTER,
                "config-1",
                &target(),
                &reference,
                &revision,
                NOW + 1,
                NOW + 10_000,
            )
            .unwrap();
        let guard = captured.guard();
        let dispatched = registry.dispatch_reuse(captured, NOW + 2).unwrap();
        registry.release_read(dispatched, NOW + 2).unwrap();
        moved(&registry, &reference, &revision);
        let acquisition = registry.begin(&target(), NOW + 3).unwrap();
        let claim = registry.consume(acquisition, NOW + 3).unwrap();
        let prepared = registry
            .prepare(&claim, validated("one", NOW + 3), 12, NOW + 3)
            .unwrap();
        let receipt = custody::WrittenVersion::fixture(prepared.version());
        let stored = registry.acknowledge(prepared, receipt, NOW + 3).unwrap();
        let published = registry.publish_with(stored, NOW + 3, false, Some(&guard));
        assert_eq!(published.as_ref().err().copied(), expected, "{published:?}");
        let held = registry.held("target-instance", NOW + 4).unwrap();
        assert_eq!(held.len(), usize::from(expected.is_none()));
    }
}

/// Security round 1 F3: a dispatch the registry refuses cancels the use.
#[test]
fn a_refused_dispatch_cancels_the_reuse_use() {
    let (root, registry) = fixture();
    let (reference, revision) = connected(&registry, &source(), "one");
    let captured = registry
        .capture_reuse(
            SOURCE,
            ADAPTER,
            "config-1",
            &target(),
            &reference,
            &revision,
            NOW + 1,
            NOW + 10_000,
        )
        .unwrap();
    // Past the capture's deadline: the dispatch is refused and the use
    // released, not left to lapse.
    assert!(registry.dispatch_reuse(captured, NOW + 60_001).is_err());
    let metadata = Metadata::inspect(root.path()).unwrap();
    let open: i64 = metadata
        .connection
        .query_row(
            "SELECT count(*) FROM registry_uses WHERE released=0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(open, 0);
}
