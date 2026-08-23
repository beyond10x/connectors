//! Grant and approval enforcement for the hosted invoke path (design 13, S-046).
//!
//! This module replaces the two S-042 fences: instead of refusing every caller-supplied approval
//! reference and every description that is not `ReadOnly` + `NotRequired`, the route now runs the
//! S-044 [`GrantEvaluator`] over every invocation and — where the description demands approval —
//! the S-045 [`ApprovalGate`], which verifies the externally issued record and spends it exactly
//! once. Effect-bearing hosted invocation is reachable, but only through the proofs:
//! [`AdmittedOperation`] is constructed here through `from_decision` alone, against real
//! wall-clock time, so an expired decision refuses at use.
//!
//! # Refusal semantics
//!
//! Straight from the domain model: no Grant store bound is an outage
//! ([`EnforcementRefusal::Unavailable`], 503); an empty store, nothing admitting, a missing or
//! mismatched approval, and a replayed approval are all the same refusal
//! ([`EnforcementRefusal::NotAdmitted`], 403) — the axis is not represented, so the route cannot
//! leak it. Replay stays distinct **internally**: the gate journals it as its own kind.
//!
//! The described read-only path (`ReadOnly` + `NotRequired`) is deliberately unchanged for
//! callers whose Grants cover no effects: evaluation still runs, and an admitting Grant produces
//! the proof, but a refusal falls back to the receiver-policy path that already admitted every
//! hosted read before this story.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use connector_state::{StateError, StateStore};
use domain::{
    AdmittedOperation, ApprovalError, ApprovalGate, ApprovalInvocation, ApprovalOutcome,
    ApprovalRecord, ApprovalRedemption, ConnectionAuthority, GrantAction, GrantDecision,
    GrantEffect, GrantEvaluator, GrantFacts, GrantIdempotency, GrantRefusal, GrantRequest,
    GrantRisk, InitiationPolicy,
};
use protocol::operation::{ApprovalPosture, EffectClass, InvokeRequest, OperationDescription};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::HostedPrincipal;

/// Every externally issued approval record lives under this prefix, keyed by the reference's
/// SHA-256 hex — the reference itself never lands in a state key, mirroring the redemption cells.
const ISSUED_KEY_PREFIX: &str = "approval.issued.";

/// An issued approval record is one bounded cell; reads use this generous bound.
const MAX_ISSUED_CELL_BYTES: usize = 16 * 1024;

/// Enforcement refusal shape. Two-valued like [`GrantRefusal`], and for the same reason: the axis
/// that refused — grant, approval, replay, expiry — is not represented at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EnforcementRefusal {
    /// 403-class: nothing admits this exact invocation.
    NotAdmitted,
    /// 503-class: the Grant or approval authority cannot answer. Damaged or absent authority is
    /// an outage, never a policy statement.
    Unavailable,
}

/// How one hosted invocation may proceed.
pub(super) enum InvokeAdmission {
    /// Described `ReadOnly` + `NotRequired` with no Grant admitting it: the pre-existing
    /// receiver-policy read path, unchanged for callers whose Grants cover no effects.
    ReadPath,
    /// Admission proven: the [`AdmittedOperation`] was constructed through `from_decision`
    /// against real wall-clock time, and dispatch consumes it **by value** (S-047) — an
    /// effect-bearing invocation reaches a backend only through the seam holding this proof,
    /// and the Integrations' own approval-presence checks are deleted in its favour.
    Proven {
        /// The proof itself, buildable only from a [`GrantDecision`] here at admission time.
        /// Boxed because the proof dwarfs the other variants, not to share it: dispatch still
        /// consumes it by value.
        operation: Box<AdmittedOperation>,
        /// Present when the description demanded approval: the one-time proof whose attempted
        /// audit row is already durable. Hand it back through [`HostedAuthority::conclude`]
        /// after dispatch.
        redemption: Option<ApprovalRedemption>,
    },
}

/// The approval redemption journal could not be settled at startup: attempted presentations
/// without a terminal outcome exist, and the store cannot answer or cannot take their
/// settlement rows. Damaged approval authority is an outage, never a policy statement.
#[derive(Debug, thiserror::Error)]
#[error("the approval redemption journal could not be settled at startup")]
pub struct RecoveryError;

