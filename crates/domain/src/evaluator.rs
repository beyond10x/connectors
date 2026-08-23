//! Grant evaluation. S-044 lands the real `GrantEvaluator` — selector semantics over the state
//! port, 403/503 refusal shape — and it will be the only production code that constructs a
//! [`GrantDecision`]. Until then this module exists to seal the proof type.

use crate::ConnectionAuthority;

/// Proof that Grant evaluation admitted one exact operation for one principal.
///
/// Fields are private and there is no public constructor: a decision can only come to exist
/// inside this module, so holding one *is* the evidence that evaluation ran. Nothing in
/// production constructs one yet — the S-044 evaluator owns the real construction path; the
/// `#[cfg(test)]` builder below exists so the sealed admission path stays compiled and tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDecision {
    provider: String,
    operation: String,
    organization: String,
    principal: String,
    grant: String,
    connection: ConnectionAuthority,
}

/// Decomposed decision handed to [`crate::AdmittedOperation`]; crate-internal so the only
/// consumers are the admission constructor and the future evaluator.
pub(crate) struct GrantDecisionParts {
    pub(crate) provider: String,
    pub(crate) operation: String,
    pub(crate) organization: String,
    pub(crate) principal: String,
    pub(crate) grant: String,
    pub(crate) connection: ConnectionAuthority,
}

impl GrantDecision {
    pub(crate) fn into_parts(self) -> GrantDecisionParts {
        GrantDecisionParts {
            provider: self.provider,
            operation: self.operation,
            organization: self.organization,
            principal: self.principal,
            grant: self.grant,
            connection: self.connection,
        }
    }

    /// Stand-in for the S-044 evaluator so tests can exercise the sealed hosted path. Compiled
    /// only for this crate's own tests; no other crate can construct a decision.
    #[cfg(test)]
    pub(crate) fn admitted_for_tests(
        provider: impl Into<String>,
        operation: impl Into<String>,
        organization: impl Into<String>,
        principal: impl Into<String>,
        grant: impl Into<String>,
        connection: ConnectionAuthority,
    ) -> Self {
        Self {
            provider: provider.into(),
            operation: operation.into(),
            organization: organization.into(),
            principal: principal.into(),
            grant: grant.into(),
            connection,
        }
    }
}
