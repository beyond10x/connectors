//! Grant evaluation: a decision is **evaluated, not asserted** (S-044).
//!
//! [`GrantEvaluator`] loads the tenant's revisioned Grants through the S-041 state port and
//! applies the declared selector semantics — risk ceiling, effects subset, idempotency class,
//! explicit allow/deny exceptions with **deny beats allow beats predicate**, inbound events as
//! closed sets. Its only success output is a [`GrantDecision`]: private fields, no public
//! constructor, buildable only inside this module, so holding one *is* the evidence that
//! evaluation ran here.
//!
//! # Refusal semantics
//!
//! Straight from the domain model, fail closed:
//!
//! - **no store bound is an outage** ([`GrantRefusal::Unavailable`], 503-class) — a placement
//!   that lost its Grant authority must not quietly refuse as if policy had spoken;
//! - **an empty store, or nothing admitting, is a refusal** ([`GrantRefusal::Refused`],
//!   403-class);
//! - **no refusal ever names the axis that refused.** Risk, effects, idempotency, exception,
//!   event set, expiry and an absent tenant all produce the same value with the same text —
//!   anything finer would be a policy-enumeration oracle a probing caller could walk.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use connector_state::StateStore;

use crate::grant::{self, GrantFacts};
use crate::ConnectionAuthority;

/// How long a decision stands before it refuses at use. Redemption and dispatch follow
/// evaluation immediately; a decision that can sit around is a decision that can be replayed
/// into a different context.
const DEFAULT_DECISION_TTL: Duration = Duration::from_secs(60);

static DECISION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Refusal shape of Grant evaluation.
///
/// Deliberately two-valued. The axis that refused is not merely omitted from the text — it is
/// not represented at all, so no future serializer, logger, or error mapper can leak it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GrantRefusal {
    /// 503-class: the Grant authority cannot answer — no store is bound, the backend is
    /// unreachable, or the stored record is unreadable. Damaged authority is an outage, never
    /// a policy statement.
    #[error("grant authority is unavailable")]
    Unavailable,
    /// 403-class: the store is empty or nothing admits this exact request.
    #[error("not granted")]
    Refused,
}

/// What the caller asks the evaluator to decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantAction {
    /// One outbound invocation of one declared operation, with its declared facts read from
    /// the reviewed description — never derived here.
    Invoke {
        operation: String,
        facts: GrantFacts,
    },
    /// One inbound provider event. Admitted only by a grant's closed event set.
    InboundEvent { event: String },
}

/// The exact request under evaluation. Everything here is caller-supplied *context* — the
/// output proof is what makes it trustworthy downstream, not the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRequest {
    /// Identity issuer that authenticated the caller.
    pub issuer: String,
    /// Tenant whose Grants decide.
    pub tenant: String,
    /// The authenticated principal (`sub`).
    pub subject: String,
    /// The acting party (`act`) when the subject is being acted for; `None` otherwise.
    pub actor: Option<String>,
    /// Target Provider of the operation or event.
    pub provider: String,
    /// The already-admitted Connection this request would exercise.
    pub connection: ConnectionAuthority,
    /// The served catalog content generation the caller described against.
    pub catalog_generation: String,
    /// The description lease (`description_ref`) the operation was re-described under.
    pub description_ref: String,
    /// Canonical input digest, computed by the route over the canonical input encoding.
    pub input_digest: String,
    pub action: GrantAction,
}

impl GrantRequest {
    /// Fail closed on a request that does not fully identify what is being decided. The
    /// refusal is the same neutral value every other refusal is.
    fn validate(&self) -> Result<(), GrantRefusal> {
        let required = [
            self.issuer.as_str(),
            self.tenant.as_str(),
            self.subject.as_str(),
            self.provider.as_str(),
            self.catalog_generation.as_str(),
            self.description_ref.as_str(),
            self.input_digest.as_str(),
        ];
        if required.iter().any(|value| value.is_empty()) {
            return Err(GrantRefusal::Refused);
        }
        if self.actor.as_deref() == Some("") {
            return Err(GrantRefusal::Refused);
        }
        let named = match &self.action {
            GrantAction::Invoke { operation, .. } => operation,
            GrantAction::InboundEvent { event } => event,
        };
        if named.is_empty() {
            return Err(GrantRefusal::Refused);
        }
        Ok(())
    }
}

