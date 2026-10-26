//! story:launch-hands-an-oauth2-access-token through the production CLI and
//! owner against a disposable Secret Service and the fake token server of
//! [`super`]. `connections launch` of an `oauth2_grant` connection hands the
//! pinned consumer the bare current access token on descriptor 3, refreshed
//! first inside the declaration's `refresh_lead_seconds`, and never the stored
//! entry with its refresh token. Each launch is one recorded
//! `connectors.oauth2.AccessTokenIssue` with delivery `consumer_launch`. Any
//! other profile launches its stored document, as before, and records no
//! issue. `launch_consumer_fixture` is the pinned consumer and never runs at
//! top level.
use super::*;
use crate::local::{
    config::{Config, Paths},
    keyring::custody::tests::Fixture,
    metadata::RecordedIssue,
    owner,
};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, fs::File, os::fd::FromRawFd, process::Output};

const ALIAS: &str = "grant";
const PROFILE: &str = "fixture-grant";
const CONSUMER: [&str; 5] = [
    "local::owner::refresh::tests::launch::launch_consumer_fixture",
    "--exact",
    "--ignored",
    "--nocapture",
    "--test-threads=1",
];
/// The consumer's exit status, which the launch's must be.
const CONSUMER_EXIT: i32 = 23;

/// Subprocess helper: the pinned consumer. It reports descriptor 3's bytes,
/// hex-encoded, then exits with [`CONSUMER_EXIT`].
#[test]
#[ignore = "subprocess helper: runs only as the pinned consumer of the OAuth2 launch journeys"]
fn launch_consumer_fixture() {
    use std::io::Write;
    // SAFETY: the launch places the credential at descriptor 3 and this
    // process owns it from exec on.
    let mut credential = unsafe { File::from_raw_fd(3) };
    let mut bytes = Vec::new();
    credential.read_to_end(&mut bytes).unwrap();
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "CONSUMER {}", hex::encode(&bytes)).unwrap();
    stdout.flush().unwrap();
    std::process::exit(CONSUMER_EXIT);
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn files(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            files(&entry.path(), found);
        } else if kind.is_file() {
            found.push(entry.path());
        }
    }
}

/// A configured instance `grant` granting [`PROFILE`], whose adapter never
/// starts, and the consumer `probe` listing it, in a disposable Secret
/// Service. Dropping it stops the owner the CLI started.
struct Launching {
    fixture: Fixture,
    paths: Paths,
    /// The PEM trust root the owner trusts for the token endpoint.
    trust: PathBuf,
    cli: OsString,
}

