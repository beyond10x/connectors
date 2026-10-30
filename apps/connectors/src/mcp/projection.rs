//! Projection of a fresh, audited owner snapshot into the selected read-only tools slice.
use super::*;
use connectors::{
    shape,
    walk::{Page, Walk},
};
use connectors_mcp::{names, protocol, results::Envelope};
use serde_json::Value;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub(super) struct Snapshot {
    pub bootstrap: runtime::Bootstrap,
    pub revision: String,
    metadata: Value,
    pub alias: String,
    pub connection: String,
    pub host: String,
    /// The store authority the owner confirmed when this snapshot was taken.
    pub authority: String,
    configuration: Configuration,
    adapter: connectors_host::local::config::Adapter,
}

pub(super) fn refusal(code: &str) -> Value {
    json!({"code":-32000,"message":"Connectors request refused","data":{
        "version":"v1alpha2","request_id":null,"status":"error",
        "error":{"code":code,"message":"Connectors request refused"},"audit_ref":null,"audit_status":"unavailable"}})
}
fn owner_error(error: owner::Error) -> Value {
    refusal(match error.code {
        Code::NotGranted => "not_granted",
        Code::Forbidden => "forbidden",
        Code::NotFound => "not_found",
        Code::StaleDescription => "stale_description",
        Code::InvalidInput => "invalid_input",
        Code::Unsupported => "unsupported",
        Code::Capacity => "capacity",
        Code::Timeout => "timeout",
        _ => "unavailable",
    })
}
fn check(until: Instant) -> Result<(), Value> {
    if Instant::now() >= until {
        Err(refusal("timeout"))
    } else {
        Ok(())
    }
}
/// What stops a request: its own cancellation before each owner call, and the
/// end of local admission at the session's first terminal before its first one
/// (sessions § 4.1); a request already at the owner keeps its reply. The owner
/// protocol has no cancellation of its own, so an owner call already started
/// runs out within its deadline.
pub(super) struct Halt {
    pub cancelled: Arc<AtomicBool>,
    pub admission_closed: Arc<AtomicBool>,
    /// The session's standard input, asked at admission whether its peer
    /// has closed it.
    pub input: std::os::fd::RawFd,
}
impl Halt {
    fn cancelled(&self) -> Result<(), Value> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(refusal("timeout"))
        } else {
            Ok(())
        }
    }
    fn admitted(&self) -> Result<(), Value> {
        self.cancelled()?;
        // The serve loop closes admission when it reads the end of file, which
        // a loaded host can delay past this worker's first owner call; the
        // input's own hangup does not wait for that read (sessions § 4.1).
        if self.admission_closed.load(Ordering::SeqCst) || hung_up(self.input) {
            Err(refusal("unavailable"))
        } else {
            Ok(())
        }
    }
}
/// Whether `input`'s peer has closed it, buffered input or not. A poll that
/// fails answers no, which leaves the end of file to the serve loop.
fn hung_up(input: std::os::fd::RawFd) -> bool {
    let mut poll = libc::pollfd {
        fd: input,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: poll names one live entry and does not wait.
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    ready > 0 && poll.revents & libc::POLLHUP != 0
}
fn client(paths: &Paths, host: Option<&str>, until: Instant) -> Result<owner::Client, Value> {
    let client = owner::Client::connect_until(paths, host.is_none(), until).map_err(owner_error)?;
    if host.is_some_and(|host| host != client.host_incarnation) {
        return Err(refusal("stale_authority"));
    }
    Ok(client)
}

pub(super) fn snapshot(
    paths: &Paths,
    host: Option<&str>,
    until: Instant,
    halt: Option<&Halt>,
) -> Result<Snapshot, Value> {
    check(until)?;
    let configuration = load(paths).map_err(owner_error)?;
    let first = configuration
        .document()
        .exposures
        .first()
        .ok_or_else(|| refusal("unsupported"))?;
    let (alias, connection) = (first.adapter_alias.clone(), first.connection_ref.clone());
    // The selected local session binding has one configured instance/connection.
    if configuration
        .document()
        .exposures
        .iter()
        .any(|e| e.adapter_alias != alias || e.connection_ref != connection)
    {
        return Err(refusal("unsupported"));
    }
    let config = Config::load(&paths.config).map_err(|_| refusal("unavailable"))?;
    let adapter = config
        .adapters
        .get(&alias)
        .cloned()
        .ok_or_else(|| refusal("not_granted"))?;
    // The request's first owner call: past it the request is at the owner.
    halt.map_or(Ok(()), Halt::admitted)?;
    let owner = client(paths, host, until)?;
    let host = owner.host_incarnation.clone();
    let authority = owner.authority().to_owned();
    let left = until.saturating_duration_since(Instant::now()).as_millis() as u64;
    check(until)?;
    halt.map_or(Ok(()), Halt::cancelled)?;
    let bytes = owner
        .projected_describe(&alias, connectors_sdk::now_ms().saturating_add(left))
        .map_err(owner_error)?;
    check(until)?;
    let envelope =
        Envelope::from_owner(&bytes, runtime::RESULT_LIMIT).map_err(|_| refusal("unavailable"))?;
    if envelope.value()["status"] != "success" {
        return Err(envelope.rpc_error().map_err(|_| refusal("unavailable"))?);
    }
    let result = &envelope.value()["result"];
    let bootstrap: runtime::Bootstrap =
        serde_json::from_value(result["bootstrap"].clone()).map_err(|_| refusal("unavailable"))?;
    bootstrap
        .validate_for(adapter.private_protocol())
        .map_err(|_| refusal("unavailable"))?;
    if bootstrap.instance != adapter.instance_id || bootstrap.adapter != adapter.adapter_id {
        return Err(refusal("stale_description"));
    }
    let revision = result["projection_revision"]
        .as_str()
        .filter(|r| connectors_core::valid_id(r))
        .ok_or_else(|| refusal("unavailable"))?
        .to_owned();
    if load(paths).map_err(owner_error)?.document() != configuration.document()
        || Config::load(&paths.config)
            .map_err(|_| refusal("unavailable"))?
            .adapters
            .get(&alias)
            .is_none_or(|current| current.selection() != adapter.selection())
    {
        return Err(refusal("stale_description"));
    }
    Ok(Snapshot {
        bootstrap,
        revision,
        metadata: result["operation_metadata"].clone(),
        alias,
        connection,
        host,
        authority,
        configuration,
        adapter,
    })
}

impl Snapshot {
    fn limits(&self, operation: &connectors_core::Operation) -> Option<(u64, u64, usize, usize)> {
        // Nesting a schema changes root-relative reference resolution. This first
        // binding advertises only self-contained inline schemas, never a broken
        // projection or an implicit external schema fetch.
        if !inline_schema(&operation.input_schema) || !inline_schema(&operation.output_schema) {
            return None;
        }
        if self.adapter.private_protocol() != runtime::PrivateProtocol::V3 {
            return None;
        }
        let metadata = self.metadata.get(&operation.id)?;
        connectors_core::operation_metadata::Metadata::parse(
            &serde_json::to_vec(metadata).ok()?,
            &operation.profile,
            65536,
        )
        .ok()?;
        let expected = match (metadata["realization"].as_str(), operation.profile.as_str()) {
            (Some("implemented"), "resource") => (20_000, 15_000, 65_536, 4_194_304),
            (Some("generic"), "generic-http" | "generic-http-page") => {
                (40_000, 30_000, 262_144, 4_194_304)
            }
            _ => return None,
        };
        if metadata["approval"] != "not_required"
            || !matches!(
                metadata["idempotency"]["kind"].as_str(),
                Some("none" | "natural")
            )
            || metadata["semantic_effects"]
                .as_array()
                .is_none_or(|effects| !effects.is_empty())
            || metadata["effects"]
                .as_array()
                .is_none_or(|effects| effects.iter().any(|effect| effect != "network"))
            || !self
                .bootstrap
                .requirements
                .iter()
                .any(|r| r.operation == operation.id && r.effect == runtime::Effect::Read)
        {
            return None;
        }
        let limits = &metadata["limits"];
        if limits["execution_ms"].as_u64() != Some(expected.0)
            || limits["provider_ms"].as_u64() != Some(expected.1)
            || limits["request_bytes"].as_u64() != Some(expected.2 as u64)
            || limits["result_bytes"].as_u64() != Some(expected.3 as u64)
            || limits["connect_ms"].as_u64() != Some(5000)
        {
            return None;
        }
        let config = serde_json::to_value(&self.configuration.document().limits).ok()?;
        if config["frame_octets"].as_u64()? < expected.2 as u64 + 8192
            || config["response_octets"].as_u64()? < 7 * expected.3 as u64 + 8192
            || config["request_milliseconds"].as_u64()? < expected.0
        {
            return None;
        }
        Some(expected)
    }
    fn exposure(&self, id: &str) -> Option<&connectors_mcp::configuration::Exposure> {
        self.configuration
            .document()
            .exposures
            .iter()
            .find(|e| e.operation_ref == id)
            .map(Box::as_ref)
    }
    fn tool(&self, operation: &connectors_core::Operation) -> Option<Value> {
        self.limits(operation)?;
        let exposure = self.exposure(&operation.id)?;
        if !exposure.enabled
            || !exposure
                .families
                .iter()
                .any(|f| serde_json::to_value(f).ok() == Some(json!("tools")))
        {
            return None;
        }
        let name = names::encode_name([
            &self.bootstrap.instance,
            &self.bootstrap.adapter,
            &operation.id,
        ])?;
        // story:mcp-tool-call-shapes-its-result: a shaped result has no schema
        // of its own, so `result` is any JSON (decision D240 c = A) and the
        // operation's output schema is metadata.
        Some(json!({"name":name,"description":operation.description,
            "inputSchema":input_schema(operation),
            "outputSchema":response_schema(json!({})),
            "annotations":{"readOnlyHint":true,"destructiveHint":false},
            "_meta":{protocol::REVISION:self.revision,"io.beyond10x.connectors/operation":{
                "instance":self.bootstrap.instance,"adapter":self.bootstrap.adapter,"operation":operation.id,
                "contract":operation.contract,"profile":operation.profile,"metadata":self.metadata[&operation.id],
                "output_schema":operation.output_schema}}}))
    }
    fn list(&self, params: &Value, primary: bool) -> Result<Value, Value> {
        let descriptor = self
            .bootstrap
            .descriptor()
            .map_err(|_| refusal("unavailable"))?;
        let mut tools: Vec<_> = descriptor
            .operations
            .iter()
            .filter_map(|op| self.tool(op))
            .collect();
        tools.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        let scope = connectors_core::digest(
            &json!({"family":"tools","host":self.host,"revision":self.revision,"alias":self.alias,"connection":self.connection}),
        );
        let mut start = 0;
        if let Some(cursor) = params.get("cursor") {
            let cursor = cursor
                .as_str()
                .filter(|c| c.len() < 100)
                .ok_or_else(|| protocol::error(-32602, "Invalid cursor"))?;
            let (prior, offset) = cursor
                .split_once(':')
                .ok_or_else(|| protocol::error(-32602, "Invalid cursor"))?;
            if prior != scope {
                return Err(refusal("stale_cursor"));
            }
            start = offset
                .parse()
                .map_err(|_| protocol::error(-32602, "Invalid cursor"))?;
            if start == 0 || start > tools.len() || offset != start.to_string() {
                return Err(refusal("stale_cursor"));
            }
        }
        let end = (start + 32).min(tools.len());
        let mut result =
            json!({"tools":tools[start..end],"_meta":{protocol::REVISION:self.revision}});
        if end < tools.len() {
            result["nextCursor"] = json!(format!("{scope}:{end}"));
        }
        Ok(protocol::complete(result, primary, true))
    }
    fn invoke(
        &self,
        paths: &Paths,
        params: &Value,
        primary: bool,
        until: Instant,
        halt: &Halt,
    ) -> Result<Value, Value> {
        let name = params["name"]
            .as_str()
            .and_then(names::decode_name)
            .ok_or_else(|| protocol::error(-32602, "Invalid tool name"))?;
        if name[0] != self.bootstrap.instance || name[1] != self.bootstrap.adapter {
            return Err(refusal("forbidden"));
        }
        if !self.adapter.permissions.operations.contains(&name[2]) {
            return Err(refusal("not_granted"));
        }
        let revision = params["_meta"][protocol::REVISION]
            .as_str()
            .filter(|r| connectors_core::valid_id(r))
            .ok_or_else(|| protocol::error(-32602, "Missing projection revision"))?;
        if revision != self.revision {
            return Err(refusal("stale_description"));
        }
        let descriptor = self
            .bootstrap
            .descriptor()
            .map_err(|_| refusal("unavailable"))?;
        let operation = descriptor
            .operation(&name[2])
            .map_err(|_| refusal("not_found"))?;
        let (execution, provider, input_limit, result_limit) =
            self.limits(operation).ok_or_else(|| refusal("not_found"))?;
        let exposure = self
            .exposure(&operation.id)
            .ok_or_else(|| refusal("not_found"))?;
        if !exposure.enabled {
            return Err(refusal("forbidden"));
        }
        if self.tool(operation).is_none() {
            return Err(refusal("not_found"));
        }
        let arguments = params["arguments"]
            .as_object()
            .filter(|args| {
                args.contains_key("input")
                    && args.keys().all(|key| ARGUMENTS.contains(&key.as_str()))
            })
            .ok_or_else(|| protocol::error(-32602, "Expected arguments.input"))?;
        // Every shaping argument is checked before anything is read.
        let shaping = Shaping::parse(arguments, operation)?;
        let reader = Reader {
            snapshot: self,
            paths,
            operation: &operation.id,
            revision: &descriptor.revision,
            schema: owner::schema(&self.bootstrap, &operation.id).map_err(owner_error)?,
            limits: (execution, provider, input_limit, result_limit),
            until,
            halt,
        };
        let input = &arguments["input"];
        let Some((walk, max_pages)) = &shaping.walk else {
            let envelope = reader.read(input)?;
            return Ok(shaping.single(&envelope, primary));
        };
        // Each page is one admitted read through the path a single call takes.
        // A page after the first is a new owner read, so it is admitted only
        // while local admission lasts (sessions § 4.1). The summary's envelope
        // is that of the last page that answered; a refused first page is the
        // plain refusal a single call answers.
        let mut answered: Option<Envelope> = None;
        let mut refused: Option<Result<Envelope, Value>> = None;
        let mut first = true;
        let result = in_memory(
            walk,
            input,
            *max_pages,
            shaping.fields.as_ref(),
            walk_bound(input_limit, result_limit),
            |page| match if std::mem::take(&mut first) {
                reader.read(page)
            } else {
                halt.admitted().and_then(|()| reader.read(page))
            } {
                Ok(envelope) if envelope.value()["status"] == "success" => {
                    let result = envelope.value()["result"].clone();
                    answered = Some(envelope);
                    Page::Answered(result)
                }
                Ok(envelope) => {
                    let error = envelope.value()["error"].clone();
                    refused = Some(Ok(envelope));
                    Page::Refused(error)
                }
                Err(rpc) => {
                    let error = match &rpc["data"]["error"] {
                        Value::Null => rpc.clone(),
                        error => error.clone(),
                    };
                    refused = Some(Err(rpc));
                    Page::Refused(error)
                }
            },
        );
        match (answered, refused) {
            (Some(envelope), _) => {
                let mut value = envelope.value().clone();
                value["result"] = Value::Object(result);
                Ok(tool_result(value, primary))
            }
            (None, Some(Ok(envelope))) => Ok(envelope.resolved_tool(primary)),
            (None, Some(Err(rpc))) => Err(rpc),
            (None, None) => Err(refusal("unavailable")),
        }
    }
}

/// The members of `tools/call`'s `arguments` (story:mcp-tool-call-shapes-its-result).
const ARGUMENTS: [&str; 5] = ["input", "fields", "each", "walk", "max_pages"];

/// The input schema a tool advertises: the operation's input under `input`,
/// `fields` and `each` beside it, and `walk` and `max_pages` when the
/// operation declares a walk.
fn input_schema(operation: &connectors_core::Operation) -> Value {
    let mut properties = json!({"input":operation.input_schema,
        "fields":{"type":"string","description":"Comma-separated paths of the result to return: name(.name)*, [] maps over an array, alias:path renames, a leading / is an RFC 6901 pointer. A path whose first name a catalog result lacks reads its body."},
        "each":{"type":"string","description":"The path of an array in the result, by member names: its elements are returned as result.items, each shaped by fields."}});
    if matches!(Walk::declared(&operation.input_schema), Ok(Some(_))) {
        properties["walk"] = json!({"type":"boolean",
            "description":"Read every page this operation declares, one admitted read per page; the records are returned as result.items beside the walk's summary."});
        properties["max_pages"] = json!({"type":"integer","minimum":1,"maximum":connectors::walk::MAX_PAGES,
            "default":connectors::walk::DEFAULT_PAGES,"description":"The most pages a walk reads."});
    }
    json!({"type":"object","properties":properties,"required":["input"],"additionalProperties":false})
}

/// An invalid shaping argument: invalid params naming it, as a call without
/// `arguments.input` is refused.
fn invalid(argument: &str) -> Value {
    protocol::error(-32602, &format!("Invalid arguments.{argument}"))
}

/// What a call asks of its result beside its input.
struct Shaping {
    fields: Option<shape::Fields>,
    each: Option<shape::Path>,
    walk: Option<(Walk, u32)>,
}

impl Shaping {
    /// The shaping arguments as the CLI's `--fields`, `--each`, `--walk` and
    /// `--max-pages` take them: `fields` and `each` strings of its grammar,
    /// `walk` a boolean only on an operation that declares a walk, `each` not
    /// beside a walk and `max_pages` a whole number from 1 to 10,000 only
    /// beside one.
    fn parse(
        arguments: &serde_json::Map<String, Value>,
        operation: &connectors_core::Operation,
    ) -> Result<Self, Value> {
        let text = |name: &str| match arguments.get(name) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.as_str())),
            Some(_) => Err(invalid(name)),
        };
        let fields = text("fields")?
            .map(|text| shape::Fields::parse(text).map_err(|_| invalid("fields")))
            .transpose()?;
        let each = text("each")?
            .map(|text| shape::Path::parse(text).map_err(|_| invalid("each")))
            .transpose()?;
        let walking = match arguments.get("walk") {
            None => false,
            Some(walk) => {
                let declared = Walk::declared(&operation.input_schema).ok().flatten();
                match (walk, declared) {
                    (Value::Bool(false), Some(_)) => false,
                    (Value::Bool(true), Some(declared)) => {
                        if each.is_some() {
                            return Err(invalid("each"));
                        }
                        let pages = match arguments.get("max_pages") {
                            None => connectors::walk::DEFAULT_PAGES,
                            Some(pages) => pages
                                .as_u64()
                                .filter(|pages| {
                                    (1..=u64::from(connectors::walk::MAX_PAGES)).contains(pages)
                                })
                                .ok_or_else(|| invalid("max_pages"))?
                                as u32,
                        };
                        return Ok(Self {
                            fields,
                            each,
                            walk: Some((declared, pages)),
                        });
                    }
                    _ => return Err(invalid("walk")),
                }
            }
        };
        if !walking && arguments.contains_key("max_pages") {
            return Err(invalid("max_pages"));
        }
        Ok(Self {
            fields,
            each,
            walk: None,
        })
    }

    /// A single call's tool result: the service response with its `result`
    /// shaped as `invoke --fields`/`--each` shapes it, `each`'s elements as
    /// `items`. A refusal, a catalog result whose status is not 2xx and a
    /// call without either stay whole.
    fn single(&self, envelope: &Envelope, primary: bool) -> Value {
        let value = envelope.value();
        let shaped = (value["status"] == "success")
            .then(|| {
                shape::answer(
                    &json!({"result":value["result"]}),
                    self.fields.as_ref(),
                    self.each.as_ref(),
                )
            })
            .flatten();
        let Some(shaped) = shaped else {
            return envelope.resolved_tool(primary);
        };
        let mut result = shaped.result;
        if self.each.is_some() {
            result["items"] = Value::Array(shaped.records);
        }
        let mut value = value.clone();
        value["result"] = result;
        tool_result(value, primary)
    }
}

