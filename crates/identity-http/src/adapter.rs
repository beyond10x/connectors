//! Hosted Identity verifier adapter for short-lived exact-audience access authority.

use std::collections::BTreeSet;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use server::hosted::{
    HostedPrincipal, IdentityVerificationError, IdentityVerifier, CONNECTORS_AUDIENCE,
};
use sha2::Digest as _;
use url::Url;
use zeroize::Zeroizing;

const MAX_IDENTITY_RESPONSE_BYTES: usize = 16 * 1024;

pub struct IdentityHttpVerifier {
    client: reqwest::Client,
    endpoint: Url,
    readiness_endpoint: Url,
    expected_issuer: String,
    expected_tenant: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityVerifierConfigError {
    #[error(
        "Identity origin must be an HTTPS origin without credentials, path, query, or fragment"
    )]
    InvalidIdentityOrigin,
    #[error("hosted tenant binding is invalid")]
    InvalidTenant,
    #[error("Identity verifier HTTP client could not be configured")]
    HttpClient,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationResponse {
    iss: String,
    sub: String,
    aud: String,
    iat: i64,
    nbf: i64,
    exp: i64,
    jti: String,
    act: Actor,
    scope: String,
    principal_kind: String,
    tenant_id: String,
    #[serde(default)]
    email: Option<String>,
    groups: Vec<String>,
    #[serde(default)]
    deployment_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Actor {
    sub: String,
}

impl IdentityHttpVerifier {
    pub fn new(
        mut origin: Url,
        expected_tenant: String,
    ) -> Result<Self, IdentityVerifierConfigError> {
        // Plaintext is admitted for exactly one origin shape, in exactly one kind of build: a
        // loopback development Connector carrying the `local-identity` feature, which
        // `crate::local_identity` documents and which a release build cannot compile. Every other
        // build resolves this constant to `false`, so the HTTPS requirement below and the
        // `https_only` client are the only paths that exist.
        #[cfg(feature = "local-identity")]
        let plaintext_loopback = crate::local_identity::admitted_origin(&origin);
        #[cfg(not(feature = "local-identity"))]
        let plaintext_loopback = false;
        if (origin.scheme() != "https" && !plaintext_loopback)
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || (origin.path() != "/" && !origin.path().is_empty())
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(IdentityVerifierConfigError::InvalidIdentityOrigin);
        }
        if !valid_ref(&expected_tenant, 256) {
            return Err(IdentityVerifierConfigError::InvalidTenant);
        }
        let expected_issuer = origin.as_str().trim_end_matches('/').to_owned();
        let mut readiness_endpoint = origin.clone();
        readiness_endpoint.set_path("/readyz");
        origin.set_path("/v1/access-authority");
        let client = reqwest::Client::builder()
            .https_only(!plaintext_loopback)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| IdentityVerifierConfigError::HttpClient)?;
        Ok(Self {
            client,
            endpoint: origin,
            readiness_endpoint,
            expected_issuer,
            expected_tenant,
        })
    }
}

#[async_trait]
impl IdentityVerifier for IdentityHttpVerifier {
    async fn ready(&self) -> Result<(), IdentityVerificationError> {
        let response = self
            .client
            .get(self.readiness_endpoint.clone())
            .send()
            .await
            .map_err(|_| IdentityVerificationError::Unavailable)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(IdentityVerificationError::Unavailable)
        }
    }

    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError> {
        if audience != CONNECTORS_AUDIENCE || !valid_access_token(credential) {
            return Err(IdentityVerificationError::Refused);
        }
        let credential = Zeroizing::new(credential.to_owned());
        let response = self
            .client
            .get(self.endpoint.clone())
            .bearer_auth(credential.as_str())
            .header("x-b10x-audience", CONNECTORS_AUDIENCE)
            .send()
            .await
            .map_err(|_| IdentityVerificationError::Unavailable)?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(IdentityVerificationError::Refused);
        }
        if !response.status().is_success()
            || response.content_length().is_some_and(|length| {
                length > u64::try_from(MAX_IDENTITY_RESPONSE_BYTES).expect("bound fits u64")
            })
        {
            return Err(IdentityVerificationError::Unavailable);
        }
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| IdentityVerificationError::Unavailable)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_IDENTITY_RESPONSE_BYTES {
                return Err(IdentityVerificationError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let admitted: VerificationResponse =
            serde_json::from_slice(&bytes).map_err(|_| IdentityVerificationError::Unavailable)?;
        let now = unix_time().ok_or(IdentityVerificationError::Unavailable)?;
        let scopes = validate_scopes(&admitted.scope).ok_or(IdentityVerificationError::Refused)?;
        let groups = validate_groups(&admitted.groups).ok_or(IdentityVerificationError::Refused)?;
        if admitted.iss != self.expected_issuer
            || admitted.aud != CONNECTORS_AUDIENCE
            || admitted.tenant_id != self.expected_tenant
            || admitted.iat > admitted.nbf
            || admitted.nbf > admitted.exp
            || admitted.exp.saturating_sub(admitted.iat) > 300
            || admitted.iat > now.saturating_add(30)
            || admitted.nbf > now.saturating_add(30)
            || admitted.exp <= now.saturating_sub(30)
            || !matches!(
                admitted.principal_kind.as_str(),
                "human" | "service" | "deployment"
            )
            || !valid_ref(&admitted.sub, 512)
            || !valid_ref(&admitted.act.sub, 512)
            || !valid_ref(&admitted.jti, 512)
            || admitted.email.as_deref().is_some_and(|email| {
                email.len() > 254
                    || !email.is_ascii()
                    || email.chars().any(char::is_whitespace)
                    || email.split('@').count() != 2
            })
            || admitted
                .deployment_id
                .as_deref()
                .is_some_and(|deployment| !valid_ref(deployment, 512))
        {
            return Err(IdentityVerificationError::Refused);
        }
        let authority_snapshot_sha256 = hex::encode(sha2::Sha256::digest(&bytes));
        Ok(HostedPrincipal {
            issuer: admitted.iss,
            tenant_id: admitted.tenant_id,
            subject: admitted.sub,
            actor_subject: admitted.act.sub,
            email: admitted.email,
            token_id: admitted.jti,
            scopes,
            groups,
            authority_snapshot_sha256,
            deployment_id: admitted.deployment_id,
        })
    }
}

