use connectors_catalog::bundle::{INDEX_FILE, load, read_index};
use connectors_catalog::pipeline::{ORDER, Progress, Request, Step, run};
use connectors_catalog::{Refusal, ingest};
use connectors_core::{Error, ErrorCode};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Two operations, and two gaps the inventory names: a document member this
/// build does not read, and a path item it does not resolve. A fixture with no
/// unsupported entry would let a record that dropped them still pass.
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

fn refused_document() -> Vec<u8> {
    br#"{"swagger":"2.0","info":{"title":"Fixture","version":"1"},"paths":{}}"#.to_vec()
}

fn written(home: &Path, bytes: &[u8]) -> PathBuf {
    let path = home.join("fixture.json");
    std::fs::write(&path, bytes).expect("write source document");
    path
}

fn request<'a>(source: &'a Path, directory: &'a Path, replace: bool) -> Request<'a> {
    Request {
        provider: "fixture",
        source,
        directory,
        auth_profile: "fixture.token",
        replace,
    }
}

/// The order the story states, written out. Comparing a record's steps against
/// `ORDER` asserts nothing: `progression` builds them from `ORDER`, so any
/// reordering of the five would keep such a comparison green. These are the five
/// literals, so a reordering has to fail here.
fn stated_order() -> Vec<Step> {
    vec![
        Step::Read,
        Step::Ingest,
        Step::Extract,
        Step::Report,
        Step::Write,
    ]
}

/// Every entry the bundle directory holds, sorted, so a listing can be compared
/// rather than a return value trusted.
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

#[test]
fn a_valid_document_is_written_indexed_and_recorded() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), &document());
    let directory = home.path().join("bundles");

    let record = run(&request(&source, &directory, false)).expect("a 3.1 document is accepted");

    assert_eq!(record.bundle_path, directory.join("fixture.bundle.json"));
    assert!(record.bundle_path.exists(), "the bundle file must be there");
    assert_eq!(
        record.source,
        ingest("fixture.json", &document()).expect("fixture ingests")
    );

    // The index the write produced is the one on disk, read back by the module
    // that owns it rather than by this test.
    let index = read_index(&directory).expect("index reads");
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.find("fixture"), Some(&record.entry));

    // The record's coverage counts are the inventory's own, not a second count.
    let bundle = load(&directory, "fixture").expect("bundle loads");
    assert_eq!(
        bundle.inventory.coverage(),
        (record.coverage.inventoried, record.coverage.unsupported)
    );
    assert_eq!(
        (record.coverage.inventoried, record.coverage.unsupported),
        (2, 2)
    );
    // The write puts the bundle in place through a working name; a successful run
    // leaves that name behind nowhere.
    assert_eq!(
        listing(&directory),
        vec!["fixture.bundle.json".to_owned(), INDEX_FILE.to_owned()]
    );
    assert_eq!(record.entry.operations, record.coverage.inventoried);
    assert_eq!(record.entry.unsupported, record.coverage.unsupported);
    assert_eq!(bundle.auth_profile, "fixture.token");

    let steps: Vec<Step> = record.steps.iter().map(|s| s.step).collect();
    assert_eq!(steps, stated_order());
    // The published constant is the same five, in the same order.
    assert_eq!(ORDER.to_vec(), stated_order());
    assert!(
        record
            .steps
            .iter()
            .all(|s| s.progress == Progress::Completed),
        "a completed run has no unfinished step: {:?}",
        record.steps
    );
}

