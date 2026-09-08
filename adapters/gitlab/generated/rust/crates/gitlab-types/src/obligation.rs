// generated from gitlab v1
// model digest 5464a959b36fe5673bd570eecf3f661540dd261b60c0fde2fd37f4a5bf2d97f3
// contract digest c3d3a9b408c90c2a8b03daf08984379733603f2c84f483640772f01ea8aea190
// do not edit: regenerate with `ess synthesize`

//! The typed refusal of an unmet obligation, and the conversion seams owed between contexts.
//!
//! An obligation is a capability the synthesis plan owes the implementor — the contract is declared,
//! the behaviour is not. Until an implementation satisfies one, its stub returns [`UnmetObligation`]:
//! a value naming the plan entry, never a panic and never a guess, so a workspace built on stubs
//! compiles and reports its own gaps.

/// A capability the synthesis plan owes and nothing has satisfied yet.
///
/// The two fields spell the plan entry: look the pair up in `PLAN.md` for the contract being
/// refused. A satisfying implementation never constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetObligation {
    /// The capability kind, as the plan spells it.
    pub capability: &'static str,
    /// The construct that requires it, in the specification's own spelling.
    pub source: &'static str,
}

impl core::fmt::Display for UnmetObligation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "unmet obligation: {} `{}` — see PLAN.md",
            self.capability, self.source
        )
    }
}