/// The Connector-owned enforcement authority the hosted route consults on every invocation:
/// the S-044 Grant evaluator and the S-045 approval gate, both over the deployment's one bound
/// S-041 state store.
#[derive(Clone)]
pub struct HostedAuthority {
    evaluator: Arc<GrantEvaluator>,
    approvals: Option<HostedApprovals>,
}

#[derive(Clone)]
struct HostedApprovals {
    gate: Arc<ApprovalGate>,
    store: Arc<dyn StateStore>,
}

impl HostedAuthority {
    /// The enforcement authority over the deployment's bound state store, accepting approval
    /// records from exactly one `approval_issuer`.
    #[must_use]
    pub fn bound(store: Arc<dyn StateStore>, approval_issuer: impl Into<String>) -> Self {
        Self {
            evaluator: Arc::new(GrantEvaluator::bound(store.clone())),
            approvals: Some(HostedApprovals {
                gate: Arc::new(ApprovalGate::new(store.clone(), approval_issuer)),
                store,
            }),
        }
    }

    /// An authority for a placement that bound no store. Every effect-bearing evaluation is an
    /// honest outage; the described read-only path still serves.
    #[must_use]
    pub fn unbound() -> Self {
        Self {
            evaluator: Arc::new(GrantEvaluator::unbound()),
            approvals: None,
        }
    }

    /// Decide one invocation against the re-described operation.
    pub(super) fn admit_invoke(
        &self,
        principal: &HostedPrincipal,
        invoke: &InvokeRequest,
        description: &OperationDescription,
    ) -> Result<InvokeAdmission, EnforcementRefusal> {
        let read_path = description.effect == EffectClass::ReadOnly
            && description.approval == ApprovalPosture::NotRequired;
        let demanded = description.approval == ApprovalPosture::Required;
        if !demanded && invoke.approval_evidence_ref.is_some() {
            // Evidence nothing demands is never silently ignored and never silently spent.
            return Err(EnforcementRefusal::NotAdmitted);
        }
        let input_digest = canonical_input_digest(&invoke.input);
        let decision = match self.evaluate(principal, invoke, description, &input_digest) {
            Ok(decision) => decision,
            Err(GrantRefusal::Refused | GrantRefusal::Unavailable) if read_path => {
                return Ok(InvokeAdmission::ReadPath);
            }
            Err(GrantRefusal::Refused) => return Err(EnforcementRefusal::NotAdmitted),
            Err(GrantRefusal::Unavailable) => return Err(EnforcementRefusal::Unavailable),
        };
        // Real wall-clock time, never cached or caller-supplied: an expired decision refuses at
        // use, and this constructor is the only way the hosted path forms an admission.
        let operation = AdmittedOperation::from_decision(decision, SystemTime::now())
            .map_err(|_| EnforcementRefusal::NotAdmitted)?;
        let redemption = if demanded {
            Some(self.redeem(principal, invoke, &input_digest)?)
        } else {
            None
        };
        Ok(InvokeAdmission::Proven {
            operation: Box::new(operation),
            redemption,
        })
    }

    /// Run the S-045 startup crash-recovery scan over the approval journal: settle every
    /// attempted presentation that has no outcome row — `aborted` when the redemption never
    /// happened, `indeterminate` when it did and the terminal row is missing. Returns how many
    /// presentations were settled; a placement that bound no store has nothing to settle.
    ///
    /// The hosted composition runs this once before serving. A journal that cannot be settled
    /// is damaged approval authority, and refusing to start is the honest outage.
    ///
    /// # Errors
    ///
    /// [`RecoveryError`] when the journal cannot be read, holds a row this build cannot parse,
    /// or cannot take a settlement row.
    pub fn recover(&self) -> Result<usize, RecoveryError> {
        let Some(approvals) = &self.approvals else {
            return Ok(0);
        };
        approvals
            .gate
            .recover(now_seconds())
            .map(|settled| settled.len())
            .map_err(|_| RecoveryError)
    }

    /// Write the terminal outcome row for a dispatch this authority admitted.
    ///
    /// A journal that cannot take the row leaves the redemption spent; the startup recovery scan
    /// (`ApprovalGate::recover`) settles it as indeterminate, so the error is deliberately not
    /// surfaced onto the already-produced response.
    pub(super) fn conclude(&self, redemption: ApprovalRedemption, completed: bool) {
        let Some(approvals) = &self.approvals else {
            return;
        };
        let outcome = if completed {
            ApprovalOutcome::Completed
        } else {
            ApprovalOutcome::Failed
        };
        let _ = approvals.gate.conclude(redemption, outcome, now_seconds());
    }

