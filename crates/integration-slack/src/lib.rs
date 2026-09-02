#![forbid(unsafe_code)]

//! Slack Integration adapter.

mod backend;

pub use backend::{SlackBackend, SlackError};

use connector_secrets::CredentialRef;
use connectors_config::HostedSlackConfig;
use service::{AdminConfigurationField, AdminCredentialRequirement, AdminIntegration};

/// Administrative requirements for one activated hosted Slack Integration.
pub fn hosted_admin_integration(
    tenant_id: &str,
    _config: &HostedSlackConfig,
) -> Result<AdminIntegration, SlackError> {
    let configuration = [
        "public_origin",
        "team_id",
        "oauth_client_id",
        "oauth_redirect_uri",
        "org_read_grant_ref",
        "user_grant_ref",
        "companion_grant_ref",
        "initiation",
        "allowed_events",
    ]
    .into_iter()
    .map(AdminConfigurationField::valid)
    .collect();
    let reference = CredentialRef::new(
        tenant_id,
        backend::AUTHORITY,
        "login",
        backend::OAUTH_CLIENT_SECRET_CREDENTIAL,
    )
    .map_err(|_| SlackError::new("credential-address"))?;
    Ok(AdminIntegration::new(
        "slack",
        configuration,
        vec![AdminCredentialRequirement::token(
            "oauth_client_secret",
            true,
            reference,
        )],
    ))
}
