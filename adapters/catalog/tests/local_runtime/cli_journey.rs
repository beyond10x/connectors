//! The shipped GitLab catalog selection through the production CLI, owner,
//! adapter child, local TLS fixture and disposable Secret Service.
use super::*;
use connectors_host::local::{config::Paths, keyring, owner};
use std::{
    io::Write,
    process::{Child as Process, Output, Stdio},
    time::Instant,
};

#[path = "guarded_merge.rs"]
mod guarded_merge;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "reason_cli_adversary.rs"]
mod reason_cli_adversary;
#[path = "retry_after_cli.rs"]
mod retry_after_cli;
#[path = "settlement_fault.rs"]
mod settlement_fault;

struct OwnedProcess(Process);

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) struct Custody {
    daemon: Option<OwnedProcess>,
    _bus: OwnedProcess,
    directory: PathBuf,
    pub(super) socket: PathBuf,
    epoch: u32,
}

impl Custody {
    pub(super) fn new(parent: &Path) -> Self {
        let directory = parent.join("custody");
        filesystem::directory(&directory, true, true).unwrap();
        filesystem::directory(&directory.join("home"), true, true).unwrap();
        filesystem::directory(&directory.join("data/keyrings"), true, true).unwrap();
        let socket = directory.join("bus");
        let bus = OwnedProcess(
            Command::new("/usr/bin/dbus-daemon")
                .args(["--session", "--nofork", "--nopidfile"])
                .arg(format!("--address=unix:path={}", socket.display()))
                .env_clear()
                .env("PATH", "/usr/bin")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "fixture bus did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut fixture = Self {
            daemon: None,
            _bus: bus,
            directory,
            socket,
            epoch: 0,
        };
        fixture.start();
        fixture
    }

    fn start(&mut self) {
        assert!(self.daemon.is_none());
        self.epoch += 1;
        let runtime = self.directory.join(format!("r{}", self.epoch));
        filesystem::directory(&runtime, true, true).unwrap();
        let mut daemon = OwnedProcess(
            Command::new("/usr/bin/gnome-keyring-daemon")
                .args([
                    "--foreground",
                    "--components=secrets",
                    "--unlock",
                    "--control-directory",
                ])
                .arg(&runtime)
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("HOME", self.directory.join("home"))
                .env("XDG_DATA_HOME", self.directory.join("data"))
                .env("XDG_RUNTIME_DIR", &runtime)
                .env(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={}", self.socket.display()),
                )
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        daemon
            .0
            .stdin
            .take()
            .unwrap()
            .write_all(b"fictional-catalog-fixture-keyring-password")
            .unwrap();
        self.daemon = Some(daemon);
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&self.socket)) {
            assert!(Instant::now() < until, "fixture custody unavailable");
            assert!(
                self.daemon
                    .as_mut()
                    .unwrap()
                    .0
                    .try_wait()
                    .unwrap()
                    .is_none()
            );
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    fn restart(&mut self) {
        drop(self.daemon.take());
        self.start();
    }
}

pub(super) struct Cli {
    binary: PathBuf,
    pub(super) paths: Paths,
    recorded: std::cell::RefCell<Option<recorded_state::Reader>>,
    fault: Option<settlement_fault::Controller>,
}

impl Cli {
    pub(super) fn new(root: &Path) -> Self {
        let binary = std::env::var_os("CONNECTORS_TEST_CLI")
            .expect("built production CLI required")
            .into();
        Self {
            binary,
            recorded: std::cell::RefCell::new(None),
            fault: None,
            paths: Paths {
                config: root.join("cli/config.toml"),
                state: root.join("cli/state"),
            },
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(&self.paths.config)
            .arg("--state-dir")
            .arg(&self.paths.state)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(fault) = &self.fault {
            fault.configure(&mut command);
        }
        command
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn status(&self) -> Value {
        success(self.run(&["adapters", "status", "--adapter", "gitlab"]))["observation"].clone()
    }

    fn recorded(&self) -> recorded_state::Facts {
        let mut reader = self.recorded.borrow_mut();
        reader
            .get_or_insert_with(|| recorded_state::Reader::open(&self.paths.state))
            .snapshot()
    }

    fn operation_result(&self, connection: &str, operation: &str, input: Value) -> Value {
        let description = success(self.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            operation,
        ]));
        success(self.run(&[
            "operations",
            "invoke",
            "--adapter",
            "gitlab",
            "--connection",
            connection,
            "--operation",
            operation,
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &input.to_string(),
        ]))
    }