/// Which arm of the grant admitted. Recorded for audit; refusals record nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantRuling {
    /// An explicit allow exception named the exact operation.
    AdmittedByException,
    /// The selector predicate admitted the declared facts.
    AdmittedBySelector,
    /// The closed inbound event set named the exact event.
    AdmittedByEventSet,
}

/// Proof that Grant evaluation admitted one exact operation for one principal.
///
/// Fields are private and there is no public constructor: a decision can only come to exist
/// inside this module, produced by [`GrantEvaluator::evaluate`]. It binds the whole decided
/// context — issuer, tenant, `sub`, `act`, operation ref, Connection, catalog generation,
/// description lease, grant revision, canonical input digest, ruling, expiry and a one-time
/// id — so nothing downstream can reinterpret what was admitted.
///
/// ```compile_fail
/// // A decision cannot be constructed from any other crate: the fields are private and no
/// // public constructor exists.
/// let forged = domain::GrantDecision {
///     issuer: "identity".to_owned(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDecision {
    issuer: String,
    organization: String,
    principal: String,
    actor: Option<String>,
    provider: String,
    operation: String,
    connection: ConnectionAuthority,
    catalog_generation: String,
    description_ref: String,
    grant: String,
    grant_revision: u64,
    input_digest: String,
    decision: GrantRuling,
    expires_at: SystemTime,
    decision_id: String,
}

/// Decomposed decision handed to [`crate::AdmittedOperation`]; crate-internal so the only
/// consumer is the admission constructor.
pub(crate) struct GrantDecisionParts {
    pub(crate) provider: String,
    pub(crate) operation: String,
    pub(crate) organization: String,
    pub(crate) principal: String,
    pub(crate) grant: String,
    pub(crate) connection: ConnectionAuthority,
}

impl GrantDecision {
    /// **An expired decision refuses at use.** The check lives here, inside the sealed module,
    /// so no consumer of the parts can forget it.
    pub(crate) fn into_parts(self, now: SystemTime) -> Result<GrantDecisionParts, GrantRefusal> {
        if now >= self.expires_at {
            return Err(GrantRefusal::Refused);
        }
        Ok(GrantDecisionParts {
            provider: self.provider,
            operation: self.operation,
            organization: self.organization,
            principal: self.principal,
            grant: self.grant,
            connection: self.connection,
        })
    }

    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// The tenant whose Grants decided.
    #[must_use]
    pub fn organization(&self) -> &str {
        &self.organization
    }

    /// The authenticated principal (`sub`).
    #[must_use]
    pub fn principal(&self) -> &str {
        &self.principal
    }

    /// The acting party (`act`), when one was bound.
    #[must_use]
    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }

    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The admitted operation ref — or, for an inbound admission, the exact event.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    #[must_use]
    pub fn connection(&self) -> &ConnectionAuthority {
        &self.connection
    }

    #[must_use]
    pub fn catalog_generation(&self) -> &str {
        &self.catalog_generation
    }

    #[must_use]
    pub fn description_ref(&self) -> &str {
        &self.description_ref
    }

    /// The admitting grant's stable reference.
    #[must_use]
    pub fn grant(&self) -> &str {
        &self.grant
    }

    /// The tenant grant-set revision that was current when this decision was made.
    #[must_use]
    pub fn grant_revision(&self) -> u64 {
        self.grant_revision
    }

    #[must_use]
    pub fn input_digest(&self) -> &str {
        &self.input_digest
    }

    #[must_use]
    pub fn decision(&self) -> GrantRuling {
        self.decision
    }

    #[must_use]
    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    /// One-time identifier for audit correlation. Unique per decision; not a secret.
    #[must_use]
    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }
}

/// Evaluates Grant requests over the state port.
///
/// A deployment either binds a store or it does not; [`GrantEvaluator::unbound`] exists so a
/// placement without one is an honest **outage** on every evaluation rather than a silent
/// universal refusal that reads as policy.
pub struct GrantEvaluator {
    store: Option<Arc<dyn StateStore>>,
    ttl: Duration,
}

