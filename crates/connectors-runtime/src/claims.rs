//! Durable one-time claims for event-triggered replies at the local dispatch seam.
//!
//! S-047 deleted the Slack Integration's `ReplyClaimStore` together with the approval-presence
//! check it rode on: an Integration re-reading `approval_evidence_ref` re-decides admission on a
//! caller-supplied string, and catalog invariant rule 15 now forbids exactly that. The mechanism
//! it carried was still real — one companion mention authorizes one outward reply — and deleting
//! it left the personal placement without it: a retried invoke could post the same Slack reply
//! twice (S-048).
//!
//! This is that mechanism, restored where it honestly belongs: at the dispatch seam, in front of
//! every Integration, keyed on the triggering event rather than decided inside an adapter. The
//! hosted placement spends externally issued approval records one-time in its `ApprovalGate`
//! (design 13); the personal placement spends `event:` references one-time here. Both journal
//! through the S-041 state port, and neither asks an Integration to read evidence.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use connector_state::{StateError, StateStore};
use serde_json::Value;

/// The claims journal never grows past the deleted store's bound.
const MAX_CLAIM_JOURNAL_BYTES: usize = 4 * 1024 * 1024;
/// Refused replays are an audit trail and get the audit journal's bound.
const MAX_REFUSAL_JOURNAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_CLAIMS: usize = 10_000;

/// One refusal or breakage of the one-time claim. Value-free: the reference that failed is the
/// caller's to report, never this journal's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClaimError {
    /// The event has already authorized one reply; this presentation is the second.
    #[error("the triggering event has already authorized one reply")]
    Replayed,
    /// The journal is at capacity; the claim was not taken and the reply must not be sent.
    #[error("the reply-claim journal is full")]
    Capacity,
    /// The journal could not be read or written; the claim was not taken.
    #[error("the reply-claim journal is unavailable")]
    Unavailable,
}

/// Durable exactly-once claims over `event:` evidence references.
///
/// Exactly-once holds through two layers: the in-process set under one lock decides races inside
/// the daemon, and the journal row is durable **before** the claim admits, so a restarted daemon
/// reloads every spent reference. The personal daemon's exclusive state lock
/// (`server::local::LocalOperationDaemon`) guarantees a single process owns the journal, which is
/// what lets the in-process lock be the whole concurrency story.
pub struct EventReplyClaims {
    store: Arc<dyn StateStore>,
    claimed: Mutex<BTreeSet<String>>,
}

impl EventReplyClaims {
    /// Journal of spent references: one strict JSON row per claim, append-only.
    pub const CLAIMS_KEY: &'static str = "local.event-reply-claims";
    /// Journal of refused replays: one strict JSON row per refused second presentation.
    pub const REFUSALS_KEY: &'static str = "local.event-reply-refusals";

    /// Reload every spent reference from the journal.
    ///
    /// A journal this build cannot parse is unknown claim state, and serving over it could send
    /// the reply a spent reference already authorized — so opening refuses instead, the same
    /// posture `ApprovalGate::recover` takes for the hosted approval journal.
    ///
    /// # Errors
    ///
    /// [`ClaimError::Unavailable`] when the journal cannot be read or holds a malformed row.
    pub fn open(store: Arc<dyn StateStore>) -> Result<Self, ClaimError> {
        let bytes = store
            .read(Self::CLAIMS_KEY, MAX_CLAIM_JOURNAL_BYTES)
            .map_err(|_| ClaimError::Unavailable)?
            .unwrap_or_default();
        let text = std::str::from_utf8(&bytes).map_err(|_| ClaimError::Unavailable)?;
        let mut claimed = BTreeSet::new();
        for line in text.lines() {
            if line.is_empty() || claimed.len() >= MAX_CLAIMS {
                return Err(ClaimError::Unavailable);
            }
            let row: Value = serde_json::from_str(line).map_err(|_| ClaimError::Unavailable)?;
            let event_ref = claim_row_event_ref(&row).ok_or(ClaimError::Unavailable)?;
            if !claimable(event_ref) || !claimed.insert(event_ref.to_owned()) {
                return Err(ClaimError::Unavailable);
            }
        }
        Ok(Self {
            store,
            claimed: Mutex::new(claimed),
        })
    }

