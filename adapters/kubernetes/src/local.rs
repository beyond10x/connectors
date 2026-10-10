//! Executable composition: native configuration and protected entry terminate
//! here. The business adapter receives only an immutable authenticated HTTP
//! read port. The SelfSubjectReview identity probe is a separate check-specific
//! capability whose endpoint this module fixes once, so business code cannot
//! reach it or POST anything else.
//!
//! `pods.exec`, where configured, is the one mutation: it is listed only on
//! the `connectors-private/2` write exchange, and its prepared write holds the
//! host's consuming write capability built for one upgraded stream, so the
//! WebSocket is opened only when the host commits a write it admitted.
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{
            Baseline, Bootstrap, Effect, EntryField, Failure, PreparedWrite, PrivateProtocol,
            Profile, Requirement, Result,
        },
    },
};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes, auth, exec};
use connectors_sdk::{
    Adapter as _, AuthProbe, AuthenticatedWrite, Credential, MessageStream, Secret, UpgradeBounds,
    Upgraded, WriteOutcome,
};
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
    /// Advertises `pods.logs`. Absent means false.
    #[serde(default)]
    pod_logs: bool,
    /// Advertises `pods.exec`, a mutation, on the `connectors-private/2`
    /// write exchange only. Absent means false. The read exchange never lists
    /// it; the write runs over the host's upgraded-stream write capability.
    #[serde(default)]
    pod_exec: bool,
    /// The absolute path of the kubeconfig whose contexts `contexts.list`
    /// reads. Its digest is part of the configuration revision the host
    /// admitted, and the operation takes no path, so a connection reads the
    /// file it was configured with and no other.
    #[serde(default)]
    kubeconfig: Option<PathBuf>,
}