    /// `run`, with the consent hook of a debug-build CLI following the
    /// authorize URL itself, trusting the fixture root `ca`, where a person
    /// would open it in a browser. A release build has no such hook.
    fn run_following(&self, args: &[&str], ca: &Path) -> Output {
        self.command(args)
            .env("CONNECTORS_TEST_OAUTH_FOLLOW", ca)
            .output()
            .unwrap()
    }

    fn run_with_stdin(&self, args: &[&str], input: &[u8]) -> Output {
        let mut process = self.command(args).stdin(Stdio::piped()).spawn().unwrap();
        process.stdin.take().unwrap().write_all(input).unwrap();
        process.wait_with_output().unwrap()
    }

    fn shutdown(&self) {
        if let Some(fault) = &self.fault {
            fault.disarm();
        }
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            if self.fault.is_some() {
                let handles = guarded_merge::owner_handles(self);
                let answer = client.shutdown(&host);
                if !guarded_merge::finish_filtered_owner(handles, answer.is_ok()) {
                    return;
                }
                let directory = filesystem::directory(&self.paths.state, false, true).unwrap();
                let lock =
                    filesystem::private_file_at(&directory, std::ffi::OsStr::new("owner.lock"))
                        .unwrap();
                // SAFETY: verified exact owned lifetime file, after pidfd exit.
                use std::os::fd::AsRawFd;
                assert_eq!(
                    unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
                    0
                );
                eprintln!("filtered owner lifetime lock reacquired after exact process exits");
            } else {
                client.shutdown(&host).unwrap();
            }
            let until = Instant::now() + Duration::from_secs(5);
            while self.paths.state.join("owner.sock").exists() {
                assert!(Instant::now() < until, "owner did not finish cleanup");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

impl Drop for Cli {
    fn drop(&mut self) {
        self.shutdown();
        // Observational bridge retires while all fault listeners still exist.
        drop(self.recorded.get_mut().take());
    }
}

#[track_caller]
pub(super) fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-pat-one"));
    assert!(!carries_basic_material(&output.stdout));
    value["result"].clone()
}

#[track_caller]
fn refusal(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-pat-one"));
    assert!(!carries_basic_material(&output.stderr));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

fn configure(cli: &Cli, provider: &Provider, custody: &Custody) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = provider.selection();
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.gitlab]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.gitlab.permissions]\nprofiles=[{}]\noperations=['project.get','issues.list']\n[adapters.gitlab.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(provider.profile),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            "missing",
            "--credential-file",
            "/never-read",
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), 0);

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
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(connected["summary"]["state"], "ready");
    assert_eq!(provider.count(), 2);

    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let schema = description["schema"].as_str().unwrap().to_owned();
    let descriptor = description["revision"].as_str().unwrap().to_owned();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
        "--operation",
        "project.get",
        "--schema",
        &schema,
        "--revision",
        &descriptor,
        "--input-json",
        r#"{"id":"org/project"}"#,
    ];
    let result = success(cli.run(&invoke));
    assert_eq!(result["result"]["body"]["id"], 7);

    let old_host = success(cli.run(&["adapters", "status", "--adapter", "gitlab"]))["observation"]
        ["host_incarnation"]
        .clone();
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let calls_before_refusal = provider.count();
    refusal(
        cli.run(&[
            "connections",
            "revalidate",
            "--adapter",
            "gitlab",
            "--connection",
            &reference,
            "--expected-revision",
            "wrong-revision",
        ]),
        "lifecycle_conflict",
    );
    assert_eq!(provider.count(), calls_before_refusal);
    assert!(!cli.paths.state.join("owner.sock").exists());
    // The saved credential survives the restart. Refresh its bounded provider
    // evidence explicitly before a new dispatch, even when the journey itself
    // took longer than the original evidence lifetime.
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    assert_eq!(provider.count(), calls_before_refusal + 2);
    // Revalidation is explicit. Start the simultaneous invocations with no
    // owner so they also exercise concurrent owner creation and saved custody.
    cli.shutdown();
    let calls_before = provider.count();
    let children: Vec<_> = (0..4)
        .map(|_| cli.command(&invoke).spawn().unwrap())
        .collect();
    for child in children {
        let result = success(child.wait_with_output().unwrap());
        assert_eq!(result["result"]["body"]["id"], 7);
    }
    assert_eq!(provider.count(), calls_before + 4);
    let new_host = success(cli.run(&["adapters", "status", "--adapter", "gitlab"]))["observation"]
        ["host_incarnation"]
        .clone();
    assert_ne!(new_host, old_host);
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    let before_refusal = provider.count();
    refusal(
        cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            "file.get",
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), before_refusal);

    let coordinates = cli.status();
    let stop = |child: &str| {
        cli.run(&[
            "adapters",
            "stop",
            "--adapter",
            "gitlab",
            "--expected-revision",
            coordinates["configuration_revision"].as_str().unwrap(),
            "--host-incarnation",
            coordinates["host_incarnation"].as_str().unwrap(),
            "--child-incarnation",
            child,
        ])
    };
    refusal(stop("stale-child"), "incarnation_mismatch");
    assert_eq!(
        cli.status()["child_incarnation"],
        coordinates["child_incarnation"]
    );
    success(stop(coordinates["child_incarnation"].as_str().unwrap()));
    assert_eq!(cli.status()["state"], "suppressed");
    cli.shutdown();
    assert_eq!(cli.status()["state"], "owner_unavailable");
    success(cli.run(&invoke));
    assert_ne!(
        cli.status()["child_incarnation"],
        coordinates["child_incarnation"]
    );

    success(
        cli.run(&[
            "connections",
            "revoke",
            "--adapter",
            "gitlab",
            "--connection",
            &reference,
            "--expected-revision",
            current["connection"]["summary"]["revision"]
                .as_str()
                .unwrap(),
        ]),
    );
    cli.shutdown();
    let revoked = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "gitlab",
        "--connection",
        &reference,
    ]));
    assert_eq!(revoked["connection"]["summary"]["state"], "revoked");
    let before = provider.count();
    refusal(cli.run(&invoke), "revoked");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
}

