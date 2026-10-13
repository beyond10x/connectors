use super::{Failure, Result};

// The timer's contract (contracts/service/clock.md, "Local continuation and
// bounds"): each reading brackets a clock that stops while the machine is
// suspended between two samples of a clock that keeps counting, and a change
// in their offset beyond the original bracket plus 1 ms refuses the estimate.
// Suspend therefore invalidates it without relying on any rate measured while
// suspended, and the configured rate bound applies to the non-suspended clock.
//
// Linux: CLOCK_BOOTTIME counts suspend, CLOCK_MONOTONIC does not; both share
// one kernel time base and the same frequency adjustment.
#[cfg(target_os = "linux")]
const SUSPEND_INCLUSIVE: libc::clockid_t = libc::CLOCK_BOOTTIME;
#[cfg(target_os = "linux")]
const SUSPEND_EXCLUSIVE: libc::clockid_t = libc::CLOCK_MONOTONIC;
// macOS keeps the guarantee: CLOCK_MONOTONIC_RAW (mach_continuous_time) counts
// sleep, CLOCK_UPTIME_RAW (mach_absolute_time) does not, and both read the same
// unadjusted time base, so their offset moves only by time asleep. Darwin's
// CLOCK_MONOTONIC is not used: clock_gettime(3) makes it subject to frequency
// and time adjustments that CLOCK_UPTIME_RAW does not see, so adjustment alone
// would move the offset past 1 ms within seconds. The rate bound then covers
// the oscillator alone, with no kernel adjustment.
#[cfg(target_os = "macos")]
const SUSPEND_INCLUSIVE: libc::clockid_t = libc::CLOCK_MONOTONIC_RAW;
#[cfg(target_os = "macos")]
const SUSPEND_EXCLUSIVE: libc::clockid_t = libc::CLOCK_UPTIME_RAW;

// Windows keeps the guarantee: QueryInterruptTimePrecise counts sleep,
// QueryUnbiasedInterruptTimePrecise does not, and both read the interrupt time
// in 100 ns units, so their offset moves only by time asleep
// (contracts/service/clock.md). QueryInterruptTime and
// QueryUnbiasedInterruptTime advance only at each timer interrupt, about
// 15.6 ms by default, coarser than the 1 ms bound, and are not used.
#[cfg(windows)]
const SUSPEND_INCLUSIVE: bool = true;
#[cfg(windows)]
const SUSPEND_EXCLUSIVE: bool = false;
#[cfg(windows)]
fn read(suspend_inclusive: bool) -> Result<u64> {
    use windows_sys::Win32::System::WindowsProgramming::{
        QueryInterruptTimePrecise, QueryUnbiasedInterruptTimePrecise,
    };
    let mut units = 0u64;
    // SAFETY: both functions write one u64 through a valid pointer.
    unsafe {
        if suspend_inclusive {
            QueryInterruptTimePrecise(&mut units);
        } else {
            QueryUnbiasedInterruptTimePrecise(&mut units);
        }
    }
    units.checked_mul(100).ok_or(Failure::Unavailable)
}
/// The precise interrupt-time functions read the performance counter, whose
/// resolution is far below 1 ms; Windows has no resolution query for them.
/// Two successive readings of each clock must not run backwards.
#[cfg(windows)]
pub(super) fn qualify_resolution() -> Result<()> {
    for clock in [SUSPEND_EXCLUSIVE, SUSPEND_INCLUSIVE] {
        let first = read(clock)?;
        if read(clock)? < first {
            return Err(Failure::Unavailable);
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct Stamp {
    pub monotonic: u64,
    pub offset_lower: i128,
    pub offset_upper: i128,
}
#[cfg(unix)]
fn ns(t: libc::timespec) -> Result<u64> {
    if t.tv_sec < 0 || !(0..1_000_000_000).contains(&t.tv_nsec) {
        return Err(Failure::Unavailable);
    }
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(t.tv_nsec as u64))
        .ok_or(Failure::Unavailable)
}
#[cfg(unix)]
fn read(clock: libc::clockid_t) -> Result<u64> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // A valid output pointer; clock ids are the fixed constants above.
    if unsafe { libc::clock_gettime(clock, &mut t) } != 0 {
        return Err(Failure::Unavailable);
    }
    ns(t)
}
#[cfg(unix)]
pub(super) fn qualify_resolution() -> Result<()> {
    for clock in [SUSPEND_EXCLUSIVE, SUSPEND_INCLUSIVE] {
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
        let before = read(SUSPEND_INCLUSIVE)?;
        let monotonic = read(SUSPEND_EXCLUSIVE)?;
        let after = read(SUSPEND_INCLUSIVE)?;
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

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;

    /// Both macOS clocks resolve to 1 ms or better.
    #[test]
    fn suspend_clocks_qualify() {
        assert_eq!(SUSPEND_INCLUSIVE, libc::CLOCK_MONOTONIC_RAW);
        assert_eq!(SUSPEND_EXCLUSIVE, libc::CLOCK_UPTIME_RAW);
        qualify_resolution().unwrap();
    }

    /// The two clocks count one timebase, so while the machine stays awake
    /// their offset holds within the 1 ms tolerance across seconds; only
    /// sleep moves it.
    #[test]
    fn offset_holds_while_awake() {
        let first = Stamp::read().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let second = Stamp::read().unwrap();
        first.continuous_with(second).unwrap();
    }

    /// Time spent asleep moves the offset, and the continuation is refused.
    #[test]
    fn slept_time_refuses_the_continuation() {
        let first = Stamp::read().unwrap();
        let mut slept = Stamp::read().unwrap();
        slept.offset_lower += 3_000_000;
        slept.offset_upper += 3_000_000;
        assert!(matches!(
            first.continuous_with(slept),
            Err(Failure::Unavailable)
        ));
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    /// Both interrupt-time clocks qualify and are monotone.
    #[test]
    fn interrupt_clocks_qualify_and_are_monotone() {
        qualify_resolution().unwrap();
        for clock in [SUSPEND_EXCLUSIVE, SUSPEND_INCLUSIVE] {
            let first = read(clock).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            let second = read(clock).unwrap();
            assert!(second > first);
        }
    }

    /// The unbiased clock excludes sleep, so it is never ahead of the clock
    /// that counts it, read just after.
    #[test]
    fn the_unbiased_clock_is_not_ahead_of_the_biased_one() {
        for _ in 0..1000 {
            let unbiased = read(SUSPEND_EXCLUSIVE).unwrap();
            let biased = read(SUSPEND_INCLUSIVE).unwrap();
            assert!(unbiased <= biased);
        }
    }

    /// While awake the offset holds within the 1 ms tolerance; a moved offset
    /// refuses the continuation.
    #[test]
    fn offset_holds_while_awake_and_sleep_refuses() {
        let first = Stamp::read().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let second = Stamp::read().unwrap();
        first.continuous_with(second).unwrap();
        let mut slept = second;
        slept.offset_lower += 3_000_000;
        slept.offset_upper += 3_000_000;
        assert!(matches!(
            first.continuous_with(slept),
            Err(Failure::Unavailable)
        ));
    }
}
