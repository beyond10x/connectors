---
format: aep.planning-md/3
id: story:pool-idle-checks-wait-outside-the-pool-lock
kind: story
status: draft
title: The pool's idle checks wait on an instance's lock outside the pool lock
relations:
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-20261019-post0540-u3-adversary-1
scope:
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
revision: 2
---
## Outcome

`Pool::working()` and `Pool::quiet()` no longer take an instance's control lock while holding the
pool's `workers` lock, so a control lock held on an idle instance cannot delay a request for any
other instance.

## Why now

Found by the adversary on wave 20261019-post0540 unit U3
(review-result:wave-20261019-post0540-u3-adversary-1, finding 2, pre-existing, INFEASIBLE).
`crates/connectors-host/src/local/owner/supervisor.rs:764` (`working`) and `:789` (`quiet`) lock each
worker's control lock under `workers`. U3 moved `Pool::send`'s wait out of `workers`; these two
callers still wait inside it.

What reaches it (from the adversary's report, not measured at runtime): a long control-lock hold
only happens on a `busy` worker, and both functions skip a busy worker before locking; other
holders (`stop`, `status`, `send`) hold it for microseconds. So no shipped path is known to stall.

## Acceptance

- The two cases in `adversary_lock_order_tests.rs` pass: while `quiet()` or `working()` waits on an
  idle instance's held control lock, a request for another instance answers by its 300 ms deadline.
  The file is reproduced under "Red test" below
  as the adversary wrote it; it is a child module of `supervisor.rs`
  (`#[cfg(test)] mod adversary_lock_order_tests;`). It is red on 52769afb3 and on U3's head.

## Scope

- **Files:** `crates/connectors-host/src/local/owner/supervisor.rs` (`working` ~:764, `quiet` ~:789) — cited (adversary report)
- **Confidence:** high for the site; the fix shape (clone controls under `workers`, lock after release, or `try_lock`) is undecided

## Red test

`crates/connectors-host/src/local/owner/supervisor/adversary_lock_order_tests.rs`, as written by the adversary:

```rust
//! story:owner-request-waits-outside-the-pool-lock: "a request for any other
//! instance is not delayed by" an instance whose control lock is held. The
//! owner's idle checks, `Pool::working` and `Pool::quiet`, still take each
//! idle worker's control lock while they hold `workers`, so while one waits
//! on a held control lock every request for every other instance waits on
//! `workers` with it.
use super::*;
use crate::local::config::{Executable, Startup};

fn adapter(instance: &str) -> Adapter {
    Adapter {
        credential_revision: None,
        instance_id: instance.into(),
        adapter_id: "adapter".into(),
        configuration_revision: "config".into(),
        protocol: "v1alpha1".into(),
        private_protocol: None,
        startup: Startup::OnDemand,
        restart: Default::default(),
        permissions: Default::default(),
        executable: Executable {
            path: "/not-launched".into(),
            sha256: "a".repeat(64),
            args: Vec::new(),
        },
    }
}

fn read() -> Task {
    Task::Invoke {
        connection: "connection".into(),
        operation: "read".into(),
        schema: "schema".into(),
        revision: "revision".into(),
        document: b"{}".to_vec(),
    }
}

/// Two stand-in workers, `answering` and `held`, each answering every job at
/// once; neither is busy.
fn pool() -> (Arc<Pool>, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let pool = Pool::new(
        Arc::new(Paths {
            config: root.path().join("unread-config"),
            state: root.path().join("unopened-state"),
        }),
        uuid::Uuid::new_v4().to_string(),
    );
    for instance in ["answering", "held"] {
        let (sender, receiver) = mpsc::sync_channel::<Work>(16);
        let thread = std::thread::spawn(move || {
            while let Ok(work) = receiver.recv() {
                if let Work::Invoke(job) = work {
                    job.reply.send(Ok(Output::Value(json!(null))));
                }
            }
        });
        pool.workers.lock().unwrap().insert(
            instance.into(),
            Worker {
                sender,
                thread,
                control: Arc::new(Mutex::new(lifecycle::Control::default())),
                busy: Arc::new(AtomicBool::new(false)),
                recovering: Arc::new(AtomicBool::new(false)),
                recovery_pending: Arc::new(AtomicBool::new(false)),
                queued: Arc::new(AtomicUsize::new(0)),
            },
        );
    }
    (Arc::new(pool), root)
}

/// Holds `held`'s control lock, runs `check` on its own thread until it holds
/// `workers` (it then waits on that control lock for as long as it is held),
/// and answers how a request for `answering`, with 300 ms to run, fared.
fn other_instance_behind(check: fn(&Pool) -> Result<bool>) -> (Option<Option<Code>>, u64) {
    let (pool, _root) = pool();
    let control = pool.workers.lock().unwrap()["held"].control.clone();
    let guard = control.lock().unwrap();
    let checking = {
        let pool = pool.clone();
        std::thread::spawn(move || {
            let _ = check(&pool);
        })
    };
    let until = Instant::now() + Duration::from_secs(5);
    while pool.workers.try_lock().is_ok() && Instant::now() < until {
        std::thread::yield_now();
    }
    let deadline = connectors_sdk::now_ms() + 300;
    let (answered, answer) = mpsc::channel();
    let other = {
        let pool = pool.clone();
        std::thread::spawn(move || {
            let code = pool
                .run("answering", &adapter("answering"), read(), deadline)
                .err()
                .map(|error| error.code);
            let _ = answered.send((code, connectors_sdk::now_ms()));
        })
    };
    let own = answer.recv_timeout(Duration::from_secs(1)).ok();
    drop(guard);
    checking.join().unwrap();
    other.join().unwrap();
    pool.shutdown().unwrap();
    match own {
        Some((code, at)) => (Some(code), at.saturating_sub(deadline)),
        None => (None, u64::MAX),
    }
}

#[test]
fn quiet_waiting_on_a_held_control_lock_delays_no_other_instance() {
    let (code, late) = other_instance_behind(Pool::quiet);
    assert_eq!(
        code,
        Some(None),
        "a request for another instance was still unanswered 1 s on, behind `quiet`"
    );
    assert_eq!(late, 0, "answered {late} ms after its deadline");
}

#[test]
fn working_waiting_on_a_held_control_lock_delays_no_other_instance() {
    let (code, late) = other_instance_behind(Pool::working);
    assert_eq!(
        code,
        Some(None),
        "a request for another instance was still unanswered 1 s on, behind `working`"
    );
    assert_eq!(late, 0, "answered {late} ms after its deadline");
}
```