impl Drop for Launching {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Launching {
    fn new(server_root: &str) -> Self {
        let cli = std::env::var_os("CONNECTORS_TEST_CLI").expect("built production CLI required");
        let fixture = Fixture::new();
        let root = fixture.root.path();
        let paths = Paths::resolve(
            Some(&root.join("config/config.toml")),
            Some(&root.join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        let consumer = std::env::current_exe().unwrap();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let args = CONSUMER.map(q).join(",");
        std::fs::write(
            &paths.config,
            format!(
                "format='connectors-local/3'\nowner_uid={}\nsecret_service_socket={}\n[adapters.{ALIAS}]\ninstance_id='{INSTANCE}'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.{ALIAS}.permissions]\nprofiles=['{PROFILE}']\n[adapters.{ALIAS}.executable]\npath='/not-installed/must-not-start'\nsha256='{}'\nargs=[]\n[consumers.probe.executable]\npath={}\nsha256={}\nargs=[{args}]\n[consumers.probe.permissions]\nconnections=['{ALIAS}']\n",
                crate::local::filesystem::uid(),
                q(fixture.socket.to_str().unwrap()),
                "a".repeat(64),
                q(consumer.to_str().unwrap()),
                q(&digest(&std::fs::read(&consumer).unwrap())),
            ),
        )
        .unwrap();
        let trust = root.join("fixture-root.pem");
        std::fs::write(&trust, server_root).unwrap();
        Self {
            fixture,
            paths,
            trust,
            cli,
        }
    }

    /// Publishes one ready connection whose stored entry is `document`,
    /// valid for `lifetime_ms`, under the authorization server `oauth2`.
    fn publish(
        &self,
        document: &[u8],
        lifetime_ms: u64,
        oauth2: Option<connectors_oauth2::ServerDeclaration>,
    ) -> String {
        let binding = Binding {
            credential_revision: None,
            executable: None,
            instance_id: INSTANCE.into(),
            adapter_id: "fixture-adapter".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.invalid".into(),
            profile: registry::StaticProfile {
                id: PROFILE.into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                declaration: None,
                oauth2,
            },
        };
        let registry = Registry::new(&self.paths.state);
        let acquisition = registry.begin(&binding, now()).unwrap();
        let claim = registry.consume(acquisition, now()).unwrap();
        let at = now();
        let grant = binding.profile.oauth2.is_some();
        let baseline = ValidatedBaseline {
            binding: None,
            identity: ExternalIdentity {
                kind: "fixture-account".into(),
                subject: "one".into(),
            },
            granted_scopes: Some(BTreeSet::from(["read".to_owned()])),
            credential_expires_at_ms: grant.then_some(at + lifetime_ms),
            collected_at_ms: at,
            valid_until_ms: at + lifetime_ms.min(60_000),
        };
        let material = Secret(document.to_vec());
        let prepared = registry
            .prepare(&claim, baseline, material.0.len(), at)
            .unwrap();
        let store = custody::Store::open_at(prepared.version().scope(), Some(&self.fixture.socket))
            .unwrap();
        registry
            .store_and_publish(prepared, &material, &store, now)
            .unwrap()
    }

    /// A published `oauth2_grant` connection whose access token, [`ACCESS`],
    /// expires `expires_in_ms` from now, refreshable with [`REFRESH`] at
    /// `server`.
    fn granted(&self, server: &Server, expires_in_ms: u64) -> String {
        let declaration = serde_json::from_value(json!({
            "authorize_url": "https://localhost/fixture/authorize",
            "token_url": server.url(),
            "device_authorization_url": null,
            "client_id": CLIENT,
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["read"],
            "required_scopes": ["read"],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["authorization_code"],
            "sources": ["https://localhost/fixture-authorization-server"],
            "state_in_exchange": false,
            "scope_in_refresh": false,
        }))
        .unwrap();
        let entry = serde_json::to_vec(&json!({
            "client_id": CLIENT,
            "access_token": ACCESS,
            "refresh_token": REFRESH,
            "expires_at_ms": now() + expires_in_ms,
            "scopes": ["read"],
        }))
        .unwrap();
        self.publish(&entry, expires_in_ms, Some(declaration))
    }

    /// `connections launch` of `connection` to the consumer `probe`.
    fn launch(&self, connection: &str) -> Output {
        std::process::Command::new(&self.cli)
            .env_clear()
            .env("CONNECTORS_TEST_OAUTH_FOLLOW", &self.trust)
            .args(["--output", "json", "--config"])
            .arg(&self.paths.config)
            .arg("--state-dir")
            .arg(&self.paths.state)
            .args(["connections", "launch", "--adapter", ALIAS])
            .args(["--connection", connection, "--consumer", "probe"])
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    }

    /// What the consumer of a launch of `connection` read on descriptor 3.
    /// Neither output stream carries any of `secrets` but through that report.
    fn delivered(&self, connection: &str, secrets: &[&str]) -> Vec<u8> {
        let output = self.launch(connection);
        assert_eq!(output.status.code(), Some(CONSUMER_EXIT), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report = stdout
            .lines()
            // libtest's `test <name> ... ` precedes the report on its line.
            .find_map(|line| line.split_once("CONSUMER ").map(|(_, report)| report))
            .unwrap_or_else(|| panic!("the consumer reported nothing: {output:?}"));
        let delivered = hex::decode(report.trim()).unwrap();
        for secret in secrets {
            assert!(!contains(&output.stderr, secret.as_bytes()), "on stderr");
            assert!(!contains(&output.stdout, secret.as_bytes()), "on stdout");
        }
        delivered
    }

    /// Stops the owner the CLI started, so its records are complete.
    fn stop(&self) {
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            let _ = client.shutdown(&host);
        }
    }

    /// Every recorded `AccessTokenIssue`, read once the owner stopped.
    fn issues(&self) -> Vec<RecordedIssue> {
        self.stop();
        Metadata::recorded_access_token_issues(&self.paths.state).unwrap()
    }

    /// No file under the state directory carries any of `secrets`.
    fn assert_no_material_at_rest(&self, secrets: &[&str]) {
        let mut state = Vec::new();
        files(&self.paths.state, &mut state);
        assert!(!state.is_empty());
        for path in state {
            let bytes = std::fs::read(&path).unwrap();
            for secret in secrets {
                assert!(!contains(&bytes, secret.as_bytes()), "{path:?}");
            }
        }
    }
}

/// Descriptor 3 held exactly `expected`. A failure names lengths and digests,
/// never the bytes.
#[track_caller]
fn assert_delivered(delivered: &[u8], expected: &[u8], what: &str) {
    assert!(
        delivered == expected,
        "descriptor 3 is not {what}: {} bytes (sha256 {}), expected {} bytes (sha256 {})",
        delivered.len(),
        digest(delivered),
        expected.len(),
        digest(expected),
    );
}

/// `issue` is one launch's: delivery `consumer_launch` of `connection`, one
/// IssueAccessToken command and one AccessTokenIssued event.
#[track_caller]
fn assert_launch_issue(issue: &RecordedIssue, connection: &str, refreshed: bool) {
    assert_eq!(issue.fields["delivery"], json!("consumer_launch"));
    assert_eq!(issue.fields["connection_ref"], json!(connection));
    assert_eq!(
        issue
            .fields
            .get("refresh_attempt_id")
            .is_some_and(|attempt| !attempt.is_null()),
        refreshed
    );
    assert_eq!(issue.commands, ["connectors.oauth2.IssueAccessToken"]);
    assert_eq!(issue.events.len(), 1);
    assert_eq!(issue.events[0].0, "connectors.oauth2.AccessTokenIssued");
    assert_eq!(issue.events[0].1["refreshed"], json!(refreshed));
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn launch_oauth2_grant_hands_only_the_access_token() {
    let server = Server::start(Vec::new());
    let launching = Launching::new(&server.root);
    let connection = launching.granted(&server, 3_600_000);
    let delivered = launching.delivered(&connection, &[REFRESH]);
    assert_delivered(&delivered, ACCESS.as_bytes(), "the access token alone");
    assert!(!contains(&delivered, REFRESH.as_bytes()));
    assert!(server.requests().is_empty(), "no refresh was needed");
    launching.assert_no_material_at_rest(&[ACCESS, REFRESH]);
    // Only the owner's launch issues for `consumer_launch`: an issue request
    // naming it is refused and records nothing.
    let claimed = crate::local::access_token::issue(
        owner::Client::connect(&launching.paths, false).unwrap(),
        ALIAS,
        &connection,
        crate::local::access_token::Delivery::ConsumerLaunch,
    );
    assert_eq!(
        claimed.err().map(|error| error.code),
        Some(owner::Code::InvalidInput)
    );
    let issues = launching.issues();
    assert_eq!(issues.len(), 1, "one issue per launch");
    assert_launch_issue(&issues[0], &connection, false);
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn launch_oauth2_grant_refreshes_inside_lead_time() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let launching = Launching::new(&server.root);
    // Valid for 30 s, inside the declared 60 s lead.
    let connection = launching.granted(&server, 30_000);
    let secrets = [REFRESH, ROTATED];
    let first = launching.delivered(&connection, &secrets);
    assert_delivered(
        &first,
        REFRESHED_ACCESS.as_bytes(),
        "the refreshed access token",
    );
    refreshed_request(&server);
    // The refreshed token is current: the next launch sends nothing.
    let second = launching.delivered(&connection, &secrets);
    assert_delivered(
        &second,
        REFRESHED_ACCESS.as_bytes(),
        "the refreshed access token",
    );
    assert_eq!(server.requests().len(), 1, "one refresh request");
    launching.assert_no_material_at_rest(&[ACCESS, REFRESHED_ACCESS, REFRESH, ROTATED]);
    let mut issues = launching.issues();
    assert_eq!(issues.len(), 2, "one issue per launch");
    issues.sort_by_key(|issue| {
        !issue
            .fields
            .get("refresh_attempt_id")
            .is_some_and(|attempt| !attempt.is_null())
    });
    assert_launch_issue(&issues[0], &connection, true);
    assert_launch_issue(&issues[1], &connection, false);
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn launch_static_profile_is_unchanged() {
    let server = Server::start(Vec::new());
    let launching = Launching::new(&server.root);
    let sentinel = format!("fictional-launch-{}", uuid::Uuid::new_v4().simple());
    let document = serde_json::to_vec(&json!({ "password": sentinel })).unwrap();
    let connection = launching.publish(&document, 60_000, None);
    let delivered = launching.delivered(&connection, &[&sentinel]);
    assert_delivered(&delivered, &document, "the stored document, byte for byte");
    assert!(server.requests().is_empty());
    launching.assert_no_material_at_rest(&[&sentinel]);
    assert!(
        launching.issues().is_empty(),
        "a static launch issues nothing"
    );
}

/// Adversary, wave 20261025-oauth2 unit U6, pass 1: cases driven from
/// consumer-launch.md §9 and the story's outcome.
mod adversary {
    use super::*;

    /// The declaration [`Launching::granted`] publishes under: public client,
    /// `refresh_lead_seconds` 60.
    fn declaration(server: &Server) -> connectors_oauth2::ServerDeclaration {
        serde_json::from_value(json!({
            "authorize_url": "https://localhost/fixture/authorize",
            "token_url": server.url(),
            "device_authorization_url": null,
            "client_id": CLIENT,
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["read"],
            "required_scopes": ["read"],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["authorization_code"],
            "sources": ["https://localhost/fixture-authorization-server"],
            "state_in_exchange": false,
            "scope_in_refresh": false,
        }))
        .unwrap()
    }

    /// A published grant whose access token expires `expires_in_ms` from
    /// now and whose evidence lasts `evidence_ms`, the profile's
    /// `evidence_lifetime_ms`: the shape a shipped grant has (catalog evidence
    /// 60 s by default, at most 300 s; access tokens live far longer).
    fn granted_with_evidence(
        launching: &Launching,
        server: &Server,
        expires_in_ms: u64,
        evidence_ms: u64,
    ) -> String {
        let binding = Binding {
            credential_revision: None,
            executable: None,
            instance_id: INSTANCE.into(),
            adapter_id: "fixture-adapter".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.invalid".into(),
            profile: registry::StaticProfile {
                id: PROFILE.into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: evidence_ms,
                declaration: None,
                oauth2: Some(declaration(server)),
            },
        };
        let registry = Registry::new(&launching.paths.state);
        let acquisition = registry.begin(&binding, now()).unwrap();
        let claim = registry.consume(acquisition, now()).unwrap();
        let at = now();
        let entry = serde_json::to_vec(&json!({
            "client_id": CLIENT,
            "access_token": ACCESS,
            "refresh_token": REFRESH,
            "expires_at_ms": at + expires_in_ms,
            "scopes": ["read"],
        }))
        .unwrap();
        let baseline = ValidatedBaseline {
            binding: None,
            identity: ExternalIdentity {
                kind: "fixture-account".into(),
                subject: "one".into(),
            },
            granted_scopes: Some(BTreeSet::from(["read".to_owned()])),
            credential_expires_at_ms: Some(at + expires_in_ms),
            collected_at_ms: at,
            valid_until_ms: at + evidence_ms.min(expires_in_ms),
        };
        let material = Secret(entry);
        let prepared = registry
            .prepare(&claim, baseline, material.0.len(), at)
            .unwrap();
        let store =
            custody::Store::open_at(prepared.version().scope(), Some(&launching.fixture.socket))
                .unwrap();
        registry
            .store_and_publish(prepared, &material, &store, now)
            .unwrap()
    }

    /// The `failure` envelope's `data` a refused launch wrote to stderr.
    fn refusal(output: &Output) -> Value {
        serde_json::from_slice::<Value>(&output.stderr)
            .map(|value| value["error"]["data"].clone())
            .unwrap_or_else(|_| json!({"unparsed_stderr_bytes": output.stderr.len()}))
    }

    fn consumer_ran(output: &Output) -> bool {
        contains(&output.stdout, b"CONSUMER ")
    }

    /// Waits, bounded, until the fake server has read `count` requests.
    fn until_requests(server: &Server, count: usize) {
        let until = Instant::now() + Duration::from_secs(20);
        while server.requests().len() < count {
            assert!(Instant::now() < until, "the server saw no request");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// §9 "Refusals": a refresh the launch needed that answers `invalid_grant`
    /// is `service_failure` with `next_action: repair_connection`, and no
    /// consumer runs; the connection is then Stale and the next launch sends
    /// nothing. `connections token` names `repair_connection` for the same
    /// refusal (`token_failure`); `connections launch` maps the owner's error
    /// through the generic `owner_failure`.
    #[test]
    #[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
    fn adversary_launch_refresh_needing_reauthorization_names_repair_connection() {
        let server = Server::start(vec![error(400, "invalid_grant")]);
        let launching = Launching::new(&server.root);
        let connection = launching.granted(&server, 30_000);
        let first = launching.launch(&connection);
        let second = launching.launch(&connection);
        for output in [&first, &second] {
            assert!(!consumer_ran(output), "a consumer ran: {}", output.status);
            for secret in [ACCESS, REFRESH] {
                assert!(!contains(&output.stderr, secret.as_bytes()));
                assert!(!contains(&output.stdout, secret.as_bytes()));
            }
        }
        assert_eq!(server.requests().len(), 1, "the Stale launch sent nothing");
        assert!(launching.issues().is_empty(), "a refusal records nothing");
        let (first, second) = (refusal(&first), refusal(&second));
        assert_eq!(first["code"], "service_failure", "{first}");
        assert_eq!(second["next_action"], "repair_connection", "{second}");
        assert_eq!(
            first["next_action"], "repair_connection",
            "consumer-launch.md §9: a refresh that needs reauthorization names repair_connection: {first}"
        );
    }

    /// §4.4's refusal table, which §9 points at: a token endpoint `429` is
    /// `unavailable` with service code `rate_limited` and its `Retry-After`
    /// in whole seconds; no consumer runs and nothing is recorded.
    #[test]
    #[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
    fn adversary_launch_rate_limited_refresh_answers_retry_after() {
        let server = Server::start(vec![Reply {
            status: 429,
            body: json!({"error": "rate_limited"}),
            retry_after: Some("7"),
        }]);
        let launching = Launching::new(&server.root);
        let connection = launching.granted(&server, 30_000);
        let output = launching.launch(&connection);
        assert!(!consumer_ran(&output), "a consumer ran: {}", output.status);
        let data = refusal(&output);
        assert_eq!(data["code"], "unavailable", "{data}");
        assert_eq!(data["service_code"], "rate_limited", "{data}");
        assert_eq!(data["retry_after_seconds"], 7, "{data}");
        assert!(launching.issues().is_empty(), "a refusal records nothing");
    }

    /// Two launches of one grant inside the lead, the second arriving while
    /// the first one's exchange is in flight: one refresh request, both
    /// consumers get the refreshed token, two `consumer_launch` issues.
    #[test]
    #[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
    fn adversary_two_concurrent_launches_exchange_once() {
        let server = Server::delayed(
            vec![token(Some(ROTATED), Some("read"))],
            Duration::from_secs(3),
        );
        let launching = Launching::new(&server.root);
        let connection = launching.granted(&server, 30_000);
        let secrets = [REFRESH, ROTATED];
        let (first, second) = std::thread::scope(|scope| {
            let first = scope.spawn(|| launching.delivered(&connection, &secrets));
            until_requests(&server, 1);
            let second = scope.spawn(|| launching.delivered(&connection, &secrets));
            (first.join().unwrap(), second.join().unwrap())
        });
        assert_delivered(&first, REFRESHED_ACCESS.as_bytes(), "the refreshed token");
        assert_delivered(&second, REFRESHED_ACCESS.as_bytes(), "the refreshed token");
        assert_eq!(server.requests().len(), 1, "one exchange for two launches");
        let issues = launching.issues();
        assert_eq!(issues.len(), 2, "one issue per launch");
        for issue in &issues {
            assert_eq!(issue.fields["delivery"], json!("consumer_launch"));
        }
    }

    /// §9 "Issue in place of the read" then the re-read: a configuration that
    /// changes while the launch's refresh is in flight is
    /// `lifecycle_conflict`, no consumer runs, and the issue already recorded
    /// stays recorded (D3). Catches a launch that drops the re-read after the
    /// issue; nothing else in the suite does.
    #[test]
    #[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
    fn adversary_configuration_changed_during_the_issue_is_lifecycle_conflict() {
        let server = Server::delayed(
            vec![token(Some(ROTATED), Some("read"))],
            Duration::from_secs(3),
        );
        let launching = Launching::new(&server.root);
        let connection = launching.granted(&server, 30_000);
        let output = std::thread::scope(|scope| {
            let launched = scope.spawn(|| launching.launch(&connection));
            until_requests(&server, 1);
            let text = std::fs::read_to_string(&launching.paths.config).unwrap();
            let changed = text.replacen(
                "[consumers.probe.executable]",
                "[consumers.probe]\npass_env=['ADVERSARY_']\n[consumers.probe.executable]",
                1,
            );
            assert_ne!(text, changed);
            let staged = launching.paths.config.with_extension("adversary");
            std::fs::write(&staged, changed).unwrap();
            // The configuration stays owner-only, as the original is.
            let permissions = std::fs::metadata(&launching.paths.config)
                .unwrap()
                .permissions();
            std::fs::set_permissions(&staged, permissions).unwrap();
            std::fs::rename(&staged, &launching.paths.config).unwrap();
            launched.join().unwrap()
        });
        assert!(!consumer_ran(&output), "a consumer ran: {}", output.status);
        let data = refusal(&output);
        assert_eq!(data["code"], "lifecycle_conflict", "{data}");
        let issues = launching.issues();
        assert_eq!(issues.len(), 1, "the recorded issue stays recorded");
        assert_launch_issue(&issues[0], &connection, true);
    }

    /// The story's outcome: the consumer gets a currently valid access token,
    /// refreshed first when it expires within the lead. A shipped grant's
    /// evidence (at most 300 s, catalog default 60 s) lapses long before its
    /// access token enters the lead, so the launch is refused at readiness
    /// while `connections token` on the same connection refreshes and issues.
    #[test]
    #[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
    fn adversary_launch_inside_the_lead_after_evidence_lapsed_refreshes() {
        let server = Server::start(vec![
            token(Some(ROTATED), Some("read")),
            token(Some(ROTATED), Some("read")),
        ]);
        let launching = Launching::new(&server.root);
        let connection = granted_with_evidence(&launching, &server, 30_000, 1_000);
        std::thread::sleep(Duration::from_millis(1_500));
        let launched = launching.launch(&connection);
        let token = std::process::Command::new(&launching.cli)
            .env_clear()
            .env("CONNECTORS_TEST_OAUTH_FOLLOW", &launching.trust)
            .args(["--output", "json", "--config"])
            .arg(&launching.paths.config)
            .arg("--state-dir")
            .arg(&launching.paths.state)
            .args(["connections", "token", "--adapter", ALIAS])
            .args(["--connection", &connection])
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            token.status.code(),
            Some(0),
            "connections token refreshes the same state: {}",
            refusal(&token)
        );
        assert_eq!(
            launched.status.code(),
            Some(CONSUMER_EXIT),
            "the launch was refused where connections token refreshed: {}",
            refusal(&launched)
        );
    }
}
