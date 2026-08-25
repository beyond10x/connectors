//! The pending-authorization table: single-use, expiring, and bounded.

use std::collections::HashMap;

use crate::OauthError;

/// Live authorizations one Integration holds before it refuses to start another.
///
/// Every connect session inserts an entry, so an unbounded table is a memory-exhaustion primitive
/// keyed by a caller-triggerable value. Expired entries are swept before the bound is consulted,
/// so a deployment that has merely been running a while never reaches it — only a deployment with
/// 1024 authorizations genuinely in flight does, and that is a flood rather than a workload.
pub const DEFAULT_PENDING_CAPACITY: usize = 1_024;

/// One in-flight authorization, keyed by its `state` value.
pub struct Pending<T> {
    /// Whatever the caller needs on the way back — a session reference, an owner, a PKCE verifier.
    pub payload: T,
    /// When this entry stops being redeemable.
    pub expires_at_unix_ms: u64,
}

/// The in-flight authorizations, keyed by `state`.
///
/// Three properties, each of which was previously re-established per Integration:
///
/// - **Single use.** [`take`](PendingStates::take) removes. A replayed callback finds nothing,
///   which is the difference between a CSRF defence and a decoration.
/// - **Expiring.** [`expire`](PendingStates::expire) drops everything past its instant, and
///   `take` refuses an entry that is merely still present.
/// - **Bounded.** An unbounded map keyed by an attacker-triggerable value is a memory
///   exhaustion primitive: every `create_session` call inserts one, and nothing before this crate
///   capped it. Insert evicts expired entries first and refuses only when the table is full of
///   live ones.
///
/// Generic over the payload because the three callers park different things: GitLab a session
/// reference plus owner plus verifier, Jira and Slack their own shapes.
pub struct PendingStates<T> {
    entries: HashMap<String, Pending<T>>,
    capacity: usize,
}

