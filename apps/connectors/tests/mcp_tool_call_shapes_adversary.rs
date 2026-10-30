//! Adversary cases for story:mcp-tool-call-shapes-its-result, driven from
//! `adapters/mcp/contracts/server/v1alpha1/projection.md`, "Shaping a tool
//! result", against the production `connectors server --transport stdio`.
//!
//! The harness is `mcp_tool_call_shapes.rs`'s: the server, its owner and an
//! adapter child run against the read-only `mounted_secret` binding in a jail
//! (`support/mounted.rs`). The adapter child is this test binary serving one
//! read, `ticket.pages`, whose input schema declares a walk that never ends by
//! itself: every page answers one record and the next cursor. The input's
//! `delay_ms` is how long the adapter takes over each page. Every read appends
//! its operation to `/work/invocations` and its input to `/work/inputs`.
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors::walk::{self, Page, Walk};
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::{config, registry, runtime};
use mounted::{ALIAS, CONFIG, Container, PROFILE, SECRET, STATE, digest, success};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "mcp_tool_call_shapes_adversary_adapter_fixture";
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "forge-local";
const ADAPTER: &str = "catalog";
const DESCRIPTION: &str = "desc-1";
const REVISION_META: &str = "io.beyond10x.connectors/revision";
const PAGES: &str = "ticket.pages";
const OPERATIONS: [&str; 1] = [PAGES];
/// The request budget the companion configures: the least the projection
/// admits for an `implemented` read (`execution_ms` 20,000).
const REQUEST_MS: u64 = 20_000;

fn walk_declaration() -> Value {
    json!({"parameter": "page[after]", "next": "/meta/after_cursor", "records": "/items",
           "continues": {"pointer": "/meta/has_more", "equals": true},
           "ends": [{"name": "no_cursor", "absent": "/meta/after_cursor"}]})
}

fn input_schema() -> Value {
    json!({"type":"object","additionalProperties":false,
        "properties":{"ticket_id":{"type":"integer"},"delay_ms":{"type":"integer"},
            "page[after]":{"type":"string"}},
        "required":["ticket_id"],
        "x-connectors-walk": walk_declaration()})
}

fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: DESCRIPTION.into(),
        operations: OPERATIONS
            .iter()
            .map(|id| connectors_core::Operation {
                id: (*id).into(),
                description: format!("{id} of one ticket"),
                contract: "operations/v1alpha1".into(),
                profile: "resource".into(),
                input_schema: input_schema(),
                output_schema: json!({"type":"object","required":["status","body","provenance"]}),
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
        provider_authority: "https://fixture.zendesk.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
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
        }],
        requirements: OPERATIONS
            .iter()
            .map(|operation| runtime::Requirement {
                operation: (*operation).into(),
                custody: false,
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

/// A provider whose walk never ends by itself: page `n` answers record `n`
/// and the cursor of page `n + 1`, after `delay_ms`.
struct Endless;

#[async_trait::async_trait]
impl runtime::Adapter for Endless {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap())
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "zendesk.user".into(),
                subject: "200".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        _: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        Err(runtime::Failure::Unsupported)
    }
    async fn invoke_bounded(
        &self,
        operation: &str,
        _: &str,
        secret: connectors_sdk::Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        if secret.0 != SECRET || operation != PAGES {
            return Err(runtime::Failure::Forbidden);
        }
        for (log, line) in [
            ("/work/invocations", operation.to_owned()),
            ("/work/inputs", input.to_string()),
        ] {
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(Path::new(log))
                .unwrap();
            writeln!(log, "{line}").unwrap();
        }
        std::thread::sleep(Duration::from_millis(
            input["delay_ms"].as_u64().unwrap_or(0),
        ));
        let page = input
            .get("page[after]")
            .and_then(Value::as_str)
            .and_then(|cursor| cursor.strip_prefix('c'))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(1);
        Ok(json!({"status": 200,
            "body": {"items": [{"id": page}],
                     "meta": {"has_more": true, "after_cursor": format!("c{}", page + 1)}},
            "provenance": {"instance": INSTANCE, "resource": "/api/v2/tickets/{ticket_id}/pages",
                           "observed_at_unix_ms": 1_758_000_000_000_u64,
                           "source_revision": "fixture-source-1"}}))
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn mcp_tool_call_shapes_adversary_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Endless);
    }
}

fn private(path: &Path, text: String) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn clock_key(seed: u8) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode([seed; 32])
}

fn entry(container: &Container) -> config::Adapter {
    config::Adapter {
        instance_id: INSTANCE.into(),
        adapter_id: ADAPTER.into(),
        configuration_revision: "cfg-1".into(),
        credential_revision: None,
        protocol: "v1alpha1".into(),
        private_protocol: Some(runtime::PrivateProtocol::V3),
        startup: Default::default(),
        restart: Default::default(),
        executable: config::Executable {
            path: "/bin-test/adapter".into(),
            sha256: digest(&fs::read(container.path("/bin-test/adapter")).unwrap()),
            args: vec!["--exact".into(), ADAPTER_FIXTURE.into(), "--".into()],
        },
        permissions: config::Permissions {
            profiles: BTreeSet::from([PROFILE.to_owned()]),
            operations: OPERATIONS.iter().map(|op| (*op).to_owned()).collect(),
            custody: Default::default(),
        },
    }
}

