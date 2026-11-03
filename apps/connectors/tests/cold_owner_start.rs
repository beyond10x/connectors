//! story:cold-owner-start-verifies-once. A read with no owner running starts
//! one, and the owner verifies the whole metadata store before it answers its
//! greeting. The observed defect: on a store of about 200 MB a cold
//! `operations describe` took 95.7 s and the next read 26.3 s, beyond a
//! consumer's 60-second limit per call, and the store was verified twice per
//! cold read, once by the CLI admitting the read directly and again by the
//! owner it then started.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). A test build
//! appends one line per verification of the store, by either process, to the
//! file `CONNECTORS_TEST_VERIFICATION_LOG` names: `<pid> open` for each fresh
//! open, which verifies the whole store, and `<pid> snapshot` for each
//! complete read. Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::runtime;
use mounted::*;
use serde_json::json;
use std::{
    path::PathBuf,
    process::{Output, Stdio},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "cold_owner_start_adapter_fixture";
/// Inside the jail; the owner a logged command starts inherits it.
const LOG: &str = "/work/verifications";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn cold_owner_start_adapter_fixture() {
    serve_when_child();
}

/// A container with one ready connection, its description cached and no
/// owner running: its reference.
fn connected() -> (Container, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    success(&container.run(&["setup", "owner-stop"]));
    (container, connection)
}

/// The CLI in the jail, as [`Container::run`] runs it, counting its
/// verifications and those of the owner it starts in [`LOG`].
fn logged(container: &Container, args: &[&str]) -> Output {
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(args)
            .env_clear()
            .env("CONNECTORS_TEST_VERIFICATION_LOG", LOG)
            .stdin(Stdio::null()),
    )
}

fn read_args<'a>(connection: &'a str, schema: &'a str, target: &'a str) -> [&'a str; 11] {
    [
        "invoke",
        target,
        READ,
        "--connection",
        connection,
        "--schema",
        schema,
        "--revision",
        REVISION,
        "--input-json",
        "{}",
    ]
}

const DESCRIBE: [&str; 6] = [
    "operations",
    "describe",
    "--adapter",
    ALIAS,
    "--operation",
    READ,
];

/// The verifications logged since the last call, by kind, and the lines.
struct Passes {
    opens: usize,
    snapshots: usize,
    processes: usize,
    lines: Vec<String>,
}

fn passes(container: &Container) -> Passes {
    let path = container.path(LOG);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let kind = |kind: &str| {
        lines
            .iter()
            .filter(|line| line.split_once(' ').is_some_and(|(_, k)| k == kind))
            .count()
    };
    let processes = lines
        .iter()
        .filter_map(|line| line.split_once(' ').map(|(pid, _)| pid))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    Passes {
        opens: kind("open"),
        snapshots: kind("snapshot"),
        processes,
        lines,
    }
}

/// One cold read, CLI and the owner it starts together, verifies the store
/// once; the next read, answered by that owner, verifies nothing again.
#[test]
fn a_cold_read_verifies_the_store_once() {
    let (container, connection) = connected();
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let args = read_args(&connection, &schema, &target);
    let read = success(&logged(&container, &args));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    let cold = passes(&container);
    assert_eq!(
        (cold.opens, cold.snapshots, cold.processes),
        (1, 1, 1),
        "fresh opens, complete reads and processes that verified: {:?}",
        cold.lines
    );
    let read = success(&logged(&container, &args));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    let warm = passes(&container);
    assert_eq!(
        (warm.opens, warm.snapshots),
        (0, 0),
        "the next read verified the store again: {:?}",
        warm.lines
    );
}

/// A cold `operations describe` of a cached description reads the store
/// directly and starts no owner: one verification.
#[test]
fn a_cold_describe_verifies_the_store_once() {
    let (container, _) = connected();
    let described = success(&logged(&container, &DESCRIBE));
    assert_eq!(described["operation"]["id"], READ, "{described}");
    let cold = passes(&container);
    assert_eq!(
        (cold.opens, cold.snapshots, cold.processes),
        (1, 1, 1),
        "fresh opens, complete reads and processes that verified: {:?}",
        cold.lines
    );
    assert!(
        !container.path(STATE).join("owner.sock").exists(),
        "a describe of a cached description started an owner"
    );
}

/// A store that fails verification refuses a cold read by the direct read's
/// own code, and no provider is called.
#[test]
fn a_store_that_fails_verification_refuses_a_cold_read_before_any_provider_call() {
    let (container, connection) = connected();
    let validations = container.seen("validations");
    assert_eq!(container.seen("invocations"), Vec::<String>::new());
    let store = rusqlite::Connection::open(container.path(STATE).join("metadata.sqlite3")).unwrap();
    let altered = store
        .execute(
            "UPDATE connectors_er_blobs SET bytes=zeroblob(length(bytes))",
            [],
        )
        .unwrap();
    assert!(altered > 0);
    store.close().unwrap();
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let (_, data) = refusal(&logged(
        &container,
        &read_args(&connection, &schema, &target),
    ));
    assert_eq!(data["code"], "metadata_unavailable", "{data}");
    assert_eq!(container.seen("invocations"), Vec::<String>::new());
    assert_eq!(container.seen("validations"), validations);
}

