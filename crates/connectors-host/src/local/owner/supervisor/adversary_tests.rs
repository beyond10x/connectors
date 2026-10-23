//! story:owner-request-waits-outside-the-pool-lock, the cases its own suite
//! leaves unsaid: a request that waits on its instance's control lock is sent
//! once that lock is free, and the queued count it holds while it waits is
//! returned on every way out of `Pool::send`.
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
        governed: false,
        projection_revision: None,
        budget: None,
    }
}

/// A pool with one stand-in worker, `answering`, which answers each job at
/// once, and one, `gated`, which reads nothing until `gate` is dropped and then
/// drops its receiver with whatever it holds queued.
fn pool() -> (Arc<Pool>, tempfile::TempDir, mpsc::Sender<()>) {
    let root = tempfile::tempdir().unwrap();
    let pool = Pool::new(
        Arc::new(Paths {
            config: root.path().join("unread-config"),
            state: root.path().join("unopened-state"),
        }),
        uuid::Uuid::new_v4().to_string(),
    );
    let (gate, gated) = mpsc::channel::<()>();
    let mut gated = Some(gated);
    for instance in ["answering", "gated"] {
        let (sender, receiver) = mpsc::sync_channel::<Work>(16);
        let thread = if instance == "answering" {
            std::thread::spawn(move || {
                while let Ok(work) = receiver.recv() {
                    if let Work::Invoke(job) = work {
                        job.reply.send(Ok(Output::Value(json!(null))));
                    }
                }
            })
        } else {
            let gated = gated.take().unwrap();
            std::thread::spawn(move || {
                let _ = gated.recv();
                drop(receiver);
            })
        };
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
    (Arc::new(pool), root, gate)
}

fn state(pool: &Pool, instance: &str) -> (Arc<Mutex<lifecycle::Control>>, Arc<AtomicUsize>) {
    let workers = pool.workers.lock().unwrap();
    let worker = &workers[instance];
    (worker.control.clone(), worker.queued.clone())
}

/// Spins until `queued` reads `want`, for at most five seconds.
fn settle(queued: &AtomicUsize, want: usize) -> usize {
    let until = Instant::now() + Duration::from_secs(5);
    while queued.load(Ordering::SeqCst) != want && Instant::now() < until {
        std::thread::yield_now();
    }
    queued.load(Ordering::SeqCst)
}

/// A request for `instance` on its own thread, answering its result code and
/// when it answered.
fn request(
    pool: &Arc<Pool>,
    instance: &'static str,
    deadline: u64,
) -> (
    mpsc::Receiver<(Option<Code>, Instant)>,
    std::thread::JoinHandle<()>,
) {
    let (answered, answer) = mpsc::channel();
    let pool = pool.clone();
    let thread = std::thread::spawn(move || {
        let code = pool
            .run(instance, &adapter(instance), read(), deadline)
            .err()
            .map(|error| error.code);
        let _ = answered.send((code, Instant::now()));
    });
    (answer, thread)
}

/// The outcome of the story: a request behind an instance's in-flight write
/// waits on that write and no longer. Once the control lock is free the
/// waiting request is sent and answered, promptly and well inside its own
/// deadline; it is not left to time out.
#[test]
fn a_request_waiting_on_its_instance_is_sent_once_the_lock_is_released() {
    let (pool, _root, gate) = pool();
    let (control, queued) = state(&pool, "answering");
    let guard = control.lock().unwrap();
    let (answer, thread) = request(&pool, "answering", connectors_sdk::now_ms() + 5_000);
    assert_eq!(
        settle(&queued, 1),
        1,
        "the request never waited on the lock"
    );
    let released = Instant::now();
    drop(guard);
    let own = answer.recv_timeout(Duration::from_secs(2)).ok();
    thread.join().unwrap();
    let after = settle(&queued, 0);
    drop(gate);
    pool.shutdown().unwrap();
    let (code, at) = own.expect("a request still waited 2 s after its instance's lock was freed");
    assert_eq!(code, None, "the released request was refused");
    let latency = at.saturating_duration_since(released);
    assert!(
        latency < Duration::from_millis(250),
        "the released request answered {latency:?} after the lock was freed"
    );
    assert_eq!(after, 0, "an answered request is still counted as queued");
}

/// `queued` is returned on every way out of `send`: the deadline passing while
/// the lock is held, the pool stopping while it is held, a full channel and a
/// worker gone. A leaked count reads as work for ever: `working()` and
/// `quiet()` would never let the owner retire.
#[test]
fn the_queued_count_is_returned_on_every_way_out_of_send() {
    let (pool, _root, gate) = pool();
    let (control, queued) = state(&pool, "answering");

    // The deadline passes while the lock is held.
    let guard = control.lock().unwrap();
    let (answer, thread) = request(&pool, "answering", connectors_sdk::now_ms() + 50);
    let (code, _) = answer.recv_timeout(Duration::from_secs(2)).unwrap();
    thread.join().unwrap();
    assert_eq!(code, Some(Code::Timeout));
    assert_eq!(settle(&queued, 0), 0, "a timed-out wait leaked its count");

    // A full channel: sixteen sent and held, the seventeenth refused.
    let (_, gated) = state(&pool, "gated");
    let mut receivers = Vec::new();
    for _ in 0..16 {
        receivers.push(
            pool.send(
                "gated",
                &adapter("gated"),
                read(),
                connectors_sdk::now_ms() + 5_000,
                None,
            )
            .map(|_| ())
            .map_err(|error| error.code),
        );
    }
    assert!(receivers.iter().all(|sent| sent.is_ok()), "{receivers:?}");
    assert_eq!(gated.load(Ordering::SeqCst), 16);
    let full = pool
        .send(
            "gated",
            &adapter("gated"),
            read(),
            connectors_sdk::now_ms() + 5_000,
            None,
        )
        .map(|_| ())
        .map_err(|error| error.code);
    assert_eq!(full, Err(Code::Capacity));
    assert_eq!(
        gated.load(Ordering::SeqCst),
        16,
        "a refused send leaked its count"
    );

    // The worker drops its receiver and the sixteen it held with it.
    drop(gate);
    assert_eq!(
        settle(&gated, 0),
        0,
        "jobs dropped unsent leaked their count"
    );
    let gone = pool
        .send(
            "gated",
            &adapter("gated"),
            read(),
            connectors_sdk::now_ms() + 5_000,
            None,
        )
        .map(|_| ())
        .map_err(|error| error.code);
    assert_eq!(gone, Err(Code::Unavailable));
    assert_eq!(
        gated.load(Ordering::SeqCst),
        0,
        "a send to a gone worker leaked its count"
    );

    // The pool stops while the lock is held.
    let (answer, thread) = request(&pool, "answering", connectors_sdk::now_ms() + 5_000);
    assert_eq!(settle(&queued, 1), 1);
    pool.stopped.store(true, Ordering::SeqCst);
    let (code, _) = answer.recv_timeout(Duration::from_secs(2)).unwrap();
    thread.join().unwrap();
    assert_eq!(code, Some(Code::Unavailable));
    assert_eq!(
        settle(&queued, 0),
        0,
        "a wait the stop ended leaked its count"
    );
    drop(guard);
    pool.shutdown().unwrap();
}
