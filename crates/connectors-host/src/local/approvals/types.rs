use super::{Failure, Result};
use crate::local::mutations::{Candidate, Origin as MutationOrigin, Route};
use connectors_sdk::Secret;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub instance: String,
    pub operation: String,
    pub connection: String,
    pub connection_revision: String,
    pub contract: String,
    pub profile: String,
    pub descriptor_revision: String,
    pub configuration_revision: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub tenant: Option<String>,
    pub realm: Option<String>,
    pub caller: String,
    pub executor: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub id: String,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Executor {
    pub agent: String,
    pub revision: String,
    pub authority_snapshot: Snapshot,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub scope: Scope,
    pub current_authority: Option<Snapshot>,
    pub executor: Option<Executor>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    Direct,
    Federated,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub kind: OriginKind,
    pub authority_ref: String,
}
/// A complete resolved value, not an authenticated request or admission grant.
/// Optional coordinates serialize as mandatory explicit nulls in proof bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub format: String,
    pub target: Target,
    pub authority: Authority,
    pub origin: Origin,
    pub route: Option<Route>,
    pub canonicalization: String,
    pub input_sha256: String,
    pub approval_mode: String,
}

/// Trusted configuration supplied only by current consumer admission. A token
/// never supplies or discovers this value. No private key is held here.
#[derive(Clone)]
pub struct ConfiguredApprovalKey {
    pub issuer: String,
    pub audience: String,
    pub kid: String,
    pub public_key: String,
    pub not_before_unix_ms: i64,
    pub not_after_unix_ms: i64,
    pub revoked: bool,
}
pub trait CurrentAdmission {
    fn key(&self) -> &ConfiguredApprovalKey;
}
/// Guard implementations must serialize current caller/realm/executor authority
/// and key changes through their drop, including the spend commit/acknowledgement.
/// Admission is bounded local work: no provider I/O while holding metadata locks.
pub trait ReceiverPolicy {
    type Guard<'a>: CurrentAdmission
    where
        Self: 'a;
    fn admit<'a>(&'a self, subject: &Subject, kid: &str) -> Result<Self::Guard<'a>>;
}
/// Separate from receiver admission: the issuer must authorize this exact
/// subject under its own issuance/human policy and hold that decision to return.
pub trait IssuancePolicy {
    type Guard<'a>: CurrentAdmission
    where
        Self: 'a;
    fn authorize<'a>(&'a self, subject: &Subject, kid: &str) -> Result<Self::Guard<'a>>;
}

/// Protected compact proof bytes; no Debug, Clone or serialization implementation.
pub struct Evidence {
    reference: String,
    compact: Secret,
}
impl Evidence {
    pub fn from_protected(reference: String, compact: Secret) -> Result<Self> {
        digest(&reference)?;
        if compact.0.is_empty() || compact.0.len() > 18 * 1024 {
            return Err(Failure::Refused);
        }
        Ok(Self { reference, compact })
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
    pub fn bytes(&self) -> &[u8] {
        &self.compact.0
    }
}
/// Establishes only initial verification and exact preparation association.
/// It is neither a lease on current policy/time nor a spend receipt.
pub struct VerifiedApproval {
    pub(super) binding: ProofBinding,
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::local) struct ProofBinding {
    pub subject: Subject,
    pub issuer: String,
    pub reference: String,
    pub proof_sha256: [u8; 32],
}
impl VerifiedApproval {
    pub(in crate::local) fn for_candidate(&self, candidate: &Candidate) -> Result<ProofBinding> {
        let s = &self.binding.subject;
        let f = &candidate.fingerprint;
        let n = &candidate.namespace;
        let origin = match &n.origin {
            MutationOrigin::Direct => Origin {
                kind: OriginKind::Direct,
                authority_ref: n.receiver_instance.clone(),
            },
            MutationOrigin::Federated { gateway_instance } => Origin {
                kind: OriginKind::Federated,
                authority_ref: gateway_instance.clone(),
            },
        };
        if !matches!(&candidate.approval, crate::local::mutations::Approval::Required { reference } if reference == &self.binding.reference)
            || s.target
                != (Target {
                    instance: f.operation.instance.clone(),
                    operation: f.operation.operation.clone(),
                    connection: f.connection_ref.clone(),
                    connection_revision: f.connection_revision.clone(),
                    contract: f.contract_ref.clone(),
                    profile: f.profile.clone(),
                    descriptor_revision: f.descriptor_revision.clone(),
                    configuration_revision: f.configuration_revision.clone(),
                })
            || s.authority.scope
                != (Scope {
                    tenant: n.tenant.clone(),
                    realm: n.realm.clone(),
                    caller: n.caller.clone(),
                    executor: n.executor.clone(),
                })
            || s.origin != origin
            || s.route != f.route
            || s.canonicalization != f.canonicalization_version
            || s.input_sha256 != f.input_digest
        {
            return Err(Failure::Refused);
        }
        Ok(self.binding.clone())
    }
}
pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(Failure::Refused);
    }
    Ok(())
}
pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Failure::Refused);
    }
    Ok(())
}
impl Subject {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let t = &self.target;
        if self.format != "connectors.approval-subject/v1"
            || self.canonicalization != "adapter-v1-canonical-json"
            || self.approval_mode != "required"
        {
            return Err(Failure::Refused);
        }
        for v in [
            &t.instance,
            &t.operation,
            &t.connection,
            &t.connection_revision,
            &t.contract,
            &t.profile,
            &t.descriptor_revision,
            &t.configuration_revision,
            &self.authority.scope.caller,
            &self.origin.authority_ref,
        ] {
            identifier(v)?;
        }
        for v in [
            &self.authority.scope.tenant,
            &self.authority.scope.realm,
            &self.authority.scope.executor,
        ]
        .into_iter()
        .flatten()
        {
            identifier(v)?;
        }
        digest(&self.input_sha256)?;
        if let Some(s) = &self.authority.current_authority {
            identifier(&s.id)?;
            digest(&s.sha256)?;
        }
        match (&self.authority.scope.executor, &self.authority.executor) {
            (None, None) => (),
            (Some(agent), Some(e)) if agent == &e.agent => {
                identifier(&e.revision)?;
                identifier(&e.authority_snapshot.id)?;
                digest(&e.authority_snapshot.sha256)?;
            }
            _ => return Err(Failure::Refused),
        }
        match (&self.origin.kind, &self.route) {
            (OriginKind::Direct, None) if self.origin.authority_ref == t.instance => (),
            (OriginKind::Federated, Some(r)) if self.origin.authority_ref == r.gateway_instance => {
                identifier(&r.route_id)?;
                identifier(&r.route_revision)?;
            }
            _ => return Err(Failure::Refused),
        }
        let bytes = canonical(self)?;
        if bytes.len() > 8192 {
            return Err(Failure::Refused);
        }
        Ok(bytes)
    }
}
pub(super) fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let value = serde_json::to_value(value).map_err(|_| Failure::Refused)?;
    Ok(connectors_core::canonical(&value))
}
