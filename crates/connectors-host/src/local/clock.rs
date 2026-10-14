//! Bounded UTC from a quorum of explicitly configured Roughtime sources under
//! local timer assumptions (docs/design/approval-clock-quorum.md).
//! Acquisition does network I/O; an acquired Clock only reads local timers.
//! Public observations cannot reconstruct the process-bound capability.
#[cfg(test)]
#[path = "../../tests/fixtures/clock/server.rs"]
pub(crate) mod fixture;
mod protocol;
#[cfg(test)]
mod tests;
mod timer;

use super::mutations::{self, Clock, ClockInterval};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    net::{SocketAddr, UdpSocket},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use timer::Stamp;

const MAX_SAFE_MS: u128 = 9_007_199_254_740_991;
const NS_PER_MS: u128 = 1_000_000;
const NS_PER_S: u128 = 1_000_000_000;
/// The one deadline every source shares: sends, receives and verification.
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);
/// contracts/service/clock.md: a source's interval and the combined answer are
/// each at most this wide; the approval proof and the ledger admit the same.
const MAX_WIDTH_MS: u128 = mutations::MAX_CLOCK_WIDTH_MS as u128;
/// The `roughtime-quorum/1` bounds on the number of sources and the quorum.
const SOURCES: std::ops::RangeInclusive<usize> = 2..=8;
pub const FORMAT: &str = "roughtime-quorum/1";
/// The retired single-source format: parsed, never used as a clock.
pub const RETIRED_FORMAT: &str = "roughtime-clock/1";

// A test whose fake clock server does contended work before it replies sets
// its own bound here, so a loaded scheduler cannot spend production's 2 s.
#[cfg(test)]
thread_local! {
    static TEST_QUERY_TIMEOUT: std::cell::Cell<Duration> = const { std::cell::Cell::new(QUERY_TIMEOUT) };
    static TEST_MAX_WIDTH_MS: std::cell::Cell<u128> = const { std::cell::Cell::new(MAX_WIDTH_MS) };
}

#[cfg(test)]
pub(crate) fn set_test_query_timeout(timeout: Duration) {
    TEST_QUERY_TIMEOUT.with(|slot| slot.set(timeout));
}

// The same test also chooses the observation width it accepts.
#[cfg(test)]
pub(crate) fn set_test_max_width_ms(width: u128) {
    TEST_MAX_WIDTH_MS.with(|slot| slot.set(width));
}

#[cfg(test)]
fn max_width_ms() -> u128 {
    TEST_MAX_WIDTH_MS.with(std::cell::Cell::get)
}

#[cfg(not(test))]
fn max_width_ms() -> u128 {
    MAX_WIDTH_MS
}

#[cfg(test)]
fn query_timeout() -> Duration {
    TEST_QUERY_TIMEOUT.with(std::cell::Cell::get)
}

#[cfg(not(test))]
fn query_timeout() -> Duration {
    QUERY_TIMEOUT
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidConfiguration,
    Unavailable,
}
type Result<T> = std::result::Result<T, Failure>;

/// Why a quorum acquisition formed no answer (`connectors.clock.Refusal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Fewer than `quorum` sources verified within the width cap.
    BelowQuorum,
    /// No instant lies in the intervals of `quorum` counted sources.
    Split,
    /// The combined answer is wider than the cap.
    TooImprecise,
}

/// What one configured source contributed (`connectors.clock.SourceResult`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceResult {
    /// Counted; its interval meets the answer.
    Agreed,
    /// Counted; its interval is disjoint from the answer.
    Outlier,
    /// Counted; no answer formed.
    Verified,
    /// No verified reply by the shared deadline, or a send or receive error.
    Unreachable,
    /// A reply that does not parse or verify against this source's key and version.
    Unverified,
    /// Verified, but its own interval is over the cap; not counted.
    TooImprecise,
}

/// One line per configured source, in configuration order
/// (`connectors.clock.SourceReport`). Never the reply, nonce or own interval.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceReport {
    pub index: usize,
    pub address: String,
    pub result: SourceResult,
    /// The signed radius, exactly when the reply verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_ms: Option<u64>,
}

