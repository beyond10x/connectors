//! Host start with `state` on a shared volume: a refused state directory is
//! named in the start refusal with a fixed reason, and no admission is loosened
//! (every ancestor of `state` is still checked).
//!
//! Unix only: its fixtures use Unix mode bits.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{Descriptor, Error, ErrorCode, Operation, Result, WIRE_VERSION};
use connectors_host::{
    credentials::CredentialRef,
    server::{ServiceConfig, ServiceState, serve},
};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const PREFIX: &str = "service state cannot be opened for this instance";

/// The refusal for `state` itself states the whole target, whichever check
/// failed first.
fn state_target(state: &Path) -> String {
    format!(
        "{PREFIX}: {state:?}, the state directory, must be owned by the host user \
         with mode 0700, no group or other bits"
    )
}

struct Fixture;
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: "leaf".into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![Operation {
                id: "read".into(),
                description: "read fixture".into(),
                contract: "operations/v1alpha1".into(),
                profile: "read".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
            }],
        }
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        Ok(input)
    }
}

fn mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// A private temporary root holding the service credential.
fn root() -> (tempfile::TempDir, CredentialRef) {
    let root = tempfile::tempdir().unwrap();
    mode(root.path(), 0o700);
    let token = root.path().join("token");
    std::fs::write(&token, b"service-token").unwrap();
    mode(&token, 0o600);
    (root, CredentialRef::File { path: token })
}

/// Starts a host on `state` and returns its start refusal.
async fn refusal(credential: CredentialRef, state: PathBuf) -> Error {
    let config = ServiceConfig {
        instance: "leaf".into(),
        listen: "127.0.0.1:0".parse().unwrap(),
        service_credential: credential,
        state: Some(state),
    };
    tokio::time::timeout(Duration::from_secs(10), serve(config, Arc::new(Fixture)))
        .await
        .expect("host start is refused before it serves")
        .expect_err("host start is refused")
}

#[tokio::test]
async fn state_under_a_world_writable_volume_root_names_the_root() {
    let (root, credential) = root();
    let volume = root.path().join("volume");
    std::fs::create_dir(&volume).unwrap();
    mode(&volume, 0o777);
    let state = volume.join("state");

    let error = refusal(credential, state.clone()).await;

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        error.message,
        format!("{PREFIX}: {volume:?}, an ancestor of state, is writable by group or others")
    );
    assert!(!state.exists(), "nothing is created under a refused root");
}

#[tokio::test]
async fn a_private_state_does_not_admit_a_world_writable_root() {
    let (root, credential) = root();
    let volume = root.path().join("volume");
    let state = volume.join("state");
    std::fs::create_dir_all(&state).unwrap();
    mode(&state, 0o700);
    mode(&volume, 0o777);

    let error = refusal(credential, state).await;

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        error.message,
        format!("{PREFIX}: {volume:?}, an ancestor of state, is writable by group or others")
    );
}

#[tokio::test]
async fn a_state_that_is_not_private_is_named() {
    let (root, credential) = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    mode(&state, 0o750);

    let error = refusal(credential, state.clone()).await;

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, state_target(&state));
}

#[tokio::test]
async fn a_world_writable_state_is_still_refused() {
    let (root, credential) = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    mode(&state, 0o777);

    let error = refusal(credential, state.clone()).await;

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, state_target(&state));
}

#[tokio::test]
async fn other_start_refusals_keep_their_text() {
    let (root, credential) = root();
    let state = root.path().join("state");
    drop(ServiceState::open(&state, "leaf", "other", "rev-1").unwrap());

    let error = refusal(credential, state).await;

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, PREFIX);
}

// Adversary cases (wave D275, unit UC). Each drives the code from the text
// this unit wrote: docs/running-services.md § The state directory, or the
// story's Outcome ("learns ... which directory is refused and what to set
// instead").

/// docs/running-services.md § The state directory: a subdirectory of the
/// volume mounted as `state` is the host user's, mode 0700. That layout opens.
/// (Correction round 1, F1: the bullet was split, so this case pins the
/// documented 0700 layout; the adversary's 0755 is refused by
/// `a_state_that_is_not_private_is_named`'s rule.)
#[test]
fn adversary_a_subpath_mounted_as_state_as_documented_opens() {
    let (root, _credential) = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    mode(&state, 0o700);

    let opened = ServiceState::open(&state, "leaf", "fixture", "rev-1");

    assert!(
        opened.is_ok(),
        "the documented subPath-as-state layout (host user, 0755) is refused: {:?}",
        opened.err()
    );
}

/// The Outcome: the operator learns what to set instead. A `0777` state
/// directory is told only "writable by group or others"; removing those bits
/// (0755) is refused again as not private. The refusal for `state` itself
/// has to say what it must be (the host user's, mode 0700).
#[tokio::test]
async fn adversary_a_world_writable_state_is_told_what_to_set() {
    let (root, credential) = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    mode(&state, 0o777);

    let error = refusal(credential, state.clone()).await;

    assert!(
        error.message.contains("0700"),
        "the refusal for a 0777 state does not say what to set: {}",
        error.message
    );
}

/// docs/running-services.md § The state directory: a directory the host
/// cannot open (here a host-owned root without search permission, 0600)
/// refuses start with the closed text and is not named. (Correction round 1,
/// F3: the adversary's case asked for a name; the coordinator decided the
/// documented behaviour is the closed text, which this case now pins.)
#[tokio::test]
async fn adversary_a_stricter_volume_root_that_refuses_is_named() {
    let (root, credential) = root();
    let volume = root.path().join("volume");
    let state = volume.join("state");
    std::fs::create_dir_all(&state).unwrap();
    mode(&state, 0o700);
    mode(&volume, 0o600);

    let error = refusal(credential, state).await;
    mode(&volume, 0o700);

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        error.message, PREFIX,
        "a directory the host cannot open refuses with the closed text"
    );
}
