//! Same-process lease issuance and data gating. No authority arrives over stdin.
use super::*;
use connectors_mcp::{
    lease::LeaseGate,
    supervision::{self as model, obligations::*, *},
};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

// At the maximum configured 1% rate error, 1900 clock milliseconds plus two
// resolution milliseconds are below 2000 real milliseconds. No receipt grace.
const LEASE_MS: i64 = 1900;
struct Clock {
    boot: u64,
    offset: i128,
    epoch: i64,
}
fn ticks(clock: libc::clockid_t) -> Option<u64> {
    let mut stamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: stamp is a valid writable timespec; callers use fixed clock ids.
    if unsafe { libc::clock_gettime(clock, &mut stamp) } != 0
        || stamp.tv_sec < 0
        || !(0..1_000_000_000).contains(&stamp.tv_nsec)
    {
        return None;
    }
    (stamp.tv_sec as u64)
        .checked_mul(1_000_000_000)?
        .checked_add(stamp.tv_nsec as u64)
}
fn sample() -> Option<(u64, i128)> {
    let before = ticks(libc::CLOCK_BOOTTIME)?;
    let mono = ticks(libc::CLOCK_MONOTONIC)?;
    let after = ticks(libc::CLOCK_BOOTTIME)?;
    if after.checked_sub(before)? > 1_000_000 {
        return None;
    }
    Some((before, i128::from(before) - i128::from(mono)))
}
impl Clock {
    fn new() -> Option<Self> {
        for clock in [libc::CLOCK_BOOTTIME, libc::CLOCK_MONOTONIC] {
            let mut resolution = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            // SAFETY: resolution is live and writable for this fixed-clock query.
            if unsafe { libc::clock_getres(clock, &mut resolution) } != 0
                || resolution.tv_sec != 0
                || !(0..=1_000_000).contains(&resolution.tv_nsec)
            {
                return None;
            }
        }
        let (boot, offset) = sample()?;
        let epoch = i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_millis(),
        )
        .ok()?;
        Some(Self {
            boot,
            offset,
            epoch,
        })
    }
    fn observe(&self) -> Option<(i64, i64)> {
        let (boot, offset) = sample()?;
        if (offset - self.offset).abs() > 2_000_000 {
            return None;
        }
        let elapsed = boot.checked_sub(self.boot)?;
        let lower = self
            .epoch
            .checked_add(i64::try_from(elapsed / 1_000_000).ok()?)?;
        Some((lower, lower.checked_add(2)?))
    }
}
fn timestamp(ms: i64) -> Timestamp {
    Timestamp(
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .expect("qualified timestamp range")
            .format(&Rfc3339)
            .expect("RFC3339 timestamp"),
    )
}
type Gate = LeaseGate<Box<dyn FnMut() -> Option<(i64, i64)>>>;
type Reducer = model::Supervision<Box<dyn FnMut() -> Timestamp>, Box<dyn FnMut() -> SessionId>>;
pub(super) struct Session {
    clock: Arc<Clock>,
    gate: Gate,
    reducer: Reducer,
    id: SessionId,
    expiry: i64,
    sequence: i64,
    renew_at: Instant,
    clock_selection: String,
    alias: String,
    connection: String,
    instance: String,
    host: String,
}
impl Session {
    pub fn admit(paths: &Paths, snapshot: &super::projection::Snapshot) -> owner::Result<Self> {
        let config = Config::load(&paths.config)?;
        // Reuse the deployment's explicit local rate assumption. The signed UTC
        // source is not sampled for a lease issued and checked on this one clock.
        let selected = config
            .approval_clock
            .as_ref()
            .ok_or(Code::InvalidConfiguration)?;
        selected
            .validate()
            .map_err(|_| Code::InvalidConfiguration)?;
        let clock = Arc::new(Clock::new().ok_or(Code::Unavailable)?);
        let now = clock.observe().ok_or(Code::Unavailable)?;
        let expiry = now.0 + LEASE_MS;
        let lease = DataLease {
            issued_at: timestamp(now.0),
            effective_expiry: timestamp(expiry),
            sequence: 1,
        };
        let gate_clock = clock.clone();
        let gate = LeaseGate::admit(
            lease.clone(),
            Box::new(move || gate_clock.observe()) as Box<dyn FnMut() -> Option<(i64, i64)>>,
        )
        .map_err(|_| Code::Unavailable)?;
        let record_clock = clock.clone();
        let mut reducer = Supervision::new(
            Box::new(move || timestamp(record_clock.observe().map_or(now.1, |n| n.1)))
                as Box<dyn FnMut() -> Timestamp>,
            Box::new(|| SessionId(uuid::Uuid::new_v4().to_string()))
                as Box<dyn FnMut() -> SessionId>,
        );
        let OfferSessionOutcome::Offered { session_offered } = reducer
            .offer_session(OfferSession {
                binding: Binding {
                    instance_ref: snapshot.bootstrap.instance.clone(),
                    connection_ref: snapshot.connection.clone(),
                    admitted_revision: snapshot.revision.clone(),
                    authority_ref: snapshot.host.clone(),
                },
                profile: Profile::InboundOffer,
                placement: Placement::Local,
                streams: vec!["stdin".into(), "stdout".into()],
            })
            .map_err(|_| Code::Unavailable)?;
        let id = session_offered.session_id;
        reducer
            .accept_offer(AcceptOffer {
                session_id: id.clone(),
            })
            .map_err(|_| Code::Unavailable)?;
        reducer
            .establish_ready(EstablishReady {
                session_id: id.clone(),
                decision: GateDecision::Allow,
                lease,
            })
            .map_err(|_| Code::Unavailable)?;
        Ok(Self {
            clock,
            gate,
            reducer,
            id,
            expiry,
            sequence: 1,
            renew_at: Instant::now() + Duration::from_millis(500),
            clock_selection: selected.sha256(),
            alias: snapshot.alias.clone(),
            connection: snapshot.connection.clone(),
            instance: snapshot.bootstrap.instance.clone(),
            host: snapshot.host.clone(),
        })
    }
    pub fn permit(&mut self, direction: &str) -> owner::Result<()> {
        let decision = self.gate.decision();
        self.reducer
            .permit_data(PermitData {
                session_id: self.id.clone(),
                decision,
                direction: direction.into(),
            })
            .map_err(|_| Code::Unavailable)?;
        if decision == GateDecision::Allow {
            Ok(())
        } else {
            Err(Code::Timeout.into())
        }
    }
    pub fn maintain(&mut self, paths: &Paths) -> owner::Result<()> {
        if self.gate.decision() != GateDecision::Allow {
            return self.permit("stdin");
        }
        if Instant::now() < self.renew_at {
            return Ok(());
        }
        let owner = owner::Client::connect_until(paths, false, self.until()?)?;
        if owner.host_incarnation != self.host {
            self.gate.revoke();
            return Err(Code::Unavailable.into());
        }
        let config = Config::load(&paths.config)?;
        let projection = load(paths)?;
        if config
            .approval_clock
            .as_ref()
            .is_none_or(|c| c.sha256() != self.clock_selection)
            || config
                .adapters
                .get(&self.alias)
                .is_none_or(|a| a.instance_id != self.instance)
            || projection
                .document()
                .exposures
                .iter()
                .any(|e| e.adapter_alias != self.alias || e.connection_ref != self.connection)
        {
            self.gate.revoke();
            return Err(Code::Forbidden.into());
        }
        let now = self.clock.observe().ok_or(Code::Unavailable)?;
        self.sequence = self.sequence.checked_add(1).ok_or(Code::Unavailable)?;
        let expiry = now.0 + LEASE_MS;
        let lease = DataLease {
            issued_at: timestamp(now.0),
            effective_expiry: timestamp(expiry),
            sequence: self.sequence,
        };
        let decision = self.gate.renew(lease.clone());
        self.reducer
            .renew_data_lease(RenewDataLease {
                session_id: self.id.clone(),
                decision,
                lease,
            })
            .map_err(|_| Code::Unavailable)?;
        if decision != GateDecision::Allow {
            return Err(Code::Timeout.into());
        }
        self.expiry = expiry;
        self.renew_at = Instant::now() + Duration::from_millis(500);
        Ok(())
    }
    pub fn until(&mut self) -> owner::Result<Instant> {
        let start = Instant::now();
        self.permit("stdin")?;
        let (_, upper) = self.clock.observe().ok_or(Code::Unavailable)?;
        let remaining = self
            .expiry
            .checked_sub(upper)
            .filter(|n| *n > 0)
            .ok_or(Code::Timeout)?;
        Ok(start + Duration::from_millis(remaining as u64))
    }
    pub fn begin_close(&mut self, graceful: bool) {
        self.gate.revoke();
        let now = self.clock.observe().map_or(self.expiry, |n| n.1);
        let _ = self.reducer.begin_close(BeginClose {
            session_id: self.id.clone(),
            terminal: TerminalFact {
                reason: if graceful {
                    TerminalReason::LocalClose
                } else {
                    TerminalReason::TransportLost
                },
                by: TerminalActor::Host,
                accepted_at: timestamp(now),
            },
            cutoff_due_at: timestamp(now),
            teardown_due_at: timestamp(now + 5000),
        });
    }
    pub fn finish_teardown(&mut self, released: bool) {
        let _ = self.reducer.finish_teardown(FinishTeardown {
            session_id: self.id.clone(),
            decision: if released {
                CleanupDecision::Released
            } else {
                CleanupDecision::Unaccounted
            },
            peer_shutdown: PeerShutdown::Unconfirmed,
        });
    }
}
