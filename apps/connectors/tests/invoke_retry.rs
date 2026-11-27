//! story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice.
//! `connectors invoke --retry-not-attempted <n>` tries a keyed write again
//! only while the provider answered its preflight with its own timeout or
//! capacity answer and nothing was sent; an applied, refused or unknown write
//! is never sent twice (`contracts/cli/v1alpha1/semantics.md`, "Invoking an
//! operation").
//!
//! The harness is `support/mounted.rs`'s jail: the production CLI and owner
//! with the read-only `mounted_secret` binding, so a write reaches its commit
//! with a credential no keyring holds. This test binary is also the adapter
//! child, filtered to [`invoke_retry_adapter_fixture`]: a stand-in that serves
//! `connectors-private/2` writes. Its preflights answer as `/work/script`
//! says for the k-th preflight, each after the script's delay, and its commit
//! as the script says; it writes one line per preflight and per commit to
//! `/work/log`, carrying the input's `value`, when the request arrives and
//! before any delay. Two local synthetic clock sources answer the approval
//! clock with the system time. The approval key is installed with its public
//! half only (`approval_keys::Store::install_for_tests`): the test signs the
//! standing grant and the approval proof with the seed itself, as `approvals
//! grant-standing` and `approvals issue` do under a writable binding. Linux
//! only: the binding is.
#![cfg(target_os = "linux")]
#[path = "../../../crates/connectors-host/tests/fixtures/clock/server.rs"]
mod clock_fixture;
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::{approval_keys, approvals, clock, registry, runtime, standing};
use connectors_sdk::{Secret, WriteOutcome};
use mounted::{CONFIG, Container, PROFILE, SECRET, STATE, digest, success};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Output, Stdio},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "invoke_retry_adapter_fixture";
const INSTANCE: &str = "forge-local";
/// The implementation id the legacy entry names, and so the connector its
/// alias `forge` is an instance of.
const ADAPTER: &str = "fixture";
const TARGET: &str = "fixture/forge";
const WRITE: &str = "write";
const READ: &str = "read";
const DESCRIPTION: &str = "desc-1";
/// Inside the jail: the stand-in's script and log, and the grant file.
const SCRIPT: &str = "/work/script";
const LOG: &str = "/work/log";
const GRANTS: &str = "/srv/connectors/grants.json";
/// The approval key's seed: public test material, never a production key.
const SEED: [u8; 32] = [42; 32];

// ---------------------------------------------------------------------------
// The stand-in adapter.

fn profile() -> runtime::Profile {
    runtime::Profile {
        id: PROFILE.into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: 60_000,
        fields: vec![runtime::EntryField {
            name: "token".into(),
            label: "Token".into(),
            max_bytes: 1024,
        }],
        acquisition: None,
        token_recipient: None,
    }
}

/// The description: a read, and on `connectors-private/2` the write
/// `{"value": boolean}` that answers its input.
fn bootstrap(write: bool) -> runtime::Bootstrap {
    let value = json!({"type":"object","required":["value"],
        "properties":{"value":{"type":"boolean"}},"additionalProperties":false});
    let mut operations = vec![(READ, "resource", runtime::Effect::Read)];
    if write {
        operations.push((WRITE, "mutation", runtime::Effect::Write));
    }
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: DESCRIPTION.into(),
        operations: operations
            .iter()
            .map(|(id, kind, _)| connectors_core::Operation {
                id: (*id).into(),
                description: format!("stand-in {id}"),
                contract: "operations/v1alpha1".into(),
                profile: (*kind).into(),
                input_schema: value.clone(),
                output_schema: value.clone(),
            })
            .collect(),
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://standin.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![profile()],
        requirements: operations
            .iter()
            .map(|(id, _, effect)| runtime::Requirement {
                operation: (*id).into(),
                custody: false,
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: *effect,
            })
            .collect(),
    }
}

/// What a test asks of the stand-in: the answer of each preflight by its
/// number from 1 (`rest` after the list), how long a preflight that does not
/// pass waits before it answers, and the commit's answer.
#[derive(Clone)]
struct Script {
    preflights: Vec<&'static str>,
    rest: &'static str,
    delay_ms: u64,
    commit: &'static str,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            preflights: Vec::new(),
            rest: "ok",
            delay_ms: 0,
            commit: "applied",
        }
    }
}

