//! Adversary, D417 U1 (story:cold-owner-start-verifies-once): one cold start
//! verifies the store once. The unit's counting tests read a description the
//! connect cached; these read one that is not cached for the instance's
//! configured selection, as an adapter upgrade or a changed executable leaves
//! it, so the read launches the adapter for its description
//! (semantics.md, "Launching for a description").
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`), counting
//! verifications through `CONNECTORS_TEST_VERIFICATION_LOG` as
//! `cold_owner_start.rs` does. Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::runtime;
use mounted::*;
use std::process::{Output, Stdio};

const ADAPTER_FIXTURE: &str = "cold_owner_start_adversary_adapter_fixture";
/// Inside the jail; the owner a logged command starts inherits it.
const LOG: &str = "/work/verifications";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn cold_owner_start_adversary_adapter_fixture() {
    serve_when_child();
}

/// A container with one ready connection and no owner running: its reference.
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

/// (fresh opens, complete reads, processes that verified) and the lines.
fn passes(container: &Container) -> ((usize, usize, usize), Vec<String>) {
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
    ((kind("open"), kind("snapshot"), processes), lines)
}

/// The instance's description cached under a selection that is no longer the
/// configured one: the store holds one cached description per instance, so the
/// configured selection then finds none.
fn uncache(container: &Container) {
    let state = runtime::state::State::new(&container.path(STATE));
    state.remember("superseded-selection", &bootstrap()).unwrap();
    assert!(
        state
            .cached("forge-local", "superseded-selection")
            .unwrap()
            .is_some(),
        "the description is cached only under the superseded selection"
    );
}

/// A cold read whose description is not cached starts the owner first; the
/// owner answers its `cached` request `description_unavailable`, and the CLI
/// then reads the store directly (`owner::cached_through` falls back to
/// `cached_direct`) and checks the launch condition directly
/// (`Registry::launch_admits`) before the owner launches the adapter: the
/// store is verified in the CLI and in the owner.
#[test]
fn a_cold_read_of_an_uncached_description_verifies_the_store_once() {
    let (container, connection) = connected();
    uncache(&container);
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let read = success(&logged(
        &container,
        &[
            "invoke",
            &target,
            READ,
            "--connection",
            &connection,
            "--schema",
            &schema,
            "--revision",
            REVISION,
            "--input-json",
            "{}",
        ],
    ));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    let (cold, lines) = passes(&container);
    assert_eq!(
        cold,
        (1, 1, 1),
        "fresh opens, complete reads and processes that verified: {lines:?}"
    );
}

/// The instance's cached description as a bound one: its own descriptor under
/// the binding `primary`, and one alternative binding `secondary` whose
/// descriptor carries its own revision, cached under the configured selection.
fn cache_bound(container: &Container) {
    let config = connectors_host::local::config::Config::load(&container.path(CONFIG)).unwrap();
    let selection = config.adapters[ALIAS].selection();
    let mut bound = bootstrap();
    let mut alternative: serde_json::Value = serde_json::from_str(&bound.descriptor).unwrap();
    alternative["revision"] = "desc-2".into();
    bound.bindings = Some(runtime::Bindings {
        id: "primary".into(),
        alternatives: vec![runtime::AlternativeBinding {
            id: "secondary".into(),
            descriptor: alternative.to_string(),
            requirements: bound.requirements.clone(),
        }],
    });
    let state = runtime::state::State::new(&container.path(STATE));
    state.remember(&selection, &bound).unwrap();
    assert!(
        state
            .cached("forge-local", &selection)
            .unwrap()
            .is_some_and(|cached| cached.bindings.is_some()),
        "the configured selection's description is the bound one"
    );
}

/// A cold read naming its connection, of an instance whose cached description
/// is bound: `owner::bound_described` reads the connection's binding with
/// `Registry::describe` in the CLI process even though the owner the read
/// started holds the verified handle, so the store is verified in the CLI and
/// in the owner.
#[test]
fn a_cold_read_of_a_bound_description_verifies_the_store_once() {
    let (container, connection) = connected();
    cache_bound(&container);
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let output = logged(
        &container,
        &[
            "invoke",
            &target,
            READ,
            "--connection",
            &connection,
            "--schema",
            &schema,
            "--revision",
            REVISION,
            "--input-json",
            "{}",
        ],
    );
    let (cold, lines) = passes(&container);
    assert_eq!(
        cold,
        (1, 1, 1),
        "fresh opens, complete reads and processes that verified: {lines:?}; exit {:?}, stdout {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );
    let read = success(&output);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
}
