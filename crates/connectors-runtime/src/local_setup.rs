//! Owner-only setup transactions. The daemon owns credential custody and provider acquisition.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use connector_secrets::{Secret, SecretStore};
use connectors_config::{
    CatalogIntegrationConfig, InitiationConfig, NetworkScopeConfig, PersonalConfig,
};
use protocol::local_setup::{CredentialSource, EnrollRequest, SetupError};
use protocol::operation::OwnerContext;
use serde_json::{json, Value};
use server::local_setup::LocalSetupHandler;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub(crate) struct PersonalSetup {
    config_path: PathBuf,
    owner: OwnerContext,
    store: Arc<dyn SecretStore>,
    backend: String,
    transaction: tokio::sync::Mutex<()>,
}

impl PersonalSetup {
    pub(crate) fn new(
        config_path: PathBuf,
        owner: OwnerContext,
        store: Arc<dyn SecretStore>,
        backend: String,
    ) -> Self {
        Self {
            config_path,
            owner,
            store,
            backend,
            transaction: tokio::sync::Mutex::new(()),
        }
    }

    fn configuration(&self) -> Result<(PersonalConfig, Zeroizing<Vec<u8>>), SetupError> {
        let bytes = trusted_bytes(&self.config_path, false, 256 * 1024)?;
        let config: PersonalConfig =
            toml::from_str(std::str::from_utf8(&bytes).map_err(|_| SetupError::Configuration)?)
                .map_err(|_| SetupError::Configuration)?;
        config.validate().map_err(|_| SetupError::Configuration)?;
        if config.owner_context() != self.owner {
            return Err(SetupError::Configuration);
        }
        Ok((config, bytes))
    }
}

