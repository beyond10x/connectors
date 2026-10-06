//! Production CLI, real private owner/adapter processes, disposable custody and
//! a real PostgreSQL server. This is the only place a genuine database is
//! required; every other SQL test runs against a loopback wire fixture.
//!
//! The custody harness below is the one the Kubernetes journey proved; only the
//! provider and the journey differ.
use super::*;
use connectors_host::local::{config::Paths, keyring, owner};
use serde_json::Value;
use std::{
    io::Write,
    process::{Child as Process, Output, Stdio},
    time::Instant,
};

struct OwnedProcess(Process);
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        if let Ok(status) = self.0.wait() {
            eprintln!("fixture process pid={} exit={status}", self.0.id());
        }
    }
}
struct Custody {
    daemon: Option<OwnedProcess>,
    _bus: OwnedProcess,
    directory: PathBuf,
    socket: PathBuf,
    epoch: u32,
}
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
    fn restart(&mut self) {
        drop(self.daemon.take());
        self.start();
    }
}
struct Cli {
    binary: PathBuf,
    paths: Paths,
}
impl Cli {
    fn new(root: &std::path::Path) -> Self {
        let binary = std::env::var_os("CONNECTORS_TEST_CLI")
            .expect("built production CLI required")
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
        let output = self.command(args).output().unwrap();
        eprintln!(
            "CLI action={:?} exit={}",
            &args[..args.len().min(2)],
            output.status
        );
        output
    }
    fn shutdown(&self) {
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            client.shutdown(&host).unwrap();
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("sandbox-reader-pw"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("sandbox-reader-pw"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

/// The sandbox is opt-in: `CONNECTORS_PG_SANDBOX=host:port` names a running
/// PostgreSQL with the role and table the journey expects. Explicitly selected
/// cases require this prerequisite rather than passing against nothing.
fn sandbox() -> Option<(String, u16)> {
    let value = std::env::var("CONNECTORS_PG_SANDBOX").ok()?;
    let (host, port) = value.rsplit_once(':')?;
    Some((host.to_owned(), port.parse().ok()?))
}

fn configure(
    cli: &Cli,
    root: &std::path::Path,
    custody: &Custody,
    host: &str,
    port: u16,
) -> PathBuf {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let native = root.join("private/postgres.json");
    private(
        &native,
        &serde_json::to_vec(&json!({
            "format":"connectors-sql-local/1","instance":"pg-sandbox",
            "host":host,"port":port,"database":"incidents","user":"reader",
            // Loopback only. The sandbox server runs without TLS; this is the
            // one thing the journey below does not establish.
            "allow_plaintext":true,"ca_file":null
        }))
        .unwrap(),
    );
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-sql"))
        .canonicalize()
        .unwrap();
    let output = Command::new(&binary)
        .arg("--local-config")
        .arg(&native)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.warehouse]\ninstance_id='pg-sandbox'\nadapter_id='sql'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.warehouse.permissions]\nprofiles=['postgres.password']\noperations=['schema.list','query.read']\n[adapters.warehouse.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&bootstrap.configuration_revision),
        q(binary.to_str().unwrap()),
        q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),
        serde_json::to_string(&["--local-config", native.to_str().unwrap()]).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
    native
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, a built production CLI and qualified disposable Secret Service"]
fn a_real_postgres_session_persists_across_cli_and_owner_restart() {
    let (host, port) = required_sandbox();
    let admin = PgAdmin::new();
    let root = tempfile::tempdir().unwrap();
    filesystem::directory(&root.path().join("private"), true, true).unwrap();
    let mut custody = Custody::new(root.path());
    let cli = Cli::new(root.path());
    configure(&cli, root.path(), &custody, &host, port);

    let credential = root.path().join("private/credential.json");
    private(&credential, br#"{"password":"sandbox-reader-pw"}"#);
    // A wrong password is the server's own refusal, not a local guess.
    let wrong = root.path().join("private/wrong.json");
    private(&wrong, br#"{"password":"not-the-password"}"#);
    // `owner::Code` carries no invalid_credential, so a rejected credential is
    // service_failure with the provider's own code preserved beside it. That is
    // the same envelope GitLab and Kubernetes produce, and 28P01 is the
    // server's answer rather than anything this binding decided.
    let refused = refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "warehouse",
            "--profile",
            "postgres.password",
            "--credential-file",
            wrong.to_str().unwrap(),
        ]),
        "service_failure",
    );
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");
    assert_eq!(refused["stage"], "dispatch");

    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "warehouse",
        "--profile",
        "postgres.password",
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

    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "warehouse",
        "--operation",
        "query.read",
    ]));
    let schema = describe["schema"].as_str().unwrap().to_owned();
    let descriptor = describe["revision"].as_str().unwrap().to_owned();
    let read = |input: &str| {
        [
            "operations",
            "invoke",
            "--adapter",
            "warehouse",
            "--connection",
            &reference,
            "--operation",
            "query.read",
            "--schema",
            &schema,
            "--revision",
            &descriptor,
            "--input-json",
            input,
        ]
        .map(String::from)
    };
    let critical = read(
        r#"{"query":"SELECT id, severity FROM incidents WHERE severity = $1 ORDER BY id","parameters":["critical"],"limit":10}"#,
    );
    let args: Vec<&str> = critical.iter().map(String::as_str).collect();
    let value = success(cli.run(&args));
    let rows: Value = value["result"].clone();
    // Two critical rows exist and the minor one must not appear: the parameter
    // reached bind rather than being interpolated into the text.
    assert_eq!(rows["rows"].as_array().map(|r| r.len()), Some(2), "{rows}");

    // This columnless write is unsupported before execution. Arbitrary transport
    // failure is not write protection evidence; independently retain table state.
    let before = admin.query("SELECT json_agg(i ORDER BY id) FROM incidents i;");
    let write = read(
        r#"{"query":"INSERT INTO incidents VALUES (99,'minor',now(),NULL)","parameters":[],"limit":1}"#,
    );
    let write: Vec<&str> = write.iter().map(String::as_str).collect();
    provider_refusal(cli.run(&write), "unsupported");
    assert_eq!(
        admin.query("SELECT json_agg(i ORDER BY id) FROM incidents i;"),
        before
    );

    // Restart both the CLI process and the owner, and drop the only other copy
    // of the password: the saved credential version must carry the next read.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    // Refresh provider evidence after the restart. This uses the saved custody
    // version: the input credential file is gone, and the original bounded
    // evidence may have expired during the earlier real server checks.
    let refreshed = success(cli.run(&[
        "connections",
        "revalidate",
        "--adapter",
        "warehouse",
        "--connection",
        &reference,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(refreshed["connection"]["summary"]["state"], "ready");
    assert_eq!(refreshed["connection"]["summary"]["revision"], revision);
    let value = success(cli.run(&args));
    let rows: Value = value["result"].clone();
    assert_eq!(rows["rows"].as_array().map(|r| r.len()), Some(2));
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "warehouse",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    // schema.list reaches the same server through the same saved credential.
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "warehouse",
        "--operation",
        "schema.list",
    ]));
    let listed = success(cli.run(&[
        "operations",
        "invoke",
        "--adapter",
        "warehouse",
        "--connection",
        &reference,
        "--operation",
        "schema.list",
        "--schema",
        describe["schema"].as_str().unwrap(),
        "--revision",
        describe["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"schema":"public","limit":100}"#,
    ]));
    assert!(listed["result"].to_string().contains("incidents"));
}

