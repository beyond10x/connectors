//! Second adversary pass on the engine lane of wave 20260930c. It drives the
//! rewritten "Array and required query parameters" section of
//! `docs/local-catalog-provider.md` against the host that binds an instance to
//! its configuration revision.
use connectors_catalog::bundle;
use connectors_host::local::{
    filesystem,
    metadata::Metadata,
    registry::{Failure, Registry},
    runtime::Bootstrap,
};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

const NOW: u64 = 1_788_998_400_000;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

/// The GitLab guide's site-form configuration, the first JSON block of the
/// guide for `gitlab`, loaded without its `ca_file` and with the shipped
/// selection set.
fn guide_config(bundles: &Path) -> Value {
    let guide = fs::read_to_string(root().join("../../docs/local-catalog-provider.md")).unwrap();
    let mut config = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .filter_map(|body| serde_json::from_str::<Value>(body).ok())
        .find(|config| config["provider"] == "gitlab")
        .expect("the guide documents a gitlab configuration");
    config["bundle_directory"] = json!(bundles);
    config["operations_file"] = json!(
        root()
            .join("providers/gitlab/operations.json")
            .canonicalize()
            .unwrap()
    );
    config.as_object_mut().unwrap().remove("ca_file");
    config
}

/// The GitLab bundle as base `81b509745` committed it, rebuilt from the current
/// one exactly as pass 1 did: no parameter repeated, a repeated parameter's
/// element type dropped, and no gaps. Its SHA-256 is checked against the base
/// index, so this is the base bundle and not an approximation of it.
fn base_gitlab(directory: &Path) {
    let mut gitlab = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    for operation in &mut gitlab.inventory.operations {
        for parameter in &mut operation.parameters {
            if parameter.repeated {
                parameter.repeated = false;
                parameter.value_type = None;
            }
        }
    }
    gitlab.inventory.unsupported.clear();
    let entry = bundle::write(directory, &gitlab, false).unwrap();
    // `git show 81b509745:adapters/catalog/generated/bundles/index.json`.
    assert_eq!(
        entry.bundle_sha256, "da458f3b69bf4634579d61482fa020733ef888c5d7de63cbb0e097b3685bd127",
        "the reconstruction is not the base bundle"
    );
}

/// `--print-local-bootstrap` of the guide's configuration over `bundles`.
fn bootstrap(bundles: &Path, scratch: &Path) -> Bootstrap {
    private_dir(scratch);
    let private_directory = scratch.join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(&guide_config(bundles)).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// `docs/local-catalog-provider.md`, "Array and required query parameters",
/// tells an operator what the rebuilt bundles require of an existing GitLab,
/// Jira or Confluence instance.
///
/// The host binds an instance id to the configuration revision it was first
/// connected under (`registry/lifecycle.rs` `register`), and the connection
/// binding is the bootstrap's (`runtime.rs` `Bootstrap::binding`). So an
/// instance connected before the rebuild, following the guide's own
/// configuration, is refused when it connects again after it under the same
/// instance id — and the adapter entry's `configuration_revision` must first be
/// copied from the bootstrap again, or the adapter does not start
/// (`runtime/process.rs` readiness check). Correction round 2 made this the
/// documented upgrade path: the guide says so, and this case holds today's
/// refusal as what the guide describes. Configuration upgrades are
/// unimplemented (`docs/local-connection-registry.md:121`).
#[test]
fn adversary2_an_instance_connected_before_the_rebuild_is_refused_under_the_same_id_after_it() {
    let scratch = tempfile::tempdir().unwrap();
    let base: PathBuf = scratch.path().join("base-bundles");
    base_gitlab(&base);
    let before = bootstrap(&base, &scratch.path().join("before"));
    let after = bootstrap(
        &root().join("generated/bundles").canonicalize().unwrap(),
        &scratch.path().join("after"),
    );
    assert_eq!(before.instance, after.instance);
    assert_ne!(
        before.configuration_revision, after.configuration_revision,
        "the rebuild moves the configuration revision the host binds"
    );

    let state = scratch.path().join("state");
    private_dir(&state);
    drop(Metadata::initialize(&state).unwrap());
    let registry = Registry::new(&state);
    let connected = registry
        .begin(&before.binding("gitlab.pat").unwrap(), NOW)
        .unwrap();
    registry.consume(connected, NOW).unwrap();

    // Today's behaviour, decided in correction round 2: the registry keeps an
    // instance id bound to the revision it first connected under, so the same
    // id is refused under the rebuilt binding. The guide tells operators to
    // connect again under a new instance id or fresh state.
    let again = registry.begin(&after.binding("gitlab.pat").unwrap(), NOW + 1);
    assert_eq!(
        again.as_ref().err(),
        Some(&Failure::Conflict),
        "instance `{}` connected under configuration revision {} and reconnecting under {} \
         after the bundle rebuild: docs/local-catalog-provider.md (\"Array and required \
         query parameters\") says this is refused and the instance connects again under a \
         new instance id or fresh state, because configuration upgrades are unimplemented \
         (docs/local-connection-registry.md:121). If the registry now accepts it, that \
         paragraph is wrong",
        after.instance,
        before.configuration_revision,
        after.configuration_revision,
    );
}
