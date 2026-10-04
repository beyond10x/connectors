//! Absolute lease decisions for the trusted local supervisor.
//!
//! The clock port must supply qualified bounds on the issuing authority's
//! timeline, including uncertainty and freshness, in milliseconds. A binding
//! issuing and checking leases in one process may use correlated local elapsed
//! time, provided it proves the real-time cutoff including rate error and suspend;
//! its timestamp labels are not independent UTC evidence. Cross-clock bindings
//! require their own qualified translation. Neither the clock nor the lease is
//! accepted from MCP input. Every I/O boundary must check a fresh decision; a
//! returned `Allow` is not a reusable grant. The enclosing supervisor applies
//! denied decisions to the generated Session reducer in the same serialized step.
pub use session_model::{
    primitives::Timestamp,
    sessions::{DataLease, GateDecision},
};
use std::io;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const NS_PER_MS: i128 = 1_000_000;
const LEASE_NS: i128 = 2_000 * NS_PER_MS;

// Derived numeric caches for exact comparisons, not an authored parallel model.
struct Current {
    value: DataLease,
    issued: i128,
    expiry: i128,
}
impl Current {
    fn parse(value: DataLease) -> Option<Self> {
        let issued = instant(&value.issued_at)?;
        let expiry = instant(&value.effective_expiry)?;
        if value.sequence <= 0 || expiry <= issued || expiry - issued > LEASE_NS {
            return None;
        }
        Some(Self {
            value,
            issued,
            expiry,
        })
    }
}
fn instant(value: &Timestamp) -> Option<i128> {
    OffsetDateTime::parse(&value.0, &Rfc3339)
        .ok()
        .map(|value| value.unix_timestamp_nanos())
}

/// Local authorization latch. There is deliberately no reset/reattach operation.
/// No Clone or wire deserializer can manufacture a second live gate from this one.
pub struct LeaseGate<C> {
    clock: C,
    last: Option<(i128, i128)>,
    current: Option<Current>,
    denied: Option<GateDecision>,
    drain: Option<(Timestamp, i128)>,
}
impl<C: FnMut() -> Option<(i64, i64)>> LeaseGate<C> {
    pub fn admit(value: DataLease, clock: C) -> Result<Self, GateDecision> {
        let mut gate = Self {
            clock,
            last: None,
            current: None,
            denied: None,
            drain: None,
        };
        let now = gate.observe().ok_or(GateDecision::Unverified)?;
        let current = Current::parse(value).ok_or(GateDecision::Unverified)?;
        if current.issued > now.0 {
            return Err(GateDecision::Unverified);
        }
        gate.current = Some(current);
        let decision = gate.check_at(now);
        if decision == GateDecision::Allow {
            Ok(gate)
        } else {
            Err(decision)
        }
    }

    /// Observe the current lease, without extending it for data or progress.
    pub fn decision(&mut self) -> GateDecision {
        if let Some(denied) = self.denied {
            return denied;
        }
        match self.observe() {
            Some(now) => self.check_at(now),
            None => self.deny(GateDecision::Unverified),
        }
    }

    /// The old lease is checked first: even a newly issued lease cannot revive it.
    /// Authority/binding authentication precedes this method at the host boundary.
    pub fn renew(&mut self, value: DataLease) -> GateDecision {
        if let Some(denied) = self.denied {
            return denied;
        }
        let Some(now) = self.observe() else {
            return self.deny(GateDecision::Unverified);
        };
        let decision = self.check_at(now);
        if decision != GateDecision::Allow {
            return decision;
        }
        let Some(mut candidate) = Current::parse(value) else {
            return self.deny(GateDecision::Unverified);
        };
        let previous = self
            .current
            .as_ref()
            .expect("allow retains a current lease");
        if candidate.value.sequence <= previous.value.sequence
            || candidate.issued < previous.issued
            || candidate.issued > now.0
        {
            return self.deny(GateDecision::Unverified);
        }
        if let Some((timestamp, deadline)) = &self.drain
            && candidate.expiry > *deadline
        {
            candidate.expiry = *deadline;
            candidate.value.effective_expiry = timestamp.clone();
        }
        if candidate.expiry <= now.1 {
            return self.deny(GateDecision::Expired);
        }
        self.current = Some(candidate);
        GateDecision::Allow
    }

    pub fn revoke(&mut self) -> GateDecision {
        self.deny(GateDecision::Revoked)
    }

    /// Planned drain refuses a shorter notice instead of silently weakening its
    /// bound. Earlier selected drains cannot be extended. Closing new-offer
    /// admission and proving actual local resource release are enclosing duties.
    pub fn begin_drain(&mut self, deadline: Timestamp) -> io::Result<()> {
        if self.denied.is_some() {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let Some(now) = self.observe() else {
            self.deny(GateDecision::Unverified);
            return Err(io::ErrorKind::BrokenPipe.into());
        };
        if self.check_at(now) != GateDecision::Allow {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let at = instant(&deadline).ok_or(io::ErrorKind::InvalidInput)?;
        if at < now.1 + LEASE_NS {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if self
            .drain
            .as_ref()
            .is_some_and(|(_, previous)| *previous <= at)
        {
            return Ok(());
        }
        if let Some(current) = &mut self.current
            && current.expiry > at
        {
            current.expiry = at;
            current.value.effective_expiry = deadline.clone();
        }
        self.drain = Some((deadline, at));
        Ok(())
    }

    /// Passive observation, never proof that the lease is live at a later read.
    pub fn lease(&self) -> Option<&DataLease> {
        self.current.as_ref().map(|current| &current.value)
    }

    fn observe(&mut self) -> Option<(i128, i128)> {
        let (lower, upper) = (self.clock)()?;
        let now = (i128::from(lower) * NS_PER_MS, i128::from(upper) * NS_PER_MS);
        if now.0 > now.1
            || now.1 - now.0 >= LEASE_NS
            || self
                .last
                .is_some_and(|last| now.0 < last.0 || now.1 < last.1)
        {
            return None;
        }
        self.last = Some(now);
        Some(now)
    }
    fn check_at(&mut self, now: (i128, i128)) -> GateDecision {
        let Some(current) = &self.current else {
            return self.deny(GateDecision::Unverified);
        };
        if self
            .drain
            .as_ref()
            .is_some_and(|(_, deadline)| *deadline <= current.expiry && now.1 >= *deadline)
        {
            return self.deny(GateDecision::Revoked);
        }
        if now.1 >= current.expiry {
            return self.deny(GateDecision::Expired);
        }
        GateDecision::Allow
    }
    fn deny(&mut self, decision: GateDecision) -> GateDecision {
        self.current = None;
        *self.denied.get_or_insert(decision)
    }
}
