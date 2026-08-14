//! Strict, value-free deployment-owned configuration for personal-local Connectors.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Read as _;
use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::time::Duration;

use domain::InitiationPolicy;
use protocol::operation::OwnerContext;
use serde::{Deserialize, Serialize};
use server::{
    SipDeploymentRoute, SipDialRouteTable, SipNetworkMode, SipSignalingTransport, SocketAperture,
    VoiceApplicationRoute,
};

const MAX_CONFIG_BYTES: u64 = 256 * 1024;

/// Personal-local composition. Voice is optional as one complete group; Slack and Grafana are
/// independent Integrations. None of these sections can carry credential values or ambient secret
/// paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalConfig {
    pub owner: OwnerConfig,
    #[serde(default)]
    pub connection: Option<ConnectionConfig>,
    #[serde(default)]
    pub authority: Option<AuthorityConfig>,
    #[serde(default)]
    pub application: Option<ApplicationConfig>,
    #[serde(default)]
    pub sip: Option<SipConfig>,
    #[serde(default)]
    pub slack: Option<SlackIntegrationConfig>,
    #[serde(default)]
    pub grafana: Option<GrafanaIntegrationConfig>,
}

/// Complete deployment selection needed to make the development `sip.dial` member callable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalVoiceConfig {
    /// Exact personal owner and current authority snapshot.
    pub owner: OwnerConfig,
    /// Connection, Grant and approval facts selected outside caller input.
    pub connection: ConnectionConfig,
    /// Session-authority issuer identity and private-key location.
    pub authority: AuthorityConfig,
    /// Exact RTVBP application endpoint and its transport destination.
    pub application: ApplicationConfig,
    /// Connection-owned SIP destination aliases and admitted apertures.
    pub sip: SipConfig,
}

/// Owner facts the daemon requires on every operation frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerConfig {
    pub tenant_id: String,
    pub agent_id: String,
    pub agent_revision: u64,
    pub authority_snapshot_id: String,
    pub authority_snapshot_sha256: String,
}

/// Deployment-owned Connection and independent operation authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionConfig {
    pub connection_ref: String,
    pub label: String,
    pub grant_ref: String,
    pub initiation: InitiationConfig,
    pub approval_evidence_ref: String,
}

/// Closed Connection initiation policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitiationConfig {
    B10x,
    Provider,
    Both,
}

/// Authority identity. The key file contains exactly 32 raw bytes or 64 lowercase hex digits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityConfig {
    pub issuer: String,
    pub key_id: String,
    pub signing_key_file: String,
}

/// Exact application route plus the deployment-resolved TCP/TLS destination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationConfig {
    pub actor: String,
    pub audience: String,
    pub deployment: String,
    pub resource: String,
    pub endpoint: String,
    pub authority_lifetime_seconds: u64,
    pub session_lease_seconds: u64,
    pub connect_address: SocketAddr,
    pub tls_server_name: String,
}

/// SIP routes belonging to one Connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipConfig {
    pub targets: Vec<SipTargetConfig>,
}

/// Value-free Slack Integration policy. App tokens arrive only through a Connect Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlackIntegrationConfig {
    pub grant_ref: String,
    pub initiation: InitiationConfig,
    pub allowed_events: Vec<String>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
}

/// Value-free Grafana policy. The service-account token arrives only through a Connect Session.
/// Target grants are independent authority for mediated Connections, keyed by target Provider id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrafanaIntegrationConfig {
    pub origin: String,
    pub grant_ref: String,
    pub initiation: InitiationConfig,
    pub target_grants: BTreeMap<String, String>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
}

/// One opaque alias mapped to an exact driver route.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipTargetConfig {
    pub alias: String,
    pub permission_subject: String,
    pub signaling_bind: SocketAddr,
    pub sent_by: String,
    pub target: SocketAddr,
    pub signaling_transport: SignalingTransportConfig,
    pub to_uri: String,
    pub from_uri: String,
    pub media_advertised: IpAddr,
    pub media_bind: IpAddr,
    pub signaling_apertures: Vec<ApertureConfig>,
    pub media_apertures: Vec<ApertureConfig>,
    pub dial_timeout_seconds: u64,
    pub network_mode: NetworkModeConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalingTransportConfig {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkModeConfig {
    Loopback,
    OperatorAuthorizedDevelopment,
}

/// Exact address and inclusive port range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApertureConfig {
    pub address: IpAddr,
    pub first_port: u16,
    pub last_port: u16,
}

