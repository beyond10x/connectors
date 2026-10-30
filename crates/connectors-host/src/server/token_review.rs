//! The `service-account-token` admission profile
//! (contracts/service/compatibility.md § 2.1, Admission;
//! `connectors.declarations.ServiceAdmissionProfile`).
//!
//! For each request the host asks the cluster API with one TokenReview whether
//! the presented projected token is authenticated for the configured audience,
//! and admits the call only as the one configured service account. Every path
//! that is not that exact positive answer refuses:
//!
//! - a definite review that does not name the configured audience and
//!   principal as authenticated is `unauthorized`;
//! - anything else (no answer, a timeout, another HTTP status, an oversized or
//!   undecodable body, the instance's own token unusable) is `unavailable`;
//! - a request that finds all [`CONCURRENT_REVIEWS`] review slots taken is
//!   `unavailable` at once: no review is sent and nothing waits, so callers
//!   presenting any bearer cannot queue reviews against the instance's own
//!   account.
//!
//! Nothing is cached across requests. The presented token is sent only in the
//! review body and never logged.
use crate::{
    credentials::CredentialRef,
    http::{HttpConfig, ScopedHttp},
};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::AuthProbe;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

/// The most TokenReviews one host has in flight. Admission runs before the
/// host's concurrency permit, so this is what bounds the reviews a stream of
/// requests can start (compatibility.md § 2.1, Admission).
pub const CONCURRENT_REVIEWS: usize = 8;

const API_VERSION: &str = "authentication.k8s.io/v1";
const KIND: &str = "TokenReview";
/// `POST <api>/apis/authentication.k8s.io/v1/tokenreviews`, fixed at start.
const REVIEW_PATH: [&str; 4] = ["apis", "authentication.k8s.io", "v1", "tokenreviews"];
const SERVICE_ACCOUNT_PREFIX: &str = "system:serviceaccount:";
/// The anchor records the principal verbatim, under audit.md § 4's bound.
pub const PRINCIPAL_LIMIT: usize = 128;
pub const AUDIENCE_LIMIT: usize = 256;
/// The longest presented token sent for review; a longer one is refused as
/// `unauthorized` without a review.
const TOKEN_LIMIT: usize = 8 * 1024;
/// The whole review, from the instance's token read to the decoded answer.
const REVIEW_DEADLINE: Duration = Duration::from_secs(5);
/// The largest review answer decoded; a larger one is `unavailable`.
const REVIEW_RESPONSE_LIMIT: usize = 64 * 1024;

/// `token_review` of `urn:connectors:config:v3:service`
/// (`connectors.declarations.TokenReviewEndpoint`).
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenReviewConfig {
    /// The cluster API's HTTPS base URL, such as `https://kubernetes.default.svc`.
    #[schemars(length(min = 1, max = 4096))]
    pub api: String,
    /// The CA bundle that alone roots the API's TLS, read at start.
    #[schemars(with = "String", length(min = 1, max = 4096))]
    pub ca_file: PathBuf,
    /// The instance's own token, read again for each review: private,
    /// owner-bound, at most 8 KiB, and not a symbolic link as the last path
    /// component (for a projected token, `<mount>/..data/token`).
    #[schemars(with = "String", length(min = 1, max = 4096))]
    pub credential_file: PathBuf,
}

/// A started `service-account-token` admission: one audience, one principal,
/// one review endpoint.
pub struct ServiceAccountAdmission {
    audience: String,
    principal: String,
    credential: CredentialRef,
    review: Arc<dyn AuthProbe>,
    deadline: Duration,
    /// [`CONCURRENT_REVIEWS`] slots; one is held for each review in flight.
    slots: Semaphore,
}

impl ServiceAccountAdmission {
    /// Checks the audience and principal and builds the review client: HTTPS
    /// only, rooted only in `ca_file`, no proxy, redirect or retry
    /// ([`ScopedHttp`]).
    pub(crate) fn new(audience: &str, principal: &str, config: &TokenReviewConfig) -> Result<Self> {
        if !valid_audience(audience) {
            return Err(Error::invalid(
                "the admission audience must be 1 to 256 bytes without control characters",
            ));
        }
        if !valid_principal(principal) {
            return Err(Error::invalid(
                "the admission principal must be system:serviceaccount:<namespace>:<name> within 128 bytes",
            ));
        }
        let credential = CredentialRef::File {
            path: config.credential_file.clone(),
        };
        let http = ScopedHttp::new(
            &HttpConfig {
                base_url: config.api.clone(),
                credential: None,
                credential_header: "authorization".into(),
                bearer: true,
                allow_plaintext: false,
                ca_file: Some(config.ca_file.clone()),
            },
            Some(Arc::new(credential.clone())),
        )?;
        Ok(Self {
            audience: audience.to_owned(),
            principal: principal.to_owned(),
            credential,
            review: http.probe_capability(&REVIEW_PATH)?,
            deadline: REVIEW_DEADLINE,
            slots: Semaphore::new(CONCURRENT_REVIEWS),
        })
    }

