//! Periodic metadata recovery. Scans are observations, never ownership. The
//! supervisor establishes a quiescent instance before applying any fence.
use super::*;
use crate::local::{clock, mutations as ledger};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(super) struct Batch {
    pub instance: String,
    pub references: Vec<ledger::AttemptRef>,
    clock: Option<Arc<clock::BoundedClock>>,
}
#[cfg(test)]
impl Batch {
    pub(super) fn without_clock(instance: String, references: Vec<ledger::AttemptRef>) -> Self {
        Self {
            instance,
            references,
            clock: None,
        }
    }
}
struct RecoveryClock(Option<Arc<clock::BoundedClock>>);
impl ledger::Clock for RecoveryClock {
    fn now(&self) -> ledger::Result<ledger::ClockInterval> {
        ledger::Clock::now(self.0.as_deref().ok_or(ledger::Failure::ClockUnavailable)?)
    }
}

pub(super) struct Background {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Background {
    pub fn start(paths: Arc<Paths>, pool: Arc<supervisor::Pool>) -> Result<Self> {
        let (stop, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("connectors-mutation-recovery".into())
            .spawn(move || {
                let mut scan = ledger::PendingScan::default();
                while matches!(
                    receiver.recv_timeout(Duration::from_secs(5)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    // A failed pass supplies no retry authority. The next pass
                    // must obtain fresh positive pending references from SQLite.
                    let _ = sweep(&paths, &pool, &mut scan);
                }
            })
            .map_err(|_| Code::Unavailable)?;
        Ok(Self {
            stop: Some(stop),
            thread: Some(thread),
        })
    }
    pub fn stop(mut self) -> Result<()> {
        self.finish()
    }
    fn finish(&mut self) -> Result<()> {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| Code::Unavailable)?;
        }
        Ok(())
    }
}
impl Drop for Background {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn sweep(paths: &Paths, pool: &supervisor::Pool, scan: &mut ledger::PendingScan) -> Result<()> {
    let store = ledger::Store::new(&paths.state, RecoveryClock(None), ledger::Limits::default())
        .map_err(|_| Code::MetadataUnavailable)?;
    let pending = store
        .pending(scan, 64)
        .map_err(|_| Code::MetadataUnavailable)?;
    let Some(first) = pending.first() else {
        return Ok(());
    };
    // The scan has closed every metadata handle before this network exchange.
    let clock = if pending
        .iter()
        .any(|entry| entry.state == ledger::State::Prepared)
    {
        Config::load(&paths.config)
            .ok()
            .and_then(|config| config.approval_clock)
            .and_then(|source| source.acquire().ok())
            .map(Arc::new)
    } else {
        None
    };
    let instance = first.instance.clone();
    if pending.iter().any(|entry| entry.instance != instance) {
        return Err(Code::MetadataUnavailable.into());
    }
    pool.recover(Batch {
        instance,
        references: pending.into_iter().map(|entry| entry.reference).collect(),
        clock,
    })
}

pub(super) fn apply(
    paths: &Paths,
    batch: Batch,
    quiescent: supervisor::Quiescent<'_>,
    stopped: &AtomicBool,
) -> Result<()> {
    if batch.instance != quiescent.instance() || batch.references.len() > 64 {
        return Err(Code::LifecycleConflict.into());
    }
    let until = Instant::now() + Duration::from_millis(250);
    // Metadata housekeeping needs no current business/result grant. Time still
    // comes only from the currently selected qualified source. A changed or
    // unavailable source cannot settle Prepared, but does not stop quarantine.
    let selected = Config::load(&paths.config)
        .ok()
        .and_then(|config| config.approval_clock)
        .map(|source| source.sha256());
    let clock = batch
        .clock
        .filter(|clock| selected.as_deref() == Some(clock.configuration_sha256()));
    let store = ledger::Store::new(
        &paths.state,
        RecoveryClock(clock),
        ledger::Limits::default(),
    )
    .map_err(|_| Code::MetadataUnavailable)?;
    for reference in batch.references {
        if stopped.load(Ordering::SeqCst) || Instant::now() >= until {
            break;
        }
        if store
            .recover_for_instance(reference, &batch.instance)
            .is_err()
        {
            // One authoritative observation after an uncertain acknowledgement,
            // never a retry inside this batch or a provider send receipt.
            let _ = store.observe(reference);
        }
    }
    Ok(())
}