/// The next action a quorum refusal names (contracts/service/clock.md, "Refusals").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextAction {
    CheckConfiguration,
    ChooseClockSource,
    RetryExplicitly,
}
impl NextAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CheckConfiguration => "check_configuration",
            Self::ChooseClockSource => "choose_clock_source",
            Self::RetryExplicitly => "retry_explicitly",
        }
    }
}

/// An acquisition's refusal: its [`Failure`], the combined [`Refusal`] and one
/// line per source when the quorum formed no answer. A local timer,
/// continuity, overflow or calendar refusal has neither.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub failure: Failure,
    pub refusal: Option<Refusal>,
    pub sources: Vec<SourceReport>,
    /// The configured table is the retired `roughtime-clock/1` shape.
    pub retired: bool,
}
impl From<Failure> for Refused {
    fn from(failure: Failure) -> Self {
        Self {
            failure,
            refusal: None,
            sources: Vec::new(),
            retired: false,
        }
    }
}
impl Refused {
    fn quorum(refusal: Refusal, sources: Vec<SourceReport>) -> Self {
        Self {
            failure: Failure::Unavailable,
            refusal: Some(refusal),
            sources,
            retired: false,
        }
    }
    /// `below_quorum` names `check_configuration` if any source is unverified,
    /// else `choose_clock_source` if any is too imprecise, else
    /// `retry_explicitly`; `split` and `too_imprecise` name
    /// `choose_clock_source`; any other refusal `check_configuration`.
    pub fn next_action(&self) -> NextAction {
        let any = |result| self.sources.iter().any(|s| s.result == result);
        match self.refusal {
            Some(Refusal::BelowQuorum) if any(SourceResult::Unverified) => {
                NextAction::CheckConfiguration
            }
            Some(Refusal::BelowQuorum) if any(SourceResult::TooImprecise) => {
                NextAction::ChooseClockSource
            }
            Some(Refusal::BelowQuorum) => NextAction::RetryExplicitly,
            Some(Refusal::Split | Refusal::TooImprecise) => NextAction::ChooseClockSource,
            None => NextAction::CheckConfiguration,
        }
    }
}

/// The one wire version a source speaks (`connectors.clock.RoughtimeVersion`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoughtimeVersion {
    /// RFC 10049's version 0x00000001.
    #[serde(rename = "roughtime-1")]
    Roughtime1,
    /// The draft-19 test version 0x8000000c.
    #[serde(rename = "roughtime-0x8000000c")]
    Draft0x8000000c,
}
impl RoughtimeVersion {
    fn wire(self) -> u32 {
        match self {
            Self::Roughtime1 => 0x0000_0001,
            Self::Draft0x8000000c => 0x8000_000c,
        }
    }
}

