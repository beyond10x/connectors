//! story:mcp-tool-call-shapes-its-result. `tools/call` of the production
//! `connectors server --transport stdio` takes `fields`, `each`, `walk` and
//! `max_pages` beside `input`, with the CLI's semantics, and `tools/list`
//! advertises them; every tool's `outputSchema.result` is any JSON and the
//! operation's own output schema is in the tool's `_meta`
//! (`adapters/mcp/contracts/server/v1alpha1/projection.md`, "Shaping a tool
//! result").
//!
//! The server, its owner and an adapter child run against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). The adapter child
//! is this test binary serving, over `connectors-private/3`, a Zendesk Support
//! instance shaped as `invoke_walks.rs` serves it: `ticket.audits` and
//! `ticket.comments` carry the walks the catalog declares for them under
//! `x-connectors-walk`, and each answers two pages. Every read appends its
//! operation to `/work/invocations` and its input to `/work/inputs`, so a
//! refusal before any read is an empty log. Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
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

const ADAPTER_FIXTURE: &str = "mcp_tool_call_shapes_adapter_fixture";
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "forge-local";
const ADAPTER: &str = "catalog";
const DESCRIPTION: &str = "desc-1";
const REVISION_META: &str = "io.beyond10x.connectors/revision";
const OPERATION_META: &str = "io.beyond10x.connectors/operation";
const SHOW: &str = "ticket.show";
const AUDITS: &str = "ticket.audits";
const COMMENTS: &str = "ticket.comments";
const OPERATIONS: [&str; 3] = [SHOW, AUDITS, COMMENTS];
const AUDIT_CURSOR: &str = "fixture-audit-cursor-2";
const COMMENT_CURSOR: &str = "fixture-comment-cursor-2";

/// The three synthetic audits of a ticket (`invoke_walks.rs`).
fn audits(ticket: u64) -> Vec<Value> {
    vec![
        json!({"id": 9101, "ticket_id": ticket, "author_id": 201,
               "created_at": "2025-09-15T08:00:00Z",
               "events": [{"id": 9201, "type": "Comment", "author_id": 201, "public": true,
                           "body": "fixture comment 1"},
                          {"id": 9202, "type": "Create", "field_name": "status", "value": "new"}]}),
        json!({"id": 9102, "ticket_id": ticket, "author_id": 200,
               "created_at": "2025-09-15T09:00:00Z",
               "events": [{"id": 9205, "type": "Change", "field_name": "status",
                           "value": "open", "previous_value": "new"}]}),
        json!({"id": 9103, "ticket_id": ticket, "author_id": 200,
               "created_at": "2025-09-16T10:00:00Z",
               "events": [{"id": 9207, "type": "Change", "field_name": "priority",
                           "value": "high", "previous_value": "normal"}]}),
    ]
}

fn comments() -> Vec<Value> {
    (1..=3)
        .map(|n| {
            json!({"id": 9300 + n, "author_id": 201, "public": true,
                   "body": format!("fixture comment {n}"),
                   "created_at": "2025-09-15T08:00:00Z"})
        })
        .collect()
}

fn catalog(resource: &str, body: Value) -> Value {
    json!({"status": 200, "body": body,
           "provenance": {"instance": INSTANCE, "resource": resource,
                          "observed_at_unix_ms": 1_758_000_000_000_u64,
                          "source_revision": "fixture-source-1"}})
}

/// The walks the catalog declares for `ticket.comments` and `ticket.audits`.
fn walk(records: &str, query: Value) -> Value {
    json!({"parameter": "page[after]", "next": "/meta/after_cursor", "records": records,
           "query": query,
           "continues": {"pointer": "/meta/has_more", "equals": true},
           "ends": [{"name": "has_more_false", "when": {"pointer": "/meta/has_more", "equals": false}},
                    {"name": "no_cursor", "absent": "/meta/after_cursor"},
                    {"name": "no_records", "empty": records}]})
}

fn input_schema(walk: Option<Value>) -> Value {
    let mut schema = json!({"type":"object","additionalProperties":false,
        "properties":{"ticket_id":{"type":"integer"},
            "page[size]":{"type":"integer"},"page[after]":{"type":"string"},
            "include_boundary_indicators":{"type":"boolean"}},
        "required":["ticket_id"]});
    if let Some(walk) = walk {
        schema["x-connectors-walk"] = walk;
    }
    schema
}

fn output_schema() -> Value {
    json!({"type":"object","required":["status","body","provenance"]})
}

