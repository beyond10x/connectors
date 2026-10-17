//! Executable composition: the native configuration
//! (`connectors_grafana.connection.LocalConfiguration`) and the protected entry of
//! `grafana.service_account` and `grafana.irm_service_account` terminate here. The
//! business adapter receives only immutable authenticated HTTP read ports: the stack port,
//! which may also send the two incident reads as a POST, and, when `irm_api_url` is
//! configured, the IRM port with `X-Grafana-URL` fixed to the stack URL. No Grafana
//! operation writes.
use connectors_grafana::{Grafana, auth};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result},
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

pub const FORMAT: &str = "connectors-grafana-local/1";

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
    /// Absent or a URL, never `null`, like `ca_file`.
    #[serde(default, deserialize_with = "present")]
    irm_api_url: Option<String>,
}
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

/// The model (`LocalConfiguration`) admits only a literal `https://` URL of at most 512
/// characters; this composition further requires it to be written in its canonical form
/// (ESS-LIMIT in connection.yaml), so nothing the model refuses is admitted here.
fn configured_url(value: &str) -> Result<String> {
    let base = connectors_host::http::canonical_base(value).map_err(Failure::from_service)?;
    if !value.starts_with("https://") || value.chars().count() > 512 || base != value {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(base)
}

fn port(base: &str, ca: Option<&[u8]>) -> Result<ScopedHttp> {
    ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.to_owned(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: false,
            ca_file: None,
        },
        None,
        ca,
    )
    .map_err(Failure::from_service)
}

/// A static token of a Grafana service account, entered once through the host's protected
/// entry. Grafana grants no scopes; the account's role and permissions decide what the
/// token reaches. Both profiles take the same entry and differ only in their probe.
fn profile(id: &str) -> Result<Profile> {
    let mut profile = Profile {
        id: id.into(),
        revision: String::new(),
        purpose: registry::Purpose::ServiceAccount,
        subject: registry::Subject::App,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
        fields: vec![EntryField {
            name: "token".into(),
            label: "Grafana service-account token".into(),
            max_bytes: auth::TOKEN_BYTES as u32,
        }],
        acquisition: None,
    };
    profile.revision =
        connectors_core::digest(&serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?);
    Ok(profile)
}

pub struct Local {
    bootstrap: Bootstrap,
    http: Arc<ScopedHttp>,
    irm: Option<Arc<ScopedHttp>>,
    adapter: Grafana,
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
        let base = configured_url(&config.base_url)?;
        let irm_api_url = config
            .irm_api_url
            .as_deref()
            .map(configured_url)
            .transpose()?;
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let mut effective = json!({"format": config.format, "instance": config.instance,
            "base_url": base,
            "ca_digest": ca.as_ref().map(|b| connectors_core::digest(&json!(b)))});
        // Only a configuration that names the IRM API URL carries it, so one that does not
        // keeps the revision it had before the member existed.
        if let Some(irm) = &irm_api_url {
            effective["irm_api_url"] = json!(irm);
        }
        let configuration_revision = connectors_core::digest(&effective);
        // The provider authority names every destination the stored token is sent to: the
        // stack URL, and the IRM API URL when one is configured. Both are canonical URLs,
        // which hold no space, so the pair is unambiguous. A changed IRM URL is therefore a
        // changed authority (a new instance id), never an upgrading revalidation; a
        // configuration without it keeps the authority it had before the member existed.
        let provider_authority = match &irm_api_url {
            None => base.clone(),
            Some(irm) => format!("{base} {irm}"),
        };
        let mut stack = port(&base, ca.as_deref())?;
        let irm = match &irm_api_url {
            None => None,
            Some(irm) => {
                // The stack port may send exactly the two incident reads as a POST.
                stack = stack
                    .with_read_posts(
                        &connectors_grafana::INCIDENT_READ_POSTS,
                        connectors_grafana::READ_POST_TIMEOUT,
                    )
                    .map_err(Failure::from_service)?;
                // The stack header is derived from configuration, never from caller input.
                Some(Arc::new(
                    port(irm, ca.as_deref())?
                        .with_fixed_header(
                            connectors_grafana::STACK_HEADER,
                            &connectors_grafana::stack_header_value(&base),
                        )
                        .map_err(Failure::from_service)?,
                ))
            }
        };
        let http = Arc::new(stack);
        // This template has no credential and performs no provider work.
        let adapter = Grafana::new(&config.instance, &effective, http.clone())
            .map_err(|_| Failure::InvalidConfiguration)?;
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance,
            adapter: "grafana".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision,
            provider_authority,
            descriptor: serde_json::to_string(&descriptor).map_err(|_| Failure::Protocol)?,
            profiles: vec![profile(auth::PROFILE_ID)?, profile(auth::IRM_PROFILE_ID)?],
            requirements: descriptor
                .operations
                .iter()
                .map(|o| {
                    let (profile, effect) = match o.id.as_str() {
                        "datasources.list" => (auth::PROFILE_ID, Effect::Read),
                        "irm.alert_groups.list"
                        | "irm.alert_group.get"
                        | "irm.incidents.list"
                        | "irm.incident.get" => (auth::IRM_PROFILE_ID, Effect::Read),
                        _ => (auth::PROFILE_ID, Effect::Unknown),
                    };
                    Requirement {
                        operation: o.id.clone(),
                        custody: false,
                        profile: profile.into(),
                        scopes: BTreeSet::new(),
                        effect,
                    }
                })
                .collect(),
        };
        // An operation this composition does not classify leaves Effect::Unknown,
        // and the read-only protocol refuses the whole bootstrap.
        bootstrap.validate_for(connectors_host::local::runtime::PrivateProtocol::V1)?;
        Ok(Self {
            bootstrap,
            http,
            irm,
            adapter,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    fn token(document: &mut Secret) -> Result<Arc<Fixed>> {
        let token = auth::ProtectedEntry::parse(std::mem::take(&mut document.0))
            .map_err(native_failure)?
            .into_secret();
        Ok(Arc::new(Fixed(token)))
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
    async fn validate(&self, profile: &str, mut document: Secret) -> Result<Baseline> {
        let value = match profile {
            auth::PROFILE_ID => {
                let http = self.http.with_credential(Self::token(&mut document)?);
                auth::validate(&http, connectors_sdk::now_ms).await
            }
            auth::IRM_PROFILE_ID => {
                // Without `irm_api_url` there is nothing to prove the token against.
                let irm = self.irm.as_ref().ok_or(Failure::Unavailable)?;
                let http = irm.with_credential(Self::token(&mut document)?);
                auth::validate_irm(&http, connectors_sdk::now_ms).await
            }
            _ => return Err(Failure::Unsupported),
        }
        .map_err(native_failure)?;
        Ok(Baseline {
            // The probe's body is not read; the subject is the configured connection
            // (identity source `configuration`, see auth.rs).
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
        mut document: Secret,
        input: Value,
    ) -> Result<Value> {
        let token = Self::token(&mut document)?;
        let mut adapter = self
            .adapter
            .with_authenticated_http(Arc::new(self.http.with_credential(token.clone())));
        if let Some(irm) = &self.irm {
            adapter = adapter.with_irm(Arc::new(irm.with_credential(token)));
        }
        adapter
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
