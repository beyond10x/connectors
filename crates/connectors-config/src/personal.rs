//! Strict, value-free deployment-owned configuration for personal-local Connectors.

use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::Duration;

use domain::audio::AudioSink;
use domain::InitiationPolicy;
use protocol::operation::OwnerContext;
use serde::{Deserialize, Serialize};
use service::PrincipalContext;
use service::{
    AudioDeploymentRoute, BrowserDeploymentRoute, SipDeploymentRoute, SipDialRouteTable,
    SipNetworkMode, SipSignalingTransport, SocketAperture, VoiceApplicationRoute,
};

use crate::file::{read_trusted_config, TrustedConfigReadError, TrustedOwner};

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
    #[serde(default)]
    pub kubernetes: Option<KubernetesIntegrationConfig>,
    #[serde(default)]
    pub b10x: Option<B10xIntegrationConfig>,
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

/// Deployment-owned Connection for B10x service and device capabilities.
///
/// This shape deliberately has no approval reference. A static configured string is not
/// receiver-verifiable evidence for one invocation, so the B10x Integration refuses its
/// approval-required operations until an approval verifier is composed at the Connector boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct B10xConnectionConfig {
    pub connection_ref: String,
    pub label: String,
    pub grant_ref: String,
    pub initiation: InitiationConfig,
}

/// Closed Connection initiation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Legacy/local grant and compatibility fallback for companion-bot authority.
    pub grant_ref: String,
    #[serde(default)]
    pub org_read_grant_ref: Option<String>,
    #[serde(default)]
    pub user_grant_ref: Option<String>,
    #[serde(default)]
    pub companion_grant_ref: Option<String>,
    #[serde(default)]
    pub expected_team_id: Option<String>,
    #[serde(default)]
    pub oauth_client_id: Option<String>,
    #[serde(default)]
    pub oauth_redirect_uri: Option<String>,
    pub initiation: InitiationConfig,
    pub allowed_events: Vec<String>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
    /// Named identities this placement holds at once, each with its own credential file.
    #[serde(default)]
    pub instances: Vec<SlackInstanceConfig>,
}

/// One named Slack identity a placement holds.
///
/// A workstation reaches Slack as more than one actor: the workspace bot for looking things up, the
/// operator themself for what only they can see, an assistant bot for posting. Each is a separate
/// Connection with its own token and its own authority, and an agent about to read or post has to
/// be able to tell them apart — which is what `name` and `purpose` are for.
///
/// The credential is a **path**, never a value. Nothing that composes this file may open it; the
/// Connector does, because the Connector is the process admitted to hold credentials. This is the
/// same shape `HostedKubernetesConfig::token_file` and `AuthorityConfig::signing_key_file` take.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlackInstanceConfig {
    /// Stable, human-chosen identity. Fixes this instance's Connection and credential addresses.
    pub name: String,
    /// Which actor this instance is, and therefore what it may do. `org_bot` is read-only.
    pub profile: SlackInstanceProfile,
    /// When an agent should reach for this instance rather than another. Carried to the workbench.
    #[serde(default)]
    pub purpose: Option<String>,
    /// Owner-only file holding the `xoxb-`/`xoxp-` token, by path.
    pub token_file: PathBuf,
}

/// The actor one declared Slack instance speaks as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlackInstanceProfile {
    /// The workspace bot, admitted for reads only.
    OrgBot,
    /// The operator themself: everything their own Slack account can see.
    OrgUser,
    /// An assistant bot that may post and react.
    CompanionBot,
}

impl SlackInstanceProfile {
    /// The `auth_profile` reference this instance's Connection carries.
    #[must_use]
    pub const fn auth_profile(self) -> &'static str {
        match self {
            Self::OrgBot => "slack.org_bot",
            Self::OrgUser => "slack.org_user",
            Self::CompanionBot => "slack.companion_bot",
        }
    }

    /// The token prefix this actor's credential must carry.
    #[must_use]
    pub const fn token_prefix(self) -> &'static str {
        match self {
            Self::OrgBot | Self::CompanionBot => "xoxb-",
            Self::OrgUser => "xoxp-",
        }
    }
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