/// One configured source (`connectors.clock.Source`): its root key is its
/// trust identity, its numeric address where it is asked.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub address: String,
    pub public_key: String,
    pub version: RoughtimeVersion,
}
impl Source {
    fn resolved(&self) -> Result<Resolved> {
        if self.address.len() > 64 || self.public_key.len() != 44 {
            return Err(Failure::InvalidConfiguration);
        }
        let address: SocketAddr = self
            .address
            .parse()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let key = STANDARD
            .decode(&self.public_key)
            .map_err(|_| Failure::InvalidConfiguration)?;
        if address.port() == 0
            || address.ip().is_unspecified()
            || address.ip().is_multicast()
            || matches!(address, SocketAddr::V4(a) if a.ip().is_broadcast())
            || matches!(address, SocketAddr::V6(a) if a.scope_id() != 0 || a.flowinfo() != 0)
            || address.to_string() != self.address
            || key.len() != 32
            || key.iter().all(|b| *b == 0)
            || STANDARD.encode(&key) != self.public_key
        {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(Resolved {
            address,
            key: key.try_into().map_err(|_| Failure::InvalidConfiguration)?,
            version: self.version.wire(),
        })
    }
}
struct Resolved {
    address: SocketAddr,
    key: [u8; 32],
    version: u32,
}

/// The `roughtime-quorum/1` table (`connectors.clock.Configuration`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub format: String,
    pub quorum: u32,
    /// Deployment assertion covering hardware error and kernel clock adjustment.
    /// Neither a public source sample nor the configured number proves it.
    pub max_rate_error_ppm: u32,
    pub sources: Vec<Source>,
}
impl Configuration {
    fn resolved(&self) -> Result<Vec<Resolved>> {
        let quorum = usize::try_from(self.quorum).map_err(|_| Failure::InvalidConfiguration)?;
        if self.format != FORMAT
            || !(1..=10_000).contains(&self.max_rate_error_ppm)
            || !SOURCES.contains(&self.sources.len())
            || !SOURCES.contains(&quorum)
            || quorum > self.sources.len()
        {
            return Err(Failure::InvalidConfiguration);
        }
        let resolved = self
            .sources
            .iter()
            .map(Source::resolved)
            .collect::<Result<Vec<_>>>()?;
        // Two entries for one address or one key are one source counted twice.
        let addresses: BTreeSet<_> = resolved.iter().map(|s| s.address).collect();
        let keys: BTreeSet<_> = resolved.iter().map(|s| s.key).collect();
        if addresses.len() != resolved.len() || keys.len() != resolved.len() {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(resolved)
    }
    pub fn validate(&self) -> Result<()> {
        self.resolved().map(|_| ())
    }
    /// Covers the whole table, source order included.
    pub fn sha256(&self) -> String {
        connectors_core::digest(&serde_json::json!(self))
    }
    /// One parallel exchange per source; never call while holding a metadata
    /// transaction or a key/policy lease. No host clock modification, DNS,
    /// fallback, or retry occurs.
    pub fn acquire(&self) -> Result<BoundedClock> {
        self.acquire_explained().map_err(|refused| refused.failure)
    }
    /// [`Self::acquire`], naming the combined [`Refusal`] and each source's
    /// result when the quorum is why it refused.
    pub fn acquire_explained(&self) -> std::result::Result<BoundedClock, Refused> {
        let resolved = self.resolved()?;
        let quorum = self.quorum as usize;
        timer::qualify_resolution()?;
        // Sockets are bound and connected before the shared deadline starts.
        let sockets: Vec<Option<UdpSocket>> = resolved.iter().map(|s| connect(s.address)).collect();
        let deadline = Instant::now() + query_timeout();
        let exchanges: Vec<Exchange> = std::thread::scope(|scope| {
            let running: Vec<_> = resolved
                .iter()
                .zip(sockets)
                .map(|(source, socket)| scope.spawn(move || exchange(source, socket, deadline)))
                .collect();
            running
                .into_iter()
                .map(|thread| {
                    thread
                        .join()
                        .unwrap_or(Exchange::Local(Failure::Unavailable))
                })
                .collect()
        });
        let mut reports = Vec::with_capacity(exchanges.len());
        let mut replies = Vec::new();
        for (index, exchange) in exchanges.into_iter().enumerate() {
            let address = self.sources[index].address.clone();
            let (result, radius_ms) = match exchange {
                Exchange::Local(failure) => return Err(failure.into()),
                Exchange::Source(result) => (result, None),
                Exchange::Reply(reply) => {
                    let radius_ms = u64::from(reply.sample.radius) * 1000;
                    replies.push(Counted { index, ..reply });
                    (SourceResult::Verified, Some(radius_ms))
                }
            };
            reports.push(SourceReport {
                index,
                address,
                result,
                radius_ms,
            });
        }
        let now = Stamp::read()?;
        let mut counted = Vec::with_capacity(replies.len());
        for source in replies {
            source.started.continuous_with(now)?;
            let (_, lower, upper) = bounds_ns(&source, now, self.max_rate_error_ppm)?;
            if upper.div_ceil(NS_PER_MS) - lower / NS_PER_MS > max_width_ms() {
                reports[source.index].result = SourceResult::TooImprecise;
            } else {
                counted.push(source);
            }
        }
        if counted.len() < quorum {
            return Err(Refused::quorum(Refusal::BelowQuorum, reports));
        }
        let clock = BoundedClock {
            last_read: AtomicU64::new(
                counted
                    .iter()
                    .map(|s| s.received.monotonic)
                    .max()
                    .unwrap_or_default(),
            ),
            counted,
            quorum,
            reports,
            rate_error_ppm: self.max_rate_error_ppm,
            configuration_sha256: self.sha256(),
            process: std::process::id(),
        };
        match clock.answer_at(now, std::process::id()) {
            Ok(_) => Ok(clock),
            Err(Combined::Split) => Err(Refused::quorum(Refusal::Split, clock.reports)),
            Err(Combined::TooImprecise) => {
                Err(Refused::quorum(Refusal::TooImprecise, clock.reports))
            }
            Err(Combined::Local) => Err(Failure::Unavailable.into()),
        }
    }
}

/// The retired single-source `roughtime-clock/1` table. It is parsed so the
/// rest of the configuration keeps loading, and refused for every clock use.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retired {
    pub format: String,
    pub address: String,
    pub public_key: String,
    pub max_rate_error_ppm: u32,
}

