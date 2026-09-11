//! Actual CLI/owner/private child, disposable HTTPS, signing clock and qualified
//! keyring. Synthetic clock/provider responses do not prove sandbox acceptance.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Clock {
    address: String,
    count: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
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
        let stop = stopped.clone();
        let calls = count.clone();
        let thread = std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            while !stop.load(Ordering::SeqCst) {
                match socket.recv_from(&mut buffer) {
                    Ok((size, peer)) => {
                        calls.fetch_add(1, Ordering::SeqCst);
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
    for mode in [0, 1, 2] {
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
        let (effect, error) = match mode {
            0 => ("applied", None),
            1 => ("unknown", Some("outcome_unknown")),
            _ => ("refused", Some("forbidden")),
        };
        let output = cli.run(&invocation);
        let first = match error {
            Some(code) => refusal(output, code),
            None => success(output),
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
        let refused = refusal(cli.run(&reused), "approval_replayed");
        assert_eq!(refused["mutation"]["classification"], "not_attempted");
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
            first["request_id"]
        );
        assert_ne!(replay["request_id"], first["request_id"]);
        assert_eq!(replay["mutation"]["replayed"], true);
        assert_eq!(provider.count(), native_calls);
        assert_eq!(clock.count.load(Ordering::SeqCst), clock_calls);
        assert!(!cli.paths.state.join("owner.sock").exists());
        let changed = input.replace("\"pipeline_id\":12", "\"pipeline_id\":13");
        let mut conflict = invocation.clone();
        let i = conflict.iter().position(|s| *s == "--input-json").unwrap() + 1;
        conflict[i] = &changed;
        let refusal = refusal(cli.run(&conflict), "idempotency_conflict");
        assert!(refusal["mutation"]["attempt"].is_null());
        assert_eq!(provider.count(), native_calls);
    }
}
