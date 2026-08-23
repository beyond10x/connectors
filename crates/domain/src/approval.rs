//! Approval verification and one-time redemption (design 13, S-045).
//!
//! Operations whose description demands approval need more than a Grant decision: an externally
//! issued approval record must be verified against the exact invocation and then *spent*. This
//! module owns both halves. Verification checks issuer, subject, operation, Connection, canonical
//! input digest and expiry, and refuses without naming which axis refused — a presenter probing a
//! stolen record learns nothing about how close it was. Redemption spends the record exactly once
//! over the S-041 state port, so every backend a deployment can bind (memory, SQLite, PostgreSQL)
//! enforces the same one-time semantics rather than each growing its own.
//!
//! # The transaction shape
//!
//! The state port is keyed byte cells, deliberately without cross-key transactions. The atomic
//! unit here is therefore one bounded [`StateStore::append`] to the presented approval's own
//! redemption cell, whose payload **is** the attempted-audit row:
//!
//! ```text
//! append(approval.redemption.<sha256(reference)>, attempted-row, bound = row length)
//! ```
//!
//! With the bound equal to the payload length, the append can only succeed while the cell has
//! never been written — the conformance suite pins that a bound-breaking append refuses **without
//! mutating** — so one write is simultaneously the redemption claim and the durable attempted
//! record. There is no interleaving of concurrent presenters in which two claims both succeed,
//! and no window in which the approval is spent but its attempted record is not durable.
//!
//! # The journal and the crash windows
//!
//! The shared audit journal ([`APPROVAL_AUDIT_STATE_KEY`]) is the readable trail: every
//! presentation appends an [`attempted`](ApprovalAuditKind::Attempted) anchor there *before* the
//! claim, and its outcome row — [`refused`](ApprovalAuditKind::Refused),
//! [`replayed`](ApprovalAuditKind::Replayed), or the terminal
//! [`completed`](ApprovalAuditKind::Completed)/[`failed`](ApprovalAuditKind::Failed) written after
//! dispatch — follows. Because the anchor precedes the claim and the claim precedes dispatch, an
//! attempted row is durable before any effect can fire; a store that cannot take the anchor
//! refuses dispatch instead.
//!
//! A crash can still separate anchor, claim and terminal row. [`ApprovalGate::recover`] is the
//! startup scan that reconciles: an anchored attempt with no outcome whose redemption cell exists
//! is [`indeterminate`](ApprovalAuditKind::Indeterminate) — the approval was spent and dispatch
//! may have fired — and one whose cell is absent is [`aborted`](ApprovalAuditKind::Aborted) —
//! nothing was spent, nothing dispatched, and the approval remains redeemable.
//!
//! S-046 wires this gate into the hosted route; nothing here dispatches, maps refusals to status
//! codes, or reads a clock — the caller supplies its own `now`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use connector_state::{StateError, StateStore};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// The shared approval audit journal's state cell: newline-delimited JSON rows.
pub const APPROVAL_AUDIT_STATE_KEY: &str = "approval.audit";

/// Every redemption cell lives under this prefix, keyed by the reference's SHA-256 hex.
const REDEMPTION_KEY_PREFIX: &str = "approval.redemption.";

/// Journal ceiling for `attempted` anchors. Once the journal is this full, new presentations
/// refuse as unavailable rather than dispatch unaudited.
const MAX_AUDIT_BYTES: usize = 4 * 1024 * 1024;

/// Outcome rows append under `MAX_AUDIT_BYTES + OUTCOME_HEADROOM_BYTES`: an attempt the journal
/// accepted must always have room for its outcome, so the headroom is reserved for rows that
/// resolve attempts and is never reachable by new anchors. Rows are well under a kilobyte, so this
/// covers far more in-flight attempts than the ceiling can admit between anchor and outcome. The
/// per-attempt byte reservation some Integration audit journals already keep is the fuller
/// pattern and can replace this constant without changing any caller.
const OUTCOME_HEADROOM_BYTES: usize = 64 * 1024;

/// Per-field byte bound, so one presentation cannot bloat the journal or a redemption cell.
const MAX_FIELD_BYTES: usize = 512;

/// A redemption cell holds exactly one attempted row; reads use this generous bound.
const MAX_REDEMPTION_CELL_BYTES: usize = 16 * 1024;