/// Value-free personal-local Kubernetes discovery policy. Authentication remains in the user's
/// standard kubeconfig credential source and is resolved only after a candidate is activated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubernetesIntegrationConfig {
    pub grant_ref: String,
    pub initiation: InitiationConfig,
    /// Empty means cluster-wide discovery when the selected identity is allowed to list Services.
    #[serde(default)]
    pub namespaces: Vec<String>,
    /// Independent grants for monitoring Connections recognized behind Kubernetes Services.
    pub target_grants: BTreeMap<String, String>,
    /// Exec and legacy auth-provider plugins can run local credential helpers and require opt-in.
    #[serde(default)]
    pub allow_exec_auth: bool,
    #[serde(default = "default_kubernetes_resource_limit")]
    pub resource_limit: u16,
}

/// Deployment-owned routes for B10x's native drivers and private services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct B10xIntegrationConfig {
    pub connection: B10xConnectionConfig,
    /// Modules granted by default to every Identity-verified member of the hosted tenant.
    /// `None` preserves the pre-policy behavior for personal and existing hosted configurations.
    #[serde(default)]
    pub tenant_member_modules: Option<Vec<String>>,
    #[serde(default)]
    pub work_origin: Option<String>,
    #[serde(default)]
    pub ontology_origin: Option<String>,
    #[serde(default)]
    pub planner_origin: Option<String>,
    #[serde(default)]
    pub workspaces_origin: Option<String>,
    #[serde(default)]
    pub colab_origin: Option<String>,
    /// Local-personal module id to owner-only Unix socket. This is a transport route, not a Grant.
    #[serde(default)]
    pub module_sockets: BTreeMap<String, std::path::PathBuf>,
    #[serde(default)]
    pub ontology_bearer_file: Option<std::path::PathBuf>,
    /// Owner-only file containing the base64url-encoded 32-byte Ed25519 module-signing seed.
    #[serde(default)]
    pub module_signing_key_file: Option<std::path::PathBuf>,
    #[serde(default)]
    pub module_signing_key_id: Option<String>,
    #[serde(default)]
    pub module_signing_issuer: Option<String>,
    #[serde(default)]
    pub audio: Option<AudioIntegrationConfig>,
    #[serde(default)]
    pub browser: Option<BrowserIntegrationConfig>,
}

/// Exact local Piper/audio route. No field is caller-visible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioIntegrationConfig {
    #[serde(default)]
    pub synthesizer: Option<std::path::PathBuf>,
    pub voice: std::path::PathBuf,
    #[serde(default)]
    pub voice_config: Option<std::path::PathBuf>,
    #[serde(default)]
    pub voice_sha256: Option<String>,
    #[serde(default)]
    pub sink: Option<AudioSink>,
    pub maximum_characters: u32,
    pub maximum_utterance_seconds: u64,
}

/// Exact dedicated Chromium-family browser route. No field is caller-visible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserIntegrationConfig {
    #[serde(default)]
    pub executable: Option<std::path::PathBuf>,
    pub user_data_dir: std::path::PathBuf,
    pub artifacts_dir: std::path::PathBuf,
    pub maximum_nodes: u32,
    pub maximum_navigation_seconds: u64,
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
        Self::read_with_owner(path, TrustedOwner::CurrentUser)
    }

    /// Read strict deployment configuration installed by root for a non-root hosted process.
    pub fn read_hosted(path: &Path) -> Result<Self, ConfigError> {
        Self::read_with_owner(path, TrustedOwner::CurrentUserOrRoot)
    }

    fn read_with_owner(path: &Path, owner: TrustedOwner) -> Result<Self, ConfigError> {
        let text =
            read_trusted_config(path, MAX_CONFIG_BYTES, owner).map_err(|error| match error {
                TrustedConfigReadError::Io(error) => ConfigError::Read(error),
                TrustedConfigReadError::Unsafe => ConfigError::Invalid,
            })?;
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

    /// Exact admitted application principal accepted by configured personal-local backends.
    pub fn principal_context(&self) -> Result<PrincipalContext, ConfigError> {
        PrincipalContext::local(&self.owner_context()).map_err(|_| ConfigError::Invalid)
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
        if let Some(kubernetes) = &self.kubernetes {
            kubernetes.validate()?;
        }
        if let Some(b10x) = &self.b10x {
            b10x.validate()?;
        }
        if voice.is_none()
            && self.slack.is_none()
            && self.grafana.is_none()
            && self.kubernetes.is_none()
            && self.b10x.is_none()
        {
            return Err(ConfigError::Invalid);
        }
        Ok(())
    }
}