impl Script {
    fn json(&self) -> Value {
        json!({"preflights":self.preflights,"rest":self.rest,"delay_ms":self.delay_ms,
            "commit":self.commit})
    }
}

/// One line of the stand-in's log, written and synced at once.
fn log(line: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG)
        .unwrap();
    file.write_all(format!("{line}\n").as_bytes()).unwrap();
    file.sync_all().unwrap();
}

fn logged(kind: &str) -> usize {
    fs::read_to_string(LOG)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with(kind))
        .count()
}

fn script() -> Value {
    serde_json::from_slice(&fs::read(SCRIPT).unwrap()).unwrap()
}

/// The adapter child: it validates the mounted credential and serves writes
/// as `/work/script` says.
struct Standin;

#[async_trait::async_trait]
impl runtime::Adapter for Standin {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap(false)
    }
    fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap(true))
    }
    async fn validate(&self, _: &str, document: Secret) -> runtime::Result<runtime::Baseline> {
        if document.0 != SECRET {
            return Err(runtime::Failure::InvalidCredential);
        }
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "standin.user".into(),
                subject: "standin-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(&self, _: &str, _: &str, _: Secret, input: Value) -> runtime::Result<Value> {
        Ok(input)
    }
    async fn prepare_write(
        &self,
        operation: &str,
        _: &str,
        document: Secret,
        input: Value,
    ) -> runtime::Result<Box<dyn runtime::PreparedWrite>> {
        if operation != WRITE || document.0 != SECRET {
            return Err(runtime::Failure::Forbidden);
        }
        // Logged as the request arrives, before any delay.
        log(&format!("preflight {}", input["value"]));
        let script = script();
        let k = logged("preflight");
        let answer = script["preflights"]
            .get(k - 1)
            .unwrap_or(&script["rest"])
            .as_str()
            .unwrap()
            .to_owned();
        if answer == "ok" {
            return Ok(Box::new(Prepared { input }));
        }
        let delay = script["delay_ms"].as_u64().unwrap();
        std::thread::sleep(Duration::from_millis(delay));
        use connectors_core::{Error, ErrorCode};
        Err(match answer.as_str() {
            // The provider's own answers to the preflight's request.
            "provider_timeout" => runtime::Failure::from_provider(
                Error::new(ErrorCode::Timeout, "provider request timed out").answered(),
            ),
            "provider_capacity" => runtime::Failure::from_provider(
                Error::new(ErrorCode::Capacity, "provider at capacity").answered(),
            ),
            "provider_forbidden" => runtime::Failure::from_provider(
                Error::new(ErrorCode::Forbidden, "provider refused").answered(),
            ),
            // The adapter's own timeout, which no provider answered.
            "adapter_timeout" => {
                runtime::Failure::from_provider(Error::new(ErrorCode::Timeout, "adapter timed out"))
            }
            other => panic!("unknown preflight answer {other}"),
        })
    }
}

struct Prepared {
    input: Value,
}

#[async_trait::async_trait]
impl runtime::PreparedWrite for Prepared {
    async fn execute(self: Box<Self>) -> WriteOutcome<Value> {
        log(&format!("commit {}", self.input["value"]));
        use connectors_core::{Error, ErrorCode};
        match script()["commit"].as_str().unwrap() {
            "applied" => WriteOutcome::Applied(Ok(self.input)),
            // A refusal after the send: a declared conflict status.
            "refused" => {
                WriteOutcome::Refused(Error::new(ErrorCode::Conflict, "target changed").answered())
            }
            // An unknown effect: the request was sent and no answer comes.
            "unknown" => std::process::exit(0),
            other => panic!("unknown commit answer {other}"),
        }
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn invoke_retry_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Standin);
    }
}

// ---------------------------------------------------------------------------
// The site: one jail with a connection, a policy, a key and a grant.

/// A synthetic clock source answering with the system time, signed by the
/// fixture key of `seed`.
fn clock_source(seed: u8) -> clock_fixture::Server {
    clock_fixture::Server::spawn(Box::new(move |_, request| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Some(
            clock_fixture::Fixture {
                seed,
                midpoint: now,
                radius: 1,
                mint: now - 86_400,
                maxt: now + 86_400,
                ..Default::default()
            }
            .reply(request),
        )
    }))
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The standing grant of a site: the values of `/value` it admits and its
/// hourly cap.
struct Grant {
    values: Vec<bool>,
    max_per_hour: u32,
}

