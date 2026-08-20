//! How this Integration is composed, for each placement that can hold it.
//!
//! Split out of `backend.rs` rather than bumping its size waiver a second time in one day: the
//! waiver's own text admits no further growth before its arms split, and construction is one of
//! the arms it names. The two constructors differ in exactly one thing — where this Integration
//! keeps its own list of Connections — and everything below `open_inner` is shared.

use std::sync::Arc;

use connector_secrets::PreparedSecretStore;
use connectors_config::HostedGitlabConfig;
use hosted_state::PostgresState;

use crate::backend::{GitlabBackend, GitlabError};
use crate::state::GitlabState;

impl GitlabBackend {
    /// Open a hosted adapter with policy-only configuration and injected secret custody.
    ///
    /// The bookkeeping lives in Postgres because a hosted Connector runs as several replicas and
    /// they must agree on which Connections exist.
    pub async fn open_hosted(
        tenant_id: String,
        policy: HostedGitlabConfig,
        credential_store: Arc<dyn PreparedSecretStore>,
        state_store: PostgresState,
    ) -> Result<Self, GitlabError> {
        Self::open_inner(
            tenant_id,
            policy,
            credential_store,
            GitlabState::Hosted(state_store),
        )
        .await
    }

    /// Open the same adapter for one personal-local placement.
    ///
    /// Identical in everything a caller can observe. The only difference is where this Integration
    /// keeps its own list of Connections: one process on one machine writes owner-only files beside
    /// its socket, and needs no database to do it. Without this constructor GitLab could not be
    /// composed anywhere without Postgres, which is why a workstation could not reach GitLab at
    /// all — a fact about replica bookkeeping that read as "GitLab needs a database".
    pub async fn open(
        tenant_id: String,
        policy: HostedGitlabConfig,
        credential_store: Arc<dyn PreparedSecretStore>,
        state_root: &std::path::Path,
    ) -> Result<Self, GitlabError> {
        Self::open_inner(
            tenant_id,
            policy,
            credential_store,
            GitlabState::Local {
                root: state_root.to_path_buf(),
            },
        )
        .await
    }
}