/// story:gitlab-feed-binding through the production CLI: a saved GitLab
/// connection finds the merge request feed by its family, lists the member
/// projects across a full page, reads one project's first page of merge
/// requests and resumes from the watermark that page returned.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn gitlab_catalog_cli_reads_the_merge_request_feed_through_a_saved_connection() {
    const FAMILY: &str = "datasource.feed/v1alpha1";
    const PROFILE: &str = "gitlab-merge-requests/1";
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let configured = fs::read_to_string(&cli.paths.config).unwrap();
    let permitted = configured.replace(
        "operations=['project.get','issues.list']",
        "operations=['project.get','feed.containers','feed.items']",
    );
    assert_ne!(permitted, configured);
    private(&cli.paths.config, permitted.as_bytes());
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let reference = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::remove_file(&credential).unwrap();

    let listed = success(cli.run(&[
        "operations",
        "list",
        "--adapter",
        "gitlab",
        "--family",
        FAMILY,
    ]));
    let family: Vec<(&str, &str, &str)> = listed["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["id"].as_str().unwrap(),
                o["contract"].as_str().unwrap(),
                o["profile"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        family,
        [
            ("feed.containers", FAMILY, PROFILE),
            ("feed.items", FAMILY, PROFILE)
        ]
    );
    assert!(listed.get("next_cursor").is_none(), "{listed}");

    let ids = |page: &Value, field: &str| -> Vec<String> {
        page[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["id"].as_str().unwrap().to_owned())
            .collect()
    };
    // The family's page an invoke answered, as the JSON value the answer
    // carries (story:owner-read-answers-json-value).
    let read = |operation: &str, input: Value| -> Value {
        let invoked = cli.operation_result(&reference, operation, input);
        assert!(invoked["result"].is_object(), "{operation}: {invoked:#}");
        invoked["result"].clone()
    };
    let first = read("feed.containers", json!({"limit": 2}));
    assert_eq!(
        first["containers"][0],
        json!({"id": "1", "name": "Org / Project 1", "kind": "project", "visibility": "private"}),
        "{first:#}"
    );
    assert_eq!(ids(&first, "containers"), ["1", "2"]);
    assert_eq!(first["complete"], false);
    let rest = read(
        "feed.containers",
        json!({"limit": 2, "cursor": first["next_cursor"]}),
    );
    assert_eq!(ids(&rest, "containers"), ["3"]);
    assert_eq!(rest["complete"], true);
    assert_eq!(rest["next_cursor"], Value::Null);

    let page = read("feed.items", json!({"container": "1", "limit": 2}));
    assert_eq!(ids(&page, "items"), ["1", "2"]);
    assert_eq!(page["complete"], false);
    assert_eq!(page["provenance"]["profile"], PROFILE);
    let first_item = &page["items"][0];
    assert_eq!(first_item["revision"], "2026-10-06T09:00:00.081Z");
    assert_eq!(first_item["updated_at"], first_item["revision"]);
    assert_eq!(first_item["url"], Value::Null);
    assert_eq!(first_item["deleted"], false);
    assert_eq!(
        first_item["author"],
        json!({"id": "42", "display_name": "Fixture Member"})
    );
    assert_eq!(first_item["body"]["content"], "Change 1");
    let resumed = read(
        "feed.items",
        json!({"container": "1", "limit": 2, "watermark": page["next_watermark"]}),
    );
    assert_eq!(ids(&resumed, "items"), ["3"]);
    assert_eq!(resumed["complete"], true);

    // The requests the binding sent, each query as a set of pairs.
    let sent: Vec<(String, std::collections::BTreeSet<String>)> = provider
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.starts_with("/api/v4/projects"))
        .map(|path| {
            let (route, query) = path.split_once('?').unwrap_or((path, ""));
            let pairs = query
                .split('&')
                .filter(|pair| !pair.is_empty())
                .map(str::to_owned)
                .collect();
            (route.to_owned(), pairs)
        })
        .collect();
    let request = |route: &str, pairs: &[&str]| {
        (
            route.to_owned(),
            pairs.iter().map(|pair| (*pair).to_owned()).collect(),
        )
    };
    let listing = ["membership=true", "order_by=id", "sort=asc", "per_page=2"];
    let merge_requests = ["order_by=updated_at", "sort=asc", "state=all", "scope=all"];
    assert_eq!(
        sent,
        [
            request("/api/v4/projects", &listing),
            request(
                "/api/v4/projects",
                &[&listing[..], &["id_after=2"]].concat()
            ),
            request("/api/v4/projects/1", &[]),
            request(
                "/api/v4/projects/1/merge_requests",
                &[&merge_requests[..], &["per_page=2"]].concat()
            ),
            request("/api/v4/projects/1", &[]),
            request(
                "/api/v4/projects/1/merge_requests",
                &[
                    &merge_requests[..],
                    &["per_page=3", "updated_after=2026-10-06T09%3A00%3A01.000Z"]
                ]
                .concat()
            ),
        ]
    );
}

