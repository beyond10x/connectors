use super::{Failure, Result};

#[derive(Clone, Copy)]
pub(super) struct Stamp {
    pub monotonic: u64,
    pub offset_lower: i128,
    pub offset_upper: i128,
}
fn ns(t: libc::timespec) -> Result<u64> {
    if t.tv_sec < 0 || !(0..1_000_000_000).contains(&t.tv_nsec) {
        return Err(Failure::Unavailable);
    }
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(t.tv_nsec as u64))
        .ok_or(Failure::Unavailable)
}
fn read(clock: libc::clockid_t) -> Result<u64> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // A valid output pointer; clock ids below are fixed Linux clock constants.
    if unsafe { libc::clock_gettime(clock, &mut t) } != 0 {
        return Err(Failure::Unavailable);
    }
    ns(t)
}
pub(super) fn qualify_resolution() -> Result<()> {
    for clock in [libc::CLOCK_MONOTONIC, libc::CLOCK_BOOTTIME] {
        let mut t = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // Read-only timer resolution query with a valid output pointer.
        if unsafe { libc::clock_getres(clock, &mut t) } != 0 || ns(t)? > 1_000_000 {
            return Err(Failure::Unavailable);
        }
    }
    Ok(())
}
impl Stamp {
    pub fn read() -> Result<Self> {
        let before = read(libc::CLOCK_BOOTTIME)?;
        let monotonic = read(libc::CLOCK_MONOTONIC)?;
        let after = read(libc::CLOCK_BOOTTIME)?;
        if after.checked_sub(before).is_none_or(|d| d > 1_000_000) {
            return Err(Failure::Unavailable);
        }
        Ok(Self {
            monotonic,
            offset_lower: i128::from(before) - i128::from(monotonic),
            offset_upper: i128::from(after) - i128::from(monotonic),
        })
    }
    pub fn continuous_with(self, current: Self) -> Result<()> {
        if current.monotonic < self.monotonic
            || current.offset_lower > self.offset_upper + 1_000_000
            || current.offset_upper < self.offset_lower - 1_000_000
        {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }
}