    /// Spend one `event:` reference, exactly once, durably.
    ///
    /// The journal row is written before the claim admits: a claim that is not yet durable could
    /// be granted again by a restarted daemon, so a journal failure refuses rather than admits.
    /// A second presentation refuses as [`ClaimError::Replayed`] and its refusal is journaled;
    /// the journal write is deliberately not allowed to turn the refusal into an outage, because
    /// the refusal is what keeps the duplicate reply unsent.
    ///
    /// # Errors
    ///
    /// [`ClaimError::Replayed`] for a spent reference, [`ClaimError::Capacity`] for a full
    /// journal, [`ClaimError::Unavailable`] when the claim could not be made durable.
    pub fn claim(&self, event_ref: &str, operation_ref: &str) -> Result<(), ClaimError> {
        if !claimable(event_ref) {
            return Err(ClaimError::Unavailable);
        }
        let now_unix_ms = now_unix_ms().ok_or(ClaimError::Unavailable)?;
        let mut claimed = self.claimed.lock().map_err(|_| ClaimError::Unavailable)?;
        if claimed.contains(event_ref) {
            let row = journal_row(&serde_json::json!({
                "event_ref": event_ref,
                "operation_ref": operation_ref,
                "refused_at_unix_ms": now_unix_ms,
            }));
            let _ = self
                .store
                .append(Self::REFUSALS_KEY, &row, MAX_REFUSAL_JOURNAL_BYTES);
            return Err(ClaimError::Replayed);
        }
        if claimed.len() >= MAX_CLAIMS {
            return Err(ClaimError::Capacity);
        }
        let row = journal_row(&serde_json::json!({
            "event_ref": event_ref,
            "operation_ref": operation_ref,
            "claimed_at_unix_ms": now_unix_ms,
        }));
        self.store
            .append(Self::CLAIMS_KEY, &row, MAX_CLAIM_JOURNAL_BYTES)
            .map_err(|error| match error {
                StateError::Capacity => ClaimError::Capacity,
                StateError::Invalid | StateError::Unavailable => ClaimError::Unavailable,
            })?;
        claimed.insert(event_ref.to_owned());
        Ok(())
    }
}

/// Only a reference the journal can round-trip is claimable: the `event:` scheme this claim is
/// for, printable bytes (the wire grammar), and a length that cannot become the payload.
fn claimable(event_ref: &str) -> bool {
    event_ref.starts_with("event:")
        && event_ref.len() <= 512
        && event_ref.bytes().all(|byte| byte.is_ascii_graphic())
}

/// The `event_ref` of one strict claim row, refusing unknown fields the same way the rest of the
/// journal family does: a row this build does not fully understand is unknown claim state.
fn claim_row_event_ref(row: &Value) -> Option<&str> {
    let row = row.as_object()?;
    if row.len() != 3
        || !row.contains_key("operation_ref")
        || !row.get("claimed_at_unix_ms")?.is_u64()
    {
        return None;
    }
    row.get("event_ref")?.as_str()
}

fn journal_row(row: &Value) -> Vec<u8> {
    let mut line = row.to_string().into_bytes();
    line.push(b'\n');
    line
}

fn now_unix_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use connector_state::MemoryState;
    use state_sqlite::SqliteState;

    use super::*;

    #[test]
    fn a_claim_survives_a_daemon_restart() {
        let store: Arc<dyn StateStore> = Arc::new(MemoryState::new());
        let claims = EventReplyClaims::open(Arc::clone(&store)).unwrap();
        claims
            .claim("event:slack:mention-1", "slack-chat-post-message")
            .unwrap();
        let reopened = EventReplyClaims::open(Arc::clone(&store)).unwrap();
        assert_eq!(
            reopened.claim("event:slack:mention-1", "slack-chat-post-message"),
            Err(ClaimError::Replayed)
        );
        assert!(reopened
            .claim("event:slack:mention-2", "slack-chat-post-message")
            .is_ok());
    }

    #[test]
    fn parallel_presentations_take_exactly_one_claim() {
        let store: Arc<dyn StateStore> = Arc::new(SqliteState::in_memory().unwrap());
        let claims = Arc::new(EventReplyClaims::open(Arc::clone(&store)).unwrap());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let outcomes: Vec<Result<(), ClaimError>> = [(); 8]
            .map(|()| {
                let claims = Arc::clone(&claims);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    claims.claim("event:slack:mention-1", "slack-chat-post-message")
                })
            })
            .into_iter()
            .map(|thread| thread.join().expect("claim thread"))
            .collect();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == Err(ClaimError::Replayed))
                .count(),
            7
        );
        let refusals = store
            .read(EventReplyClaims::REFUSALS_KEY, MAX_REFUSAL_JOURNAL_BYTES)
            .unwrap()
            .unwrap();
        assert_eq!(std::str::from_utf8(&refusals).unwrap().lines().count(), 7);
    }

    #[test]
    fn a_journal_this_build_cannot_parse_refuses_to_open() {
        let store: Arc<dyn StateStore> = Arc::new(MemoryState::new());
        store
            .append(
                EventReplyClaims::CLAIMS_KEY,
                b"{\"event_ref\":\"event:x\",\"operation_ref\":\"op\",\"claimed_at_unix_ms\":1,\"surplus\":true}\n",
                MAX_CLAIM_JOURNAL_BYTES,
            )
            .unwrap();
        assert_eq!(
            EventReplyClaims::open(store).err(),
            Some(ClaimError::Unavailable)
        );
    }

    #[test]
    fn only_an_event_reference_is_claimable() {
        let claims = EventReplyClaims::open(Arc::new(MemoryState::new())).unwrap();
        for reference in ["approval-1", "event:with space", "", &"e".repeat(513)] {
            assert_eq!(
                claims.claim(reference, "slack-chat-post-message"),
                Err(ClaimError::Unavailable),
                "{reference:?}"
            );
        }
        assert!(claims
            .claim("event:slack:mention-1", "slack-chat-post-message")
            .is_ok());
    }
}