impl Default for Grant {
    fn default() -> Self {
        Self {
            values: vec![true, false],
            max_per_hour: 100,
        }
    }
}

struct Site {
    container: Container,
    clocks: [clock_fixture::Server; 2],
    key: approvals::ConfiguredApprovalKey,
    connection: String,
}

impl Site {
    fn new(grant: Grant) -> Self {
        let clocks = [clock_source(7), clock_source(8)];
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        let adapter = container.path("/bin-test/adapter");
        let args = ["--exact", ADAPTER_FIXTURE, "--"];
        let text = format!(
            "format='connectors-local/4'\nowner_uid={uid}\n{custody}{clock}[adapters.forge]\ninstance_id='{INSTANCE}'\nadapter_id='{ADAPTER}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\nstanding_grants='{GRANTS}'\n[adapters.forge.permissions]\nprofiles=['{PROFILE}']\noperations=['{READ}','{WRITE}']\n[adapters.forge.executable]\npath='/bin-test/adapter'\nsha256='{sha256}'\nargs={args}\n",
            uid = connectors_host::local::filesystem::uid(),
            custody = Container::mounted(),
            clock = clock_fixture::quorum_toml(
                2,
                10_000,
                &[(&clocks[0].address, 7), (&clocks[1].address, 8)]
            ),
            sha256 = digest(&fs::read(adapter).unwrap()),
            args = serde_json::to_string(&args).unwrap(),
        );
        private(&container.path(CONFIG), text.as_bytes());
        let site = Self {
            container,
            clocks,
            key: approvals::ConfiguredApprovalKey {
                issuer: String::new(),
                audience: String::new(),
                kid: String::new(),
                public_key: String::new(),
                not_before_unix_ms: 0,
                not_after_unix_ms: 9_007_199_254_740_991,
                revoked: false,
            },
            connection: String::new(),
        };
        site.script(&Script::default());
        site.setup(grant)
    }

