//! The canonical document, typed.
//!
//! One deterministic JSON document per provider (`catalog/<id>.catalog.json`, C-536), served from
//! the embedded pack by the dependency-free reader (C-537). This module turns the parts of it a
//! **request** is made of into types: the services' base URLs, and each operation's request
//! template, declared parameters and endpoint slots.
//!
//! Everything else the document carries — response schemas, operation traits, events, the OAuth2 spec — is
//! skipped rather than modelled. Interpreting those is somebody else's job, and a struct that
//! claimed them would have to be kept in step with a schema this crate does not own.
//!
//! # The caller-facing symbol, which the document now carries (C-552)
//!
//! A caller addresses a parameter by the name the operation's **contract** advertises, and that name
//! is a *Flux symbol*: `time.start` is declared `time_start`, `$top` is `_top`, and a parameter
//! called `response` becomes `response_2` because the emitter binds `response` itself. The document
//! publishes the IR name (`time.start`) and the wire name; since C-552 it **also** publishes the
//! symbol, computed at build time by the emitter's own allocator (which reserves a symbol for every
//! body parameter, `const`-pinned ones included) and stored beside `name`. So this crate **reads**
//! the symbol rather than reproducing the allocation, and the whole-catalogue differential gate
//! (`connector-pack/tests/main/catalogue_differential.rs`) asserts, for all 835 operations, that the
//! stored symbol is the name the emitted declaration declares.
//!
//! [`Symbols`] survives only as the fallback for a pre-C-552 document that carries no symbol — no
//! build this repository produces — kept under the C-537 forward-compat contract that an additive
//! field's absence must still read. Reading the stored symbol is what closed C-538's ADJACENT-2
//! trap: a `const`-pinned body field whose name normalizes onto a later parameter's symbol shifts
//! that parameter under the emitter's allocation, and this crate's reproduction — which never saw
//! the `const`-pinned field, because the document omits it — would have missed the shift.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::Value;

use crate::slot::Slot;

/// The symbols the emitter binds inside an op body, reserved unconditionally.
///
/// Transcribed from `connector-flux`'s `names.rs::RESERVED`. Reserved everywhere rather than per
/// operation, exactly as there: a `GET` binds neither `payload` nor `content_type`, but reserving
/// them everywhere keeps a parameter's symbol independent of which other parameters its operation
/// happens to declare.
const RESERVED: &[&str] = &[
    "base",
    "url",
    "content_type",
    "payload",
    "form_sep",
    "response",
];

/// The parameter name a free-form body is declared under.
const FREE_FORM_BODY: &str = "body";

/// Hands out a unique Flux symbol name for each vendor parameter name, in declaration order.
///
/// A reproduction of `connector-flux`'s `Symbols`. Stateful rather than a pure function because
/// `time.start` and `time_start` are distinct names that normalize identically, and collapsing them
/// would send one parameter's value under the other's name.
struct Symbols {
    taken: BTreeSet<String>,
}

impl Symbols {
    fn new() -> Self {
        Self {
            taken: RESERVED.iter().map(|name| (*name).to_string()).collect(),
        }
    }

    /// The Flux symbol for `wire`, reserving it so no later parameter can collide with it.
    ///
    /// 1. every character outside `[A-Za-z0-9_]` becomes `_` (`time.start` → `time_start`);
    /// 2. a name that would start with a digit is prefixed with `p_`;
    /// 3. a name already taken gains a `_2`, `_3`, … suffix.
    fn allocate(&mut self, wire: &str) -> String {
        let mut symbol: String = wire
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        if symbol.starts_with(|c: char| c.is_ascii_digit()) {
            symbol.insert_str(0, "p_");
        }
        if self.taken.contains(&symbol) {
            let base = symbol.clone();
            let mut n = 2;
            while self.taken.contains(&symbol) {
                symbol = format!("{base}_{n}");
                n += 1;
            }
        }
        self.taken.insert(symbol.clone());
        symbol
    }

    /// Reserve a symbol the document already states, so a later fallback allocation cannot reuse it.
    ///
    /// Consulted only for a pre-C-552 document that carries symbols for some parameters and not
    /// others — which no build this repository produces — and harmless otherwise.
    fn reserve(&mut self, symbol: &str) {
        self.taken.insert(symbol.to_owned());
    }
}

