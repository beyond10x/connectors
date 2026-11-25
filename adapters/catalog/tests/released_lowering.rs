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
//! `inputs.json`: one sha256 per input file with the digest over them, and one sha256 per
//! lowered file (every file of the directory but `inputs.json`).
//!
//! The rule for every later change: who changes an input of the digest regenerates the
//! committed lowering in the same change. An input is any file under
//! `crates/connectors-catalog/tests/fixtures/definitions-0.5.2/` and any `.rs` file under
//! `crates/connectors-catalog/src/`; a file or directory whose name starts with `.` is not
//! one. The change runs
//!
//! ```text
//! CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering
//! ```
//!
//! and commits the directory with the new `inputs.json`; when the lowering's bytes did not
//! change, only `inputs.json` changes. `the_committed_lowering_is_current` enforces the
//! rule: it is red in the gate until the change regenerates. It also binds every lowered
//! file to the sha256 the record holds for it: it is red when a lowered file was edited,
//! removed or added and the record was not. A lowered file edited together with its row
//! passes it; what sees that is the lane test
//! `slow_lane_the_committed_lowering_equals_a_fresh_lowering`, which lowers again and
//! compares every byte. `Cargo.lock` is not an input: a dependency that changes the bytes is
//! caught by the lane test as well. Only the value `1` writes; any other value of the
//! variable is refused, and so is every value where `CI` is set.
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
/// The record: one sha256 per input file with the digest over them, and one per lowered file.
const INPUTS_FILE: &str = "inputs.json";
/// What `definition::lower` returned: the provider, and each path from the directory.
const LOWERED_FILE: &str = "lowered.json";
const FORMAT: &str = "connectors-released-lowering/2";
/// Set to `1`, the lane test writes the committed directory in place of comparing with it.
const WRITE: &str = "CONNECTORS_WRITE_RELEASED_LOWERING";
/// The command that regenerates the committed lowering.
const REGENERATE: &str = "CONNECTORS_WRITE_RELEASED_LOWERING=1 cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering";

/// Paths and their sha256, by path.
type Rows = BTreeMap<String, String>;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed() -> PathBuf {
    repository().join(DIRECTORY)
}

