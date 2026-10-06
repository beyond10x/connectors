//! What one read invoke's metadata transactions cost as the store grows by
//! read invokes, the way an operator's store grows.
use super::tests::{
    NOW, binding, fixture, prepared, publish_fixture, recorded_clock_floor_events, recorded_events,
};
use super::*;
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt, time::Duration};

/// The transactions of one `operations invoke` of a read operation, in the
/// process that performs each, as `tests::read_invoke` runs them: the CLI's
/// cached-description reads, admission and owner handshake in a fresh
/// process, then the owner's capture, dispatch and release. Returns each
/// step's wall time and the metadata work it did.
fn measured_invoke(root: &Path, registry: &Registry, reference: &str, now: u64) -> Vec<Step> {
    use super::super::metadata::{exit_process, simulate_process, take_costs};
    let mut steps = Vec::new();
    let mut step = |name: &'static str, run: &mut dyn FnMut()| {
        take_costs();
        let cpu = process_cpu();
        let started = std::time::Instant::now();
        run();
        steps.push(Step {
            name,
            elapsed: started.elapsed(),
            cpu: process_cpu().saturating_sub(cpu),
            costs: take_costs(),
        });
    };
    simulate_process(now);
    step("cli.inspect", &mut || {
        drop(Metadata::inspect(root).unwrap())
    });
    step("cli.inspect", &mut || {
        drop(Metadata::inspect(root).unwrap())
    });
    step("cli.admit_read", &mut || {
        registry
            .admit_read(&binding(), reference, &BTreeSet::new(), now)
            .unwrap();
    });
    step("cli.inspect", &mut || {
        drop(Metadata::inspect(root).unwrap())
    });
    // The CLI exits; its handles do not outlive it.
    exit_process();
    simulate_process(1);
    let mut captured = None;
    step("owner.capture_read", &mut || {
        captured = Some(
            registry
                .capture_read(&binding(), reference, &BTreeSet::new(), now, now + 1000)
                .unwrap(),
        );
    });
    let mut dispatched = None;
    step("owner.dispatch_read", &mut || {
        dispatched = Some(
            registry
                .dispatch_read(captured.take().unwrap(), now)
                .unwrap(),
        );
    });
    step("owner.release_read", &mut || {
        registry
            .release_read(dispatched.take().unwrap(), now)
            .unwrap();
    });
    steps
}

struct Step {
    name: &'static str,
    elapsed: Duration,
    /// CPU time of every thread of this process, the bridge workers
    /// included: unlike wall time, it does not grow with other load on the
    /// host.
    cpu: Duration,
    costs: BTreeMap<&'static str, (u64, Duration)>,
}

fn process_cpu() -> Duration {
    // SAFETY: `getrusage` writes only the struct it is handed.
    let usage = unsafe {
        let mut usage = std::mem::zeroed::<libc::rusage>();
        libc::getrusage(libc::RUSAGE_SELF, &mut usage);
        usage
    };
    let time = |value: libc::timeval| {
        Duration::from_secs(value.tv_sec as u64) + Duration::from_micros(value.tv_usec as u64)
    };
    time(usage.ru_utime) + time(usage.ru_stime)
}

/// A store grown by read invokes to each of `sizes` recorded events, in
/// ascending order. With `saved`, each size's store is copied there once and
/// reused by later runs, so two builds are measured against the same bytes.
fn grown_stores(sizes: &[i64], saved: Option<&Path>) -> Vec<(tempfile::TempDir, Registry, String)> {
    let stored = |events: i64| saved.map(|saved| saved.join(format!("read-invokes-{events}")));
    if sizes
        .iter()
        .all(|events| stored(*events).is_some_and(|stored| stored.join("reference").exists()))
    {
        return sizes
            .iter()
            .map(|events| load(&stored(*events).unwrap()))
            .collect();
    }
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let mut grown = Vec::new();
    let mut now = NOW + 1;
    for &events in sizes {
        while recorded_events(root.path()) < events {
            measured_invoke(root.path(), &registry, &reference, now);
            now += 1;
        }
        let copy = tempfile::tempdir().unwrap();
        save(root.path(), copy.path(), &reference);
        if let Some(stored) = stored(events) {
            save(root.path(), &stored, &reference);
        }
        // Measured on a copy, so growing to the next size starts from this
        // store as it was, not as the measurement left it.
        grown.push(load(copy.path()));
    }
    grown
}

fn save(root: &Path, stored: &Path, reference: &str) {
    std::fs::create_dir_all(stored).unwrap();
    let target = stored.join("metadata.sqlite3");
    let _ = std::fs::remove_file(&target);
    rusqlite::Connection::open(root.join("metadata.sqlite3"))
        .unwrap()
        .execute("VACUUM INTO ?1", [target.to_str().unwrap()])
        .unwrap();
    // The copy is written in rollback mode; metadata admits only WAL.
    rusqlite::Connection::open(&target)
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    std::fs::write(stored.join("reference"), reference).unwrap();
    // The registry's durable clock floor belongs to the store; a store copied
    // without it would refuse every sample within one interval of its record.
    let floor = stored.join(FLOOR_FILE);
    let _ = std::fs::remove_file(&floor);
    if root.join(FLOOR_FILE).exists() {
        std::fs::copy(root.join(FLOOR_FILE), &floor).unwrap();
    }
}