pub struct Local {
    bootstrap: Bootstrap,
    /// Present only when the configuration admitted `pods.exec`.
    bootstrap_v2: Option<Bootstrap>,
    http: Arc<ScopedHttp>,
    adapter: Kubernetes,
    kubeconfig: Option<PathBuf>,
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
        // The file is read once here so a path that is not an owner-only
        // kubeconfig refuses the configuration rather than a later read. The
        // bytes are dropped zeroized; only the path is kept.
        if let Some(path) = &config.kubeconfig {
            let bytes = zeroize::Zeroizing::new(
                filesystem::read_bounded(
                    filesystem::private_file(path).map_err(|_| Failure::InvalidConfiguration)?,
                    connectors_kubernetes::kubeconfig::MAX_FILE_BYTES,
                )
                .map_err(|_| Failure::InvalidConfiguration)?,
            );
            connectors_kubernetes::kubeconfig::contexts(&bytes)
                .map_err(|_| Failure::InvalidConfiguration)?;
        }
        let mut effective = json!({"format":config.format,"instance":config.instance,"api_base":base,
            "ca_digest":ca.as_ref().map(|b|connectors_core::digest(&json!(b))),
            "namespaces":config.namespaces,"resource_kinds":config.resource_kinds,
            "discover_hosts":config.discover_hosts,
            "helm_release_reads":config.helm_release_reads});
        // Each new field enters the effective document only when it enables
        // something, so a configuration that names none of them keeps the
        // revision it had before they existed.
        if config.pod_logs {
            effective["pod_logs"] = json!(true);
        }
        if config.pod_exec {
            effective["pod_exec"] = json!(true);
        }
        if let Some(path) = &config.kubeconfig {
            let path = path.to_str().ok_or(Failure::InvalidConfiguration)?;
            effective["kubeconfig_path_digest"] = json!(connectors_core::digest(&json!(path)));
        }
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
                pod_logs: config.pod_logs,
                pod_exec: config.pod_exec,
                kubeconfig_contexts: config.kubeconfig.is_some(),
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
            acquisition: None,
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let full = adapter.descriptor();
        let requirement = |o: &connectors_core::Operation| Requirement {
            operation: o.id.clone(),
            profile: auth::PROFILE_ID.into(),
            scopes: BTreeSet::new(),
            effect: match o.id.as_str() {
                "resources.list"
                | "resources.get"
                | "namespaces.list"
                | "deployments.history"
                | "endpoints.discover"
                | "hosts.discover"
                | "helm_releases.history"
                | "helm_releases.status"
                | "helm_releases.values"
                | "helm_releases.manifest"
                | "pods.logs"
                | "contexts.list" => Effect::Read,
                "pods.exec" => Effect::Write,
                _ => Effect::Unknown,
            },
        };
        // The read exchange lists every operation but the mutation; it keeps
        // the revision the adapter derived, so a configuration without
        // pod_exec describes exactly what it described before.
        let mut reads = full.clone();
        reads.operations.retain(|o| o.id != "pods.exec");
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "kubernetes".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision,
            provider_authority: base,
            descriptor: serde_json::to_string(&reads).map_err(|_| Failure::Protocol)?,
            profiles: vec![profile],
            requirements: reads.operations.iter().map(requirement).collect(),
        };
        // A kind this composition does not classify leaves Effect::Unknown, and
        // the read-only protocol refuses the whole bootstrap rather than
        // advertising an operation whose effect nobody stated.
        bootstrap.validate_for(PrivateProtocol::V1)?;
        // The write exchange exists only where pod_exec admitted the mutation.
        // Its full projection is revised separately from the read one, so a
        // revision from either exchange is stale on the other.
        let bootstrap_v2 = if config.pod_exec {
            let mut all = full;
            all.revision = connectors_core::digest(
                &json!({"descriptor": reads.revision, "private_protocol": "connectors-private/2"}),
            );
            let mut bootstrap_v2 = bootstrap.clone();
            bootstrap_v2.descriptor = serde_json::to_string(&all).map_err(|_| Failure::Protocol)?;
            bootstrap_v2.requirements = all.operations.iter().map(requirement).collect();
            bootstrap_v2.validate_for(PrivateProtocol::V2)?;
            Some(bootstrap_v2)
        } else {
            None
        };
        Ok(Self {
            bootstrap,
            bootstrap_v2,
            http,
            adapter,
            kubeconfig: config.kubeconfig,
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
/// The largest single exec message the host admits: well above the 64 KiB
/// status bound and any stdout or stderr chunk an API server sends.
const EXEC_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
/// The host's total bound sits above the adapter's own consumable bound
/// (`exec::MAX_CONSUMED_BYTES`), so an over-long exec is reported by the
/// adapter and the host's bound is the backstop. Both outcomes are unknown.
const EXEC_MAX_STREAM_BYTES: usize = exec::MAX_CONSUMED_BYTES + EXEC_MAX_MESSAGE_BYTES;

/// One prepared `pods.exec`: the admitted request and the consuming write
/// capability for its one upgraded stream. No Clone and no retry.
struct Exec {
    prepared: exec::Prepared,
    write: Box<dyn AuthenticatedWrite>,
}
#[async_trait::async_trait]
impl PreparedWrite for Exec {
    async fn execute(self: Box<Self>) -> WriteOutcome<Value> {
        let Exec { prepared, write } = *self;
        let upgraded = {
            let segments = prepared.segments();
            let query = prepared.query();
            write.upgrade(&segments, &query, &exec::PROTOCOLS).await
        };
        let upgrade = match upgraded {
            Upgraded::NotSent(error) => exec::Upgrade::NotSent(error),
            Upgraded::Answered(status) => exec::Upgrade::Answered(status),
            Upgraded::Lost(error) => exec::Upgrade::Lost(error),
            Upgraded::Accepted(stream) => exec::Upgrade::Accepted(Box::new(Stream(stream))),
        };
        exec::settle(prepared, upgrade).await
    }
}
/// The host's message stream as the adapter's exec port.
struct Stream(Box<dyn MessageStream>);
#[async_trait::async_trait]
impl exec::ExecStream for Stream {
    fn protocol(&self) -> &str {
        self.0.protocol()
    }
    async fn next_message(&mut self) -> connectors_core::Result<Option<Vec<u8>>> {
        self.0.next_message().await
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
    fn bootstrap_v2(&self) -> Result<Bootstrap> {
        self.bootstrap_v2.clone().ok_or(Failure::Unsupported)
    }
    /// Every refusal here precedes any I/O: the namespace, the input bounds
    /// and the protected entry are checked, and the prepared write only holds
    /// the capability. The upgrade happens in `execute`, after the host
    /// committed the write it admitted.
    async fn prepare_write(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Box<dyn PreparedWrite>> {
        if operation != "pods.exec" || self.bootstrap_v2.is_none() {
            drop(document);
            return Err(Failure::Unsupported);
        }
        let prepared = self
            .adapter
            .prepare_exec(input)
            .map_err(Failure::from_provider)?;
        let bounds = UpgradeBounds::new(
            EXEC_MAX_MESSAGE_BYTES,
            EXEC_MAX_STREAM_BYTES,
            prepared.timeout(),
        )
        .map_err(|_| Failure::Protocol)?;
        let write = self.authenticated(document)?.into_upgrade_write(bounds);
        Ok(Box::new(Exec { prepared, write }))
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
        // Answered from the one file this configuration fixed, read now so a
        // changed current-context shows. No provider request is made and the
        // credential is not used.
        if operation == "contexts.list" {
            drop(document);
            let path = self.kubeconfig.as_ref().ok_or(Failure::Forbidden)?;
            let bytes = zeroize::Zeroizing::new(
                filesystem::read_bounded(
                    filesystem::private_file(path).map_err(|_| Failure::Unavailable)?,
                    connectors_kubernetes::kubeconfig::MAX_FILE_BYTES,
                )
                .map_err(|_| Failure::Unavailable)?,
            );
            return self
                .adapter
                .contexts_page(&bytes, input)
                .map_err(Failure::from_provider);
        }
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
