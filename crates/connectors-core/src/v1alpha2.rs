//! The first v1alpha2 invoke binding, `POST /v1alpha2/invoke`.
//!
//! Field for field these are the `connectors.service_wire` declarations in
//! `ess/domains/service_wire.yaml`: `InvokeRequest`, `InvokeResponse`, `Error`,
//! `ResponseStatus`, `MutationObservation`, `AttemptReference`, `AuditStatus`
//! and `SourceAudit`. ESS generates their JSON Schema, not this code. Every
//! struct is closed, a member declared `null_when_absent` is required and
//! nullable, and one declared `omitted_when_absent` is absent rather than null.
//!
//! The § 5 rules a JSON Schema validator does not evaluate are enforced by
//! every reader of [`InvokeResponse`] ([`InvokeResponse::section5_rules`]);
//! [`InvokeResponse::decode`] and [`InvokeResponse::encode`] add the first
//! binding's narrowing ([`InvokeResponse::first_binding_rules`]). The rules of `contracts/service/compatibility.md`
//! § 2.1 that select a request's refusal code are [`InvokeRequest::decode`]'s.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The envelope version this binding's route admits.
pub const VERSION: &str = "v1alpha2";

/// Largest safe `Error.message`, in UTF-8 bytes (compatibility § 5).
pub const MESSAGE_LIMIT: usize = 512;

/// `connectors.service_wire.Version`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Version {
    #[serde(rename = "v1alpha1")]
    V1alpha1,
    #[serde(rename = "v1alpha2")]
    V1alpha2,
}

/// `connectors.service_wire.ErrorCode`: the closed extended set.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    Unsupported,
    Unauthorized,
    Forbidden,
    NotFound,
    StaleDescription,
    StaleCursor,
    RateLimited,
    Unavailable,
    Capacity,
    Timeout,
    UpstreamProtocol,
    Internal,
    ApprovalRequired,
    ApprovalRefused,
    ApprovalReplayed,
    IdempotencyConflict,
    OutcomeUnknown,
    NotGranted,
    StaleAuthority,
    ConnectionNotReady,
    RouteUnavailable,
    RouteRefused,
    InsufficientScope,
    SessionNotReady,
    SessionLost,
    OfferExpired,
    OfferRejected,
    LeaseExpired,
    Revoked,
}

impl ErrorCode {
    /// One of the 13 [`BaseErrorCode`]s.
    pub fn is_base(self) -> bool {
        matches!(
            self,
            Self::InvalidInput
                | Self::Unsupported
                | Self::Unauthorized
                | Self::Forbidden
                | Self::NotFound
                | Self::StaleDescription
                | Self::StaleCursor
                | Self::RateLimited
                | Self::Unavailable
                | Self::Capacity
                | Self::Timeout
                | Self::UpstreamProtocol
                | Self::Internal
        )
    }
}

/// `connectors.service_wire.BaseErrorCode`: the 13 v1alpha1 codes.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaseErrorCode {
    InvalidInput,
    Unsupported,
    Unauthorized,
    Forbidden,
    NotFound,
    StaleDescription,
    StaleCursor,
    RateLimited,
    Unavailable,
    Capacity,
    Timeout,
    UpstreamProtocol,
    Internal,
}

/// `connectors.service_wire.CauseStage`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CauseStage {
    Preflight,
    AttemptStore,
    Approval,
    Dispatch,
    Response,
    Observation,
    Audit,
}

/// `connectors.service_wire.DiagnosticCause`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticCause {
    pub code: BaseErrorCode,
    pub stage: CauseStage,
}

/// `connectors.mutations.AttemptId`: a UUID in its hyphenated text form,
/// kept exactly as written.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub struct AttemptId(String);

impl AttemptId {
    /// The identifier when `text` is a hyphenated UUID (8-4-4-4-12 hex digits).
    pub fn parse(text: &str) -> Option<Self> {
        let groups: Vec<&str> = text.split('-').collect();
        let shaped = groups.len() == 5
            && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, len)| {
                group.len() == len && group.bytes().all(|b| b.is_ascii_hexdigit())
            });
        shaped.then(|| Self(text.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AttemptId {
    type Error = Violation;
    fn try_from(text: String) -> std::result::Result<Self, Violation> {
        Self::parse(&text).ok_or(Violation("attempt id is not a UUID"))
    }
}

impl From<AttemptId> for String {
    fn from(id: AttemptId) -> Self {
        id.0
    }
}

/// `connectors.service_wire.AttemptReference`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttemptReference {
    pub instance: String,
    pub id: AttemptId,
}

/// `connectors.mutations.EffectKnowledge`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKnowledge {
    NotAttempted,
    Refused,
    Applied,
    Unknown,
}