fn arguments(owned: &[String]) -> Vec<&str> {
    owned.iter().map(String::as_str).collect()
}

fn files_under(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            files_under(&entry.path(), found);
        } else if kind.is_file() {
            found.push(entry.path());
        }
    }
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code() {
    let provider = Provider::basic("api");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let connect = |source: &[&str]| {
        let mut args = vec![
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            BASIC_PROFILE,
        ];
        args.extend_from_slice(source);
        args.into_iter().map(str::to_owned).collect::<Vec<_>>()
    };

    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &basic(BASIC_TOKEN).0);
    let by_file = connect(&["--credential-file", credential.to_str().unwrap()]);
    let connected = success(cli.run(&arguments(&by_file)))["connection"].clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let identity = provider.authorizations.lock().unwrap().clone();
    assert!(
        identity
            .iter()
            .any(|(route, value)| route == "/api/v4/user" && value.as_deref() == Some(BASIC_HEADER))
    );

    let by_stdin = connect(&["--credential-stdin"]);
    let connected =
        success(cli.run_with_stdin(&arguments(&by_stdin), &basic(BASIC_TOKEN).0))["connection"]
            .clone();
    assert_eq!(connected["summary"]["state"], "ready");

    let before = provider.authorizations.lock().unwrap().len();
    refusal(
        cli.run_with_stdin(&arguments(&by_stdin), &basic(BASIC_WRONG_TOKEN).0),
        "service_failure",
    );
    assert!(provider.authorizations.lock().unwrap().len() > before);

    // Written configuration, owner state, and every provider child's argv and
    // environment carry neither the account nor the token.
    let mut written = vec![cli.paths.config.clone(), provider.config.clone()];
    files_under(&cli.paths.state, &mut written);
    for path in &written {
        let bytes = fs::read(path).unwrap();
        assert!(!carries_basic_material(&bytes), "{}", path.display());
    }
    assert_children_carry_no_basic_material(&provider.config);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn basic_catalog_cli_refuses_a_credential_below_minimum_scopes() {
    let provider = Provider::basic("admin");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &basic(BASIC_TOKEN).0);
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            BASIC_PROFILE,
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "not_granted",
    );
    assert!(provider.count() > 0);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary_basic_cli_wrong_token_is_unauthorized_and_malformed_is_invalid_input() {
    let provider = Provider::basic("api");
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let args = [
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        BASIC_PROFILE,
        "--credential-stdin",
    ];
    let wrong = cli.run_with_stdin(&args, &basic(BASIC_WRONG_TOKEN).0);
    assert!(!carries_basic_material(&wrong.stderr));
    let value: Value = serde_json::from_slice(&wrong.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], "service_failure", "{value}");
    assert_eq!(
        value["error"]["data"]["service_code"], "unauthorized",
        "{value}"
    );

    let before = provider.count();
    refusal(
        cli.run_with_stdin(&args, br#"{"token":"fixture-api-token-one"}"#),
        "invalid_input",
    );
    assert_eq!(provider.count(), before);
}

