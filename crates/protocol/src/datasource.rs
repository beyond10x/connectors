//! Credential-free, read-only datasource protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::operation::OwnerContext;

/// Exact protocol identity. A different value is a different contract.
pub const CONTRACT: &str = "b10x.connector-datasource.v0alpha1";
/// Maximum encoded request frame.
pub const MAX_FRAME_BYTES: usize = 512 * 1024;
/// Maximum encoded response frame.
pub const MAX_RESULT_BYTES: usize = 256 * 1024;
/// Maximum returned discovery or record count.
pub const MAX_RESULTS: u16 = 25;
const MAX_REFERENCE_BYTES: usize = 512;
const MAX_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_KEY_BYTES: usize = 4 * 1024;

/// One strict datasource request frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Protocol identity.
    pub protocol: String,
    /// Caller-generated correlation reference.
    pub request_id: String,
    /// Caller-presented owner context, independently authenticated by the transport.
    pub context: OwnerContext,
    /// Requested datasource method.
    pub request: DatasourceRequest,
}

/// Closed datasource method set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DatasourceRequest {
    /// Search admitted datasource definitions.
    Search(SearchRequest),
    /// Describe one admitted definition and its schemas.
    Describe(DescribeRequest),
    /// List admitted bindings for one definition.
    Bindings(BindingSearchRequest),
    /// Perform one read against an exact binding and description lease.
    Read(ReadRequest),
}

/// Bounded definition search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// Human query matched by the owner.
    pub query: String,
    /// Maximum definitions to return.
    pub limit: u16,
}

/// Exact definition lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    /// Stable datasource identity.
    pub datasource_ref: String,
}

/// Bounded binding search for a definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingSearchRequest {
    /// Stable datasource identity.
    pub datasource_ref: String,
    /// Human query matched against safe binding labels.
    pub query: String,
    /// Maximum bindings to return.
    pub limit: u16,
}

/// One read tied to an exact definition, binding, and description generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    /// Stable datasource identity.
    pub datasource_ref: String,
    /// Exact admitted datasource binding.
    pub binding_ref: String,
    /// Opaque description lease returned by `describe`.
    pub description_ref: String,
    /// Read-only access verb and arguments.
    pub read: DatasourceRead,
}

/// Read-only access verbs in the first datasource contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "verb", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatasourceRead {
    /// Return compact records in owner order.
    List {
        /// Maximum records in this page.
        limit: u16,
        /// Opaque owner cursor from the preceding page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// Return one normalized detail record by its owner-declared key.
    Get {
        /// Owner-declared key validated against `key_schema`.
        key: Value,
    },
}

/// Datasource execution posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    /// Every read observes the owner live; no local index is implied.
    Live,
}

/// Supported read verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadVerb {
    /// Bounded compact listing.
    List,
    /// Exact normalized detail lookup.
    Get,
}

/// Small discovery projection for one datasource definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSummary {
    /// Stable datasource identity.
    pub datasource_ref: String,
    /// Human-readable title.
    pub title: String,
    /// Live or indexed execution posture.
    pub access_mode: AccessMode,
    /// Supported verbs, sorted and unique.
    pub verbs: Vec<ReadVerb>,
}

/// Complete schema and projection declaration for one datasource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourceDescription {
    /// Discovery summary.
    pub summary: DatasourceSummary,
    /// Safe purpose description.
    pub description: String,
    /// JSON Schema for exact get keys.
    pub key_schema: Value,
    /// JSON Schema for compact list values.
    pub compact_schema: Value,
    /// JSON Schema for get/detail values.
    pub detail_schema: Value,
    /// Exact value-projection protocol identity.
    pub projection_protocol: String,
    /// SHA-256 of the complete owner projection declaration.
    pub projection_sha256: String,
    /// Opaque authority and definition generation lease.
    pub description_ref: String,
}