/// An externally issued approval record, exactly as presented. Plain data, not a proof: nothing
/// is trusted until [`ApprovalGate::redeem`] verifies and spends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    /// The one-time reference the issuer minted. It never lands in a state key or an audit row —
    /// both carry its SHA-256 — so the journal cannot be read back into a presentable reference.
    pub reference: String,
    pub issuer: String,
    pub subject: String,
    pub operation: String,
    pub connection: String,
    /// Digest of the canonical invocation input the approval was granted for.
    pub input_digest: String,
    /// The record is redeemable strictly before this instant.
    pub expires_at_seconds: u64,
}

/// The invocation an approval is being presented for, described by the Connector itself — never
/// by the presenter's claims about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalInvocation {
    pub subject: String,
    pub operation: String,
    pub connection: String,
    pub input_digest: String,
    /// Caller-supplied clock, in seconds. The gate reads no clock of its own.
    pub now_seconds: u64,
}

/// Why a presentation did not yield a redemption.
///
/// [`Refused`](Self::Refused) and [`Replayed`](Self::Replayed) render identically on purpose: the
/// axis that refused is never named, and whether a reference was already spent is not disclosed to
/// the presenter either. The variants stay distinct because the journal and S-046's metrics treat
/// replay as its own event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ApprovalError {
    /// Verification refused — deliberately axis-free.
    #[error("approval was refused")]
    Refused,
    /// The same reference was already redeemed.
    #[error("approval was refused")]
    Replayed,
    /// The state port refused; nothing about the presentation was decided.
    #[error("approval store is unavailable")]
    Unavailable,
}

/// Closed set of journal row kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApprovalAuditKind {
    /// A presentation was anchored before any claim; the dispatch attempt this row precedes may
    /// or may not have gone on to redeem.
    Attempted,
    /// Verification refused the presentation.
    Refused,
    /// The reference was already spent — replay, distinct from every other refusal.
    Replayed,
    /// Terminal: dispatch completed.
    Completed,
    /// Terminal: dispatch failed.
    Failed,
    /// Recovery: the presentation died before any redemption, so nothing dispatched and the
    /// approval remains redeemable.
    Aborted,
    /// Recovery: the approval was redeemed but no terminal row exists — dispatch may have fired.
    Indeterminate,
}

impl ApprovalAuditKind {
    /// Stable journal token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Attempted => "attempted",
            Self::Refused => "refused",
            Self::Replayed => "replayed",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
            Self::Indeterminate => "indeterminate",
        }
    }

    /// The inverse of [`as_str`](Self::as_str); `None` for a token this build does not know.
    #[must_use]
    pub fn parse(kind: &str) -> Option<Self> {
        [
            Self::Attempted,
            Self::Refused,
            Self::Replayed,
            Self::Completed,
            Self::Failed,
            Self::Aborted,
            Self::Indeterminate,
        ]
        .into_iter()
        .find(|candidate| candidate.as_str() == kind)
    }
}

/// Terminal outcome of the dispatch a redemption admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Completed,
    Failed,
}

impl ApprovalOutcome {
    const fn kind(self) -> ApprovalAuditKind {
        match self {
            Self::Completed => ApprovalAuditKind::Completed,
            Self::Failed => ApprovalAuditKind::Failed,
        }
    }
}

/// Proof that one approval was verified against one invocation and spent, exactly once.
///
/// Fields are private, there is no public constructor and the type is deliberately not `Clone`:
/// holding one *is* the evidence that [`ApprovalGate::redeem`] won the one-time claim, and
/// [`ApprovalGate::conclude`] consumes it so a redemption cannot terminate twice. This mirrors
/// [`crate::GrantDecision`]'s seal.
#[derive(Debug, PartialEq, Eq)]
pub struct ApprovalRedemption {
    reference_sha256: String,
    issuer: String,
    subject: String,
    operation: String,
    connection: String,
    input_digest: String,
    redeemed_at_seconds: u64,
}

impl ApprovalRedemption {
    /// SHA-256 hex of the spent reference — the journal's name for this redemption.
    #[must_use]
    pub fn reference_sha256(&self) -> &str {
        &self.reference_sha256
    }

    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    #[must_use]
    pub fn connection(&self) -> &str {
        &self.connection
    }

    #[must_use]
    pub fn input_digest(&self) -> &str {
        &self.input_digest
    }