    fn evaluate(
        &self,
        principal: &HostedPrincipal,
        invoke: &InvokeRequest,
        description: &OperationDescription,
        input_digest: &str,
    ) -> Result<GrantDecision, GrantRefusal> {
        // The Provider comes from the Connector's own description of the exact Connection the
        // caller invokes over — never from the caller's claims. A Connection the description
        // does not carry admits nothing.
        let provider = description
            .connections
            .iter()
            .find(|connection| connection.connection_ref == invoke.connection_ref)
            .map(|connection| connection.provider.clone())
            .ok_or(GrantRefusal::Refused)?;
        let connection = ConnectionAuthority::new(
            invoke.connection_ref.clone(),
            InitiationPolicy::b10x_only(),
        )
        .map_err(|_| GrantRefusal::Refused)?;
        let request = GrantRequest {
            issuer: principal.issuer.clone(),
            tenant: principal.tenant_id.clone(),
            subject: principal.subject.clone(),
            actor: (principal.actor_subject != principal.subject)
                .then(|| principal.actor_subject.clone()),
            provider,
            connection,
            // The description lease is the only generation-bearing value served at this seam
            // today; when the catalog content generation travels with the description, it
            // replaces this binding.
            catalog_generation: description.description_ref.clone(),
            description_ref: description.description_ref.clone(),
            input_digest: input_digest.to_owned(),
            action: GrantAction::Invoke {
                operation: invoke.operation_ref.clone(),
                facts: undeclared_facts(),
            },
        };
        self.evaluator.evaluate(&request, SystemTime::now())
    }

    fn redeem(
        &self,
        principal: &HostedPrincipal,
        invoke: &InvokeRequest,
        input_digest: &str,
    ) -> Result<ApprovalRedemption, EnforcementRefusal> {
        let Some(approvals) = &self.approvals else {
            return Err(EnforcementRefusal::Unavailable);
        };
        let Some(reference) = invoke.approval_evidence_ref.as_deref() else {
            return Err(EnforcementRefusal::NotAdmitted);
        };
        let record = load_issued(&*approvals.store, reference)?;
        let invocation = ApprovalInvocation {
            subject: principal.subject.clone(),
            operation: invoke.operation_ref.clone(),
            connection: invoke.connection_ref.clone(),
            input_digest: input_digest.to_owned(),
            now_seconds: now_seconds(),
        };
        match approvals.gate.redeem(&record, &invocation) {
            Ok(redemption) => Ok(redemption),
            // Replay renders exactly as every other refusal; the journal already holds its own
            // `replayed` kind, and that is the only place the distinction exists.
            Err(ApprovalError::Refused | ApprovalError::Replayed) => {
                Err(EnforcementRefusal::NotAdmitted)
            }
            Err(ApprovalError::Unavailable) => Err(EnforcementRefusal::Unavailable),
        }
    }
}

/// The declared facts this seam cannot yet read: the served description does not carry the
/// operation's reviewed risk, effect set, or idempotency class, so the route claims the worst on
/// every axis. A selector therefore admits only when it admits everything, and the exact allow
/// exception is the expected admission shape until the declared facts travel with the
/// description. The effect set is [`GrantEffect::ALL`], which is exhaustive by construction —
/// a future effect variant cannot silently weaken this worst-case claim.
fn undeclared_facts() -> GrantFacts {
    GrantFacts {
        risk: GrantRisk::Destructive,
        effects: BTreeSet::from(GrantEffect::ALL),
        idempotency: GrantIdempotency::NonIdempotent,
    }
}

/// The stored shape of one externally issued approval record. The reference is deliberately not
/// a field: it is the presenter's half, and the cell is already keyed by its SHA-256.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IssuedApproval {
    issuer: String,
    subject: String,
    operation: String,
    connection: String,
    input_digest: String,
    expires_at_seconds: u64,
}