/// `connectors.service_wire.MutationObservation`: all five members required;
/// `attempt`, `original_request_id` and `cause` are null, never omitted.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MutationObservation {
    pub classification: EffectKnowledge,
    #[serde(deserialize_with = "nullable")]
    pub attempt: Option<AttemptReference>,
    #[serde(deserialize_with = "nullable")]
    pub original_request_id: Option<String>,
    pub replayed: bool,
    #[serde(deserialize_with = "nullable")]
    pub cause: Option<DiagnosticCause>,
}

/// `connectors.service_wire.AuditStatus`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Complete,
    Incomplete,
    Unavailable,
    NotRequired,
}

/// `connectors.service_wire.SourceAudit`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAudit {
    pub instance: String,
    #[serde(deserialize_with = "nullable")]
    pub audit_ref: Option<String>,
    pub audit_status: AuditStatus,
}

/// `connectors.service_wire.Error`: `{code, message, retry_after_seconds?}`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    #[serde(
        default,
        deserialize_with = "omittable",
        skip_serializing_if = "Option::is_none"
    )]
    pub retry_after_seconds: Option<u64>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_after_seconds: None,
        }
    }
    /// The bounds the declaration leaves to the codec: a message of at most
    /// [`MESSAGE_LIMIT`] UTF-8 bytes and a positive retry value.
    pub fn check(&self) -> std::result::Result<(), Violation> {
        if self.message.len() > MESSAGE_LIMIT {
            return Err(Violation("error message exceeds 512 bytes"));
        }
        if self.retry_after_seconds == Some(0) {
            return Err(Violation("retry_after_seconds is not positive"));
        }
        Ok(())
    }
}

/// `connectors.service_wire.ResponseStatus`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Error,
}

/// `connectors.service_wire.InvokeRequest`: the five v1alpha1 members, closed.
/// `version` is text so a wrong version decodes and is answered `unsupported`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    pub version: String,
    pub request_id: String,
    pub operation: String,
    pub revision: String,
    pub input: Value,
}

/// The § 4 invocation members the first binding refuses with `unsupported`.
const UNSUPPORTED_MEMBERS: [&str; 4] = ["connection", "idempotency_key", "approval", "executor"];
const REQUEST_MEMBERS: [&str; 5] = ["version", "request_id", "operation", "revision", "input"];

/// A request this binding answers without dispatch: the code, and the
/// request ID the answer echoes (`None` before a valid envelope exists).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub request_id: Option<String>,
    pub error: Error,
}

impl Refusal {
    /// The § 5 Response for a refusal before the audit anchor: no record was
    /// written, so `audit_ref` is null and `audit_status` is `unavailable`.
    pub fn into_response(self) -> InvokeResponse {
        InvokeResponse {
            version: Version::V1alpha2,
            request_id: self.request_id,
            status: ResponseStatus::Error,
            result: None,
            error: Some(self.error),
            audit_ref: None,
            audit_status: AuditStatus::Unavailable,
            source_audit: None,
            mutation: None,
        }
    }
}