impl<T> PendingStates<T> {
    /// An empty table holding at most `capacity` live entries.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
        }
    }

    /// Record an in-flight authorization.
    ///
    /// Expired entries are swept first, so a table that has simply been running a while does not
    /// refuse. `now_unix_ms` is what that sweep is measured against.
    ///
    /// # Errors
    ///
    /// [`OauthError::StateCapacity`] when `capacity` live entries remain after the sweep.
    pub fn insert(
        &mut self,
        state: String,
        payload: T,
        expires_at_unix_ms: u64,
        now_unix_ms: u64,
    ) -> Result<(), OauthError> {
        self.expire(now_unix_ms);
        if !self.entries.contains_key(&state) && self.entries.len() >= self.capacity {
            return Err(OauthError::StateCapacity);
        }
        self.entries.insert(
            state,
            Pending {
                payload,
                expires_at_unix_ms,
            },
        );
        Ok(())
    }

    /// Redeem `state`, removing it. Returns `None` if it is unknown or already expired.
    ///
    /// An expired entry is removed rather than left, so a caller cannot distinguish "expired" from
    /// "never existed" by calling twice.
    pub fn take(&mut self, state: &str, now_unix_ms: u64) -> Option<Pending<T>> {
        let entry = self.entries.remove(state)?;
        (now_unix_ms < entry.expires_at_unix_ms).then_some(entry)
    }

    /// Redeem `state` whether or not it has expired, returning the entry with its expiry.
    ///
    /// [`take`](PendingStates::take) is the safe default and the one to reach for. This exists for
    /// a caller that must tell "expired" apart from "never existed" in what it reports — GitLab
    /// answers an expired callback with a refusal rather than a not-found, and tears down the
    /// browser session it belongs to on the way, which it cannot do without the payload.
    ///
    /// The single-use property is unaffected: the entry is gone either way. Only the expiry
    /// judgement moves to the caller, which must then make it.
    pub fn remove(&mut self, state: &str) -> Option<Pending<T>> {
        self.entries.remove(state)
    }

    /// Whether `state` is present at all, expired or not, without redeeming it.
    ///
    /// This is what a backend answers `owns_hosted_oauth_state` with. It deliberately ignores
    /// expiry: ownership decides *which* backend handles the callback, and a backend that
    /// disowns its own expired state leaves the dispatcher with no claimant, so the browser is
    /// told the callback is unknown rather than that it expired. Claim it, then refuse it in the
    /// completion, where [`remove`](PendingStates::remove) hands back the expiry to judge.
    #[must_use]
    pub fn contains_any(&self, state: &str) -> bool {
        self.entries.contains_key(state)
    }

    /// Whether `state` is present and still live, without redeeming it.
    ///
    /// This is what a backend answers `owns_hosted_oauth_state` with: the dispatcher asks before
    /// routing a callback, and exactly one backend must claim it.
    #[must_use]
    pub fn contains(&self, state: &str, now_unix_ms: u64) -> bool {
        self.entries
            .get(state)
            .is_some_and(|entry| now_unix_ms < entry.expires_at_unix_ms)
    }

    /// Drop every entry, live or not.
    ///
    /// For a caller tearing down all of its connect sessions at once — Slack does this when the
    /// workspace configuration is replaced, because every in-flight authorization was started
    /// against the configuration that just went away.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Drop every entry at or past its expiry.
    pub fn expire(&mut self, now_unix_ms: u64) {
        self.entries
            .retain(|_, entry| now_unix_ms < entry.expires_at_unix_ms);
    }

    /// Live and expired entries currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table holds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PendingStates<&'static str> {
        PendingStates::new(4)
    }

    #[test]
    fn a_state_is_redeemable_once() {
        let mut states = table();
        states
            .insert("st".to_owned(), "payload", 100, 0)
            .expect("insert");
        assert_eq!(
            states.take("st", 50).map(|entry| entry.payload),
            Some("payload")
        );
        assert!(
            states.take("st", 50).is_none(),
            "a replayed callback must find nothing"
        );
    }

    #[test]
    fn an_expired_state_is_not_redeemable_and_does_not_linger() {
        let mut states = table();
        states
            .insert("st".to_owned(), "payload", 100, 0)
            .expect("insert");
        assert!(states.take("st", 100).is_none(), "expiry is inclusive");
        assert!(
            states.is_empty(),
            "the expired entry is removed, so a second call cannot tell it apart from unknown"
        );
    }

    #[test]
    fn remove_returns_an_expired_entry_so_a_caller_can_refuse_rather_than_not_find() {
        let mut states = table();
        states
            .insert("st".to_owned(), "payload", 100, 0)
            .expect("insert");
        let entry = states
            .remove("st")
            .expect("expired entries are still returned");
        assert_eq!(entry.expires_at_unix_ms, 100);
        assert!(states.remove("st").is_none(), "still single use");
    }

    #[test]
    fn contains_reports_liveness_without_redeeming() {
        let mut states = table();
        states
            .insert("st".to_owned(), "payload", 100, 0)
            .expect("insert");
        assert!(states.contains("st", 50));
        assert!(!states.contains("st", 100), "expired is not owned");
        assert!(!states.contains("other", 50));
        assert!(
            states.take("st", 50).is_some(),
            "contains must not consume the entry"
        );
    }

    #[test]
    fn contains_any_claims_an_expired_state_so_the_callback_is_refused_not_lost() {
        let mut states = table();
        states
            .insert("st".to_owned(), "payload", 100, 0)
            .expect("insert");
        assert!(!states.contains("st", 200), "not live");
        assert!(
            states.contains_any("st"),
            "ownership must survive expiry, or the dispatcher finds no claimant"
        );
    }

    #[test]
    fn clear_drops_live_entries_too() {
        let mut states = table();
        states
            .insert("st".to_owned(), "a", 1_000, 0)
            .expect("insert");
        states.clear();
        assert!(states.is_empty());
        assert!(!states.contains("st", 0));
    }

    #[test]
    fn expire_drops_only_what_is_past() {
        let mut states = table();
        states
            .insert("early".to_owned(), "a", 100, 0)
            .expect("insert");
        states
            .insert("late".to_owned(), "b", 300, 0)
            .expect("insert");
        states.expire(200);
        assert!(!states.contains("early", 200));
        assert!(states.contains("late", 200));
    }

    #[test]
    fn insert_refuses_once_the_table_is_full_of_live_entries() {
        let mut states = table();
        for index in 0..4 {
            states
                .insert(format!("st{index}"), "payload", 100, 0)
                .expect("insert within capacity");
        }
        assert_eq!(
            states.insert("overflow".to_owned(), "payload", 100, 0),
            Err(OauthError::StateCapacity),
            "an unbounded map keyed by a caller-triggerable value is a memory exhaustion primitive"
        );
    }

    #[test]
    fn insert_sweeps_expired_entries_before_refusing() {
        let mut states = table();
        for index in 0..4 {
            states
                .insert(format!("st{index}"), "payload", 100, 0)
                .expect("insert within capacity");
        }
        states
            .insert("fresh".to_owned(), "payload", 400, 200)
            .expect("the four expired entries are swept, leaving room");
        assert_eq!(states.len(), 1);
    }

    #[test]
    fn replacing_a_live_state_does_not_count_against_capacity() {
        let mut states = table();
        for index in 0..4 {
            states
                .insert(format!("st{index}"), "payload", 100, 0)
                .expect("insert within capacity");
        }
        states
            .insert("st0".to_owned(), "replacement", 100, 0)
            .expect("overwriting a key adds no row");
        assert_eq!(states.len(), 4);
    }
}
