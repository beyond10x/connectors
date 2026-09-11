//! Actual CLI/owner/private child, disposable HTTPS, signing clock and qualified
//! keyring. Synthetic clock/provider responses do not prove sandbox acceptance.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use connectors_host::local::owner::{approval_issuance, mutation};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// An exact kernel handle obtained from this test's private owner socket.
/// The crash fixture requires Linux SO_PEERPIDFD, not a numeric PID kill.
fn owner_process(cli: &Cli) -> (OwnedFd, Vec<OwnedFd>) {
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
    assert_eq!(
        children.len(),
        1,
        "fixture must own exactly one native child"
    );
    (owner, children)
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
        .invoke("forge", request, Some("owned-merge"), None, deadline)
        .unwrap();
    serde_json::to_value(delivery).unwrap()
}

fn finish_process(mut running: OwnedProcess) -> Output {
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
    let database = rusqlite::Connection::open_with_flags(
        cli.paths.state.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    database
        .prepare("SELECT record_json FROM execution_audits WHERE instance_id='fixture-gitlab'")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
        .collect()
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
            while !stop.load(Ordering::SeqCst) {
                match socket.recv_from(&mut buffer) {
                    Ok((size, peer)) => {
                        calls.fetch_add(1, Ordering::SeqCst);
                        if !replies.load(Ordering::SeqCst) {
                            continue;
                        }
                        socket
                            .send_to(
                                &clock_fixture::Fixture::default().reply(&buffer[..size]),
                                peer,
                            )
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
fn gitlab_cli_guarded_merge_applied_refused_and_lost_response_restart() {
    for mode in [0, 1, 2, 3] {
        journey(mode);
    }
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn gitlab_cli_guarded_merge_revocation_finishes_admitted_audit() {
    journey(4);
}

#[test]
#[ignore = "requires built production CLI, qualified GNOME, dbus-daemon and task-owned TMPDIR"]
fn gitlab_cli_recovers_abandoned_preparation_only_with_trusted_time() {
    journey(5);
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
                adapter: "gitlab".into(),
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
        caller_key: Some("owned-merge".into()),
        request_id: "a6b7af40-a60f-4a73-a4b2-fd247c773c11".into(),
        // This process exercises the durable port, not native preflight or
        // approval spending. No provider capability or dispatch gate is used.
        approval: ledger::Approval::NotRequired,
    };
    assert!(matches!(
        store.prepare(&candidate).unwrap(),
        ledger::Preparation::Prepared(_)
    ));
    std::process::exit(73);
}

fn stored_attempt(cli: &Cli) -> (String, String, Option<i64>, Option<i64>) {
    let db = rusqlite::Connection::open_with_flags(
        cli.paths.state.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    db.query_row("SELECT a.state,k.state,k.settled_at_ms,k.replay_expires_at_ms FROM mutation_attempts a JOIN mutation_keys k ON k.attempt_id=a.attempt_id", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap()
}

fn journey(mode: u8) {
    let provider = Provider::new();
    provider.merge_mode.store(mode, Ordering::SeqCst);
    let mut custody = Custody::new(provider.root.path());
    let clock = Clock::new();
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let config = fs::read_to_string(&cli.paths.config)
        .unwrap()
        .replace("format='connectors-local/1'", "format='connectors-local/2'")
        .replace(
            "[adapters.forge]\n",
            "[adapters.forge]\nprivate_protocol='connectors-private/2'\n",
        )
        .replace(
            "operations=['project.get','issues.list','file.get']",
            "operations=['project.get','merge_request.merge']",
        );
    private(&cli.paths.config, format!("{config}\n[approval_clock]\nformat='roughtime-clock/1'\naddress='{}'\npublic_key='{}'\nmax_rate_error_ppm=100\n", clock.address, STANDARD.encode(clock_fixture::root_key())).as_bytes());
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "forge",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap();
    fs::remove_file(&credential).unwrap();
    success(cli.run(&["approvals", "key-init", "--adapter", "forge"]));
    let policy = provider.root.path().join("private/policy.json");
    private(&policy, br#"{"operations":["merge_request.merge"]}"#);
    success(cli.run(&[
        "approvals",
        "policy-set",
        "--adapter",
        "forge",
        "--input-file",
        policy.to_str().unwrap(),
    ]));
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "merge_request.merge",
    ]));
    let input = json!({"project":"org/project","iid":4,"sha":"0123456789abcdef0123456789abcdef01234567","pipeline_id":12}).to_string();
    let request = approval_issuance::Request {
        connection,
        operation: "merge_request.merge",
        schema: description["schema"].as_str().unwrap(),
        revision: description["revision"].as_str().unwrap(),
        input: &input,
    };
    let target = [
        "--adapter",
        "forge",
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
        assert_eq!(
            stored_attempt(&cli),
            ("prepared".into(), "pending".into(), None, None)
        );
        let native_calls = provider.count();
        let clock_calls = clock.count.load(Ordering::SeqCst);
        clock.respond.store(false, Ordering::SeqCst);
        let pending = refusal(cli.run(&invocation), "outcome_unknown");
        assert_eq!(pending["mutation"]["classification"], "unknown");
        assert_eq!(pending["mutation"]["replayed"], false);
        assert_eq!(stored_attempt(&cli).0, "prepared");
        assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls + 1);
        assert_eq!(pending["mutation"]["cause"]["code"], "unavailable");
        assert!(cli.status()["child_incarnation"].is_null());
        clock.respond.store(true, Ordering::SeqCst);
        let recovered = refusal(cli.run(&invocation), "interrupted");
        assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls + 2);
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
                "forge",
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
        _ => ("refused", Some("forbidden")),
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
        let pending = owner_replay(&cli, &request);
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
    if mode != 3 {
        let refused = refusal(cli.run(&reused), "approval_replayed");
        assert_eq!(refused["mutation"]["classification"], "not_attempted");
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
    cli.shutdown();
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
    cli.operation_result(connection, "project.get", json!({"project":"org/project"}));
    let restarted = owner::Client::connect(&cli.paths, false)
        .unwrap()
        .host_incarnation;
    assert_ne!(restarted, original_owner);
    drop(custody.daemon.take());
    let native_calls = provider.count();
    let clock_calls = clock.count.load(Ordering::SeqCst);
    // Contact the new owner/2 directly; CLI passive observation cannot
    // satisfy this check. Neither custody nor proof bytes are available.
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
    assert_eq!(provider.count(), native_calls);
    assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls);
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
                "forge",
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
}