impl InvokeRequest {
    /// Decode a request body on `/v1alpha2/invoke` (compatibility § 2, § 2.1).
    ///
    /// Malformed or ambiguous JSON, a duplicate member at any depth, a missing
    /// or mistyped member and any unknown member other than the four § 4
    /// members are `invalid_input`. A § 4 member (`connection`,
    /// `idempotency_key`, `approval`, `executor`) is `unsupported`. Neither is
    /// a valid envelope, so neither echoes a request ID. A valid envelope whose
    /// `version` is not [`VERSION`] is `unsupported` with its request ID.
    pub fn decode(bytes: &[u8]) -> std::result::Result<Self, Refusal> {
        let refuse = |request_id, code, message: &str| Refusal {
            request_id,
            error: Error::new(code, message),
        };
        let invalid = || refuse(None, ErrorCode::InvalidInput, "invalid invocation envelope");
        let value: Value = crate::read_json(bytes).map_err(|_| invalid())?;
        let Value::Object(members) = &value else {
            return Err(invalid());
        };
        let unknown: Vec<&str> = members
            .keys()
            .map(String::as_str)
            .filter(|k| !REQUEST_MEMBERS.contains(k))
            .collect();
        if unknown.iter().any(|k| !UNSUPPORTED_MEMBERS.contains(k)) {
            return Err(invalid());
        }
        if !unknown.is_empty() {
            return Err(refuse(
                None,
                ErrorCode::Unsupported,
                "invocation member is not supported on this binding",
            ));
        }
        let request: Self = serde_json::from_value(value).map_err(|_| invalid())?;
        if request.version != VERSION {
            return Err(refuse(
                Some(request.request_id),
                ErrorCode::Unsupported,
                "unsupported request version",
            ));
        }
        Ok(request)
    }

    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("an invocation envelope is serializable")
    }
}

/// `connectors.service_wire.InvokeResponse`, the § 5 application Response.
///
/// `request_id` and `audit_ref` are required and nullable; `result`, `error`,
/// `source_audit` and `mutation` are omitted when absent. `result` is
/// `Some(Value::Null)` for a success whose result is JSON null. Every serde reader
/// refuses a value [`section5_rules`](Self::section5_rules) refuses; [`decode`](Self::decode)
/// and [`encode`](Self::encode) refuse what [`check`](Self::check) refuses.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "ResponseWire")]
pub struct InvokeResponse {
    pub version: Version,
    pub request_id: Option<String>,
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    pub audit_ref: Option<String>,
    pub audit_status: AuditStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_audit: Option<SourceAudit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation: Option<MutationObservation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseWire {
    version: Version,
    #[serde(deserialize_with = "nullable")]
    request_id: Option<String>,
    status: ResponseStatus,
    #[serde(default, deserialize_with = "omittable")]
    result: Option<Value>,
    #[serde(default, deserialize_with = "omittable")]
    error: Option<Error>,
    #[serde(deserialize_with = "nullable")]
    audit_ref: Option<String>,
    audit_status: AuditStatus,
    #[serde(default, deserialize_with = "omittable")]
    source_audit: Option<SourceAudit>,
    #[serde(default, deserialize_with = "omittable")]
    mutation: Option<MutationObservation>,
}

impl TryFrom<ResponseWire> for InvokeResponse {
    type Error = Violation;
    fn try_from(wire: ResponseWire) -> std::result::Result<Self, Violation> {
        let response = Self {
            version: wire.version,
            request_id: wire.request_id,
            status: wire.status,
            result: wire.result,
            error: wire.error,
            audit_ref: wire.audit_ref,
            audit_status: wire.audit_status,
            source_audit: wire.source_audit,
            mutation: wire.mutation,
        };
        response.section5_rules()?;
        Ok(response)
    }
}

fn audit_consistent(
    status: AuditStatus,
    reference: &Option<String>,
) -> std::result::Result<(), Violation> {
    let needs_ref = matches!(status, AuditStatus::Complete | AuditStatus::Incomplete);
    if needs_ref != reference.is_some() {
        return Err(Violation("audit_ref does not match audit_status"));
    }
    Ok(())
}

impl InvokeResponse {
    /// The rules of compatibility § 5 that the generated schema carries only
    /// as `x-ess-invariants` annotations, and the codec obligations
    /// `service_wire.yaml` names:
    ///
    /// 1. `version` is `v1alpha2`;
    /// 2. `success` carries `result` and no `error`; `error` carries `error`
    ///    and no `result`;
    /// 3. `complete`/`incomplete` carry a non-null `audit_ref`;
    ///    `unavailable`/`not_required` carry null; the same for `source_audit`;
    /// 4. `success` with `mutation` is classified `applied`;
    /// 5. `Error` bounds ([`Error::check`]);
    /// 6. `attempt` and `original_request_id` are both null or both set,
    ///    `replayed: true` names both, and `unknown` answers `outcome_unknown`.
    ///
    /// Then the narrowing of the first binding, [`first_binding_rules`](Self::first_binding_rules).
    /// [`decode`](Self::decode) and [`encode`](Self::encode) apply both; a serde reader
    /// applies [`section5_rules`](Self::section5_rules) alone.
    pub fn check(&self) -> std::result::Result<(), Violation> {
        self.section5_rules()?;
        self.first_binding_rules()
    }

