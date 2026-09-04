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
    config: &HostedSlackConfig,
) -> Result<AdminIntegration, SlackError> {
    let mut configuration = [
        "public_origin",
        "team_id",
        "org_read_grant_ref",
        "user_grant_ref",
        "companion_grant_ref",
        "initiation",
        "allowed_events",
    ]
    .into_iter()
    .map(AdminConfigurationField::valid)
    .collect::<Vec<_>>();
    let mut credentials = vec![
        credential_requirement(
            tenant_id,
            backend::SERVICE,
            backend::APP_TOKEN_CREDENTIAL,
            false,
        )?,
        credential_requirement(
            tenant_id,
            backend::SERVICE,
            backend::BOT_TOKEN_CREDENTIAL,
            false,
        )?,
    ];
    if config.oauth_client_id.is_some() {
        configuration.push(AdminConfigurationField::valid("oauth_client_id"));
        configuration.push(AdminConfigurationField::valid("oauth_redirect_uri"));
        credentials.push(credential_requirement(
            tenant_id,
            "login",
            backend::OAUTH_CLIENT_SECRET_CREDENTIAL,
            true,
        )?);
    }
    Ok(AdminIntegration::new(
        "slack",
        configuration,
        credentials,
    ))
}

fn credential_requirement(
    tenant_id: &str,
    service: &str,
    credential: &str,
    required: bool,
) -> Result<AdminCredentialRequirement, SlackError> {
    let reference = CredentialRef::new(
        tenant_id,
        backend::AUTHORITY,
        service,
        credential,
    )
    .map_err(|_| SlackError::new("credential-address"))?;
    Ok(AdminCredentialRequirement::token(
        credential, required, reference,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use connector_secrets::MemoryStore;
    use connector_state::MemoryState;
    use connectors_config::InitiationConfig;
    use service::AdminRegistry;

    use super::*;

    fn config(oauth: bool) -> HostedSlackConfig {
        HostedSlackConfig {
            public_origin: "https://connectors.example.test/api/connectors/v1".to_owned(),
            team_id: "T01234567".to_owned(),
            oauth_client_id: oauth.then(|| "123456789.987654321".to_owned()),
            oauth_redirect_uri: oauth.then(|| {
                "https://connectors.example.test/api/connectors/v1/oauth/slack/callback".to_owned()
            }),
            org_read_grant_ref: "grant:slack:org-read".to_owned(),
            user_grant_ref: "grant:slack:org-user".to_owned(),
            companion_grant_ref: "grant:slack:companion".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned()],
            connect_session_ttl_seconds: 300,
        }
    }

    #[tokio::test]
    async fn organization_credentials_do_not_claim_personal_oauth_is_configured() {
        for (oauth, expected) in [
            (false, vec!["app_token", "bot_token"]),
            (
                true,
                vec!["app_token", "bot_token", "oauth_client_secret"],
            ),
        ] {
            let store = Arc::new(MemoryStore::new());
            let integration = hosted_admin_integration("tenant-one", &config(oauth)).unwrap();
            let registry = AdminRegistry::new(
                "tenant-one".to_owned(),
                store,
                Arc::new(MemoryState::new()),
                vec![integration],
            )
            .unwrap();
            let status = registry.status().await;
            let credentials = status.integrations[0]
                .credentials
                .iter()
                .map(|credential| credential.name.as_str())
                .collect::<Vec<_>>();
            assert_eq!(credentials, expected);
            assert!(!status.integrations[0].credentials[0].required);
            assert!(!status.integrations[0].credentials[1].required);
            assert_eq!(status.integrations[0].ready, !oauth);
        }
    }
}
