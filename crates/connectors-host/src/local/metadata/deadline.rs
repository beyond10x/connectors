//! Scoped cutoff for synchronous metadata admission; it grants no authority.
use super::{Failure, Result, WAIT_BOUND};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

thread_local! {
    static CURRENT: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub(in crate::local) fn within<T>(until: Instant, work: impl FnOnce() -> Result<T>) -> Result<T> {
    struct Restore(Option<Instant>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|slot| slot.set(self.0));
        }
    }
    let previous =
        CURRENT.with(|slot| slot.replace(Some(slot.get().map_or(until, |old| old.min(until)))));
    let _restore = Restore(previous);
    check()?;
    let result = work();
    check()?;
    result
}

pub(super) fn cutoff(wait: Duration) -> Instant {
    let ordinary = Instant::now() + wait;
    CURRENT.with(|slot| slot.get().map_or(ordinary, |until| until.min(ordinary)))
}

pub(super) fn check() -> Result<()> {
    if CURRENT.with(|slot| slot.get().is_some_and(|until| Instant::now() >= until)) {
        Err(Failure::MetadataUnavailable)
    } else {
        Ok(())
    }
}

pub(super) fn configure(connection: &rusqlite::Connection) -> Result<()> {
    check()?;
    if CURRENT.with(|slot| slot.get().is_some()) {
        connection
            .busy_handler(Some(busy))
            .map_err(super::unavailable)
    } else {
        connection
            .busy_timeout(WAIT_BOUND)
            .map_err(super::unavailable)
    }
}

pub(super) fn lock<T>(mutex: &std::sync::Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    let Some(until) = CURRENT.with(Cell::get) else {
        return mutex.lock().map_err(|_| Failure::MetadataUnavailable);
    };
    loop {
        check()?;
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(Failure::MetadataUnavailable),
            Err(std::sync::TryLockError::WouldBlock) => std::thread::sleep(
                Duration::from_millis(5).min(until.saturating_duration_since(Instant::now())),
            ),
        }
    }
}

fn busy(attempt: i32) -> bool {
    let pause = Duration::from_millis(5);
    let until = CURRENT.with(Cell::get);
    if let Some(until) = until {
        let Some(left) = until.checked_duration_since(Instant::now()) else {
            return false;
        };
        std::thread::sleep(left.min(pause));
        Instant::now() < until
    } else if attempt >= 0 && (attempt as u128) * pause.as_millis() < WAIT_BOUND.as_millis() {
        // A connection can outlive the inspection scope. Restore an ordinary
        // finite wait instead of keeping a past caller's deadline or spinning.
        std::thread::sleep(pause);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_cutoff_only_shortens_and_scope_restores_on_unwind() {
        let outer = Instant::now() + Duration::from_secs(10);
        within(outer, || {
            assert_eq!(cutoff(Duration::from_secs(30)), outer);
            within(outer + Duration::from_secs(10), || {
                assert_eq!(cutoff(Duration::from_secs(30)), outer);
                Ok(())
            })?;
            let inner = Instant::now() + Duration::from_secs(1);
            let panicked = std::panic::catch_unwind(|| {
                let _: Result<()> = within(inner, || {
                    assert_eq!(cutoff(Duration::from_secs(30)), inner);
                    panic!("fixture unwind");
                });
            });
            assert!(panicked.is_err());
            assert_eq!(cutoff(Duration::from_secs(30)), outer);
            Ok(())
        })
        .unwrap();
        assert!(CURRENT.with(Cell::get).is_none());
        assert!(within::<()>(Instant::now(), || panic!("expired scope ran work")).is_err());
        assert!(CURRENT.with(Cell::get).is_none());
    }

    #[test]
    fn metadata_lock_admission_keeps_the_original_cutoff() {
        use std::os::fd::AsRawFd;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("held.lock");
        let held = std::fs::File::create(&path).unwrap();
        let waiting = std::fs::File::open(&path).unwrap();
        // SAFETY: held owns this descriptor for the whole fixture hold.
        assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX) }, 0);
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            drop(held);
        });
        let started = Instant::now();
        let result = within(started + Duration::from_millis(100), || {
            super::super::acquire_lifecycle_lock(&waiting)
        });
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_millis(400));
        release.join().unwrap();
        super::super::acquire_lifecycle_lock(&waiting).unwrap();
    }

    #[test]
    fn sqlite_busy_wait_observes_the_current_absolute_cutoff() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("held.sqlite3");
        let holder = rusqlite::Connection::open(&path).unwrap();
        holder
            .execute_batch("CREATE TABLE fixture (value); BEGIN EXCLUSIVE")
            .unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            holder.execute_batch("ROLLBACK").unwrap();
        });
        let started = Instant::now();
        let result = within(started + Duration::from_secs(10), || {
            configure(&connection)?;
            // The already-open connection must observe the inner cutoff too.
            within(started + Duration::from_millis(100), || {
                connection
                    .execute_batch("INSERT INTO fixture VALUES(1)")
                    .map_err(super::super::unavailable)
            })
        });
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_millis(400));
        release.join().unwrap();
        connection
            .execute_batch("INSERT INTO fixture VALUES(2)")
            .unwrap();
    }

    #[test]
    fn pooled_mutex_admission_keeps_original_cutoff() {
        let mutex = std::sync::Arc::new(std::sync::Mutex::new(()));
        let (ready, entered) = std::sync::mpsc::channel();
        let held = mutex.clone();
        let release = std::thread::spawn(move || {
            let _guard = held.lock().unwrap();
            ready.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(600));
        });
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        assert!(
            within(started + Duration::from_millis(100), || lock(&mutex)
                .map(drop))
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_millis(400));
        release.join().unwrap();
        drop(lock(&mutex).unwrap());
    }
}
