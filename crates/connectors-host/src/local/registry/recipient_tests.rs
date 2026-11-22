//! Security review 2026-11-22, finding 1: a connection whose profile exchanges
//! its stored material at a token endpoint (the catalog's
//! `oauth2_client_credentials`) is bound to that endpoint's canonical
//! recipient (`Binding::token_recipient`). A configuration that moves it is no
//! upgrade a revalidation follows in place: the revalidation would read the
//! stored client secret and send it to the new endpoint before any identity
//! answers. It is refused at admission and at capture, so no use is captured
//! and no custody version is handed to the owner, which opens custody only
//! for a captured use (owner/supervisor.rs `revalidate_inner`). An
//! operation-only move still revalidates in place.
use super::tests::{NOW, binding, fixture, publish_fixture};
use super::*;

const LATER: u64 = NOW + 1;
const R1: &str = "recipient-1";
const R2: &str = "recipient-2";

/// Saved under a token endpoint, with the credential side and the
/// executable recorded.
fn saved() -> Binding {
    Binding {
        credential_revision: Some("credential-1".into()),
        executable: Some("a".repeat(64)),
        token_recipient: Some(R1.into()),
        ..binding()
    }
}

/// Only the operation side moved: another selection or a rebuilt bundle.
fn operations() -> Binding {
    Binding {
        configuration_revision: "config-2".into(),
        ..saved()
    }
}

/// The credential side moved and the token recipient did not: another API
/// CA bundle, say. D12 still follows it in place.
fn trust() -> Binding {
    Binding {
        configuration_revision: "config-2".into(),
        credential_revision: Some("credential-2".into()),
        ..saved()
    }
}

/// The token endpoint moved (another host, port or path, or other trust
/// roots for it): the credential side and the recipient both move.
fn moved() -> Binding {
    Binding {
        token_recipient: Some(R2.into()),
        ..trust()
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

fn configured(binding: &Binding) -> Configured<'_> {
    Configured {
        configuration_revision: &binding.configuration_revision,
        credential_revision: binding.credential_revision.as_deref(),
        executable: binding.executable.as_deref(),
    }
}

fn connect(registry: &Registry, binding: &Binding) -> (String, String) {
    let acquisition = registry.begin(binding, NOW).unwrap();
    let claim = registry.consume(acquisition, NOW).unwrap();
    let candidate = registry
        .prepare(&claim, validated("one", NOW), 12, NOW)
        .unwrap();
    let reference = publish_fixture(registry, candidate, NOW);
    let revision = registry
        .describe(
            "fixture-instance",
            "fixture-adapter",
            configured(binding),
            &reference,
            NOW,
            true,
        )
        .unwrap()
        .revision;
    (reference, revision)
}

/// The recorded binding column and the number of captured uses.
fn recorded(root: &Path, reference: &str) -> (String, i64) {
    let metadata = Metadata::inspect(root).unwrap();
    let binding = metadata
        .connection
        .query_row(
            "SELECT binding FROM registry_connections WHERE connection_ref=?1",
            [reference],
            |r| r.get(0),
        )
        .unwrap();
    let uses = metadata
        .connection
        .query_row("SELECT count(*) FROM registry_uses", [], |r| r.get(0))
        .unwrap();
    (binding, uses)
}

fn revalidate(
    registry: &Registry,
    configured: &Binding,
    reference: &str,
    revision: &str,
) -> Result<()> {
    let captured =
        registry.capture_revalidation(configured, reference, revision, LATER, LATER + 1000)?;
    let dispatched = registry.dispatch_revalidation(captured, LATER)?;
    registry.finish_revalidation(dispatched, Ok(validated("one", LATER)), LATER)
}

#[test]
fn security_an_operation_only_move_of_a_token_profile_still_revalidates_in_place() {
    for target in [operations(), trust()] {
        let (root, registry) = fixture();
        let (reference, revision) = connect(&registry, &saved());
        revalidate(&registry, &target, &reference, &revision).unwrap();
        let (stored, _) = recorded(root.path(), &reference);
        assert_eq!(decode_stored(&stored).unwrap().0, target);
    }
}

#[test]
fn security_a_moved_token_recipient_is_refused_before_any_use_is_captured() {
    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &saved());
    let before = recorded(root.path(), &reference);
    assert_eq!(before.1, 0);
    assert!(!saved().follows_in_place(&moved()));
    assert_eq!(follow(&saved(), &moved()), Follow::Changed);
    assert_eq!(
        registry.admit_revalidation(&moved(), &reference, &revision, LATER),
        Err(Failure::BindingChanged),
        "a moved token endpoint is followed in place"
    );
    assert_eq!(
        registry
            .capture_revalidation(&moved(), &reference, &revision, LATER, LATER + 1000)
            .map(|_| ())
            .err(),
        Some(Failure::BindingChanged),
        "a revalidation captured the stored client secret for a moved token endpoint"
    );
    // Nothing captured, so no custody version reaches the owner: zero reads.
    assert_eq!(recorded(root.path(), &reference), before);
    // A read and an approval name a new connection, never a revalidation.
    let scopes = BTreeSet::from(["read".to_owned()]);
    assert_eq!(
        registry.admit_read(&moved(), &reference, &scopes, LATER),
        Err(Failure::BindingChanged)
    );
    assert_eq!(
        registry.approval_target(&moved(), &reference, &BTreeSet::new()),
        Err(Failure::BindingChanged)
    );
    // Fresh authorization: revoke, then a new connection under the same
    // instance id is admitted under the moved recipient.
    registry
        .revoke(
            "fixture-instance",
            "fixture-adapter",
            &reference,
            &revision,
            LATER,
        )
        .unwrap();
    registry.begin(&moved(), LATER).unwrap();
}

/// A connection saved before the recipient was recorded follows a
/// configuration whose credential side is unchanged (the new executable
/// records the recipient), and nothing else in place.
#[test]
fn security_a_binding_saved_without_a_recipient_follows_only_an_unchanged_credential_side() {
    let legacy = Binding {
        token_recipient: None,
        ..saved()
    };
    let recording = Binding {
        executable: Some("b".repeat(64)),
        ..saved()
    };
    assert!(legacy.follows_in_place(&recording));
    assert!(legacy.follows_in_place(&operations()));
    assert!(!legacy.follows_in_place(&trust()));
    assert!(!legacy.follows_in_place(&moved()));
    // Saved before D12 too: only the configuration revision tells.
    let older = Binding {
        credential_revision: None,
        executable: None,
        token_recipient: None,
        ..saved()
    };
    assert!(older.follows_in_place(&saved()));
    assert!(!older.follows_in_place(&operations()));

    let (root, registry) = fixture();
    let (reference, revision) = connect(&registry, &legacy);
    assert_eq!(
        registry
            .capture_revalidation(&moved(), &reference, &revision, LATER, LATER + 1000)
            .map(|_| ())
            .err(),
        Some(Failure::BindingChanged)
    );
    assert_eq!(recorded(root.path(), &reference).1, 0);
    revalidate(&registry, &recording, &reference, &revision).unwrap();
    let (stored, _) = recorded(root.path(), &reference);
    assert_eq!(decode_stored(&stored).unwrap().0, recording);
}