fn validate_groups(value: &[String]) -> Option<BTreeSet<String>> {
    let groups = value.iter().cloned().collect::<BTreeSet<_>>();
    if groups.len() != value.len()
        || groups.iter().any(|group| {
            !(1..=64).contains(&group.len())
                || !group.bytes().enumerate().all(|(index, byte)| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit() && index > 0
                        || matches!(byte, b'-' | b'_') && index > 0
                })
        })
    {
        return None;
    }
    Some(groups)
}

fn valid_access_token(value: &str) -> bool {
    value
        .strip_prefix("identity_access_v1_")
        .is_some_and(|token| {
            token.len() == 43
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}

fn validate_scopes(value: &str) -> Option<BTreeSet<String>> {
    const ALLOWED: [&str; 13] = [
        "connectors.approvals.issue",
        "connectors.audit.read",
        "connectors.catalog.read",
        "connectors.channels.manage",
        "connectors.connections.manage",
        "connectors.connections.self",
        "connectors.credentials.lease",
        "connectors.deliveries.manage",
        "connectors.events.read",
        "connectors.events.self",
        "connectors.grants.manage",
        "connectors.integrations.manage",
        "connectors.invoke",
    ];
    if value.is_empty() || value.len() > 1024 {
        return None;
    }
    let tokens = value.split_ascii_whitespace().collect::<Vec<_>>();
    if tokens.join(" ") != value {
        return None;
    }
    let scopes = tokens
        .iter()
        .map(|scope| (*scope).to_owned())
        .collect::<BTreeSet<_>>();
    if scopes.len() != tokens.len()
        || scopes
            .iter()
            .any(|scope| !ALLOWED.contains(&scope.as_str()))
    {
        return None;
    }
    Some(scopes)
}

fn unix_time() -> Option<i64> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(seconds).ok()
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_issuance_scope_is_admitted() {
        assert_eq!(
            validate_scopes("connectors.approvals.issue"),
            Some(BTreeSet::from(["connectors.approvals.issue".to_owned()]))
        );
    }

    #[test]
    fn hosted_verifier_requires_https_origin_and_closed_access_token_shape() {
        assert!(IdentityHttpVerifier::new(
            Url::parse("http://identity.example.test").unwrap(),
            "tenant-dev".to_owned()
        )
        .is_err());
        assert!(IdentityHttpVerifier::new(
            Url::parse("https://identity.example.test").unwrap(),
            "tenant-dev".to_owned()
        )
        .is_ok());
        assert!(valid_access_token(&format!(
            "identity_access_v1_{}",
            "a".repeat(43)
        )));
        assert!(!valid_access_token("identity_access_v1_short"));
    }

    /// The deployed posture, asserted in every build including the one that carries the loopback
    /// exception: a plaintext Identity origin a second machine could address is refused. This is
    /// the claim the whole `local-identity` guard exists to keep true.
    #[test]
    fn a_routable_plaintext_identity_origin_is_refused_in_every_build() {
        for origin in [
            "http://identity.example.test",
            "http://identity.dev.b10x.example",
            "http://10.0.0.4:18085",
            "http://[2001:db8::1]:18085",
            // 127.0.0.1 spelled as a routable name resolves off this machine.
            "http://loopback.example.test:18085",
        ] {
            assert!(
                IdentityHttpVerifier::new(Url::parse(origin).unwrap(), "tenant-dev".to_owned())
                    .is_err(),
                "plaintext Identity origin {origin} must be refused"
            );
        }
    }

    /// Only under the feature, and only for loopback: the local process stack can run the hosted
    /// surface against its own Identity, and nothing else gains a plaintext door.
    #[cfg(feature = "local-identity")]
    #[test]
    fn the_local_identity_build_admits_only_a_loopback_plaintext_origin() {
        assert!(IdentityHttpVerifier::new(
            Url::parse("http://127.0.0.1:18085").unwrap(),
            "tenant-local".to_owned()
        )
        .is_ok());
        assert!(IdentityHttpVerifier::new(
            Url::parse("http://localhost:18085").unwrap(),
            "tenant-local".to_owned()
        )
        .is_ok());
        assert!(IdentityHttpVerifier::new(
            Url::parse("http://127.0.0.1:18085/v1/access-authority").unwrap(),
            "tenant-local".to_owned()
        )
        .is_err());
    }
}