/// The way out every failed check names.
fn regenerate() -> String {
    format!("Regenerate it and commit the directory in the same change:\n{REGENERATE}")
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path)
        .unwrap_or_else(|error| panic!("{}: {error}. {}", path.display(), regenerate()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{}: {error}. {}", path.display(), regenerate()))
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
/// Without `hidden`, a file or directory whose name starts with `.` is left out, and so is
/// all that is under such a directory.
fn files_under(directory: &Path, hidden: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).unwrap_or_else(|error| panic!("{}: {error}", next.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|error| panic!("{}: {error}", next.display()));
            if !hidden && entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
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

/// `path` from `base`, as text: the record holds no absolute path.
fn relative(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or_else(|_| panic!("{} is not under {}", path.display(), base.display()))
        .to_str()
        .unwrap_or_else(|| panic!("{} is not UTF-8", path.display()))
        .to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Rows as the record holds them: `[{"path": …, "sha256": …}]`, by path.
fn rows_json(rows: &Rows) -> Value {
    rows.iter()
        .map(|(path, digest)| json!({"path": path, "sha256": digest}))
        .collect()
}

/// The rows of the list `list` of a record; a list that is none reads as no rows. A path that
/// two rows name is a failure, whatever their sha256: neither row is the one that counts.
fn rows(record: &Value, list: &str) -> Result<Rows, String> {
    let mut rows = Rows::new();
    for file in record[list].as_array().into_iter().flatten() {
        let text = |key: &str| file[key].as_str().unwrap_or_default().to_owned();
        let path = text("path");
        if rows.insert(path.clone(), text("sha256")).is_some() {
            return Err(format!(
                "the list `{list}` in the record of the committed lowering \
                 ({DIRECTORY}/{INPUTS_FILE}) names the path `{path}` twice. The record was not \
                 written by the regeneration command. {}",
                regenerate()
            ));
        }
    }
    Ok(rows)
}

/// The record of the digest's inputs: every input file under `roots`, each a directory under
/// `base`, with its sha256, sorted by path, and the digest over those lines. A name that
/// starts with `.` is not an input (an editor's lock, `.DS_Store`). Paths are from `base`, so
/// the record is the same wherever the tree is checked out.
fn inputs(base: &Path, roots: &[Root<'_>]) -> Value {
    let mut files = Rows::new();
    for (root, extension) in roots {
        for path in files_under(&base.join(root), false) {
            let is_input = extension
                .is_none_or(|extension| path.extension().is_some_and(|found| found == extension));
            if is_input {
                files.insert(relative(base, &path), sha256(&read(&path)));
            }
        }
    }
    let summary: String = files
        .iter()
        .map(|(path, digest)| format!("file {digest} {path}\n"))
        .collect();
    // Keys in alphabetical order, here and in every object written to the directory: the
    // bytes are the same whether or not serde_json keeps insertion order in this build.
    json!({
        "digest": sha256(summary.as_bytes()),
        "files": rows_json(&files),
        "format": FORMAT,
        "regenerate": REGENERATE,
    })
}

/// The lowered files of `directory` with their sha256: every file but the record itself.
fn lowered(directory: &Path) -> Rows {
    files_under(directory, true)
        .iter()
        .map(|path| relative(directory, path))
        .filter(|name| name != INPUTS_FILE)
        .map(|name| {
            let digest = sha256(&read(&directory.join(&name)));
            (name, digest)
        })
        .collect()
}

/// What `inputs.json` holds: the record of the inputs, and the lowered files made from them.
fn record(inputs: &Value, lowered: &Rows) -> Value {
    json!({
        "digest": inputs["digest"],
        "files": inputs["files"],
        "format": inputs["format"],
        "lowered": rows_json(lowered),
        "regenerate": inputs["regenerate"],
    })
}

/// One line per path the two sets of rows do not hold with the same sha256, under the word
/// for its case: in both, only in `current`, only in `recorded`.
fn differences(recorded: &Rows, current: &Rows, words: [&str; 3]) -> String {
    let [changed, only_current, only_recorded] = words;
    let paths: BTreeSet<&String> = recorded.keys().chain(current.keys()).collect();
    let mut lines = String::new();
    for path in paths {
        let word = match (recorded.get(path), current.get(path)) {
            (Some(was), Some(is)) if was == is => continue,
            (Some(_), Some(_)) => changed,
            (None, _) => only_current,
            (_, None) => only_recorded,
        };
        lines.push_str(&format!("\n  {word}: {path}"));
    }
    lines
}

/// The inputs half of the fast check: the record was made from the inputs as they are now.
/// Its failure says that the committed lowering is stale, names the command that regenerates
/// it and lists the input files that changed; a record of another format, a path that two
/// rows name and a digest that is not the one of the record's own rows are each said to be
/// that. No failure prints a value that makes the check pass when it is pasted into the
/// record's digest or rows.
fn check(recorded: &Value, current: &Value) -> Result<(), String> {
    if recorded["format"] != current["format"] {
        return Err(format!(
            "the record of the committed lowering ({DIRECTORY}/{INPUTS_FILE}) has the format {} \
             and this check reads {}. {}",
            recorded["format"],
            current["format"],
            regenerate()
        ));
    }
    let changed = differences(
        &rows(recorded, "files")?,
        &rows(current, "files")?,
        ["changed", "added", "removed"],
    );
    if !changed.is_empty() {
        return Err(format!(
            "the committed lowering of the released definition is stale: an input changed after \
             {DIRECTORY} was made. A file listed as added that is not to be committed (an \
             editor's backup, a `.orig`) is removed instead of regenerating. {}\n\
             input files that changed:{changed}",
            regenerate()
        ));
    }
    if !recorded["digest"].is_string() || recorded["digest"] != current["digest"] {
        return Err(format!(
            "the digest in the record of the committed lowering ({DIRECTORY}/{INPUTS_FILE}) is \
             not the digest of the input files it lists: every listed sha256 is the current one, \
             and the record holds {} as the digest over them, which it is not. The record was \
             not written by the regeneration command. {}",
            recorded["digest"],
            regenerate()
        ));
    }
    Ok(())
}

/// The fast check on `directory`: its record reads, was made from the inputs `current`, and
/// names every lowered file of the directory with the sha256 the file has. Each of these
/// failures names what differs and the command that regenerates. A file of the directory that
/// cannot be read (a link that points nowhere, which only a commit by hand puts there) panics
/// with its path and without the command. Nothing is lowered.
fn check_directory(directory: &Path, current: &Value) -> Result<(), String> {
    let path = directory.join(INPUTS_FILE);
    let text = fs::read(&path)
        .map_err(|error| format!("{}: {error}. {}", path.display(), regenerate()))?;
    let recorded: Value = serde_json::from_slice(&text).map_err(|error| {
        format!(
            "{} does not read as JSON: {error}. {}",
            path.display(),
            regenerate()
        )
    })?;
    check(&recorded, current)?;
    let differing = differences(
        &rows(&recorded, "lowered")?,
        &lowered(directory),
        ["changed", "not in the record", "missing"],
    );
    if !differing.is_empty() {
        return Err(format!(
            "the lowered files in {} are not the ones {INPUTS_FILE} records: a file of the \
             committed lowering was edited, removed or added after the directory was made. {}\n\
             lowered files that differ:{differing}",
            directory.display(),
            regenerate()
        ));
    }
    Ok(())
}

/// Compare two directories byte for byte: the error names the first file, in path order, that
/// one of them lacks or that holds other bytes.
fn compare(committed: &Path, fresh: &Path) -> Result<(), String> {
    let names = |directory: &Path| -> BTreeSet<String> {
        files_under(directory, true)
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
        let one = read(&committed.join(name));
        let other = read(&fresh.join(name));
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
    for file in files_under(from, true) {
        let target = to.join(relative(from, &file));
        let parent = target.parent().expect("a file has a directory");
        fs::create_dir_all(parent).unwrap_or_else(|error| panic!("{}: {error}", parent.display()));
        fs::copy(&file, &target).unwrap_or_else(|error| panic!("{}: {error}", target.display()));
    }
}

/// Lower the released definition into `directory`, complete as the committed directory is:
/// what `definition::lower` writes, what it returns, and the record.
fn lower_into(directory: &Path) {
    let scratch = tempfile::tempdir().unwrap();
    let released = repository().join(DEFINITIONS);
    let lowered_to = definition::lower(&definition::Request {
        definition: &released.join("connectors/github.yaml"),
        root: &released,
        out: directory,
        scratch: scratch.path(),
    })
    .expect("the released definition lowers");
    write_json(
        &directory.join(LOWERED_FILE),
        &json!({
            "bundle": relative(directory, &lowered_to.bundle),
            "connector": relative(directory, &lowered_to.connector),
            "operations": relative(directory, &lowered_to.operations),
            "provider": lowered_to.provider,
        }),
    );
    write_json(
        &directory.join(INPUTS_FILE),
        &record(&inputs(&repository(), &ROOTS), &lowered(directory)),
    );
}

/// What the lane test does with a fresh lowering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Compare it with the committed directory, byte for byte.
    Compare,
    /// Write it over the committed directory, and compare nothing.
    Write,
}

/// The lane test's mode, from the value of `CONNECTORS_WRITE_RELEASED_LOWERING` and the value
/// of `CI` (`None`: not set). Unset compares. Only `1` writes, and never where `CI` is set: a
/// lane that writes compares nothing and would pass on any lowering. Every other set value is
/// refused, so that an empty value or a `0` meant as "off" does not skip the comparison.
fn mode(write: Option<&str>, ci: Option<&str>) -> Result<Mode, String> {
    match (write, ci) {
        (None, _) => Ok(Mode::Compare),
        (Some(_), Some(_)) => Err(format!(
            "`{WRITE}` is set and `CI` is set: a lane that writes compares nothing. Unset \
             `{WRITE}`; the lowering is regenerated on a developer's machine and committed"
        )),
        (Some("1"), None) => Ok(Mode::Write),
        (Some(other), None) => Err(format!(
            "`{WRITE}` is set to `{other}`: only the value `1` writes the committed lowering. \
             Unset it to compare"
        )),
    }
}

/// A copy of the committed directory in a temporary directory: a test edits the copy.
fn copy_of_committed() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let copy = temp.path().join("copy");
    copy_directory(&committed(), &copy);
    (temp, copy)
}

/// The paths of the input rows of a record, in path order.
fn input_paths(record: &Value) -> Vec<String> {
    match rows(record, "files") {
        Ok(rows) => rows.into_keys().collect(),
        Err(failure) => panic!("{failure}"),
    }
}

/// Change one byte in the middle of the file; the byte's offset.
fn change_the_middle_byte(path: &Path) -> usize {
    let mut bytes = read(path);
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    middle
}

/// The committed lowering is made from the inputs as they are now, and its lowered files are
/// the ones its record names: the digest and every sha256 in `inputs.json` are the current
/// ones. Nothing is lowered here.
#[test]
fn the_committed_lowering_is_current() {
    if let Err(failure) = check_directory(&committed(), &inputs(&repository(), &ROOTS)) {
        panic!("{failure}");
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

/// A file or directory whose name starts with `.` is not an input: a `.DS_Store`, a hidden
/// directory and an editor's lock beside two inputs leave the record as it was.
#[test]
fn a_name_that_starts_with_a_dot_is_not_an_input() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("one.yaml"), "one\n").unwrap();
    fs::write(temp.path().join("two.rs"), "two\n").unwrap();
    let roots: [Root<'_>; 1] = [("", None)];
    let first = inputs(temp.path(), &roots);

    fs::write(temp.path().join(".DS_Store"), "finder\n").unwrap();
    fs::create_dir(temp.path().join(".cache")).unwrap();
    fs::write(temp.path().join(".cache/three.rs"), "three\n").unwrap();
    std::os::unix::fs::symlink("nobody@nowhere.1:1", temp.path().join(".#two.rs")).unwrap();
    let second = inputs(temp.path(), &roots);
    assert_eq!(
        first, second,
        "the hidden names change neither the digest nor the rows"
    );
    assert_eq!(
        input_paths(&second),
        ["one.yaml", "two.rs"],
        "the record names the two inputs and no hidden file"
    );
}

/// A root with an extension takes the files of that extension under it, at any depth, and no
/// other file: not a file of another extension, not one whose name only ends in the letters,
/// not one whose extension follows it, and not one of the extension beside the root.
#[test]
fn an_extension_root_takes_its_files_at_any_depth_and_no_file_of_another_extension() {
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "src/a.rs",
        "src/deep/b.rs",
        "src/deep/deeper/c.rs",
        "src/deep/e.yaml",
        "src/d.txt",
        "src/f.rs.orig",
        "src/rs",
        "src/grs",
        "beside/h.rs",
    ] {
        let path = temp.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{name}\n")).unwrap();
    }
    assert_eq!(
        input_paths(&inputs(temp.path(), &[("src", Some("rs"))])),
        ["src/a.rs", "src/deep/b.rs", "src/deep/deeper/c.rs"],
        "the `.rs` files under `src`, at every depth, and nothing else"
    );
    assert_eq!(
        input_paths(&inputs(temp.path(), &[("src/deep", None)])),
        ["src/deep/b.rs", "src/deep/deeper/c.rs", "src/deep/e.yaml"],
        "a root without an extension takes every file under it"
    );
}

/// An input that cannot be read fails with its path: here a link that points nowhere.
#[test]
fn an_input_that_cannot_be_read_fails_with_its_path() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("here.rs"), "here\n").unwrap();
    std::os::unix::fs::symlink("nowhere", temp.path().join("gone.rs")).unwrap();
    let base = temp.path().to_path_buf();
    let panic = std::panic::catch_unwind(move || inputs(&base, &[("", Some("rs"))]))
        .expect_err("a link that points nowhere cannot be read");
    let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.contains("gone.rs"),
        "the failure names the file that could not be read: {message}"
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

/// The rows count, not only the digest over them: a row whose sha256 is not the current one
/// is reported as changed where the recorded digest is the current one too.
#[test]
fn a_changed_row_under_the_current_digest_is_reported_as_changed() {
    let current = inputs(&repository(), &ROOTS);
    let mut recorded = current.clone();
    recorded["files"][0]["sha256"] = json!("0".repeat(64));
    let path = recorded["files"][0]["path"].as_str().unwrap().to_owned();
    assert_eq!(
        recorded["digest"], current["digest"],
        "the digests are equal"
    );
    let stale = check(&recorded, &current).expect_err("a row that is not the current one");
    assert!(
        stale.contains("stale")
            && stale.contains(&format!("changed: {path}"))
            && stale.contains(REGENERATE),
        "the failure says stale, lists `{path}` and holds the regeneration command: {stale}"
    );
}

/// An input that was added lists as added, and the failure says the other way out beside the
/// regeneration command: a file that is not to be committed (here an editor's backup, which is
/// an input because it lies under a root) is removed.
#[test]
fn an_added_input_is_listed_and_the_failure_says_to_remove_one_that_is_not_to_be_committed() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("one.yaml"), "one\n").unwrap();
    let roots: [Root<'_>; 1] = [("", None)];
    let first = inputs(temp.path(), &roots);
    fs::write(temp.path().join("one.yaml~"), "one\n").unwrap();
    let second = inputs(temp.path(), &roots);
    let stale = check(&first, &second).expect_err("an added input makes the record stale");
    assert!(
        stale.contains("added: one.yaml~") && stale.contains(REGENERATE),
        "the failure lists the added file and holds the regeneration command: {stale}"
    );
    assert!(
        stale.contains("that is not to be committed") && stale.contains("is removed instead"),
        "the failure says that a file not to be committed is removed: {stale}"
    );
}

/// A record of another format fails the check before its rows are read, and the failure names
/// the record's format, the format this check reads and the regeneration command.
#[test]
fn a_record_of_another_format_fails_with_both_formats_and_the_regeneration_command() {
    let current = inputs(&repository(), &ROOTS);
    let mut recorded = current.clone();
    recorded["format"] = json!("connectors-released-lowering/1");
    let failure = check(&recorded, &current).expect_err("a record of format 1 is not read");
    assert!(
        failure.contains("has the format \"connectors-released-lowering/1\"")
            && failure.contains(&format!("this check reads \"{FORMAT}\""))
            && failure.contains(REGENERATE),
        "the failure names both formats and the regeneration command: {failure}"
    );
}

/// A recorded digest that is not the digest of the record's own rows, each of them current,
/// is not reported as a stale lowering with an empty list: the failure says what is wrong. It
/// does not print the current digest: pasted into the record, that value would pass the check
/// with nothing regenerated.
#[test]
fn a_wrong_digest_over_current_rows_says_that_the_digest_is_not_theirs() {
    let current = inputs(&repository(), &ROOTS);
    let mut recorded = current.clone();
    recorded["digest"] = json!("0".repeat(64));
    let failure = check(&recorded, &current).expect_err("the recorded digest is not the current");
    assert!(
        failure.contains("is not the digest of the input files it lists")
            && failure.contains(REGENERATE),
        "the failure says that the digest is not the one of the listed files: {failure}"
    );
    assert!(
        !failure.contains("input files that changed"),
        "no empty list of changed files: {failure}"
    );
    let digest = current["digest"].as_str().expect("the digest is text");
    assert!(
        digest.len() == 64 && !failure.contains(digest),
        "the failure does not print the current digest `{digest}`: {failure}"
    );
}

/// A path that two rows name fails the fast check, in the list of the inputs and in the list
/// of the lowered files, whether the first of the two rows holds another sha256 (the later
/// row would hide it) or the same. The failure names the list, the path and the command.
#[test]
fn a_path_that_two_rows_name_fails_the_fast_check() {
    let (_temp, copy) = copy_of_committed();
    let path = copy.join(INPUTS_FILE);
    let committed = read_json(&path);
    check_directory(&copy, &committed).expect("an unchanged copy passes the fast check");
    for list in ["files", "lowered"] {
        let row = committed[list][0].clone();
        let name = row["path"].as_str().unwrap().to_owned();
        let own = row["sha256"].as_str().unwrap().to_owned();
        for first in ["0".repeat(64), own] {
            let mut recorded = committed.clone();
            let listed = recorded[list].as_array_mut().unwrap();
            listed.insert(0, json!({"path": name, "sha256": first}));
            write_json(&path, &recorded);
            let Err(failure) = check_directory(&copy, &committed) else {
                panic!("`{name}` twice in `{list}`, first with {first}, passes the fast check");
            };
            assert!(
                failure.contains(&format!("the list `{list}`"))
                    && failure.contains(&format!("names the path `{name}` twice"))
                    && failure.contains(REGENERATE),
                "the failure names `{list}`, `{name}` and the regeneration command: {failure}"
            );
        }
    }
    write_json(&path, &committed);
    check_directory(&copy, &committed).expect("the restored record passes again");
}

/// The fast check on a directory compares the directory's record with the inputs it is given,
/// not with itself: where an input has another sha256 now than the record of the copy holds,
/// it fails as stale and names that input.
#[test]
fn the_fast_check_of_a_directory_fails_when_a_current_input_is_not_the_recorded_one() {
    let (_temp, copy) = copy_of_committed();
    let recorded = read_json(&copy.join(INPUTS_FILE));
    check_directory(&copy, &recorded).expect("an unchanged copy passes the fast check");
    let mut current = recorded.clone();
    current["files"][0]["sha256"] = json!("0".repeat(64));
    current["digest"] = json!("1".repeat(64));
    let name = current["files"][0]["path"].as_str().unwrap().to_owned();
    let failure = check_directory(&copy, &current).expect_err("an input changed after the record");
    assert!(
        failure.contains("stale")
            && failure.contains(&format!("changed: {name}"))
            && failure.contains(REGENERATE),
        "the failure says stale, lists `{name}` and holds the regeneration command: {failure}"
    );
}

/// A record that does not read as JSON (here: a merge's conflict marker) and a record that is
/// missing both fail the fast check with the regeneration command.
#[test]
fn a_record_that_does_not_read_prints_the_regeneration_command() {
    let (_temp, copy) = copy_of_committed();
    let path = copy.join(INPUTS_FILE);
    let current = read_json(&path);
    check_directory(&copy, &current).expect("an unchanged copy passes the fast check");

    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, format!("<<<<<<< HEAD\n{text}")).unwrap();
    let failure = check_directory(&copy, &current).expect_err("a conflict marker is not JSON");
    assert!(
        failure.contains(INPUTS_FILE) && failure.contains(REGENERATE),
        "the failure names the record and the regeneration command: {failure}"
    );

    fs::remove_file(&path).unwrap();
    let failure = check_directory(&copy, &current).expect_err("a missing record fails");
    assert!(
        failure.contains(INPUTS_FILE) && failure.contains(REGENERATE),
        "the failure names the record and the regeneration command: {failure}"
    );
}

/// The fast check binds the lowered files, not only the inputs: one changed byte in any file
/// of a copy of the committed directory fails it, and the failure names the file and the
/// regeneration command. Nothing is lowered here.
#[test]
fn a_changed_byte_in_a_lowered_file_of_a_copy_fails_the_fast_check() {
    let (_temp, copy) = copy_of_committed();
    let current = read_json(&copy.join(INPUTS_FILE));
    check_directory(&copy, &current).expect("an unchanged copy passes the fast check");
    let names: Vec<String> = lowered(&copy).into_keys().collect();
    assert_eq!(
        names,
        [
            "github.bundle.json",
            "github/connector.json",
            "github/operations.json",
            "index.json",
            "lowered.json"
        ],
        "the lowered files of the directory"
    );
    for name in &names {
        let path = copy.join(name);
        let bytes = read(&path);
        change_the_middle_byte(&path);
        match check_directory(&copy, &current) {
            Ok(()) => panic!("one changed byte of {name} passes the fast check"),
            Err(failure) => assert!(
                failure.contains(&format!("changed: {name}")) && failure.contains(REGENERATE),
                "the failure names {name} and the regeneration command: {failure}"
            ),
        }
        fs::write(&path, bytes).unwrap();
    }
    check_directory(&copy, &current).expect("the restored copy passes again");
}

/// The bundle's own guard is the digest in `index.json`. A bundle edited together with that
/// row loads, so the load does not notice it; the fast check does, and names both files.
#[test]
fn a_bundle_edited_together_with_its_index_row_fails_the_fast_check() {
    let (_temp, copy) = copy_of_committed();
    let current = read_json(&copy.join(INPUTS_FILE));
    let replace = |name: &str, from: &str, to: &str| {
        let text = fs::read_to_string(copy.join(name)).unwrap();
        assert!(text.contains(from), "`{from}` is in {name}");
        fs::write(copy.join(name), text.replacen(from, to, 1)).unwrap();
    };
    let before = sha256(&read(&copy.join("github.bundle.json")));
    replace(
        "github.bundle.json",
        "\"license\":\"MIT\"",
        "\"license\":\"MIX\"",
    );
    let after = sha256(&read(&copy.join("github.bundle.json")));
    replace("index.json", &before, &after);
    bundle::load(&copy, "github").expect("the edited bundle loads: its index row names its digest");

    let failure = check_directory(&copy, &current).expect_err("the pair fails the fast check");
    assert!(
        failure.contains("changed: github.bundle.json")
            && failure.contains("changed: index.json")
            && failure.contains(REGENERATE),
        "the failure names both files and the regeneration command: {failure}"
    );
}

/// A lowered file that is missing and a file the record does not name both fail the fast
/// check, each by its name.
#[test]
fn a_missing_and_an_extra_lowered_file_fail_the_fast_check() {
    let (_temp, copy) = copy_of_committed();
    let current = read_json(&copy.join(INPUTS_FILE));
    let name = "github/connector.json";
    let bytes = read(&copy.join(name));
    fs::remove_file(copy.join(name)).unwrap();
    let failure = check_directory(&copy, &current).expect_err("a missing file fails");
    assert!(
        failure.contains(&format!("missing: {name}")) && failure.contains(REGENERATE),
        "the failure names the missing file and the regeneration command: {failure}"
    );
    fs::write(copy.join(name), bytes).unwrap();
    check_directory(&copy, &current).expect("the restored copy passes again");

    fs::write(copy.join("github/extra.json"), "{}\n").unwrap();
    let failure = check_directory(&copy, &current).expect_err("an extra file fails");
    assert!(
        failure.contains("not in the record: github/extra.json") && failure.contains(REGENERATE),
        "the failure names the extra file and the regeneration command: {failure}"
    );
}

/// In the committed directory a name that starts with `.` is a file like any other: the record
/// does not name it, so the fast check fails and names it. (Among the inputs such a name is
/// left out; here nothing is.)
#[test]
fn a_name_that_starts_with_a_dot_in_the_directory_fails_the_fast_check_as_not_in_the_record() {
    let (_temp, copy) = copy_of_committed();
    let current = read_json(&copy.join(INPUTS_FILE));
    for name in [
        ".DS_Store",
        "github/.operations.json.swp",
        ".cache/index.json",
    ] {
        let path = copy.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{}\n").unwrap();
        let Err(failure) = check_directory(&copy, &current) else {
            panic!("`{name}` in the directory passes the fast check");
        };
        assert!(
            failure.contains(&format!("not in the record: {name}")) && failure.contains(REGENERATE),
            "the failure names `{name}` and the regeneration command: {failure}"
        );
        fs::remove_file(&path).unwrap();
    }
    fs::remove_dir(copy.join(".cache")).unwrap();
    check_directory(&copy, &current).expect("the copy without the three passes again");
}

/// The comparison the lane test makes fails on one changed byte and names the file. Nothing is
/// lowered here: the second directory is a copy of the committed one.
#[test]
fn a_changed_byte_in_a_copy_of_the_fixture_fails_the_comparison() {
    let committed = committed();
    let (_temp, copy) = copy_of_committed();
    compare(&committed, &copy).expect("an unchanged copy compares equal");

    let name = "github/operations.json";
    let middle = change_the_middle_byte(&copy.join(name));
    let difference = compare(&committed, &copy).expect_err("one changed byte fails the comparison");
    assert!(
        difference.contains(name) && difference.contains(&format!("byte {middle}")),
        "the failure names the changed file and the byte: {difference}"
    );
}

/// The lane test writes only for the value `1` and only where `CI` is not set. Unset, it
/// compares, with or without `CI`. Every other set value is refused and nothing is skipped:
/// an empty value and `0` do not write, and do not pass without comparing either.
#[test]
fn only_the_value_1_outside_ci_writes_and_every_other_set_value_is_refused() {
    assert_eq!(mode(None, None), Ok(Mode::Compare));
    assert_eq!(mode(None, Some("true")), Ok(Mode::Compare));
    assert_eq!(mode(Some("1"), None), Ok(Mode::Write));
    for other in ["", "0", "true", "yes", " 1", "1 ", "11"] {
        let Err(refused) = mode(Some(other), None) else {
            panic!("the value `{other}` writes or compares: it is neither `1` nor unset");
        };
        assert!(
            refused.contains(WRITE) && refused.contains("only the value `1` writes"),
            "the refusal of `{other}` names the variable and the one value: {refused}"
        );
    }
    for ci in ["true", "1", "", "false", "0"] {
        for value in ["1", "0", ""] {
            let Err(refused) = mode(Some(value), Some(ci)) else {
                panic!("the value `{value}` writes or compares where CI is `{ci}`");
            };
            assert!(
                refused.contains(WRITE) && refused.contains("`CI` is set"),
                "the refusal names the variable and CI: {refused}"
            );
        }
    }
}

/// Regeneration from the sources, outside the gate (lane `slow`): the released definition
/// lowers to the committed directory, byte for byte. With `CONNECTORS_WRITE_RELEASED_LOWERING`
/// set to `1` it writes the directory in place of comparing: that is how the lowering is
/// regenerated. Any other value, and `1` where `CI` is set, is refused before anything is
/// lowered (see `mode`).
#[test]
#[ignore = "lane slow"]
fn slow_lane_the_committed_lowering_equals_a_fresh_lowering() {
    let variable =
        |name: &str| std::env::var_os(name).map(|value| value.to_string_lossy().into_owned());
    let mode = match mode(variable(WRITE).as_deref(), variable("CI").as_deref()) {
        Ok(mode) => mode,
        Err(refused) => panic!("{refused}"),
    };
    let temp = tempfile::tempdir().unwrap();
    let fresh = temp.path().join("fresh");
    lower_into(&fresh);
    let committed = committed();
    match mode {
        Mode::Write => {
            if committed.exists() {
                fs::remove_dir_all(&committed)
                    .unwrap_or_else(|error| panic!("{}: {error}", committed.display()));
            }
            copy_directory(&fresh, &committed);
        }
        Mode::Compare => {
            if let Err(difference) = compare(&committed, &fresh) {
                panic!(
                    "{DIRECTORY} (the first directory) is not what the released definition \
                     lowers to now (the second): {difference}\n{}",
                    regenerate()
                );
            }
        }
    }
}
