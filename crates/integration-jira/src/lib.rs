#![forbid(unsafe_code)]

//! Jira Cloud organization-read and delegated-user Integration adapter.

mod backend;

pub use backend::{JiraBackend, JiraError};

use connector_secrets::CredentialRef;
use connectors_config::{HostedJiraConfig, JiraSharedAuth};
use service::{AdminConfigurationField, AdminCredentialRequirement, AdminIntegration};

/// Administrative requirements for one activated hosted Jira Integration.
pub fn hosted_admin_integration(
    tenant_id: &str,
    config: &HostedJiraConfig,
) -> Result<AdminIntegration, JiraError> {
    let configuration = [
        "cloud_id",
        "site_origin",
        "public_origin",
        "allowed_project_keys",
        "shared_auth",
        "service_oauth_client_id",
        "user_oauth_client_id",
        "oauth_redirect_uri",
        "organization_read_grant_ref",
        "user_grant_ref",
        "initiation",
    ]
    .into_iter()
    .map(AdminConfigurationField::valid)
    .collect();
    let requirement = |name: &str, leaf: &str| {
        CredentialRef::new(tenant_id, backend::AUTHORITY, backend::LOGIN_SERVICE, leaf)
            .map(|reference| AdminCredentialRequirement::token(name, true, reference))
            .map_err(|_| JiraError::new("credential-address"))
    };
    let mut credentials = vec![requirement(
        "oauth_client_secret",
        backend::USER_CLIENT_SECRET_CREDENTIAL,
    )?];
    credentials.push(match config.shared_auth {
        JiraSharedAuth::ServiceOauth => requirement(
            "service_oauth_client_secret",
            backend::SERVICE_CLIENT_SECRET_CREDENTIAL,
        )?,
        JiraSharedAuth::ServiceApiToken => {
            requirement("service_api_token", backend::SERVICE_API_TOKEN_CREDENTIAL)?
        }
    });
    Ok(AdminIntegration::new("jira", configuration, credentials))
}
