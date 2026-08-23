//! S-045 acceptance evidence over the SQLite backend.
//!
//! The approval gate's one-time redemption rests on the port invariant that a bound-breaking
//! `append` refuses without mutating; the conformance suite pins the invariant, and these tests
//! prove the property the story actually demands on top of it: N genuinely concurrent identical
//! presentations spend the approval exactly once, a replay survives reopening the database file,
//! and a crash between redemption and terminal write leaves a durable attempted row that the
//! startup recovery scan settles.

use std::path::Path;
use std::sync::{Arc, Barrier};

use connector_state::StateStore;
use domain::{
    ApprovalAuditKind, ApprovalError, ApprovalGate, ApprovalInvocation, ApprovalOutcome,
    ApprovalRecord, APPROVAL_AUDIT_STATE_KEY,
};
use state_sqlite::SqliteState;

const ISSUER: &str = "identity.example";

fn record() -> ApprovalRecord {
    ApprovalRecord {
        reference: "apr-c9d1".to_owned(),
        issuer: ISSUER.to_owned(),
        subject: "user:operator-1".to_owned(),
        operation: "kubernetes.rollout_restart".to_owned(),
        connection: "conn-prod-cluster".to_owned(),
        input_digest: "sha256:4b5c6d".to_owned(),
        expires_at_seconds: 2_000,
    }
}

fn invocation() -> ApprovalInvocation {
    ApprovalInvocation {
        subject: "user:operator-1".to_owned(),
        operation: "kubernetes.rollout_restart".to_owned(),
        connection: "conn-prod-cluster".to_owned(),
        input_digest: "sha256:4b5c6d".to_owned(),
        now_seconds: 1_000,
    }
}

fn gate_over(path: &Path) -> ApprovalGate {
    let store: Arc<dyn StateStore> = Arc::new(SqliteState::open(path).expect("the database opens"));
    ApprovalGate::new(store, ISSUER)
}

fn journal_kinds(path: &Path) -> Vec<ApprovalAuditKind> {
    let store = SqliteState::open(path).expect("the database opens");
    let Some(journal) = store
        .read(APPROVAL_AUDIT_STATE_KEY, 8 * 1024 * 1024)
        .expect("the journal reads")
    else {
        return Vec::new();
    };
    journal
        .split(|byte| *byte == b'\n')
        .filter(|row| !row.is_empty())
        .map(|row| {
            let row: serde_json::Value = serde_json::from_slice(row).expect("a JSON journal row");
            ApprovalAuditKind::parse(row["kind"].as_str().expect("a kind")).expect("a known kind")
        })
        .collect()
}

fn count(kinds: &[ApprovalAuditKind], kind: ApprovalAuditKind) -> usize {
    kinds.iter().filter(|candidate| **candidate == kind).count()
}

#[test]
fn sixteen_concurrent_identical_presentations_redeem_exactly_once() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("approvals.db");
    let store: Arc<dyn StateStore> =
        Arc::new(SqliteState::open(&path).expect("the database opens"));
    let gate = Arc::new(ApprovalGate::new(store, ISSUER));

    let presenters = 16;
    let barrier = Arc::new(Barrier::new(presenters));
    let outcomes: Vec<Result<_, ApprovalError>> = (0..presenters)
        .map(|_| {
            let gate = Arc::clone(&gate);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                // Every presenter reaches the gate before any is allowed to race for the claim.
                barrier.wait();
                gate.redeem(&record(), &invocation())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|presenter| presenter.join().expect("presenter thread"))
        .collect();

    let spent = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    let replayed = outcomes
        .iter()
        .filter(|outcome| **outcome == Err(ApprovalError::Replayed))
        .count();
    assert_eq!(
        (spent, replayed),
        (1, presenters - 1),
        "exactly one of {presenters} concurrent identical presentations proceeds"
    );

    // The winner concludes; the journal then accounts for every presentation.
    let winner = outcomes
        .into_iter()
        .find_map(Result::ok)
        .expect("the one redemption");
    gate.conclude(winner, ApprovalOutcome::Completed, 1_001)
        .expect("the terminal row lands");
    let kinds = journal_kinds(&path);
    assert_eq!(count(&kinds, ApprovalAuditKind::Attempted), presenters);
    assert_eq!(count(&kinds, ApprovalAuditKind::Replayed), presenters - 1);
    assert_eq!(count(&kinds, ApprovalAuditKind::Completed), 1);
    assert_eq!(
        gate.recover(1_002).expect("the scan runs"),
        [],
        "nothing is left unresolved"
    );
}

#[test]
fn a_replay_survives_reopening_the_database() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("approvals.db");
    gate_over(&path)
        .redeem(&record(), &invocation())
        .expect("the first presentation redeems");

    // A different process lifetime: fresh connection, same file.
    let reopened = gate_over(&path);
    assert_eq!(
        reopened.redeem(&record(), &invocation()),
        Err(ApprovalError::Replayed),
        "the spend is durable, not a property of the process that made it"
    );
    assert_eq!(count(&journal_kinds(&path), ApprovalAuditKind::Replayed), 1);
}

#[test]
fn a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("approvals.db");
    let reference_sha256 = {
        // The "process" that crashes: it redeems — dispatch may have fired — and is dropped
        // without ever writing a terminal row.
        let crashed = gate_over(&path);
        let redemption = crashed
            .redeem(&record(), &invocation())
            .expect("the presentation redeems");
        redemption.reference_sha256().to_owned()
    };

    // Before any recovery: the attempted row is already durable in the reopened file, because it
    // landed in the same atomic append as the redemption claim and was anchored in the journal
    // before dispatch could begin.
    let kinds = journal_kinds(&path);
    assert_eq!(count(&kinds, ApprovalAuditKind::Attempted), 1);
    assert_eq!(kinds.len(), 1, "and no outcome row exists yet");

    // Startup after the crash: the scan finds the redeemed-but-nonterminal presentation.
    let restarted = gate_over(&path);
    let recovered = restarted.recover(1_100).expect("the scan runs");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].reference_sha256, reference_sha256);
    assert_eq!(recovered[0].resolution, ApprovalAuditKind::Indeterminate);
    assert_eq!(recovered[0].unresolved, 1);
    let kinds = journal_kinds(&path);
    assert_eq!(count(&kinds, ApprovalAuditKind::Indeterminate), 1);

    // The settlement is durable and idempotent, and the approval stays spent.
    assert_eq!(restarted.recover(1_101).expect("the scan runs again"), []);
    assert_eq!(
        restarted.redeem(&record(), &invocation()),
        Err(ApprovalError::Replayed),
        "recovery never un-spends an approval"
    );
}