    /// The one principal this host admits; the anchor's `principal_ref`.
    pub fn principal(&self) -> &str {
        &self.principal
    }

    /// Host start: the instance's own token must be readable now. It is read
    /// again for every review.
    pub(crate) async fn ready(&self) -> Result<()> {
        use connectors_sdk::Credential;
        self.credential
            .resolve()
            .await
            .map(drop)
            .map_err(|_| Error::invalid("the token_review credential_file cannot be read"))
    }

    /// Admits `token` only on a review that authenticates it for the
    /// configured audience as the configured principal.
    pub(crate) async fn admit(&self, token: &str) -> Result<()> {
        // Refusal: no token, or one over the bound, is never sent for review.
        if token.is_empty() {
            return Err(unauthorized("service authentication required"));
        }
        if token.len() > TOKEN_LIMIT {
            return Err(unauthorized("service authentication failed"));
        }
        // Refusal: every review slot is taken. Nothing is sent and nothing
        // waits for a slot; the slot is held until this review is decided.
        let Ok(_slot) = self.slots.try_acquire() else {
            return Err(unavailable("review capacity"));
        };
        let request = json!({
            "apiVersion": API_VERSION,
            "kind": KIND,
            "spec": {"token": token, "audiences": [self.audience]},
        });
        // Fail closed: no answer within the deadline is `unavailable`.
        let answer = match tokio::time::timeout(self.deadline, self.review.probe(&request)).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => return Err(unavailable("no answer")),
            Err(_) => return Err(unavailable("deadline")),
        };
        // Fail closed: only a 2xx carries a decision. A 401 or 403 here is the
        // cluster refusing the instance's own token, never the caller's.
        if !(200..300).contains(&answer.status) {
            tracing::warn!(status = answer.status, "token review unavailable");
            return Err(unavailable("status"));
        }
        if answer.body.len() > REVIEW_RESPONSE_LIMIT {
            return Err(unavailable("oversized answer"));
        }
        let Ok(review) = serde_json::from_slice::<Review>(&answer.body) else {
            return Err(unavailable("undecodable answer"));
        };
        if review.api_version != API_VERSION || review.kind != KIND {
            return Err(unavailable("not a TokenReview"));
        }
        decide(&review.status, &self.audience, &self.principal)
    }

    #[cfg(all(test, unix))]
    fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }
}

/// The decision on a decoded review. All three facts are required; each
/// missing or different one refuses as `unauthorized`.
fn decide(status: &ReviewStatus, audience: &str, principal: &str) -> Result<()> {
    if status.authenticated != Some(true) {
        tracing::info!(reason = "not authenticated", "token review refused");
        return Err(unauthorized("service authentication failed"));
    }
    if !status
        .audiences
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|reviewed| reviewed == audience)
    {
        tracing::info!(reason = "audience", "token review refused");
        return Err(unauthorized("service authentication failed"));
    }
    let username = status
        .user
        .as_ref()
        .and_then(|user| user.username.as_deref());
    if username != Some(principal) {
        tracing::info!(reason = "principal", "token review refused");
        return Err(unauthorized("service authentication failed"));
    }
    Ok(())
}

fn unauthorized(message: &str) -> Error {
    Error::new(ErrorCode::Unauthorized, message)
}

fn unavailable(reason: &'static str) -> Error {
    tracing::warn!(reason, "token review unavailable");
    Error::new(
        ErrorCode::Unavailable,
        "service authentication is unavailable",
    )
}

/// The part of a TokenReview answer admission reads; other members are
/// ignored. `authenticated` is omitted by the cluster when false.
#[derive(Deserialize)]
struct Review {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    #[serde(default)]
    status: ReviewStatus,
}
#[derive(Default, Deserialize)]
struct ReviewStatus {
    authenticated: Option<bool>,
    audiences: Option<Vec<String>>,
    user: Option<ReviewUser>,
}
#[derive(Deserialize)]
struct ReviewUser {
    username: Option<String>,
}

