//! Adversarial probes against `pipeline::run`, driven from the story's own
//! statements rather than from the implementation's shape.
//!
//! The two under attack are the story's Scope sentence — "Nothing is written to
//! the bundle directory until every earlier step has succeeded, so a refusal
//! leaves the directory exactly as it was — including the index" — and its
//! Acceptance sentence that a run's index entry is what `bundle::read_index`
//! reports.

use connectors_catalog::bundle::{INDEX_FILE, Index, load, read_index};
use connectors_catalog::pipeline::{ORDER, Request, Run, Step, run};
use connectors_core::ErrorCode;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Barrier;

/// The same shape the unit's own fixture uses: two operations and two named gaps.
fn document() -> Vec<u8> {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Fixture", "version": "4.2", "license": {"name": "MIT"}},
        "webhooks": {"ping": {}},
        "paths": {
            "/things": {
                "get": {
                    "operationId": "listThings",
                    "responses": {"200": {"content": {"application/json": {}}}}
                },
                "post": {"responses": {"201": {}}}
            },
            "/elsewhere": {"$ref": "#/components/pathItems/elsewhere"}
        }
    })
    .to_string()
    .into_bytes()
}

/// A document with enough operations that serialising the bundle takes long
/// enough to see an interleave, rather than one small enough to hide it.
fn wide_document(paths: usize) -> Vec<u8> {
    let mut items = serde_json::Map::new();
    for index in 0..paths {
        items.insert(
            format!("/resource/{index}"),
            json!({
                "get": {
                    "operationId": format!("read{index}"),
                    "parameters": [{"name": "cursor", "in": "query", "required": false}],
                    "responses": {"200": {"content": {"application/json": {}}}}
                }
            }),
        );
    }
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Wide", "version": "1.0"},
        "paths": serde_json::Value::Object(items)
    })
    .to_string()
    .into_bytes()
}

fn written(home: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = home.join(name);
    std::fs::write(&path, bytes).expect("write source document");
    path
}

fn request<'a>(
    provider: &'a str,
    source: &'a Path,
    directory: &'a Path,
    replace: bool,
) -> Request<'a> {
    Request {
        provider,
        source,
        directory,
        auth_profile: "fixture.token",
        replace,
    }
}

fn listing(directory: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.expect("directory entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Story, Scope: "a refusal leaves the directory exactly as it was — including
/// the index."
///
/// `bundle::write` writes the bundle file before it writes the index, and the
/// pipeline has no cleanup path, so a refusal raised by the index write leaves a
/// bundle file in the directory that no index names. The unwritable index here
/// stands for the ordinary filesystem conditions that fail a second write after
/// a first one succeeded — ENOSPC, EDQUOT, EROFS.
#[test]
fn a_write_step_refusal_leaves_a_bundle_file_behind() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), "fixture.json", &document());
    let directory = home.path().join("bundles");
    std::fs::create_dir_all(&directory).expect("create the bundle directory");

    // An index that reads cleanly and cannot be rewritten.
    let index = directory.join(INDEX_FILE);
    let empty = serde_json::to_vec_pretty(&Index::default()).expect("serialise an empty index");
    std::fs::write(&index, &empty).expect("write the index");
    let mut mode = std::fs::metadata(&index)
        .expect("index metadata")
        .permissions();
    mode.set_mode(0o444);
    std::fs::set_permissions(&index, mode).expect("make the index read-only");
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&index)
            .is_err(),
        "this probe needs a user the file mode applies to; the index is still writable"
    );
    assert!(
        read_index(&directory).is_ok(),
        "the index must still read, or the refusal would come before the bundle write"
    );

    let before = listing(&directory);
    assert_eq!(before, vec![INDEX_FILE.to_owned()]);

    let failure = run(&request("fixture", &source, &directory, false))
        .expect_err("the index cannot be rewritten, so the run must refuse");

    assert_eq!(failure.step, Step::Write);
    assert_eq!(failure.error.code, ErrorCode::Unavailable);
    assert_eq!(
        std::fs::read(&index).expect("index reads"),
        empty,
        "the index itself is unchanged"
    );
    assert_eq!(
        listing(&directory),
        before,
        "a refusal must leave the directory exactly as it was, and this one left a bundle file \
         that no index names"
    );
}

