//! Every auth profile the committed connector definitions lower to
//! (`crates/connectors-catalog/tests/fixtures/definitions/`) is a profile the
//! catalog provider loads: a `connectors-catalog-local/2` configuration with
//! that `auth` block, the definition's `base_url` as `api_base`, the committed
//! bundle and the shipped selection prints its bootstrap. No network.
#![cfg(unix)]
use connectors_catalog::definition;
use connectors_host::local::filesystem;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}

fn definitions() -> PathBuf {
    root_path("../../crates/connectors-catalog/tests/fixtures/definitions")
}

/// A URL template with each `{variable}` given its default, or `example`.
fn filled(template: &str, definition: &definition::ProviderDefinition) -> String {
    let mut text = template.to_owned();
    for (name, field) in definition.config.iter().flatten() {
        let value = field.default.as_deref().unwrap_or("example");
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

fn print_bootstrap(config: &Value) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    fs::write(&path, serde_json::to_vec(config).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
}

#[test]
fn every_lowered_auth_profile_loads_in_the_catalog_provider() {
    let root_dir = definitions();
    let mut files: Vec<PathBuf> = fs::read_dir(root_dir.join("connectors"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "yaml"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 14, "one definition per shipped selection");
    let mut loaded = 0;
    for file in &files {
        let name = file.file_stem().unwrap().to_str().unwrap();
        let parsed = definition::parse(&fs::read(file).unwrap())
            .unwrap_or_else(|refusal| panic!("`{name}` parses: {refusal}"));
        let catalog = parsed.catalog.as_deref().unwrap_or(name);
        let selection = match name.strip_prefix(&format!("{catalog}-")) {
            Some(stem) => format!("{stem}-operations.json"),
            None => "operations.json".to_owned(),
        };
        let auth = definition::auth(file, &root_dir)
            .unwrap_or_else(|refusal| panic!("`{name}` lowers its auth: {refusal}"));
        assert_eq!(auth["provider"], catalog);
        for (id, block) in auth["profiles"].as_object().unwrap() {
            let mut block = block.clone();
            for url in ["token_url", "authorize_url"] {
                if let Some(template) = block[url].as_str() {
                    block[url] = json!(filled(template, &parsed));
                }
            }
            let config = json!({
                "format": "connectors-catalog-local/2",
                "instance": "definition-check",
                "provider": catalog,
                "bundle_directory": root_path("generated/bundles"),
                "api_base": filled(&parsed.base_url, &parsed),
                "auth": block,
                "operations_file": root_path(&format!("providers/{catalog}/{selection}")),
            });
            let output = print_bootstrap(&config);
            assert!(
                output.status.success(),
                "`{name}` profile `{id}` does not load: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            loaded += 1;
        }
    }
    assert_eq!(loaded, 18, "every profile of every definition is loaded");
}
