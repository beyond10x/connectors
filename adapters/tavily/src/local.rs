//! Executable composition: native configuration and protected entry terminate
//! here. The business adapter receives only an authenticated HTTP port whose
//! read POSTs this module fixes once, to `READ_POSTS`, so business code can
//! POST nowhere else and holds no write capability.
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result},
    },
};
use connectors_sdk::{Adapter as _, Credential, Secret};
use connectors_tavily::{READ_POST_TIMEOUT, READ_POSTS, Tavily, auth};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path, sync::Arc};

pub const DEFAULT_API_BASE: &str = "https://api.tavily.com/";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    format: String,
    instance: String,
    #[serde(default = "default_api_base")]
    api_base: String,
}
fn default_api_base() -> String {
    DEFAULT_API_BASE.into()
}

pub struct Local {
    bootstrap: Bootstrap,
    http: Arc<ScopedHttp>,
    adapter: Tavily,
}
impl Local {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = filesystem::read_bounded(
            filesystem::private_file(path).map_err(|_| Failure::InvalidConfiguration)?,
            64 * 1024,
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        let config: Configuration =
            connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        if config.format != "connectors-tavily-local/1"
            || !connectors_core::valid_id(&config.instance)
        {
            return Err(Failure::InvalidConfiguration);
        }
        let base = connectors_host::http::canonical_base(&config.api_base)
            .map_err(Failure::from_service)?;
        if !base.starts_with("https://") {
            return Err(Failure::InvalidConfiguration);
        }
        let effective =
            json!({"format": config.format, "instance": config.instance, "api_base": base});
        let configuration_revision = connectors_core::digest(&effective);
        let http = Arc::new(
            ScopedHttp::new_with_ca_bytes(
                &HttpConfig {
                    base_url: base.clone(),
                    credential: None,
                    credential_header: "authorization".into(),
                    bearer: true,
                    allow_plaintext: false,
                    ca_file: None,
                },
                None,
                None,
            )
            .map_err(Failure::from_service)?
            .with_read_posts(&READ_POSTS, READ_POST_TIMEOUT)
            .map_err(Failure::from_service)?,
        );
        // This template has no credential and performs no provider work.
        let adapter = Tavily::new(&config.instance, &effective, http.clone())
            .map_err(Failure::from_service)?;
        let mut profile = Profile {
            id: auth::PROFILE_ID.into(),
            revision: String::new(),
            purpose: registry::Purpose::AppLevel,
            subject: registry::Subject::App,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            // Tavily grants no scopes; a key reaches every endpoint its plan has.
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
            fields: vec![EntryField {
                name: "api_key".into(),
                label: "Tavily API key".into(),
                max_bytes: 512,
            }],
            acquisition: None,
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "tavily".into(),
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
                    scopes: BTreeSet::new(),
                    // Each is a read Tavily serves as a POST; see
                    // contracts/websearch/v1alpha1/semantics.md beside this adapter.
                    effect: match o.id.as_str() {
                        "websearch.search" | "websearch.fetch" | "websearch.crawl" => Effect::Read,
                        _ => Effect::Unknown,
                    },
                })
                .collect(),
        };
        bootstrap.validate_for(connectors_host::local::runtime::PrivateProtocol::V1)?;
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
        let key = auth::ProtectedEntry::parse(std::mem::take(&mut document.0))
            .map_err(native_failure)?
            .into_secret();
        Ok(self.http.with_credential(Arc::new(Fixed(key))))
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
impl connectors_host::local::runtime::Adapter for Local {
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
            // Tavily exposes no account identifier; see auth.rs.
            identity: registry::ExternalIdentity {
                kind: "tavily.api-key".into(),
                subject: value.plan.unwrap_or_else(|| "api-key".into()),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: value.collected_at_ms,
            valid_until_ms: value.valid_until_ms,
        })
    }
    async fn invoke(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value> {
        let http = self.authenticated(document)?;
        self.adapter
            .with_authenticated_http(Arc::new(http))
            .invoke(operation, input)
            .await
            .map_err(Failure::from_provider)
    }
}
fn native_failure(value: auth::Failure) -> Failure {
    match value {
        auth::Failure::InvalidEntry => Failure::InvalidInput,
        auth::Failure::InvalidCredential => Failure::InvalidCredential,
        auth::Failure::InvalidResponse => Failure::Protocol,
        auth::Failure::Unavailable => Failure::Unavailable,
        auth::Failure::Deadline => Failure::Timeout,
    }
}