/// Configuration refusal. It deliberately contains no credential contents.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Connector configuration could not be read: {0}")]
    Read(std::io::Error),
    #[error("Connector configuration is malformed: {0}")]
    Parse(toml::de::Error),
    #[error("Connector configuration is incomplete or inconsistent")]
    Invalid,
}

impl PersonalConfig {
    /// Read one strict TOML configuration.
    pub fn read(path: &Path) -> Result<Self, ConfigError> {
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)
            .map_err(ConfigError::Read)?;
        let metadata = file.metadata().map_err(ConfigError::Read)?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o022 != 0
            || metadata.len() > MAX_CONFIG_BYTES
        {
            return Err(ConfigError::Invalid);
        }
        let mut text = String::new();
        (&mut file)
            .take(MAX_CONFIG_BYTES + 1)
            .read_to_string(&mut text)
            .map_err(ConfigError::Read)?;
        if text.len() as u64 > MAX_CONFIG_BYTES {
            return Err(ConfigError::Invalid);
        }
        let config: Self = toml::from_str(&text).map_err(ConfigError::Parse)?;
        config.validate()?;
        Ok(config)
    }

    /// Exact owner context accepted by every configured backend.
    #[must_use]
    pub fn owner_context(&self) -> OwnerContext {
        OwnerContext {
            tenant_id: self.owner.tenant_id.clone(),
            agent_id: self.owner.agent_id.clone(),
            agent_revision: self.owner.agent_revision,
            authority_snapshot_id: self.owner.authority_snapshot_id.clone(),
            authority_snapshot_sha256: self.owner.authority_snapshot_sha256.clone(),
        }
    }

    /// Return voice configuration only when the complete group is present.
    pub fn voice(&self) -> Result<Option<PersonalVoiceConfig>, ConfigError> {
        match (
            &self.connection,
            &self.authority,
            &self.application,
            &self.sip,
        ) {
            (None, None, None, None) => Ok(None),
            (Some(connection), Some(authority), Some(application), Some(sip)) => {
                let voice = PersonalVoiceConfig {
                    owner: self.owner.clone(),
                    connection: connection.clone(),
                    authority: authority.clone(),
                    application: application.clone(),
                    sip: sip.clone(),
                };
                voice.validate()?;
                Ok(Some(voice))
            }
            _ => Err(ConfigError::Invalid),
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.owner_context().validate_for_config()?;
        let voice = self.voice()?;
        if let Some(slack) = &self.slack {
            slack.validate()?;
        }
        if let Some(grafana) = &self.grafana {
            grafana.validate()?;
        }
        if voice.is_none() && self.slack.is_none() && self.grafana.is_none() {
            return Err(ConfigError::Invalid);
        }
        Ok(())
    }
}

impl SlackIntegrationConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        let mut events = self.allowed_events.clone();
        events.sort();
        events.dedup();
        if !config_ref(&self.grant_ref, 512)
            || matches!(self.initiation, InitiationConfig::B10x)
            || events != self.allowed_events
            || events.is_empty()
            || events
                .iter()
                .any(|event| !matches!(event.as_str(), "app_mention" | "message"))
            || !(30..=900).contains(&self.connect_session_ttl_seconds)
        {
            return Err(ConfigError::Invalid);
        }
        Ok(())
    }
}

impl GrafanaIntegrationConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        let origin = url::Url::parse(&self.origin).map_err(|_| ConfigError::Invalid)?;
        let canonical_path = origin.path().is_empty() || origin.path() == "/";
        if origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || !canonical_path
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !config_ref(&self.grant_ref, 512)
            || matches!(self.initiation, InitiationConfig::Provider)
            || self.target_grants.is_empty()
            || self.target_grants.iter().any(|(provider, grant)| {
                !matches!(provider.as_str(), "prometheus" | "loki" | "alertmanager")
                    || !config_ref(grant, 512)
            })
            || !(30..=900).contains(&self.connect_session_ttl_seconds)
        {
            return Err(ConfigError::Invalid);
        }
        Ok(())
    }

    /// Independent grant selected for one recognized target Provider.
    #[must_use]
    pub fn target_grant(&self, provider: &str) -> Option<&str> {
        self.target_grants.get(provider).map(String::as_str)
    }

    /// Canonical origin without a trailing slash.
    #[must_use]
    pub fn canonical_origin(&self) -> String {
        self.origin.trim_end_matches('/').to_owned()
    }
}