    #[must_use]
    pub fn redeemed_at_seconds(&self) -> u64 {
        self.redeemed_at_seconds
    }
}

/// One unresolved presentation [`ApprovalGate::recover`] reconciled at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredPresentation {
    /// SHA-256 hex of the presented reference.
    pub reference_sha256: String,
    /// [`Indeterminate`](ApprovalAuditKind::Indeterminate) when the approval was redeemed and no
    /// terminal row exists — dispatch may have fired. [`Aborted`](ApprovalAuditKind::Aborted)
    /// when every unresolved presentation died before redeeming — nothing dispatched.
    pub resolution: ApprovalAuditKind,
    /// How many anchored attempts this resolution row settles.
    pub unresolved: usize,
}

/// The approval authority: verifies externally issued records and spends each exactly once.
pub struct ApprovalGate {
    store: Arc<dyn StateStore>,
    issuer: String,
}

impl ApprovalGate {
    /// A gate that accepts approvals from exactly one `issuer`, redeeming them in `store`.
    pub fn new(store: Arc<dyn StateStore>, issuer: impl Into<String>) -> Self {
        Self {
            store,
            issuer: issuer.into(),
        }
    }

    /// Verify `record` against `invocation` and spend it.
    ///
    /// On success the returned [`ApprovalRedemption`] is the caller's license to dispatch, and the
    /// attempted-audit row is already durable — it is the redemption cell's own payload, landed in
    /// the same atomic append as the claim (see the module documentation for the transaction
    /// shape). After dispatch, hand the proof to [`conclude`](Self::conclude).
    ///
    /// # Errors
    ///
    /// - [`ApprovalError::Refused`] — any verification mismatch or expiry; the axis is never
    ///   named, and a `refused` journal row is written.
    /// - [`ApprovalError::Replayed`] — the reference was already spent; a `replayed` journal row
    ///   is written.
    /// - [`ApprovalError::Unavailable`] — the journal or the redemption cell could not be
    ///   written. Nothing may dispatch, because no attempted row is durable.
    pub fn redeem(
        &self,
        record: &ApprovalRecord,
        invocation: &ApprovalInvocation,
    ) -> Result<ApprovalRedemption, ApprovalError> {
        let reference_sha256 = sha256_hex(record.reference.as_bytes());
        if !within_field_bounds(record, invocation) {
            // Too large to journal faithfully; refuse without an anchor, but still on the record.
            self.append_outcome(
                ApprovalAuditKind::Refused,
                &reference_sha256,
                invocation.now_seconds,
                None,
            )?;
            return Err(ApprovalError::Refused);
        }
        let attempted = attempted_row(&reference_sha256, record, invocation)?;
        self.append_journal(&attempted, MAX_AUDIT_BYTES)?;
        if !self.verifies(record, invocation) {
            self.append_outcome(
                ApprovalAuditKind::Refused,
                &reference_sha256,
                invocation.now_seconds,
                None,
            )?;
            return Err(ApprovalError::Refused);
        }
        // The one-time claim: bound == payload length makes this append succeed only on a cell
        // that has never been written, and the payload is the attempted-audit row itself.
        match self.store.append(
            &redemption_key(&reference_sha256),
            &attempted,
            attempted.len(),
        ) {
            Ok(_) => Ok(ApprovalRedemption {
                reference_sha256,
                issuer: record.issuer.clone(),
                subject: record.subject.clone(),
                operation: record.operation.clone(),
                connection: record.connection.clone(),
                input_digest: record.input_digest.clone(),
                redeemed_at_seconds: invocation.now_seconds,
            }),
            Err(StateError::Capacity) => {
                self.append_outcome(
                    ApprovalAuditKind::Replayed,
                    &reference_sha256,
                    invocation.now_seconds,
                    None,
                )?;
                Err(ApprovalError::Replayed)
            }
            Err(_) => Err(ApprovalError::Unavailable),
        }
    }

    /// Write the terminal outcome row for a dispatch this gate admitted.
    ///
    /// Consumes the proof: one redemption, one terminal row.
    ///
    /// # Errors
    ///
    /// [`ApprovalError::Unavailable`] when the journal cannot take the row; the redemption stays
    /// spent either way, and [`recover`](Self::recover) will settle it as indeterminate.
    pub fn conclude(
        &self,
        redemption: ApprovalRedemption,
        outcome: ApprovalOutcome,
        now_seconds: u64,
    ) -> Result<(), ApprovalError> {
        self.append_outcome(
            outcome.kind(),
            &redemption.reference_sha256,
            now_seconds,
            None,
        )
    }