#[track_caller]
pub(super) fn refused_data(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-pat-one"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

/// One connection reads a project the provider answers and a project the
/// provider refuses. The provider's refusal is reported at `dispatch`; the
/// host's own refusal of an operation the configuration does not grant still
/// reads `admission`.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_provider_refusal_reads_dispatch_and_a_host_refusal_reads_admission() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let reference = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let schema = description["schema"].as_str().unwrap().to_owned();
    let descriptor = description["revision"].as_str().unwrap().to_owned();
    let read = |operation: &str, project: &str| {
        let input = json!({"id": project}).to_string();
        cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "gitlab",
            "--connection",
            &reference,
            "--operation",
            operation,
            "--schema",
            &schema,
            "--revision",
            &descriptor,
            "--input-json",
            &input,
        ])
    };

    let answered = success(read("project.get", "org/project"));
    assert_eq!(answered["result"]["body"]["id"], 7);

    let before = provider.count();
    let refused = refused_data(read("project.get", "org/fixture-refused"));
    assert!(provider.count() > before, "the provider was asked");
    assert_eq!(refused["code"], "forbidden", "{refused}");
    assert_eq!(refused["stage"], "dispatch", "{refused}");
    assert_eq!(refused["next_action"], "request_permission", "{refused}");

    let missing = refused_data(read("project.get", "org/fixture-missing"));
    assert_eq!(missing["stage"], "dispatch", "{missing}");
    assert_eq!(missing["next_action"], "none", "{missing}");
    assert_eq!(missing["code"], "service_failure", "{missing}");
    assert_eq!(missing["service_code"], "not_found", "{missing}");

    // The same connection still reads the answered project afterwards.
    success(read("project.get", "org/project"));

    let before = provider.count();
    // `file.get` is not among the configuration's granted operations.
    let admission = refused_data(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "file.get",
    ]));
    assert_eq!(provider.count(), before, "the provider was never asked");
    assert_eq!(admission["code"], "forbidden", "{admission}");
    assert_eq!(admission["stage"], "admission", "{admission}");
    assert_eq!(
        admission["next_action"], "request_permission",
        "{admission}"
    );
}