    fn setup(mut self, grant: Grant) -> Self {
        let init = success(&self.container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        let connected = success(&self.container.connect(PROFILE));
        self.connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap_or_else(|| panic!("{connected}"))
            .to_owned();
        // The key, public half only: the seed stays here.
        let state = self.container.path(STATE);
        let view = approval_keys::Store::new(&state, None, INSTANCE, ADAPTER, "cfg-1")
            .unwrap()
            .install_for_tests(&Secret(SEED.to_vec()))
            .unwrap();
        let active = view
            .keys
            .iter()
            .find(|key| key.state == approval_keys::State::Active)
            .unwrap();
        self.key.issuer = view.issuer.clone();
        self.key.audience = view.audience.clone();
        self.key.kid = active.key_id.to_string();
        self.key.public_key = active.public_key.clone();
        // The write is the policy's one operation.
        private(
            &self.container.path("/work/policy.json"),
            br#"{"operations":["write"]}"#,
        );
        success(&self.container.run(&[
            "approvals",
            "policy-set",
            "--adapter",
            "forge",
            "--input-file",
            "/work/policy.json",
        ]));
        // The grant, signed as `approvals grant-standing` signs it.
        let reading = standing::reading(&self.clock()).unwrap();
        let target = if grant.values.len() == 1 {
            json!({"pointer":"/value","value":grant.values[0]})
        } else {
            json!({"pointer":"/value","one_of":grant.values})
        };
        let request = json!({"operation":WRITE,"target":[target],
            "not_after_unix_ms":reading.upper_unix_ms + 3_600_000,
            "max_per_hour":grant.max_per_hour});
        standing::grant(
            &standing::lock(&state).unwrap(),
            &self.container.path(GRANTS),
            &self.signer(),
            &self.key,
            INSTANCE,
            reading,
            &serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        self
    }

    fn signer(&self) -> approvals::Signer {
        approvals::Signer::from_seed(Secret(SEED.to_vec()), self.key.kid.clone()).unwrap()
    }

    /// One reading of the site's two clock sources.
    fn clock(&self) -> clock::BoundedClock {
        use base64::Engine;
        clock::ApprovalClock::Quorum(clock::Configuration {
            format: "roughtime-quorum/1".into(),
            quorum: 2,
            max_rate_error_ppm: 10_000,
            sources: self
                .clocks
                .iter()
                .zip([7, 8])
                .map(|(server, seed)| clock::Source {
                    address: server.address.clone(),
                    public_key: base64::engine::general_purpose::STANDARD
                        .encode(clock_fixture::key(seed)),
                    version: clock::RoughtimeVersion::Draft0x8000000c,
                })
                .collect(),
        })
        .acquire()
        .unwrap()
    }

    /// The stand-in's script from now on, and an empty log.
    fn script(&self, script: &Script) {
        fs::write(
            self.container.path(SCRIPT),
            serde_json::to_vec(&script.json()).unwrap(),
        )
        .unwrap();
        let _ = fs::remove_file(self.container.path(LOG));
    }

    /// The stand-in's log, one entry per line.
    fn log(&self) -> Vec<String> {
        fs::read_to_string(self.container.path(LOG))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// `invoke fixture/forge write --input-json {"value":<value>}` with
    /// `more`.
    fn args(value: bool, more: &[&str]) -> Vec<String> {
        let input = json!({"value": value}).to_string();
        ["invoke", TARGET, WRITE, "--input-json", &input]
            .into_iter()
            .chain(more.iter().copied())
            .map(str::to_owned)
            .collect()
    }

    fn invoke(&self, value: bool, more: &[&str]) -> Output {
        let args = Self::args(value, more);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.container.run(&args)
    }

    /// The same invocation, started and not awaited.
    fn spawn(&self, value: bool, more: &[&str]) -> std::process::Child {
        let mut command = self
            .container
            .jail
            .command("/bin-test/connectors", Map::Own);
        command
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(Self::args(value, more))
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            match command.spawn() {
                Ok(child) => return child,
                Err(error)
                    if error.raw_os_error() == Some(libc::ETXTBSY) && Instant::now() < until =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        }
    }

    /// A proof of `{"value": <value>}`, as `approvals issue` writes one, at
    /// `path` inside the jail.
    fn proof(&self, value: bool, path: &str) {
        let schema = connectors_host::local::owner::schema(&bootstrap(true), WRITE).unwrap();
        let input = json!({"value": value}).to_string();
        let prepared = success(&self.container.run(&[
            "approvals",
            "prepare",
            "--adapter",
            "forge",
            "--connection",
            &self.connection,
            "--operation",
            WRITE,
            "--schema",
            &schema,
            "--revision",
            DESCRIPTION,
            "--input-json",
            &input,
        ]));
        let subject: approvals::Subject =
            serde_json::from_value(prepared["preparation"]["subject"].clone())
                .unwrap_or_else(|_| panic!("{prepared}"));
        let evidence = self
            .signer()
            .issue(&subject, &Issuing(&self.key), &self.clock())
            .unwrap();
        let document = json!({"reference": evidence.reference(),
            "evidence": std::str::from_utf8(evidence.bytes()).unwrap()});
        private(
            &self.container.path(path),
            &serde_json::to_vec(&document).unwrap(),
        );
    }
}

/// The issuer's own admission of a subject: the key the site installed.
struct Issuing<'a>(&'a approvals::ConfiguredApprovalKey);

impl approvals::CurrentAdmission for Issuing<'_> {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        self.0
    }
}

impl approvals::IssuancePolicy for Issuing<'_> {
    type Guard<'a>
        = Issuing<'a>
    where
        Self: 'a;
    fn authorize<'a>(&'a self, _: &approvals::Subject, _: &str) -> approvals::Result<Issuing<'a>> {
        Ok(Issuing(self.0))
    }
}

/// The success envelope on standard output, its exit status 0.
#[track_caller]
fn answered(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let lines: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "{output:?}");
    let envelope: Value = serde_json::from_slice(lines[0]).unwrap();
    assert_eq!(envelope["ok"], true, "{envelope}");
    envelope["result"].clone()
}