impl B10xIntegrationConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        if !valid_connection(&self.connection)
            || matches!(self.connection.initiation, InitiationConfig::Provider)
            || self.ontology_bearer_file.is_some()
            || self
                .module_signing_key_file
                .as_deref()
                .is_some_and(|path| !path.is_absolute())
            || self
                .work_origin
                .as_deref()
                .is_some_and(|origin| !private_origin(origin))
            || self
                .ontology_origin
                .as_deref()
                .is_some_and(|origin| !private_origin(origin))
            || self
                .planner_origin
                .as_deref()
                .is_some_and(|origin| !private_origin(origin))
            || self
                .workspaces_origin
                .as_deref()
                .is_some_and(|origin| !private_origin(origin))
            || self
                .colab_origin
                .as_deref()
                .is_some_and(|origin| !private_origin(origin))
            || self.module_sockets.iter().any(|(module, socket)| {
                !matches!(
                    module.as_str(),
                    "colab" | "ontology" | "planner" | "work" | "workspaces"
                ) || !socket.is_absolute()
                    || self.origin_configured(module)
            })
            || self.tenant_member_modules.as_ref().is_some_and(|modules| {
                let mut canonical = modules.clone();
                canonical.sort();
                canonical.dedup();
                canonical != *modules
                    || modules.iter().any(|module| {
                        !matches!(
                            module.as_str(),
                            "colab" | "ontology" | "planner" | "work" | "workspaces"
                        ) || !self.module_configured(module)
                    })
            })
            || ((self.work_origin.is_some()
                || self.ontology_origin.is_some()
                || self.planner_origin.is_some()
                || self.workspaces_origin.is_some()
                || self.colab_origin.is_some())
                && (self.module_signing_key_file.is_none()
                    || self
                        .module_signing_key_id
                        .as_deref()
                        .is_none_or(|value| !config_ref(value, 128))
                    || self
                        .module_signing_issuer
                        .as_deref()
                        .is_none_or(|value| !config_ref(value, 256))))
            || (self.work_origin.is_none()
                && self.ontology_origin.is_none()
                && self.planner_origin.is_none()
                && self.workspaces_origin.is_none()
                && self.colab_origin.is_none()
                && self.module_sockets.is_empty()
                && self.audio.is_none()
                && self.browser.is_none())
        {
            return Err(ConfigError::Invalid);
        }
        if let Some(route) = self.audio_route() {
            service::validate_audio_deployment_route(&route).map_err(|_| ConfigError::Invalid)?;
        }
        if let Some(route) = self.browser_route() {
            service::validate_browser_deployment_route(&route).map_err(|_| ConfigError::Invalid)?;
        }
        Ok(())
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

    /// Build the deployment-only audio route when configured.
    #[must_use]
    pub fn audio_route(&self) -> Option<AudioDeploymentRoute> {
        self.audio.as_ref().map(|audio| AudioDeploymentRoute {
            connection: self.connection.connection_ref.clone(),
            synthesizer: audio.synthesizer.clone(),
            voice: audio.voice.clone(),
            voice_config: audio.voice_config.clone(),
            voice_sha256: audio.voice_sha256.clone(),
            sink: audio.sink,
            maximum_characters: audio.maximum_characters,
            maximum_utterance: Duration::from_secs(audio.maximum_utterance_seconds),
        })
    }

    /// Build the deployment-only browser route when configured.
    #[must_use]
    pub fn browser_route(&self) -> Option<BrowserDeploymentRoute> {
        self.browser.as_ref().map(|browser| BrowserDeploymentRoute {
            connection: self.connection.connection_ref.clone(),
            executable: browser.executable.clone(),
            user_data_dir: browser.user_data_dir.clone(),
            artifacts_dir: browser.artifacts_dir.clone(),
            maximum_nodes: browser.maximum_nodes,
            maximum_navigation: Duration::from_secs(browser.maximum_navigation_seconds),
        })
    }

    /// Canonical Work origin without a trailing slash.
    #[must_use]
    pub fn work_origin(&self) -> Option<String> {
        self.work_origin
            .as_deref()
            .map(|origin| origin.trim_end_matches('/').to_owned())
    }

    /// Canonical Ontology origin without a trailing slash.
    #[must_use]
    pub fn ontology_origin(&self) -> Option<String> {
        self.ontology_origin
            .as_deref()
            .map(|origin| origin.trim_end_matches('/').to_owned())
    }

    /// Canonical Planner origin without a trailing slash.
    #[must_use]
    pub fn planner_origin(&self) -> Option<String> {
        self.planner_origin
            .as_deref()
            .map(|origin| origin.trim_end_matches('/').to_owned())
    }

    /// Canonical Workspaces origin without a trailing slash.
    #[must_use]
    pub fn workspaces_origin(&self) -> Option<String> {
        self.workspaces_origin
            .as_deref()
            .map(|origin| origin.trim_end_matches('/').to_owned())
    }

    /// Canonical Colab origin without a trailing slash.
    #[must_use]
    pub fn colab_origin(&self) -> Option<String> {
        self.colab_origin
            .as_deref()
            .map(|origin| origin.trim_end_matches('/').to_owned())
    }

    /// Exact owner-only local socket for one module, when configured.
    #[must_use]
    pub fn module_socket(&self, module: &str) -> Option<&Path> {
        self.module_sockets
            .get(module)
            .map(std::path::PathBuf::as_path)
    }

    /// Whether one module has exactly one hosted or local-personal transport route.
    #[must_use]
    pub fn module_configured(&self, module: &str) -> bool {
        self.origin_configured(module) || self.module_sockets.contains_key(module)
    }

    fn origin_configured(&self, module: &str) -> bool {
        match module {
            "work" => self.work_origin.is_some(),
            "ontology" => self.ontology_origin.is_some(),
            "planner" => self.planner_origin.is_some(),
            "workspaces" => self.workspaces_origin.is_some(),
            "colab" => self.colab_origin.is_some(),
            _ => false,
        }
    }

    /// Whether a hosted tenant member receives one configured module by deployment policy.
    #[must_use]
    pub fn tenant_member_module_enabled(&self, module: &str) -> bool {
        self.tenant_member_modules.as_ref().is_none_or(|modules| {
            modules
                .binary_search_by(|configured| configured.as_str().cmp(module))
                .is_ok()
        })
    }
}

