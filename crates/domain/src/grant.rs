//! Revisioned tenant Grant records and where they live: one bounded state cell per tenant,
//! reached through the S-041 [`connector_state::StateStore`] port.
//!
//! A Grant is the unit of authority from the domain model: tenant-scoped, per-connector,
//! admitting operations by **selector** over declared facts — risk ceiling, effects subset,
//! idempotency class — plus explicit allow/deny exceptions where **deny beats allow beats
//! predicate**, and admitting inbound provider events as a **closed** set with no wildcard
//! grammar at all.
//!
//! The vocabulary here mirrors the catalogue's (`connector_spec::Risk`, `SemanticEffect`,
//! `Idempotency`) rather than re-exporting it, for the same reason `crates/catalog` mirrors it:
//! this crate resolves no compiler machinery, and the spellings below are the stable published
//! words the canonical document carries.
//!
//! [`GrantSet::write`] is the bootstrap write path. The CAS-revisioned proposal/receipt
//! mutation surface (`connectors.grants.manage`) is a later story; evaluation (S-044) only ever
//! loads, and binds the revision it read into every decision it issues.

use std::collections::BTreeSet;

use connector_state::{StateError, StateStore};
use serde::{Deserialize, Serialize};

use crate::evaluator::GrantRefusal;

/// The largest serialized grant set any backend stores or serves for one tenant.
pub const GRANTS_CELL_BOUND: usize = 64 * 1024;

/// How much damage an operation claims it can do. Ordered: a ceiling admits everything at or
/// below it. The spellings are the canonical document's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantRisk {
    Low,
    Medium,
    High,
    Destructive,
}

/// What executing the operation *means*, in the catalogue's semantic-effect spellings. A grant
/// names the set it admits; the operation's declared effects must be a subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantEffect {
    Pure,
    Read,
    Model,
    Network,
    WriteFile,
    WriteDb,
    SendExternal,
    Delete,
    Money,
    HumanVisible,
}

impl GrantEffect {
    /// Every declared effect, in declaration order. A worst-case claim — an operation whose
    /// declared facts are not yet served — must name all of them, and it must do so through
    /// this constant rather than by hand.
    ///
    /// Exhaustive by construction: the `match` below names every variant without a wildcard,
    /// so adding one refuses to compile until this list is revisited — a new effect can widen
    /// a worst-case claim but can never be silently missing from it.
    pub const ALL: [Self; 10] = {
        match Self::Pure {
            Self::Pure
            | Self::Read
            | Self::Model
            | Self::Network
            | Self::WriteFile
            | Self::WriteDb
            | Self::SendExternal
            | Self::Delete
            | Self::Money
            | Self::HumanVisible => {}
        }
        [
            Self::Pure,
            Self::Read,
            Self::Model,
            Self::Network,
            Self::WriteFile,
            Self::WriteDb,
            Self::SendExternal,
            Self::Delete,
            Self::Money,
            Self::HumanVisible,
        ]
    };
}

/// Whether repeating the operation repeats its consequence. A grant names the classes it admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantIdempotency {
    Idempotent,
    NonIdempotent,
    Conditional,
}

/// The declared facts of the exact operation under evaluation, read from the reviewed
/// description — never derived at the evaluation seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFacts {
    pub risk: GrantRisk,
    pub effects: BTreeSet<GrantEffect>,
    pub idempotency: GrantIdempotency,
}

/// The predicate arm of a Grant: admits an operation only when **all three** axes admit it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSelector {
    /// Admits operations whose declared risk is at or below this ceiling.
    pub risk_ceiling: GrantRisk,
    /// Admits operations whose declared effects are a subset of this set.
    pub effects: BTreeSet<GrantEffect>,
    /// Admits operations whose declared idempotency class is in this set.
    pub idempotency: BTreeSet<GrantIdempotency>,
}

impl GrantSelector {
    pub(crate) fn admits(&self, facts: &GrantFacts) -> bool {
        facts.risk <= self.risk_ceiling
            && facts.effects.is_subset(&self.effects)
            && self.idempotency.contains(&facts.idempotency)
    }
}

/// One tenant Grant: per-connector, bound to one Connection, never to a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Stable reference bound into every decision this grant admits.
    pub grant: String,
    /// The target Provider this grant speaks for.
    pub provider: String,
    /// The exact Connection a principal may exercise under this grant.
    pub connection: String,
    /// The predicate. Absent means this grant admits by exception or event set only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<GrantSelector>,
    /// Exact operation refs admitted regardless of the predicate. Deny still beats allow.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub allow: BTreeSet<String>,
    /// Exact operation refs refused regardless of everything else.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub deny: BTreeSet<String>,
    /// The closed set of provider events this grant admits inbound. No wildcards, ever.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub inbound_events: BTreeSet<String>,
}

impl Grant {
    fn validate(&self) -> Result<(), StateError> {
        if self.grant.is_empty() || self.provider.is_empty() || self.connection.is_empty() {
            return Err(StateError::Invalid);
        }
        for entry in self
            .allow
            .iter()
            .chain(self.deny.iter())
            .chain(self.inbound_events.iter())
        {
            // Closed sets are closed: an empty entry matches nothing honest, and a wildcard
            // entry is a grammar this vocabulary deliberately does not have. Refusing the
            // record at the boundary is what keeps "closed" a checked property rather than a
            // comment.
            if entry.is_empty() || entry.contains('*') || entry.contains('?') {
                return Err(StateError::Invalid);
            }
        }
        Ok(())
    }
}

