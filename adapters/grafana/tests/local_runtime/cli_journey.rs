//! Production CLI, real private owner/adapter processes and the platform's custody.
//! The Grafana counterpart of the Loki journey: a saved `grafana.service_account`
//! connection answers `datasources.list` through `invoke` against the
//! recorded fixture, and `connections revalidate` repeats the identity probe.
//!
//! One journey body ([`journey`]) runs on each platform's custody binding: on
//! Linux a disposable Secret Service the test starts, on macOS the user's login
//! keychain, the binding the macOS build fixes.
use super::*;
#[cfg(target_os = "linux")]
use connectors_host::local::keyring;
use connectors_host::local::{config::Paths, owner};
#[cfg(target_os = "linux")]
use std::io::Write;
use std::{
    process::{Child as Process, Output, Stdio},
    time::{Duration, Instant},
};

/// The custody the journey's owner stores the saved credential in.
trait JourneyCustody {
    /// The `custody_backend` `setup check` reports for it.
    fn backend(&self) -> &'static str;
    /// The configuration lines that select it.
    fn configuration(&self) -> String;
    /// Restarts custody while no owner runs.
    fn restart(&mut self);
}

#[cfg(target_os = "linux")]
struct OwnedProcess(Process);
#[cfg(target_os = "linux")]
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let result = self.0.wait();
        eprintln!("owned custody process {} exit={result:?}", self.0.id());
    }
}

struct BoundedChild {
    child: Option<Process>,
    stdout: Option<std::thread::JoinHandle<Vec<u8>>>,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
}
impl BoundedChild {
    fn spawn(mut command: Command) -> Self {
        fn reader(
            mut stream: impl std::io::Read + Send + 'static,
        ) -> std::thread::JoinHandle<Vec<u8>> {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(
                    &mut std::io::Read::take(&mut stream, 4 * 1024 * 1024),
                    &mut bytes,
                )
                .unwrap();
                bytes
            })
        }
        let mut child = command.spawn().unwrap();
        let stdout = Some(reader(child.stdout.take().unwrap()));
        let stderr = Some(reader(child.stderr.take().unwrap()));
        Self {
            child: Some(child),
            stdout,
            stderr,
        }
    }
    fn finish(mut self) -> Output {
        let until = Instant::now() + Duration::from_secs(45);
        let status = loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < until,
                "owned CLI process exceeded fixture wait bound"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        eprintln!(
            "owned CLI process {} exit={status}",
            self.child.as_ref().unwrap().id()
        );
        self.child.take();
        let stdout = self.stdout.take().unwrap().join().unwrap();
        let stderr = self.stderr.take().unwrap().join().unwrap();
        assert!(
            stdout.len() < 4 * 1024 * 1024 && stderr.len() < 4 * 1024 * 1024,
            "CLI output bound"
        );
        Output {
            status,
            stdout,
            stderr,
        }
    }
}
impl Drop for BoundedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            eprintln!(
                "owned CLI process {} cleanup exit={:?}",
                child.id(),
                child.wait()
            );
        }
        if let Some(reader) = self.stdout.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr.take() {
            let _ = reader.join();
        }
    }
}
/// A disposable GNOME Secret Service on a private bus: the Linux binding.
#[cfg(target_os = "linux")]
struct Custody {
    daemon: Option<OwnedProcess>,
    _bus: OwnedProcess,
    directory: PathBuf,
    socket: PathBuf,
    epoch: u32,
}
#[cfg(target_os = "linux")]
impl Custody {
    fn new(parent: &std::path::Path) -> Self {
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
            .write_all(b"fictional-cli-fixture-keyring-password")
            .unwrap();
        self.daemon = Some(daemon);
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&self.socket)) {
            assert!(
                Instant::now() < until,
                "qualified private fixture custody unavailable"
            );
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
}
#[cfg(target_os = "linux")]
impl JourneyCustody for Custody {
    fn backend(&self) -> &'static str {
        "secret_service"
    }
    fn configuration(&self) -> String {
        format!(
            "secret_service_socket={}\n",
            serde_json::to_string(self.socket.to_str().unwrap()).unwrap()
        )
    }
    fn restart(&mut self) {
        drop(self.daemon.take());
        self.start();
    }
}