/// A resolved tool's result carrying `value`, a service response, in both
/// representations, as `Envelope::resolved_tool` carries an unshaped one.
fn tool_result(value: Value, primary: bool) -> Value {
    let text = value.to_string();
    protocol::complete(
        json!({"isError":value["status"] == "error","structuredContent":value,
            "content":[{"type":"text","text":text}]}),
        primary,
        false,
    )
}

/// The bytes a walk's records may take: the operation's result bound, which
/// the configured response bound holds in both representations (see
/// `Snapshot::limits`), less room for the summary and its `resume`, a page
/// input within the request bound.
fn walk_bound(input_limit: usize, result_limit: usize) -> usize {
    result_limit.saturating_sub(input_limit.saturating_add(8192))
}

/// A walk run in memory: its summary (`connectors.cli.InvokeWalked` without
/// `out`) with the records as `items`. A page whose records would take them
/// past `bound` bytes of JSON is not taken, and the walk stops `bound` with
/// that page as `resume`.
fn in_memory(
    walk: &Walk,
    caller: &Value,
    max_pages: u32,
    fields: Option<&shape::Fields>,
    bound: usize,
    read: impl FnMut(&Value) -> Page,
) -> serde_json::Map<String, Value> {
    let mut items = Vec::new();
    let mut used = 0usize;
    let walked = connectors::walk::run(walk, caller, max_pages, fields, read, |records| {
        let size = records
            .iter()
            .map(|record| record.to_string().len() + 1)
            .sum::<usize>();
        if used.saturating_add(size) > bound {
            return Err("bound");
        }
        used += size;
        items.extend(records);
        Ok(())
    });
    let mut summary = walked.summary();
    summary.insert("items".into(), Value::Array(items));
    summary
}

