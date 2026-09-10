//! Bounded UTC under explicitly configured source and local timer assumptions.
//! Acquisition does network I/O; an acquired Clock only reads local timers.
//! Public observations cannot reconstruct the process-bound capability.
#[cfg(test)]
#[path = "../../tests/fixtures/clock/server.rs"]
mod fixture;
mod protocol;
#[cfg(test)]
mod tests;
mod timer;

use super::mutations::{self, Clock, ClockInterval};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddr, UdpSocket},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use timer::Stamp;

const MAX_SAFE_MS: u128 = 9_007_199_254_740_991;
const NS_PER_MS: u128 = 1_000_000;
const NS_PER_S: u128 = 1_000_000_000;
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidConfiguration,
    Unavailable,
}
type Result<T> = std::result::Result<T, Failure>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub format: String,
    pub address: String,
    pub public_key: String,
    /// Deployment assertion covering hardware error and kernel clock adjustment.
    /// Neither a public source sample nor the configured number proves it.
    pub max_rate_error_ppm: u32,
}
impl Configuration {
    fn resolved(&self) -> Result<(SocketAddr, [u8; 32])> {
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
        if self.format != "roughtime-clock/1"
            || !(1..=10_000).contains(&self.max_rate_error_ppm)
            || address.port() == 0
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
        Ok((
            address,
            key.try_into().map_err(|_| Failure::InvalidConfiguration)?,
        ))
    }
    pub fn validate(&self) -> Result<()> {
        self.resolved().map(|_| ())
    }
    pub fn sha256(&self) -> String {
        connectors_core::digest(&serde_json::json!(self))
    }
    /// One exchange; never call while holding a metadata transaction or a key/
    /// policy lease. No host clock modification, DNS, fallback, or retry occurs.
    pub fn acquire(&self) -> Result<BoundedClock> {
        let (address, key) = self.resolved()?;
        timer::qualify_resolution()?;
        let socket = UdpSocket::bind(if address.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })
        .map_err(|_| Failure::Unavailable)?;
        socket.connect(address).map_err(|_| Failure::Unavailable)?;
        let deadline = Instant::now() + QUERY_TIMEOUT;
        let started = Stamp::read()?;
        let (request, nonce) = protocol::request(&key)?;
        socket
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(|_| Failure::Unavailable)?;
        if socket.send(&request).map_err(|_| Failure::Unavailable)? != request.len() {
            return Err(Failure::Unavailable);
        }
        socket
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(|_| Failure::Unavailable)?;
        let mut bytes = [0u8; protocol::PACKET_SIZE + 1];
        let n = socket.recv(&mut bytes).map_err(|_| Failure::Unavailable)?;
        let received = Stamp::read()?;
        remaining(deadline)?;
        started.continuous_with(received)?;
        if received.monotonic < started.monotonic || n > protocol::PACKET_SIZE {
            return Err(Failure::Unavailable);
        }
        let sample = protocol::verify(&bytes[..n], &request, &nonce, &key)?;
        remaining(deadline)?;
        let clock = BoundedClock {
            sample,
            started,
            received,
            rate_error_ppm: self.max_rate_error_ppm,
            configuration_sha256: self.sha256(),
            process: std::process::id(),
            last_read: AtomicU64::new(received.monotonic),
        };
        clock.observe()?;
        Ok(clock)
    }
}
fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Failure::Unavailable)
}

/// Safe observation only. It is never accepted as clock evidence on an ingress.
#[derive(Debug, Serialize)]
pub struct Observation {
    pub configuration_sha256: String,
    pub lower_unix_ms: i64,
    pub upper_unix_ms: i64,
}

/// No Clone or Deserialize; a restart/fork must acquire a new signed sample.
/// Consumers separately retain current clock-selection and authorization policy.
pub struct BoundedClock {
    sample: protocol::Sample,
    started: Stamp,
    received: Stamp,
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
        let interval = self.interval_at(Stamp::read()?, std::process::id())?;
        Ok(Observation {
            configuration_sha256: self.configuration_sha256.clone(),
            lower_unix_ms: interval.lower_unix_ms,
            upper_unix_ms: interval.upper_unix_ms,
        })
    }
    fn interval_at(&self, now: Stamp, process: u32) -> Result<ClockInterval> {
        if process != self.process
            || now.monotonic < self.last_read.fetch_max(now.monotonic, Ordering::SeqCst)
        {
            return Err(Failure::Unavailable);
        }
        self.started.continuous_with(now)?;
        let from_start = now
            .monotonic
            .checked_sub(self.started.monotonic)
            .ok_or(Failure::Unavailable)?;
        let from_receive = now
            .monotonic
            .checked_sub(self.received.monotonic)
            .ok_or(Failure::Unavailable)?;
        let elapsed_upper = elapsed(from_start, self.rate_error_ppm, true)?;
        if elapsed_upper > 30 * NS_PER_S {
            return Err(Failure::Unavailable);
        }
        let midpoint = u128::from(self.sample.midpoint) * NS_PER_S;
        let radius = u128::from(self.sample.radius) * NS_PER_S;
        let origin_lower = midpoint
            .checked_sub(radius)
            .and_then(|v| v.checked_sub(3 * NS_PER_MS))
            .ok_or(Failure::Unavailable)?;
        let lower = origin_lower
            .checked_add(elapsed(from_receive, self.rate_error_ppm, false)?)
            .ok_or(Failure::Unavailable)?
            / NS_PER_MS;
        let upper = midpoint
            .checked_add(radius)
            .and_then(|v| v.checked_add(elapsed_upper))
            .and_then(|v| v.checked_add(3 * NS_PER_MS))
            .ok_or(Failure::Unavailable)?
            .div_ceil(NS_PER_MS);
        if upper > MAX_SAFE_MS || upper.checked_sub(lower).is_none_or(|w| w > 4000) {
            return Err(Failure::Unavailable);
        }
        // No leap announcement is consumed. Avoid extrapolating across any UTC
        // midnight, including the possible skipped/repeated second around it.
        let day_ms = 86_400_000;
        let origin_ms = origin_lower / NS_PER_MS;
        if origin_ms / day_ms != upper / day_ms
            || origin_ms % day_ms < 1000
            || upper % day_ms >= day_ms - 1000
        {
            return Err(Failure::Unavailable);
        }
        Ok(ClockInterval {
            lower_unix_ms: lower as i64,
            upper_unix_ms: upper as i64,
        })
    }
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