/// The exit status and the failure envelope's `error.data` on standard error.
#[track_caller]
fn refused(output: &Output) -> (Option<i32>, Value) {
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: Value =
        serde_json::from_slice(&output.stderr).unwrap_or_else(|_| panic!("{output:?}"));
    assert_eq!(envelope["ok"], false, "{envelope}");
    (output.status.code(), envelope["error"]["data"].clone())
}

const PROVIDER_TIMEOUT: &str = "provider_timeout";

// ---------------------------------------------------------------------------
// The acceptance statements.

/// Statement 1, under a standing grant of `max_per_hour` 1 whose preflights 1
/// and 2 answer the provider's timeout: with `--retry-not-attempted 3` the
/// write is applied on its third try.
#[test]
fn a_write_whose_first_two_preflights_time_out_is_applied_on_its_third_try() {
    let site = Site::new(Grant {
        values: vec![true],
        max_per_hour: 1,
    });
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, PROVIDER_TIMEOUT],
        ..Default::default()
    });
    let result = answered(&site.invoke(
        true,
        &["--idempotency-key", "retry-1", "--retry-not-attempted", "3"],
    ));
    assert_eq!(result["mutation"]["classification"], "applied", "{result}");
    assert_eq!(result["mutation"]["tries"], 3, "{result}");
    assert_eq!(result["result"], json!({"value": true}), "{result}");
    assert_eq!(
        site.log(),
        [
            "preflight true",
            "preflight true",
            "preflight true",
            "commit true"
        ]
    );
}

/// Statement 1, the same site and script without the option: today's
/// answer, `not_attempted` after one preflight, with no `tries`. A test of
/// its own, so each stays under statement 15's 5 s.
#[test]
fn the_same_write_without_the_option_is_not_attempted() {
    let site = Site::new(Grant {
        values: vec![true],
        max_per_hour: 1,
    });
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, PROVIDER_TIMEOUT],
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(true, &["--idempotency-key", "plain-1"]));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert!(data["mutation"].get("tries").is_none(), "{data}");
    assert_eq!(site.log(), ["preflight true"]);
}

/// Statement 2: an unknown effect is never sent again: exit 3 with the
/// option, `tries` 1 and one commit; exit 1 and no `tries` without it.
#[test]
fn an_unknown_write_is_sent_once_and_exits_3_with_the_option() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        commit: "unknown",
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &[
            "--idempotency-key",
            "unknown-1",
            "--retry-not-attempted",
            "3",
        ],
    ));
    assert_eq!(code, Some(3), "{data}");
    assert_eq!(data["mutation"]["classification"], "unknown", "{data}");
    assert_eq!(data["mutation"]["tries"], 1, "{data}");
    assert_eq!(site.log(), ["preflight true", "commit true"]);
    site.script(&Script {
        commit: "unknown",
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(false, &["--idempotency-key", "unknown-2"]));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(data["mutation"]["classification"], "unknown", "{data}");
    assert!(data["mutation"].get("tries").is_none(), "{data}");
    assert_eq!(site.log(), ["preflight false", "commit false"]);
}

/// Statement 3: a write the provider refused after the send is not sent
/// again: exit 1, `refused`, `tries` 1, one commit.
#[test]
fn a_refused_write_is_sent_once() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        commit: "refused",
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &[
            "--idempotency-key",
            "refused-1",
            "--retry-not-attempted",
            "3",
        ],
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(data["mutation"]["classification"], "refused", "{data}");
    assert_eq!(data["mutation"]["tries"], 1, "{data}");
    assert_eq!(site.log(), ["preflight true", "commit true"]);
}

/// Statement 4: every preflight times out at the provider: two tries, then
/// the last `not_attempted`, nothing committed.
#[test]
fn the_tries_end_at_their_number_with_the_last_answer() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        rest: PROVIDER_TIMEOUT,
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &["--idempotency-key", "spent-1", "--retry-not-attempted", "2"],
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(data["mutation"]["tries"], 2, "{data}");
    assert_eq!(data["code"], "timeout", "{data}");
    assert_eq!(data["stage"], "dispatch", "{data}");
    assert_eq!(data["next_action"], "retry_explicitly", "{data}");
    assert_eq!(site.log(), ["preflight true", "preflight true"]);
}