/// One definition bound to an exact Connector Connection and owner scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourceBinding {
    /// Stable definition identity.
    pub datasource_ref: String,
    /// Opaque binding identity.
    pub binding_ref: String,
    /// Connector Connection supplying the owner access.
    pub connection_ref: String,
    /// Safe scope label, such as one admitted namespace.
    pub label: String,
    /// When a caller should read through this binding rather than another of the same datasource.
    ///
    /// Free text the deployment or the operator wrote. One datasource can be bound to several
    /// Connections that are genuinely different actors — a workspace bot, a person, an assistant —
    /// and which one to read through is not answerable from a label alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Authority generation for stale-binding refusal.
    pub generation: u64,
}

/// Compact or detailed record view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordView {
    /// List/search projection.
    Compact,
    /// Exact get projection.
    Detail,
}

/// One normalized datasource record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourceRecord {
    /// Owner-declared stable lookup key.
    pub key: Value,
    /// Compact or detailed projection.
    pub view: RecordView,
    /// Schema-checked normalized value.
    pub value: Value,
}

/// Whether owner bounds omitted related data from this page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// All related values admitted by the read were returned.
    Complete,
    /// At least one declared bound truncated related values.
    Partial,
}

/// Provenance retained after provider envelopes are removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourceProvenance {
    /// Exact binding used for the read.
    pub binding_ref: String,
    /// Projection declaration digest.
    pub projection_sha256: String,
    /// Connector-owned audit reference.
    pub connector_audit_ref: String,
}

/// Standard bounded datasource read result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasourcePage {
    /// Datasource identity.
    pub datasource_ref: String,
    /// Normalized records.
    pub records: Vec<DatasourceRecord>,
    /// Opaque cursor when more records exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Whether related records were truncated.
    pub completeness: Completeness,
    /// Owner observation time in Unix milliseconds.
    pub observed_at_unix_ms: u64,
    /// Read provenance and audit identity.
    pub provenance: DatasourceProvenance,
}

/// Successful datasource response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "result",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DatasourceResult {
    /// Definition search results.
    Search {
        /// Definitions admitted to the caller.
        definitions: Vec<DatasourceSummary>,
    },
    /// Complete definition description.
    Describe(DatasourceDescription),
    /// Admitted bindings.
    Bindings {
        /// Bindings admitted to the caller.
        bindings: Vec<DatasourceBinding>,
    },
    /// List or get read result.
    Read(DatasourcePage),
}

/// Closed datasource failure vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatasourceErrorCode {
    /// Owner or provider is unavailable.
    Unavailable,
    /// Definition, binding, or record does not exist.
    NotFound,
    /// Verified authority does not admit the request.
    NotGranted,
    /// Input does not conform to the owner declaration.
    InvalidInput,
    /// Authority, definition, binding, or description lease changed.
    StaleAuthority,
    /// Cursor expired, was evicted, or belongs to another scope.
    CursorExpired,
    /// Result exceeded a declared bound.
    ResultTooLarge,
    /// Protocol identity or framing is invalid.
    Protocol,
}

/// Typed datasource failure with safe diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, schemars::JsonSchema)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct DatasourceError {
    /// Failure class.
    pub code: DatasourceErrorCode,
    /// Safe bounded detail.
    pub message: String,
    /// Whether a distinct later attempt may succeed without widening authority.
    pub retriable: bool,
}

impl DatasourceError {
    /// Construct one safe typed failure.
    #[must_use]
    pub fn new(code: DatasourceErrorCode, message: impl Into<String>, retriable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retriable,
        }
    }
}

/// Response success/failure discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    /// Successful response.
    Ok,
    /// Typed failure response.
    Error,
}

/// One strict datasource response frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    /// Protocol identity.
    pub protocol: String,
    /// Request correlation reference.
    pub request_id: String,
    /// Success/failure discriminator.
    pub status: ResponseStatus,
    /// Present only for success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<DatasourceResult>,
    /// Present only for failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<DatasourceError>,
}