impl GrantEvaluator {
    /// An evaluator over the deployment's bound Grant store.
    #[must_use]
    pub fn bound(store: Arc<dyn StateStore>) -> Self {
        Self {
            store: Some(store),
            ttl: DEFAULT_DECISION_TTL,
        }
    }

    /// An evaluator for a placement that bound no Grant store. Every evaluation answers
    /// [`GrantRefusal::Unavailable`].
    #[must_use]
    pub fn unbound() -> Self {
        Self {
            store: None,
            ttl: DEFAULT_DECISION_TTL,
        }
    }

    /// Override how long an issued decision stands before it refuses at use.
    #[must_use]
    pub fn with_decision_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Evaluate one exact request against the tenant's revisioned Grants.
    ///
    /// Precedence, per the domain model: **deny beats allow beats predicate.** A deny in any
    /// applicable grant refuses even when another applicable grant allows. Inbound events are
    /// admitted only by a grant's closed event set — and a deny still beats that.
    ///
    /// # Errors
    ///
    /// [`GrantRefusal`], carrying deliberately nothing about which axis refused.
    pub fn evaluate(
        &self,
        request: &GrantRequest,
        now: SystemTime,
    ) -> Result<GrantDecision, GrantRefusal> {
        let store = self.store.as_deref().ok_or(GrantRefusal::Unavailable)?;
        request.validate()?;
        let set = grant::load(store, &request.tenant)?;
        let applicable: Vec<&grant::Grant> = set
            .grants
            .iter()
            .filter(|candidate| {
                candidate.provider == request.provider
                    && candidate.connection == request.connection.id()
            })
            .collect();
        if applicable.is_empty() {
            return Err(GrantRefusal::Refused);
        }
        let (named, admitted) = match &request.action {
            GrantAction::Invoke { operation, facts } => {
                if applicable.iter().any(|g| g.deny.contains(operation)) {
                    return Err(GrantRefusal::Refused);
                }
                let admitted = if let Some(by_exception) =
                    applicable.iter().find(|g| g.allow.contains(operation))
                {
                    (by_exception, GrantRuling::AdmittedByException)
                } else if let Some(by_selector) = applicable.iter().find(|g| {
                    g.selector
                        .as_ref()
                        .is_some_and(|selector| selector.admits(facts))
                }) {
                    (by_selector, GrantRuling::AdmittedBySelector)
                } else {
                    return Err(GrantRefusal::Refused);
                };
                (operation, admitted)
            }
            GrantAction::InboundEvent { event } => {
                if applicable.iter().any(|g| g.deny.contains(event)) {
                    return Err(GrantRefusal::Refused);
                }
                let Some(by_events) = applicable.iter().find(|g| g.inbound_events.contains(event))
                else {
                    return Err(GrantRefusal::Refused);
                };
                (event, (by_events, GrantRuling::AdmittedByEventSet))
            }
        };
        let (admitting, ruling) = admitted;
        Ok(GrantDecision {
            issuer: request.issuer.clone(),
            organization: request.tenant.clone(),
            principal: request.subject.clone(),
            actor: request.actor.clone(),
            provider: request.provider.clone(),
            operation: named.clone(),
            connection: request.connection.clone(),
            catalog_generation: request.catalog_generation.clone(),
            description_ref: request.description_ref.clone(),
            grant: admitting.grant.clone(),
            grant_revision: set.revision,
            input_digest: request.input_digest.clone(),
            decision: ruling,
            expires_at: now + self.ttl,
            decision_id: next_decision_id(now),
        })
    }
}

/// Unique per decision within and across processes with overwhelming likelihood: a process-wide
/// sequence joined with the wall-clock nanoseconds of issue. It is an audit correlation handle,
/// not a capability and not a secret — the decision value itself is the capability.
fn next_decision_id(now: SystemTime) -> String {
    let sequence = DECISION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("gd-{nanos:x}-{sequence:x}")
}

pub mod conformance {
    //! **The grant-evaluation exercise every state backend runs against itself.**
    //!
    //! The S-041 port already has byte-cell conformance (`connector_state::conformance`). This
    //! is the layer above it: the same Grant records, written and evaluated through a real
    //! backend, must produce the same admissions and the same neutral refusals. A backend's
    //! test is one line:
    //!
    //! ```no_run
    //! # use std::sync::Arc;
    //! domain::grant_conformance::run(Arc::new(connector_state::MemoryState::new()));
    //! ```

    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    use connector_state::StateStore;