fn valid_connection(connection: &B10xConnectionConfig) -> bool {
    config_ref(&connection.connection_ref, 512)
        && !connection.label.is_empty()
        && connection.label.len() <= 1024
        && config_ref(&connection.grant_ref, 512)
}

fn private_origin(value: &str) -> bool {
    let Ok(origin) = url::Url::parse(value) else {
        return false;
    };
    matches!(origin.scheme(), "http" | "https")
        && origin.host_str().is_some()
        && origin.username().is_empty()
        && origin.password().is_none()
        && (origin.path().is_empty() || origin.path() == "/")
        && origin.query().is_none()
        && origin.fragment().is_none()
}

impl SlackIntegrationConfig {
    #[must_use]
    pub fn grant_for_profile(&self, profile: &str) -> &str {
        match profile {
            "slack.org_bot" => self
                .org_read_grant_ref
                .as_deref()
                .unwrap_or(&self.grant_ref),
            "slack.org_user" => self.user_grant_ref.as_deref().unwrap_or(&self.grant_ref),
            "slack.companion_bot" => self
                .companion_grant_ref
                .as_deref()
                .unwrap_or(&self.grant_ref),
            _ => &self.grant_ref,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        let mut events = self.allowed_events.clone();
        events.sort();
        events.dedup();
        let valid_optional_ref =
            |value: &Option<String>| value.as_deref().is_none_or(|value| config_ref(value, 512));
        let valid_team = self.expected_team_id.as_deref().is_none_or(|team| {
            (2..=64).contains(&team.len()) && team.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
        let valid_client = self.oauth_client_id.as_deref().is_none_or(|client| {
            !client.is_empty()
                && client.len() <= 256
                && client
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
        });
        let valid_redirect = self.oauth_redirect_uri.as_deref().is_none_or(|redirect| {
            url::Url::parse(redirect).is_ok_and(|url| {
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            })
        });
        if !config_ref(&self.grant_ref, 512)
            || !valid_optional_ref(&self.org_read_grant_ref)
            || !valid_optional_ref(&self.user_grant_ref)
            || !valid_optional_ref(&self.companion_grant_ref)
            || !valid_team
            || !valid_client
            || !valid_redirect
            || self.oauth_client_id.is_some() != self.oauth_redirect_uri.is_some()
            || matches!(self.initiation, InitiationConfig::B10x)
            || events != self.allowed_events
            || events.is_empty()
            || events
                .iter()
                .any(|event| !matches!(event.as_str(), "app_mention" | "message.channels"))
            || !(30..=900).contains(&self.connect_session_ttl_seconds)
        {
            return Err(ConfigError::Invalid);
        }
        let mut names = BTreeSet::new();
        for instance in &self.instances {
            instance.validate()?;
            if !names.insert(instance.name.as_str()) {
                return Err(ConfigError::Invalid);
            }
        }
        Ok(())
    }
}

impl SlackInstanceConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // The name is an identity, not prose: it addresses a Connection, a credential and a
        // datasource binding, and it appears in a path. Keeping it to a lowercase label means a
        // name can never spell a second instance's address or escape its own directory.
        if self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || self.name.starts_with('-')
            || self.name.ends_with('-')
        {
            return Err(ConfigError::Invalid);
        }
        if self
            .purpose
            .as_deref()
            .is_some_and(|purpose| purpose.is_empty() || purpose.len() > 512)
        {
            return Err(ConfigError::Invalid);
        }
        // Absolute, because a relative credential path resolves against whatever directory the
        // Connector happened to start in. The file's own ownership and mode are checked where it is
        // read, not here: this type states the reference, and only the reader can state what it
        // found.
        if !self.token_file.is_absolute() {
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

impl KubernetesIntegrationConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        let mut namespaces = self.namespaces.clone();
        namespaces.sort();
        namespaces.dedup();
        if !config_ref(&self.grant_ref, 512)
            || matches!(self.initiation, InitiationConfig::Provider)
            || namespaces != self.namespaces
            || self
                .namespaces
                .iter()
                .any(|namespace| !dns_label(namespace))
            || self.target_grants.is_empty()
            || self.target_grants.iter().any(|(provider, grant)| {
                !matches!(
                    provider.as_str(),
                    "grafana" | "prometheus" | "loki" | "alertmanager"
                ) || !config_ref(grant, 512)
            })
            || !(1..=512).contains(&self.resource_limit)
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
}

const fn default_kubernetes_resource_limit() -> u16 {
    256
}

fn dns_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
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

    /// Exact admitted application principal accepted by the voice backend.
    pub fn principal_context(&self) -> Result<PrincipalContext, ConfigError> {
        PrincipalContext::local(&self.owner_context()).map_err(|_| ConfigError::Invalid)
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
                service::validate_sip_deployment_route(&route).map_err(|_| ConfigError::Invalid)?;
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
        service::validate_voice_application_route(&self.application_route())
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
allowed_events = ["app_mention", "message.channels"]
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

    #[test]
    fn kubernetes_configuration_is_policy_only() {
        let config: PersonalConfig = toml::from_str(include_str!(
            "../examples/kubernetes-discovery.example.toml"
        ))
        .unwrap();
        config.validate().unwrap();
        let kubernetes = config.kubernetes.unwrap();
        assert_eq!(kubernetes.namespaces, ["monitoring"]);
        assert_eq!(
            kubernetes.target_grant("grafana"),
            Some("grant:grafana:cluster-service")
        );
        let encoded = toml::to_string(&kubernetes).unwrap();
        assert!(!encoded.contains("kubeconfig"));
        assert!(!encoded.contains("server_url"));
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("password"));
        assert!(!encoded.contains("secret"));
    }
}