// Real-provider acceptance is deliberately opt-in and fails closed when an
// explicitly selected ignored case lacks its disposable fixture prerequisites.
struct PgAdmin {
    container: String,
}
impl PgAdmin {
    fn new() -> Self {
        let container = std::env::var("CONNECTORS_PG_CONTAINER")
            .expect("CONNECTORS_PG_CONTAINER must name an owned disposable PostgreSQL container");
        assert!(
            !container.is_empty()
                && container
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        );
        let (host, port) = required_sandbox();
        let endpoint = Command::new("docker")
            .args(["port", &container, "5432/tcp"])
            .output()
            .expect("owned fixture requires docker");
        assert!(
            endpoint.status.success(),
            "could not verify owned container port"
        );
        assert_eq!(
            String::from_utf8(endpoint.stdout).unwrap().trim(),
            format!("{host}:{port}"),
            "admin container and SUT endpoint differ"
        );
        let admin = Self { container };
        assert!(
            admin.query("SHOW server_version;").starts_with("17.6"),
            "fixture identity must be PostgreSQL 17.6"
        );
        admin
    }
    fn query(&self, sql: &str) -> String {
        let mut child = Command::new("docker")
            .args([
                "exec",
                "-i",
                &self.container,
                "psql",
                "-X",
                "-qAt",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                "incidents",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("owned fixture admin requires docker/psql");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "fixture admin failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
fn marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "sql_acceptance_{}_{}_{}",
        std::process::id(),
        connectors_sdk::now_ms(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    )
}
fn required_sandbox() -> (String, u16) {
    let (host, port) = sandbox()
        .expect("CONNECTORS_PG_SANDBOX=127.0.0.1:PORT is required; case unexecuted without it");
    assert_eq!(host, "127.0.0.1", "acceptance authorizes loopback only");
    (host, port)
}
struct LivePg {
    // Drop the CLI/owner before custody, and schema before its TempDir.
    cli: Cli,
    _custody: Custody,
    root: tempfile::TempDir,
    admin: PgAdmin,
    native: PathBuf,
    credential: PathBuf,
    reference: String,
    revision: String,
    descriptor: Value,
    schema: String,
}
impl LivePg {
    fn new() -> Self {
        let (host, port) = required_sandbox();
        let admin = PgAdmin::new();
        let root = tempfile::tempdir().unwrap();
        filesystem::directory(&root.path().join("private"), true, true).unwrap();
        let custody = Custody::new(root.path());
        let cli = Cli::new(root.path());
        let native = configure(&cli, root.path(), &custody, &host, port);
        let credential = root.path().join("private/credential.json");
        private(&credential, br#"{"password":"sandbox-reader-pw"}"#);
        let connected = success(cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "warehouse",
            "--profile",
            "postgres.password",
            "--credential-file",
            credential.to_str().unwrap(),
        ]))["connection"]
            .clone();
        let descriptor = success(cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "warehouse",
            "--operation",
            "query.read",
        ]));
        let schema = marker();
        admin.query(&format!("CREATE SCHEMA {schema}; CREATE TABLE {schema}.teams (id integer PRIMARY KEY, name text); CREATE TABLE {schema}.events (id integer PRIMARY KEY, team integer REFERENCES {schema}.teams, severity text, opened_at timestamptz); INSERT INTO {schema}.teams VALUES (1,'O''Reilly'),(2,'Other'); INSERT INTO {schema}.events VALUES (1,1,'critical','2026-10-01 23:59:59+00'),(2,1,'critical','2026-10-02 00:00:00+00'),(3,1,'minor','2026-10-02 00:59:59+00'),(4,2,'critical','2026-10-02 01:00:00+00'); GRANT USAGE ON SCHEMA {schema} TO reader; GRANT SELECT ON ALL TABLES IN SCHEMA {schema} TO reader;"));
        Self {
            cli,
            _custody: custody,
            root,
            admin,
            native,
            credential,
            reference: connected["summary"]["connection"].as_str().unwrap().into(),
            revision: connected["summary"]["revision"].as_str().unwrap().into(),
            descriptor,
            schema,
        }
    }
    fn args(&self, query: &str, parameters: Value, limit: u32) -> Vec<String> {
        let input = json!({"query":query,"parameters":parameters,"limit":limit}).to_string();
        [
            "operations",
            "invoke",
            "--adapter",
            "warehouse",
            "--connection",
            &self.reference,
            "--operation",
            "query.read",
            "--schema",
            self.descriptor["schema"].as_str().unwrap(),
            "--revision",
            self.descriptor["revision"].as_str().unwrap(),
            "--input-json",
            &input,
        ]
        .map(String::from)
        .to_vec()
    }
    fn read(&self, query: &str, parameters: Value, limit: u32) -> Output {
        let args = self.args(query, parameters, limit);
        self.cli
            .run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    }
    fn rows(&self, query: &str, parameters: Value, limit: u32) -> Value {
        let result = success(self.read(query, parameters, limit));
        result["result"].clone()
    }
    fn snapshot(&self) -> String {
        self.admin.query(&format!(
            "SELECT json_agg(e ORDER BY id) FROM {}.events e;",
            self.schema
        ))
    }
    fn repair(&self, credential: &std::path::Path) -> Output {
        self.cli.run(&[
            "connections",
            "repair",
            "--adapter",
            "warehouse",
            "--connection",
            &self.reference,
            "--expected-revision",
            &self.revision,
            "--credential-file",
            credential.to_str().unwrap(),
        ])
    }
    fn unchanged_authority(&self) {
        let status = success(self.cli.run(&[
            "connections",
            "status",
            "--adapter",
            "warehouse",
            "--connection",
            &self.reference,
        ]));
        assert_eq!(status["connection"]["summary"]["revision"], self.revision);
        assert_eq!(status["connection"]["summary"]["state"], "ready");
        assert_eq!(
            self.rows("SELECT current_user, current_database()", json!([]), 1)["rows"],
            json!([["reader", "incidents"]])
        );
    }
}
impl Drop for LivePg {
    fn drop(&mut self) {
        self.cli.shutdown();
        self.admin
            .query(&format!("DROP SCHEMA {} CASCADE;", self.schema));
    }
}
#[track_caller]
fn provider_refusal(output: Output, code: &str) {
    // Only unauthorized lacks a matching owner code. Native bounds and
    // unsupported/invalid statements retain the corresponding owner code.
    if code == "unauthorized" {
        let value = refusal(output, "service_failure");
        assert_eq!(value["service_code"], code, "{value}");
        assert_eq!(value["stage"], "dispatch");
    } else {
        refusal(output, code);
    }
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service"]
fn postgres_cli_preserves_join_group_utc_and_quoted_parameters() {
    let pg = LivePg::new();
    let query = format!(
        "SELECT t.name, count(*) AS total FROM {}.events e JOIN {}.teams t ON e.team=t.id WHERE e.opened_at >= $1::timestamptz AND e.opened_at < $2::timestamptz AND t.name = $3 GROUP BY t.name ORDER BY t.name",
        pg.schema, pg.schema
    );
    let value = pg.rows(
        &query,
        json!(["2026-10-02T00:00:00Z", "2026-10-02T01:00:00Z", "O'Reilly"]),
        10,
    );
    assert_eq!(value["rows"], json!([["O'Reilly", "2"]]));
    assert_eq!(
        value["columns"],
        json!([{"name":"name","native_type":"text"},{"name":"total","native_type":"int8"}])
    );
    assert_eq!(value["truncated"], false);
    let boundaries = pg.rows(&format!("SELECT id, to_char(opened_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS utc FROM {}.events WHERE opened_at >= $1::timestamptz AND opened_at < $2::timestamptz ORDER BY id", pg.schema), json!(["2026-10-02T00:00:00Z", "2026-10-02T01:00:00Z"]), 10);
    assert_eq!(
        boundaries["rows"],
        json!([["2", "2026-10-02 00:00:00"], ["3", "2026-10-02 00:59:59"]])
    );
    let quoted = "O'Reilly'; DELETE FROM events; --";
    assert_eq!(
        pg.rows("SELECT $1::text AS bound", json!([quoted]), 1)["rows"],
        json!([[quoted]])
    );
    assert_eq!(
        pg.rows(
            &format!("SELECT count(*) FROM {}.events", pg.schema),
            json!([]),
            1
        )["rows"],
        json!([["4"]])
    );
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service"]
fn postgres_cli_distinguishes_empty_truncated_capacity_and_timeout() {
    let pg = LivePg::new();
    let empty = pg.rows(
        &format!("SELECT id, severity FROM {}.events WHERE false", pg.schema),
        json!([]),
        10,
    );
    assert_eq!(empty["rows"], json!([]));
    assert_eq!(
        empty["columns"],
        json!([{"name":"id","native_type":"int4"},{"name":"severity","native_type":"text"}])
    );
    assert_eq!(empty["truncated"], false);
    let truncated = pg.rows(
        &format!("SELECT id FROM {}.events ORDER BY id", pg.schema),
        json!([]),
        2,
    );
    assert_eq!(truncated["rows"], json!([["1"], ["2"]]));
    assert_eq!(truncated["truncated"], true);
    provider_refusal(
        pg.read(
            &format!(
                "SELECT repeat('x', {}) AS oversized",
                connectors_core::RESPONSE_LIMIT + 1
            ),
            json!([]),
            1,
        ),
        "capacity",
    );
    let started = Instant::now();
    provider_refusal(pg.read("SELECT pg_sleep(60)", json!([]), 1), "timeout");
    eprintln!(
        "postgres timeout elapsed_ms={}",
        started.elapsed().as_millis()
    );
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "provider/CLI unchanged deadline exceeded"
    );
    assert_eq!(pg.rows("SELECT current_setting('statement_timeout'), current_setting('lock_timeout'), current_setting('search_path'), current_setting('transaction_read_only')", json!([]), 1)["rows"], json!([["10s","2s","public, pg_catalog","on"]]));
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service"]
fn postgres_cli_cannot_escape_read_only_transaction() {
    let pg = LivePg::new();
    let before = pg.snapshot();
    assert_eq!(pg.admin.query("SELECT rolsuper::text || ':' || rolcreaterole::text || ':' || rolcreatedb::text FROM pg_roles WHERE rolname='reader';"), "false:false:false");
    assert_eq!(
        pg.rows(
            "SELECT current_user, session_user, current_setting('transaction_read_only')",
            json!([]),
            1
        )["rows"],
        json!([["reader", "reader", "on"]])
    );
    for (query, code) in [
        (
            format!(
                "INSERT INTO {}.events VALUES (99,1,'minor',now())",
                pg.schema
            ),
            "unsupported",
        ),
        (
            format!(
                "INSERT INTO {}.events VALUES (99,1,'minor',now()) RETURNING id",
                pg.schema
            ),
            "invalid_input",
        ),
        (
            format!(
                "WITH changed AS (DELETE FROM {}.events RETURNING id) SELECT * FROM changed",
                pg.schema
            ),
            "unsupported",
        ),
        (
            format!("SELECT 1; DELETE FROM {}.events", pg.schema),
            "invalid_input",
        ),
        ("COMMIT".into(), "unsupported"),
        ("SET TRANSACTION READ WRITE".into(), "unsupported"),
    ] {
        provider_refusal(pg.read(&query, json!([]), 1), code);
        assert_eq!(pg.snapshot(), before, "fixture changed after {query}");
    }
    assert_eq!(
        pg.rows(
            "SELECT current_user, current_setting('transaction_read_only')",
            json!([]),
            1
        )["rows"],
        json!([["reader", "on"]])
    );
}

// The observer never dispatches business SQL. Correlation requires both the
// exact backend PID and the unique literal query marker under the reader role.
struct Backend<'a> {
    admin: &'a PgAdmin,
    marker: String,
    pid: Option<i32>,
}
impl<'a> Backend<'a> {
    fn new(admin: &'a PgAdmin) -> Self {
        Self {
            admin,
            marker: marker(),
            pid: None,
        }
    }
    fn observe(&mut self) -> bool {
        let rows = self.admin.query(&format!("SELECT json_build_object('pid',pid,'query',query,'backend_start',backend_start) FROM pg_stat_activity WHERE datname='incidents' AND usename='reader' AND state='active' AND position('{}' in query)>0;", self.marker));
        if rows.is_empty() {
            return false;
        }
        let observations: Vec<Value> = rows
            .lines()
            .map(|v| serde_json::from_str(v).unwrap())
            .collect();
        assert_eq!(
            observations.len(),
            1,
            "marker must select exactly one owned backend"
        );
        let pid = i32::try_from(observations[0]["pid"].as_i64().unwrap()).unwrap();
        if let Some(previous) = self.pid {
            assert_eq!(previous, pid, "backend identity changed");
        } else {
            eprintln!("postgres backend observation={}", observations[0]);
        }
        self.pid = Some(pid);
        true
    }
    fn terminate(&self) {
        if let Some(pid) = self.pid {
            self.admin.query(&format!("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid={pid} AND datname='incidents' AND usename='reader' AND position('{}' in query)>0;", self.marker));
        }
    }
}
impl Drop for Backend<'_> {
    fn drop(&mut self) {
        self.terminate();
    }
}
struct NativePassword;
#[async_trait::async_trait]
impl connectors_sdk::Credential for NativePassword {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(b"sandbox-reader-pw".to_vec()))
    }
}
struct NativeCall(tokio::task::JoinHandle<connectors_core::Result<Value>>);
impl Drop for NativeCall {
    fn drop(&mut self) {
        // Retain cancellation ownership even when an observation assertion fails.
        self.0.abort();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires CONNECTORS_PG_SANDBOX and CONNECTORS_PG_CONTAINER for owned PostgreSQL observation"]
async fn postgres_dropped_invocation_cancels_its_backend() {
    use connectors_sdk::Adapter as _;
    let (host, port) = required_sandbox();
    let admin = PgAdmin::new();
    for drop_invocation in [false, true] {
        let config = connectors_sql::Config {
            host: host.clone(),
            port,
            database: "incidents".into(),
            user: "reader".into(),
            allow_plaintext: true,
            ca_file: None,
        };
        let effective = json!({"service":{"instance":"acceptance","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
        let sql =
            connectors_sql::Sql::new("acceptance", config, effective, Arc::new(NativePassword))
                .unwrap();
        let mut backend = Backend::new(&admin);
        let query = format!("SELECT pg_sleep(60) /* {} */", backend.marker);
        let start = Instant::now();
        let mut call = NativeCall(tokio::spawn(async move {
            sql.invoke("query.read", json!({"query":query,"limit":1}))
                .await
        }));
        while !backend.observe() {
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "marked query not observed within two seconds"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "late observation cannot satisfy acceptance"
        );
        let observed_ms = start.elapsed().as_millis();
        let drop_at = Instant::now();
        if drop_invocation {
            call.0.abort();
            assert!((&mut call.0).await.unwrap_err().is_cancelled());
            while backend.observe() {
                assert!(
                    drop_at.elapsed() < Duration::from_secs(5),
                    "marked backend did not stop within five seconds of invocation drop"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert!(drop_at.elapsed() < Duration::from_secs(5));
        } else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            assert!(
                !call.0.is_finished(),
                "no-drop control completed prematurely"
            );
            assert!(
                backend.observe(),
                "no-drop control did not remain active for five seconds"
            );
            eprintln!(
                "postgres no-drop control active_after_ms={}",
                drop_at.elapsed().as_millis()
            );
            backend.terminate();
            let result = tokio::time::timeout(Duration::from_secs(5), &mut call.0)
                .await
                .unwrap()
                .unwrap();
            assert!(
                result.is_err(),
                "terminated control cannot return successful query data"
            );
        }
        eprintln!(
            "postgres cancellation drop={drop_invocation} pid={} marker={} observed_ms={observed_ms} elapsed_after_observation_ms={}",
            backend.pid.unwrap(),
            backend.marker,
            drop_at.elapsed().as_millis()
        );
    }
}

#[test]
#[ignore = "requires CONNECTORS_PG_SANDBOX, CONNECTORS_PG_CONTAINER, built CONNECTORS_TEST_CLI and qualified disposable Secret Service"]
fn postgres_repair_revoke_and_busy_stop_preserve_authority() {
    let pg = LivePg::new();
    // SQL role identity is configuration-owned, never accepted from a password
    // document. A changed role therefore refuses bootstrap binding first.
    let native = fs::read(&pg.native).unwrap();
    pg.cli.shutdown();
    let mut changed: Value = serde_json::from_slice(&native).unwrap();
    changed["user"] = json!("different_disposable_role");
    private(&pg.native, &serde_json::to_vec(&changed).unwrap());
    refusal(pg.repair(&pg.credential), "readiness_mismatch");
    pg.cli.shutdown();
    private(&pg.native, &native);
    pg.unchanged_authority();
    let wrong = pg.root.path().join("private/wrong-repair.json");
    private(&wrong, br#"{"password":"wrong-fixture-password"}"#);
    provider_refusal(pg.repair(&wrong), "unauthorized");
    pg.unchanged_authority();

    let mut backend = Backend::new(&pg.admin);
    let query = format!("SELECT pg_sleep(60) /* {} */", backend.marker);
    let args = pg.args(&query, json!([]), 1);
    let mut call = OwnedProcess(
        pg.cli
            .command(&args.iter().map(String::as_str).collect::<Vec<_>>())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    while !backend.observe() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "busy query was not dispatched"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let status = success(
        pg.cli
            .run(&["adapters", "status", "--adapter", "warehouse"]),
    );
    let observation = &status["observation"];
    let stopped_at = Instant::now();
    let stopped = success(pg.cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "warehouse",
        "--expected-revision",
        observation["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        observation["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        observation["child_incarnation"].as_str().unwrap(),
    ]));
    assert!(
        matches!(
            stopped["observation"]["state"].as_str(),
            Some("suppressed" | "stopping")
        ),
        "{stopped}"
    );
    assert!(
        stopped_at.elapsed() < Duration::from_secs(10),
        "busy stop did not honor bounded lifecycle"
    );
    let status = call.0.wait().unwrap();
    use std::io::Read;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    call.0
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    call.0
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    refusal(
        Output {
            status,
            stdout,
            stderr,
        },
        "unavailable",
    );
    eprintln!(
        "postgres busy stop pid={} elapsed_ms={} invocation_exit={status}",
        backend.pid.unwrap(),
        stopped_at.elapsed().as_millis()
    );
    // Killing the adapter is not the native cancellation case. Explicitly clean
    // only this exact observed backend before checking a newly admitted read.
    backend.terminate();
    pg.unchanged_authority();
    success(pg.cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "warehouse",
        "--connection",
        &pg.reference,
        "--expected-revision",
        &pg.revision,
    ]));
    let revoked_marker = marker();
    refusal(
        pg.read(
            &format!("SELECT pg_sleep(60) /* {revoked_marker} */"),
            json!([]),
            1,
        ),
        "revoked",
    );
    assert_eq!(pg.admin.query(&format!("SELECT count(*) FROM pg_stat_activity WHERE datname='incidents' AND usename='reader' AND position('{revoked_marker}' in query)>0;")), "0");
    let status = success(pg.cli.run(&[
        "connections",
        "status",
        "--adapter",
        "warehouse",
        "--connection",
        &pg.reference,
    ]));
    assert_eq!(status["connection"]["summary"]["state"], "revoked");
}