#[async_trait]
impl LocalSetupHandler for PersonalSetup {
    async fn enroll(&self, request: EnrollRequest) -> Result<Value, SetupError> {
        let _guard = self.transaction.lock().await;
        if !request.valid() {
            return Err(SetupError::InvalidInput);
        }
        let (mut config, previous) = self.configuration()?;
        let provider = catalog::provider(catalog::ProviderKey::id(&request.provider))
            .ok_or(SetupError::InvalidInput)?;
        let authority = provider.authority.ok_or(SetupError::InvalidInput)?;
        let credential = provider
            .auth
            .iter()
            .find(|credential| credential.name == request.credential)
            .ok_or(SetupError::InvalidInput)?;
        let endpoint_keys: std::collections::BTreeSet<_> = provider
            .config
            .iter()
            .filter_map(|field| field.binds.strip_prefix("endpoint."))
            .collect();
        let username_keys: std::collections::BTreeSet<_> = provider
            .config
            .iter()
            .filter_map(|field| field.binds.strip_prefix("username."))
            .collect();
        if request
            .endpoints
            .keys()
            .any(|key| !endpoint_keys.contains(key.as_str()))
            || request
                .usernames
                .keys()
                .any(|key| !username_keys.contains(key.as_str()))
        {
            return Err(SetupError::InvalidInput);
        }
        let identity = request.instance.as_deref().unwrap_or(&request.provider);
        let index = config
            .catalog
            .iter()
            .position(|entry| entry.provider == request.provider && entry.instance() == identity);
        let added = index.is_some();
        let entry = if let Some(index) = index {
            let entry = &mut config.catalog[index];
            // Adding a credential must preserve the existing route and grant ceiling.
            if request
                .endpoints
                .iter()
                .any(|(key, value)| entry.endpoints.get(key) != Some(value))
                || (request.allow_writes && !entry.allow_writes)
                || (request.operator_network
                    && !matches!(entry.network, NetworkScopeConfig::Operator))
            {
                return Err(SetupError::Configuration);
            }
            entry.usernames.extend(request.usernames.clone());
            entry.clone()
        } else {
            let entry = CatalogIntegrationConfig {
                provider: request.provider.clone(),
                instance: request.instance.clone(),
                label: None,
                grant_ref: format!("grant:{}:local", request.provider),
                initiation: InitiationConfig::Platform,
                allow_writes: request.allow_writes,
                endpoints: request.endpoints.clone(),
                usernames: request.usernames.clone(),
                operator_approved: true,
                credential: Some(request.credential.clone()),
                network: if request.operator_network {
                    NetworkScopeConfig::Operator
                } else {
                    NetworkScopeConfig::Public
                },
                credential_file: None,
                oauth: None,
            };
            config.catalog.push(entry.clone());
            entry
        };
        config.validate().map_err(|_| SetupError::Configuration)?;
        // Prove the declared origin shape before provider acquisition or custody changes.
        integration_catalog::admitted_origins(&entry).map_err(|_| SetupError::Configuration)?;
        let reference = integration_catalog::credential_address(
            &config.owner.tenant_id,
            authority,
            &entry,
            credential.leaf,
        )
        .map_err(|_| SetupError::InvalidInput)?;
        let next = toml::to_string_pretty(&config).map_err(|_| SetupError::Configuration)?;
        if next.len() > 256 * 1024 {
            return Err(SetupError::Configuration);
        }
        let (value, acquired) = match request.source {
            CredentialSource::Pasted { value } => {
                (Zeroizing::new(value.expose().trim().to_owned()), None)
            }
            CredentialSource::File { path } => {
                let bytes = trusted_bytes(&path, true, 32 * 1024)?;
                let value = std::str::from_utf8(&bytes).map_err(|_| SetupError::InvalidInput)?;
                (Zeroizing::new(value.trim().to_owned()), None)
            }
            CredentialSource::Argocd {
                username,
                password,
                project,
                role,
                expires_in_seconds,
            } => {
                if provider.id != "argocd" {
                    return Err(SetupError::InvalidInput);
                }
                let origin = entry
                    .endpoints
                    .get("origin")
                    .ok_or(SetupError::InvalidInput)?
                    .clone();
                let acquire = integration_catalog::argocd::AcquireRequest {
                    origin,
                    username,
                    password: Zeroizing::new(password.expose().into()),
                    project,
                    role,
                    allow_sync: entry.allow_writes,
                    expires_in_seconds,
                };
                let (value, report) = crate::composition::acquire_argocd_token(
                    acquire,
                    matches!(entry.network, NetworkScopeConfig::Operator),
                )
                .await
                .map_err(|_| SetupError::OutcomeUnknown)?;
                let report = json!({"project":report.project,"role":report.role,"role_created":report.role_created,
                    "token_id":report.token_id,"policies":report.policies,"expires_in_days":report.expires_in_seconds / 86400});
                (value, Some(report))
            }
        };
        if value.is_empty() {
            return Err(SetupError::InvalidInput);
        }
        // Refuse concurrent owner edits before changing credentials. Exact prior bytes are retained.
        if trusted_bytes(&self.config_path, false, 256 * 1024)?.as_slice() != previous.as_slice() {
            return Err(SetupError::Configuration);
        }
        let backup = backup_configuration(&self.config_path, &previous)?;
        self.store
            .put(&reference, &Secret::new(value.as_str()))
            .await
            .map_err(|_| SetupError::Unavailable)?;
        drop(value);
        publish_configuration(&self.config_path, &previous, next.as_bytes())
            .map_err(|_| SetupError::OutcomeUnknown)?;
        Ok(
            json!({"provider":request.provider,"name":identity,"credential":request.credential,
            "store":self.backend,"added_to_existing_identity":added,"verify":provider.verify,
            "operations":provider.operations.len(),"reload_required":true,"backup":backup,"acquired":acquired}),
        )
    }

