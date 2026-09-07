//! Credential-free discovered interfaces and operator-owned bindings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One interface on an immutable discovered resource identity. Inventory is not a Grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// Opaque source-qualified identity, incorporating the resource UID and interface.
    pub endpoint_ref: String,
    /// Connection or configured source that owns resource access.
    pub source_ref: String,
    /// Resource namespace; absent for cluster-scoped resources.
    pub namespace: Option<String>,
    /// Owner-declared resource kind, such as Service.
    pub resource_kind: String,
    /// Display name; never substitutes for the immutable resource UID.
    pub resource_name: String,
    /// Immutable upstream identity; replacement creates a different endpoint.
    pub resource_uid: String,
    /// Declared port name, if present.
    pub port_name: Option<String>,
    /// Advertised port; credentials may supply it later for external resources.
    pub port: Option<u16>,
    /// Advertised transport, independently of driver availability.
    pub transport: EndpointTransport,
    /// Interface identity such as http, ari, ami, or sql.
    pub interface: String,
    /// Recognized catalog provider; absent when recognition is unresolved.
    pub provider: Option<String>,
    /// Current readiness explanation; Ready still requires invocation admission.
    pub state: EndpointState,
    /// Reviewed operator binding, with references and never credential values.
    pub binding: Option<EndpointBinding>,
}

/// A discovered transport does not by itself install a protocol driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointTransport {
    /// Stream transport supported by Kubernetes Pod forwarding.
    Tcp,
    /// Datagram transport, visible even when execution is unsupported.
    Udp,
    /// Stream Control Transmission Protocol.
    Sctp,
}

/// Actionable state of a discovered interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointState {
    /// Recognized and configured; invocation authorization remains independent.
    Ready,
    /// No unambiguous catalog provider matches the interface.
    UnknownProvider,
    /// No installed driver or route supports the advertised interface.
    UnsupportedProtocol,
    /// A required credential reference is not configured or cannot be resolved.
    MissingCredentials,
    /// Current source or endpoint policy refuses access.
    Denied,
    /// No approved reachable route exists.
    UnavailableRoute,
    /// The source resource has disappeared or current evidence is unavailable.
    Stale,
}

/// Operator-controlled interpretation of an endpoint, persisted outside the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointBinding {
    /// Stable provider from the installed catalog.
    pub provider: String,
    /// Optional HTTP path prefix; no query, fragment, or authority is accepted.
    pub base_path: Option<String>,
    /// Exact credential reference authorized by source policy.
    pub credential: Option<EndpointCredentialReference>,
    /// Explicit credential-free origin for an approved direct route.
    pub direct_address: Option<String>,
    /// SQL database override; Crossplane may instead provide the authoritative external name.
    #[serde(default)]
    pub database: Option<String>,
    /// SQL transport policy; absence requires TLS. HTTP profiles use their declared scheme.
    #[serde(default)]
    pub tls: Option<EndpointTls>,
    /// Explicit HTTP scheme for a forwarded service; does not select a direct route.
    #[serde(default)]
    pub scheme: Option<EndpointScheme>,
}

/// HTTP transport scheme selected independently of route placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointScheme {
    /// Unencrypted HTTP between the runtime and the service.
    Http,
    /// HTTP with TLS verified against the logical service hostname.
    Https,
}

/// Operator-owned TLS policy, separate from Kubernetes authentication and tunnel encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointTls {
    /// Verify TLS using the logical target hostname and optionally a referenced CA.
    Required,
    /// Explicit operator consent to a plaintext backend protocol.
    Disabled,
}

/// A named credential source; this type cannot carry Secret values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointCredentialReference {
    /// Kubernetes Secret fields mapped to provider-declared credential requirements.
    KubernetesSecret {
        /// Namespace admitted by the source's credential policy.
        namespace: String,
        /// Exact Secret name, never a discovery selector.
        name: String,
        /// Provider credential field to Secret data key; neither side is a value.
        keys: BTreeMap<String, String>,
    },
}

impl Endpoint {
    /// Check portable inventory bounds without performing source or credential access.
    pub fn validate(&self) -> bool {
        [
            &self.endpoint_ref,
            &self.source_ref,
            &self.resource_kind,
            &self.resource_name,
            &self.resource_uid,
            &self.interface,
        ]
        .into_iter()
        .all(|value| reference(value))
            && self.namespace.as_deref().is_none_or(reference)
            && self.port_name.as_deref().is_none_or(reference)
            && self.port.is_none_or(|port| port > 0)
            && self.provider.as_deref().is_none_or(provider_id)
            && self.binding.as_ref().is_none_or(EndpointBinding::validate)
    }
}

impl EndpointBinding {
    /// Validate references and path shape; the runtime additionally enforces route policy.
    pub fn validate(&self) -> bool {
        provider_id(&self.provider)
            && self.database.as_deref().is_none_or(reference)
            && self.base_path.as_deref().is_none_or(|path| {
                path.starts_with('/')
                    && !path.starts_with("//")
                    && path.len() <= 2048
                    && !path
                        .chars()
                        .any(|c| c.is_control() || matches!(c, '?' | '#' | '\\'))
            })
            && self
                .credential
                .as_ref()
                .is_none_or(EndpointCredentialReference::validate)
            && self.direct_address.as_deref().is_none_or(|address| {
                !address.is_empty()
                    && address.len() <= 2048
                    && !address.chars().any(|c| {
                        c.is_whitespace() || c.is_control() || matches!(c, '@' | '?' | '#' | '\\')
                    })
            })
    }
}

impl EndpointCredentialReference {
    /// Check exact names and bounded field mappings, never fetching a Secret.
    pub fn validate(&self) -> bool {
        match self {
            Self::KubernetesSecret {
                namespace,
                name,
                keys,
            } => {
                reference(namespace)
                    && reference(name)
                    && !keys.is_empty()
                    && keys.len() <= 32
                    && keys
                        .iter()
                        .all(|(field, key)| reference(field) && reference(key))
            }
        }
    }
}

fn reference(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn provider_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}