/// A Google-shaped OAuth connection through the production CLI. One fixture
/// host serves consent, token and API; `connections connect` and `repair`
/// receive Google's installed-client download instead of an entry, and the
/// debug-build consent hook follows the authorize URL in place of a browser.
struct OAuthJourney {
    // Dropped in this order: the owner stops before custody and the fixture.
    cli: Cli,
    client_file: PathBuf,
    _custody: Custody,
    provider: super::oauth2_refresh::OAuthProvider,
}

impl OAuthJourney {
    fn new() -> Self {
        use super::oauth2_refresh::*;
        let provider = OAuthProvider::new(Source::Api);
        let authorize_url = format!("https://localhost:{}/authorize", provider.port);
        let token_url = format!("https://localhost:{}/token", provider.port);
        provider.write_config(&oauth_config(
            provider.port,
            &provider.ca,
            Source::Api,
            &token_url,
            &authorize_url,
        ));
        let custody = Custody::new(provider.root.path());
        let cli = Cli::new(provider.root.path());
        success(cli.run(&["setup", "init"]));
        success(cli.run(&["setup", "check"]));
        let adapter = provider.selection();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let configuration = format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.drive]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.drive.permissions]\nprofiles=[{}]\noperations=['project.get']\n[adapters.drive.executable]\npath={}\nsha256={}\nargs={}\n",
            filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&adapter.instance_id),
            q(&adapter.configuration_revision),
            q(OAUTH_PROFILE),
            q(adapter.executable.path.to_str().unwrap()),
            q(&adapter.executable.sha256),
            serde_json::to_string(&adapter.executable.args).unwrap()
        );
        private(&cli.paths.config, configuration.as_bytes());
        // The shape Google's console downloads for a Desktop app client.
        let client_file = provider.root.path().join("private/client_secret.json");
        private(
            &client_file,
            &serde_json::to_vec(&json!({"installed": {
                "client_id": CLIENT_ID,
                "project_id": "fixture-project",
                "auth_uri": authorize_url,
                "token_uri": token_url,
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": CLIENT_SECRET,
                "redirect_uris": ["http://localhost"],
            }}))
            .unwrap(),
        );
        Self {
            cli,
            client_file,
            _custody: custody,
            provider,
        }
    }

    /// Runs `args` with the consent hook, and checks that no OAuth material
    /// reaches either output stream.
    fn follow(&self, args: &[&str]) -> Output {
        let output = self.cli.run_following(args, &self.provider.ca);
        self.clean(&output);
        output
    }

    fn clean(&self, output: &Output) {
        use super::oauth2_refresh::carries_oauth_material;
        assert!(
            !carries_oauth_material(&output.stdout),
            "material on stdout"
        );
        assert!(
            !carries_oauth_material(&output.stderr),
            "material on stderr"
        );
    }

    fn connect(&self) -> Value {
        success(self.follow(&[
            "connections",
            "connect",
            "--adapter",
            "drive",
            "--profile",
            super::oauth2_refresh::OAUTH_PROFILE,
            "--credential-file",
            self.client_file.to_str().unwrap(),
        ]))["connection"]
            .clone()
    }

    fn invoke(&self, reference: &str) -> Output {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "drive",
            "--operation",
            "project.get",
        ]));
        let output = self.cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "drive",
            "--connection",
            reference,
            "--operation",
            "project.get",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            r#"{"id":"org/project"}"#,
        ]);
        self.clean(&output);
        output
    }

    /// The form of every token request, in order.
    fn token_forms(&self) -> Vec<Vec<(String, String)>> {
        self.provider
            .requests()
            .iter()
            .filter(|request| request.route == "/token")
            .map(|request| super::oauth2_refresh::decode_form(&request.body))
            .collect()
    }

    /// Whether a refresh-token exchange presented the `n`th acquired token.
    fn refreshed_with(&self, n: u32) -> bool {
        use super::oauth2_refresh::{ACQUIRED_REFRESH_TOKEN, field};
        let acquired = format!("{ACQUIRED_REFRESH_TOKEN}{n}");
        self.token_forms().iter().any(|form| {
            field(form, "grant_type") == Some("refresh_token")
                && field(form, "refresh_token") == Some(acquired.as_str())
        })
    }

    /// The configuration, the owner's state and the provider children carry
    /// no OAuth material.
    fn assert_no_material_at_rest(&self) {
        use super::oauth2_refresh::carries_oauth_material;
        let mut written = vec![self.cli.paths.config.clone(), self.provider.config.clone()];
        files_under(&self.cli.paths.state, &mut written);
        for path in &written {
            let bytes = fs::read(path).unwrap_or_default();
            assert!(!carries_oauth_material(&bytes), "{}", path.display());
        }
    }
}