    async fn auth_status(&self) -> Result<Value, SetupError> {
        let (config, _) = self.configuration()?;
        let mut providers = Vec::new();
        for entry in &config.catalog {
            let Some(provider) = catalog::provider(catalog::ProviderKey::id(&entry.provider))
            else {
                providers.push(json!({"provider":entry.provider,"instance":entry.instance(),"status":"unknown-provider"}));
                continue;
            };
            let Some(authority) = provider.authority else {
                continue;
            };
            let mut credentials = Vec::new();
            let mut stored = std::collections::BTreeSet::new();
            for declared in provider.auth {
                let reference = integration_catalog::credential_address(
                    &config.owner.tenant_id,
                    authority,
                    entry,
                    declared.leaf,
                )
                .map_err(|_| SetupError::Configuration)?;
                let user_half = matches!(declared.acquire, catalog::Acquisition::BasicJoin { .. })
                    .then(|| {
                        entry
                            .usernames
                            .get(declared.name)
                            .is_some_and(|value| !value.trim().is_empty())
                    });
                let state = match self.store.exists(&reference).await {
                    Ok(true) if user_half == Some(false) => "stored-without-user-half",
                    Ok(true) => {
                        stored.insert(declared.name);
                        "stored"
                    }
                    Ok(false) => "absent",
                    Err(_) => "unavailable",
                };
                let subject = match declared.subject {
                    catalog::Subject::App => "app",
                    catalog::Subject::User => "user",
                    catalog::Subject::Unstated => "unstated",
                };
                credentials.push(json!({"credential":declared.name,"subject":subject,"state":state,"user_half_configured":user_half}));
            }
            let mechanisms: std::collections::BTreeSet<Vec<&str>> = provider
                .operations
                .iter()
                .flat_map(|operation| operation.credentials.iter())
                .map(|mechanism| mechanism.to_vec())
                .collect();
            let satisfied: Vec<_> = mechanisms
                .iter()
                .filter(|mechanism| mechanism.iter().all(|name| stored.contains(name)))
                .map(|mechanism| mechanism.join(" + "))
                .collect();
            providers.push(json!({"provider":entry.provider,"instance":entry.instance(),"credentials":credentials,
                "status":if satisfied.is_empty(){"not-callable"}else{"callable"},"satisfied_mechanisms":satisfied,"verify":provider.verify}));
        }
        Ok(json!({"store":self.backend,"providers":providers}))
    }
}

fn trusted_bytes(
    path: &Path,
    confidential: bool,
    maximum: u64,
) -> Result<Zeroizing<Vec<u8>>, SetupError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| SetupError::Configuration)?;
    let metadata = file.metadata().map_err(|_| SetupError::Configuration)?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & (if confidential { 0o077 } else { 0o022 }) != 0
        || metadata.len() > maximum
    {
        return Err(SetupError::Configuration);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    (&mut file)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| SetupError::Configuration)?;
    if bytes.len() as u64 > maximum {
        return Err(SetupError::Configuration);
    }
    Ok(bytes)
}

fn backup_configuration(path: &Path, previous: &[u8]) -> Result<PathBuf, SetupError> {
    let backup = path.with_extension(format!(
        "{}.bak",
        &hex::encode(Sha256::digest(previous))[..16]
    ));
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&backup)
    {
        Ok(mut file) => {
            file.write_all(previous)
                .and_then(|_| file.sync_all())
                .map_err(|_| SetupError::Configuration)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if trusted_bytes(&backup, true, 256 * 1024)?.as_slice() != previous {
                return Err(SetupError::Configuration);
            }
        }
        Err(_) => return Err(SetupError::Configuration),
    }
    Ok(backup)
}

fn publish_configuration(path: &Path, previous: &[u8], next: &[u8]) -> Result<(), SetupError> {
    let parent = path.parent().ok_or(SetupError::Configuration)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| SetupError::Configuration)?;
    temporary
        .write_all(next)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| SetupError::Configuration)?;
    if trusted_bytes(path, false, 256 * 1024)?.as_slice() != previous {
        return Err(SetupError::Configuration);
    }
    temporary
        .persist(path)
        .map_err(|_| SetupError::Configuration)?;
    std::fs::File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|_| SetupError::Configuration)
}

#[cfg(test)]
#[path = "local_setup_tests.rs"]
mod tests;
