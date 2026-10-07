//! Actual CLI/owner/private child, disposable HTTPS, signing clock and qualified
//! keyring. Synthetic clock/provider responses do not prove sandbox acceptance.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use connectors_host::local::owner::{approval_issuance, mutation};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[path = "background_recovery.rs"]
mod background_recovery;
#[path = "owner_replay.rs"]
mod owner_replay;

/// An exact kernel handle obtained from this test's private owner socket.
/// The crash fixture requires Linux SO_PEERPIDFD, not a numeric PID kill.
fn owner_process(cli: &Cli) -> (OwnedFd, Vec<OwnedFd>) {
    let handles = owner_handles(cli);
    assert_eq!(
        handles.1.len(),
        1,
        "fixture must own exactly one native child"
    );
    handles
}

pub(super) fn owner_handles(cli: &Cli) -> (OwnedFd, Vec<OwnedFd>) {
    let socket =
        std::os::unix::net::UnixStream::connect(cli.paths.state.join("owner.sock")).unwrap();
    let mut raw = -1;
    let mut size = std::mem::size_of_val(&raw) as libc::socklen_t;
    // SAFETY: socket is held and both output pointers have the declared size.
    let result = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERPIDFD,
            (&mut raw as *mut libc::c_int).cast(),
            &mut size,
        )
    };
    assert_eq!(
        result,
        0,
        "fixture requires SO_PEERPIDFD: {}",
        std::io::Error::last_os_error()
    );
    assert_eq!(size as usize, std::mem::size_of::<libc::c_int>());
    assert!(raw >= 0);
    // SAFETY: successful SO_PEERPIDFD returns a new descriptor owned by us.
    let owner = unsafe { OwnedFd::from_raw_fd(raw) };
    let info = fs::read_to_string(format!("/proc/self/fdinfo/{raw}")).unwrap();
    let pid: u32 = info
        .lines()
        .find_map(|line| line.strip_prefix("Pid:"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut children = Vec::new();
    for task in fs::read_dir(format!("/proc/{pid}/task")).unwrap() {
        let list = match fs::read_to_string(task.unwrap().path().join("children")) {
            Ok(list) => list,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("fixture owner thread: {error}"),
        };
        for child in list.split_whitespace() {
            let pid: u32 = child.parse().unwrap();
            // SAFETY: create an observation handle for this held owner's child;
            // these handles are only polled, never used to signal a process.
            let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
            assert!(fd >= 0);
            // SAFETY: successful pidfd_open returned a new owned descriptor.
            children.push(unsafe { OwnedFd::from_raw_fd(fd) });
        }
    }
    (owner, children)
}

pub(super) fn finish_filtered_owner(
    (owner, children): (OwnedFd, Vec<OwnedFd>),
    acknowledged: bool,
) -> bool {
    let mut failures = Vec::new();
    if !acknowledged {
        failures.push("owner shutdown was not acknowledged".to_owned());
    }
    let mut event = libc::pollfd {
        fd: owner.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: only this owner slot comes from the authenticated private socket's
    // SO_PEERPIDFD. A timeout does not grant signal authority to child-list fds.
    if unsafe { libc::poll(&mut event, 1, 5000) } != 1 || event.revents & libc::POLLIN == 0 {
        failures.push("strongly identified owner required forced cleanup".to_owned());
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                owner.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            failures.push(format!(
                "owner signal failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        if unsafe { libc::poll(&mut event, 1, 5000) } != 1 || event.revents & libc::POLLIN == 0 {
            failures.push("strongly identified owner exit was not observed".to_owned());
        }
    }
    for child_observation in children {
        let mut event = libc::pollfd {
            fd: child_observation.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: poll only. The numeric child-list capture does not establish
        // ownership at acquisition and therefore NEVER authorizes signalling.
        if unsafe { libc::poll(&mut event, 1, 5000) } != 1 || event.revents & libc::POLLIN == 0 {
            failures
                .push("observation-only child exit was not observed; no signal sent".to_owned());
        }
    }
    if failures.is_empty() {
        eprintln!("filtered owner and observation-only child exits observed; graceful=true");
        true
    } else {
        eprintln!("filtered cleanup refusal: {failures:?}");
        if !std::thread::panicking() {
            panic!("filtered cleanup failed: {failures:?}");
        }
        // Do not double-panic through Cli Drop: notification-proven fallback
        // cleanup and the observer/listener joins must still be reached.
        false
    }
}

#[test]
fn catalog_fault_cleanup_observation_handle_cannot_authorize_signal() {
    // Constructed boundary control, not a kernel PID-reuse reproduction. Both
    // processes are direct test-owned children, retained and reaped by us. The
    // second has no ownership relationship to the selected owner.
    let spawn = || {
        OwnedProcess(
            Command::new("/usr/bin/sleep")
                .arg("60")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    };
    let handle = |child: &OwnedProcess| {
        // SAFETY: an unreaped direct Child retains this exact process identity.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, child.0.id(), 0) } as i32;
        assert!(fd >= 0);
        unsafe { OwnedFd::from_raw_fd(fd) }
    };
    let mut owner = spawn();
    let owner_fd = handle(&owner);
    let mut observation_only = spawn();
    let observation_fd = handle(&observation_only);
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        finish_filtered_owner((owner_fd, vec![observation_fd]), true)
    }));
    let status = observation_only.0.try_wait().unwrap();
    eprintln!(
        "constructed cleanup boundary: observation-only child status={status:?}, cleanup_refused={}",
        result.is_err()
    );
    assert!(
        status.is_none(),
        "observation-only pidfd authorized a signal to a test-owned sibling"
    );
    assert!(
        result.is_err(),
        "an unobserved child exit must refuse cleanup"
    );
    drop(observation_only);

    // A strongly owned direct-child stand-in still permits forced retirement;
    // the actual runtime supplies this slot from SO_PEERPIDFD.
    let mut owner = spawn();
    let owner_fd = handle(&owner);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        finish_filtered_owner((owner_fd, vec![]), false)
    }));
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(owner.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    assert!(
        result.is_err(),
        "forced retirement must retain its cleanup failure"
    );
}

