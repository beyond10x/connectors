//! Tenant-bound administration of credentials required by activated hosted Integrations.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use connector_secrets::{CredentialRef, Secret, SecretStore, StoreError};
use connector_state::StateStore;
use serde::Serialize;
use zeroize::Zeroizing;

const AUDIT_KEY: &str = "admin.integrations.audit";
const MAX_AUDIT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CREDENTIAL_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminConfigurationField {
    pub name: String,
    pub state: &'static str,
}

impl AdminConfigurationField {
    #[must_use]
    pub fn valid(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: "valid",
        }
    }
}

#[derive(Clone)]
pub struct AdminCredentialRequirement {
    name: String,
    required: bool,
    reference: CredentialRef,
}

impl std::fmt::Debug for AdminCredentialRequirement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdminCredentialRequirement")
            .field("name", &self.name)
            .field("required", &self.required)
            .field("reference", &"[DERIVED]")
            .finish()
    }
}

impl AdminCredentialRequirement {
    #[must_use]
    pub fn token(name: impl Into<String>, required: bool, reference: CredentialRef) -> Self {
        Self {
            name: name.into(),
            required,
            reference,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdminIntegration {
    integration_ref: String,
    configuration: Vec<AdminConfigurationField>,
    credentials: Vec<AdminCredentialRequirement>,
}

impl AdminIntegration {
    #[must_use]
    pub fn new(
        integration_ref: impl Into<String>,
        configuration: Vec<AdminConfigurationField>,
        credentials: Vec<AdminCredentialRequirement>,
    ) -> Self {
        Self {
            integration_ref: integration_ref.into(),
            configuration,
            credentials,
        }
    }
}

pub struct AdminCredentialInput(Zeroizing<String>);

impl AdminCredentialInput {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    fn validate(&self) -> Result<(), AdminError> {
        if self.0.is_empty()
            || self.0.len() > MAX_CREDENTIAL_BYTES
            || self.0.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(AdminError::Invalid);
        }
        Ok(())
    }
}

impl std::fmt::Debug for AdminCredentialInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AdminCredentialInput([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialState {
    Present,
    Missing,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminCredentialStatus {
    pub name: String,
    pub required: bool,
    pub state: CredentialState,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminIntegrationStatus {
    pub integration_ref: String,
    pub active: bool,
    pub configuration: Vec<AdminConfigurationField>,
    pub credentials: Vec<AdminCredentialStatus>,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminStatus {
    pub integrations: Vec<AdminIntegrationStatus>,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AdminError {
    #[error("the administrative request is invalid")]
    Invalid,
    #[error("the activated Integration or credential requirement does not exist")]
    NotFound,
    #[error("the credential is already configured")]
    Conflict,
    #[error("credential custody is unavailable")]
    Unavailable,
    #[error("the administrative action could not be audited")]
    AuditUnavailable,
}

pub struct AdminRegistry {
    tenant_id: String,
    credentials: Arc<dyn SecretStore>,
    state: Arc<dyn StateStore>,
    integrations: BTreeMap<String, AdminIntegration>,
}

impl AdminRegistry {
    pub fn new(
        tenant_id: String,
        credentials: Arc<dyn SecretStore>,
        state: Arc<dyn StateStore>,
        integrations: Vec<AdminIntegration>,
    ) -> Result<Self, AdminError> {
        let mut indexed = BTreeMap::new();
        for integration in integrations {
            let mut names = BTreeSet::new();
            if integration.integration_ref.is_empty()
                || integration.credentials.iter().any(|requirement| {
                    requirement.reference.tenant() != tenant_id
                        || !names.insert(requirement.name.clone())
                })
                || indexed
                    .insert(integration.integration_ref.clone(), integration)
                    .is_some()
            {
                return Err(AdminError::Invalid);
            }
        }
        Ok(Self {
            tenant_id,
            credentials,
            state,
            integrations: indexed,
        })
    }

    pub async fn status(&self) -> AdminStatus {
        let mut integrations = Vec::with_capacity(self.integrations.len());
        for integration in self.integrations.values() {
            let mut credentials = Vec::with_capacity(integration.credentials.len());
            let mut ready = true;
            for requirement in &integration.credentials {
                let state = match self.credentials.exists(&requirement.reference).await {
                    Ok(true) => CredentialState::Present,
                    Ok(false) => CredentialState::Missing,
                    Err(_) => CredentialState::Unavailable,
                };
                if requirement.required && state != CredentialState::Present {
                    ready = false;
                }
                credentials.push(AdminCredentialStatus {
                    name: requirement.name.clone(),
                    required: requirement.required,
                    state,
                });
            }
            integrations.push(AdminIntegrationStatus {
                integration_ref: integration.integration_ref.clone(),
                active: true,
                configuration: integration.configuration.clone(),
                credentials,
                ready,
            });
        }
        let ready = integrations.iter().all(|integration| integration.ready);
        AdminStatus {
            integrations,
            ready,
        }
    }

    pub async fn put(
        &self,
        actor: &str,
        request_id: &str,
        integration_ref: &str,
        credential: &str,
        input: AdminCredentialInput,
        replace: bool,
    ) -> Result<bool, AdminError> {
        input.validate()?;
        let integration = self
            .integrations
            .get(integration_ref)
            .ok_or(AdminError::NotFound)?;
        let requirement = integration
            .credentials
            .iter()
            .find(|requirement| requirement.name == credential)
            .ok_or(AdminError::NotFound)?;
        self.audit(actor, request_id, integration_ref, credential, "attempted")?;
        let existed = self
            .credentials
            .exists(&requirement.reference)
            .await
            .map_err(map_store)?;
        if existed && !replace {
            self.audit(actor, request_id, integration_ref, credential, "refused")?;
            return Err(AdminError::Conflict);
        }
        let secret = Secret::new(input.0.to_string());
        self.credentials
            .put(&requirement.reference, &secret)
            .await
            .map_err(map_store)?;
        drop(secret);
        self.audit(actor, request_id, integration_ref, credential, "completed")?;
        Ok(existed)
    }

    fn audit(
        &self,
        actor: &str,
        request_id: &str,
        integration_ref: &str,
        credential: &str,
        outcome: &'static str,
    ) -> Result<(), AdminError> {
        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct Row<'a> {
            protocol: &'static str,
            tenant_id: &'a str,
            actor: &'a str,
            request_id: &'a str,
            integration_ref: &'a str,
            credential: &'a str,
            action: &'static str,
            outcome: &'static str,
            at_unix_ms: u128,
        }
        let at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AdminError::AuditUnavailable)?
            .as_millis();
        let mut row = serde_json::to_vec(&Row {
            protocol: "b10x.connectors.admin-audit.v1",
            tenant_id: &self.tenant_id,
            actor,
            request_id,
            integration_ref,
            credential,
            action: "credential_set",
            outcome,
            at_unix_ms,
        })
        .map_err(|_| AdminError::AuditUnavailable)?;
        row.push(b'\n');
        self.state
            .append(AUDIT_KEY, &row, MAX_AUDIT_BYTES)
            .map(|_| ())
            .map_err(|_| AdminError::AuditUnavailable)
    }
}

fn map_store(_error: StoreError) -> AdminError {
    AdminError::Unavailable
}

#[cfg(test)]
mod tests {
    use connector_secrets::MemoryStore;
    use connector_state::MemoryState;

    use super::*;

    fn fixture() -> (
        AdminRegistry,
        Arc<MemoryStore>,
        Arc<MemoryState>,
        CredentialRef,
    ) {
        let credentials = Arc::new(MemoryStore::new());
        let state = Arc::new(MemoryState::new());
        let reference = CredentialRef::new(
            "tenant-one",
            "com.gitlab.api",
            "login",
            "oauth_client_secret",
        )
        .unwrap();
        let registry = AdminRegistry::new(
            "tenant-one".to_owned(),
            credentials.clone(),
            state.clone(),
            vec![AdminIntegration::new(
                "gitlab",
                vec![AdminConfigurationField::valid("oauth_client_id")],
                vec![AdminCredentialRequirement::token(
                    "oauth_client_secret",
                    true,
                    reference.clone(),
                )],
            )],
        )
        .unwrap();
        (registry, credentials, state, reference)
    }

    #[tokio::test]
    async fn status_reports_presence_without_reading_the_value() {
        let (registry, _, _, _) = fixture();
        let status = registry.status().await;
        assert!(!status.ready);
        assert_eq!(
            status.integrations[0].credentials[0].state,
            CredentialState::Missing
        );
    }

    #[tokio::test]
    async fn write_requires_explicit_replacement_and_audits_no_secret() {
        let (registry, credentials, state, reference) = fixture();
        let marker = "admin-test-secret-marker";
        assert!(!registry
            .put(
                "operator-one",
                "request-one",
                "gitlab",
                "oauth_client_secret",
                AdminCredentialInput::new(marker.to_owned()),
                false,
            )
            .await
            .unwrap());
        assert_eq!(
            credentials.get(&reference).await.unwrap().expose_secret(),
            marker
        );
        assert_eq!(
            registry
                .put(
                    "operator-one",
                    "request-two",
                    "gitlab",
                    "oauth_client_secret",
                    AdminCredentialInput::new("replacement".to_owned()),
                    false,
                )
                .await,
            Err(AdminError::Conflict)
        );
        assert!(registry
            .put(
                "operator-one",
                "request-three",
                "gitlab",
                "oauth_client_secret",
                AdminCredentialInput::new("replacement".to_owned()),
                true,
            )
            .await
            .unwrap());
        let audit = state.read(AUDIT_KEY, MAX_AUDIT_BYTES).unwrap().unwrap();
        let audit = String::from_utf8(audit).unwrap();
        assert!(audit.contains("operator-one"));
        assert!(audit.contains("completed"));
        assert!(!audit.contains(marker));
        assert!(!audit.contains("replacement"));
    }
}