/// The configured `[approval_clock]` table: the quorum, or the retired shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApprovalClock {
    Quorum(Configuration),
    Retired(Retired),
}
impl ApprovalClock {
    /// The configuration-load check: a malformed quorum table or any format
    /// other than the two refuses the whole configuration; the retired shape
    /// does not.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Quorum(configuration) => configuration.validate(),
            Self::Retired(retired) if retired.format == RETIRED_FORMAT => Ok(()),
            Self::Retired(_) => Err(Failure::InvalidConfiguration),
        }
    }
    /// The quorum table every clock use needs; the retired shape refuses
    /// `InvalidConfiguration` (rewrite it as `roughtime-quorum/1`,
    /// docs/local-clock.md).
    pub fn configuration(&self) -> Result<&Configuration> {
        match self {
            Self::Quorum(configuration) => Ok(configuration),
            Self::Retired(_) => Err(Failure::InvalidConfiguration),
        }
    }
    /// Whether the table is the retired `roughtime-clock/1` shape.
    pub fn is_retired(&self) -> bool {
        matches!(self, Self::Retired(_))
    }
    pub fn acquire(&self) -> Result<BoundedClock> {
        self.configuration()?.acquire()
    }
    pub fn acquire_explained(&self) -> std::result::Result<BoundedClock, Refused> {
        match self {
            Self::Quorum(configuration) => configuration.acquire_explained(),
            Self::Retired(_) => Err(Refused {
                retired: true,
                ..Failure::InvalidConfiguration.into()
            }),
        }
    }
}

fn connect(address: SocketAddr) -> Option<UdpSocket> {
    let socket = UdpSocket::bind(if address.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })
    .ok()?;
    socket.connect(address).ok()?;
    Some(socket)
}

/// One source's exchange, on its own thread under the shared deadline.
enum Exchange {
    Reply(Counted),
    Source(SourceResult),
    /// A local timer or randomness failure: the whole acquisition refuses.
    Local(Failure),
}
fn exchange(source: &Resolved, socket: Option<UdpSocket>, deadline: Instant) -> Exchange {
    let unreachable = Exchange::Source(SourceResult::Unreachable);
    let Some(socket) = socket else {
        return unreachable;
    };
    let started = match Stamp::read() {
        Ok(stamp) => stamp,
        Err(failure) => return Exchange::Local(failure),
    };
    let (request, nonce) = match protocol::request(&source.key, source.version) {
        Ok(request) => request,
        Err(failure) => return Exchange::Local(failure),
    };
    let sent = remaining(deadline)
        .ok()
        .and_then(|left| socket.set_write_timeout(Some(left)).ok())
        .and_then(|()| socket.send(&request).ok());
    if sent != Some(request.len()) {
        return unreachable;
    }
    let mut bytes = [0u8; protocol::PACKET_SIZE + 1];
    let Some(n) = remaining(deadline)
        .ok()
        .and_then(|left| socket.set_read_timeout(Some(left)).ok())
        .and_then(|()| socket.recv(&mut bytes).ok())
    else {
        return unreachable;
    };
    let received = match Stamp::read() {
        Ok(stamp) => stamp,
        Err(failure) => return Exchange::Local(failure),
    };
    if remaining(deadline).is_err() {
        return unreachable;
    }
    if let Err(failure) = started.continuous_with(received) {
        return Exchange::Local(failure);
    }
    if received.monotonic < started.monotonic {
        return Exchange::Local(Failure::Unavailable);
    }
    if n > protocol::PACKET_SIZE {
        return Exchange::Source(SourceResult::Unverified);
    }
    let Ok(sample) = protocol::verify(&bytes[..n], &request, &nonce, &source.key, source.version)
    else {
        return Exchange::Source(SourceResult::Unverified);
    };
    if remaining(deadline).is_err() {
        return unreachable;
    }
    Exchange::Reply(Counted {
        index: 0,
        sample,
        started,
        received,
    })
}
fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Failure::Unavailable)
}

