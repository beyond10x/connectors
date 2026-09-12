//! Executable composition: native configuration and protected entry terminate
//! here. The business adapter receives only an immutable authenticated HTTP
//! read port. The SelfSubjectReview identity probe is a separate check-specific
//! capability whose endpoint this module fixes once, so business code cannot
//! reach it or POST anything else.
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result},
    },
};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes, auth};
use connectors_sdk::{Adapter as _, AuthProbe, Credential, Secret};
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
    namespaces: Vec<String>,
    resource_kinds: Vec<String>,
    #[serde(default)]
    discover_hosts: bool,
    /// Absent means no Helm release read is advertised at all. Disclosure of a
    /// release's recorded values or rendered manifest is a separate, explicit
    /// step above that, and even then only as a redacted projection.
    #[serde(default)]
    helm_release_reads: HelmReleaseReads,
}

pub struct Local {
    bootstrap: Bootstrap,
    http: Arc<ScopedHttp>,
    adapter: Kubernetes,
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
        if config.format != "connectors-kubernetes-local/1"
            || !connectors_core::valid_id(&config.instance)
            || config.namespaces.is_empty()
            || config.namespaces.len() > 256
            || config.resource_kinds.is_empty()
            || config.resource_kinds.len() > 16
        {
            return Err(Failure::InvalidConfiguration);
        }
        // A duplicate entry would multiply the configured scope without changing
        // it, so the revision must not depend on how it was written down.
        config.namespaces.sort();
        config.resource_kinds.sort();
        if config.namespaces.windows(2).any(|n| n[0] == n[1])
            || config.resource_kinds.windows(2).any(|k| k[0] == k[1])
        {
            return Err(Failure::InvalidConfiguration);
        }
        let base = connectors_host::http::canonical_base(&config.api_base)
            .map_err(Failure::from_service)?;
        if !base.starts_with("https://") {
            return Err(Failure::InvalidConfiguration);
        }
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let effective = json!({"format":config.format,"instance":config.instance,"api_base":base,
            "ca_digest":ca.as_ref().map(|b|connectors_core::digest(&json!(b))),
            "namespaces":config.namespaces,"resource_kinds":config.resource_kinds,
            "discover_hosts":config.discover_hosts,
            "helm_release_reads":config.helm_release_reads});
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
        // Build the probe port once here so a fixed endpoint this composition
        // cannot express is refused before any credential is accepted.
        http.probe_capability(&auth::REVIEW_ENDPOINT)
            .map_err(Failure::from_service)?;
        // This template has no credential and performs no provider work.
        let adapter = Kubernetes::new(
            &config.instance,
            Config {
                namespaces: config.namespaces,
                resource_kinds: config.resource_kinds,
                discover_hosts: config.discover_hosts,
                helm_release_reads: config.helm_release_reads,
            },
            effective,
            http.clone(),
        )
        .map_err(Failure::from_service)?;
        // Kubernetes authorization is RBAC on the authenticated name, not a
        // scope grant carried by the credential. An empty required set is the
        // truthful representation; it is not a claim that any read is permitted.
        let mut profile = Profile {
            id: auth::PROFILE_ID.into(),
            revision: String::new(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
            fields: vec![EntryField {
                name: "token".into(),
                label: "Kubernetes bearer token".into(),
                max_bytes: 8192,
            }],
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "kubernetes".into(),
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
                        "resources.list"
                        | "endpoints.discover"
                        | "hosts.discover"
                        | "helm_releases.history"
                        | "helm_releases.status"
                        | "helm_releases.values"
                        | "helm_releases.manifest" => Effect::Read,
                        _ => Effect::Unknown,
                    },
                })
                .collect(),
        };
        // A kind this composition does not classify leaves Effect::Unknown, and
        // the read-only protocol refuses the whole bootstrap rather than
        // advertising an operation whose effect nobody stated.
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
    fn probe_with(&self, mut document: Secret) -> Result<Arc<dyn AuthProbe>> {
        let token = auth::ProtectedEntry::parse(std::mem::take(&mut document.0))
            .map_err(native_failure)?
            .into_secret();
        self.http
            .with_credential(Arc::new(Fixed(token)))
            .probe_capability(&auth::REVIEW_ENDPOINT)
            .map_err(Failure::from_service)
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
        let probe = self.probe_with(document)?;
        let value = auth::validate(probe.as_ref(), connectors_sdk::now_ms)
            .await
            .map_err(native_failure)?;
        Ok(Baseline {
            identity: registry::ExternalIdentity {
                kind: "kubernetes.user".into(),
                subject: value.username,
            },
            // Kubernetes issues no scope grant; see the profile above.
            granted_scopes: None,
            // A bound token's expiry is inside the credential, which this
            // module does not decode. Absent means not observed, not unlimited.
            credential_expires_at_ms: None,
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
        auth::Failure::InvalidResponse => Failure::Protocol,
        auth::Failure::Unavailable => Failure::Unavailable,
        auth::Failure::PermissionDenied => Failure::Forbidden,
        auth::Failure::Deadline => Failure::Timeout,
    }
}
