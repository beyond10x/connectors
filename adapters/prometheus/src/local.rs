//! Executable composition: the native configuration
//! (`connectors_prometheus.connection.LocalConfiguration`) and the protected entry of
//! `prometheus.bearer` terminate here. The business adapter receives only an
//! immutable authenticated HTTP read port; every Prometheus operation is a GET.
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result},
    },
};
use connectors_prometheus::{Prometheus, auth};
use connectors_sdk::{Adapter as _, Credential, Secret};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const FORMAT: &str = "connectors-prometheus-local/1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryScope {
    allowed_matchers: Vec<Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    format: String,
    instance: String,
    base_url: String,
    /// Absent or a path; an explicit `null` is refused, as the model's
    /// `Optional<String>` refuses it.
    #[serde(default, deserialize_with = "present")]
    ca_file: Option<PathBuf>,
    query_scope: QueryScope,
}
fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<PathBuf>, D::Error> {
    PathBuf::deserialize(deserializer).map(Some)
}

pub struct Local {
    bootstrap: Bootstrap,
    http: Arc<ScopedHttp>,
    adapter: Prometheus,
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
        if config.format != FORMAT || !connectors_core::valid_id(&config.instance) {
            return Err(Failure::InvalidConfiguration);
        }
        // A nonempty scope needs a PromQL parser that does not exist (§11.3); the
        // descriptor's configuration schema refuses it below as well.
        if !config.query_scope.allowed_matchers.is_empty() {
            return Err(Failure::InvalidConfiguration);
        }
        // The model (`LocalConfiguration`) admits only a literal `https://` URL of at most 512
        // characters; this composition further requires it to be written in its canonical
        // form (ESS-LIMIT in connection.yaml), so nothing the model refuses is admitted here.
        let base = connectors_host::http::canonical_base(&config.base_url)
            .map_err(Failure::from_service)?;
        if !config.base_url.starts_with("https://")
            || config.base_url.chars().count() > 512
            || base != config.base_url
        {
            return Err(Failure::InvalidConfiguration);
        }
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let effective = json!({"format": config.format, "instance": config.instance,
            "base_url": base,
            "ca_digest": ca.as_ref().map(|b| connectors_core::digest(&json!(b))),
            "query_scope": {"allowed_matchers": []}});
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
                ca.as_deref(),
            )
            .map_err(Failure::from_service)?,
        );
        // This template has no credential and performs no provider work.
        let adapter = Prometheus::new(&config.instance, &effective, http.clone())
            .map_err(|_| Failure::InvalidConfiguration)?;
        // A static token of a deployment's (or its Grafana's) service account.
        // Prometheus grants no scopes; its gateway decides what a token reaches.
        let mut profile = Profile {
            id: auth::PROFILE_ID.into(),
            revision: String::new(),
            purpose: registry::Purpose::ServiceAccount,
            subject: registry::Subject::App,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
            fields: vec![EntryField {
                name: "token".into(),
                label: "Prometheus bearer token".into(),
                max_bytes: auth::TOKEN_BYTES as u32,
            }],
            acquisition: None,
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "prometheus".into(),
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
                    effect: match o.id.as_str() {
                        "series.query" | "series.query_range" | "rules.list" => Effect::Read,
                        _ => Effect::Unknown,
                    },
                })
                .collect(),
        };
        // An operation this composition does not classify leaves Effect::Unknown,
        // and the read-only protocol refuses the whole bootstrap.
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
            // Prometheus has no user or account read; the subject is the configured
            // connection (identity source `configuration`, see auth.rs).
            identity: registry::ExternalIdentity {
                kind: auth::IDENTITY_KIND.into(),
                subject: self.bootstrap.instance.clone(),
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