/// The user's login keychain, the macOS binding
/// (`connectors_custody::keychain`, service `connectors`). This journey touches
/// it: the owner writes real items into it. They are named by the purpose, the
/// test store's authority and version ids, so no item of another store is ever
/// selected, and [`LoginKeychain::remove_written`] deletes them through the
/// custody binding's own deletion (also when the test fails, on drop). A locked
/// keychain answers `unavailable` without a prompt (user interaction is
/// disallowed for every custody call), and `setup check` names it before any
/// credential is read.
#[cfg(target_os = "macos")]
struct LoginKeychain {
    /// The CLI's state directory, whose metadata store names the authority.
    state: PathBuf,
    /// When the test started, which bounds the log it prints on a failure.
    started: Instant,
}
#[cfg(target_os = "macos")]
impl LoginKeychain {
    /// The accounts of the `connectors` items in the login keychain that this
    /// test's store wrote.
    fn written(&self) -> Result<Vec<String>, String> {
        use security_framework::{
            item::{ItemClass, ItemSearchOptions, Limit},
            os::macos::keychain::SecKeychain,
        };
        // No state directory or no metadata store (`metadata.sqlite3`, the
        // file `Metadata::authority_at` opens), nothing written: the owner
        // writes custody items only under the store's authority. Any other
        // failure to read the authority fails the cleanup.
        let store = self.state.join("metadata.sqlite3");
        match std::fs::symlink_metadata(&store) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("{}: {error}", store.display())),
            Ok(_) => {}
        }
        let authority = connectors_host::local::metadata::Metadata::authority_at(&self.state)
            .map_err(|e| format!("the store's authority: {e:?}"))?;
        let marker = format!("/{authority}/");
        // As every custody call: user interaction disallowed, so a locked
        // keychain answers an error and never shows an unlock prompt.
        let _quiet = SecKeychain::disable_user_interaction()
            .map_err(|e| format!("disallowing keychain user interaction: {e}"))?;
        // The user's keychain search list, the login keychain among them;
        // attributes only, never an item's data.
        let found = ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(connectors_custody::keychain::SERVICE)
            .load_attributes(true)
            .limit(Limit::All)
            .search();
        let found = match found {
            Ok(found) => found,
            // errSecItemNotFound: no item carries the service.
            Err(error) if error.code() == -25300 => Vec::new(),
            Err(error) => return Err(format!("keychain search: {error}")),
        };
        Ok(found
            .iter()
            .filter_map(|item| item.simplify_dict()?.remove("acct"))
            .filter(|account| account.contains(&marker))
            .collect())
    }

    /// Deletes every item this test's store wrote and answers how many. Every
    /// item is tried; a refusal names how many items it left behind.
    fn remove_written(&self) -> Result<usize, String> {
        let accounts = self.written()?;
        let failures: Vec<String> = accounts
            .iter()
            .filter_map(|account| Self::remove(account).err())
            .collect();
        if failures.is_empty() {
            Ok(accounts.len())
        } else {
            Err(format!(
                "{} of {} login keychain item(s) of this test left behind: {}",
                failures.len(),
                accounts.len(),
                failures.join("; ")
            ))
        }
    }

    /// Deletes one item through the custody binding's own deletion.
    fn remove(account: &str) -> Result<(), String> {
        use connectors_custody::{Backend, Scope, Version, keychain::Keychain};
        let parts: Vec<&str> = account.split('/').collect();
        let [purpose, authority, allocation, id] = parts[..] else {
            return Err(format!("unexpected custody item {account}"));
        };
        let uuid = |text: &str| uuid::Uuid::parse_str(text).map_err(|e| format!("{account}: {e}"));
        let (authority, allocation) = (uuid(authority)?, uuid(allocation)?);
        let scope = match purpose {
            "credential" => Scope::new(authority, allocation),
            "approval-signing" => Scope::approval_signing(authority, allocation),
            other => return Err(format!("unexpected custody purpose {other}")),
        }
        .map_err(|e| format!("{account}: {e}"))?;
        let version = Version::new(scope, uuid(id)?).map_err(|e| format!("{account}: {e}"))?;
        Keychain::open(scope)
            .and_then(|keychain| {
                keychain.delete(version, Instant::now() + Duration::from_secs(10), || Ok(()))
            })
            .map_err(|e| format!("deleting {account}: {e}"))
    }
}
#[cfg(target_os = "macos")]
impl JourneyCustody for LoginKeychain {
    fn backend(&self) -> &'static str {
        "macos_keychain"
    }
    /// Nothing: absent `secret_service_socket` and `[custody]`, the platform
    /// binding the macOS build fixes is the login keychain.
    fn configuration(&self) -> String {
        String::new()
    }
    /// The login keychain is the user's, not a process this test owns, so
    /// there is nothing of custody to restart: the journey restarts the owner
    /// only, which still reads the saved credential back from the keychain.
    fn restart(&mut self) {}
}
#[cfg(target_os = "macos")]
impl LoginKeychain {
    /// On a failure: the unified log's errors of the owner's process (named
    /// `image`, after its launch copy) since this test started. The custody
    /// binding names each failing keychain call there with its `OSStatus`
    /// (`connectors custody: … answered OSStatus …`), which the CLI's closed
    /// refusal does not carry.
    fn print_owner_errors(&self) {
        let minutes = self.started.elapsed().as_secs() / 60 + 2;
        let shown = Command::new("/usr/bin/log")
            .args(["show", "--style", "compact", "--last"])
            .arg(format!("{minutes}m"))
            .arg("--predicate")
            .arg(
                r#"process == "image" AND (eventMessage BEGINSWITH "connectors custody:" OR messageType == error OR messageType == fault)"#,
            )
            .stdin(Stdio::null())
            .output();
        match shown {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout);
                eprintln!(
                    "owner errors in the unified log (log show exit {}):",
                    output.status
                );
                for line in text.lines().take(300) {
                    eprintln!("  {line}");
                }
            }
            Err(error) => eprintln!("log show did not run: {error}"),
        }
    }
}
#[cfg(target_os = "macos")]
impl Drop for LoginKeychain {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.print_owner_errors();
        }
        match self.remove_written() {
            Ok(removed) => eprintln!("removed {removed} login keychain item(s) of this test"),
            Err(error) => eprintln!("login keychain cleanup failed: {error}"),
        }
    }
}