fn walk_of(id: &str) -> Option<Value> {
    match id {
        AUDITS => Some(walk(
            "/audits",
            json!({"include_boundary_indicators": true}),
        )),
        COMMENTS => Some(walk("/comments", json!({}))),
        _ => None,
    }
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
                input_schema: input_schema(walk_of(id)),
                output_schema: output_schema(),
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

/// The Zendesk Support stand-in over bounded reads.
struct Zendesk;

#[async_trait::async_trait]
impl runtime::Adapter for Zendesk {
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
        if secret.0 != SECRET {
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
        let id = input["ticket_id"].as_u64().unwrap_or(0);
        let after = input.get("page[after]").and_then(Value::as_str);
        match (operation, after) {
            (AUDITS, None) => Ok(catalog(
                "/api/v2/tickets/{ticket_id}/audits",
                json!({"audits": audits(id)[..2],
                       "meta": {"has_more": true, "after_cursor": AUDIT_CURSOR,
                                "before_cursor": null}}),
            )),
            (AUDITS, Some(AUDIT_CURSOR)) => Ok(catalog(
                "/api/v2/tickets/{ticket_id}/audits",
                json!({"audits": audits(id)[2..],
                       "meta": {"has_more": false, "after_cursor": "fixture-audit-cursor-3",
                                "before_cursor": AUDIT_CURSOR}}),
            )),
            (COMMENTS, None) => Ok(catalog(
                "/api/v2/tickets/{ticket_id}/comments",
                json!({"comments": comments()[..2], "meta": {"after_cursor": COMMENT_CURSOR}}),
            )),
            (COMMENTS, Some(COMMENT_CURSOR)) => Ok(catalog(
                "/api/v2/tickets/{ticket_id}/comments",
                json!({"comments": comments()[2..], "meta": {"after_cursor": null}}),
            )),
            (SHOW, _) => Ok(catalog(
                "/api/v2/tickets/{ticket_id}",
                json!({"ticket": {"id": id, "status": "open", "subject": "fixture subject",
                                  "via": {"channel": "web"}}}),
            )),
            _ => Err(runtime::Failure::Unsupported),
        }
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn mcp_tool_call_shapes_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Zendesk);
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

/// One jail with a connection, the curation pinned to the adapter and every
/// operation exposed as a tool.
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
            "concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures})
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

    fn request(&mut self, id: i32, method: &str, mut params: Value) -> Value {
        if !params["_meta"].is_object() {
            params["_meta"] = json!({});
        }
        params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
        params["_meta"]["io.modelcontextprotocol/clientCapabilities"] = json!({});
        let frame = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(format!("{frame}\n").as_bytes()).unwrap();
        let line = self
            .frames
            .recv_timeout(Duration::from_secs(20))
            .expect("the server did not reply");
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }

    /// `tools/list`: the tools by operation, and the projection revision.
    fn list(&mut self) -> (Vec<Value>, Value) {
        let list = self.request(1, "tools/list", json!({}));
        let tools = list["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("{list}"))
            .clone();
        assert_eq!(tools.len(), OPERATIONS.len(), "{list}");
        let revision = list["result"]["_meta"][REVISION_META].clone();
        assert!(revision.is_string(), "{list}");
        (tools, revision)
    }

    fn call(&mut self, id: i32, operation: &str, arguments: Value, revision: &Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({"name":name(operation),"arguments":arguments,"_meta":{REVISION_META:revision}}),
        )
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The tool result's `structuredContent`, checked to be a success whose text
/// copy decodes to the same value.
#[track_caller]
fn structured(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let content = reply["result"]["structuredContent"].clone();
    assert_eq!(content["status"], "success", "{reply}");
    assert!(content["request_id"].is_string(), "{reply}");
    assert_eq!(content["audit_status"], "complete", "{reply}");
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{reply}"));
    assert_eq!(
        serde_json::from_str::<Value>(text).unwrap(),
        content,
        "story:mcp-tool-call-shapes-its-result: the text copy carries the shaped result"
    );
    content
}

fn inputs(container: &Container) -> Vec<Value> {
    container
        .seen("inputs")
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// story:mcp-tool-call-shapes-its-result: each tool's input schema declares
/// `fields` and `each` beside `input`, `walk` and `max_pages` only where the
/// operation declares a walk; `outputSchema.result` is any JSON and the
/// operation's output schema is under the tool's operation `_meta`.
#[test]
fn tools_list_declares_the_shaping_arguments_and_any_json_result() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (tools, _) = peer.list();
    for operation in OPERATIONS {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name(operation))
            .unwrap_or_else(|| panic!("{operation} is not listed: {tools:?}"));
        let schema = &tool["inputSchema"];
        let mut names: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        let expected: &[&str] = if operation == SHOW {
            &["each", "fields", "input"]
        } else {
            &["each", "fields", "input", "max_pages", "walk"]
        };
        assert_eq!(
            names, expected,
            "story:mcp-tool-call-shapes-its-result: {tool}"
        );
        assert_eq!(schema["required"], json!(["input"]), "{tool}");
        assert_eq!(schema["additionalProperties"], false, "{tool}");
        assert_eq!(
            schema["properties"]["input"],
            input_schema(walk_of(operation)),
            "{tool}"
        );
        assert_eq!(schema["properties"]["fields"]["type"], "string", "{tool}");
        assert_eq!(schema["properties"]["each"]["type"], "string", "{tool}");
        if operation != SHOW {
            assert_eq!(schema["properties"]["walk"]["type"], "boolean", "{tool}");
            assert_eq!(
                (
                    &schema["properties"]["max_pages"]["type"],
                    &schema["properties"]["max_pages"]["minimum"],
                    &schema["properties"]["max_pages"]["maximum"]
                ),
                (&json!("integer"), &json!(1), &json!(10_000)),
                "{tool}"
            );
        }
        assert_eq!(
            tool["outputSchema"]["properties"]["result"],
            json!({}),
            "story:mcp-tool-call-shapes-its-result: decision D240 c = A: {tool}"
        );
        assert_eq!(
            tool["_meta"][OPERATION_META]["output_schema"],
            output_schema(),
            "story:mcp-tool-call-shapes-its-result: {tool}"
        );
    }
    assert!(container.seen("invocations").is_empty(), "listing read");
}

/// story:mcp-tool-call-shapes-its-result: `fields` narrows the result in both
/// `structuredContent` and its text copy; the envelope stays the service's.
#[test]
fn fields_narrow_structured_content_and_the_text_copy() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (_, revision) = peer.list();
    let whole = peer.call(2, SHOW, json!({"input":{"ticket_id":101}}), &revision);
    assert_eq!(
        structured(&whole)["result"]["body"]["ticket"]["subject"],
        "fixture subject",
        "without fields the result is whole: {whole}"
    );
    let reply = peer.call(
        3,
        SHOW,
        json!({"input":{"ticket_id":101},"fields":"ticket.id,state:ticket.status,due_at"}),
        &revision,
    );
    let content = structured(&reply);
    assert_eq!(
        content["result"],
        json!({"ticket":{"id":101},"state":"open","missing":["due_at"]}),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
    assert!(content["audit_ref"].is_string(), "{reply}");
    assert!(
        reply.to_string().len() < whole.to_string().len(),
        "the shaped reply is smaller"
    );
    assert_eq!(container.seen("invocations"), [SHOW, SHOW]);
}

/// story:mcp-tool-call-shapes-its-result: `each` returns the elements of the
/// named array, each shaped by `fields`, as `items` beside their count.
#[test]
fn each_returns_the_records_as_items() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (_, revision) = peer.list();
    let reply = peer.call(
        2,
        AUDITS,
        json!({"input":{"ticket_id":101},"each":"audits","fields":"id,author:author_id"}),
        &revision,
    );
    assert_eq!(
        structured(&reply)["result"],
        json!({"records":2,"items":[{"id":9101,"author":201},{"id":9102,"author":200}]}),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
    assert_eq!(
        container.seen("invocations"),
        [AUDITS],
        "one page, one read"
    );
}

/// story:mcp-tool-call-shapes-its-result: `walk` reads both pages, one
/// admitted read each with the declared query and the cursor from the second,
/// and returns every record as `items` with the summary naming the end.
#[test]
fn walk_over_two_pages_returns_the_records_and_the_end() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (_, revision) = peer.list();
    let reply = peer.call(
        2,
        AUDITS,
        json!({"input":{"ticket_id":101,"page[size]":2},"walk":true}),
        &revision,
    );
    assert_eq!(
        structured(&reply)["result"],
        json!({"pages":2,"records":3,"complete":true,"end":"has_more_false","items":audits(101)}),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
    assert_eq!(container.seen("invocations"), [AUDITS, AUDITS]);
    assert_eq!(
        inputs(&container),
        [
            json!({"ticket_id":101,"page[size]":2,"include_boundary_indicators":true}),
            json!({"ticket_id":101,"page[size]":2,"include_boundary_indicators":true,
                "page[after]":AUDIT_CURSOR}),
        ],
        "story:mcp-tool-call-shapes-its-result: one admitted read per page"
    );
    let comments_reply = peer.call(
        3,
        COMMENTS,
        json!({"input":{"ticket_id":101},"walk":true,"fields":"id,due_at"}),
        &revision,
    );
    assert_eq!(
        structured(&comments_reply)["result"],
        json!({"pages":2,"records":3,"complete":true,"end":"no_cursor",
            "items":[{"id":9301},{"id":9302},{"id":9303}],"missing":["due_at"]}),
        "story:mcp-tool-call-shapes-its-result: {comments_reply}"
    );
}

/// story:mcp-tool-call-shapes-its-result: `max_pages` 1 stops after one page
/// with `complete:false` and `resume`, the input of the next page.
#[test]
fn max_pages_one_stops_with_resume() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (_, revision) = peer.list();
    let reply = peer.call(
        2,
        AUDITS,
        json!({"input":{"ticket_id":101,"page[size]":2},"walk":true,"max_pages":1}),
        &revision,
    );
    assert_eq!(
        structured(&reply)["result"],
        json!({"pages":1,"records":2,"complete":false,"stopped":"max_pages",
            "resume":{"ticket_id":101,"page[size]":2,"include_boundary_indicators":true,
                "page[after]":AUDIT_CURSOR},
            "items":audits(101)[..2]}),
        "story:mcp-tool-call-shapes-its-result: {reply}"
    );
    assert_eq!(container.seen("invocations"), [AUDITS]);
}

/// story:mcp-tool-call-shapes-its-result: a malformed `fields` or `each`, a
/// `walk` on an operation that declares none and a walk's own arguments out of
/// place are refused as invalid params (`-32602`) naming the argument, before
/// any read.
#[test]
fn malformed_arguments_are_refused_before_any_read() {
    let container = served();
    let mut peer = Peer::start(&container);
    let (_, revision) = peer.list();
    let input = json!({"ticket_id":101});
    let cases = [
        (SHOW, json!({"input":input,"fields":"a..b"}), "fields"),
        (SHOW, json!({"input":input,"fields":"records"}), "fields"),
        (SHOW, json!({"input":input,"fields":3}), "fields"),
        (AUDITS, json!({"input":input,"each":"audits[]"}), "each"),
        (AUDITS, json!({"input":input,"each":["audits"]}), "each"),
        (SHOW, json!({"input":input,"walk":true}), "walk"),
        (SHOW, json!({"input":input,"walk":false}), "walk"),
        (AUDITS, json!({"input":input,"walk":"yes"}), "walk"),
        (
            AUDITS,
            json!({"input":input,"walk":true,"each":"audits"}),
            "each",
        ),
        (AUDITS, json!({"input":input,"max_pages":2}), "max_pages"),
        (
            AUDITS,
            json!({"input":input,"walk":false,"max_pages":2}),
            "max_pages",
        ),
        (
            AUDITS,
            json!({"input":input,"walk":true,"max_pages":0}),
            "max_pages",
        ),
        (
            AUDITS,
            json!({"input":input,"walk":true,"max_pages":10001}),
            "max_pages",
        ),
        (
            AUDITS,
            json!({"input":input,"walk":true,"max_pages":1.5}),
            "max_pages",
        ),
        (
            AUDITS,
            json!({"input":input,"walk":true,"max_pages":"2"}),
            "max_pages",
        ),
    ];
    for (id, (operation, arguments, field)) in (2..).zip(cases) {
        let reply = peer.call(id, operation, arguments.clone(), &revision);
        assert_eq!(
            (&reply["error"]["code"], &reply["error"]["message"]),
            (&json!(-32602), &json!(format!("Invalid arguments.{field}"))),
            "story:mcp-tool-call-shapes-its-result: {arguments}: {reply}"
        );
    }
    for (id, arguments) in [
        (40, json!({"input":input,"other":1})),
        (41, json!({"fields":"id"})),
    ] {
        let reply = peer.call(id, SHOW, arguments.clone(), &revision);
        assert_eq!(
            (&reply["error"]["code"], &reply["error"]["message"]),
            (&json!(-32602), &json!("Expected arguments.input")),
            "{arguments}: {reply}"
        );
    }
    assert!(
        container.seen("invocations").is_empty(),
        "story:mcp-tool-call-shapes-its-result: a refused call read"
    );
}
