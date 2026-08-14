use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Which side of a configured Connection may begin an interaction.
///
/// The names are relative to the Connectors boundary. They deliberately avoid `inbound` and
/// `outbound`, whose meaning changes with the observer and which would collide with an Operation's
/// existing read/write direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionInitiator {
    /// A B10x principal may ask Connectors to start a declared Operation at the provider.
    B10x,
    /// The provider may start a declared Channel or session toward B10x.
    Provider,
}

/// The non-empty set of sides that may initiate through one Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiationPolicy {
    allowed: BTreeSet<ConnectionInitiator>,
}

impl InitiationPolicy {
    /// Construct a policy. A Connection with no permitted initiator is invalid rather than a
    /// second spelling of an inactive lifecycle state.
    pub fn new(
        allowed: impl IntoIterator<Item = ConnectionInitiator>,
    ) -> Result<Self, ConnectionAuthorityError> {
        let allowed = allowed.into_iter().collect::<BTreeSet<_>>();
        if allowed.is_empty() {
            return Err(ConnectionAuthorityError::NoAllowedInitiator);
        }
        Ok(Self { allowed })
    }

    #[must_use]
    pub fn b10x_only() -> Self {
        Self {
            allowed: BTreeSet::from([ConnectionInitiator::B10x]),
        }
    }

    #[must_use]
    pub fn provider_only() -> Self {
        Self {
            allowed: BTreeSet::from([ConnectionInitiator::Provider]),
        }
    }

    #[must_use]
    pub fn bidirectional() -> Self {
        Self {
            allowed: BTreeSet::from([
                ConnectionInitiator::B10x,
                ConnectionInitiator::Provider,
            ]),
        }
    }

    #[must_use]
    pub fn allows(&self, initiator: ConnectionInitiator) -> bool {
        self.allowed.contains(&initiator)
    }

    pub fn iter(&self) -> impl Iterator<Item = ConnectionInitiator> + '_ {
        self.allowed.iter().copied()
    }
}

/// Connection identity plus its independently configured initiation boundary.
///
/// Grants remain separate: this value says which side may start, not which principal may execute
/// which operation or receive which channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionAuthority {
    id: String,
    initiation: InitiationPolicy,
}

impl ConnectionAuthority {
    pub fn new(
        id: impl Into<String>,
        initiation: InitiationPolicy,
    ) -> Result<Self, ConnectionAuthorityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ConnectionAuthorityError::EmptyConnection);
        }
        Ok(Self { id, initiation })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn initiation(&self) -> &InitiationPolicy {
        &self.initiation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConnectionAuthorityError {
    #[error("Connection identity is empty")]
    EmptyConnection,
    #[error("Connection initiation policy allows no initiator")]
    NoAllowedInitiator,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_policies_are_explicit_sets() {
        let b10x = InitiationPolicy::b10x_only();
        assert!(b10x.allows(ConnectionInitiator::B10x));
        assert!(!b10x.allows(ConnectionInitiator::Provider));

        let provider = InitiationPolicy::provider_only();
        assert!(!provider.allows(ConnectionInitiator::B10x));
        assert!(provider.allows(ConnectionInitiator::Provider));

        let both = InitiationPolicy::bidirectional();
        assert!(both.allows(ConnectionInitiator::B10x));
        assert!(both.allows(ConnectionInitiator::Provider));
    }

    #[test]
    fn inactive_is_a_lifecycle_state_not_an_empty_policy() {
        assert_eq!(
            InitiationPolicy::new([]),
            Err(ConnectionAuthorityError::NoAllowedInitiator)
        );
    }
}
