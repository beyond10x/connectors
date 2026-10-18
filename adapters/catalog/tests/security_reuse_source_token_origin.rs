//! Security review round 2 of story:invoke-definition-reuse-credential: a
//! reused credential is sent only where the source connection sends it.
//!
//! The round-1 fix (`definitions::receives_reuse`) lets a definition instance
//! receive a copy when its own `auth` names no URL off its API origin. It looks
//! at the target alone. A source whose client secret is exchanged at an
//! identity provider on another origin than its API, copied for a definition
//! whose token endpoint is on the API origin, passes it: the copy's validation
//! (`Local::validate`, exchange before the identity probe) posts the client
//! secret to the API host, which the source never sends it to. These cases
//! compose the real gates a reuse passes: the target rule, then the two binding
//! comparisons of `registry/reuse.rs` `reusable` on the bootstraps the catalog
//! provider prints. Fixtures only; nothing is sent.
#![cfg(unix)]

use connectors_host::local::{
    config::{Adapter, Config, Executable, Paths, Permissions},
    definitions, filesystem,
    runtime::{Bootstrap, PrivateProtocol},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

const API: &str = "https://api.example.com";
const IDP_TOKEN: &str = "https://idp.example.org/oauth/tokens";
const PROFILE: &str = "zendesk.oauth";

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn repository(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(path)
        .canonicalize()
        .unwrap()
}

/// A `connectors-catalog-local/2` configuration of the shipped `zendesk.oauth`
/// client-credentials profile at [`API`], its token endpoint `token_url`; only
/// the keys `operations invoke --definition` writes.
fn native(instance: &str, token_url: &str) -> Value {
    let declaration: Value =
        serde_json::from_slice(&fs::read(repository("providers/zendesk/connector.json")).unwrap())
            .unwrap();
    let mut auth = declaration["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["profile"] == PROFILE)
        .unwrap()
        .clone();
    auth["token_url"] = json!(token_url);
    json!({
        "format": "connectors-catalog-local/2",
        "instance": instance,
        "provider": "zendesk",
        "bundle_directory": repository("generated/bundles"),
        "api_base": API,
        "auth": auth,
        "operations_file": repository("providers/zendesk/operations.json"),
    })
}

fn bootstrap(path: &Path) -> Bootstrap {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bootstrap of {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn security_reuse_a_source_secret_is_never_sent_to_an_origin_its_connection_does_not_send_it_to() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    Config::initialize(&paths).unwrap();

    // The definition instance `zendesk.definition`, recorded as an invoke
    // records it, whose token endpoint is on its API origin.
    let digest = "c".repeat(64);
    let entry_directory = paths.state.join(definitions::DIRECTORY).join(&digest);
    let native_path = entry_directory.join(definitions::NATIVE);
    let entry = definitions::Entry {
        digest: digest.clone(),
        adapter: Adapter {
            credential_revision: Some("credential-1".into()),
            instance_id: definitions::alias("zendesk"),
            adapter_id: "catalog".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: Executable {
                path: paths.state.join("connectors-catalog-provider"),
                sha256: "a".repeat(64),
                args: vec![
                    "--local-config".into(),
                    native_path.to_str().unwrap().into(),
                ],
            },
            permissions: Permissions {
                profiles: BTreeSet::from([PROFILE.to_owned()]),
                operations: BTreeSet::from(["tickets.list".to_owned()]),
                custody: Default::default(),
            },
        },
    };
    definitions::record(&paths, "zendesk", &entry).unwrap();
    filesystem::directory(&entry_directory, true, true).unwrap();
    let on_api = format!("{API}/oauth/tokens");
    private(
        &native_path,
        &serde_json::to_vec(&native("zendesk.definition", &on_api)).unwrap(),
    );
    let config = Config::read_at(&paths).unwrap();
    let receives = definitions::receives_reuse(&paths, &config, "zendesk.definition").unwrap();
    // Round-2 fix: a profile naming any token endpoint receives no reuse, so
    // the pair below is refused by the target rule itself.
    assert!(
        !receives,
        "a token endpoint, even on the API origin, receives no reuse"
    );

    // The configured `zendesk/main`, as an operator pins it: client secret
    // exchanged at the identity provider, never at the API.
    let main_path = root.path().join("zendesk-main.json");
    private(
        &main_path,
        &serde_json::to_vec(&native("zendesk.main", IDP_TOKEN)).unwrap(),
    );
    let source = bootstrap(&main_path).binding(PROFILE).unwrap();
    let target = bootstrap(&native_path).binding(PROFILE).unwrap();

    let offered = receives
        && source.provider_authority == target.provider_authority
        && source.profile.same_authentication(&target.profile);
    assert!(
        !offered,
        "zendesk/main's client secret (exchanged only at {IDP_TOKEN}) is offered to, and on \
         reuse validated by, zendesk.definition, which exchanges it at {on_api}: the target \
         rule checks the definition's URLs against its own API origin, never against where \
         the source sends the credential"
    );
}
