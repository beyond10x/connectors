//! Security review of story:connection-binding-excludes-operations
//! (architecture-decision-record:provider-definitions-and-runtimes D12): every
//! input that decides where a stored credential travels or how it
//! authenticates moves the printed `credential_revision`, and an
//! operation-side change alone does not. Fixtures only: the provider prints
//! its bootstrap from local files and sends nothing.
#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

const S1: [&str; 2] = ["project.get", "issues.list"];
const S2: [&str; 3] = ["project.get", "issues.list", "project.languages"];
const FILES: &str = "https://files.fixture.invalid";

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

fn root_ca() -> Vec<u8> {
    rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .unwrap()
        .cert
        .pem()
        .into_bytes()
}

/// The shipped GitLab operations narrowed to `selection`, written to `path`.
fn selection_file(path: &Path, selection: &[&str]) {
    let shipped: Value =
        serde_json::from_slice(&fs::read(repository("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    let mut narrowed = shipped.clone();
    narrowed["operations"] = json!(
        shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|operation| selection.contains(&operation["id"].as_str().unwrap_or("")))
            .cloned()
            .collect::<Vec<_>>()
    );
    private(path, &serde_json::to_vec(&narrowed).unwrap());
}

/// A GitLab catalog configuration under selection S1 plus one download read
/// reaching `FILES`, which the connection admits with its credential.
fn base(directory: &Path) -> Value {
    let ca = directory.join("ca.pem");
    private(&ca, &root_ca());
    let operations = directory.join("operations.json");
    selection_file(&operations, &S1);
    json!({
        "format": "connectors-catalog-local/2",
        "instance": "fixture-gitlab",
        "provider": "gitlab",
        "bundle_directory": repository("generated/bundles"),
        "api_base": "https://localhost:8443/api/v4",
        "ca_file": ca,
        "hosts": [{"origin": FILES, "credential": true}],
        "auth": {
            "profile": "gitlab.pat",
            "header": "PRIVATE-TOKEN",
            "bearer": false,
            "label": "GitLab personal access token",
            "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
            "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
            "minimum_scopes": ["api"]
        },
        "operations": [{
            "id": "fixture.download",
            "effect": "read",
            "response": "binary",
            "binary": {"max_bytes": 1024, "hosts": [FILES]},
            "download": {"path_prefix": "/files-pri/"}
        }],
        "operations_file": operations,
    })
}

/// The configuration revision and credential-side digest the provider prints
/// for `document`.
fn printed(directory: &Path, document: &Value) -> (String, String) {
    let config = directory.join("catalog.json");
    private(&config, &serde_json::to_vec(document).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bootstrap inspection failed for {document}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    (
        bootstrap["configuration_revision"]
            .as_str()
            .unwrap()
            .to_owned(),
        bootstrap["credential_revision"]
            .as_str()
            .expect("the catalog provider prints a credential-side digest")
            .to_owned(),
    )
}

/// Invariant 1: each credential-routing input, changed alone, moves the
/// credential-side digest (and the configuration revision); a selection-only
/// change moves the configuration revision alone.
#[test]
fn security_every_credential_routing_input_moves_the_credential_revision() {
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path();
    let original = base(directory);
    let (revision, credential) = printed(directory, &original);

    type Edit = fn(&mut Value, &Path);
    let routing: [(&str, Edit); 10] = [
        ("admitted host stops carrying the credential", |d, _| {
            d["hosts"][0]["credential"] = json!(false)
        }),
        ("admitted host origin", |d, _| {
            let other = "https://other-files.fixture.invalid";
            d["hosts"][0]["origin"] = json!(other);
            d["operations"][0]["binary"]["hosts"] = json!([other]);
        }),
        ("api base host", |d, _| {
            d["api_base"] = json!("https://127.0.0.1:8443/api/v4")
        }),
        ("api base port", |d, _| {
            d["api_base"] = json!("https://localhost:9443/api/v4")
        }),
        ("credential header and bearer placement", |d, _| {
            d["auth"]["header"] = json!("Authorization");
            d["auth"]["bearer"] = json!(true);
        }),
        ("credential prefix", |d, _| {
            d["auth"]["prefix"] = json!("Token")
        }),
        ("identity probe path", |d, _| {
            d["auth"]["identity"]["path"] = json!("users/self")
        }),
        ("granted-scope probe", |d, _| {
            d["auth"]["scopes"]["path"] = json!("personal_access_tokens/other")
        }),
        ("minimum scopes", |d, _| {
            d["auth"]["minimum_scopes"] = json!(["api", "read_user"])
        }),
        ("CA file bytes at the same path", |_, directory| {
            private(&directory.join("ca.pem"), &root_ca())
        }),
    ];
    let mut unmoved = Vec::new();
    for (name, edit) in routing {
        let ca = fs::read(directory.join("ca.pem")).unwrap();
        let mut document = original.clone();
        edit(&mut document, directory);
        let (changed_revision, changed_credential) = printed(directory, &document);
        if changed_credential == credential || changed_revision == revision {
            unmoved.push(name);
        }
        private(&directory.join("ca.pem"), &ca);
    }
    assert!(
        unmoved.is_empty(),
        "credential-routing inputs that leave the credential-side digest: {unmoved:?}"
    );

    selection_file(&directory.join("operations.json"), &S2);
    let (selected_revision, selected_credential) = printed(directory, &original);
    assert_ne!(selected_revision, revision);
    assert_eq!(selected_credential, credential);
}