fn kill_owner((process, children): (OwnedFd, Vec<OwnedFd>)) {
    // SAFETY: the held pidfd identifies only the peer of our task-owned socket.
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                process.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        },
        0
    );
    wait_owner((process, children));
}

fn wait_owner((process, children): (OwnedFd, Vec<OwnedFd>)) {
    for process in std::iter::once(process).chain(children) {
        let mut event = libc::pollfd {
            fd: process.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one initialized entry, with the pidfd held throughout the poll.
        assert_eq!(
            unsafe { libc::poll(&mut event, 1, 5000) },
            1,
            "owner or native child did not exit"
        );
        assert_ne!(event.revents & libc::POLLIN, 0);
    }
}

fn owner_replay(cli: &Cli, request: &approval_issuance::Request<'_>) -> Value {
    let deadline = mutation::Deadline::start().unwrap();
    let delivery = owner::WriteClient::connect(&cli.paths, false, deadline)
        .unwrap()
        .invoke("gitlab", request, Some("owned-merge"), None, deadline)
        .unwrap();
    serde_json::to_value(delivery).unwrap()
}

pub(super) fn finish_process(mut running: OwnedProcess) -> Output {
    use std::io::Read;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    running
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    running
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    Output {
        status: running.0.wait().unwrap(),
        stdout,
        stderr,
    }
}

fn audits(cli: &Cli) -> Vec<Value> {
    cli.recorded().audits()
}

fn revoked_restoration_control(cli: &Cli, provider: &Provider, mode: u8) {
    assert!(!cli.paths.state.join("owner.sock").exists());
    let before = cli.recorded();
    let calls = provider.count();
    let effects = provider.merge_effects.load(Ordering::SeqCst);
    // Closing a raw database fd releases this process's POSIX locks for that
    // inode. Drain the idle observer before the control opens its raw fd, then
    // reopen the same authority only after that fd has closed.
    drop(cli.recorded.borrow_mut().take());
    cli.fault
        .as_ref()
        .unwrap()
        .prove_readonly_sync_restored(&cli.binary, mode);
    before.assert_unchanged(&cli.recorded());
    assert_eq!(provider.count(), calls);
    assert_eq!(provider.merge_effects.load(Ordering::SeqCst), effects);
    assert!(!cli.paths.state.join("owner.sock").exists());
}

struct Clock {
    address: String,
    count: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
    respond: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Clock {
    fn new() -> Self {
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let address = socket.local_addr().unwrap().to_string();
        let count = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let respond = Arc::new(AtomicBool::new(true));
        let replies = respond.clone();
        let stop = stopped.clone();
        let calls = count.clone();
        let thread = std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            let started = Instant::now();
            while !stop.load(Ordering::SeqCst) {
                match socket.recv_from(&mut buffer) {
                    Ok((size, peer)) => {
                        let sequence = calls.fetch_add(1, Ordering::SeqCst) as u64;
                        if !replies.load(Ordering::SeqCst) {
                            continue;
                        }
                        // Synthetic time advances across exchanges as well as
                        // within an acquired sample. Repeating the fixed epoch
                        // can legitimately fail the ledger's rollback guard.
                        let mut fixture = clock_fixture::Fixture::default();
                        fixture.midpoint += started.elapsed().as_secs() + sequence;
                        socket
                            .send_to(&fixture.reply(&buffer[..size]), peer)
                            .unwrap();
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(e) => panic!("fixture clock: {e}"),
                }
            }
        });
        Self {
            address,
            count,
            stopped,
            respond,
            thread: Some(thread),
        }
    }
}
impl Drop for Clock {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_guarded_merge_applied_refused_and_lost_response_restart() {
    for mode in [0, 2, 1, 3] {
        journey(mode);
    }
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_guarded_merge_revocation_finishes_admitted_audit() {
    journey(4);
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_recovers_abandoned_preparation_only_with_trusted_time() {
    journey(5);
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts() {
    journey(6);
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_background_recovers_revoked_removed_target_without_disclosure() {
    journey(7);
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively() {
    // Same binary, response latch and observer, without armed I/O injection.
    journey(12);
    for mode in [8, 9, 10] {
        journey(mode);
    }
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted() {
    journey(11);
}

#[test]
#[ignore = "subprocess fixture; no action without its explicit task-owned root"]
fn prepared_attempt_exit_fixture() {
    use connectors_host::local::{approvals::Subject, mutations as ledger};
    let Some(root) = std::env::var_os("CONNECTORS_PREPARED_FIXTURE") else {
        return;
    };
    let root = PathBuf::from(root);
    let subject: Subject =
        serde_json::from_slice(&fs::read(root.join("private/subject.json")).unwrap()).unwrap();
    let t = subject.target;
    struct NoClock;
    impl ledger::Clock for NoClock {
        fn now(&self) -> ledger::Result<ledger::ClockInterval> {
            Err(ledger::Failure::ClockUnavailable)
        }
    }
    let store =
        ledger::Store::new(&root.join("cli/state"), NoClock, ledger::Limits::default()).unwrap();
    let candidate = ledger::Candidate {
        namespace: ledger::Namespace {
            receiver_instance: t.instance.clone(),
            tenant: None,
            realm: None,
            caller: subject.authority.scope.caller,
            executor: None,
            origin: ledger::Origin::Direct,
        },
        fingerprint: ledger::Fingerprint {
            operation: ledger::OperationRef {
                instance: t.instance,
                adapter: "catalog".into(),
                operation: t.operation,
            },
            connection_ref: t.connection,
            connection_revision: t.connection_revision,
            contract_ref: t.contract,
            profile: t.profile,
            descriptor_revision: t.descriptor_revision,
            configuration_revision: t.configuration_revision,
            canonicalization_version: subject.canonicalization,
            input_digest: subject.input_sha256,
            route: None,
        },
        caller_key: std::env::var_os("CONNECTORS_PREPARED_FIXTURE_UNKEYED")
            .is_none()
            .then(|| "owned-merge".into()),
        request_id: "a6b7af40-a60f-4a73-a4b2-fd247c773c11".into(),
        // This process exercises the durable port, not native preflight or
        // approval spending. No provider capability or dispatch gate is used.
        approval: ledger::Approval::NotRequired,
    };
    let ledger::Preparation::Prepared(prepared) = store.prepare(&candidate).unwrap() else {
        panic!("seed unexpectedly reused an attempt");
    };
    let reference = prepared.reference();
    let path = root.join("private/seeded-attempt.json");
    private(&path, &serde_json::to_vec(&json!({"authority":reference.authority.to_string(), "id":reference.attempt_id.to_string()})).unwrap());
    fs::File::open(path).unwrap().sync_all().unwrap();
    std::process::exit(73);
}

fn stored_attempt(cli: &Cli) -> recorded_state::StoredAttempt {
    cli.recorded().only_attempt()
}

/// The exact original attempt and its business key, selected by identity so a
/// later attempt under another key can never answer for it.
fn stored_original(cli: &Cli, attempt: &Value) -> recorded_state::StoredAttempt {
    cli.recorded().original(attempt["id"].as_str().unwrap())
}

fn journey(mode: u8) {
    let label = if mode == 12 {
        "settlement control"
    } else {
        "historical variant"
    };
    eprintln!("catalog {label} {mode}: start");
    journey_case(mode);
    eprintln!("catalog {label} {mode}: passed");
}

fn journey_case(mode: u8) {
    let provider = Provider::new();
    provider.merge_mode.store(
        match mode {
            6 => 3,
            8 | 10 | 12 => 5,
            9 => 6,
            // Adversary pass 2 probe; the same held Applied response as mode 10.
            11 => 5,
            other => other,
        },
        Ordering::SeqCst,
    );
    let mut custody = Custody::new(provider.root.path());
    let clock = Clock::new();
    let mut cli = Cli::new(provider.root.path());
    if matches!(mode, 8..=12) {
        cli.fault = Some(settlement_fault::Controller::new(provider.root.path()));
    }
    configure(&cli, &provider, &custody);
    let config = fs::read_to_string(&cli.paths.config)
        .unwrap()
        .replace("format='connectors-local/1'", "format='connectors-local/2'")
        .replace(
            "[adapters.gitlab]\n",
            "[adapters.gitlab]\nprivate_protocol='connectors-private/2'\n",
        )
        .replace(
            "operations=['project.get','issues.list']",
            "operations=['project.get','merge_request.merge']",
        );
    private(&cli.paths.config, format!("{config}\n[approval_clock]\nformat='roughtime-clock/1'\naddress='{}'\npublic_key='{}'\nmax_rate_error_ppm=100\n", clock.address, STANDARD.encode(clock_fixture::root_key())).as_bytes());
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap();
    fs::remove_file(&credential).unwrap();
    success(cli.run(&["approvals", "key-init", "--adapter", "gitlab"]));
    let policy = provider.root.path().join("private/policy.json");
    private(&policy, br#"{"operations":["merge_request.merge"]}"#);
    success(cli.run(&[
        "approvals",
        "policy-set",
        "--adapter",
        "gitlab",
        "--input-file",
        policy.to_str().unwrap(),
    ]));
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "merge_request.merge",
    ]));
    let input = json!({"id":"org/project","merge_request_iid":4,"body":{"sha":"0123456789abcdef0123456789abcdef01234567"},"pipeline_id":12}).to_string();
    let request = approval_issuance::Request {
        connection,
        operation: "merge_request.merge",
        schema: description["schema"].as_str().unwrap(),
        revision: description["revision"].as_str().unwrap(),
        input: &input,
    };
    let target = [
        "--adapter",
        "gitlab",
        "--connection",
        connection,
        "--operation",
        "merge_request.merge",
        "--schema",
        description["schema"].as_str().unwrap(),
        "--revision",
        description["revision"].as_str().unwrap(),
        "--input-json",
        &input,
    ];
    let args = |group, action| {
        let mut args = vec![group, action];
        args.extend(target);
        args
    };
    let before = provider.count();
    let mut missing = args("operations", "invoke");
    missing.extend(["--idempotency-key", "owned-merge"]);
    let missing_result = refusal(cli.run(&missing), "approval_required");
    assert_eq!(
        missing_result["mutation"]["classification"],
        "not_attempted"
    );
    assert_eq!(provider.count(), before);
    let prepared = success(cli.run(&args("approvals", "prepare")));
    let proof = provider.root.path().join("private/proof.json");
    let mut issue = args("approvals", "issue");
    issue.extend([
        "--approve-subject",
        prepared["preparation"]["subject_sha256"].as_str().unwrap(),
        "--proof-output",
        proof.to_str().unwrap(),
    ]);
    success(cli.run(&issue));
    // Attach while metadata is writable. The observer only reads this exact
    // authority, and never needs a fresh write-intent attachment during faults.
    let before_dispatch = cli.recorded();
    before_dispatch.cross_check(&cli.paths.state);
    assert!(before_dispatch.audits().is_empty());
    if cli.fault.is_some() {
        before_dispatch.evidence(mode, "before");
    }
    let mut invocation = missing.clone();
    invocation.extend(["--approval-file", proof.to_str().unwrap()]);
    if mode == 5 {
        cli.shutdown();
        drop(custody.daemon.take());
        fs::remove_file(&proof).unwrap();
        private(
            &provider.root.path().join("private/subject.json"),
            &serde_json::to_vec(&prepared["preparation"]["subject"]).unwrap(),
        );
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cli_journey::guarded_merge::prepared_attempt_exit_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env_clear()
            .env("CONNECTORS_PREPARED_FIXTURE", provider.root.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(73), "{output:?}");
        let seeded: Value = serde_json::from_slice(
            &fs::read(provider.root.path().join("private/seeded-attempt.json")).unwrap(),
        )
        .unwrap();
        cli.recorded().assert_seeded(&seeded, true);
        assert_eq!(
            stored_attempt(&cli),
            ("prepared".into(), "pending".into(), None, None)
        );
        let native_calls = provider.count();
        let clock_calls = clock.count.load(Ordering::SeqCst);
        clock.respond.store(false, Ordering::SeqCst);
        let pending = refusal(cli.run(&invocation), "outcome_unknown");
        assert_eq!(pending["mutation"]["attempt"]["id"], seeded["id"]);
        assert_eq!(pending["mutation"]["attempt"]["instance"], "fixture-gitlab");
        assert_eq!(pending["mutation"]["classification"], "unknown");
        assert_eq!(pending["mutation"]["replayed"], false);
        assert_eq!(stored_attempt(&cli).0, "prepared");
        assert!(clock.count.load(Ordering::SeqCst) > clock_calls);
        assert_eq!(pending["mutation"]["cause"]["code"], "unavailable");
        assert!(cli.status()["child_incarnation"].is_null());
        let failed_clock_calls = clock.count.load(Ordering::SeqCst);
        clock.respond.store(true, Ordering::SeqCst);
        let recovered = refusal(cli.run(&invocation), "interrupted");
        assert!(clock.count.load(Ordering::SeqCst) > failed_clock_calls);
        assert_eq!(recovered["mutation"]["classification"], "not_attempted");
        assert_eq!(recovered["mutation"]["replayed"], true);
        assert_eq!(
            recovered["mutation"]["attempt"],
            pending["mutation"]["attempt"]
        );
        assert_eq!(
            recovered["mutation"]["original_request_id"],
            "a6b7af40-a60f-4a73-a4b2-fd247c773c11"
        );
        let settled = stored_attempt(&cli);
        assert_eq!((&*settled.0, &*settled.1), ("aborted", "replayable"));
        assert_eq!(settled.3.unwrap() - settled.2.unwrap(), 86_400_000);
        cli.shutdown();
        clock.respond.store(false, Ordering::SeqCst);
        let clock_calls = clock.count.load(Ordering::SeqCst);
        let replay = refusal(cli.run(&invocation), "interrupted");
        assert_eq!(
            replay["mutation"]["attempt"],
            recovered["mutation"]["attempt"]
        );
        assert!(!cli.paths.state.join("owner.sock").exists());
        assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls);
        assert_eq!(stored_attempt(&cli), settled);
        assert_eq!(provider.count(), native_calls);
        assert!(
            !provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "PUT")
        );
        return;
    }
    if mode == 11 {
        // Adversary pass 2 probe of the mode 10 residual. The same construction
        // as mode 10 -- a known Applied effect held at the provider, disclosure
        // admission withdrawn while metadata is still writable, metadata then
        // subjected to controlled write EIO, then the response released. Mode 10
        // is what the refused disclosure reports about the effect that did
        // happen, and what the durable record may become after a restart.
        let mut running = OwnedProcess(cli.command(&invocation).spawn().unwrap());
        let until = Instant::now() + Duration::from_secs(10);
        while provider.merge_held.load(Ordering::SeqCst) == 0 {
            assert!(
                running.0.try_wait().unwrap().is_none(),
                "merge ended before the provider held its response"
            );
            assert!(Instant::now() < until, "provider did not hold the merge");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
        let held = cli.recorded();
        held.evidence(mode, "held");
        let original = held.assert_dispatched_and_spent();
        success(
            cli.run(&[
                "connections",
                "revoke",
                "--adapter",
                "gitlab",
                "--connection",
                connection,
                "--expected-revision",
                connected["connection"]["summary"]["revision"]
                    .as_str()
                    .unwrap(),
            ]),
        );
        let before_arm = cli.recorded();
        before_arm.evidence(mode, "before-arm");
        assert_eq!(before_arm.assert_dispatched_and_spent(), original);
        let fault = cli.fault.as_ref().unwrap().arm();
        provider.merge_mode.store(0, Ordering::SeqCst);
        let refused = refusal(finish_process(running), "revoked");
        cli.fault
            .as_ref()
            .unwrap()
            .evidence(mode, "after-response", true);
        cli.recorded().evidence(mode, "post");
        // The merge applied. A refused disclosure may withhold the projection
        // entirely, and it may report the known effect, but it can never answer
        // for an applied effect with not-attempted.
        assert!(
            refused["mutation"].is_null() || refused["mutation"]["classification"] == "applied",
            "a refused disclosure answered for an applied merge: {refused}"
        );
        assert_eq!(
            cli.recorded().original(&original),
            ("dispatching".into(), "pending".into(), None, None)
        );
        drop(fault);
        fs::remove_file(&proof).unwrap();
        cli.shutdown();
        let native_calls = provider.count();
        refusal(cli.run(&invocation), "revoked");
        assert_eq!(provider.count(), native_calls);
        assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
        // Background maintenance may or may not have swept the abandoned
        // attempt by now, so the reachable states are a pair. Every other state
        // either discloses the effect or releases its business key for reuse,
        // and that part of the invariant is deterministic across the sweep.
        let (attempt, key, settled_at, replay_expires) = stored_attempt(&cli);
        assert!(
            matches!(attempt.as_str(), "dispatching" | "indeterminate"),
            "attempt {attempt}/{key} after a revoked disclosure of an applied merge"
        );
        assert!(
            matches!(key.as_str(), "pending" | "quarantined"),
            "attempt {attempt}/{key} after a revoked disclosure of an applied merge"
        );
        assert_eq!((settled_at, replay_expires), (None, None));
        // Measured: the refused disclosure starts no owner, so no background
        // sweep can run. The record state after restart is therefore not
        // indeterminate-by-timing; it is fixed.
        assert!(!cli.paths.state.join("owner.sock").exists());
        revoked_restoration_control(&cli, &provider, mode);
        return;
    }
    if matches!(mode, 8 | 9 | 10 | 12) {
        // Terminal settlement persistence fails after a known native response.
        // The provider holds that response while the fixture arms exact metadata
        // I/O failure, so the owner cannot retain the result it is about to report.
        let known = mode != 9;
        // Mode 10 additionally withdraws current admission for disclosing that
        // known effect while the response is still held.
        let revoked = mode == 10;
        let mut running = OwnedProcess(cli.command(&invocation).spawn().unwrap());
        let until = Instant::now() + Duration::from_secs(10);
        // Wait for the hold itself. A recorded method or an effect count is a
        // weaker witness: releasing on one of those can overtake the handler and
        // have it answer the released mode instead of the held one.
        while provider.merge_held.load(Ordering::SeqCst) == 0 {
            assert!(
                running.0.try_wait().unwrap().is_none(),
                "merge ended before the provider held its response"
            );
            assert!(Instant::now() < until, "provider did not hold the merge");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "PUT")
                .count(),
            1
        );
        assert_eq!(
            provider.merge_effects.load(Ordering::SeqCst),
            usize::from(known)
        );
        let held = cli.recorded();
        held.evidence(mode, "held");
        let original = held.assert_dispatched_and_spent();
        if revoked {
            // Withdraw the connection that governs disclosure of this known
            // effect. Metadata is still writable: this is admission, not storage.
            success(
                cli.run(&[
                    "connections",
                    "revoke",
                    "--adapter",
                    "gitlab",
                    "--connection",
                    connection,
                    "--expected-revision",
                    connected["connection"]["summary"]["revision"]
                        .as_str()
                        .unwrap(),
                ]),
            );
        }
        // Preparation, approval spend and the dispatch gate are committed. Release
        // the fixed response, allowing catalog postflight, then prove which
        // persistence step actually failed from its cause and durable state.
        let before_arm = cli.recorded();
        before_arm.evidence(mode, "before-arm");
        assert_eq!(before_arm.assert_dispatched_and_spent(), original);
        let fault = (mode != 12).then(|| cli.fault.as_ref().unwrap().arm());
        provider.merge_mode.store(0, Ordering::SeqCst);
        let output = finish_process(running);
        cli.fault
            .as_ref()
            .unwrap()
            .evidence(mode, "after-response", mode != 12);
        cli.recorded().evidence(mode, "post");
        if mode == 12 {
            let control = success(output);
            assert_eq!(control["mutation"]["classification"], "applied");
            assert!(control["mutation"]["cause"].is_null());
            assert_eq!(control["source_audit"]["audit_status"], "complete");
            let facts = cli.recorded();
            let settled = facts.original(&original);
            assert_eq!((&*settled.0, &*settled.1), ("completed", "replayable"));
            assert_eq!(settled.3.unwrap() - settled.2.unwrap(), 86_400_000);
            facts.cross_check(&cli.paths.state);
            assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
            assert_eq!(
                provider
                    .methods
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|m| *m == "PUT")
                    .count(),
                1
            );
            eprintln!("catalog settlement control: held Applied response settled; audit complete");
            return;
        }
        if revoked {
            // Final disclosure admission is checked after settlement. A known
            // applied effect whose current admission is gone is not disclosed,
            // and the refusal authorizes no second effect.
            let refused = refusal(output, "revoked");
            // The refusal carries the admission failure and nothing else: no
            // projection, so no classification and no original attempt identity.
            assert!(refused.get("mutation").is_none(), "{refused}");
            assert_eq!(
                stored_attempt(&cli),
                ("dispatching".into(), "pending".into(), None, None)
            );
            assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
            drop(fault);
            fs::remove_file(&proof).unwrap();
            cli.shutdown();
            let native_calls = provider.count();
            // Restart changes nothing: current admission still governs, and the
            // refused disclosure never replays the native write.
            refusal(cli.run(&invocation), "revoked");
            assert_eq!(provider.count(), native_calls);
            assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
            assert_eq!(
                provider
                    .methods
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|method| method.as_str() == "PUT")
                    .count(),
                1
            );
            revoked_restoration_control(&cli, &provider, mode);
            return;
        }
        let first = if known {
            success(output)
        } else {
            refusal(output, "invalid_input")
        };
        let durable = cli.recorded().original(&original);
        eprintln!(
            "catalog settlement injection mode {mode}: classification={} cause={} audit={} durable={durable:?}",
            first["mutation"]["classification"],
            first["mutation"]["cause"],
            first["source_audit"]["audit_status"]
        );
        assert_eq!(
            first["mutation"]["classification"],
            if known { "applied" } else { "refused" },
            "{first}"
        );
        assert_eq!(first["mutation"]["replayed"], false);
        // The failure discloses its safe cause; it never replaces the known
        // effect with uncertainty or with a not-attempted answer.
        assert_eq!(first["mutation"]["cause"]["code"], "unavailable", "{first}");
        assert_eq!(first["mutation"]["cause"]["stage"], "attempt_store");
        if known {
            let payload: Value = first["result"].clone();
            assert_eq!(payload["body"]["state"], "merged");
        }
        // The same unwritable metadata leaves the admitted audit incomplete.
        // That cannot change the known effect either.
        assert_eq!(first["source_audit"]["audit_status"], "incomplete");
        // The known result was not retained: the durable record stays uncertain.
        assert_eq!(
            stored_original(&cli, &first["mutation"]["attempt"]),
            ("dispatching".into(), "pending".into(), None, None)
        );
        assert_eq!(
            provider.merge_effects.load(Ordering::SeqCst),
            usize::from(known)
        );
        drop(fault);
        let continued = cli.fault.as_ref().unwrap().continued();
        let original_owner = owner::Client::connect(&cli.paths, false)
            .unwrap()
            .host_incarnation;
        cli.shutdown();
        // The spend committed before dispatch: the same proof stays spent under
        // another business key and creates no second attempt or provider effect.
        let mut reused = invocation.clone();
        let key_index = reused
            .iter()
            .position(|arg| *arg == "--idempotency-key")
            .unwrap()
            + 1;
        reused[key_index] = "second-owned-merge";
        let refused = refusal(cli.run(&reused), "approval_replayed");
        assert_eq!(refused["mutation"]["classification"], "not_attempted");
        assert!(refused["mutation"]["cause"].is_null(), "{refused}");
        // The spent proof never re-enters the original uncertain attempt.
        assert_ne!(refused["mutation"]["attempt"], first["mutation"]["attempt"]);
        eprintln!("catalog mode {mode} post-spent-proof response: {refused}");
        let replacement_owner = owner::Client::connect(&cli.paths, false)
            .unwrap()
            .host_incarnation;
        assert_ne!(replacement_owner, original_owner);
        eprintln!(
            "catalog mode {mode} post-spent-proof socket={} owner={:?}",
            cli.paths.state.join("owner.sock").exists(),
            owner::Client::connect(&cli.paths, false).map(|client| client.host_incarnation)
        );
        cli.recorded().evidence(mode, "post-spent-proof");
        cli.fault
            .as_ref()
            .unwrap()
            .evidence(mode, "post-spent-proof", true);
        // Retaining a now-missing proof path proves recovery does not open it.
        fs::remove_file(&proof).unwrap();
        cli.shutdown();
        let native_calls = provider.count();
        // Exact owner exit makes this a quiescent observation. Background
        // maintenance may already have recovered the original while the real
        // spent-proof-control owner was alive; both production paths are valid.
        let before_retry = cli.recorded();
        before_retry.evidence(mode, "quiescent-before-retry");
        let (attempt, key, settled, expiry) = before_retry.original(&original);
        assert_eq!((settled, expiry), (None, None));
        let pending = match (attempt.as_str(), key.as_str()) {
            ("dispatching", "pending") => true,
            ("indeterminate", "quarantined") => false,
            pair => panic!("unexpected quiescent recovery precondition: {pair:?}"),
        };
        let replay = refusal(cli.run(&invocation), "outcome_unknown");
        eprintln!("catalog mode {mode} recovery response: {replay}");
        eprintln!(
            "catalog mode {mode} recovery socket={} owner={:?}",
            cli.paths.state.join("owner.sock").exists(),
            owner::Client::connect(&cli.paths, false).map(|client| client.host_incarnation)
        );
        cli.recorded().evidence(mode, "recovery");
        cli.fault.as_ref().unwrap().evidence(mode, "recovery", true);
        if pending {
            let recovered_owner = owner::Client::connect(&cli.paths, false)
                .unwrap()
                .host_incarnation;
            assert_ne!(recovered_owner, original_owner);
            assert_ne!(recovered_owner, replacement_owner);
            eprintln!("catalog mode {mode}: observed pending recovery through new owner");
        } else {
            assert!(!cli.paths.state.join("owner.sock").exists());
            assert!(owner::Client::connect(&cli.paths, false).is_err());
            eprintln!(
                "catalog mode {mode}: observed passive terminal replay after prior owner recovery"
            );
        }
        assert_eq!(replay["mutation"]["classification"], "unknown");
        assert_eq!(replay["mutation"]["replayed"], true);
        assert_eq!(replay["source_audit"]["audit_status"], "complete");
        assert_eq!(replay["mutation"]["attempt"], first["mutation"]["attempt"]);
        assert_eq!(
            replay["mutation"]["original_request_id"],
            first["mutation"]["original_request_id"]
        );
        assert_eq!(
            stored_original(&cli, &first["mutation"]["attempt"]),
            ("indeterminate".into(), "quarantined".into(), None, None)
        );
        assert_eq!(provider.count(), native_calls);
        assert_eq!(
            provider.merge_effects.load(Ordering::SeqCst),
            usize::from(known)
        );
        assert_eq!(
            provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "PUT")
                .count(),
            1
        );
        cli.fault.as_ref().unwrap().assert_resumed(continued);
        cli.fault
            .as_ref()
            .unwrap()
            .evidence(mode, "after-recovery", true);
        return;
    }
    if mode == 6 || mode == 7 {
        let run = if mode == 6 {
            background_recovery::run
        } else {
            background_recovery::removed
        };
        run(
            &cli,
            &provider,
            &mut custody,
            &clock,
            &prepared["preparation"]["subject"],
            &invocation,
        );
        return;
    }
    if mode == 4 {
        let mut running = OwnedProcess(cli.command(&invocation).spawn().unwrap());
        let until = Instant::now() + Duration::from_secs(10);
        while !provider.calls.lock().unwrap().iter().any(|path| {
            path.split('?').next() == Some("/api/v4/projects/org%2Fproject/merge_requests/4")
        }) {
            assert!(running.0.try_wait().unwrap().is_none());
            assert!(Instant::now() < until, "preflight did not reach provider");
            std::thread::sleep(Duration::from_millis(10));
        }
        let admitted = audits(&cli);
        assert_eq!(admitted.len(), 1);
        assert!(admitted[0]["final_observation"].is_null());
        success(
            cli.run(&[
                "connections",
                "revoke",
                "--adapter",
                "gitlab",
                "--connection",
                connection,
                "--expected-revision",
                connected["connection"]["summary"]["revision"]
                    .as_str()
                    .unwrap(),
            ]),
        );
        provider.merge_mode.store(0, Ordering::SeqCst);
        refusal(finish_process(running), "revoked");
        assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 0);
        assert!(
            !provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "PUT")
        );
        let completed = audits(&cli);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0]["reference"], admitted[0]["reference"]);
        assert_eq!(
            completed[0]["final_observation"]["outcome"], "refused",
            "{completed:?}"
        );
        assert_eq!(completed[0]["final_observation"]["code"], "revoked");
        return;
    }
    let (effect, error) = match mode {
        0 => ("applied", None),
        1 | 3 => ("unknown", Some("outcome_unknown")),
        _ => ("refused", Some("invalid_input")),
    };
    let original_owner = owner::Client::connect(&cli.paths, false)
        .unwrap()
        .host_incarnation;
    let first = if mode == 3 {
        let owner = owner_process(&cli);
        let mut running = OwnedProcess(cli.command(&invocation).spawn().unwrap());
        let until = Instant::now() + Duration::from_secs(10);
        while provider.merge_effects.load(Ordering::SeqCst) == 0 {
            assert!(
                running.0.try_wait().unwrap().is_none(),
                "merge ended before provider effect"
            );
            assert!(Instant::now() < until, "provider did not receive merge");
            std::thread::sleep(Duration::from_millis(10));
        }
        // A concurrent observer must leave the original live dispatch alone.
        let pending = refusal(cli.run(&invocation), "outcome_unknown");
        assert_eq!(pending["mutation"]["classification"], "unknown");
        assert_eq!(pending["mutation"]["replayed"], false);
        assert!(!pending["mutation"]["attempt"].is_null());
        kill_owner(owner);
        provider.merge_mode.store(0, Ordering::SeqCst);
        let lost = refusal(finish_process(running), "outcome_unknown");
        assert_eq!(lost["mutation"]["classification"], "unknown");
        pending
    } else {
        let output = cli.run(&invocation);
        match error {
            Some(code) => refusal(output, code),
            None => success(output),
        }
    };
    assert_eq!(first["mutation"]["classification"], effect, "{first}");
    assert_eq!(first["mutation"]["replayed"], false);
    assert_eq!(first["source_audit"]["audit_status"], "complete");
    if matches!(mode, 0 | 2) {
        let facts = cli.recorded();
        let original = facts.original(first["mutation"]["attempt"]["id"].as_str().unwrap());
        assert_eq!(
            (&*original.0, &*original.1),
            (if mode == 0 { "completed" } else { "failed" }, "replayable")
        );
        assert_eq!(original.3.unwrap() - original.2.unwrap(), 86_400_000);
        facts.cross_check(&cli.paths.state);
        let records = facts.audits();
        assert_eq!(records.len(), 1);
        assert!(!records[0]["final_observation"].is_null());
    }
    assert_eq!(
        provider.merge_effects.load(Ordering::SeqCst),
        usize::from(mode != 2)
    );
    assert_eq!(
        provider
            .methods
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.as_str() == "PUT")
            .count(),
        1
    );
    let mut reused = invocation.clone();
    let key_index = reused
        .iter()
        .position(|arg| *arg == "--idempotency-key")
        .unwrap()
        + 1;
    reused[key_index] = "second-owned-merge";
    if mode == 0 {
        // A merged MR representation is correctly refused by catalog preflight
        // before reaching the spend fence. Then deliberately supply the pinned
        // opened snapshot to isolate the independent spent-proof assertion.
        provider
            .merge_preflight_merged
            .store(true, Ordering::SeqCst);
        let blocked = refusal(cli.run(&reused), "forbidden");
        assert_eq!(blocked["mutation"]["classification"], "not_attempted");
        assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
        assert_eq!(
            provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .filter(|m| m.as_str() == "PUT")
                .count(),
            1
        );
        provider
            .merge_preflight_merged
            .store(false, Ordering::SeqCst);
    }
    if mode != 3 {
        let refused = refusal(cli.run(&reused), "approval_replayed");
        assert_eq!(refused["mutation"]["classification"], "not_attempted");
        assert!(
            refused["mutation"]["cause"].is_null(),
            "mode {mode}: refused preparation did not settle: {refused}"
        );
    }
    assert_eq!(
        provider
            .methods
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.as_str() == "PUT")
            .count(),
        1
    );
    let crash_proof = (mode == 3).then(|| Secret(fs::read(&proof).unwrap()));
    fs::remove_file(&proof).unwrap();
    let exiting = (mode != 3).then(|| owner_process(&cli));
    cli.shutdown();
    if let Some(exiting) = exiting {
        wait_owner(exiting);
    }
    drop(custody.daemon.take());
    let native_calls = provider.count();
    let clock_calls = clock.count.load(Ordering::SeqCst);
    // Retaining a now-missing proof path proves replay does not open it.
    let output = cli.run(&invocation);
    let replay = match error {
        Some(code) => refusal(output, code),
        None => success(output),
    };
    assert_eq!(replay["mutation"]["classification"], effect);
    assert_eq!(replay["mutation"]["attempt"], first["mutation"]["attempt"]);
    assert_eq!(
        replay["mutation"]["original_request_id"],
        first["mutation"]["original_request_id"]
    );
    assert_ne!(replay["request_id"], first["request_id"]);
    assert_eq!(replay["mutation"]["replayed"], true);
    assert_eq!(provider.count(), native_calls);
    assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls);
    if mode == 3 {
        assert_ne!(
            owner::Client::connect(&cli.paths, false)
                .unwrap()
                .host_incarnation,
            original_owner
        );
        assert!(cli.status()["child_incarnation"].is_null());
        assert_eq!(
            stored_attempt(&cli),
            ("indeterminate".into(), "quarantined".into(), None, None)
        );
    } else {
        assert!(
            std::os::unix::net::UnixStream::connect(cli.paths.state.join("owner.sock")).is_err()
        );
    }
    let changed = input.replace("\"pipeline_id\":12", "\"pipeline_id\":13");
    let mut conflict = invocation.clone();
    let i = conflict.iter().position(|s| *s == "--input-json").unwrap() + 1;
    conflict[i] = &changed;
    let refusal = refusal(cli.run(&conflict), "idempotency_conflict");
    assert!(refusal["mutation"]["attempt"].is_null());
    assert_eq!(provider.count(), native_calls);

    custody.start();
    cli.operation_result(connection, "project.get", json!({"id":"org/project"}));
    let restarted = owner::Client::connect(&cli.paths, false)
        .unwrap()
        .host_incarnation;
    assert_ne!(restarted, original_owner);
    let exiting = owner_process(&cli);
    cli.shutdown();
    wait_owner(exiting);
    drop(custody.daemon.take());
    if mode == 0 {
        // First establish real inherited-FD startup and complete lock handoff
        // independently of a replay request.
        owner_replay::FixtureOwner::start(&cli, provider.root.path()).shutdown();
    }
    let fixture_owner = owner_replay::FixtureOwner::start(&cli, provider.root.path());
    assert_ne!(fixture_owner.incarnation, restarted);
    let native_calls = provider.count();
    let clock_calls = clock.count.load(Ordering::SeqCst);
    let before_replay = audits(&cli);
    // Same-image host-library owner/2 segment. The original effect above came
    // through the production CLI owner. No custody or proof is available here.
    let replay = owner_replay(&cli, &request);
    assert_eq!(
        replay["mutation"],
        first["mutation"]
            .as_object()
            .map(|m| {
                let mut m = m.clone();
                m.insert("replayed".into(), json!(true));
                Value::Object(m)
            })
            .unwrap()
    );
    assert_ne!(replay["request_id"], first["request_id"]);
    assert_eq!(replay["source_audit"]["audit_status"], "complete");
    match error {
        Some(code) => assert_eq!(replay["error"]["code"], code),
        None => {
            let expected: Value = first["result"].clone();
            assert_eq!(replay["result"], expected);
            assert!(replay["error"].is_null());
        }
    }
    assert!(fixture_owner.status()["child_incarnation"].is_null());
    let after_replay = audits(&cli);
    assert_eq!(after_replay.len(), before_replay.len() + 1);
    for record in &before_replay {
        assert!(after_replay.contains(record));
    }
    let added: Vec<_> = after_replay
        .iter()
        .filter(|record| !before_replay.contains(record))
        .collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0]["anchor"]["request_id"], replay["request_id"]);
    assert_eq!(
        added[0]["reference"]["audit_ref"],
        replay["source_audit"]["audit_ref"]
    );
    assert!(!added[0]["final_observation"].is_null());
    assert_eq!(provider.count(), native_calls);
    assert_eq!(
        clock.count.load(Ordering::SeqCst),
        clock_calls,
        "mode {mode}: clock access during settled owner replay"
    );
    assert_eq!(
        provider.merge_effects.load(Ordering::SeqCst),
        usize::from(mode != 2)
    );
    if let Some(proof) = crash_proof {
        custody.start();
        let deadline = mutation::Deadline::start().unwrap();
        let refused = owner::WriteClient::connect(&cli.paths, false, deadline)
            .unwrap()
            .invoke(
                "gitlab",
                &request,
                Some("second-owned-merge"),
                Some(&proof),
                deadline,
            )
            .unwrap();
        let refused = serde_json::to_value(refused).unwrap();
        assert_eq!(refused["error"]["code"], "approval_replayed");
        assert_eq!(refused["mutation"]["classification"], "not_attempted");
    }
    assert_eq!(
        provider
            .methods
            .lock()
            .unwrap()
            .iter()
            .filter(|method| method.as_str() == "PUT")
            .count(),
        1
    );
    fixture_owner.shutdown();
}