/// A tenant's Grants plus the revision that produced them. The revision travels into every
/// decision, so an audit row can say which published policy admitted an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSet {
    pub revision: u64,
    pub grants: Vec<Grant>,
}

impl GrantSet {
    fn validate(&self) -> Result<(), StateError> {
        for grant in &self.grants {
            grant.validate()?;
        }
        Ok(())
    }

    /// Store this set as the tenant's current grants.
    ///
    /// This is the bootstrap write path: a plain replace, validated first so an invalid record
    /// can never become the stored one. The previewable CAS proposal/receipt surface arrives
    /// with the `connectors.grants.manage` route family and will supersede direct writes.
    ///
    /// # Errors
    ///
    /// [`StateError::Invalid`] for a tenant that cannot form a state key or a grant that fails
    /// validation; otherwise whatever the backend answers.
    pub fn write(&self, store: &dyn StateStore, tenant: &str) -> Result<(), StateError> {
        self.validate()?;
        let key = grants_key(tenant)?;
        let body = serde_json::to_vec(self).map_err(|_| StateError::Invalid)?;
        store.replace(&key, &body, GRANTS_CELL_BOUND)
    }
}

/// The state cell that holds one tenant's grants.
///
/// # Errors
///
/// [`StateError::Invalid`] when the tenant cannot appear in the closed key grammar. Such a
/// tenant cannot have stored grants, which evaluation reads as a refusal, not an outage.
pub(crate) fn grants_key(tenant: &str) -> Result<String, StateError> {
    let key = format!("grants.{tenant}");
    connector_state::validate_key(&key)?;
    Ok(key)
}

/// Load the tenant's revisioned grants for evaluation, mapping every outcome onto the refusal
/// semantics the domain model states:
///
/// - the backend cannot answer, or the cell is unreadable or undecodable → outage
///   ([`GrantRefusal::Unavailable`]): damaged authority must not read as policy;
/// - no cell, an empty set, or a tenant outside the key grammar → refusal
///   ([`GrantRefusal::Refused`]): fail closed.
pub(crate) fn load(store: &dyn StateStore, tenant: &str) -> Result<GrantSet, GrantRefusal> {
    let key = grants_key(tenant).map_err(|_| GrantRefusal::Refused)?;
    let body = match store.read(&key, GRANTS_CELL_BOUND) {
        Ok(Some(body)) => body,
        Ok(None) => return Err(GrantRefusal::Refused),
        Err(StateError::Invalid) => return Err(GrantRefusal::Refused),
        Err(StateError::Unavailable | StateError::Capacity) => {
            return Err(GrantRefusal::Unavailable);
        }
    };
    let set: GrantSet = serde_json::from_slice(&body).map_err(|_| GrantRefusal::Unavailable)?;
    set.validate().map_err(|_| GrantRefusal::Unavailable)?;
    if set.grants.is_empty() {
        return Err(GrantRefusal::Refused);
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use connector_state::MemoryState;

    use super::*;

    fn minimal(grant: &str) -> Grant {
        Grant {
            grant: grant.to_owned(),
            provider: "grafana".to_owned(),
            connection: "connection:grafana:ops".to_owned(),
            selector: None,
            allow: BTreeSet::new(),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }
    }

    #[test]
    fn a_grant_set_round_trips_through_its_cell() {
        let store = MemoryState::new();
        let set = GrantSet {
            revision: 7,
            grants: vec![minimal("grant:observability-read")],
        };
        set.write(&store, "tenant:acme").expect("write");
        let loaded = load(&store, "tenant:acme").expect("load");
        assert_eq!(loaded, set);
    }

    #[test]
    fn a_tenant_outside_the_key_grammar_has_no_grants() {
        let store = MemoryState::new();
        assert_eq!(load(&store, "Tenant Acme"), Err(GrantRefusal::Refused));
        let set = GrantSet {
            revision: 1,
            grants: vec![minimal("grant:x")],
        };
        assert_eq!(set.write(&store, "Tenant Acme"), Err(StateError::Invalid));
    }

    #[test]
    fn closed_sets_refuse_wildcard_and_empty_entries() {
        let store = MemoryState::new();
        for bad in ["grafana.*", "grafana.alert.?", ""] {
            let mut grant = minimal("grant:events");
            grant.inbound_events = BTreeSet::from([bad.to_owned()]);
            let set = GrantSet {
                revision: 1,
                grants: vec![grant],
            };
            assert_eq!(
                set.write(&store, "tenant:acme"),
                Err(StateError::Invalid),
                "{bad:?} must not be storable"
            );
        }
    }

    #[test]
    fn a_damaged_cell_is_an_outage_not_a_policy() {
        let store = MemoryState::new();
        let key = grants_key("tenant:acme").expect("key");
        store
            .replace(&key, b"not a grant set", GRANTS_CELL_BOUND)
            .expect("seed");
        assert_eq!(load(&store, "tenant:acme"), Err(GrantRefusal::Unavailable));
    }

    #[test]
    fn an_absent_or_empty_set_refuses() {
        let store = MemoryState::new();
        assert_eq!(load(&store, "tenant:acme"), Err(GrantRefusal::Refused));
        let set = GrantSet {
            revision: 2,
            grants: Vec::new(),
        };
        set.write(&store, "tenant:acme").expect("write");
        assert_eq!(load(&store, "tenant:acme"), Err(GrantRefusal::Refused));
    }
}