/// Statement 5: preflights that time out after 6 s: a third try would not
/// leave its 6 s and the 3 s reserve before the 20 s write deadline.
#[test]
fn no_try_starts_without_its_duration_and_the_reserve_before_the_deadline() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        rest: PROVIDER_TIMEOUT,
        delay_ms: 6_000,
        ..Default::default()
    });
    let started = Instant::now();
    let output = site.invoke(
        true,
        &["--idempotency-key", "late-1", "--retry-not-attempted", "10"],
    );
    let took = started.elapsed();
    let (code, data) = refused(&output);
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(data["mutation"]["tries"], 2, "{data}");
    assert!(took < Duration::from_secs(20), "{took:?}");
    assert_eq!(site.log(), ["preflight true", "preflight true"]);
}

/// Statement 6: the option without a key, and a number outside 1 to 10, are
/// refused at `arguments` before anything is sent.
#[test]
fn the_option_is_refused_without_a_key_or_outside_its_bounds() {
    let site = Site::new(Grant::default());
    for more in [
        &["--retry-not-attempted", "2"][..],
        &[
            "--idempotency-key",
            "bounds-1",
            "--retry-not-attempted",
            "0",
        ],
        &[
            "--idempotency-key",
            "bounds-1",
            "--retry-not-attempted",
            "11",
        ],
    ] {
        let (code, data) = refused(&site.invoke(true, more));
        assert_eq!(code, Some(2), "{more:?}: {data}");
        assert_eq!(data["code"], "invalid_input", "{more:?}: {data}");
        assert_eq!(data["field"], "retry_not_attempted", "{more:?}: {data}");
        assert_eq!(data["stage"], "arguments", "{more:?}: {data}");
    }
    assert_eq!(site.log(), Vec::<String>::new());
}

/// Statement 7: two processes with two keys: each write's tries run on the
/// instance's one worker before the other write's first.
#[test]
fn two_writes_take_their_tries_one_after_the_other() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec![
            PROVIDER_TIMEOUT,
            PROVIDER_TIMEOUT,
            "ok",
            PROVIDER_TIMEOUT,
            PROVIDER_TIMEOUT,
        ],
        ..Default::default()
    });
    let first = site.spawn(
        true,
        &["--idempotency-key", "pair-1", "--retry-not-attempted", "3"],
    );
    let second = site.spawn(
        false,
        &["--idempotency-key", "pair-2", "--retry-not-attempted", "3"],
    );
    for child in [first, second] {
        let result = answered(&child.wait_with_output().unwrap());
        assert_eq!(result["mutation"]["classification"], "applied", "{result}");
        assert_eq!(result["mutation"]["tries"], 3, "{result}");
    }
    let log = site.log();
    let one = &log[0]["preflight ".len()..];
    let other = if one == "true" { "false" } else { "true" };
    let expected: Vec<String> = [one, other]
        .iter()
        .flat_map(|value| {
            [
                format!("preflight {value}"),
                format!("preflight {value}"),
                format!("preflight {value}"),
                format!("commit {value}"),
            ]
        })
        .collect();
    assert_eq!(log, expected);
}

/// Statement 8: a preflight the provider refused with 403 is not tried again.
#[test]
fn a_forbidden_preflight_is_not_tried_again() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec!["provider_forbidden"],
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &[
            "--idempotency-key",
            "forbidden-1",
            "--retry-not-attempted",
            "3",
        ],
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(data["code"], "forbidden", "{data}");
    assert_eq!(data["mutation"]["tries"], 1, "{data}");
    assert_eq!(site.log(), ["preflight true"]);
}

/// Statement 9: a same-key replay of statement 1's call answers its applied
/// write with `tries` 0 and sends nothing.
#[test]
fn a_same_key_replay_names_no_try() {
    let site = Site::new(Grant {
        values: vec![true],
        max_per_hour: 1,
    });
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, PROVIDER_TIMEOUT],
        ..Default::default()
    });
    let call = [
        "--idempotency-key",
        "replay-1",
        "--retry-not-attempted",
        "3",
    ];
    let result = answered(&site.invoke(true, &call));
    assert_eq!(result["mutation"]["tries"], 3, "{result}");
    let before = site.log();
    let replay = answered(&site.invoke(true, &call));
    assert_eq!(replay["mutation"]["classification"], "applied", "{replay}");
    assert_eq!(replay["mutation"]["replayed"], true, "{replay}");
    assert_eq!(replay["mutation"]["tries"], 0, "{replay}");
    assert_eq!(site.log(), before);
}