impl RequestEnvelope {
    /// Validate framing and bounds before owner work.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal for invalid protocol, context, references, or arguments.
    pub fn validate(&self) -> Result<(), DatasourceError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        validate_context(&self.context)?;
        match &self.request {
            DatasourceRequest::Search(request) => validate_search(&request.query, request.limit)?,
            DatasourceRequest::Describe(request) => require_ref(&request.datasource_ref)?,
            DatasourceRequest::Bindings(request) => {
                require_ref(&request.datasource_ref)?;
                validate_search(&request.query, request.limit)?;
            }
            DatasourceRequest::Read(request) => {
                require_ref(&request.datasource_ref)?;
                require_ref(&request.binding_ref)?;
                require_ref(&request.description_ref)?;
                match &request.read {
                    DatasourceRead::List { limit, cursor } => {
                        if *limit == 0
                            || *limit > MAX_RESULTS
                            || cursor
                                .as_deref()
                                .is_some_and(|value| !valid_ref(value, MAX_REFERENCE_BYTES))
                        {
                            return Err(invalid("datasource list bounds are invalid"));
                        }
                    }
                    DatasourceRead::Get { key } => {
                        if serde_json::to_vec(key).map_or(true, |bytes| bytes.len() > MAX_KEY_BYTES)
                        {
                            return Err(invalid("datasource get key is invalid"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl ResponseEnvelope {
    /// Construct a successful response.
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: DatasourceResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    /// Construct a failed response.
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: DatasourceError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    /// Validate response framing and bounds.
    ///
    /// # Errors
    ///
    /// Returns a protocol or result bound refusal.
    pub fn validate(&self) -> Result<(), DatasourceError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_refusal());
        }
        match (self.status, &self.response, &self.error) {
            (ResponseStatus::Ok, Some(result), None) => validate_result(result)?,
            (ResponseStatus::Error, None, Some(error))
                if !error.message.is_empty() && error.message.len() <= 4096 => {}
            _ => return Err(protocol_refusal()),
        }
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_RESULT_BYTES) {
            return Err(DatasourceError::new(
                DatasourceErrorCode::ResultTooLarge,
                "datasource response exceeds the admitted bound",
                false,
            ));
        }
        Ok(())
    }
}

fn validate_result(result: &DatasourceResult) -> Result<(), DatasourceError> {
    match result {
        DatasourceResult::Search { definitions } => {
            if definitions.len() > usize::from(MAX_RESULTS) {
                return Err(protocol_refusal());
            }
            for definition in definitions {
                validate_summary(definition)?;
            }
        }
        DatasourceResult::Describe(description) => {
            validate_summary(&description.summary)?;
            if description.description.is_empty()
                || description.description.len() > 16_384
                || !valid_schema(&description.key_schema)
                || !valid_schema(&description.compact_schema)
                || !valid_schema(&description.detail_schema)
                || !valid_ref(&description.projection_protocol, 128)
                || !valid_sha256(&description.projection_sha256)
                || !valid_ref(&description.description_ref, MAX_REFERENCE_BYTES)
            {
                return Err(protocol_refusal());
            }
        }
        DatasourceResult::Bindings { bindings } => {
            if bindings.len() > usize::from(MAX_RESULTS) {
                return Err(protocol_refusal());
            }
            for binding in bindings {
                if !valid_ref(&binding.datasource_ref, MAX_REFERENCE_BYTES)
                    || !valid_ref(&binding.binding_ref, MAX_REFERENCE_BYTES)
                    || !valid_ref(&binding.connection_ref, MAX_REFERENCE_BYTES)
                    || binding.label.is_empty()
                    || binding.label.len() > 1024
                    || binding.generation == 0
                {
                    return Err(protocol_refusal());
                }
            }
        }
        DatasourceResult::Read(page) => {
            if !valid_ref(&page.datasource_ref, MAX_REFERENCE_BYTES)
                || page.records.len() > usize::from(MAX_RESULTS)
                || page.observed_at_unix_ms == 0
                || page
                    .next_cursor
                    .as_deref()
                    .is_some_and(|value| !valid_ref(value, MAX_REFERENCE_BYTES))
                || !valid_ref(&page.provenance.binding_ref, MAX_REFERENCE_BYTES)
                || !valid_sha256(&page.provenance.projection_sha256)
                || !valid_ref(&page.provenance.connector_audit_ref, MAX_REFERENCE_BYTES)
            {
                return Err(protocol_refusal());
            }
            for record in &page.records {
                if serde_json::to_vec(&record.key).map_or(true, |bytes| bytes.len() > MAX_KEY_BYTES)
                {
                    return Err(protocol_refusal());
                }
            }
        }
    }
    Ok(())
}

fn validate_summary(summary: &DatasourceSummary) -> Result<(), DatasourceError> {
    if !valid_ref(&summary.datasource_ref, MAX_REFERENCE_BYTES)
        || summary.title.is_empty()
        || summary.title.len() > 1024
        || summary.verbs.is_empty()
        || summary.verbs.len() > 8
        || summary
            .verbs
            .iter()
            .enumerate()
            .any(|(index, verb)| summary.verbs[..index].contains(verb))
    {
        return Err(protocol_refusal());
    }
    Ok(())
}

fn validate_context(context: &OwnerContext) -> Result<(), DatasourceError> {
    if !valid_ref(&context.tenant_id, 256)
        || !valid_ref(&context.agent_id, 256)
        || context.agent_revision == 0
        || !valid_ref(&context.authority_snapshot_id, 256)
        || !valid_sha256(&context.authority_snapshot_sha256)
    {
        return Err(DatasourceError::new(
            DatasourceErrorCode::StaleAuthority,
            "owner authority context is invalid",
            false,
        ));
    }
    Ok(())
}

fn validate_search(query: &str, limit: u16) -> Result<(), DatasourceError> {
    if query.len() > 512 || limit == 0 || limit > MAX_RESULTS {
        Err(invalid("datasource search bounds are invalid"))
    } else {
        Ok(())
    }
}

fn require_ref(value: &str) -> Result<(), DatasourceError> {
    if valid_ref(value, MAX_REFERENCE_BYTES) {
        Ok(())
    } else {
        Err(invalid("datasource reference is invalid"))
    }
}

fn valid_schema(value: &Value) -> bool {
    serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= MAX_SCHEMA_BYTES)
        && matches!(value, Value::Bool(_) | Value::Object(_))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn invalid(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::InvalidInput, message, false)
}

fn protocol_refusal() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::Protocol,
        "datasource protocol identity or framing is invalid",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-1".into(),
            agent_id: "agent-1".into(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".into(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn list_is_bounded_and_get_key_is_structured() {
        let mut request = RequestEnvelope {
            protocol: CONTRACT.into(),
            request_id: "request-1".into(),
            context: context(),
            request: DatasourceRequest::Read(ReadRequest {
                datasource_ref: "kubernetes.workloads".into(),
                binding_ref: "binding:kubernetes:latest".into(),
                description_ref: "description:kubernetes:workloads:1".into(),
                read: DatasourceRead::List {
                    limit: MAX_RESULTS,
                    cursor: None,
                },
            }),
        };
        request.validate().unwrap();
        let DatasourceRequest::Read(read) = &mut request.request else {
            unreachable!();
        };
        read.read = DatasourceRead::List {
            limit: MAX_RESULTS + 1,
            cursor: None,
        };
        assert_eq!(
            request.validate().unwrap_err().code,
            DatasourceErrorCode::InvalidInput
        );
    }

    #[test]
    fn response_refuses_ambiguous_success_and_failure() {
        let mut response = ResponseEnvelope::success(
            "request-1",
            DatasourceResult::Search {
                definitions: Vec::new(),
            },
        );
        response.validate().unwrap();
        response.error = Some(DatasourceError::new(
            DatasourceErrorCode::Unavailable,
            "unavailable",
            true,
        ));
        assert_eq!(
            response.validate().unwrap_err().code,
            DatasourceErrorCode::Protocol
        );
    }
}