/// Why no answer formed at one reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Combined {
    Split,
    TooImprecise,
    /// A local timer, continuity, overflow, validity or calendar refusal.
    Local,
}

/// The quorum rule (docs/design/approval-clock-quorum.md, "Quorum rule") over
/// closed intervals in ns: the hull of the instants at least `quorum`
/// intervals contain, rounded outward to ms, and at most `max_width_ms` wide.
fn quorum_answer(
    intervals: &[(u128, u128)],
    quorum: usize,
    max_width_ms: u128,
) -> std::result::Result<(u128, u128), Combined> {
    // Starts sort before ends at equal values, so touching intervals overlap.
    let mut events: Vec<(u128, bool)> = intervals
        .iter()
        .flat_map(|(lower, upper)| [(*lower, false), (*upper, true)])
        .collect();
    events.sort_unstable();
    let (mut covering, mut lower, mut upper) = (0usize, None, None);
    for (at, end) in events {
        if end {
            if covering == quorum {
                upper = Some(at);
            }
            covering -= 1;
        } else {
            covering += 1;
            if covering == quorum && lower.is_none() {
                lower = Some(at);
            }
        }
    }
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return Err(Combined::Split);
    };
    let (lower, upper) = (lower / NS_PER_MS, upper.div_ceil(NS_PER_MS));
    if upper - lower > max_width_ms {
        return Err(Combined::TooImprecise);
    }
    Ok((lower, upper))
}

/// Safe observation only. It is never accepted as clock evidence on an ingress.
#[derive(Debug, Serialize)]
pub struct Observation {
    pub configuration_sha256: String,
    pub lower_unix_ms: i64,
    pub upper_unix_ms: i64,
    pub sources: Vec<SourceReport>,
}

/// A counted source: its signed sample and its own monotonic stamps, S_i read
/// before its nonce was created and R_i after its reply arrived.
struct Counted {
    index: usize,
    sample: protocol::Sample,
    started: Stamp,
    received: Stamp,
}

/// The answer at one reading: its interval and each counted source's own.
struct Answer {
    interval: ClockInterval,
    sources: Vec<(usize, u128, u128)>,
    lower_ns: u128,
    upper_ns: u128,
}