/// Store one externally issued approval record where the hosted route resolves presented
/// references.
///
/// This is the bootstrap issuance path, exactly as `GrantSet::write` is for Grants: deployment
/// tooling and tests write records directly until an issuance surface owns the flow. Presenting
/// the reference spends the record through the one-time redemption cell, never by deleting this
/// one.
///
/// # Errors
///
/// [`StateError`] as the backend answers; [`StateError::Invalid`] for a record that cannot
/// serialize.
pub fn issue_approval(store: &dyn StateStore, record: &ApprovalRecord) -> Result<(), StateError> {
    let issued = IssuedApproval {
        issuer: record.issuer.clone(),
        subject: record.subject.clone(),
        operation: record.operation.clone(),
        connection: record.connection.clone(),
        input_digest: record.input_digest.clone(),
        expires_at_seconds: record.expires_at_seconds,
    };
    let body = serde_json::to_vec(&issued).map_err(|_| StateError::Invalid)?;
    store.replace(
        &issued_key(record.reference.as_bytes()),
        &body,
        MAX_ISSUED_CELL_BYTES,
    )
}

fn load_issued(
    store: &dyn StateStore,
    reference: &str,
) -> Result<ApprovalRecord, EnforcementRefusal> {
    let body = match store.read(&issued_key(reference.as_bytes()), MAX_ISSUED_CELL_BYTES) {
        Ok(Some(body)) => body,
        // An approval nobody issued is the same neutral refusal every other one is.
        Ok(None) | Err(StateError::Invalid) => return Err(EnforcementRefusal::NotAdmitted),
        Err(StateError::Unavailable | StateError::Capacity) => {
            return Err(EnforcementRefusal::Unavailable);
        }
    };
    // A damaged record is damaged authority — an outage, never a policy statement.
    let issued: IssuedApproval =
        serde_json::from_slice(&body).map_err(|_| EnforcementRefusal::Unavailable)?;
    Ok(ApprovalRecord {
        reference: reference.to_owned(),
        issuer: issued.issuer,
        subject: issued.subject,
        operation: issued.operation,
        connection: issued.connection,
        input_digest: issued.input_digest,
        expires_at_seconds: issued.expires_at_seconds,
    })
}

fn issued_key(reference: &[u8]) -> String {
    format!("{ISSUED_KEY_PREFIX}{}", sha256_hex(reference))
}

/// SHA-256 hex over the canonical encoding of one invocation input: object members sorted by
/// key at every depth, no insignificant whitespace, array order preserved. Approval issuance
/// computes the same digest over the same encoding, so a semantically identical input matches
/// regardless of member order on the wire.
#[must_use]
pub fn canonical_input_digest(input: &serde_json::Value) -> String {
    let mut canonical = String::new();
    write_canonical(input, &mut canonical);
    sha256_hex(canonical.as_bytes())
}

fn write_canonical(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        serde_json::Value::Object(members) => {
            let mut ordered: Vec<(&String, &serde_json::Value)> = members.iter().collect();
            ordered.sort_by_key(|(key, _)| key.as_str());
            out.push('{');
            for (index, (key, member)) in ordered.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(member, out);
            }
            out.push('}');
        }
        primitive => out.push_str(&primitive.to_string()),
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use connector_state::MemoryState;

    use super::*;

    #[test]
    fn the_canonical_digest_ignores_member_order_and_nothing_else() {
        let a = serde_json::json!({"b": 1, "a": {"y": [1, 2], "x": null}});
        let b = serde_json::json!({"a": {"x": null, "y": [1, 2]}, "b": 1});
        let reordered_array = serde_json::json!({"b": 1, "a": {"y": [2, 1], "x": null}});
        assert_eq!(canonical_input_digest(&a), canonical_input_digest(&b));
        assert_ne!(
            canonical_input_digest(&a),
            canonical_input_digest(&reordered_array)
        );
    }

    #[test]
    fn an_issued_record_round_trips_without_its_reference_in_any_key() {
        let store = MemoryState::new();
        let record = ApprovalRecord {
            reference: "approval:secret-reference".to_owned(),
            issuer: "https://identity.example.test".to_owned(),
            subject: "person:test".to_owned(),
            operation: "test/restart".to_owned(),
            connection: "connection:test".to_owned(),
            input_digest: canonical_input_digest(&serde_json::json!({})),
            expires_at_seconds: u64::MAX,
        };
        issue_approval(&store, &record).expect("issue");
        let loaded = load_issued(&store, "approval:secret-reference").expect("load");
        assert_eq!(loaded, record);
        assert!(store
            .read(
                &issued_key(b"approval:secret-reference"),
                MAX_ISSUED_CELL_BYTES
            )
            .expect("read")
            .is_some());
        assert_eq!(
            load_issued(&store, "approval:never-issued"),
            Err(EnforcementRefusal::NotAdmitted)
        );
    }
}