// ---------------------------------------------------------------------------------------------
// The document, as JSON
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawDocument {
    connector: String,
    #[serde(default)]
    services: Vec<RawService>,
    #[serde(default)]
    operations: Vec<RawOperation>,
}

#[derive(Debug, Deserialize)]
struct RawService {
    name: String,
    base_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum HostEffect {
    Read,
    Write,
    Network,
    Process,
    Browser,
    Filesystem,
    LocalSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum InteractionShape {
    Unary,
    Stream,
    Subscription,
    LeasedSession,
    SessionEstablishment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ProtocolDriver {
    HttpV1,
    SipV1,
    AudioV1,
    CdpV1,
    SqlV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum PlacementRequirement {
    ConnectorsDeployment,
    SubstrateWorkload,
    FederatedSatellite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ImplementationForm {
    BuiltIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum RequiredCapability {
    PublicNetwork,
    PrivateNetwork,
    UnixSocket,
    FileSecret,
    Process,
    Container,
    Device,
}

#[derive(Debug, Deserialize)]
struct RawOperation {
    id: String,
    service: String,
    #[serde(default)]
    expose: bool,
    #[serde(default)]
    params: Vec<RawParam>,
    #[serde(flatten)]
    protocol: RawProtocolOperation,
    #[serde(default)]
    endpoint: BTreeMap<String, Vec<String>>,
    /// The declared risk tier, as flux's own vocabulary spells it (`low`/`medium`/…). Carried so a
    /// consumer builds the model-facing contract from the document rather than the emitted Flux
    /// (C-552). Empty for a document written before the field existed.
    #[serde(default)]
    risk: String,
    /// The declared idempotency, likewise (`idempotent`/`non_idempotent`/`conditional`).
    #[serde(default)]
    idempotency: String,
    /// The **host** effects the authority projection reads — `["read", "network"]` and the like,
    /// read from the document and never derived (C-552). Distinct from `semantic_effects`, which the
    /// document also carries and which this crate does not model.
    effects: Vec<HostEffect>,
    interaction_shape: InteractionShape,
    placement_requirement: PlacementRequirement,
    implementation_form: ImplementationForm,
    required_capabilities: Vec<RequiredCapability>,
    /// The model-facing contract projection — the error-envelope-extended description and the
    /// lowered, Flux-typed input schema — computed at build time and stored so a consumer needs no
    /// engine to read them (C-552). Absent for a document written before the field existed.
    #[serde(default)]
    contract: Option<RawContract>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "protocol_driver", rename_all = "snake_case")]
enum RawProtocolOperation {
    HttpV1 { request: RequestTemplate },
    SipV1 { request: EmptySipRequest },
    AudioV1 { request: EmptyAudioRequest },
    CdpV1 { request: EmptyCdpRequest },
    SqlV1 { request: EmptySqlRequest },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptySipRequest {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyAudioRequest {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyCdpRequest {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptySqlRequest {}

/// The stored model-facing contract projection: the ToolSpec's description and input schema, as the
/// build computed them from the emitted declaration's own lowering.
#[derive(Debug, Deserialize)]
struct RawContract {
    #[serde(default)]
    description: String,
    #[serde(default)]
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct RawParam {
    name: String,
    position: String,
    /// The caller-facing Flux symbol the emitted `op` declares this parameter by (C-552) — the name
    /// a caller addresses it under, which is not its document `name`. Empty for a document written
    /// before the field existed, in which case [`Operation::resolve`] falls back to reproducing the
    /// emitter's allocation.
    #[serde(default)]
    symbol: String,
    /// Whether the caller must supply this parameter.
    ///
    /// Defaulted to optional for a document written before the field existed — the same
    /// forward-compatibility contract `symbol` carries. Every shipped catalogue emits it.
    #[serde(default)]
    required: bool,
}

/// One operation's request, in the document's closed template vocabulary.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestTemplate {
    /// The HTTP method, stated rather than defaulted.
    pub method: String,
    /// The URL, always `{base}/…`, interpolating endpoint slots and caller parameters.
    pub url: String,
    /// The headers, keyed by wire name. A `BTreeMap`, which is also the order they travel in.
    #[serde(default)]
    pub headers: BTreeMap<String, ValueTemplate>,
    /// The structured query, in the order the document states.
    #[serde(default)]
    pub query: Vec<QueryEntry>,
    /// The body, when the operation has one.
    #[serde(default)]
    pub body: Option<BodyTemplate>,
}

/// A literal (which may interpolate endpoint slots) or a whole-parameter splice.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ValueTemplate {
    /// `{"$param": "name"}` — the whole value of the named caller parameter.
    Splice {
        /// The parameter's **document** name, which is not its caller-facing symbol.
        #[serde(rename = "$param")]
        param: String,
    },
    /// A literal, which may carry `{slot}` placeholders.
    Literal(String),
}

/// One structured-query pair.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryEntry {
    /// The wire name, as the vendor spells it.
    pub name: String,
    /// The value.
    pub value: ValueTemplate,
}

/// The request body, by encoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "encoding", rename_all = "lowercase")]
pub enum BodyTemplate {
    /// A JSON body: literals, nested objects and arrays, `$param` splices at caller leaves.
    Json {
        /// The template.
        template: Value,
    },
    /// A form-encoded body.
    Form {
        /// The fields, in the order they travel.
        fields: Vec<FormField>,
    },
}

/// One form-encoded body field.
#[derive(Debug, Clone, Deserialize)]
pub struct FormField {
    /// The wire name.
    pub name: String,
    /// The value.
    pub value: ValueTemplate,
    /// Whether the field always travels, or only when its value is truthy.
    pub required: bool,
}

// ---------------------------------------------------------------------------------------------
// The resolved view
// ---------------------------------------------------------------------------------------------

/// One provider's canonical document, resolved into the shape a request is derived from.
#[derive(Debug)]
pub struct Document {
    /// The connector id.
    pub connector: String,
    /// Each service's base URL, keyed by service name.
    services: BTreeMap<String, String>,
    /// Each operation, keyed by id.
    operations: BTreeMap<String, Operation>,
}

impl Document {
    /// Parse one provider document from its canonical JSON text.
    ///
    /// # Errors
    ///
    /// The parse failure, as `serde_json` states it. Unreachable for a document this repository
    /// generated — `connector-cli`'s `build` writes it and the pack's reader verifies the bytes —
    /// and reported rather than unwrapped, because the alternative is a panic inside a host's
    /// registration call.
    pub fn parse(text: &str) -> Result<Document, String> {
        let raw: RawDocument = serde_json::from_str(text).map_err(|error| error.to_string())?;
        let services = raw
            .services
            .into_iter()
            .map(|service| (service.name, service.base_url))
            .collect();
        let operations = raw
            .operations
            .into_iter()
            .map(|operation| (operation.id.clone(), Operation::resolve(operation)))
            .collect();
        Ok(Document {
            connector: raw.connector,
            services,
            operations,
        })
    }

    /// The base URL template of `service`.
    pub fn base_url(&self, service: &str) -> Option<&str> {
        self.services.get(service).map(String::as_str)
    }

    /// One operation, by id.
    pub fn operation(&self, id: &str) -> Option<&Operation> {
        self.operations.get(id)
    }

    /// Every operation this document carries, in id order.
    pub fn operations(&self) -> impl Iterator<Item = &Operation> {
        self.operations.values()
    }
}

/// One operation's document: its request template, and everything derived from it once.
#[derive(Debug)]
pub struct Operation {
    /// The operation id.
    pub id: String,
    /// The service it belongs to — `default` for a connector with a single API surface.
    pub service: String,
    /// Whether the operation reaches a model as a tool (C-413).
    pub expose: bool,
    /// Driver-specific request template. SIP never masquerades as an HTTP request.
    pub request: ProtocolRequestTemplate,
    /// The configuration variables the request needs, in stable order.
    variables: Vec<String>,
    /// Where each of them lands.
    slots: BTreeMap<String, Slot>,
    /// The document name of each declared parameter, in declaration order.
    parameters: Vec<String>,
    /// Document name → the caller-facing symbol. See this module's documentation.
    symbols: BTreeMap<String, String>,
    /// Caller parameters the template places in a URL path segment (C-478), by symbol.
    caller_path_parameters: BTreeSet<String>,
    /// The caller-facing symbol of every parameter a caller may simply leave out.
    ///
    /// Exactly the ones the document does not mark required, whatever their position.
    ///
    /// Position is deliberately not part of this. Across the shipped catalogue every `path`
    /// parameter is already marked required — there are none that are not — so the URL cannot be
    /// left holding a literal `{ticket_id}`. What position *would* have excluded is body fields,
    /// and there are 193 optional ones: an operation like `freshdesk-ticket-create` declares a
    /// dozen optional fields, and requiring them turned every create in the catalogue into a call
    /// nobody could make without sending a null for each.
    caller_omittable_parameters: BTreeSet<String>,
    /// The error-envelope-extended description the model-facing contract carries (C-552).
    description: String,
    /// The lowered, Flux-typed input schema the model-facing contract carries (C-552).
    input_schema: Value,
    /// The host effects the authority projection reads, read from the document (C-552).
    effects: Vec<HostEffect>,
    interaction_shape: InteractionShape,
    placement_requirement: PlacementRequirement,
    implementation_form: ImplementationForm,
    required_capabilities: Vec<RequiredCapability>,
    /// The declared risk tier, as flux's vocabulary spells it (C-552).
    risk: String,
    /// The declared idempotency, likewise (C-552).
    idempotency: String,
}

/// The canonical document's closed driver request vocabulary.
#[derive(Debug, Clone)]
pub enum ProtocolRequestTemplate {
    /// One HTTP request template.
    HttpV1(RequestTemplate),
    /// One admitted SIP session-establishment operation.
    SipV1,
    /// One admitted local-audio unary operation.
    AudioV1,
    /// One admitted browser operation on a leased `DevTools` session.
    CdpV1,
    /// One admitted bounded database read over the closed SQL driver.
    SqlV1,
}

impl ProtocolRequestTemplate {
    /// The selected closed driver.
    pub const fn driver(&self) -> ProtocolDriver {
        match self {
            Self::HttpV1(_) => ProtocolDriver::HttpV1,
            Self::SipV1 => ProtocolDriver::SipV1,
            Self::AudioV1 => ProtocolDriver::AudioV1,
            Self::CdpV1 => ProtocolDriver::CdpV1,
            Self::SqlV1 => ProtocolDriver::SqlV1,
        }
    }

    /// Borrow the HTTP request template, if this operation is HTTP.
    pub const fn http(&self) -> Option<&RequestTemplate> {
        match self {
            Self::HttpV1(request) => Some(request),
            Self::SipV1 | Self::AudioV1 | Self::CdpV1 | Self::SqlV1 => None,
        }
    }
}

impl Operation {
    fn resolve(raw: RawOperation) -> Operation {
        let mut allocator = Symbols::new();
        let mut symbols = BTreeMap::new();
        let mut parameters = Vec::new();
        let mut caller_path_parameters = BTreeSet::new();
        let mut caller_omittable_parameters = BTreeSet::new();
        for param in &raw.params {
            // **The caller-facing symbol is the document's** (C-552): the emitter computed it at
            // build time and stored it beside `name`, so no consumer reproduces the allocation. A
            // document written before the field existed carries no symbol, and only then does the
            // allocator below reproduce it — the fallback keeps an older document readable under the
            // C-537 forward-compat contract, and the whole-catalogue differential gate proves the
            // stored symbol is the emitter's for every shipped operation.
            let symbol = if param.symbol.is_empty() {
                allocator.allocate(&param.name)
            } else {
                allocator.reserve(&param.symbol);
                param.symbol.clone()
            };
            if param.position == "path" {
                caller_path_parameters.insert(symbol.clone());
            }
            if !param.required {
                caller_omittable_parameters.insert(symbol.clone());
            }
            parameters.push(param.name.clone());
            symbols.insert(param.name.clone(), symbol);
        }

        let (description, input_schema) = match raw.contract {
            Some(contract) => (contract.description, contract.input_schema),
            None => (String::new(), Value::Null),
        };

        let slots: BTreeMap<String, Slot> = raw
            .endpoint
            .iter()
            .map(|(variable, positions)| {
                // One declared position is that slot; more than one is `Unplaced`, which is the
                // intersection of every rule rather than the absence of any (C-229).
                let slot = match positions.as_slice() {
                    [only] => Slot::from_document(only),
                    _ => Slot::Unplaced,
                };
                (variable.clone(), slot)
            })
            .collect();

        let request = match raw.protocol {
            RawProtocolOperation::HttpV1 { request } => ProtocolRequestTemplate::HttpV1(request),
            RawProtocolOperation::SipV1 { request } => {
                let EmptySipRequest {} = request;
                ProtocolRequestTemplate::SipV1
            }
            RawProtocolOperation::AudioV1 { request } => {
                let EmptyAudioRequest {} = request;
                ProtocolRequestTemplate::AudioV1
            }
            RawProtocolOperation::CdpV1 { request } => {
                let EmptyCdpRequest {} = request;
                ProtocolRequestTemplate::CdpV1
            }
            RawProtocolOperation::SqlV1 { request } => {
                let EmptySqlRequest {} = request;
                ProtocolRequestTemplate::SqlV1
            }
        };

        Operation {
            variables: slots.keys().cloned().collect(),
            slots,
            parameters,
            symbols,
            caller_path_parameters,
            caller_omittable_parameters,
            id: raw.id,
            service: raw.service,
            expose: raw.expose,
            request,
            description,
            input_schema,
            effects: raw.effects,
            interaction_shape: raw.interaction_shape,
            placement_requirement: raw.placement_requirement,
            implementation_form: raw.implementation_form,
            required_capabilities: raw.required_capabilities,
            risk: raw.risk,
            idempotency: raw.idempotency,
        }
    }

    /// The configuration variables this operation's request needs, in stable order.
    pub fn endpoint_variables(&self) -> &[String] {
        &self.variables
    }

    /// **The error-envelope-extended description the model-facing contract carries** (C-552).
    ///
    /// Not the document's one-line `description`: the extended text a model is handed, computed at
    /// build time from the emitted declaration's own lowering and stored so a consumer needs no
    /// engine to read it. Empty for a document written before the field existed.
    pub fn contract_description(&self) -> &str {
        &self.description
    }

    /// **The lowered, Flux-typed input schema the model-facing contract carries** (C-552).
    ///
    /// `OpSpec::lower`'s output — keyed by caller-facing Flux symbols, with Flux types (`int64`
    /// integer, not the vendor's `number`) — so a consumer maps it directly into a `flux_spec`
    /// ToolSpec without parsing the emitted Flux. [`Value::Null`] for a document written before the
    /// field existed.
    pub fn input_schema(&self) -> &Value {
        &self.input_schema
    }

    /// **The host effects the authority projection reads** (C-552), read from the document and never
    /// derived — `["read", "network"]` and the like. Distinct from `semantic_effects`.
    pub fn effects(&self) -> &[HostEffect] {
        &self.effects
    }

    /// The declared member lifecycle.
    pub fn interaction_shape(&self) -> InteractionShape {
        self.interaction_shape
    }
    /// The closed, versioned protocol driver.
    pub fn protocol_driver(&self) -> ProtocolDriver {
        self.request.driver()
    }
    /// The placement requirement, before deployment selection.
    pub fn placement_requirement(&self) -> PlacementRequirement {
        self.placement_requirement
    }
    /// How the implementation is supplied.
    pub fn implementation_form(&self) -> ImplementationForm {
        self.implementation_form
    }
    /// Capabilities admission must prove before dispatch.
    pub fn required_capabilities(&self) -> &[RequiredCapability] {
        &self.required_capabilities
    }

    /// The declared risk tier, as flux's vocabulary spells it (C-552).
    pub fn risk(&self) -> &str {
        &self.risk
    }

    /// The declared idempotency, as flux's vocabulary spells it (C-552).
    pub fn idempotency(&self) -> &str {
        &self.idempotency
    }

    /// Where each of those variables lands on the request.
    pub fn endpoint_slots(&self) -> &BTreeMap<String, Slot> {
        &self.slots
    }

    /// Caller-visible parameters the request template places in the URL path.
    pub fn caller_path_parameters(&self) -> &BTreeSet<String> {
        &self.caller_path_parameters
    }

    /// The caller-facing symbol of every parameter a caller may simply leave out.
    #[must_use]
    pub fn caller_omittable_parameters(&self) -> &BTreeSet<String> {
        &self.caller_omittable_parameters
    }

    /// The caller-facing name of each declared parameter, in declaration order.
    pub fn caller_parameters(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .map(|name| self.symbols[name].as_str())
            .collect()
    }

    /// The caller-facing symbol of the document parameter `name`.
    pub(crate) fn symbol(&self, name: &str) -> Option<&str> {
        self.symbols.get(name).map(String::as_str)
    }

    /// Whether this operation declares a free-form body — one parameter carrying the whole payload,
    /// which travels through `parse(…, as: "json")` and is therefore validated rather than sent
    /// verbatim.
    pub(crate) fn has_free_form_body(&self) -> bool {
        matches!(self.request.http().and_then(|request| request.body.as_ref()),
            Some(BodyTemplate::Json { template })
                if matches!(splice_of(template), Some(name) if name == FREE_FORM_BODY))
    }
}

/// The parameter a `{"$param": name}` splice names, or `None` for anything else.
pub(crate) fn splice_of(value: &Value) -> Option<&str> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object.get("$param").and_then(Value::as_str)
}

// ---------------------------------------------------------------------------------------------
// The embedded catalogue
// ---------------------------------------------------------------------------------------------

/// Documents parsed on demand and kept for the life of the process, keyed by connector id.
///
/// Leaked rather than reference-counted for the same reason `catalog::Operation` is `&'static`: a
/// catalogue is process-lifetime data, and a resolved operation is held by every projected tool.
/// One provider's document is parsed the first time an operation of that provider is resolved, so a
/// host that installs one connector does not pay for fifty-five.
fn cache() -> &'static Mutex<BTreeMap<String, &'static Document>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, &'static Document>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// One provider's canonical document, from the embedded pack.
///
/// `None` when the pack carries no such provider. A document that does not parse is also `None`:
/// the reader has already verified the container's digest and schema version, so a parse failure
/// here is a corrupt build rather than an input a caller can act on, and the callers all report
/// their own refusal naming the operation.
pub fn provider(id: &str) -> Option<&'static Document> {
    let mut cache = cache().lock().expect("the document cache is not poisoned");
    if let Some(document) = cache.get(id) {
        return Some(document);
    }
    let text = catalog_reader::provider(id)?.document();
    let document: &'static Document = Box::leak(Box::new(Document::parse(text).ok()?));
    cache.insert(id.to_owned(), document);
    Some(document)
}

/// One operation's document, by operation id.
pub fn operation(id: &str) -> Option<&'static Operation> {
    let record = catalog_reader::operation(id)?;
    provider(record.provider())?.operation(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_symbol_allocation_reproduces_the_emitters() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("per_page"), "per_page");
        assert_eq!(symbols.allocate("time.start"), "time_start");
        assert_eq!(symbols.allocate("time_start"), "time_start_2");
        assert_eq!(symbols.allocate("$top"), "_top");
        assert_eq!(symbols.allocate("2fa"), "p_2fa");
    }

    #[test]
    fn the_emitters_own_symbols_are_reserved() {
        let mut symbols = Symbols::new();
        assert_eq!(symbols.allocate("url"), "url_2");
        assert_eq!(symbols.allocate("base"), "base_2");
        assert_eq!(symbols.allocate("payload"), "payload_2");
        assert_eq!(symbols.allocate("response"), "response_2");
        assert_eq!(symbols.allocate("content_type"), "content_type_2");
    }

    /// **C-538's ADJACENT 2 trap, guarded from the reader's side** (C-552).
    ///
    /// The document states each parameter's symbol, and the reader honors it. The fixture's declared
    /// `a_b` carries the emitter's shifted symbol `a_b_2` — shifted because a `const`-pinned body
    /// field the document omits reserved `a_b` — and the naive allocation over the declared
    /// parameters alone (which never saw that `const` field) would have produced `a_b`. Honoring the
    /// stored symbol is what sends the request under the right name.
    #[test]
    fn a_stated_symbol_is_honored_over_the_naive_allocation() {
        let text = r#"{
            "connector": "vendor",
            "services": [{"name": "default", "base_url": "https://x"}],
            "operations": [{
                "id": "vendor-thing-create",
                "service": "default",
                "expose": true,
                "effects": ["write", "network"],
                "interaction_shape": "unary",
                "protocol_driver": "http_v1",
                "placement_requirement": "connectors_deployment",
                "implementation_form": "built_in",
                "required_capabilities": ["public_network"],
                "params": [{"name": "a_b", "position": "body", "symbol": "a_b_2"}],
                "request": {"method": "POST", "url": "{base}/things"}
            }]
        }"#;
        let document = Document::parse(text).expect("the fixture parses");
        let operation = document
            .operation("vendor-thing-create")
            .expect("its record");
        assert_eq!(operation.caller_parameters(), ["a_b_2"]);
        assert_eq!(operation.symbol("a_b"), Some("a_b_2"));

        // Proof the seed is not a no-op: the naive allocation over the declared parameter alone would
        // have handed back `a_b`, the very shift the stored symbol exists to preserve.
        assert_eq!(Symbols::new().allocate("a_b"), "a_b");
    }

    /// A document written before C-552 carries no symbol, so the reader falls back to reproducing the
    /// emitter's allocation — the C-537 forward-compat contract, that an additive field's absence
    /// still reads.
    #[test]
    fn a_pre_c552_document_without_symbols_falls_back_to_the_allocation() {
        let text = r#"{
            "connector": "vendor",
            "services": [{"name": "default", "base_url": "https://x"}],
            "operations": [{
                "id": "vendor-thing-list",
                "service": "default",
                "expose": true,
                "effects": ["read", "network"],
                "interaction_shape": "unary",
                "protocol_driver": "http_v1",
                "placement_requirement": "connectors_deployment",
                "implementation_form": "built_in",
                "required_capabilities": ["public_network"],
                "params": [{"name": "time.start", "position": "query"}],
                "request": {"method": "GET", "url": "{base}/things"}
            }]
        }"#;
        let document = Document::parse(text).expect("the fixture parses");
        let operation = document.operation("vendor-thing-list").expect("its record");
        assert_eq!(operation.caller_parameters(), ["time_start"]);
    }

    #[test]
    fn a_sip_session_driver_survives_the_canonical_document() {
        let text = r#"{
            "connector": "voice-provider",
            "services": [{"name": "default", "base_url": "sip:pbx.example.test"}],
            "operations": [{
                "id": "voice-provider-call-establish",
                "service": "default",
                "expose": false,
                "effects": ["write", "network"],
                "interaction_shape": "session_establishment",
                "protocol_driver": "sip_v1",
                "placement_requirement": "connectors_deployment",
                "implementation_form": "built_in",
                "required_capabilities": ["private_network"],
                "params": [],
                "request": {}
            }]
        }"#;
        let document = Document::parse(text).expect("the SIP fixture parses");
        let operation = document
            .operation("voice-provider-call-establish")
            .expect("its record");
        assert_eq!(
            operation.interaction_shape(),
            InteractionShape::SessionEstablishment
        );
        assert_eq!(operation.protocol_driver(), ProtocolDriver::SipV1);
    }

