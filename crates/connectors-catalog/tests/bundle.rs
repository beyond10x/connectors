use connectors_catalog::bundle::{Bundle, INDEX_FILE, load, read_index, write};
use connectors_catalog::{ingest, inventory::extract};
use connectors_core::ErrorCode;
use serde_json::json;

fn source_bytes() -> Vec<u8> {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "2"},
        "paths": {"/things": {"get": {"operationId": "listThings", "responses": {"200": {}}}}}
    })
    .to_string()
    .into_bytes()
}

fn bundle(provider: &str) -> Bundle {
    let bytes = source_bytes();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    Bundle {
        provider: provider.to_owned(),
        source: ingest("fixture.json", &bytes).expect("3.1 is supported"),
        inventory: extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

#[test]
fn a_written_bundle_loads_back_unchanged() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let original = bundle("fixture");
    let entry = write(directory.path(), &original, false).expect("first write");
    assert_eq!(entry.operations, 1);
    assert_eq!(entry.unsupported, 0);
    assert_eq!(entry.source_sha256, original.source.source_sha256);
    let loaded = load(directory.path(), "fixture").expect("load");
    assert_eq!(loaded, original);
}

#[test]
fn one_altered_byte_makes_the_load_refuse() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let entry = write(directory.path(), &bundle("fixture"), false).expect("first write");
    let path = directory.path().join(&entry.file_name);
    let mut bytes = std::fs::read(&path).expect("read bundle");
    let last = bytes.len() - 2;
    bytes[last] = if bytes[last] == b' ' { b'\t' } else { b' ' };
    std::fs::write(&path, bytes).expect("rewrite bundle");
    let error = load(directory.path(), "fixture").expect_err("must refuse");
    assert!(
        error.message.contains("digest the index recorded"),
        "unexpected message: {}",
        error.message
    );
}

#[test]
fn an_unknown_provider_and_a_missing_file_refuse_differently() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let entry = write(directory.path(), &bundle("fixture"), false).expect("first write");
    let unknown = load(directory.path(), "absent").expect_err("must refuse");
    assert!(unknown.message.contains("no such provider"));
    std::fs::remove_file(directory.path().join(&entry.file_name)).expect("remove bundle");
    let missing = load(directory.path(), "fixture").expect_err("must refuse");
    assert!(missing.message.contains("indexed bundle file is absent"));
}

#[test]
fn replacing_an_indexed_provider_needs_asking() {
    let directory = tempfile::tempdir().expect("temporary directory");
    write(directory.path(), &bundle("fixture"), false).expect("first write");
    let refused = write(directory.path(), &bundle("fixture"), false).expect_err("must refuse");
    assert!(refused.message.contains("already indexed"));
    write(directory.path(), &bundle("fixture"), true).expect("explicit replacement");
    assert_eq!(
        read_index(directory.path()).expect("index").entries.len(),
        1
    );
}

#[test]
fn the_index_lists_providers_in_a_stable_order() {
    let directory = tempfile::tempdir().expect("temporary directory");
    for provider in ["gitlab", "acme", "fixture"] {
        write(directory.path(), &bundle(provider), false).expect("write");
    }
    let index = read_index(directory.path()).expect("index");
    assert_eq!(index.providers(), vec!["acme", "fixture", "gitlab"]);
}

#[test]
fn an_empty_directory_reads_as_an_empty_index_rather_than_an_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    assert!(
        read_index(directory.path())
            .expect("index")
            .entries
            .is_empty()
    );
}

/// Rewrite the index `write` left in `directory` with `edit` applied to its
/// rows, the way a hand-edited build tree would.
fn edit_index(directory: &std::path::Path, edit: impl FnOnce(&mut Vec<serde_json::Value>)) {
    let path = directory.join(INDEX_FILE);
    let mut index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("index reads")).expect("index parses");
    edit(
        index["entries"]
            .as_array_mut()
            .expect("the index has entries"),
    );
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&index).expect("index serialises"),
    )
    .expect("index rewrites");
}

fn assert_refused_as_invalid(directory: &std::path::Path) {
    let refused = read_index(directory).expect_err("read_index must refuse");
    assert_eq!(refused.code, ErrorCode::InvalidInput, "{}", refused.message);
    let refused = load(directory, "fixture").expect_err("load must refuse");
    assert_eq!(refused.code, ErrorCode::InvalidInput, "{}", refused.message);
}

#[test]
fn an_index_naming_one_provider_twice_is_refused() {
    let directory = tempfile::tempdir().expect("temporary directory");
    write(directory.path(), &bundle("fixture"), false).expect("first write");
    edit_index(directory.path(), |entries| {
        let row = entries[0].clone();
        entries.push(row);
    });
    assert_refused_as_invalid(directory.path());
}

/// A bundle with the right digest placed outside the directory: the index row
/// that points at it must be refused, not followed.
fn bundle_outside(directory: &std::path::Path, outside: &std::path::Path) -> std::path::PathBuf {
    let entry = write(directory, &bundle("fixture"), false).expect("first write");
    let target = outside.join(&entry.file_name);
    std::fs::copy(directory.join(&entry.file_name), &target).expect("copy bundle outside");
    std::fs::remove_file(directory.join(&entry.file_name)).expect("remove the inside copy");
    target
}

#[test]
fn an_index_naming_an_absolute_file_is_refused() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    let target = bundle_outside(&directory, &outside);
    let absolute = target.to_str().expect("utf-8 path").to_owned();
    edit_index(&directory, |entries| {
        entries[0]["file_name"] = absolute.into();
    });
    assert_refused_as_invalid(&directory);
}

#[test]
fn an_index_naming_a_file_above_the_directory_is_refused() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    bundle_outside(&directory, &outside);
    edit_index(&directory, |entries| {
        entries[0]["file_name"] = "../elsewhere/fixture.bundle.json".into();
    });
    assert_refused_as_invalid(&directory);
}

#[test]
fn an_index_naming_another_providers_file_is_refused() {
    let directory = tempfile::tempdir().expect("temporary directory");
    write(directory.path(), &bundle("fixture"), false).expect("first write");
    write(directory.path(), &bundle("other"), false).expect("second write");
    edit_index(directory.path(), |entries| {
        for entry in entries.iter_mut() {
            if entry["provider"] == "fixture" {
                entry["file_name"] = "other.bundle.json".into();
            }
        }
    });
    assert_refused_as_invalid(directory.path());
}

#[test]
fn an_index_naming_an_invalid_provider_is_refused() {
    let root = tempfile::tempdir().expect("temporary directory");
    let directory = root.path().join("bundles");
    let outside = root.path().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("outside directory");
    bundle_outside(&directory, &outside);
    // The provider is chosen so `<provider>.bundle.json` is exactly the escaping
    // name: a check on the file name alone would pass it.
    edit_index(&directory, |entries| {
        entries[0]["provider"] = "../elsewhere/fixture".into();
        entries[0]["file_name"] = "../elsewhere/fixture.bundle.json".into();
    });
    let refused = read_index(&directory).expect_err("read_index must refuse");
    assert_eq!(refused.code, ErrorCode::InvalidInput, "{}", refused.message);
}
