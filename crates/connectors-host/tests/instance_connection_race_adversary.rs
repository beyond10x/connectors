//! story:connect-refused-while-instance-holds-connection, adversary: two
//! connects of one instance that start at the same moment, each through its
//! own registry handle as two CLI processes reach the owner. The contract
//! (`contracts/cli/v1alpha1/semantics.md`, one connection per instance) admits
//! exactly one and refuses the other as `lifecycle_conflict` with
//! `next_action = retry_status` (`Failure::Conflict`), never as a metadata or
//! revision failure, and never both.
//!
//! Unix only: the state directory is created with private mode bits.
#![cfg(unix)]

use connectors_host::local::{
    metadata::Metadata,
    registry::{Binding, Failure, Purpose, Registry, StaticProfile, Subject},
};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Barrier},
    thread,
};

const NOW: u64 = 1_788_998_400_000;
const ROUNDS: usize = 12;

fn binding() -> Binding {
    Binding {
        instance_id: "mail.personal".into(),
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

fn initialized(state: &Path) {
    fs::create_dir_all(state).unwrap();
    fs::set_permissions(state, fs::Permissions::from_mode(0o700)).unwrap();
    drop(Metadata::initialize(state).unwrap());
}

#[test]
fn two_simultaneous_connects_admit_one_and_refuse_the_other_as_in_flight() {
    for round in 0..ROUNDS {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        initialized(&state);
        let barrier = Arc::new(Barrier::new(2));
        let racers = (0..2)
            .map(|_| {
                let state = state.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let registry = Registry::new(&state);
                    barrier.wait();
                    registry
                        .begin(&binding(), NOW)
                        .map(|acquisition| acquisition.connection_reference().to_owned())
                })
            })
            .collect::<Vec<_>>();
        let results = racers
            .into_iter()
            .map(|racer| racer.join().unwrap())
            .collect::<Vec<_>>();
        let admitted = results.iter().filter(|result| result.is_ok()).count();
        let refusals = results
            .iter()
            .filter_map(|result| result.as_ref().err().copied())
            .collect::<Vec<_>>();
        assert_eq!(
            (admitted, refusals.as_slice()),
            (1, [Failure::Conflict].as_slice()),
            "round {round}: {results:?}"
        );
    }
}
