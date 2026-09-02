#![forbid(unsafe_code)]

//! GitLab delegated-user Integration adapter.

mod backend;
mod open;
mod profiles;
mod state;
mod transport;

pub use backend::{GitlabBackend, GitlabError};

use connector_secrets::CredentialRef;
use connectors_config::HostedGitlabConfig;
use service::{AdminConfigurationField, AdminCredentialRequirement, AdminIntegration};

/// Administrative requirements for one activated hosted GitLab Integration.
pub fn hosted_admin_integration(
    tenant_id: &str,
    _config: &HostedGitlabConfig,
) -> Result<AdminIntegration, GitlabError> {
    let configuration = [
        "origin",
        "public_origin",
        "oauth_client_id",
        "oauth_redirect_uri",
        "user_grant_ref",
        "initiation",
    ]
    .into_iter()
    .map(AdminConfigurationField::valid)
    .collect();
    let reference = CredentialRef::new(
        tenant_id,
        backend::AUTHORITY,
        backend::LOGIN_SERVICE,
        backend::OAUTH_CLIENT_SECRET_CREDENTIAL,
    )
    .map_err(|_| GitlabError::new("credential-address"))?;
    Ok(AdminIntegration::new(
        "gitlab",
        configuration,
        vec![AdminCredentialRequirement::token(
            "oauth_client_secret",
            true,
            reference,
        )],
    ))
}