/// One admitted, audited and bounded owner read of a resolved tool's
/// operation: what a single call does once, and a walk once per page.
struct Reader<'a> {
    snapshot: &'a Snapshot,
    paths: &'a Paths,
    operation: &'a str,
    revision: &'a str,
    schema: String,
    limits: (u64, u64, usize, usize),
    until: Instant,
    halt: &'a Halt,
}

impl Reader<'_> {
    /// The owner's service response to `input`: a success, or an execution
    /// failure the tool-error channel carries; `Err` is an rpc-error.
    fn read(&self, input: &Value) -> Result<Envelope, Value> {
        let (execution, provider, input_limit, result_limit) = self.limits;
        let (snapshot, until, halt) = (self.snapshot, self.until, self.halt);
        let input = input.to_string();
        let budget = runtime::ReadBudget::start(execution, provider, input_limit, result_limit)
            .and_then(|b| b.constrain(until))
            .map_err(|_| refusal("timeout"))?;
        halt.cancelled()?;
        let owner = client(self.paths, Some(&snapshot.host), until)?;
        // The last check before the owner read: past it a cancellation only
        // suppresses the reply.
        halt.cancelled()?;
        let bytes = owner
            .bounded_projected_read(
                &snapshot.alias,
                &owner::approval_issuance::Request {
                    connection: &snapshot.connection,
                    operation: self.operation,
                    schema: &self.schema,
                    revision: self.revision,
                    input: &input,
                },
                &snapshot.revision,
                budget,
            )
            .map_err(owner_error)?;
        check(until)?;
        let envelope =
            Envelope::from_owner(&bytes, result_limit).map_err(|_| refusal("unavailable"))?;
        if envelope.value()["status"] == "error"
            && (matches!(
                envelope.value()["error"]["code"].as_str(),
                Some(
                    "not_granted" | "forbidden" | "not_found" | "stale_description" | "unsupported"
                )
            ) || envelope.value()["audit_status"] == "unavailable")
        {
            return Err(envelope.rpc_error().map_err(|_| refusal("unavailable"))?);
        }
        Ok(envelope)
    }
}

