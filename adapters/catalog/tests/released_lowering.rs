//! story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition:
//! the released connectors-definitions 0.5.2 `connectors/github.yaml` lowers to the same
//! bytes every time, and one lowering takes about 1.5 s. So the lowering is committed, in
//! `crates/connectors-catalog/tests/fixtures/released-github-lowering/`, and a test that
//! needs the engine of the unchanged released definition builds it from there with
//! [`engine`]. A test that lowers a changed definition keeps lowering.
//!
//! The directory holds what `definition::lower` wrote into its `out` (`index.json`,
//! `github.bundle.json`, `github/operations.json`, `github/connector.json`), what it
//! returned (`lowered.json`: the provider, and the paths from the directory) and
//! `inputs.json`: one sha256 per input file and the digest over them.
//!
//! The rule for every later change: who changes an input of the digest regenerates the
//! committed lowering in the same change. An input is any file under
//! `crates/connectors-catalog/tests/fixtures/definitions-0.5.2/` and any `.rs` file under
//! `crates/connectors-catalog/src/`. The change runs
//!
//! ```text
//! CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering
//! ```
//!
//! and commits the directory with the new `inputs.json`; when the lowering's bytes did not
//! change, only `inputs.json` changes. `the_committed_lowering_is_current` enforces the
//! rule: it is red in the gate until the change regenerates. `Cargo.lock` is not an input:
//! a dependency that changes the bytes is caught by the lane test
//! `slow_lane_the_committed_lowering_equals_a_fresh_lowering`, which lowers again and
//! compares every byte.
#![cfg(unix)]
use connectors_catalog::{bundle, definition};
use connectors_catalog_provider::{Engine, Selection};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// The committed lowering, from the repository root.
const DIRECTORY: &str = "crates/connectors-catalog/tests/fixtures/released-github-lowering";
/// The released definitions tree the lowering is made from, from the repository root.
const DEFINITIONS: &str = "crates/connectors-catalog/tests/fixtures/definitions-0.5.2";
/// One input root of the digest: a directory under the base, and the extension its input
/// files have (`None`: every file under it is an input).
type Root<'a> = (&'a str, Option<&'a str>);
/// The digest's input roots, from the repository root.
const ROOTS: [Root<'static>; 2] = [
    (DEFINITIONS, None),
    ("crates/connectors-catalog/src", Some("rs")),
];
/// The record of the inputs: one sha256 per input file, and the digest over them.
const INPUTS_FILE: &str = "inputs.json";
/// What `definition::lower` returned: the provider, and each path from the directory.
const LOWERED_FILE: &str = "lowered.json";
const FORMAT: &str = "connectors-released-lowering/1";
/// Set, the lane test writes the committed directory in place of comparing with it.
const WRITE: &str = "CONNECTORS_WRITE_RELEASED_LOWERING";
/// The command that regenerates the committed lowering.
const REGENERATE: &str = "CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed() -> PathBuf {
    repository().join(DIRECTORY)
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}; the committed lowering is made with `{REGENERATE}`",
            path.display()
        )
    });
    serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

/// The engine the released definition lowers to, built from the committed lowering:
/// nothing is lowered here.
pub(crate) fn engine() -> Engine {
    let directory = committed();
    let lowered = read_json(&directory.join(LOWERED_FILE));
    let provider = lowered["provider"]
        .as_str()
        .expect("lowered.json names the provider");
    let operations = lowered["operations"]
        .as_str()
        .expect("lowered.json names the selection file");
    let selections: Vec<Selection> =
        serde_json::from_value(read_json(&directory.join(operations))["operations"].clone())
            .expect("the lowered selection reads as the engine's selections");
    let bundle = bundle::load(&directory, provider)
        .unwrap_or_else(|error| panic!("the committed bundle loads: {}", error.message));
    match Engine::new(&bundle, "", &selections) {
        Ok(engine) => engine,
        Err(error) => panic!("the lowered selection loads: {}", error.message),
    }
}

