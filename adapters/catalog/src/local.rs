//! Executable composition for one provider bundle: native configuration and
//! protected entry terminate here. The engine receives only an immutable
//! authenticated HTTP port; the host keeps admission, approval and the ledger.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{self, Baseline, Bootstrap, EntryField, Failure, Profile, Requirement, Result},
    },
};
use connectors_sdk::{AuthenticatedHttp as _, Credential, Secret};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};
use zeroize::Zeroizing;

pub const FORMAT: &str = "connectors-catalog-local/2";
/// A selection set shipped with the repository, referenced by `operations_file`.
pub const OPERATIONS_FORMAT: &str = "connectors-catalog-operations/1";
const DOCUMENT_LIMIT: usize = 64 * 1024;

/// A read the profile performs to learn who the credential is, and optionally
/// which scopes it was granted. Paths are relative to the provider authority.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityProbe {
    path: String,
    kind: String,
    subject_pointer: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopesProbe {
    path: String,
    pointer: String,
}
/// The declarative authentication profile: where the token goes and how the
/// provider is asked who holds it. No credential value lives here.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthConfig {
    profile: String,
    header: String,
    bearer: bool,
    label: String,
    identity: IdentityProbe,
    #[serde(default)]
    scopes: Option<ScopesProbe>,
    #[serde(default)]
    minimum_scopes: BTreeSet<String>,
    #[serde(default = "default_evidence_lifetime")]
    evidence_lifetime_ms: u64,
}
fn default_evidence_lifetime() -> u64 {
    60_000
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    format: String,
    instance: String,
    provider: String,
    bundle_directory: PathBuf,
    api_base: String,
    ca_file: Option<PathBuf>,
    auth: AuthConfig,
    /// Selections written into this file.
    #[serde(default)]
    operations: Vec<Selection>,
    /// An absolute path to a reviewed selection set for `provider`, appended to
    /// `operations`. The repository ships one per provider it has reviewed.
    #[serde(default)]
    operations_file: Option<PathBuf>,
}

/// The document `operations_file` names.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationsFile {
    format: String,
    provider: String,
    operations: Vec<Selection>,
}

/// Sensitive input intentionally has no Debug or Serialize implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedEntry {
    #[serde(deserialize_with = "token")]
    token: Zeroizing<String>,
}
fn token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty() || value.len() > 8192 || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
}

struct Fixed(Secret);
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.0.clone()))
    }
}