    use crate::grant::{
        Grant, GrantEffect, GrantFacts, GrantIdempotency, GrantRisk, GrantSelector, GrantSet,
    };

    use super::{GrantAction, GrantEvaluator, GrantRefusal, GrantRequest, GrantRuling};
    use crate::{ConnectionAuthority, InitiationPolicy};

    const TENANT: &str = "conformance.grants.tenant";
    const DAMAGED_TENANT: &str = "conformance.grants.damaged";

    fn connection() -> ConnectionAuthority {
        ConnectionAuthority::new(
            "connection:grafana:ops",
            InitiationPolicy::b10x_only(),
        )
        .expect("valid connection reference")
    }

    fn read_facts() -> GrantFacts {
        GrantFacts {
            risk: GrantRisk::Low,
            effects: BTreeSet::from([GrantEffect::Read, GrantEffect::Network]),
            idempotency: GrantIdempotency::Idempotent,
        }
    }

    fn seeded_set() -> GrantSet {
        GrantSet {
            revision: 7,
            grants: vec![Grant {
                grant: "grant:observability".to_owned(),
                provider: "grafana".to_owned(),
                connection: "connection:grafana:ops".to_owned(),
                selector: Some(GrantSelector {
                    risk_ceiling: GrantRisk::Medium,
                    effects: BTreeSet::from([GrantEffect::Read, GrantEffect::Network]),
                    idempotency: BTreeSet::from([GrantIdempotency::Idempotent]),
                }),
                allow: BTreeSet::from([
                    "grafana/rollout.restart".to_owned(),
                    "grafana/collide.op".to_owned(),
                ]),
                deny: BTreeSet::from([
                    "grafana/datasource.delete".to_owned(),
                    "grafana/collide.op".to_owned(),
                ]),
                inbound_events: BTreeSet::from(["grafana.alert.fired".to_owned()]),
            }],
        }
    }

    fn request(action: GrantAction) -> GrantRequest {
        GrantRequest {
            issuer: "https://identity.example".to_owned(),
            tenant: TENANT.to_owned(),
            subject: "principal:svc-observer".to_owned(),
            actor: None,
            provider: "grafana".to_owned(),
            connection: connection(),
            catalog_generation: "generation:11".to_owned(),
            description_ref: "description:grafana:11".to_owned(),
            input_digest: "sha256:0f0f".to_owned(),
            action,
        }
    }

    fn invoke(operation: &str, facts: GrantFacts) -> GrantRequest {
        request(GrantAction::Invoke {
            operation: operation.to_owned(),
            facts,
        })
    }