fn configure(container: &Container, adapter: &config::Adapter) {
    let uid = connectors_host::local::filesystem::uid();
    let text = format!(
        "format='connectors-local/4'\nowner_uid={uid}\n{custody}[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{key_a}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{key_b}'\nversion='roughtime-0x8000000c'\n[adapters.{ALIAS}]\ninstance_id='{INSTANCE}'\nadapter_id='{ADAPTER}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/3'\n[adapters.{ALIAS}.permissions]\nprofiles={profiles}\noperations={operations}\n[adapters.{ALIAS}.executable]\npath='/bin-test/adapter'\nsha256='{sha256}'\nargs={args}\n",
        custody = Container::mounted(),
        key_a = clock_key(7),
        key_b = clock_key(8),
        profiles = serde_json::to_string(&adapter.permissions.profiles).unwrap(),
        operations = serde_json::to_string(&adapter.permissions.operations).unwrap(),
        sha256 = adapter.executable.sha256,
        args = serde_json::to_string(&adapter.executable.args).unwrap(),
    );
    private(&container.path(CONFIG), text);
}

/// One jail with a connection, the curation pinned to the adapter and the
/// read exposed as a tool, under a `REQUEST_MS` request budget.
fn served() -> Container {
    bootstrap()
        .validate_for(runtime::PrivateProtocol::V3)
        .expect("the fixture's description passes the child's own readiness check");
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    let adapter = entry(&container);
    configure(&container, &adapter);
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap_or_else(|| panic!("{connected}"))
        .to_owned();
    let operations: Vec<Value> = OPERATIONS
        .iter()
        .map(|operation| {
            json!({"operation":operation,"metadata":{"effects":["network"],"semantic_effects":[],"risk":"low",
                "idempotency":{"kind":"none"},"approval":"not_required","realization":"implemented",
                "limits":{"request_bytes":65536,"result_bytes":4194304,"execution_ms":20000,"provider_ms":15000,"connect_ms":5000}}})
        })
        .collect();
    private(
        &container.path(CURATION),
        json!({"format":"connectors-operation-curation/1","adapters":[{"adapter_alias":ALIAS,
            "executable_selection":adapter.selection(),
            "bootstrap_sha256":connectors_core::digest(&serde_json::to_value(bootstrap()).unwrap()),
            "operations":operations}]})
        .to_string(),
    );
    let exposures: Vec<Value> = OPERATIONS
        .iter()
        .map(|operation| {
            json!({"adapter_alias":ALIAS,"operation_ref":operation,"connection_ref":connection,
                "families":["tools"],"enabled":true})
        })
        .collect();
    private(
        &container.path(COMPANION),
        json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
            "concurrent_requests":1,"request_milliseconds":REQUEST_MS},"exposures":exposures})
        .to_string(),
    );
    assert!(container.seen("invocations").is_empty());
    container
}

fn name(operation: &str) -> String {
    connectors_mcp::names::encode_name([INSTANCE, ADAPTER, operation]).unwrap()
}

/// A primary-revision MCP client of the jailed server.
struct Peer {
    child: Child,
    frames: mpsc::Receiver<String>,
}