#[test]
fn an_ingest_refusal_leaves_the_bundle_directory_uncreated() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), &refused_document());
    let directory = home.path().join("bundles");

    let failure = run(&request(&source, &directory, false)).expect_err("swagger 2.0 is refused");

    assert_eq!(failure.step, Step::Ingest);
    // The refusing step's own error, not a new one written over it.
    let own = Error::from(Refusal::VersionAbsent);
    assert_eq!(failure.error.message, own.message);
    assert_eq!(failure.error.code, own.code);
    // Listed, not inferred from the return value: no bundle, and no index.
    assert!(!directory.exists(), "the directory must not be created");
    assert!(!directory.join(INDEX_FILE).exists());
    assert_eq!(listing(&directory), Vec::<String>::new());

    assert!(failure.source.is_none());
    assert!(failure.coverage.is_none());
    let steps: Vec<Step> = failure.steps.iter().map(|s| s.step).collect();
    assert_eq!(steps, stated_order());
    assert_eq!(failure.steps[0].progress, Progress::Completed);
    assert_eq!(failure.steps[1].progress, Progress::Refused);
    assert!(
        failure.steps[2..]
            .iter()
            .all(|s| s.progress == Progress::NotReached),
        "no step after the refusal ran: {:?}",
        failure.steps
    );
    // The step travels beside the error when the failure is rendered.
    assert!(
        format!("{failure}").contains("ingest"),
        "unexpected rendering: {failure}"
    );
}

#[test]
fn an_absent_source_names_the_read_step() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = home.path().join("absent.json");
    let directory = home.path().join("bundles");

    let failure = run(&request(&source, &directory, false)).expect_err("an absent file is refused");

    assert_eq!(failure.step, Step::Read);
    let own = Error::from(Refusal::Unreadable);
    assert_eq!(failure.error.message, own.message);
    assert_eq!(failure.error.code, ErrorCode::NotFound);
    assert!(!directory.exists(), "the directory must not be created");
    assert_eq!(failure.steps[0].progress, Progress::Refused);
}

#[test]
fn a_refusal_leaves_an_existing_index_byte_identical() {
    let home = tempfile::tempdir().expect("temporary directory");
    let good = written(home.path(), &document());
    let directory = home.path().join("bundles");
    run(&request(&good, &directory, false)).expect("the first run writes");

    let before = std::fs::read(directory.join(INDEX_FILE)).expect("index reads");
    let listed = listing(&directory);

    let bad = home.path().join("other.json");
    std::fs::write(&bad, refused_document()).expect("write source document");
    let failure = run(&Request {
        provider: "second",
        source: &bad,
        directory: &directory,
        auth_profile: "fixture.token",
        replace: false,
    })
    .expect_err("swagger 2.0 is refused");

    assert_eq!(failure.step, Step::Ingest);
    assert_eq!(
        std::fs::read(directory.join(INDEX_FILE)).expect("index reads"),
        before,
        "the index must be exactly as it was"
    );
    assert_eq!(listing(&directory), listed);
}

#[test]
fn an_indexed_provider_refuses_at_the_write_step_with_the_earlier_results() {
    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), &document());
    let directory = home.path().join("bundles");
    let first = run(&request(&source, &directory, false)).expect("the first run writes");
    let before = std::fs::read(directory.join(INDEX_FILE)).expect("index reads");
    let listed = listing(&directory);

    let failure = run(&request(&source, &directory, false))
        .expect_err("a second write without replacement is refused");

    assert_eq!(failure.step, Step::Write);
    assert!(
        failure.error.message.contains("already indexed"),
        "unexpected message: {}",
        failure.error.message
    );
    assert_eq!(failure.error.code, ErrorCode::InvalidInput);
    // The earlier steps' results are still in the record.
    assert_eq!(failure.source.as_ref(), Some(&first.source));
    assert_eq!(failure.coverage.as_ref(), Some(&first.coverage));
    let steps: Vec<Step> = failure.steps.iter().map(|s| s.step).collect();
    assert_eq!(steps, stated_order());
    assert!(
        failure.steps[..4]
            .iter()
            .all(|s| s.progress == Progress::Completed),
        "every step before the write completed: {:?}",
        failure.steps
    );
    assert_eq!(failure.steps[4].progress, Progress::Refused);
    // Nothing reached the directory: the index is the one the first run left.
    assert_eq!(
        std::fs::read(directory.join(INDEX_FILE)).expect("index reads"),
        before
    );
    assert_eq!(listing(&directory), listed);

    // Replacement asked for is the only difference.
    let replaced = run(&request(&source, &directory, true)).expect("replacement is accepted");
    assert_eq!(replaced.entry, first.entry);
    assert_eq!(
        read_index(&directory).expect("index reads").entries.len(),
        1
    );
}