    /// Run every case. Panics with a named case on the first divergence. Cleans up its keys,
    /// so it is safe against a shared, durable backend.
    pub fn run(store: Arc<dyn StateStore>) {
        cleanup(store.as_ref());
        seeded_set().write(store.as_ref(), TENANT).expect("seed");
        let evaluator = GrantEvaluator::bound(Arc::clone(&store));
        let now = SystemTime::now();

        let admitted = evaluator
            .evaluate(&invoke("grafana/datasource.query", read_facts()), now)
            .expect("the selector admits declared read facts");
        assert_eq!(admitted.decision(), GrantRuling::AdmittedBySelector);
        assert_eq!(admitted.grant(), "grant:observability");
        assert_eq!(admitted.grant_revision(), 7, "the read revision is bound");
        assert!(!admitted.decision_id().is_empty());
        assert!(admitted.expires_at() > now);

        let mut destructive = read_facts();
        destructive.risk = GrantRisk::Destructive;
        let by_exception = evaluator
            .evaluate(&invoke("grafana/rollout.restart", destructive.clone()), now)
            .expect("the explicit allow admits what the predicate refuses");
        assert_eq!(by_exception.decision(), GrantRuling::AdmittedByException);

        let collision = evaluator
            .evaluate(&invoke("grafana/collide.op", read_facts()), now)
            .expect_err("deny beats allow on the same operation");
        let nothing = evaluator
            .evaluate(&invoke("grafana/user.delete", destructive), now)
            .expect_err("nothing admits");
        let absent_tenant = {
            let mut other = invoke("grafana/datasource.query", read_facts());
            other.tenant = "conformance.grants.absent".to_owned();
            evaluator
                .evaluate(&other, now)
                .expect_err("an absent tenant refuses")
        };
        for refusal in [collision, nothing, absent_tenant] {
            assert_eq!(
                refusal,
                GrantRefusal::Refused,
                "every policy refusal is the same neutral value"
            );
        }

        let event = evaluator
            .evaluate(
                &request(GrantAction::InboundEvent {
                    event: "grafana.alert.fired".to_owned(),
                }),
                now,
            )
            .expect("a listed event is admitted");
        assert_eq!(event.decision(), GrantRuling::AdmittedByEventSet);
        assert_eq!(
            evaluator.evaluate(
                &request(GrantAction::InboundEvent {
                    event: "grafana.alert.resolved".to_owned(),
                }),
                now,
            ),
            Err(GrantRefusal::Refused),
            "the event set is closed"
        );

        let damaged_key = format!("grants.{DAMAGED_TENANT}");
        store
            .replace(&damaged_key, b"not a grant set", 1024)
            .expect("seed damage");
        let mut damaged = invoke("grafana/datasource.query", read_facts());
        damaged.tenant = DAMAGED_TENANT.to_owned();
        assert_eq!(
            evaluator.evaluate(&damaged, now),
            Err(GrantRefusal::Unavailable),
            "a damaged record is an outage, not a policy"
        );

        let expiring = GrantEvaluator::bound(Arc::clone(&store))
            .with_decision_ttl(Duration::from_secs(1))
            .evaluate(&invoke("grafana/datasource.query", read_facts()), now)
            .expect("admitted");
        assert_eq!(
            crate::AdmittedOperation::from_decision(expiring, now + Duration::from_secs(2)),
            Err(GrantRefusal::Refused),
            "an expired decision refuses at use"
        );

        cleanup(store.as_ref());
    }