/// Every file under `directory`, at any depth, sorted: the order is not the file system's.
fn files_under(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).unwrap_or_else(|error| panic!("{}: {error}", next.display()));
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// `path` from `base`, as text: no part of the record or of a message is an absolute path.
fn relative(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or_else(|_| panic!("{} is not under {}", path.display(), base.display()))
        .to_str()
        .expect("a UTF-8 path")
        .to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The record of the digest's inputs: every input file under `roots`, each a directory under
/// `base`, with its sha256, sorted by path, and the digest over those lines. Paths are from
/// `base`, so the record is the same wherever the tree is checked out.
fn inputs(base: &Path, roots: &[Root<'_>]) -> Value {
    let mut files = Vec::new();
    for (root, extension) in roots {
        for path in files_under(&base.join(root)) {
            let is_input = extension
                .is_none_or(|extension| path.extension().is_some_and(|found| found == extension));
            if is_input {
                files.push((relative(base, &path), sha256(&fs::read(&path).unwrap())));
            }
        }
    }
    files.sort();
    let summary: String = files
        .iter()
        .map(|(path, digest)| format!("file {digest} {path}\n"))
        .collect();
    // Keys in alphabetical order, here and in every object written to the directory: the
    // bytes are the same whether or not serde_json keeps insertion order in this build.
    json!({
        "digest": sha256(summary.as_bytes()),
        "files": files
            .iter()
            .map(|(path, digest)| json!({"path": path, "sha256": digest}))
            .collect::<Vec<_>>(),
        "format": FORMAT,
        "regenerate": REGENERATE,
    })
}

/// The fast check: the recorded digest is the current one. Its failure says that the
/// committed lowering is stale, names the command that regenerates it and lists the input
/// files that changed.
fn check(recorded: &Value, current: &Value) -> Result<(), String> {
    if recorded["digest"].is_string() && recorded["digest"] == current["digest"] {
        return Ok(());
    }
    let files = |record: &Value| -> BTreeMap<String, String> {
        record["files"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|file| {
                let text = |key: &str| file[key].as_str().unwrap_or_default().to_owned();
                (text("path"), text("sha256"))
            })
            .collect()
    };
    let (recorded, current) = (files(recorded), files(current));
    let paths: BTreeSet<&String> = recorded.keys().chain(current.keys()).collect();
    let mut changed = String::new();
    for path in paths {
        let what = match (recorded.get(path), current.get(path)) {
            (Some(was), Some(is)) if was == is => continue,
            (Some(_), Some(_)) => "changed",
            (None, _) => "added",
            (_, None) => "removed",
        };
        changed.push_str(&format!("\n  {what}: {path}"));
    }
    Err(format!(
        "the committed lowering of the released definition is stale: an input changed after \
         {DIRECTORY} was made. Regenerate it and commit the directory in the same change:\n\
         {REGENERATE}\ninput files that changed:{changed}"
    ))
}

/// Compare two directories byte for byte: the error names the first file, in path order, that
/// one of them lacks or that holds other bytes.
fn compare(committed: &Path, fresh: &Path) -> Result<(), String> {
    let names = |directory: &Path| -> BTreeSet<String> {
        files_under(directory)
            .iter()
            .map(|path| relative(directory, path))
            .collect()
    };
    let (in_committed, in_fresh) = (names(committed), names(fresh));
    for name in in_committed.union(&in_fresh) {
        if !in_fresh.contains(name) {
            return Err(format!(
                "`{name}` is in the first directory and not in the second"
            ));
        }
        if !in_committed.contains(name) {
            return Err(format!(
                "`{name}` is in the second directory and not in the first"
            ));
        }
        let one = fs::read(committed.join(name)).unwrap();
        let other = fs::read(fresh.join(name)).unwrap();
        if one != other {
            let at = one
                .iter()
                .zip(&other)
                .position(|(a, b)| a != b)
                .unwrap_or(one.len().min(other.len()));
            return Err(format!(
                "`{name}` differs at byte {at} ({} bytes in the first directory, {} in the second)",
                one.len(),
                other.len()
            ));
        }
    }
    Ok(())
}

fn copy_directory(from: &Path, to: &Path) {
    for file in files_under(from) {
        let target = to.join(relative(from, &file));
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(&file, &target).unwrap_or_else(|error| panic!("{}: {error}", target.display()));
    }
}

/// Lower the released definition into `directory`, complete as the committed directory is:
/// what `definition::lower` writes, what it returns, and the record of the inputs.
fn lower_into(directory: &Path) {
    let scratch = tempfile::tempdir().unwrap();
    let released = repository().join(DEFINITIONS);
    let lowered = definition::lower(&definition::Request {
        definition: &released.join("connectors/github.yaml"),
        root: &released,
        out: directory,
        scratch: scratch.path(),
    })
    .expect("the released definition lowers");
    write_json(
        &directory.join(LOWERED_FILE),
        &json!({
            "bundle": relative(directory, &lowered.bundle),
            "connector": relative(directory, &lowered.connector),
            "operations": relative(directory, &lowered.operations),
            "provider": lowered.provider,
        }),
    );
    write_json(&directory.join(INPUTS_FILE), &inputs(&repository(), &ROOTS));
}

/// The committed lowering is made from the inputs as they are now: the digest recorded in
/// `inputs.json` is the current one. Nothing is lowered here.
#[test]
fn the_committed_lowering_is_current() {
    let recorded = read_json(&committed().join(INPUTS_FILE));
    if let Err(stale) = check(&recorded, &inputs(&repository(), &ROOTS)) {
        panic!("{stale}");
    }
}

/// One changed byte of one input file gives another digest, and the same bytes the same.
#[test]
fn a_changed_input_byte_changes_the_digest() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("one.yaml"), "one\n").unwrap();
    fs::write(temp.path().join("two.rs"), "two\n").unwrap();
    let roots: [Root<'_>; 1] = [("", None)];
    let first = inputs(temp.path(), &roots);
    assert_eq!(
        first["files"].as_array().map(Vec::len),
        Some(2),
        "both files are inputs: {first}"
    );
    assert_eq!(
        first["digest"].as_str().map(str::len),
        Some(64),
        "the digest is a sha256: {first}"
    );
    assert_eq!(
        first,
        inputs(temp.path(), &roots),
        "the same bytes give the same record"
    );

    fs::write(temp.path().join("two.rs"), "twp\n").unwrap();
    let second = inputs(temp.path(), &roots);
    assert_ne!(
        first["digest"], second["digest"],
        "one changed byte of two.rs changes the digest"
    );
    let stale = check(&first, &second).expect_err("the first record is stale after the change");
    assert!(
        stale.contains("changed: two.rs") && !stale.contains("one.yaml"),
        "the failure lists the changed file and no other: {stale}"
    );
}

/// A recorded digest that is not the current one fails the check, and the failure holds the
/// regeneration command character for character and lists the input that changed.
#[test]
fn a_stale_digest_prints_the_regeneration_command() {
    let current = inputs(&repository(), &ROOTS);
    check(&current, &current).expect("a record equal to the current one is current");
    let mut recorded = current.clone();
    recorded["digest"] = json!("0".repeat(64));
    recorded["files"][0]["sha256"] = json!("0".repeat(64));
    let path = recorded["files"][0]["path"].as_str().unwrap().to_owned();
    let stale = check(&recorded, &current).expect_err("another recorded digest is stale");
    assert!(
        stale.contains(
            "CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering"
        ),
        "the failure holds the regeneration command: {stale}"
    );
    assert!(
        stale.contains("stale") && stale.contains(&format!("changed: {path}")),
        "the failure says stale and lists the changed input `{path}`: {stale}"
    );
}

/// The comparison the lane test makes fails on one changed byte and names the file. Nothing is
/// lowered here: the second directory is a copy of the committed one.
#[test]
fn a_changed_byte_in_a_copy_of_the_fixture_fails_the_comparison() {
    let committed = committed();
    let temp = tempfile::tempdir().unwrap();
    let copy = temp.path().join("copy");
    copy_directory(&committed, &copy);
    compare(&committed, &copy).expect("an unchanged copy compares equal");

    let name = "github/operations.json";
    let mut bytes = fs::read(copy.join(name)).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    fs::write(copy.join(name), bytes).unwrap();
    let difference = compare(&committed, &copy).expect_err("one changed byte fails the comparison");
    assert!(
        difference.contains(name) && difference.contains(&format!("byte {middle}")),
        "the failure names the changed file and the byte: {difference}"
    );
}

/// Regeneration from the sources, outside the gate (lane `slow`): the released definition
/// lowers to the committed directory, byte for byte. With `CONNECTORS_WRITE_RELEASED_LOWERING`
/// set it writes the directory in place of comparing: that is how the lowering is regenerated.
#[test]
#[ignore = "lane slow"]
fn slow_lane_the_committed_lowering_equals_a_fresh_lowering() {
    let temp = tempfile::tempdir().unwrap();
    let fresh = temp.path().join("fresh");
    lower_into(&fresh);
    let committed = committed();
    if std::env::var_os(WRITE).is_some() {
        if committed.exists() {
            fs::remove_dir_all(&committed).unwrap();
        }
        copy_directory(&fresh, &committed);
        return;
    }
    if let Err(difference) = compare(&committed, &fresh) {
        panic!(
            "{DIRECTORY} (the first directory) is not what the released definition lowers to \
             now (the second): {difference}\nregenerate it and commit the directory:\n{REGENERATE}"
        );
    }
}