impl Peer {
    fn start(container: &Container) -> Self {
        let mut command = container.jail.command("/bin-test/connectors", Map::Own);
        command
            .args(["--config", CONFIG, "--state-dir", STATE])
            .args(["server", "--transport", "stdio"])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut child = loop {
            match command.spawn() {
                Ok(child) => break child,
                Err(error)
                    if error.raw_os_error() == Some(libc::ETXTBSY) && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        };
        let stdout = child.stdout.take().unwrap();
        let (sender, frames) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        Self { child, frames }
    }

    fn send(&mut self, id: i32, method: &str, mut params: Value) {
        if !params["_meta"].is_object() {
            params["_meta"] = json!({});
        }
        params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
        params["_meta"]["io.modelcontextprotocol/clientCapabilities"] = json!({});
        let frame = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(format!("{frame}\n").as_bytes()).unwrap();
    }

    /// The reply to `id` within `wait`, or what became of the server instead.
    fn reply(&mut self, id: i32, wait: Duration) -> Value {
        let started = Instant::now();
        match self.frames.recv_timeout(wait) {
            Ok(line) => {
                let reply: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(reply["id"], id, "{reply}");
                reply
            }
            Err(error) => {
                std::thread::sleep(Duration::from_millis(200));
                panic!(
                    "no reply to request {id} ({error:?} after {:?}); the server's exit: {:?}",
                    started.elapsed(),
                    self.child.try_wait()
                )
            }
        }
    }

    fn request(&mut self, id: i32, method: &str, params: Value) -> Value {
        self.send(id, method, params);
        self.reply(id, Duration::from_secs(20))
    }

    fn revision(&mut self) -> Value {
        let list = self.request(1, "tools/list", json!({}));
        let revision = list["result"]["_meta"][REVISION_META].clone();
        assert!(revision.is_string(), "{list}");
        revision
    }

    fn call(&mut self, id: i32, arguments: Value, revision: &Value) {
        self.send(
            id,
            "tools/call",
            json!({"name":name(PAGES),"arguments":arguments,"_meta":{REVISION_META:revision}}),
        );
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// projection.md, "Shaping a tool result": "All pages share the request's own
/// time budget, so a walk that outlasts it stops refused with `timeout`", and
/// "A page refused after the first stops the walk with `stopped: "refused"`
/// and the refusal's service `error`; the reply is a success carrying the
/// records already taken." Each page here takes 4 s of a 20 s budget, so the
/// walk outlasts it after about five pages.
#[test]
fn a_walk_that_outlasts_the_request_budget_answers_the_records_taken() {
    let container = served();
    let mut peer = Peer::start(&container);
    let revision = peer.revision();
    peer.call(
        2,
        json!({"input":{"ticket_id":101,"delay_ms":4000},"walk":true,"max_pages":100}),
        &revision,
    );
    let reply = peer.reply(2, Duration::from_millis(REQUEST_MS + 20_000));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let result = &reply["result"]["structuredContent"]["result"];
    assert_eq!(
        (&result["stopped"], &result["error"]["code"]),
        (&json!("refused"), &json!("timeout")),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
    assert!(
        result["pages"].as_u64().is_some_and(|pages| pages >= 1),
        "{reply}"
    );
    assert_eq!(
        result["items"].as_array().map(Vec::len),
        result["records"].as_u64().map(|n| n as usize),
        "{reply}"
    );
}

/// The advertised `max_pages` is `{"type":"integer","minimum":1,...}`, and a
/// JSON Schema integer is any number with a zero fractional part, so `2.0`
/// passes a client's validation against the tool's `inputSchema`. The
/// contract refuses only a `max_pages` "outside 1 to 10,000".
#[test]
fn max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits() {
    let container = served();
    let mut peer = Peer::start(&container);
    let revision = peer.revision();
    peer.call(
        2,
        json!({"input":{"ticket_id":101},"walk":true,"max_pages":2.0}),
        &revision,
    );
    let reply = peer.reply(2, Duration::from_secs(20));
    assert_eq!(
        reply["result"]["structuredContent"]["result"]["pages"],
        json!(2),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
}

fn declared() -> Walk {
    serde_json::from_value(
        json!({"parameter":"after","next":"/meta/next","records":"/items",
        "ends":[{"name":"no_cursor","absent":"/meta/next"}]}),
    )
    .unwrap()
}

fn page(input: &Value) -> Page {
    Page::Answered(match input.get("after").and_then(Value::as_str) {
        None => json!({"items":[{"id":1}],"meta":{"next":"c2"}}),
        Some("c2") => json!({"items":[{"id":2,"x":1}],"meta":{}}),
        Some(other) => panic!("an unknown cursor {other}"),
    })
}

/// The MCP walk's sink refuses a page past its byte bound and the walk stops
/// `bound` with that page as `resume`; its records are not returned. CLI
/// semantics, "Walking pages": `missing` lists the `--fields` paths no record
/// had, when there was a record. Here the one record taken lacks `x`; only the
/// page that was not taken has it.
#[test]
fn a_page_the_bound_refused_does_not_decide_missing() {
    let fields = connectors::shape::Fields::parse("id,x").unwrap();
    let mut taken = 0;
    let walked = walk::run(&declared(), &json!({}), 20, Some(&fields), page, |_| {
        taken += 1;
        if taken == 1 { Ok(()) } else { Err("bound") }
    });
    assert_eq!(
        (walked.records, walked.resume.clone()),
        (1, Some(json!({"after":"c2"}))),
        "{walked:?}"
    );
    assert_eq!(
        walked.missing,
        ["x"],
        "story:mcp-tool-call-shapes-its-result: the record returned lacks x: {walked:?}"
    );
}

/// As above, on the first page: no record was taken, so nothing is missing.
#[test]
fn a_first_page_the_bound_refused_reports_nothing_missing() {
    let fields = connectors::shape::Fields::parse("id,y").unwrap();
    let walked = walk::run(&declared(), &json!({}), 20, Some(&fields), page, |_| {
        Err("bound")
    });
    assert_eq!(walked.records, 0, "{walked:?}");
    assert_eq!(
        walked.missing,
        Vec::<String>::new(),
        "story:mcp-tool-call-shapes-its-result: no record was returned: {walked:?}"
    );
}