const FLOOR_FILE: &str = "registry-clock.floor";

fn load(stored: &Path) -> (tempfile::TempDir, Registry, String) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = root.path().join("metadata.sqlite3");
    std::fs::copy(stored.join("metadata.sqlite3"), &target).unwrap();
    std::fs::write(root.path().join("metadata.lock"), b"").unwrap();
    let mut names = vec!["metadata.sqlite3", "metadata.lock"];
    if stored.join(FLOOR_FILE).exists() {
        std::fs::copy(stored.join(FLOOR_FILE), root.path().join(FLOOR_FILE)).unwrap();
        names.push(FLOOR_FILE);
    }
    for name in names {
        std::fs::set_permissions(
            root.path().join(name),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    let reference = std::fs::read_to_string(stored.join("reference")).unwrap();
    let registry = Registry::new(root.path());
    (root, registry, reference)
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Per-invoke metadata time at 50, 600 and 1,200 recorded events, or at the
/// comma-separated sizes in `CONNECTORS_STORE_COST_EVENTS`, with where the
/// time went: per step, and per kind of metadata work. A measurement, not a
/// check; run it in a release build:
/// `CONNECTORS_STORE_COST_STORES=<dir> cargo test --release -p connectors-host --lib read_invoke_cost_by_store_size -- --ignored --nocapture`.
/// The first run grows and saves the stores in `<dir>`; later runs reuse them.
#[test]
#[ignore = "timing measurement; run explicitly"]
fn read_invoke_cost_by_store_size() {
    let saved = std::env::var_os("CONNECTORS_STORE_COST_STORES").map(std::path::PathBuf::from);
    let sizes: Vec<i64> = std::env::var("CONNECTORS_STORE_COST_EVENTS").map_or_else(
        |_| vec![50, 600, 1200],
        |sizes| {
            sizes
                .split(',')
                .map(|size| size.trim().parse().unwrap())
                .collect()
        },
    );
    const MEASURED: u64 = 5;
    let stores = grown_stores(&sizes, saved.as_deref());
    for (root, registry, reference) in &stores {
        let recorded = recorded_events(root.path());
        // The first invoke starts the owner; the next ones are measured.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            measured_invoke(root.path(), registry, reference, NOW + 10_000)
        }));
        let before = recorded_events(root.path());
        let floors_before = recorded_clock_floor_events(root.path());
        let mut totals = Vec::new();
        let mut cpu = Vec::new();
        let mut failed = 0;
        let mut by_step: BTreeMap<String, (Duration, Duration)> = BTreeMap::new();
        let mut by_kind: BTreeMap<&'static str, (u64, Duration)> = BTreeMap::new();
        for invoke in 1..=MEASURED {
            // An invoke past the bridge deadline answers `OutcomeUnknown`, as
            // it does for an operator: counted, not measured.
            let Ok(steps) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                measured_invoke(root.path(), registry, reference, NOW + 10_000 + invoke)
            })) else {
                failed += 1;
                continue;
            };
            totals.push(steps.iter().map(|step| step.elapsed).sum::<Duration>());
            cpu.push(steps.iter().map(|step| step.cpu).sum::<Duration>());
            for (index, step) in steps.iter().enumerate() {
                let entry = by_step.entry(format!("{index}.{}", step.name)).or_default();
                entry.0 += step.elapsed;
                entry.1 += step
                    .costs
                    .get("er.execute_batch")
                    .map_or(Duration::ZERO, |cost| cost.1);
                for (kind, (calls, spent)) in &step.costs {
                    let entry = by_kind.entry(kind).or_default();
                    entry.0 += calls;
                    entry.1 += *spent;
                }
            }
        }
        let measured = totals.len();
        if measured == 0 {
            println!("events={recorded} invokes={MEASURED} failed={failed}");
            continue;
        }
        let appended = recorded_events(root.path()) - before;
        let floors = recorded_clock_floor_events(root.path()) - floors_before;
        totals.sort_unstable();
        cpu.sort_unstable();
        let per = |total: Duration| ms(total) / measured as f64;
        println!(
            "events={recorded} invokes={measured} failed={failed} appended_per_invoke={} clock_floor_events_per_invoke={:.1} median_ms={:.0} min_ms={:.0} max_ms={:.0} median_cpu_ms={:.0}",
            appended / MEASURED as i64,
            floors as f64 / MEASURED as f64,
            ms(totals[measured / 2]),
            ms(totals[0]),
            ms(totals[measured - 1]),
            ms(cpu[measured / 2]),
        );
        for (step, (spent, batch)) in &by_step {
            println!(
                "events={recorded}   step {step:<22} ms_per_invoke={:.1} of_which_execute_batch_ms={:.1}",
                per(*spent),
                per(*batch),
            );
        }
        for (kind, (calls, spent)) in &by_kind {
            println!(
                "events={recorded}   kind {kind:<22} calls_per_invoke={:.1} ms_per_invoke={:.1}",
                *calls as f64 / measured as f64,
                per(*spent),
            );
        }
    }
}