    #[test]
    fn a_shipped_document_parses_into_its_services_and_operations() {
        let document = provider("zendesk").expect("the pack carries zendesk");
        assert_eq!(
            document.base_url("default"),
            Some("https://{subdomain}.zendesk.com")
        );
        let operation = document
            .operation("zendesk-ticket-update")
            .expect("the document carries it");
        let request = operation.request.http().expect("HTTP request");
        assert_eq!(request.method, "PUT");
        assert_eq!(request.url, "{base}/api/v2/tickets/{ticket_id}");
        assert_eq!(operation.endpoint_variables(), ["subdomain"]);
        assert_eq!(operation.endpoint_slots()["subdomain"], Slot::Host);
        assert_eq!(operation.caller_parameters(), ["ticket_id", "ticket"]);
        assert!(operation.caller_path_parameters().contains("ticket_id"));
    }

    #[test]
    fn an_operation_resolves_through_the_embedded_pack_without_naming_its_provider() {
        let operation = operation("zendesk-ticket-show").expect("the pack carries it");
        assert_eq!(operation.service, "default");
        assert!(operation.expose);
    }

    #[test]
    fn an_unknown_provider_or_operation_is_absent_rather_than_a_panic() {
        assert!(provider("no-such-vendor").is_none());
        assert!(operation("no-such-operation").is_none());
    }
}
