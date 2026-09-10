//! Executable composition: native configuration and protected entry terminate
//! here. The business adapter receives only an immutable authenticated HTTP port.
use connectors_gitlab::{Config, GitLab, auth};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{
            self, Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result,
        },
    },
};
use connectors_sdk::{Adapter as _, Credential, Secret};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    format: String,
    instance: String,
    api_base: String,
    ca_file: Option<PathBuf>,
    allowed_projects: Vec<String>,
}

pub struct Local {
    bootstrap: Bootstrap,
    http: Arc<ScopedHttp>,
    adapter: GitLab,
}
impl Local {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = filesystem::read_bounded(
            filesystem::private_file(path).map_err(|_| Failure::InvalidConfiguration)?,
            1024 * 1024,
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        let mut config: Configuration =
            connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        if config.format != "connectors-gitlab-local/1"
            || !connectors_core::valid_id(&config.instance)
            || config.allowed_projects.is_empty()
            || config.allowed_projects.len() > 1000
            || config
                .allowed_projects
                .iter()
                .any(|p| p.is_empty() || p.len() > 512 || p.chars().any(char::is_control))
        {
            return Err(Failure::InvalidConfiguration);
        }
        config.allowed_projects.sort();
        if config.allowed_projects.windows(2).any(|p| p[0] == p[1]) {
            return Err(Failure::InvalidConfiguration);
        }
        let base = connectors_host::http::canonical_base(&config.api_base)
            .map_err(Failure::from_service)?;
        if !base.starts_with("https://") || !base.ends_with("/api/v4/") {
            return Err(Failure::InvalidConfiguration);
        }
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let effective = json!({"format":config.format,"instance":config.instance,"api_base":base,"ca_digest":ca.as_ref().map(|b|connectors_core::digest(&json!(b))),"allowed_projects":config.allowed_projects});
        let configuration_revision = connectors_core::digest(&effective);
        let http = Arc::new(
            ScopedHttp::new_with_ca_bytes(
                &HttpConfig {
                    base_url: base.clone(),
                    credential: None,
                    credential_header: "PRIVATE-TOKEN".into(),
                    bearer: false,
                    allow_plaintext: false,
                    ca_file: None,
                },
                None,
                ca.as_deref(),
            )
            .map_err(Failure::from_service)?,
        );
        // This template has no credential and performs no provider work.
        let adapter = GitLab::new(
            &config.instance,
            Config {
                allowed_projects: config.allowed_projects,
            },
            effective,
            http.clone(),
        )
        .map_err(Failure::from_service)?;
        let mut profile = Profile {
            id: auth::PROFILE_ID.into(),
            revision: String::new(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::from(["read_api".into()]),
            evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
            fields: vec![EntryField {
                name: "token".into(),
                label: "GitLab personal access token".into(),
                max_bytes: 8192,
            }],
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "gitlab".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision,
            provider_authority: base,
            descriptor: serde_json::to_string(&descriptor).map_err(|_| Failure::Protocol)?,
            profiles: vec![profile],
            requirements: descriptor
                .operations
                .iter()
                .map(|o| Requirement {
                    operation: o.id.clone(),
                    profile: auth::PROFILE_ID.into(),
                    scopes: BTreeSet::from(["read_api".into()]),
                    effect: match o.id.as_str() {
                        "project.get" | "issues.list" | "file.get" | "pipelines.list"
                        | "pipeline.get" | "pipeline.jobs" | "job.get" | "job.trace" => {
                            Effect::Read
                        }
                        _ => Effect::Unknown,
                    },
                })
                .collect(),
        };
        bootstrap.validate()?;
        Ok(Self {
            bootstrap,
            http,
            adapter,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    fn authenticated(&self, mut document: Secret) -> Result<ScopedHttp> {
        let token = auth::ProtectedEntry::parse(std::mem::take(&mut document.0))
            .map_err(native_failure)?
            .into_secret();
        Ok(self.http.with_credential(Arc::new(Fixed(token))))
    }
}
struct Fixed(Secret);
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.0.clone()))
    }
}

#[async_trait::async_trait]
impl runtime::Adapter for Local {
    fn bootstrap(&self) -> Bootstrap {
        self.bootstrap.clone()
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline> {
        if profile != auth::PROFILE_ID {
            return Err(Failure::Unsupported);
        }
        let http = self.authenticated(document)?;
        let value = auth::validate(&http, connectors_sdk::now_ms)
            .await
            .map_err(native_failure)?;
        Ok(Baseline {
            identity: registry::ExternalIdentity {
                kind: "gitlab.user".into(),
                subject: value.user_id.to_string(),
            },
            granted_scopes: Some(value.granted_scopes),
            credential_expires_at_ms: value.expires_at_ms,
            collected_at_ms: value.collected_at_ms,
            valid_until_ms: value.valid_until_ms,
        })
    }
    async fn invoke(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value> {
        let http = self.authenticated(document)?;
        self.adapter
            .with_authenticated_http(Arc::new(http), partition)
            .map_err(Failure::from_service)?
            .invoke(operation, input)
            .await
            .map_err(Failure::from_provider)
    }
}
fn native_failure(value: auth::Failure) -> Failure {
    match value {
        auth::Failure::InvalidEntry => Failure::InvalidInput,
        auth::Failure::InvalidCredential => Failure::InvalidCredential,
        auth::Failure::InsufficientScope => Failure::InsufficientScope,
        auth::Failure::IdentityMismatch => Failure::IdentityMismatch,
        auth::Failure::InvalidResponse => Failure::Protocol,
        auth::Failure::Unavailable => Failure::Unavailable,
        auth::Failure::PermissionDenied => Failure::Forbidden,
        auth::Failure::Expired => Failure::InvalidCredential,
        auth::Failure::Deadline => Failure::Timeout,
    }
}