#[track_caller]
fn read_project(output: Output) {
    let result = success(output);
    assert_eq!(result["result"]["body"]["id"], 7);
}

/// `connections connect` with Google's client file runs consent against the
/// fixture authorize/token host and stores the acquired triple; the connection
/// is ready and an invoke reads with a fixture access token.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth_connect_journey() {
    use super::oauth2_refresh::{ACCESS_TOKEN, DRIVE_SCOPE, decode_form, field};
    let journey = OAuthJourney::new();
    let connected = journey.connect();
    assert_eq!(connected["summary"]["state"], "ready");
    let reference = connected["summary"]["connection"].as_str().unwrap();
    assert_eq!(journey.provider.acquired(), 1, "one code was exchanged");

    let requests = journey.provider.requests();
    let consent: Vec<_> = requests
        .iter()
        .filter(|request| request.route == "/authorize")
        .collect();
    assert_eq!(consent.len(), 1, "consent was requested once");
    let query = decode_form(consent[0].query.as_bytes());
    assert_eq!(field(&query, "code_challenge_method"), Some("S256"));
    assert_eq!(field(&query, "access_type"), Some("offline"));
    assert_eq!(field(&query, "prompt"), Some("consent"));
    let scopes: std::collections::BTreeSet<&str> =
        field(&query, "scope").unwrap().split(' ').collect();
    assert_eq!(scopes, ["openid", DRIVE_SCOPE].into());
    let codes = journey
        .token_forms()
        .iter()
        .filter(|form| field(form, "grant_type") == Some("authorization_code"))
        .count();
    assert_eq!(codes, 1);
    // The owner validated the stored triple by a refresh of its own.
    assert!(journey.refreshed_with(1));

    read_project(journey.invoke(reference));
    let used = journey.provider.api_authorizations();
    let bearer = used.last().unwrap().as_deref().unwrap();
    assert!(bearer.starts_with(&format!("Bearer {ACCESS_TOKEN}")));
    journey.assert_no_material_at_rest();
}

/// A revoked grant is reported as `repair_connection`; `connections repair`
/// with the same client file runs consent again, and the next invoke reads
/// with an access token from the new refresh token.
#[test]
#[ignore = "requires built debug production CLI and qualified disposable Secret Service"]
fn oauth_repair_journey() {
    use super::oauth2_refresh::ACQUIRED_REFRESH_TOKEN;
    let journey = OAuthJourney::new();
    let connected = journey.connect();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    read_project(journey.invoke(&reference));
    let before = journey.provider.api_authorizations();

    journey
        .provider
        .revoke(&format!("{ACQUIRED_REFRESH_TOKEN}1"));
    // A new owner and child hold no cached access token.
    journey.cli.shutdown();
    let refused = refused_data(journey.invoke(&reference));
    assert_eq!(refused["code"], "service_failure", "{refused}");
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(refused["next_action"], "repair_connection", "{refused}");

    let status = success(journey.cli.run(&[
        "connections",
        "status",
        "--adapter",
        "drive",
        "--connection",
        &reference,
    ]));
    let revision = status["connection"]["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let repaired = success(journey.follow(&[
        "connections",
        "repair",
        "--adapter",
        "drive",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
        "--credential-file",
        journey.client_file.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(repaired["summary"]["state"], "ready");
    assert_eq!(repaired["summary"]["connection"], reference.as_str());
    assert_eq!(journey.provider.acquired(), 2, "consent ran again");

    read_project(journey.invoke(&reference));
    assert!(journey.refreshed_with(2));
    let after = journey.provider.api_authorizations();
    let bearer = after.last().unwrap();
    assert!(
        !before.contains(bearer),
        "the read used an access token issued before the repair"
    );
    journey.assert_no_material_at_rest();
}