pub(super) fn execute(
    paths: &Paths,
    host: &str,
    request: &Value,
    primary: bool,
    until: Instant,
    halt: &Halt,
) -> Result<Value, Value> {
    halt.admitted()?;
    let current = snapshot(paths, Some(host), until, Some(halt))?;
    match request["method"].as_str() {
        Some("tools/list") => current.list(&request["params"], primary),
        Some("tools/call") => current.invoke(paths, &request["params"], primary, until, halt),
        _ => Err(protocol::error(-32601, "Method not found")),
    }
}

// Native projection of the already-selected service envelope: the provider's
// schema is nested under result, never asserted to describe the outer response.
fn response_schema(result: Value) -> Value {
    json!({"type":"object","required":["version","request_id","status","audit_ref","audit_status"],
        "properties":{"version":{"const":"v1alpha2"},"request_id":{"type":["string","null"]},
            "status":{"enum":["success","error"]},"result":result,"error":{"type":"object"},
            "audit_ref":{"type":["string","null"]},"audit_status":{"enum":["complete","incomplete","unavailable","not_required"]},
            "source_audit":{"type":"object"},"mutation":{"type":"object"}},
        "oneOf":[{"properties":{"status":{"const":"success"}},"required":["result"],"not":{"required":["error"]}},
            {"properties":{"status":{"const":"error"}},"required":["error"],"not":{"required":["result"]}}],"additionalProperties":false})
}