/// Grows the store in `state` by remembering descriptions of filler instances,
/// each record carrying about `record` bytes, until the database file and its
/// log hold `target` bytes. Returns the records written.
fn grow(state: &std::path::Path, target: u64, record: usize) -> u64 {
    let size = || {
        ["metadata.sqlite3", "metadata.sqlite3-wal"]
            .iter()
            .map(|name| std::fs::metadata(state.join(name)).map_or(0, |m| m.len()))
            .sum::<u64>()
    };
    let store = runtime::state::State::new(state);
    let padding = "p".repeat(record);
    let mut written = 0u64;
    while size() < target {
        let mut filler = bootstrap();
        filler.instance = format!("filler-{}", written % 900);
        let mut descriptor = filler.descriptor().unwrap();
        descriptor.instance = filler.instance.clone();
        descriptor.configuration_schema =
            json!({"type":"object","description":format!("{written} {padding}")});
        filler.descriptor = serde_json::to_string(&descriptor).unwrap();
        store.remember("filler", &filler).unwrap();
        written += 1;
    }
    let sqlite = rusqlite::Connection::open(state.join("metadata.sqlite3")).unwrap();
    sqlite
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .unwrap();
    written
}

fn timed(container: &Container, args: &[&str]) -> (Duration, Output) {
    let started = Instant::now();
    let output = logged(container, args);
    (started.elapsed(), output)
}

/// The 60-second line on a store of about 200 MB (the reported one held
/// 172 MB in 21,830 blobs): a cold `operations describe` and a cold read each
/// answer within 60 s, each verifies the store once, and the next read
/// verifies nothing and takes no longer than a warm one. A measurement against a deadline, run
/// explicitly, on a release build for the record (which logs no verification,
/// so the counts are checked on a test build only):
/// `cargo test --release -p connectors --test cold_owner_start -- --ignored --nocapture`.
/// `CONNECTORS_COLD_START_STORE_MB` and `CONNECTORS_COLD_START_RECORD_BYTES`
/// set the store's size (200) and each record's padding (8,000).
#[test]
#[ignore = "timing measurement; run explicitly on a release build"]
fn a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds() {
    let number = |name: &str, default: u64| {
        std::env::var(name).map_or(default, |value| value.trim().parse().unwrap())
    };
    let target = number("CONNECTORS_COLD_START_STORE_MB", 200) * 1024 * 1024;
    let record = usize::try_from(number("CONNECTORS_COLD_START_RECORD_BYTES", 8_000)).unwrap();
    let (container, connection) = connected();
    let state: PathBuf = container.path(STATE);
    let started = Instant::now();
    let written = grow(&state, target, record);
    let bytes = std::fs::metadata(state.join("metadata.sqlite3"))
        .unwrap()
        .len();
    let blobs: i64 = rusqlite::Connection::open(state.join("metadata.sqlite3"))
        .unwrap()
        .query_row("SELECT count(*) FROM connectors_er_blobs", [], |row| {
            row.get(0)
        })
        .unwrap();
    println!(
        "store bytes={bytes} blobs={blobs} records_written={written} grown_in_s={:.1}",
        started.elapsed().as_secs_f64()
    );
    let limit = Duration::from_secs(60);
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let args = read_args(&connection, &schema, &target);

    let (describe, described) = timed(&container, &DESCRIBE);
    let describe_passes = passes(&container);
    let (cold, cold_read) = timed(&container, &args);
    let cold_passes = passes(&container);
    let (next, next_read) = timed(&container, &args);
    let next_passes = passes(&container);
    let (warm, warm_read) = timed(&container, &args);
    let warm_passes = passes(&container);
    println!(
        "cold_describe_s={:.1} cold_read_s={:.1} next_read_s={:.2} warm_read_s={:.2} exits={:?}",
        describe.as_secs_f64(),
        cold.as_secs_f64(),
        next.as_secs_f64(),
        warm.as_secs_f64(),
        [&described, &cold_read, &next_read, &warm_read].map(|output| output.status.code()),
    );
    for (name, logged) in [
        ("cold_describe", &describe_passes),
        ("cold_read", &cold_passes),
        ("next_read", &next_passes),
        ("warm_read", &warm_passes),
    ] {
        println!("{name} verifications={:?}", logged.lines);
    }
    for output in [&described, &cold_read, &next_read, &warm_read] {
        success(output);
    }
    assert!(describe < limit, "cold describe took {describe:?}");
    assert!(cold < limit, "cold read took {cold:?}");
    // A release build logs no verification; the counts hold in a test build.
    if cfg!(debug_assertions) {
        assert_eq!(describe_passes.opens, 1, "{:?}", describe_passes.lines);
        assert_eq!(cold_passes.opens, 1, "{:?}", cold_passes.lines);
        assert_eq!(next_passes.opens, 0, "{:?}", next_passes.lines);
        assert_eq!(warm_passes.opens, 0, "{:?}", warm_passes.lines);
    }
    // One warm read against another: both answered by the running owner, so
    // the next read is held to the warm one with a second of scheduling slack.
    assert!(
        next <= warm + Duration::from_secs(1),
        "the next read took {next:?}, a warm one {warm:?}"
    );
}
