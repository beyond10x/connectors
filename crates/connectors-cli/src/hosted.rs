//! Hosted Identity verifier adapter. Opaque sessions are forwarded only to their Identity owner.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use server::hosted::{
    HostedPrincipal, IdentityVerificationError, IdentityVerifier, CONNECTORS_AUDIENCE,
};
use url::Url;
use zeroize::Zeroizing;

const MAX_IDENTITY_RESPONSE_BYTES: usize = 16 * 1024;

pub struct IdentityHttpVerifier {
    client: reqwest::Client,
    endpoint: Url,
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
    active: bool,
    audience: String,
    tenant_id: String,
    subject: String,
    email: Option<String>,
    expires_in: i64,
}

impl IdentityHttpVerifier {
    pub fn new(
        mut origin: Url,
        expected_tenant: String,
    ) -> Result<Self, IdentityVerifierConfigError> {
        if origin.scheme() != "https"
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
        origin.set_path("/v1/session");
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| IdentityVerifierConfigError::HttpClient)?;
        Ok(Self {
            client,
            endpoint: origin,
            expected_tenant,
        })
    }
}

#[async_trait]
impl IdentityVerifier for IdentityHttpVerifier {
    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError> {
        if audience != CONNECTORS_AUDIENCE || !valid_session(credential) {
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
        let bytes = response
            .bytes()
            .await
            .map_err(|_| IdentityVerificationError::Unavailable)?;
        if bytes.len() > MAX_IDENTITY_RESPONSE_BYTES {
            return Err(IdentityVerificationError::Unavailable);
        }
        let admitted: VerificationResponse =
            serde_json::from_slice(&bytes).map_err(|_| IdentityVerificationError::Unavailable)?;
        let _ = admitted.email;
        if !admitted.active
            || admitted.audience != CONNECTORS_AUDIENCE
            || admitted.tenant_id != self.expected_tenant
            || admitted.expires_in <= 0
            || !valid_ref(&admitted.subject, 512)
        {
            return Err(IdentityVerificationError::Refused);
        }
        Ok(HostedPrincipal {
            tenant_id: admitted.tenant_id,
            subject: admitted.subject,
        })
    }
}

fn valid_session(value: &str) -> bool {
    value.strip_prefix("dl_session_v1_").is_some_and(|token| {
        token.len() == 43
            && token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_verifier_requires_https_origin_and_closed_session_shape() {
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
        assert!(valid_session(&format!("dl_session_v1_{}", "a".repeat(43))));
        assert!(!valid_session("dl_session_v1_short"));
    }
}