struct Cli {
    binary: PathBuf,
    paths: Paths,
}
impl Cli {
    fn new(root: &std::path::Path) -> Self {
        let binary = std::env::var_os("CONNECTORS_TEST_CLI")
            .expect(
                "built production CLI required: set CONNECTORS_TEST_CLI to target/debug/connectors",
            )
            .into();
        Self {
            binary,
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
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        BoundedChild::spawn(self.command(args)).finish()
    }
    fn status(&self) -> Value {
        success(self.run(&["adapters", "status", "--adapter", "grafana"]))["observation"].clone()
    }
    fn shutdown(&self) {
        self.try_shutdown().unwrap();
    }
    /// Stops a running owner and waits for it to remove its socket.
    fn try_shutdown(&self) -> Result<(), String> {
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            client
                .shutdown(&host)
                .map_err(|e| format!("owner shutdown: {e:?}"))?;
            let until = Instant::now() + Duration::from_secs(5);
            while self.paths.state.join("owner.sock").exists() {
                if Instant::now() >= until {
                    return Err("owner did not finish cleanup".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            eprintln!("owner {host} acknowledged shutdown and removed its socket");
        }
        Ok(())
    }
}
impl Drop for Cli {
    /// A failed shutdown while the test is already failing is reported, not
    /// raised: a second panic would abort the process before the custody
    /// cleanup declared before the CLI (the login keychain's drop) runs.
    fn drop(&mut self) {
        if std::thread::panicking() {
            if let Err(error) = self.try_shutdown() {
                eprintln!("owner shutdown during the failure failed: {error}");
            }
        } else {
            self.shutdown();
        }
    }
}
#[track_caller]
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-grafana-token"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-grafana-token"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

fn configure(cli: &Cli, provider: &Provider, custody: &dyn JourneyCustody) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = provider.selection();
    // JSON string/array literals are valid TOML for these fixture-only ASCII
    // paths. No credential is a configuration input.
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\n{}[adapters.grafana]\ninstance_id={}\nadapter_id='grafana'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.grafana.permissions]\nprofiles=['grafana.service_account']\noperations=['datasources.list']\n[adapters.grafana.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        custody.configuration(),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
    // The configured binding is the expected one and qualified before any
    // credential is read: a locked or absent store is named here.
    let check = success(cli.run(&["setup", "check"]));
    assert_eq!(check["custody_backend"], custody.backend(), "{check}");
    assert_eq!(check["keyring"], "available", "{check}");
    assert!(
        check["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "persistent_custody_qualification" && p["state"] == "ready"),
        "{check}"
    );
}

/// `operations describe`, then `invoke` with the described schema
/// and revision, on the saved connection.
fn invoke_saved(cli: &Cli, connection: &str, operation: &str, input: Value) -> Value {
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "grafana",
        "--operation",
        operation,
    ]));
    let schema = describe["schema"].as_str().unwrap().to_owned();
    let revision = describe["revision"].as_str().unwrap().to_owned();
    let input = input.to_string();
    success(cli.run(&[
        "invoke",
        "grafana/grafana",
        operation,
        "--connection",
        connection,
        "--schema",
        &schema,
        "--revision",
        &revision,
        "--input-json",
        &input,
    ]))["result"]
        .clone()
}