/// No Clone or Deserialize; a restart/fork must acquire new signed samples.
/// Consumers separately retain current clock-selection and authorization policy.
/// The counted set is fixed at acquisition; no reading adds or drops a source.
pub struct BoundedClock {
    counted: Vec<Counted>,
    quorum: usize,
    reports: Vec<SourceReport>,
    rate_error_ppm: u32,
    configuration_sha256: String,
    process: u32,
    last_read: AtomicU64,
}
impl BoundedClock {
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }
    pub fn observe(&self) -> Result<Observation> {
        let answer = self
            .answer_at(Stamp::read()?, std::process::id())
            .map_err(|_| Failure::Unavailable)?;
        let mut sources = self.reports.clone();
        for (index, lower, upper) in answer.sources {
            sources[index].result = if lower <= answer.upper_ns && upper >= answer.lower_ns {
                SourceResult::Agreed
            } else {
                SourceResult::Outlier
            };
        }
        Ok(Observation {
            configuration_sha256: self.configuration_sha256.clone(),
            lower_unix_ms: answer.interval.lower_unix_ms,
            upper_unix_ms: answer.interval.upper_unix_ms,
            sources,
        })
    }
    fn interval_at(&self, now: Stamp, process: u32) -> Result<ClockInterval> {
        self.answer_at(now, process)
            .map(|answer| answer.interval)
            .map_err(|_| Failure::Unavailable)
    }
    fn answer_at(&self, now: Stamp, process: u32) -> std::result::Result<Answer, Combined> {
        if process != self.process
            || now.monotonic < self.last_read.fetch_max(now.monotonic, Ordering::SeqCst)
        {
            return Err(Combined::Local);
        }
        let mut origin = u128::MAX;
        let mut sources = Vec::with_capacity(self.counted.len());
        for source in &self.counted {
            source
                .started
                .continuous_with(now)
                .map_err(|_| Combined::Local)?;
            let (origin_lower, lower, upper) =
                bounds_ns(source, now, self.rate_error_ppm).map_err(|_| Combined::Local)?;
            origin = origin.min(origin_lower);
            sources.push((source.index, lower, upper));
        }
        let intervals: Vec<_> = sources.iter().map(|(_, l, u)| (*l, *u)).collect();
        let (lower, upper) = quorum_answer(&intervals, self.quorum, max_width_ms())?;
        if upper > MAX_SAFE_MS {
            return Err(Combined::Local);
        }
        // No leap announcement is consumed. Avoid extrapolating across any UTC
        // midnight, including the possible skipped/repeated second around it,
        // from the earliest counted source's origin.
        let day_ms = 86_400_000;
        let origin_ms = origin / NS_PER_MS;
        if origin_ms / day_ms != upper / day_ms
            || origin_ms % day_ms < 1000
            || upper % day_ms >= day_ms - 1000
        {
            return Err(Combined::Local);
        }
        Ok(Answer {
            interval: ClockInterval {
                lower_unix_ms: lower as i64,
                upper_unix_ms: upper as i64,
            },
            sources,
            lower_ns: lower * NS_PER_MS,
            upper_ns: upper * NS_PER_MS,
        })
    }
}
/// A source's interval at `now` before any admission check, in ns: its origin's
/// lower bound (M - A - 3 ms), then l = origin + elapsed_lower(N - R) and
/// u = M + A + 3 ms + elapsed_upper(N - S). Refused once elapsed_upper(N - S)
/// passes 30 s.
fn bounds_ns(source: &Counted, now: Stamp, ppm: u32) -> Result<(u128, u128, u128)> {
    let from_start = now
        .monotonic
        .checked_sub(source.started.monotonic)
        .ok_or(Failure::Unavailable)?;
    let from_receive = now
        .monotonic
        .checked_sub(source.received.monotonic)
        .ok_or(Failure::Unavailable)?;
    let elapsed_upper = elapsed(from_start, ppm, true)?;
    if elapsed_upper > 30 * NS_PER_S {
        return Err(Failure::Unavailable);
    }
    let midpoint = u128::from(source.sample.midpoint) * NS_PER_S;
    let radius = u128::from(source.sample.radius) * NS_PER_S;
    let origin_lower = midpoint
        .checked_sub(radius)
        .and_then(|v| v.checked_sub(3 * NS_PER_MS))
        .ok_or(Failure::Unavailable)?;
    let lower = origin_lower
        .checked_add(elapsed(from_receive, ppm, false)?)
        .ok_or(Failure::Unavailable)?;
    let upper = midpoint
        .checked_add(radius)
        .and_then(|v| v.checked_add(elapsed_upper))
        .and_then(|v| v.checked_add(3 * NS_PER_MS))
        .ok_or(Failure::Unavailable)?;
    Ok((origin_lower, lower, upper))
}
impl Clock for BoundedClock {
    fn now(&self) -> mutations::Result<ClockInterval> {
        Stamp::read()
            .and_then(|n| self.interval_at(n, std::process::id()))
            .map_err(|_| mutations::Failure::ClockUnavailable)
    }
}
fn elapsed(ns: u64, ppm: u32, upper: bool) -> Result<u128> {
    if !(1..=10_000).contains(&ppm) {
        return Err(Failure::Unavailable);
    }
    let ns = u128::from(ns);
    if upper {
        Ok(((ns + 2 * NS_PER_MS) * 1_000_000).div_ceil(1_000_000 - u128::from(ppm)))
    } else {
        Ok(ns.saturating_sub(2 * NS_PER_MS) * 1_000_000 / (1_000_000 + u128::from(ppm)))
    }
}
