use super::{Failure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Trusted host evidence, never unqualified SystemTime or request input. The
/// source must establish containment of actual time, including after restart.
#[derive(Clone, Copy, Debug)]
pub struct ClockInterval {
    pub lower_unix_ms: i64,
    pub upper_unix_ms: i64,
}
pub trait Clock: Send + Sync {
    /// A bounded local observation, without provider/network I/O or a blocking
    /// synchronization operation while the metadata transaction is held.
    fn now(&self) -> Result<ClockInterval>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Namespace {
    pub receiver_instance: String,
    pub tenant: Option<String>,
    pub realm: Option<String>,
    pub caller: String,
    pub executor: Option<String>,
    pub origin: Origin,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Direct,
    Federated { gateway_instance: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRef {
    pub instance: String,
    pub adapter: String,
    pub operation: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub gateway_instance: String,
    pub route_id: String,
    pub route_revision: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub operation: OperationRef,
    pub connection_ref: String,
    pub connection_revision: String,
    pub contract_ref: String,
    pub profile: String,
    pub descriptor_revision: String,
    pub configuration_revision: String,
    pub canonicalization_version: String,
    pub input_digest: String,
    pub route: Option<Route>,
}

/// Coordinates come from current trusted host resolution. This storage value
/// has no public request codec and grants no caller or provider authority.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub namespace: Namespace,
    pub fingerprint: Fingerprint,
    pub caller_key: Option<String>,
    pub request_id: String,
    pub approval: Approval,
}
#[derive(Clone, Debug)]
pub enum Approval {
    NotRequired,
    Required { reference: String },
    EventClaim { reference: String },
}
impl Approval {
    pub(super) fn coordinates(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::NotRequired => ("not_required", None),
            Self::Required { reference } => ("required", Some(reference)),
            Self::EventClaim { reference } => ("event_claim", Some(reference)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptRef {
    pub authority: Uuid,
    pub attempt_id: Uuid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Prepared,
    Dispatching,
    Aborted,
    Completed,
    Failed,
    Indeterminate,
}
impl State {
    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "dispatching" => Ok(Self::Dispatching),
            "aborted" => Ok(Self::Aborted),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "indeterminate" => Ok(Self::Indeterminate),
            _ => Err(Failure::MetadataUnavailable),
        }
    }
    pub(super) fn terminal(self) -> bool {
        !matches!(self, Self::Prepared | Self::Dispatching)
    }
}

/// Private observation. Host admission must precede disclosure and public
/// delivery; reading it cannot reconstruct a preparation or dispatch handle.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub reference: AttemptRef,
    pub reservation_id: Option<Uuid>,
    pub request_id: String,
    pub state: State,
    pub result: Option<Value>,
    pub settled_at_ms: Option<i64>,
    pub replay_expires_at_ms: Option<i64>,
}
pub enum Preparation {
    Prepared(Prepared),
    Existing(Observation),
}
/// Not Clone, serializable, or reconstructible from persisted observation.
pub struct Prepared {
    pub(super) reference: AttemptRef,
    pub(super) nonce: Uuid,
    pub(super) process: u32,
    pub(super) binding: super::GateBinding,
}
impl Prepared {
    pub fn reference(&self) -> AttemptRef {
        self.reference
    }
}
/// Definite ledger CAS acknowledgement only. The coordinator must still own
/// current approval, audit and connection-bound capabilities before any send.
/// No read/recovery API creates this receipt; consuming it cannot be retried.
pub struct GateWinner {
    pub(super) reference: AttemptRef,
    pub(super) process: u32,
}
impl GateWinner {
    pub fn reference(&self) -> AttemptRef {
        self.reference
    }
    pub fn consume(self) -> Result<AttemptRef> {
        if self.process != std::process::id() {
            return Err(Failure::Conflict);
        }
        Ok(self.reference)
    }
}

/// Safe, bounded host-selected payload, already projected by the native
/// operation. Native evidence, not this enum or an HTTP status, proves effect.
pub enum Outcome {
    Applied(Value),
    Refused(Value),
    Unknown(Value),
}

#[derive(Clone, Copy)]
pub struct Limits {
    pub attempts_per_instance: u32,
    pub result_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            attempts_per_instance: 10_000,
            result_bytes: 262_144,
        }
    }
}

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        Err(Failure::InvalidInput)
    } else {
        Ok(())
    }
}
impl Candidate {
    pub(super) fn encode(&self) -> Result<(Option<String>, String)> {
        let n = &self.namespace;
        let f = &self.fingerprint;
        for value in [
            &n.receiver_instance,
            &n.caller,
            &f.operation.instance,
            &f.operation.adapter,
            &f.operation.operation,
            &f.connection_ref,
            &f.connection_revision,
            &f.contract_ref,
            &f.profile,
            &f.descriptor_revision,
            &f.configuration_revision,
            &self.request_id,
        ] {
            identifier(value)?;
        }
        for value in [&n.tenant, &n.realm, &n.executor].into_iter().flatten() {
            identifier(value)?;
        }
        if f.operation.instance != n.receiver_instance
            || f.canonicalization_version != "adapter-v1-canonical-json"
            || f.input_digest.len() != 64
            || !f
                .input_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Failure::InvalidInput);
        }
        if let Some(reference) = self.approval.coordinates().1 {
            identifier(reference)?;
        }
        let (origin, authority) = match (&n.origin, &f.route) {
            (Origin::Direct, None) => ("direct", &n.receiver_instance),
            (Origin::Federated { gateway_instance }, Some(route))
                if gateway_instance == &route.gateway_instance =>
            {
                identifier(gateway_instance)?;
                identifier(&route.route_id)?;
                identifier(&route.route_revision)?;
                ("federated", gateway_instance)
            }
            _ => return Err(Failure::InvalidInput),
        };
        let key = self
            .caller_key
            .as_ref()
            .map(|key| {
                // Opaque bytes: no trim, control filtering or Unicode normalization.
                if key.is_empty() || key.len() > 256 {
                    return Err(Failure::InvalidInput);
                }
                serde_json::to_string(&serde_json::json!([
                    "mutation-key/v1",
                    n.receiver_instance,
                    n.tenant,
                    n.realm,
                    n.caller,
                    n.executor,
                    origin,
                    authority,
                    key
                ]))
                .map_err(|_| Failure::InvalidInput)
            })
            .transpose()?;
        let operation = serde_json::json!([
            "connectors.operation/v1",
            f.operation.instance,
            f.operation.adapter,
            f.operation.operation
        ]);
        let fingerprint = serde_json::to_string(&serde_json::json!([
            "mutation-request/v2",
            operation,
            f.connection_ref,
            f.connection_revision,
            f.contract_ref,
            f.profile,
            f.descriptor_revision,
            f.configuration_revision,
            f.canonicalization_version,
            f.input_digest,
            f.route
        ]))
        .map_err(|_| Failure::InvalidInput)?;
        if key.as_ref().is_some_and(|v| v.len() > 4096) || fingerprint.len() > 16_384 {
            return Err(Failure::InvalidInput);
        }
        Ok((key, fingerprint))
    }
}
