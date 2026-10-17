//! `contracts/auth/connection/v1alpha1/semantics.md` §4.2, as amended by
//! story:ess-connector-vocabulary-model, states that an instance holds at most
//! one non-revoked connection and that allocation is refused while the
//! instance holds a Live or Stale connection; its §6 positive test says a
//! second connect on such an instance is refused. The model carries the rule
//! as `AllocateConnection`'s owner-supplied `held_connection_ref` input
//! (`ess/domains/auth_bindings.yaml`), which the host never supplies, so the
//! host's allocation still admits a second Live connection.
//!
//! Unix only: the state directory is created with private mode bits.
#![cfg(unix)]

use connectors_host::local::{
    metadata::Metadata,
    registry::{Binding, Purpose, Registry, StaticProfile, Subject},
};
use std::{collections::BTreeSet, fs, os::unix::fs::PermissionsExt, path::Path};

const NOW: u64 = 1_788_998_400_000;

fn binding(instance: &str) -> Binding {
    Binding {
        instance_id: instance.into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "config-1".into(),
        provider_authority: "https://fixture.invalid/fixed-authority".into(),
        profile: StaticProfile {
            id: "fixture-token".into(),
            revision: "profile-1".into(),
            purpose: Purpose::DelegatedUser,
            subject: Subject::User,
            minimum_scopes: BTreeSet::from(["read".to_owned()]),
            evidence_lifetime_ms: 60_000,
            declaration: None,
        },
    }
}

fn initialized(state: &Path) -> Registry {
    fs::create_dir_all(state).unwrap();
    fs::set_permissions(state, fs::Permissions::from_mode(0o700)).unwrap();
    drop(Metadata::initialize(state).unwrap());
    Registry::new(state)
}

#[test]
fn a_second_allocation_on_an_instance_holding_a_live_connection_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let registry = initialized(&root.path().join("state"));

    let first = registry.begin(&binding("mail.personal"), NOW).unwrap();
    // Control: another instance still allocates, so a refusal below is the
    // cardinality rule and not a broken fixture.
    registry.begin(&binding("mail.work"), NOW + 1).unwrap();

    match registry.begin(&binding("mail.personal"), NOW + 2) {
        Err(_) => {}
        Ok(second) => panic!(
            "instance mail.personal holds Live connection {} and a second allocation \
             was admitted as {} (semantics.md §4.2: refused while the instance holds a \
             Live or Stale connection)",
            first.connection_reference(),
            second.connection_reference()
        ),
    }
}