    /// The startup crash-recovery scan: settle every anchored attempt that has no outcome row.
    ///
    /// For each such presentation the redemption cell decides the truth. Present → the approval
    /// was spent and dispatch may have fired between redemption and terminal write; an
    /// `indeterminate` row is appended (unless a terminal row already settled the spender, in
    /// which case the leftover anchors belonged to presentations that died before claiming and are
    /// `aborted`). Absent → no redemption happened, nothing dispatched, the approval remains
    /// redeemable; an `aborted` row is appended. Running the scan again returns nothing: the rows
    /// it appends are themselves the settlement.
    ///
    /// # Errors
    ///
    /// [`ApprovalError::Unavailable`] when the journal cannot be read, holds a row this build
    /// cannot parse, or cannot take a settlement row.
    pub fn recover(&self, now_seconds: u64) -> Result<Vec<RecoveredPresentation>, ApprovalError> {
        #[derive(Default)]
        struct Tally {
            attempted: usize,
            resolved: usize,
            terminal: bool,
        }
        let Some(journal) = self
            .store
            .read(
                APPROVAL_AUDIT_STATE_KEY,
                MAX_AUDIT_BYTES + OUTCOME_HEADROOM_BYTES,
            )
            .map_err(|_| ApprovalError::Unavailable)?
        else {
            return Ok(Vec::new());
        };
        let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
        for row in journal.split(|byte| *byte == b'\n') {
            if row.is_empty() {
                continue;
            }
            let row: serde_json::Value =
                serde_json::from_slice(row).map_err(|_| ApprovalError::Unavailable)?;
            let kind = row
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .and_then(ApprovalAuditKind::parse)
                .ok_or(ApprovalError::Unavailable)?;
            let reference_sha256 = row
                .get("approval_sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or(ApprovalError::Unavailable)?;
            let count = match row.get("count") {
                None => 1,
                Some(count) => count
                    .as_u64()
                    .and_then(|count| usize::try_from(count).ok())
                    .ok_or(ApprovalError::Unavailable)?,
            };
            let tally = tallies.entry(reference_sha256.to_owned()).or_default();
            match kind {
                ApprovalAuditKind::Attempted => tally.attempted += count,
                ApprovalAuditKind::Refused
                | ApprovalAuditKind::Replayed
                | ApprovalAuditKind::Aborted => tally.resolved += count,
                ApprovalAuditKind::Completed
                | ApprovalAuditKind::Failed
                | ApprovalAuditKind::Indeterminate => {
                    tally.resolved += count;
                    tally.terminal = true;
                }
            }
        }
        let mut recovered = Vec::new();
        for (reference_sha256, tally) in tallies {
            let unresolved = tally.attempted.saturating_sub(tally.resolved);
            if unresolved == 0 {
                continue;
            }
            let redeemed = self
                .store
                .read(
                    &redemption_key(&reference_sha256),
                    MAX_REDEMPTION_CELL_BYTES,
                )
                .map_err(|_| ApprovalError::Unavailable)?
                .is_some();
            let resolution = if redeemed && !tally.terminal {
                ApprovalAuditKind::Indeterminate
            } else {
                ApprovalAuditKind::Aborted
            };
            self.append_outcome(
                resolution,
                &reference_sha256,
                now_seconds,
                (unresolved > 1).then_some(unresolved),
            )?;
            recovered.push(RecoveredPresentation {
                reference_sha256,
                resolution,
                unresolved,
            });
        }
        Ok(recovered)
    }

    /// Every check, one boolean: callers translate `false` into the one axis-free refusal.
    fn verifies(&self, record: &ApprovalRecord, invocation: &ApprovalInvocation) -> bool {
        let populated = !record.reference.is_empty()
            && !record.issuer.is_empty()
            && !record.subject.is_empty()
            && !record.operation.is_empty()
            && !record.connection.is_empty()
            && !record.input_digest.is_empty();
        populated
            && record.issuer == self.issuer
            && record.subject == invocation.subject
            && record.operation == invocation.operation
            && record.connection == invocation.connection
            && record.input_digest == invocation.input_digest
            && invocation.now_seconds < record.expires_at_seconds
    }

    fn append_journal(&self, row: &[u8], maximum: usize) -> Result<(), ApprovalError> {
        self.store
            .append(APPROVAL_AUDIT_STATE_KEY, row, maximum)
            .map(|_| ())
            .map_err(|_| ApprovalError::Unavailable)
    }

    fn append_outcome(
        &self,
        kind: ApprovalAuditKind,
        reference_sha256: &str,
        at_seconds: u64,
        count: Option<usize>,
    ) -> Result<(), ApprovalError> {
        let row = journal_row(&OutcomeRow {
            kind: kind.as_str(),
            approval_sha256: reference_sha256,
            at: at_seconds,
            count,
        })?;
        // Outcome rows draw on the reserved headroom: an attempt the journal accepted must always
        // be able to resolve.
        self.append_journal(&row, MAX_AUDIT_BYTES + OUTCOME_HEADROOM_BYTES)
    }

    /// Anchor an attempt and stop — the crash window between anchor and claim, for tests.
    #[cfg(test)]
    fn crash_after_anchor(
        &self,
        record: &ApprovalRecord,
        invocation: &ApprovalInvocation,
    ) -> Result<(), ApprovalError> {
        let reference_sha256 = sha256_hex(record.reference.as_bytes());
        let attempted = attempted_row(&reference_sha256, record, invocation)?;
        self.append_journal(&attempted, MAX_AUDIT_BYTES)
    }
}

#[derive(Serialize)]
struct AttemptRow<'a> {
    kind: &'a str,
    approval_sha256: &'a str,
    issuer: &'a str,
    subject: &'a str,
    operation: &'a str,
    connection: &'a str,
    input_digest: &'a str,
    at: u64,
}