/// The read-back of a handle's own write is not a licence to skip the next
/// verification: blob bytes altered after it are refused on the next reopen
/// whenever a full replay of the same bytes refuses them.
#[test]
fn a_reopen_after_its_own_write_refuses_every_blob_altered_since() {
    use super::super::metadata::simulate_process;
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    let store = root.path().join("metadata.sqlite3");
    let rows: Vec<i64> =
        rusqlite::Connection::open_with_flags(&store, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap()
            .prepare("SELECT rowid FROM connectors_er_blobs ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
    assert!(!rows.is_empty());
    let mut accepted = Vec::new();
    let mut refused_by_replay = 0;
    for (index, rowid) in rows.into_iter().enumerate() {
        simulate_process(7);
        // This process writes, so its handle last verified its own read-back.
        registry
            .admit_read(
                &binding(),
                &reference,
                &BTreeSet::new(),
                NOW + 1 + index as u64,
            )
            .unwrap();
        let physical = rusqlite::Connection::open(&store).unwrap();
        let original: rusqlite::types::Value = physical
            .query_row(
                "SELECT bytes FROM connectors_er_blobs WHERE rowid=?1",
                [rowid],
                |row| row.get(0),
            )
            .unwrap();
        physical
            .execute(
                "UPDATE connectors_er_blobs SET bytes=zeroblob(length(bytes)) WHERE rowid=?1",
                [rowid],
            )
            .unwrap();
        let fresh = tempfile::tempdir().unwrap();
        save(root.path(), fresh.path(), &reference);
        let (copy, _, _) = load(fresh.path());
        simulate_process(1_000 + index as u64);
        let replayed = Metadata::inspect(copy.path()).map(drop);
        simulate_process(7);
        let reopened = Metadata::inspect(root.path()).map(drop);
        if replayed.is_err() {
            refused_by_replay += 1;
            if reopened.is_ok() {
                accepted.push(rowid);
            }
        }
        physical
            .execute(
                "UPDATE connectors_er_blobs SET bytes=?1 WHERE rowid=?2",
                rusqlite::params![original, rowid],
            )
            .unwrap();
    }
    assert!(
        refused_by_replay > 1,
        "a full replay refused {refused_by_replay} alterations"
    );
    assert!(
        accepted.is_empty(),
        "a reopen after a write accepted {} of {refused_by_replay} blob alterations a full replay refuses: {accepted:?}",
        accepted.len()
    );
}

/// One variable against the stores `read_invoke_cost_by_store_size` saved:
/// whether a batch's cost follows the store's size or the subjects its batch
/// reads reach. A runtime record is a new subject in no earlier batch; the
/// registry clock is a member of every registry batch the store holds. Run as
/// `CONNECTORS_STORE_COST_STORES=<dir> cargo test --release -p connectors-host --lib batch_cost_by_subject -- --ignored --nocapture`.
#[test]
#[ignore = "timing measurement; run explicitly"]
fn batch_cost_by_subject() {
    use super::super::metadata::{simulate_process, take_costs};
    let saved = std::env::var_os("CONNECTORS_STORE_COST_STORES").map(std::path::PathBuf::from);
    let sizes: Vec<i64> = std::env::var("CONNECTORS_STORE_COST_EVENTS").map_or_else(
        |_| vec![50, 600, 1200],
        |sizes| {
            sizes
                .split(',')
                .map(|size| size.trim().parse().unwrap())
                .collect()
        },
    );
    let batch_ms = |costs: BTreeMap<&'static str, (u64, Duration)>| {
        costs.get("er.execute_batch").map_or(0.0, |cost| ms(cost.1))
    };
    for (root, registry, reference) in grown_stores(&sizes, saved.as_deref()) {
        let recorded = recorded_events(root.path());
        simulate_process(2);
        drop(Metadata::inspect(root.path()).unwrap());
        let mut runtime = Vec::new();
        let mut clock = Vec::new();
        for round in 0..3 {
            take_costs();
            let mut metadata = Metadata::update(root.path(), true).unwrap();
            metadata
                .connection
                .execute(
                    "INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES (?1,0)",
                    params![format!("probe-{round}")],
                )
                .unwrap();
            metadata.persist_runtime_state().unwrap();
            drop(metadata);
            runtime.push(batch_ms(take_costs()));
            registry
                .admit_read(
                    &binding(),
                    &reference,
                    &BTreeSet::new(),
                    NOW + 20_000 + round,
                )
                .unwrap();
            clock.push(batch_ms(take_costs()));
        }
        println!(
            "events={recorded} runtime_record_batch_ms={runtime:.0?} registry_clock_batch_ms={clock:.0?}"
        );
    }
}