fn inline_schema(value: &Value) -> bool {
    match value {
        Value::Object(fields) => fields.iter().all(|(key, value)| {
            !matches!(
                key.as_str(),
                "$ref"
                    | "$dynamicRef"
                    | "$recursiveRef"
                    | "$id"
                    | "$anchor"
                    | "$dynamicAnchor"
                    | "$recursiveAnchor"
            ) && inline_schema(value)
        }),
        Value::Array(values) => values.iter().all(inline_schema),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    fn pipe() -> (OwnedFd, OwnedFd) {
        let mut fds = [0; 2];
        // SAFETY: fds is a live two-entry array the call fills.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        // SAFETY: pipe returned two fresh descriptors this test now owns.
        unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
    }

    fn halt(input: &OwnedFd) -> Halt {
        Halt {
            cancelled: Arc::new(AtomicBool::new(false)),
            admission_closed: Arc::new(AtomicBool::new(false)),
            input: input.as_raw_fd(),
        }
    }

    /// sessions § 4.1, local MCP stdio: no request reaches the owner after
    /// standard input closed. Admission asks the input itself, so a request is
    /// refused once its peer has closed, even before the serve loop has read
    /// the end of file and closed admission.
    #[test]
    fn admission_refuses_once_the_peer_closed_its_input() {
        let (read, write) = pipe();
        assert_eq!(halt(&read).admitted(), Ok(()), "input still open");
        // SAFETY: write is a live pipe end; the frame is written whole.
        let frame = b"{\"jsonrpc\":\"2.0\"}\n";
        assert_eq!(
            unsafe { libc::write(write.as_raw_fd(), frame.as_ptr().cast(), frame.len()) },
            frame.len() as isize
        );
        assert_eq!(
            halt(&read).admitted(),
            Ok(()),
            "unread input with the peer still open"
        );
        drop(write);
        assert_eq!(
            halt(&read).admitted(),
            Err(refusal("unavailable")),
            "unread input after the peer closed"
        );
    }

    /// story:mcp-tool-call-shapes-its-result: a walk over MCP keeps its records
    /// in memory as `items`; a page that would take them past the bound is not
    /// taken, and the walk stops `bound`, incomplete, with that page as `resume`.
    #[test]
    fn a_walk_past_the_response_bound_stops_with_resume() {
        let walk: Walk = serde_json::from_value(json!({"parameter":"after",
            "next":"/meta/next","records":"/items",
            "ends":[{"name":"no_cursor","absent":"/meta/next"}]}))
        .unwrap();
        let page = |input: &Value| {
            Page::Answered(match input.get("after").and_then(Value::as_str) {
                None => json!({"items":[{"id":1},{"id":2}],"meta":{"next":"c2"}}),
                Some("c2") => json!({"items":[{"id":3}],"meta":{}}),
                Some(other) => panic!("an unknown cursor {other}"),
            })
        };
        let caller = json!({"q":1});
        assert_eq!(
            Value::Object(in_memory(&walk, &caller, 20, None, 1024, page)),
            json!({"pages":2,"records":3,"complete":true,"end":"no_cursor",
                "items":[{"id":1},{"id":2},{"id":3}]})
        );
        // `{"id":1}` and `{"id":2}` take 9 bytes each with their separator.
        assert_eq!(
            Value::Object(in_memory(&walk, &caller, 20, None, 20, page)),
            json!({"pages":1,"records":2,"complete":false,"stopped":"bound",
                "resume":{"q":1,"after":"c2"},"items":[{"id":1},{"id":2}]}),
            "story:mcp-tool-call-shapes-its-result"
        );
        assert_eq!(
            Value::Object(in_memory(&walk, &caller, 20, None, 17, page)),
            json!({"pages":0,"records":0,"complete":false,"stopped":"bound",
                "resume":{"q":1},"items":[]}),
            "story:mcp-tool-call-shapes-its-result: the first page past the bound"
        );
        assert_eq!(walk_bound(65_536, 4_194_304), 4_194_304 - 65_536 - 8192);
    }

    #[test]
    fn nested_projection_does_not_advertise_relocated_schema_references() {
        assert!(inline_schema(
            &json!({"type":"object","properties":{"value":{"type":"number"}}})
        ));
        for schema in [
            json!({"$ref":"#/$defs/value"}),
            json!({"properties":{"nested":{"$dynamicRef":"#node"}}}),
            json!({"$id":"https://schema.invalid/root"}),
        ] {
            assert!(!inline_schema(&schema));
        }
    }
}
