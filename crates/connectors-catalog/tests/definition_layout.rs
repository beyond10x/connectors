//! A definition none of the shipped selections came from lowers to a set of
//! files that agree with each other: the selection the connector declaration
//! names is written beside it under that name, and the bundle the declaration
//! runs on is indexed in the bundle directory. The output directory holds the
//! bundle and its index, and `<catalog>/` the declaration and its selection,
//! as `adapters/catalog/` holds `generated/bundles/` and `providers/<catalog>/`.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog::definition::{self, Request};
use serde_json::{Value, json};
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// A two-read API written for this test; no shipped selection names it.
fn document() -> Vec<u8> {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Widgets", "version": "1", "license": {"name": "MIT"}},
        "paths": {
            "/me": {
                "get": {
                    "operationId": "getMe",
                    "responses": {"200": {"content": {"application/json": {}}}}
                }
            },
            "/things": {
                "get": {
                    "operationId": "listThings",
                    "responses": {"200": {"content": {"application/json": {}}}}
                }
            }
        }
    })
    .to_string()
    .into_bytes()
}

/// A definitions root holding the document gzipped and `connectors/<name>.yaml`,
/// the definition with `head` (such as a `catalog` line) after its schema.
fn root(name: &str, head: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(root.join("upstream/widgets")).unwrap();
    std::fs::create_dir_all(root.join("connectors")).unwrap();
    let source = document();
    std::fs::write(root.join("upstream/widgets/openapi.json.gz"), gzip(&source)).unwrap();
    let text = format!(
        "schema: urn:beyond10x:connectors-definition:1\n{head}source:\n  openapi: upstream/widgets/openapi.json.gz\n  sha256: {}\nbase_url: https://api.widgets.example\nauth:\n  widgets.token:\n    scheme: bearer\n    label: Widgets API token\n    identity: {{path: me, kind: widgets.user, subject: /id}}\noperations:\n  me.get: {{op: getMe, description: Read the token's user}}\n  things.list: {{op: listThings, description: List things}}\n",
        sha256(&source)
    );
    let file = root.join("connectors").join(format!("{name}.yaml"));
    std::fs::write(&file, text).unwrap();
    (temp, file)
}

/// Every file under `directory`, relative and sorted.
fn listing(directory: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(directory).unwrap();
                names.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    names
}

/// Lower `connectors/<name>.yaml` and hold every file the declaration names
/// against what was written.
fn lowers_consistently(name: &str, head: &str, catalog: &str, files: &[&str]) {
    let (temp, file) = root(name, head);
    let out = temp.path().join("out");
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: &file,
        root: &temp.path().join("definitions"),
        out: &out,
        scratch: &scratch,
    })
    .unwrap_or_else(|refusal| panic!("`{name}` lowers: {refusal}"));

    let declaration: Value =
        serde_json::from_slice(&std::fs::read(&lowered.connector).unwrap()).unwrap();
    assert_eq!(declaration["connector_id"], name);
    assert_eq!(declaration["catalog_provider"], catalog);
    let named = declaration["selection"].as_str().unwrap();
    let beside = lowered.connector.parent().unwrap().join(named);
    assert!(
        beside.is_file(),
        "`{name}`: the declaration names selection `{named}`, which is not beside it; the output holds {:?}",
        listing(&out)
    );
    assert_eq!(
        beside, lowered.operations,
        "`{name}`: the selection written"
    );
    let selection: Value = serde_json::from_slice(&std::fs::read(&beside).unwrap()).unwrap();
    assert_eq!(selection["provider"], catalog);
    let ids: Vec<&str> = selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|operation| operation["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["me.get", "things.list"]);

    let index = bundle::read_index(&out).unwrap();
    let entry = index.find(catalog).expect("the bundle is indexed");
    assert_eq!(out.join(&entry.file_name), lowered.bundle);
    bundle::load(&out, catalog).expect("the index verifies");
    assert_eq!(listing(&out), files, "`{name}`: the files written");
}

#[test]
fn a_new_definition_lowers_to_a_declaration_beside_the_selection_it_names() {
    lowers_consistently(
        "widgets",
        "",
        "widgets",
        &[
            "index.json",
            "widgets.bundle.json",
            "widgets/connector.json",
            "widgets/operations.json",
        ],
    );
}

#[test]
fn a_second_connector_over_a_catalog_lowers_to_the_selection_its_declaration_names() {
    lowers_consistently(
        "widgets-trial",
        "catalog: widgets\n",
        "widgets",
        &[
            "index.json",
            "widgets.bundle.json",
            "widgets/connector.json",
            "widgets/trial-operations.json",
        ],
    );
}