fn valid_audience(audience: &str) -> bool {
    !audience.is_empty()
        && audience.len() <= AUDIENCE_LIMIT
        && !audience.chars().any(char::is_control)
}

/// `system:serviceaccount:<namespace>:<name>`: a DNS-label namespace and a
/// DNS-subdomain name, the whole at most [`PRINCIPAL_LIMIT`] bytes.
pub(crate) fn valid_principal(principal: &str) -> bool {
    if principal.len() > PRINCIPAL_LIMIT {
        return false;
    }
    let Some((namespace, name)) = principal
        .strip_prefix(SERVICE_ACCOUNT_PREFIX)
        .and_then(|rest| rest.split_once(':'))
    else {
        return false;
    };
    dns_label(namespace) && name.len() <= 253 && name.split('.').all(dns_label)
}

fn dns_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    (1..=63).contains(&bytes.len())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        && bytes.first() != Some(&b'-')
        && bytes.last() != Some(&b'-')
}

/// The JSON Schema pattern of [`valid_principal`] (its byte bound is the
/// schema's `maxLength`).
pub(crate) const PRINCIPAL_PATTERN: &str = "^system:serviceaccount:[a-z0-9]([-a-z0-9]{0,61}[a-z0-9])?:[a-z0-9]([-a-z0-9]{0,61}[a-z0-9])?(\\.[a-z0-9]([-a-z0-9]{0,61}[a-z0-9])?)*$";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn principals_are_service_accounts_within_the_anchor_bound() {
        for valid in [
            "system:serviceaccount:agents:example-agent",
            "system:serviceaccount:a:b",
            "system:serviceaccount:agents:example.agent-1",
        ] {
            assert!(valid_principal(valid), "{valid}");
        }
        let long = format!("system:serviceaccount:agents:{}", "a".repeat(100));
        for invalid in [
            "",
            "system:serviceaccount:agents",
            "system:serviceaccount:agents:",
            "system:serviceaccount::example-agent",
            "system:serviceaccount:Agents:example-agent",
            "system:serviceaccount:agents:example-agent:extra",
            "system:serviceaccount:agents:-example",
            "system:serviceaccount:agents:example..agent",
            "system:serviceaccounts:agents",
            "user:example-agent",
            long.as_str(),
        ] {
            assert!(!valid_principal(invalid), "{invalid}");
        }
    }

    fn status(value: serde_json::Value) -> ReviewStatus {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn only_all_three_facts_admit() {
        let principal = "system:serviceaccount:agents:example-agent";
        let admitted = status(
            json!({"authenticated": true, "audiences": ["aud"], "user": {"username": principal}}),
        );
        assert!(decide(&admitted, "aud", principal).is_ok());
        for refused in [
            json!({}),
            json!({"audiences": ["aud"], "user": {"username": principal}}),
            json!({"authenticated": false, "audiences": ["aud"], "user": {"username": principal}}),
            json!({"authenticated": true, "user": {"username": principal}}),
            json!({"authenticated": true, "audiences": ["other"], "user": {"username": principal}}),
            json!({"authenticated": true, "audiences": ["aud"]}),
            json!({"authenticated": true, "audiences": ["aud"], "user": {}}),
            json!({"authenticated": true, "audiences": ["aud"], "user": {"username": "system:serviceaccount:agents:other"}}),
        ] {
            let error = decide(&status(refused.clone()), "aud", principal).unwrap_err();
            assert_eq!(error.code, ErrorCode::Unauthorized, "{refused}");
        }
    }

    /// A review endpoint that accepts and never answers is `unavailable` at the
    /// deadline, not a hang.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_silent_review_is_unavailable_at_its_deadline() {
        let root = tempfile::tempdir().unwrap();
        let ca = root.path().join("ca.crt");
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        std::fs::write(&ca, certificate.cert.pem()).unwrap();
        let token = root.path().join("token");
        std::fs::write(&token, b"instance-token").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let silent = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((stream, _)) = listener.accept().await {
                held.push(stream);
            }
        });
        let admission = ServiceAccountAdmission::new(
            "aud",
            "system:serviceaccount:agents:example-agent",
            &TokenReviewConfig {
                api: format!("https://localhost:{port}"),
                ca_file: ca,
                credential_file: token,
            },
        )
        .unwrap()
        .with_deadline(Duration::from_millis(200));
        let started = std::time::Instant::now();
        let error = admission.admit("caller-token").await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert!(started.elapsed() < Duration::from_secs(5));
        silent.abort();
    }
}