/// Linux only: its custody is a GNOME Secret Service on a private D-Bus, which
/// macOS does not have; the macOS test below runs the same journey on the
/// login keychain.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn grafana_cli_connects_with_a_service_account_token_and_lists_datasources_on_the_saved_connection()
{
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    journey(&provider, &mut custody, &cli);
}

/// The same journey on macOS through the production CLI, with the platform
/// binding the macOS build fixes: the user's login keychain. It touches the
/// login keychain (service `connectors`) and removes every item it wrote at the
/// end. It needs `CONNECTORS_TEST_CLI` (the built `target/debug/connectors`) and
/// fails without it rather than passing without having run.
#[cfg(target_os = "macos")]
#[test]
fn grafana_cli_connects_with_a_service_account_token_and_lists_datasources_with_the_login_keychain()
{
    let provider = Provider::new();
    // Declared before the CLI so that on a failure the owner is shut down
    // (the CLI's drop) before the keychain items are removed (this drop).
    let mut keychain = LoginKeychain {
        state: provider.root.path().join("cli/state"),
        started: Instant::now(),
    };
    let cli = Cli::new(provider.root.path());
    assert_eq!(cli.paths.state, keychain.state);
    journey(&provider, &mut keychain, &cli);
    cli.shutdown();
    let written = keychain.written().unwrap();
    assert!(
        !written.is_empty(),
        "the saved credential was not in the login keychain"
    );
    // The owner created the item from its launch copy, so this test binary is
    // not on its access list, and no product command deletes a connection's
    // credential item yet (`connections revoke` fences it; nothing sweeps the
    // fence). Removal from here is attempted and reported, never asserted: the
    // item belongs to a disposable runner's keychain.
    match keychain.remove_written() {
        Ok(removed) => eprintln!("removed {removed} login keychain item(s) of this test"),
        Err(error) => eprintln!("login keychain cleanup left items behind: {error}"),
    }
}

/// Connect refuses an unaccepted token, connects with the accepted one,
/// lists the data sources on the saved connection, and revalidates it after
/// the owner and custody restart, from the saved credential alone.
fn journey(provider: &Provider, custody: &mut dyn JourneyCustody, cli: &Cli) {
    configure(cli, provider, custody);
    assert_eq!(cli.status()["state"], "owner_unavailable");

    // An unaccepted token saves nothing and performs only the identity probe.
    let wrong = provider.root.path().join("private/wrong.json");
    private(&wrong, &token("fixture-grafana-token-unaccepted").0);
    let refused = refusal(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "grafana",
        "--profile",
        "grafana.service_account",
        "--credential-file",
        wrong.to_str().unwrap(),
    ]));
    assert_eq!(refused["code"], "service_failure", "{refused}");
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(provider.routes(), ["/api/datasources"]);

    // grafana.test: connect proves the token with the datasources probe.
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(TOKEN_ONE).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "grafana",
        "--profile",
        "grafana.service_account",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(provider.routes(), ["/api/datasources"; 2]);

    // grafana.datasource.list
    let listed = invoke_saved(cli, &reference, "datasources.list", json!({}));
    assert_eq!(listed["provenance"]["instance"], INSTANCE);
    assert_eq!(listed["complete"], true);
    let uids: Vec<&str> = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["uid"].as_str().unwrap())
        .collect();
    assert_eq!(
        uids,
        ["P8E80F9AEF21F6940", "PBFA97CFB590B2093", "legacy_browser-1"]
    );
    assert!(!listed.to_string().contains("monitoring.svc"));
    assert_eq!(provider.routes(), ["/api/datasources"; 3]);

    // grafana.test again, after an owner and custody restart (the owner only on macOS,
    // see `LoginKeychain::restart`): revalidation repeats the probe with the saved
    // credential, which is now the only copy.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    let before = provider.count();
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "grafana",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    assert_eq!(provider.count(), before + 1);
    assert_eq!(provider.routes().last().unwrap(), "/api/datasources");
    assert!(
        provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .skip(1)
            .all(|c| c.method == "GET" && c.authorization == format!("Bearer {TOKEN_ONE}"))
    );
}