/// The lines of `ess/domains/cli.yaml` from the declaration of `name` to the
/// next declaration.
fn declared(name: &str) -> Vec<&'static str> {
    const SPEC: &str = include_str!("../../../ess/domains/cli.yaml");
    let start = format!("  - name: {name}");
    SPEC.lines()
        .skip_while(|line| *line != start)
        .skip(1)
        .take_while(|line| !line.starts_with("  - name:"))
        .collect()
}

/// Statement 10: the two members are declared `Optional<Integer>`.
#[test]
fn the_model_declares_retry_not_attempted_and_tries() {
    for (structure, member) in [
        ("connectors.cli.InvokeInput", "retry_not_attempted"),
        ("connectors.cli.MutationObservation", "tries"),
    ] {
        let line = format!("      - {{name: {member}, type: \"Optional<Integer>\"}}");
        assert!(
            declared(structure).contains(&line.as_str()),
            "{structure} lacks {line}"
        );
    }
}

/// Statement 11: the exit table names exit 3 for an unknown write under the
/// option.
#[test]
fn the_exit_table_names_exit_3_for_an_unknown_write_under_the_option() {
    const SEMANTICS: &str = include_str!("../../../contracts/cli/v1alpha1/semantics.md");
    let rows: Vec<&str> = SEMANTICS
        .lines()
        .skip_while(|line| *line != "| Condition | `Failure.code` | Exit |")
        .take_while(|line| line.starts_with('|'))
        .collect();
    assert!(
        rows.iter()
            .any(|row| row.contains("`--retry-not-attempted`")
                && row.contains("`unknown`")
                && row.ends_with("| 3 |")),
        "{rows:#?}"
    );
}

/// Statement 12: a provider capacity answer to the first preflight is tried
/// again under a per-input proof, which the retried try did not spend.
#[test]
fn a_capacity_answer_is_tried_again_under_an_approval_file() {
    let site = Site::new(Grant {
        values: vec![false],
        max_per_hour: 1,
    });
    site.proof(true, "/work/proof.json");
    site.script(&Script {
        preflights: vec!["provider_capacity"],
        ..Default::default()
    });
    let result = answered(&site.invoke(
        true,
        &[
            "--approval-file",
            "/work/proof.json",
            "--idempotency-key",
            "proof-1",
            "--retry-not-attempted",
            "2",
        ],
    ));
    assert_eq!(result["mutation"]["classification"], "applied", "{result}");
    assert_eq!(result["mutation"]["tries"], 2, "{result}");
    assert_eq!(
        site.log(),
        ["preflight true", "preflight true", "commit true"]
    );
}

/// Statement 13: the adapter's own timeout is the host's, at `admission`:
/// not one of the causes a try is made again for.
#[test]
fn the_adapters_own_timeout_is_not_tried_again() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec!["adapter_timeout"],
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &["--idempotency-key", "own-1", "--retry-not-attempted", "3"],
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(data["mutation"]["tries"], 1, "{data}");
    assert_eq!(data["code"], "timeout", "{data}");
    assert_eq!(data["stage"], "admission", "{data}");
    assert_eq!(site.log(), ["preflight true"]);
}

/// Statement 14: once the caller is interrupted no further try starts.
#[test]
fn no_try_starts_after_the_caller_is_interrupted() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        rest: PROVIDER_TIMEOUT,
        delay_ms: 2_000,
        ..Default::default()
    });
    let child = site.spawn(
        true,
        &["--idempotency-key", "gone-1", "--retry-not-attempted", "10"],
    );
    let until = Instant::now() + Duration::from_secs(5);
    while site.log().is_empty() {
        assert!(Instant::now() < until, "no preflight arrived");
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: signals the child this test started and still holds.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let signalled = Instant::now();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{output:?}");
    std::thread::sleep(Duration::from_secs(6).saturating_sub(signalled.elapsed()));
    assert_eq!(site.log(), ["preflight true"]);
}

#[path = "invoke_retry/adversary.rs"]
mod adversary;