pub struct Local {
    bootstrap: Bootstrap,
    bootstrap_v2: Bootstrap,
    http: Arc<ScopedHttp>,
    engine: Engine,
    instance: String,
    auth: AuthConfig,
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

impl Local {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = filesystem::read_bounded(
            filesystem::private_file(path).map_err(|_| Failure::InvalidConfiguration)?,
            1024 * 1024,
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        let config: Configuration =
            connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        if config.format != FORMAT
            || !connectors_core::valid_id(&config.instance)
            || !connectors_core::valid_id(&config.provider)
            || !connectors_core::valid_id(&config.auth.profile)
            || config.auth.header.is_empty()
            || config.auth.label.is_empty()
            || config.auth.label.len() > 128
            || config.auth.evidence_lifetime_ms == 0
            || config.auth.evidence_lifetime_ms > 300_000
            || segments(&config.auth.identity.path).is_empty()
            || config
                .auth
                .scopes
                .as_ref()
                .is_some_and(|s| segments(&s.path).is_empty())
            || (config.auth.scopes.is_none() && !config.auth.minimum_scopes.is_empty())
            || !config.bundle_directory.is_absolute()
            || config
                .operations_file
                .as_ref()
                .is_some_and(|path| !path.is_absolute())
        {
            return Err(Failure::InvalidConfiguration);
        }
        let mut operations = config.operations.clone();
        if let Some(path) = &config.operations_file {
            // A shipped selection set is ordinary repository content, readable by
            // anyone; only the configuration that names it must be private.
            let bytes = std::fs::File::open(path)
                .map_err(|_| Failure::InvalidConfiguration)
                .and_then(|file| {
                    filesystem::read_bounded(file, 1024 * 1024)
                        .map_err(|_| Failure::InvalidConfiguration)
                })?;
            let shipped: OperationsFile =
                connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
            if shipped.format != OPERATIONS_FORMAT || shipped.provider != config.provider {
                return Err(Failure::InvalidConfiguration);
            }
            operations.extend(shipped.operations);
        }
        let bundle = bundle::load(&config.bundle_directory, &config.provider)
            .map_err(Failure::from_service)?;
        let base = connectors_host::http::canonical_base(&config.api_base)
            .map_err(Failure::from_service)?;
        if !base.starts_with("https://") {
            return Err(Failure::InvalidConfiguration);
        }
        let base_path = url_path(&base).ok_or(Failure::InvalidConfiguration)?;
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let engine =
            Engine::new(&bundle, &base_path, &operations).map_err(Failure::from_service)?;
        let effective = json!({
            "format": config.format,
            "instance": config.instance,
            "provider": config.provider,
            "bundle_sha256": bundle::read_index(&config.bundle_directory)
                .ok()
                .and_then(|index| index.find(&config.provider).map(|e| e.bundle_sha256.clone())),
            "source_sha256": bundle.source.source_sha256,
            "api_base": base,
            "ca_digest": ca.as_ref().map(|b| connectors_core::digest(&json!(b))),
            "auth": serde_json::to_value(&config.auth).map_err(|_| Failure::Protocol)?,
            "operations": serde_json::to_value(&operations).map_err(|_| Failure::Protocol)?,
        });
        let configuration_revision = connectors_core::digest(&effective);
        let http = Arc::new(
            ScopedHttp::new_with_ca_bytes(
                &HttpConfig {
                    base_url: base.clone(),
                    credential: None,
                    credential_header: config.auth.header.clone(),
                    bearer: config.auth.bearer,
                    allow_plaintext: false,
                    ca_file: None,
                },
                None,
                ca.as_deref(),
            )
            .map_err(Failure::from_service)?,
        );
        let mut profile = Profile {
            id: config.auth.profile.clone(),
            revision: String::new(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: config.auth.minimum_scopes.clone(),
            evidence_lifetime_ms: config.auth.evidence_lifetime_ms,
            fields: vec![EntryField {
                name: "token".into(),
                label: config.auth.label.clone(),
                max_bytes: 8192,
            }],
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = |operations: Vec<connectors_core::Operation>| -> Result<String> {
            let descriptor = connectors_core::Descriptor {
                version: connectors_core::WIRE_VERSION.into(),
                instance: config.instance.clone(),
                adapter: "catalog".into(),
                revision: connectors_core::digest(&json!({
                    "configuration_revision": configuration_revision,
                    "operations": operations.iter().map(|o| o.id.clone()).collect::<Vec<_>>(),
                })),
                operations,
                configuration_schema: json!({"type": "object"}),
            };
            serde_json::to_string(&descriptor).map_err(|_| Failure::Protocol)
        };
        let requirement = |operation: &connectors_core::Operation| Requirement {
            operation: operation.id.clone(),
            profile: config.auth.profile.clone(),
            scopes: config.auth.minimum_scopes.clone(),
            effect: match engine.effect(&operation.id) {
                Some(Effect::Read) => runtime::Effect::Read,
                Some(Effect::Write) => runtime::Effect::Write,
                None => runtime::Effect::Unknown,
            },
        };
        let reads = engine.declarations(&[Effect::Read]);
        let all = engine.declarations(&[Effect::Read, Effect::Write]);
        let bootstrap = Bootstrap {
            instance: config.instance.clone(),
            adapter: "catalog".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision: configuration_revision.clone(),
            provider_authority: base.clone(),
            descriptor: descriptor(reads.clone())?,
            profiles: vec![profile],
            requirements: reads.iter().map(requirement).collect(),
        };
        bootstrap.validate()?;
        let mut bootstrap_v2 = bootstrap.clone();
        bootstrap_v2.descriptor = descriptor(all.clone())?;
        bootstrap_v2.requirements = all.iter().map(requirement).collect();
        bootstrap_v2.validate_for(runtime::PrivateProtocol::V2)?;
        Ok(Self {
            bootstrap,
            bootstrap_v2,
            http,
            engine,
            instance: config.instance,
            auth: config.auth,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    fn authenticated(&self, mut document: Secret) -> Result<ScopedHttp> {
        let document = Zeroizing::new(std::mem::take(&mut document.0));
        if document.len() > DOCUMENT_LIMIT {
            return Err(Failure::InvalidInput);
        }
        let entry: ProtectedEntry =
            serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
        let token = Secret(entry.token.as_bytes().to_vec());
        Ok(self.http.with_credential(Arc::new(Fixed(token))))
    }
}

/// The path of a canonical base URL, such as `/api/v4/` of `https://host/api/v4/`.
fn url_path(base: &str) -> Option<String> {
    let rest = base.strip_prefix("https://")?;
    Some(match rest.find('/') {
        Some(index) => rest[index..].to_owned(),
        None => "/".into(),
    })
}

fn probe_failure(status: u16) -> Failure {
    match status {
        401 => Failure::InvalidCredential,
        403 => Failure::Forbidden,
        429 | 500..=599 => Failure::Unavailable,
        _ => Failure::Protocol,
    }
}

#[async_trait::async_trait]
impl runtime::Adapter for Local {
    fn bootstrap(&self) -> Bootstrap {
        self.bootstrap.clone()
    }
    fn bootstrap_v2(&self) -> Result<Bootstrap> {
        Ok(self.bootstrap_v2.clone())
    }
    async fn prepare_write(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Box<dyn runtime::PreparedWrite>> {
        let http = self.authenticated(document)?;
        let prepared = self
            .engine
            .prepare(&http, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider)?;
        Ok(Box::new(Write {
            prepared,
            http: http.into_write(),
        }))
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline> {
        if profile != self.auth.profile {
            return Err(Failure::Unsupported);
        }
        let http = self.authenticated(document)?;
        let collected_at_ms = connectors_sdk::now_ms();
        let response = http
            .get(&segments(&self.auth.identity.path), &[])
            .await
            .map_err(Failure::from_provider)?;
        if response.status != 200 {
            return Err(probe_failure(response.status));
        }
        if response.body.len() > DOCUMENT_LIMIT {
            return Err(Failure::Protocol);
        }
        let identity: Value =
            connectors_core::read_json(&response.body).map_err(|_| Failure::Protocol)?;
        let subject = match identity.pointer(&self.auth.identity.subject_pointer) {
            Some(Value::String(text)) if !text.is_empty() && text.len() <= 256 => text.clone(),
            Some(Value::Number(number)) => number.to_string(),
            _ => return Err(Failure::Protocol),
        };
        let granted_scopes = match &self.auth.scopes {
            None => None,
            Some(probe) => {
                let response = http
                    .get(&segments(&probe.path), &[])
                    .await
                    .map_err(Failure::from_provider)?;
                if response.status != 200 {
                    return Err(probe_failure(response.status));
                }
                if response.body.len() > DOCUMENT_LIMIT {
                    return Err(Failure::Protocol);
                }
                let document: Value =
                    connectors_core::read_json(&response.body).map_err(|_| Failure::Protocol)?;
                let scopes = document
                    .pointer(&probe.pointer)
                    .and_then(Value::as_array)
                    .ok_or(Failure::Protocol)?;
                let mut granted = BTreeSet::new();
                for scope in scopes {
                    let scope = scope.as_str().ok_or(Failure::Protocol)?;
                    if scope.is_empty()
                        || scope.len() > 256
                        || !scope.bytes().all(|b| b.is_ascii_graphic())
                    {
                        return Err(Failure::Protocol);
                    }
                    granted.insert(scope.to_owned());
                }
                if granted.len() > 64 {
                    return Err(Failure::Protocol);
                }
                if !self.auth.minimum_scopes.is_subset(&granted) {
                    return Err(Failure::InsufficientScope);
                }
                Some(granted)
            }
        };
        Ok(Baseline {
            identity: registry::ExternalIdentity {
                kind: self.auth.identity.kind.clone(),
                subject,
            },
            granted_scopes,
            credential_expires_at_ms: None,
            collected_at_ms,
            valid_until_ms: collected_at_ms
                .checked_add(self.auth.evidence_lifetime_ms)
                .ok_or(Failure::Protocol)?,
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
        self.engine
            .read(&http, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider)
    }
}
struct Write {
    prepared: connectors_catalog_provider::Prepared,
    http: Box<dyn connectors_sdk::AuthenticatedWrite>,
}
#[async_trait::async_trait]
impl runtime::PreparedWrite for Write {
    async fn execute(self: Box<Self>) -> connectors_sdk::WriteOutcome<Value> {
        self.prepared.execute(self.http).await
    }
}