/// Story, Acceptance: a run "returns a record ... whose index entry matches what
/// `bundle::read_index` reports."
///
/// `bundle::write` held no lock over its read-modify-write, so two runs into one
/// directory lost each other's entries: every run returned `Ok` with an entry,
/// and the index named fewer providers than there were bundle files beside it.
/// It now takes an exclusive lock for that section, and this case asserts the
/// property the adversary pass originally wrote it for.
#[test]
fn concurrent_runs_into_one_directory_lose_index_entries() {
    const WRITERS: usize = 8;
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), "wide.json", &wide_document(400));

    for round in 0..3 {
        let directory = home.path().join(format!("bundles-{round}"));
        let providers: Vec<String> = (0..WRITERS).map(|n| format!("provider-{n}")).collect();
        let barrier = Barrier::new(WRITERS);

        let records: Vec<Run> = std::thread::scope(|scope| {
            let handles: Vec<_> = providers
                .iter()
                .map(|provider| {
                    let barrier = &barrier;
                    let source = source.as_path();
                    let directory = directory.as_path();
                    scope.spawn(move || {
                        barrier.wait();
                        run(&request(provider, source, directory, false))
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|handle| handle.join().expect("writer thread").ok())
                .collect()
        });

        let index = read_index(&directory).expect("index reads");
        let mut named: Vec<&str> = index.providers();
        named.sort_unstable();
        let mut accepted: Vec<&str> = records.iter().map(|r| r.entry.provider.as_str()).collect();
        accepted.sort_unstable();

        for record in &records {
            assert!(
                record.bundle_path.exists(),
                "round {round}: {} was written",
                record.entry.provider
            );
        }
        // The property holds: every run that returned a record is named by the index.
        // `bundle::write` now holds an exclusive lock over the read-modify-write, so a
        // second writer waits rather than reading a copy the first is about to replace.
        // This equality is the acceptance of `story:catalog-index-concurrent-writers`.
        assert_eq!(
            named,
            accepted,
            "round {round}: the index must name exactly the runs that returned a record; \
             directory holds {:?}",
            listing(&directory)
        );
    }
}

// ---------------------------------------------------------------------------
// Probes that hold. Kept so the report can say what was attacked and did not
// break, rather than only what did.
// ---------------------------------------------------------------------------

/// A provider named `index` produces `index.bundle.json`, which is not the index
/// file: no legal provider name can collide with `index.json`.
#[test]
fn a_provider_named_index_does_not_collide_with_the_index_file() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), "fixture.json", &document());
    let directory = home.path().join("bundles");

    let record = run(&request("index", &source, &directory, false)).expect("`index` is a legal id");
    assert_eq!(record.entry.file_name, "index.bundle.json");
    assert_ne!(record.entry.file_name, INDEX_FILE);
    assert_eq!(
        read_index(&directory).expect("index reads").providers(),
        vec!["index"]
    );
    load(&directory, "index").expect("bundle loads");
}

/// A second provider into a directory that already holds one leaves the first
/// bundle loadable and the index naming both.
#[test]
fn a_second_provider_leaves_the_first_bundle_intact() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), "fixture.json", &document());
    let directory = home.path().join("bundles");

    let first = run(&request("alpha", &source, &directory, false)).expect("first run");
    let second = run(&request("beta", &source, &directory, false)).expect("second run");

    let index = read_index(&directory).expect("index reads");
    assert_eq!(index.providers(), vec!["alpha", "beta"]);
    assert_eq!(index.find("alpha"), Some(&first.entry));
    assert_eq!(index.find("beta"), Some(&second.entry));
    load(&directory, "alpha").expect("the first bundle still loads");
    assert!(first.bundle_path.exists());
}

/// `replace = true` against an index whose recorded file is gone rewrites the
/// file rather than refusing, and the directory is consistent afterwards.
#[test]
fn replacement_restores_an_indexed_bundle_whose_file_is_missing() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), "fixture.json", &document());
    let directory = home.path().join("bundles");

    let first = run(&request("fixture", &source, &directory, false)).expect("first run");
    std::fs::remove_file(&first.bundle_path).expect("delete the indexed bundle file");
    assert!(load(&directory, "fixture").is_err(), "the bundle is gone");

    let replaced = run(&request("fixture", &source, &directory, true)).expect("replacement runs");
    assert_eq!(replaced.entry, first.entry);
    load(&directory, "fixture").expect("the bundle loads again");
}

/// A source that is a directory is refused at the read step, not further on.
#[test]
fn a_directory_as_the_source_names_the_read_step() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = home.path().join("a-directory");
    std::fs::create_dir(&source).expect("create the source directory");
    let directory = home.path().join("bundles");

    let failure =
        run(&request("fixture", &source, &directory, false)).expect_err("a directory is refused");
    assert_eq!(failure.step, Step::Read);
    assert_eq!(failure.error.code, ErrorCode::NotFound);
    assert!(!directory.exists(), "the directory must not be created");
}

/// A symlinked source is followed, and the name recorded is the link's own.
#[test]
fn a_symlinked_source_records_the_link_name() {
    let home = tempfile::tempdir().expect("temporary directory");
    let target = written(home.path(), "target.json", &document());
    let link = home.path().join("link.json");
    std::os::unix::fs::symlink(&target, &link).expect("create the symlink");
    let directory = home.path().join("bundles");

    let record = run(&request("fixture", &link, &directory, false)).expect("a symlink is followed");
    assert_eq!(record.source.file_name, "link.json");
    assert_eq!(record.coverage.source_file_name, "link.json");
}

/// `Step::label` and the serialised form name the same five words, as the
/// module's own comment says they do.
#[test]
fn every_step_label_is_its_serialised_name() {
    for step in ORDER {
        let serialised = serde_json::to_string(&step).expect("a step serialises");
        assert_eq!(serialised, format!("\"{}\"", step.label()));
    }
    assert_eq!(
        ORDER.to_vec(),
        vec![
            Step::Read,
            Step::Ingest,
            Step::Extract,
            Step::Report,
            Step::Write
        ],
        "the order the story states, pinned to the five names rather than to ORDER itself"
    );
}