#[derive(Serialize)]
struct OutcomeRow<'a> {
    kind: &'a str,
    approval_sha256: &'a str,
    at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
}

/// The attempted-audit row: which approval was presented (by digest), who issued it, and the
/// invocation as the Connector described it — not as the presenter claimed it.
fn attempted_row(
    reference_sha256: &str,
    record: &ApprovalRecord,
    invocation: &ApprovalInvocation,
) -> Result<Vec<u8>, ApprovalError> {
    journal_row(&AttemptRow {
        kind: ApprovalAuditKind::Attempted.as_str(),
        approval_sha256: reference_sha256,
        issuer: &record.issuer,
        subject: &invocation.subject,
        operation: &invocation.operation,
        connection: &invocation.connection,
        input_digest: &invocation.input_digest,
        at: invocation.now_seconds,
    })
}

fn journal_row<T: Serialize>(row: &T) -> Result<Vec<u8>, ApprovalError> {
    let mut bytes = serde_json::to_vec(row).map_err(|_| ApprovalError::Unavailable)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn redemption_key(reference_sha256: &str) -> String {
    format!("{REDEMPTION_KEY_PREFIX}{reference_sha256}")
}

fn within_field_bounds(record: &ApprovalRecord, invocation: &ApprovalInvocation) -> bool {
    [
        record.reference.len(),
        record.issuer.len(),
        record.subject.len(),
        record.operation.len(),
        record.connection.len(),
        record.input_digest.len(),
        invocation.subject.len(),
        invocation.operation.len(),
        invocation.connection.len(),
        invocation.input_digest.len(),
    ]
    .into_iter()
    .all(|length| length <= MAX_FIELD_BYTES)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use connector_state::MemoryState;

    use super::*;

    const ISSUER: &str = "identity.example";

    fn record() -> ApprovalRecord {
        ApprovalRecord {
            reference: "apr-7f3a".to_owned(),
            issuer: ISSUER.to_owned(),
            subject: "user:operator-1".to_owned(),
            operation: "kubernetes.rollout_restart".to_owned(),
            connection: "conn-prod-cluster".to_owned(),
            input_digest: "sha256:1f2e3d".to_owned(),
            expires_at_seconds: 2_000,
        }
    }

    fn invocation() -> ApprovalInvocation {
        ApprovalInvocation {
            subject: "user:operator-1".to_owned(),
            operation: "kubernetes.rollout_restart".to_owned(),
            connection: "conn-prod-cluster".to_owned(),
            input_digest: "sha256:1f2e3d".to_owned(),
            now_seconds: 1_000,
        }
    }

    fn gate(store: &Arc<MemoryState>) -> ApprovalGate {
        ApprovalGate::new(Arc::clone(store) as Arc<dyn StateStore>, ISSUER)
    }

    fn journal_kinds(store: &MemoryState) -> Vec<ApprovalAuditKind> {
        let Some(journal) = store
            .read(
                APPROVAL_AUDIT_STATE_KEY,
                MAX_AUDIT_BYTES + OUTCOME_HEADROOM_BYTES,
            )
            .expect("journal reads")
        else {
            return Vec::new();
        };
        journal
            .split(|byte| *byte == b'\n')
            .filter(|row| !row.is_empty())
            .map(|row| {
                let row: serde_json::Value = serde_json::from_slice(row).expect("a JSON row");
                ApprovalAuditKind::parse(row["kind"].as_str().expect("a kind"))
                    .expect("a known kind")
            })
            .collect()
    }

    #[test]
    fn a_matching_presentation_redeems_and_the_proof_carries_the_facts() {
        let store = Arc::new(MemoryState::new());
        let redemption = gate(&store)
            .redeem(&record(), &invocation())
            .expect("a matching presentation redeems");
        assert_eq!(redemption.issuer(), ISSUER);
        assert_eq!(redemption.subject(), "user:operator-1");
        assert_eq!(redemption.operation(), "kubernetes.rollout_restart");
        assert_eq!(redemption.connection(), "conn-prod-cluster");
        assert_eq!(redemption.input_digest(), "sha256:1f2e3d");
        assert_eq!(redemption.redeemed_at_seconds(), 1_000);
        assert_eq!(
            redemption.reference_sha256(),
            sha256_hex(b"apr-7f3a"),
            "the journal's name for the redemption is the reference digest"
        );
    }

    #[test]
    fn the_redemption_cell_is_the_attempted_audit_row() {
        // The transaction shape itself: the one atomic append that spends the approval carries the
        // attempted-audit row as its payload, so neither can exist without the other.
        let store = Arc::new(MemoryState::new());
        let redemption = gate(&store)
            .redeem(&record(), &invocation())
            .expect("redeems");
        let cell = store
            .read(
                &redemption_key(redemption.reference_sha256()),
                MAX_REDEMPTION_CELL_BYTES,
            )
            .expect("cell reads")
            .expect("the claim wrote the cell");
        let expected = attempted_row(redemption.reference_sha256(), &record(), &invocation())
            .expect("the row serializes");
        assert_eq!(cell, expected, "cell payload == attempted-audit row");
        assert_eq!(
            journal_kinds(&store),
            [ApprovalAuditKind::Attempted],
            "and the journal anchor was written before the claim, with no outcome yet"
        );
    }

    #[test]
    fn every_verification_failure_refuses_without_naming_the_axis() {
        let expired = ApprovalRecord {
            expires_at_seconds: 1_000, // == now: redeemable strictly before expiry
            ..record()
        };
        let wrong_issuer = ApprovalRecord {
            issuer: "someone-else.example".to_owned(),
            ..record()
        };
        let wrong_subject = ApprovalRecord {
            subject: "user:operator-2".to_owned(),
            ..record()
        };
        let wrong_operation = ApprovalRecord {
            operation: "kubernetes.scale".to_owned(),
            ..record()
        };
        let wrong_connection = ApprovalRecord {
            connection: "conn-staging-cluster".to_owned(),
            ..record()
        };
        let wrong_digest = ApprovalRecord {
            input_digest: "sha256:aaaaaa".to_owned(),
            ..record()
        };
        let empty_reference = ApprovalRecord {
            reference: String::new(),
            ..record()
        };
        for (axis, mismatched) in [
            ("expiry", expired),
            ("issuer", wrong_issuer),
            ("subject", wrong_subject),
            ("operation", wrong_operation),
            ("connection", wrong_connection),
            ("input digest", wrong_digest),
            ("empty reference", empty_reference),
        ] {
            let store = Arc::new(MemoryState::new());
            let refusal = gate(&store)
                .redeem(&mismatched, &invocation())
                .expect_err(axis);
            assert_eq!(refusal, ApprovalError::Refused, "{axis}");
            assert_eq!(
                refusal.to_string(),
                ApprovalError::Refused.to_string(),
                "one refusal text for every axis, `{axis}` included"
            );
            assert_eq!(
                journal_kinds(&store),
                [ApprovalAuditKind::Attempted, ApprovalAuditKind::Refused],
                "a `{axis}` refusal is still anchored and journaled"
            );
            let reference_sha256 = sha256_hex(mismatched.reference.as_bytes());
            assert_eq!(
                store
                    .read(
                        &redemption_key(&reference_sha256),
                        MAX_REDEMPTION_CELL_BYTES
                    )
                    .expect("cell reads"),
                None,
                "a `{axis}` refusal spends nothing"
            );
        }
    }

    #[test]
    fn a_refusal_does_not_spend_the_approval() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        let wrong_digest = ApprovalInvocation {
            input_digest: "sha256:other".to_owned(),
            ..invocation()
        };
        assert_eq!(
            gate.redeem(&record(), &wrong_digest),
            Err(ApprovalError::Refused)
        );
        gate.redeem(&record(), &invocation())
            .expect("the record is still redeemable by the matching invocation");
    }

    #[test]
    fn a_second_presentation_refuses_and_audits_as_replay() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        gate.redeem(&record(), &invocation()).expect("first spends");
        let replay = gate
            .redeem(&record(), &invocation())
            .expect_err("second refuses");
        assert_eq!(replay, ApprovalError::Replayed);
        assert_eq!(
            replay.to_string(),
            ApprovalError::Refused.to_string(),
            "the presenter is not told the reference was already spent"
        );
        assert_eq!(
            journal_kinds(&store),
            [
                ApprovalAuditKind::Attempted,
                ApprovalAuditKind::Attempted,
                ApprovalAuditKind::Replayed
            ],
            "replay is its own journal kind"
        );
    }

    #[test]
    fn a_mismatched_presentation_of_a_spent_approval_reads_as_refused_not_replay() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        gate.redeem(&record(), &invocation()).expect("spends");
        let mismatched = ApprovalInvocation {
            input_digest: "sha256:other".to_owned(),
            ..invocation()
        };
        assert_eq!(
            gate.redeem(&record(), &mismatched),
            Err(ApprovalError::Refused),
            "verification refuses before the spent state is ever consulted"
        );
    }

    #[test]
    fn conclude_writes_the_terminal_row_and_consumes_the_proof() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        let redemption = gate.redeem(&record(), &invocation()).expect("redeems");
        gate.conclude(redemption, ApprovalOutcome::Completed, 1_001)
            .expect("the terminal row lands");
        assert_eq!(
            journal_kinds(&store),
            [ApprovalAuditKind::Attempted, ApprovalAuditKind::Completed]
        );
        // `redemption` is moved: a second terminal row for the same proof does not compile.
    }

    #[test]
    fn an_oversized_presentation_refuses_and_is_still_on_the_record() {
        let store = Arc::new(MemoryState::new());
        let oversized = ApprovalRecord {
            subject: "s".repeat(MAX_FIELD_BYTES + 1),
            ..record()
        };
        assert_eq!(
            gate(&store).redeem(&oversized, &invocation()),
            Err(ApprovalError::Refused)
        );
        assert_eq!(
            journal_kinds(&store),
            [ApprovalAuditKind::Refused],
            "no anchor for a row the journal cannot carry faithfully, but the refusal is written"
        );
    }

    #[test]
    fn an_unwritable_journal_refuses_before_any_dispatch_could_go_unaudited() {
        struct AnchorlessStore(MemoryState);
        impl StateStore for AnchorlessStore {
            fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError> {
                self.0.read(key, maximum)
            }
            fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError> {
                self.0.replace(key, body, maximum)
            }
            fn append(
                &self,
                key: &str,
                suffix: &[u8],
                maximum: usize,
            ) -> Result<usize, StateError> {
                if key == APPROVAL_AUDIT_STATE_KEY {
                    return Err(StateError::Unavailable);
                }
                self.0.append(key, suffix, maximum)
            }
            fn delete(&self, key: &str) -> Result<(), StateError> {
                self.0.delete(key)
            }
        }
        let store: Arc<dyn StateStore> = Arc::new(AnchorlessStore(MemoryState::new()));
        let gate = ApprovalGate::new(Arc::clone(&store), ISSUER);
        assert_eq!(
            gate.redeem(&record(), &invocation()),
            Err(ApprovalError::Unavailable),
            "no durable attempted row, no dispatch"
        );
        let reference_sha256 = sha256_hex(record().reference.as_bytes());
        assert_eq!(
            store
                .read(
                    &redemption_key(&reference_sha256),
                    MAX_REDEMPTION_CELL_BYTES
                )
                .expect("cell reads"),
            None,
            "and nothing was spent"
        );
    }

    #[test]
    fn recovery_settles_a_redeemed_but_nonterminal_presentation_as_indeterminate() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        let redemption = gate.redeem(&record(), &invocation()).expect("redeems");
        let reference_sha256 = redemption.reference_sha256().to_owned();
        drop(redemption); // the crash: dispatch may have fired, the terminal row never landed

        let recovered = gate.recover(1_050).expect("the scan runs");
        assert_eq!(
            recovered,
            [RecoveredPresentation {
                reference_sha256,
                resolution: ApprovalAuditKind::Indeterminate,
                unresolved: 1,
            }]
        );
        assert_eq!(
            journal_kinds(&store),
            [
                ApprovalAuditKind::Attempted,
                ApprovalAuditKind::Indeterminate
            ]
        );
        assert_eq!(
            gate.recover(1_051).expect("the scan runs again"),
            [],
            "the settlement row is itself what makes the scan idempotent"
        );
    }

    #[test]
    fn recovery_settles_a_crash_before_redemption_as_aborted_and_the_approval_stays_redeemable() {
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        gate.crash_after_anchor(&record(), &invocation())
            .expect("the anchor lands");

        let recovered = gate.recover(1_050).expect("the scan runs");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].resolution, ApprovalAuditKind::Aborted);
        gate.redeem(&record(), &invocation())
            .expect("nothing was spent, so the approval still redeems exactly once");
        assert_eq!(
            gate.redeem(&record(), &invocation()),
            Err(ApprovalError::Replayed),
            "...exactly once"
        );
    }

    #[test]
    fn recovery_distinguishes_dead_losers_from_the_settled_winner() {
        // Winner redeemed and concluded; one loser journaled its replay; a second loser crashed
        // after its anchor. The leftover anchor must settle as aborted — the winner's terminal row
        // proves the spend was accounted for, so the dead presentation cannot have dispatched.
        let store = Arc::new(MemoryState::new());
        let gate = gate(&store);
        let redemption = gate.redeem(&record(), &invocation()).expect("winner");
        assert_eq!(
            gate.redeem(&record(), &invocation()),
            Err(ApprovalError::Replayed),
            "journaled loser"
        );
        gate.crash_after_anchor(&record(), &invocation())
            .expect("dead loser's anchor");
        gate.conclude(redemption, ApprovalOutcome::Completed, 1_002)
            .expect("winner concludes");

        let recovered = gate.recover(1_050).expect("the scan runs");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].resolution, ApprovalAuditKind::Aborted);
        assert_eq!(recovered[0].unresolved, 1);
    }

    #[test]
    fn concurrent_identical_presentations_redeem_exactly_once_in_memory() {
        let store = Arc::new(MemoryState::new());
        let gate = Arc::new(gate(&store));
        let presenters = 8;
        let barrier = Arc::new(Barrier::new(presenters));
        let outcomes: Vec<Result<ApprovalRedemption, ApprovalError>> = (0..presenters)
            .map(|_| {
                let gate = Arc::clone(&gate);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
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
        assert_eq!((spent, replayed), (1, presenters - 1));
    }

    #[test]
    fn journal_kind_tokens_round_trip_and_are_closed() {
        for kind in [
            ApprovalAuditKind::Attempted,
            ApprovalAuditKind::Refused,
            ApprovalAuditKind::Replayed,
            ApprovalAuditKind::Completed,
            ApprovalAuditKind::Failed,
            ApprovalAuditKind::Aborted,
            ApprovalAuditKind::Indeterminate,
        ] {
            assert_eq!(ApprovalAuditKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(ApprovalAuditKind::parse("granted"), None);
    }

    #[test]
    fn the_redemption_key_obeys_the_state_key_grammar() {
        let key = redemption_key(&sha256_hex(b"any reference at all, any bytes \xff\x00"));
        connector_state::validate_key(&key).expect("prefix plus 64 hex characters is a valid key");
    }
}
