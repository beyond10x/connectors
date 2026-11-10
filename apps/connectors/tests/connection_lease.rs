//! story:connection-lease-holds-unattended. The production CLI, owner and an
//! adapter child (`support/mounted.rs`): a connection revalidated once keeps
//! serving reads past its validation evidence's lifetime with no caller
//! revalidation, and two revalidations of one connection started together
//! both answer `ready` within the admission bound. Linux only: the jail is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use std::time::{Duration, Instant};

const ADAPTER_FIXTURE: &str = "connection_lease_adapter_fixture";

/// The bound a `connections revalidate` answers within
/// (contracts/cli/v1alpha1/semantics.md: its owner deadline).
const ADMISSION_BOUND: Duration = Duration::from_secs(30);

/// The adapter child the owner starts. The ordinary run does nothing.
#[test]
fn connection_lease_adapter_fixture() {
    serve_when_child();
}

/// A container with the item delivered under the mounted binding and one
/// connection made: its reference and revision.
fn connected() -> (Container, String, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    let reference = summary["connection"].as_str().unwrap().to_owned();
    let revision = summary["revision"].as_str().unwrap().to_owned();
    (container, reference, revision)
}

/// Acceptance 1: a connection revalidated once serves reads across three
/// evidence lifetimes, with no revalidation by the caller.
#[test]
fn a_revalidated_connection_serves_reads_across_three_evidence_lifetimes() {
    // Well above one read's own host cost (about 3 s measured on a loaded
    // host, 2026-10-15): evidence shorter than that lapses between a read's
    // recollection and its dispatch, which no lifetime near the 60 s default does.
    const LIFETIME_MS: u64 = 8_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    let revalidated = success(&container.revalidate(&connection, &revision));
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
    let until = Instant::now() + Duration::from_millis(3 * LIFETIME_MS + 1_000);
    let mut reads = 0;
    while Instant::now() < until {
        let output = container.invoke(&connection);
        assert!(
            output.status.success(),
            "read {reads}, {} ms after the revalidation, was refused: {}",
            LIFETIME_MS * 3 + 1_000
                - until.saturating_duration_since(Instant::now()).as_millis() as u64,
            String::from_utf8_lossy(&output.stderr)
        );
        let read = success(&output);
        assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
        reads += 1;
        std::thread::sleep(Duration::from_millis(250));
    }
    assert!(reads >= 3, "{reads} reads");
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// When the adapter child last wrote one of its logs, in epoch milliseconds.
fn last_written_ms(container: &Container, log: &str) -> u64 {
    container
        .path("/work")
        .join(log)
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// The identity probes one read ran, when it was admitted `age_ms` (within
/// `TOLERANCE_MS`) after evidence of a 20 s lifetime was collected (its
/// `valid_until_ms` less the lifetime).
///
/// The CLI process takes a host-dependent time between its launch and the
/// owner's admission of the read, so the launch cannot say what age the read
/// was admitted at. The adapter child's own logs do: a read that probed
/// began that probe at its admission (the `validations` log's last write), and
/// one that did not was dispatched after it (the `invocations` log's last
/// write), so the age it was dispatched at bounds the age it was admitted at.
/// A first read on fresh evidence measures the launch-to-dispatch time, and
/// the second is launched that much before its age. A run whose admission
/// missed the age is repeated, so a loaded host cannot fail it; a read that
/// never lands near the age fails the test.
fn probes_of_a_read_at(age_ms: u64) -> usize {
    const LIFETIME_MS: u64 = 20_000;
    const TOLERANCE_MS: u64 = 500;
    const ATTEMPTS: usize = 6;
    let mut missed = Vec::new();
    let (container, connection, mut revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    for _ in 0..ATTEMPTS {
        let revalidated = success(&container.revalidate(&connection, &revision));
        revision = revalidated["connection"]["summary"]["revision"]
            .as_str()
            .unwrap_or_else(|| panic!("{revalidated}"))
            .to_owned();
        let valid_until = revalidated["connection"]["valid_until_ms"]
            .as_u64()
            .unwrap_or_else(|| panic!("{revalidated}"));
        let collected = valid_until - LIFETIME_MS;
        let launched = now_ms();
        success(&container.invoke(&connection));
        let cost = last_written_ms(&container, "invocations").saturating_sub(launched);
        std::thread::sleep(Duration::from_millis(
            (collected + age_ms)
                .saturating_sub(cost)
                .saturating_sub(now_ms()),
        ));
        let validations = container.seen("validations").len();
        let read = success(&container.invoke(&connection));
        assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
        let probes = container.seen("validations").len() - validations;
        let admitted = last_written_ms(
            &container,
            if probes == 0 {
                "invocations"
            } else {
                "validations"
            },
        )
        .saturating_sub(collected);
        if admitted.abs_diff(age_ms) < TOLERANCE_MS {
            return probes;
        }
        missed.push(admitted);
    }
    panic!(
        "no read was admitted within {TOLERANCE_MS} ms of {age_ms} ms of the \
         evidence's {LIFETIME_MS} ms in {ATTEMPTS} attempts: {missed:?}"
    );
}

/// The early-lapse window is half the evidence's own lifetime, not half the
/// profile's 60 s bound and not nine tenths of it: evidence the provider gave
/// 20 s, read at 9.5 s, still has more than half of that left, so the read
/// runs no identity probe of its own.
#[test]
fn a_read_at_9_5_s_of_a_20_s_lifetime_probes_nothing() {
    assert_eq!(probes_of_a_read_at(9_500), 0);
}

/// The other side of the half life: the same evidence read at 10.5 s has
/// under half its lifetime left, so the read recollects it once first.
#[test]
fn a_read_at_10_5_s_of_a_20_s_lifetime_recollects_once() {
    assert_eq!(probes_of_a_read_at(10_500), 1);
}

/// A read whose evidence lapsed recollects it once; when the provider refuses
/// the credential in that recollection, the read is not dispatched, answers
/// as a stored credential refused on a read (`repair_connection`), and the
/// connection is cut off: the provider refused its credential, so it is `stale`.
#[test]
fn a_read_whose_recollection_is_refused_answers_repair() {
    const LIFETIME_MS: u64 = 1_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    success(&container.revalidate(&connection, &revision));
    std::thread::sleep(Duration::from_millis(LIFETIME_MS + 200));
    container.set_knob("refuse", 1);
    let invocations = container.seen("invocations").len();
    let validations = container.seen("validations").len();
    let (code, data) = refusal(&container.invoke(&connection));
    assert_eq!(
        (
            code,
            &data["code"],
            &data["service_code"],
            &data["next_action"]
        ),
        (
            Some(1),
            &serde_json::json!("service_failure"),
            &serde_json::json!("unauthorized"),
            &serde_json::json!("repair_connection")
        ),
        "{data}"
    );
    assert_eq!(container.seen("validations").len(), validations + 1);
    assert_eq!(container.seen("invocations").len(), invocations);
    let status = success(&container.run(&[
        "connections",
        "status",
        "--adapter",
        ALIAS,
        "--connection",
        &connection,
    ]));
    assert_eq!(
        status["connection"]["summary"]["state"], "stale",
        "{status}"
    );
}

/// A read's recollection never re-pins changed material: when the mounted
/// item's bytes were rotated while the evidence lapsed, the read is refused
/// naming a revalidation, the new bytes reach neither the probe nor the
/// provider, and only `connections revalidate` re-pins them.
#[test]
fn a_read_recollection_never_repins_rotated_mounted_material() {
    const LIFETIME_MS: u64 = 1_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    success(&container.revalidate(&connection, &revision));
    std::thread::sleep(Duration::from_millis(LIFETIME_MS + 200));
    let rotated: &[u8] = b"fictional-mounted-token-0002";
    container.rotate("..2026_10_15_00_00_00.000000002", rotated);
    let (code, data) = refusal(&container.invoke(&connection));
    assert_eq!(
        (code, &data["code"], &data["next_action"]),
        (
            Some(1),
            &serde_json::json!("custody_unavailable"),
            &serde_json::json!("revalidate_connection")
        ),
        "{data}"
    );
    assert!(!container.seen("validations").contains(&digest(rotated)));
    assert!(!container.seen("invocations").contains(&digest(rotated)));
    // The re-pinned bytes' evidence lasts the 60 s default: the read after it
    // is about the new bytes, not about another lapse.
    container.set_knob("evidence_ms", 60_000);
    success(&container.revalidate(&connection, &revision));
    assert_eq!(container.seen("validations").last(), Some(&digest(rotated)));
    let read = success(&container.invoke(&connection));
    assert_eq!(read["result"]["credential"], digest(rotated), "{read}");
}

/// Acceptance 2: two revalidations of one connection started together both
/// answer `ready`, each within the admission bound. The adapter's identity
/// probe takes 18 s, more than half the bound (the downstream report has a live
/// revalidation answering after 24.5 s), and the evidence lifetime is the 60 s
/// default: two revalidations queued one behind the other, each running its
/// own probe, would need 36 s, so only the second joining the first answers
/// both in time, and the provider sees one probe for the pair
/// (contracts/cli/v1alpha1/owner.md).
#[test]
fn concurrent_revalidations_both_answer_ready_within_the_admission_bound() {
    const PROBE: Duration = Duration::from_secs(18);
    assert!(2 * PROBE > ADMISSION_BOUND && PROBE < ADMISSION_BOUND);
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", 60_000);
    container.set_knob("probe_ms", PROBE.as_millis() as u64);
    let probes = container.seen("validations").len();
    let timed = || {
        let started = Instant::now();
        let output = container.revalidate(&connection, &revision);
        (started.elapsed(), output)
    };
    let answers = std::thread::scope(|scope| {
        let first = scope.spawn(timed);
        let second = scope.spawn(timed);
        [first.join().unwrap(), second.join().unwrap()]
    });
    for (elapsed, output) in &answers {
        assert!(
            output.status.success(),
            "a concurrent revalidation was refused after {elapsed:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let answer = success(output);
        assert_eq!(
            answer["connection"]["summary"]["state"], "ready",
            "{answer}"
        );
        assert!(
            *elapsed < ADMISSION_BOUND,
            "a concurrent revalidation answered after {elapsed:?}"
        );
    }
    assert_eq!(
        container.seen("validations").len() - probes,
        1,
        "the second revalidation ran its own probe instead of joining the first"
    );
}