#[test]
fn two_runs_over_the_same_bytes_record_the_same_thing() {
    let first_home = tempfile::tempdir().expect("temporary directory");
    let second_home = tempfile::tempdir().expect("temporary directory");
    let first_source = written(first_home.path(), &document());
    let second_source = written(second_home.path(), &document());
    let first_directory = first_home.path().join("bundles");
    let second_directory = second_home.path().join("bundles");

    let first = run(&request(&first_source, &first_directory, false)).expect("first run");
    let second = run(&request(&second_source, &second_directory, false)).expect("second run");

    assert_ne!(first.bundle_path, second.bundle_path);
    // Equal apart from the path: normalising the one path is enough to make the
    // whole records compare equal, so no other field carries a directory.
    let mut normalised = second.clone();
    normalised.bundle_path = first.bundle_path.clone();
    assert_eq!(first, normalised);
}

/// The other half of the pair the story's Scope sentence governs: the adversary's
/// probe fails the index write after the bundle landed; this one fails the bundle
/// write and asks whether the index survived it. A directory nothing may create a
/// file in is the ordinary `EACCES`/`EROFS` condition.
#[test]
fn a_bundle_write_refusal_leaves_the_index_exactly_as_it_was() {
    use std::os::unix::fs::PermissionsExt;

    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), &document());
    let directory = home.path().join("bundles");
    run(&request(&source, &directory, false)).expect("the first run writes");
    let before = std::fs::read(directory.join(INDEX_FILE)).expect("index reads");
    let listed = listing(&directory);

    let mut mode = std::fs::metadata(&directory)
        .expect("directory metadata")
        .permissions();
    mode.set_mode(0o555);
    std::fs::set_permissions(&directory, mode).expect("make the directory unwritable");
    assert!(
        std::fs::write(directory.join("probe"), b"x").is_err(),
        "this case needs a user the directory mode applies to"
    );

    let failure = run(&Request {
        provider: "second",
        source: &source,
        directory: &directory,
        auth_profile: "fixture.token",
        replace: false,
    })
    .expect_err("a directory that takes no new file must refuse");

    assert_eq!(failure.step, Step::Write);
    assert_eq!(failure.error.code, ErrorCode::Unavailable);
    assert_eq!(
        std::fs::read(directory.join(INDEX_FILE)).expect("index reads"),
        before,
        "the index must be exactly as it was"
    );
    assert_eq!(
        listing(&directory),
        listed,
        "no half-written bundle and no working file may be left behind"
    );

    // Restore, or the temporary directory cannot be cleaned up.
    let mut mode = std::fs::metadata(&directory)
        .expect("directory metadata")
        .permissions();
    mode.set_mode(0o755);
    std::fs::set_permissions(&directory, mode).expect("restore the directory");
}

/// A refusal has to travel: a caller composing this run with anything else uses
/// `?`, and that needs the standard trait, not just a rendering. The module's own
/// error stays reachable as the cause rather than being flattened into a string.
#[test]
fn a_failure_carries_through_the_question_mark_operator() {
    fn compose(request: &Request<'_>) -> Result<(), Box<dyn std::error::Error>> {
        run(request)?;
        Ok(())
    }

    let home = tempfile::tempdir().expect("temporary directory");
    let source = written(home.path(), &refused_document());
    let directory = home.path().join("bundles");

    let error = compose(&request(&source, &directory, false)).expect_err("swagger 2.0 is refused");
    assert!(
        error.to_string().contains("ingest"),
        "unexpected rendering: {error}"
    );
    let cause = std::error::Error::source(&*error).expect("the refusing module's own error");
    assert!(
        cause.to_string().contains(Refusal::VersionAbsent.reason()),
        "unexpected cause: {cause}"
    );
}