    fn cleanup(store: &dyn StateStore) {
        for tenant in [TENANT, DAMAGED_TENANT, "conformance.grants.absent"] {
            let _ = store.delete(&format!("grants.{tenant}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    use connector_state::MemoryState;

    use crate::grant::{
        Grant, GrantEffect, GrantFacts, GrantIdempotency, GrantRisk, GrantSelector, GrantSet,
    };
    use crate::{AdmittedOperation, ConnectionAuthority, InitiationPolicy};

    use super::*;

    const TENANT: &str = "tenant:acme";

    fn connection() -> ConnectionAuthority {
        ConnectionAuthority::new(
            "connection:grafana:ops",
            InitiationPolicy::b10x_only(),
        )
        .expect("valid connection reference")
    }

    fn read_facts() -> GrantFacts {
        GrantFacts {
            risk: GrantRisk::Low,
            effects: BTreeSet::from([GrantEffect::Read, GrantEffect::Network]),
            idempotency: GrantIdempotency::Idempotent,
        }
    }

    fn selector_grant(grant: &str) -> Grant {
        Grant {
            grant: grant.to_owned(),
            provider: "grafana".to_owned(),
            connection: "connection:grafana:ops".to_owned(),
            selector: Some(GrantSelector {
                risk_ceiling: GrantRisk::Medium,
                effects: BTreeSet::from([GrantEffect::Read, GrantEffect::Network]),
                idempotency: BTreeSet::from([GrantIdempotency::Idempotent]),
            }),
            allow: BTreeSet::new(),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }
    }

    fn bound_over(set: &GrantSet) -> GrantEvaluator {
        let store = MemoryState::new();
        set.write(&store, TENANT).expect("seed");
        GrantEvaluator::bound(Arc::new(store))
    }

    fn invoke(operation: &str, facts: GrantFacts) -> GrantRequest {
        GrantRequest {
            issuer: "https://identity.example".to_owned(),
            tenant: TENANT.to_owned(),
            subject: "principal:svc-observer".to_owned(),
            actor: Some("principal:agent".to_owned()),
            provider: "grafana".to_owned(),
            connection: connection(),
            catalog_generation: "generation:11".to_owned(),
            description_ref: "description:grafana:11".to_owned(),
            input_digest: "sha256:0f0f".to_owned(),
            action: GrantAction::Invoke {
                operation: operation.to_owned(),
                facts,
            },
        }
    }

    #[test]
    fn the_memory_backend_serves_grant_evaluation() {
        conformance::run(Arc::new(MemoryState::new()));
    }

    #[test]
    fn no_store_bound_is_an_outage_not_a_refusal() {
        let evaluator = GrantEvaluator::unbound();
        assert_eq!(
            evaluator.evaluate(
                &invoke("grafana/datasource.query", read_facts()),
                SystemTime::now(),
            ),
            Err(GrantRefusal::Unavailable)
        );
    }

    #[test]
    fn deny_beats_allow_on_the_same_operation() {
        let mut grant = selector_grant("grant:observability");
        grant.allow.insert("grafana/collide.op".to_owned());
        grant.deny.insert("grafana/collide.op".to_owned());
        let evaluator = bound_over(&GrantSet {
            revision: 3,
            grants: vec![grant],
        });
        // The predicate would admit these facts and the allow names the operation exactly;
        // the deny still wins.
        assert_eq!(
            evaluator.evaluate(
                &invoke("grafana/collide.op", read_facts()),
                SystemTime::now()
            ),
            Err(GrantRefusal::Refused)
        );
    }

    #[test]
    fn a_deny_in_one_grant_beats_an_allow_in_another() {
        let mut allowing = selector_grant("grant:allowing");
        allowing.allow.insert("grafana/contested.op".to_owned());
        let mut denying = selector_grant("grant:denying");
        denying.selector = None;
        denying.deny.insert("grafana/contested.op".to_owned());
        let evaluator = bound_over(&GrantSet {
            revision: 4,
            grants: vec![allowing, denying],
        });
        assert_eq!(
            evaluator.evaluate(
                &invoke("grafana/contested.op", read_facts()),
                SystemTime::now(),
            ),
            Err(GrantRefusal::Refused)
        );
    }

    #[test]
    fn an_explicit_allow_beats_the_predicate() {
        let mut grant = selector_grant("grant:observability");
        grant.allow.insert("grafana/rollout.restart".to_owned());
        let evaluator = bound_over(&GrantSet {
            revision: 5,
            grants: vec![grant],
        });
        let mut facts = read_facts();
        facts.risk = GrantRisk::Destructive;
        facts.effects.insert(GrantEffect::Delete);
        facts.idempotency = GrantIdempotency::NonIdempotent;
        let decision = evaluator
            .evaluate(&invoke("grafana/rollout.restart", facts), SystemTime::now())
            .expect("the exception admits what every selector axis refuses");
        assert_eq!(decision.decision(), GrantRuling::AdmittedByException);
    }

    #[test]
    fn the_selector_admits_only_within_all_three_axes() {
        let evaluator = bound_over(&GrantSet {
            revision: 6,
            grants: vec![selector_grant("grant:observability")],
        });
        let now = SystemTime::now();

        let admitted = evaluator
            .evaluate(&invoke("grafana/datasource.query", read_facts()), now)
            .expect("within every axis");
        assert_eq!(admitted.decision(), GrantRuling::AdmittedBySelector);

        let mut risky = read_facts();
        risky.risk = GrantRisk::High;
        let mut effectful = read_facts();
        effectful.effects.insert(GrantEffect::SendExternal);
        let mut repeating = read_facts();
        repeating.idempotency = GrantIdempotency::NonIdempotent;
        for outside in [risky, effectful, repeating] {
            assert_eq!(
                evaluator.evaluate(&invoke("grafana/datasource.query", outside), now),
                Err(GrantRefusal::Refused)
            );
        }
    }

    #[test]
    fn no_refusal_names_the_axis_that_refused() {
        let evaluator = bound_over(&GrantSet {
            revision: 7,
            grants: vec![{
                let mut grant = selector_grant("grant:observability");
                grant.deny.insert("grafana/denied.op".to_owned());
                grant
            }],
        });
        let now = SystemTime::now();

        let mut risky = read_facts();
        risky.risk = GrantRisk::Destructive;
        let mut effectful = read_facts();
        effectful.effects.insert(GrantEffect::Money);
        let mut repeating = read_facts();
        repeating.idempotency = GrantIdempotency::Conditional;
        let mut absent_tenant = invoke("grafana/datasource.query", read_facts());
        absent_tenant.tenant = "tenant:unknown".to_owned();

        let refusals = [
            evaluator.evaluate(&invoke("grafana/datasource.query", risky), now),
            evaluator.evaluate(&invoke("grafana/datasource.query", effectful), now),
            evaluator.evaluate(&invoke("grafana/datasource.query", repeating), now),
            evaluator.evaluate(&invoke("grafana/denied.op", read_facts()), now),
            evaluator.evaluate(&absent_tenant, now),
        ];
        let texts: BTreeSet<String> = refusals
            .iter()
            .map(|refusal| refusal.clone().expect_err("refuses").to_string())
            .collect();
        assert_eq!(
            texts.len(),
            1,
            "every axis must refuse with byte-identical text; distinct texts are an oracle"
        );
        let text = texts.into_iter().next().expect("one text");
        for oracle in [
            "risk",
            "effect",
            "idempot",
            "deny",
            "allow",
            "exception",
            "expir",
            "event",
            "selector",
            "revision",
            "ceiling",
            "tenant",
        ] {
            assert!(
                !text.to_lowercase().contains(oracle),
                "refusal text {text:?} names the {oracle:?} axis"
            );
        }
    }

    #[test]
    fn an_expired_decision_refuses_at_use() {
        let evaluator = bound_over(&GrantSet {
            revision: 8,
            grants: vec![selector_grant("grant:observability")],
        })
        .with_decision_ttl(Duration::from_secs(30));
        let issued_at = SystemTime::now();
        let decision = evaluator
            .evaluate(&invoke("grafana/datasource.query", read_facts()), issued_at)
            .expect("admitted");

        let fresh =
            AdmittedOperation::from_decision(decision.clone(), issued_at + Duration::from_secs(29))
                .expect("a live decision admits");
        assert_eq!(fresh.operation(), "grafana/datasource.query");
        assert_eq!(
            AdmittedOperation::from_decision(decision, issued_at + Duration::from_secs(30)),
            Err(GrantRefusal::Refused),
            "at and past expiry the decision refuses"
        );
    }

    #[test]
    fn the_decision_binds_the_whole_decided_context() {
        let evaluator = bound_over(&GrantSet {
            revision: 9,
            grants: vec![selector_grant("grant:observability")],
        });
        let now = SystemTime::now();
        let decision = evaluator
            .evaluate(&invoke("grafana/datasource.query", read_facts()), now)
            .expect("admitted");

        assert_eq!(decision.issuer(), "https://identity.example");
        assert_eq!(decision.organization(), TENANT);
        assert_eq!(decision.principal(), "principal:svc-observer");
        assert_eq!(decision.actor(), Some("principal:agent"));
        assert_eq!(decision.provider(), "grafana");
        assert_eq!(decision.operation(), "grafana/datasource.query");
        assert_eq!(decision.connection().id(), "connection:grafana:ops");
        assert_eq!(decision.catalog_generation(), "generation:11");
        assert_eq!(decision.description_ref(), "description:grafana:11");
        assert_eq!(decision.grant(), "grant:observability");
        assert_eq!(decision.grant_revision(), 9);
        assert_eq!(decision.input_digest(), "sha256:0f0f");
        assert_eq!(decision.expires_at(), now + DEFAULT_DECISION_TTL);

        let second = evaluator
            .evaluate(&invoke("grafana/datasource.query", read_facts()), now)
            .expect("admitted");
        assert_ne!(
            decision.decision_id(),
            second.decision_id(),
            "decision ids are one-time"
        );
    }

    #[test]
    fn a_request_that_does_not_identify_itself_refuses() {
        let evaluator = bound_over(&GrantSet {
            revision: 10,
            grants: vec![selector_grant("grant:observability")],
        });
        let now = SystemTime::now();
        let blank: [fn(&mut GrantRequest); 5] = [
            |request| request.issuer.clear(),
            |request| request.subject.clear(),
            |request| request.catalog_generation.clear(),
            |request| request.description_ref.clear(),
            |request| request.input_digest.clear(),
        ];
        for erase in blank {
            let mut request = invoke("grafana/datasource.query", read_facts());
            erase(&mut request);
            assert_eq!(
                evaluator.evaluate(&request, now),
                Err(GrantRefusal::Refused)
            );
        }
    }
}
