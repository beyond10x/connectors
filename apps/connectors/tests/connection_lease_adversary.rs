//! Adversarial cases for story:connection-lease-holds-unattended, driven from
//! its outcome and from contracts/cli/v1alpha1/semantics.md and owner.md,
//! against the production CLI, owner and an adapter child under the read-only
//! `mounted_secret` binding (`support/mounted.rs`). Linux only: the jail is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use std::time::{Duration, Instant};

const ADAPTER_FIXTURE: &str = "connection_lease_adversary_adapter_fixture";

/// The bound a `connections revalidate` answers within (its owner deadline).
const ADMISSION_BOUND: Duration = Duration::from_secs(30);

/// The adapter child the owner starts. The ordinary run does nothing.
#[test]
fn connection_lease_adversary_adapter_fixture() {
    serve_when_child();
}

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

/// The story's outcome: two concurrent revalidations of one connection both
/// finish within the admission bound. The story's own report has a live
/// revalidation answering `ready` after 24.5 s. Here the identity probe takes
/// 16 s: one revalidation alone answers `ready` well within the bound (the
/// control), but two started together queue on the instance's serialized
/// worker, each running its own probe, so the second needs about 32 s and its
/// deadline, which covers its queueing, passes first. The story's expected
/// shape (a second revalidation joins the first) would answer both.
#[test]
fn adversary_two_revalidations_with_a_slow_probe_both_answer_ready_within_the_bound() {
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", 60_000);
    container.set_knob("probe_ms", 16_000);
    let timed = || {
        let started = Instant::now();
        let output = container.revalidate(&connection, &revision);
        (started.elapsed(), output)
    };
    // Control: one revalidation with this probe fits the bound.
    let (alone, output) = timed();
    assert!(
        output.status.success() && alone < ADMISSION_BOUND,
        "one revalidation alone did not fit the bound ({alone:?}): {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
}

/// The story's outcome: a revalidated connection keeps serving reads "for as
/// long as its evidence and credentials allow". Evidence of 20 s, read 11 s
/// after its revalidation, is still current for about 9 s, which is plenty for
/// this read, but under half its lifetime is left, so the read recollects it
/// early. When the provider is briefly unavailable for that optional
/// recollection, the read is refused although the evidence it was captured on
/// still allows it; before this change the same read was served.
#[test]
fn adversary_a_read_on_current_evidence_is_served_when_its_early_recollection_fails_transiently() {
    const LIFETIME_MS: u64 = 20_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    success(&container.revalidate(&connection, &revision));
    let revalidated = Instant::now();
    std::thread::sleep(Duration::from_millis(LIFETIME_MS / 2 + 1_000));
    container.set_knob("unavailable", 1);
    let validations = container.seen("validations").len();
    let output = container.invoke(&connection);
    // The read began with its evidence still current.
    assert!(revalidated.elapsed() < Duration::from_millis(LIFETIME_MS - 2_000));
    // The early recollection was attempted: the window was reached.
    assert!(
        container.seen("validations").len() > validations,
        "the read never attempted its early recollection"
    );
    assert!(
        output.status.success(),
        "a read on still-current evidence was refused because its optional \
         early recollection failed transiently: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let read = success(&output);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
}

/// contracts/cli/v1alpha1/owner.md: a read's recollection of lapsed evidence
/// is "this same recollection" a revalidation runs, and "a revalidation that
/// arrives while one of the same connection, at the same expected semantic
/// revision, is queued or running on the owner joins it". ess/domains/cli.yaml
/// names both causes of one `LocalRevalidationAdmission`. A read finds its
/// evidence lapsed and recollects it with an 18 s probe (the story's live probe
/// took 24.5 s); a `connections revalidate` arriving meanwhile queues behind
/// the read's job and then runs a second probe of its own, so it needs about
/// 36 s and its 30 s deadline passes first.
#[test]
fn adversary_a_revalidation_during_a_reads_recollection_joins_it_within_the_bound() {
    const LIFETIME_MS: u64 = 1_000;
    const PROBE_MS: u64 = 18_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    success(&container.revalidate(&connection, &revision));
    std::thread::sleep(Duration::from_millis(LIFETIME_MS + 200));
    container.set_knob("evidence_ms", 60_000);
    container.set_knob("probe_ms", PROBE_MS);
    let probes = container.seen("validations").len();
    let (read, (elapsed, revalidated)) = std::thread::scope(|scope| {
        let read = scope.spawn(|| container.invoke(&connection));
        // The read's job is on the worker, probing, when the revalidation arrives.
        std::thread::sleep(Duration::from_millis(3_000));
        let started = Instant::now();
        let revalidated = container.revalidate(&connection, &revision);
        (read.join().unwrap(), (started.elapsed(), revalidated))
    });
    let read = success(&read);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    assert!(
        revalidated.status.success() && elapsed < ADMISSION_BOUND,
        "a revalidation arriving during a read's recollection answered after \
         {elapsed:?}: {}",
        String::from_utf8_lossy(&revalidated.stderr)
    );
    let answer = success(&revalidated);
    assert_eq!(answer["connection"]["summary"]["state"], "ready", "{answer}");
    assert_eq!(
        container.seen("validations").len() - probes,
        1,
        "the revalidation ran its own probe instead of joining the read's recollection"
    );
}

/// contracts/cli/v1alpha1/semantics.md: a recollection run only because
/// still-current evidence would lapse early "keeps the read admitted on that
/// evidence: when it fails with anything but such a credential refusal (the
/// provider or custody briefly unavailable, a timeout), the read is served on
/// the evidence it was admitted on, for as long as that evidence stays
/// current". Evidence of 20 s, read 11 s after its revalidation, has about 9 s
/// left. The provider's identity probe now hangs (40 s). The optional early
/// recollection is bounded by the read's deadline (at most 30 s), not by the
/// evidence it was admitted on, so it outlives that evidence and the read,
/// which a provider serving reads could have answered on its current evidence,
/// is lost to the probe.
#[test]
fn adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence() {
    const LIFETIME_MS: u64 = 20_000;
    let (container, connection, revision) = connected();
    container.set_knob("evidence_ms", LIFETIME_MS);
    success(&container.revalidate(&connection, &revision));
    let revalidated = Instant::now();
    std::thread::sleep(Duration::from_millis(LIFETIME_MS / 2 + 1_000));
    container.set_knob("probe_ms", 40_000);
    let validations = container.seen("validations").len();
    let invocations = container.seen("invocations").len();
    let started = Instant::now();
    let output = container.invoke(&connection);
    let elapsed = started.elapsed();
    // The read began with its evidence still current, under half of it left.
    assert!(started.duration_since(revalidated) < Duration::from_millis(LIFETIME_MS - 4_000));
    assert!(
        container.seen("validations").len() > validations,
        "the read never attempted its early recollection"
    );
    assert!(
        output.status.success(),
        "a read on still-current evidence was lost to its optional early \
         recollection after {elapsed:?} (dispatched reads: {}): {}",
        container.seen("invocations").len() - invocations,
        String::from_utf8_lossy(&output.stderr)
    );
    let read = success(&output);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
}