const fn default_connect_session_ttl_seconds() -> u64 {
    300
}

impl PersonalVoiceConfig {
    /// Exact owner context accepted by the voice backend.
    #[must_use]
    pub fn owner_context(&self) -> OwnerContext {
        OwnerContext {
            tenant_id: self.owner.tenant_id.clone(),
            agent_id: self.owner.agent_id.clone(),
            agent_revision: self.owner.agent_revision,
            authority_snapshot_id: self.owner.authority_snapshot_id.clone(),
            authority_snapshot_sha256: self.owner.authority_snapshot_sha256.clone(),
        }
    }

    /// Build the non-wire initiation policy.
    #[must_use]
    pub fn initiation_policy(&self) -> InitiationPolicy {
        match self.connection.initiation {
            InitiationConfig::B10x => InitiationPolicy::b10x_only(),
            InitiationConfig::Provider => InitiationPolicy::provider_only(),
            InitiationConfig::Both => InitiationPolicy::bidirectional(),
        }
    }

    /// Build the exact Connection-owned route table.
    pub fn sip_routes(&self) -> Result<SipDialRouteTable, ConfigError> {
        let routes = self
            .sip
            .targets
            .iter()
            .map(|target| {
                let route = SipDeploymentRoute {
                    connection: self.connection.connection_ref.clone(),
                    signaling_bind: target.signaling_bind,
                    sent_by: target.sent_by.clone(),
                    target: target.target,
                    signaling_transport: match target.signaling_transport {
                        SignalingTransportConfig::Udp => SipSignalingTransport::Udp,
                        SignalingTransportConfig::Tcp => SipSignalingTransport::Tcp,
                    },
                    to_uri: target.to_uri.clone(),
                    from_uri: target.from_uri.clone(),
                    media_advertised: target.media_advertised,
                    media_bind: target.media_bind,
                    signaling_apertures: apertures(&target.signaling_apertures)?,
                    media_apertures: apertures(&target.media_apertures)?,
                    dial_timeout: Duration::from_secs(target.dial_timeout_seconds),
                    network_mode: match target.network_mode {
                        NetworkModeConfig::Loopback => SipNetworkMode::Loopback,
                        NetworkModeConfig::OperatorAuthorizedDevelopment => {
                            SipNetworkMode::OperatorAuthorizedDevelopment
                        }
                    },
                };
                server::validate_sip_deployment_route(&route).map_err(|_| ConfigError::Invalid)?;
                Ok((target.alias.clone(), route))
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;
        SipDialRouteTable::new(&self.connection.connection_ref, routes)
            .map_err(|_| ConfigError::Invalid)
    }

    /// Build the admitted application route; the TCP/TLS target remains in the connector object.
    #[must_use]
    pub fn application_route(&self) -> VoiceApplicationRoute {
        VoiceApplicationRoute {
            actor: self.application.actor.clone(),
            audience: self.application.audience.clone(),
            deployment: self.application.deployment.clone(),
            resource: self.application.resource.clone(),
            endpoint: self.application.endpoint.clone(),
            authority_lifetime: Duration::from_secs(self.application.authority_lifetime_seconds),
            session_lease: Duration::from_secs(self.application.session_lease_seconds),
        }
    }

    /// Return the reviewed permission subject for one exact configured alias.
    #[must_use]
    pub fn permission_subject(&self, alias: &str) -> Option<&str> {
        self.sip
            .targets
            .iter()
            .find(|target| target.alias == alias)
            .map(|target| target.permission_subject.as_str())
            .filter(|subject| !subject.is_empty())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let owner = self.owner_context();
        owner.validate_for_config()?;
        if !config_ref(&self.connection.connection_ref, 512)
            || self.connection.label.is_empty()
            || self.connection.label.len() > 1024
            || !config_ref(&self.connection.grant_ref, 512)
            || !config_ref(&self.connection.approval_evidence_ref, 512)
            || !config_ref(&self.authority.key_id, 128)
            || self.authority.signing_key_file.is_empty()
            || self.application.tls_server_name.is_empty()
            || self.sip.targets.is_empty()
            || self
                .sip
                .targets
                .iter()
                .any(|target| !config_ref(&target.permission_subject, 512))
        {
            return Err(ConfigError::Invalid);
        }
        let issuer = url::Url::parse(&self.authority.issuer).map_err(|_| ConfigError::Invalid)?;
        if issuer.scheme() != "https"
            || issuer.host_str().is_none()
            || !issuer.username().is_empty()
            || issuer.password().is_some()
            || issuer.query().is_some()
            || issuer.fragment().is_some()
        {
            return Err(ConfigError::Invalid);
        }
        let endpoint =
            url::Url::parse(&self.application.endpoint).map_err(|_| ConfigError::Invalid)?;
        if endpoint.host_str() != Some(self.application.tls_server_name.as_str()) {
            return Err(ConfigError::Invalid);
        }
        self.sip_routes()?;
        server::validate_voice_application_route(&self.application_route())
            .map_err(|_| ConfigError::Invalid)?;
        Ok(())
    }
}

trait OwnerConfigValidation {
    fn validate_for_config(&self) -> Result<(), ConfigError>;
}

impl OwnerConfigValidation for OwnerContext {
    fn validate_for_config(&self) -> Result<(), ConfigError> {
        let request = protocol::operation::RequestEnvelope {
            protocol: protocol::operation::CONTRACT.to_owned(),
            request_id: "configuration-validation".to_owned(),
            context: self.clone(),
            request: protocol::operation::OperationRequest::Search(
                protocol::operation::SearchRequest {
                    query: String::new(),
                    limit: 1,
                },
            ),
        };
        request.validate().map_err(|_| ConfigError::Invalid)
    }
}

fn apertures(config: &[ApertureConfig]) -> Result<Vec<SocketAperture>, ConfigError> {
    config
        .iter()
        .map(|aperture| {
            SocketAperture::new(
                aperture.address,
                RangeInclusive::new(aperture.first_port, aperture.last_port),
            )
            .map_err(|_| ConfigError::Invalid)
        })
        .collect()
}

fn config_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    #[test]
    fn documented_development_configuration_stays_strict_and_valid() {
        let config: PersonalConfig =
            toml::from_str(include_str!("../examples/asterisk-dev.example.toml")).unwrap();
        config.validate().unwrap();
        let config = config.voice().unwrap().unwrap();
        assert_eq!(
            config.permission_subject("asterisk-dev"),
            Some("private:asterisk-development")
        );
    }

    #[test]
    fn deployment_configuration_cannot_be_symlinked_or_group_writable() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("voice.toml");
        fs::write(&path, include_str!("../examples/asterisk-dev.example.toml")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(PersonalConfig::read(&path).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        PersonalConfig::read(&path).unwrap();

        let link = root.path().join("voice-link.toml");
        std::os::unix::fs::symlink(path, &link).unwrap();
        assert!(PersonalConfig::read(&link).is_err());
    }

    #[test]
    fn slack_only_configuration_contains_policy_but_no_secret_source() {
        let config: PersonalConfig = toml::from_str(
            r#"
[owner]
tenant_id = "tenant-local"
agent_id = "agent-dev"
agent_revision = 1
authority_snapshot_id = "authority-1"
authority_snapshot_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[slack]
grant_ref = "grant:slack-inbound"
initiation = "provider"
allowed_events = ["app_mention", "message"]
"#,
        )
        .unwrap();
        config.validate().unwrap();
        assert!(config.voice().unwrap().is_none());
    }

    #[test]
    fn grafana_configuration_names_origin_and_independent_target_grants_only() {
        let config: PersonalConfig =
            toml::from_str(include_str!("../examples/grafana-federation.example.toml")).unwrap();
        config.validate().unwrap();
        let grafana = config.grafana.unwrap();
        assert_eq!(
            grafana.canonical_origin(),
            "https://grafana.monitoring.example"
        );
        assert_eq!(grafana.target_grant("loki"), Some("grant:loki-read"));

        let encoded = toml::to_string(&grafana).unwrap();
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("password"));
        assert!(!encoded.contains("secret"));
    }
}