    /// The generic compatibility § 5 rules listed on [`check`](Self::check).
    pub fn section5_rules(&self) -> std::result::Result<(), Violation> {
        if self.version != Version::V1alpha2 {
            return Err(Violation("response version is not v1alpha2"));
        }
        let shaped = match self.status {
            ResponseStatus::Success => self.result.is_some() && self.error.is_none(),
            ResponseStatus::Error => self.error.is_some() && self.result.is_none(),
        };
        if !shaped {
            return Err(Violation(
                "status does not carry exactly its result or error",
            ));
        }
        audit_consistent(self.audit_status, &self.audit_ref)?;
        if let Some(source) = &self.source_audit {
            audit_consistent(source.audit_status, &source.audit_ref)?;
        }
        if let Some(error) = &self.error {
            error.check()?;
        }
        if let Some(mutation) = &self.mutation {
            if self.status == ResponseStatus::Success
                && mutation.classification != EffectKnowledge::Applied
            {
                return Err(Violation("success with mutation is not applied"));
            }
            if mutation.attempt.is_some() != mutation.original_request_id.is_some() {
                return Err(Violation("attempt and original_request_id disagree"));
            }
            if mutation.replayed && mutation.attempt.is_none() {
                return Err(Violation("replayed names no original attempt"));
            }
            if mutation.classification == EffectKnowledge::Unknown
                && self.error.as_ref().map(|e| e.code) != Some(ErrorCode::OutcomeUnknown)
            {
                return Err(Violation("unknown effect without outcome_unknown"));
            }
        }
        Ok(())
    }

    /// How the first binding, `POST /v1alpha2/invoke`, narrows § 5
    /// (compatibility § 2.1). A later binding widens these in this one place.
    ///
    /// - `source_audit` is always omitted;
    /// - `audit_status: not_required` is never answered;
    /// - `mutation.replayed` is false, and `mutation.original_request_id`,
    ///   when set, is this response's `request_id`;
    /// - `error.code` is one of the 13 base codes or `outcome_unknown`
    ///   (`cause.code` is a [`BaseErrorCode`] by declaration).
    pub fn first_binding_rules(&self) -> std::result::Result<(), Violation> {
        if self.source_audit.is_some() {
            return Err(Violation("source_audit is not answered on this binding"));
        }
        if self.audit_status == AuditStatus::NotRequired {
            return Err(Violation("not_required is not answered on this binding"));
        }
        if let Some(mutation) = &self.mutation {
            if mutation.replayed {
                return Err(Violation("this binding does not replay"));
            }
            if mutation
                .original_request_id
                .as_ref()
                .is_some_and(|original| Some(original) != self.request_id.as_ref())
            {
                return Err(Violation("original_request_id is not this request's ID"));
            }
        }
        if let Some(error) = &self.error
            && !(error.code.is_base() || error.code == ErrorCode::OutcomeUnknown)
        {
            return Err(Violation("error code is not emitted on this binding"));
        }
        Ok(())
    }

    /// Strictly decode a Response: duplicate members refused at every depth,
    /// unknown members refused, then [`check`](Self::check). A refusal is the
    /// reader's `upstream_protocol`.
    pub fn decode(bytes: &[u8]) -> std::result::Result<Self, Error> {
        let refused = || {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "response is not a valid v1alpha2 invoke envelope",
            )
        };
        let response: Self = crate::read_json(bytes).map_err(|_| refused())?;
        response.first_binding_rules().map_err(|_| refused())?;
        Ok(response)
    }

    /// Encode a Response that passes [`check`](Self::check).
    pub fn encode(&self) -> std::result::Result<Vec<u8>, Violation> {
        self.check()?;
        Ok(serde_json::to_vec(self).expect("an invoke response is serializable"))
    }
}

/// A rule of the v1alpha2 envelope that a value breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Violation(pub &'static str);

/// A required member whose value may be null.
fn nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

/// An optional member that is absent, never null; with `#[serde(default)]`.
fn omittable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
