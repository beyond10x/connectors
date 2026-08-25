//! Random tokens, the PKCE pair, and the authorize URL.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use url::Url;
use zeroize::Zeroizing;

use crate::OauthError;

/// `bytes` bytes from the system random source, base64url-encoded without padding.
///
/// The encoding is unpadded so the value is safe in a query string and in a URL fragment without
/// escaping, which is what every caller does with it.
///
/// # Errors
///
/// [`OauthError::Randomness`] if the system random source refuses.
pub fn random_token(bytes: usize) -> Result<String, OauthError> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).map_err(|_| OauthError::Randomness)?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

/// A PKCE verifier and its S256 challenge.
///
/// Deliberately no `Debug` and no `Display`: the verifier is the half that proves possession, and
/// a derive here would put it in every caller's error path for free.
pub struct Pkce {
    verifier: Zeroizing<String>,
    challenge: String,
}

impl Pkce {
    /// A fresh 48-byte verifier and the SHA-256 challenge over its ASCII bytes.
    ///
    /// 48 bytes encodes to 64 characters, inside RFC 7636's 43..=128 range.
    ///
    /// # Errors
    ///
    /// [`OauthError::Randomness`] if the system random source refuses.
    pub fn generate() -> Result<Self, OauthError> {
        let verifier = Zeroizing::new(random_token(48)?);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Ok(Self {
            verifier,
            challenge,
        })
    }

    /// The verifier, sent on the token exchange and never stored.
    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The challenge, sent on the authorization request.
    #[must_use]
    pub fn challenge(&self) -> &str {
        &self.challenge
    }

    /// Take the verifier, so a caller can park it in its own pending-state payload.
    #[must_use]
    pub fn into_verifier(self) -> Zeroizing<String> {
        self.verifier
    }
}

/// What an authorization request carries, apart from the provider's origin and path.
///
/// `code_challenge` is `None` for a provider that is not a PKCE client. Jira and Slack are
/// confidential clients that authenticate the exchange with a secret and declare no
/// `public_client`, so sending a challenge for them would be an undeclared protocol change.
pub struct AuthorizeParams<'a> {
    /// The public client identifier. Deployment configuration, never catalog data.
    pub client_id: &'a str,
    /// Where the provider sends the browser back.
    pub redirect_uri: &'a str,
    /// Requested scopes, already in the spelling this provider wants.
    pub scope: &'a str,
    /// The single-use CSRF value, matched on the way back.
    pub state: &'a str,
    /// The S256 challenge, for a PKCE client.
    pub code_challenge: Option<&'a str>,
    /// Extra provider-specific pairs, appended in order — Jira's `audience` and `prompt`.
    pub extra: &'a [(&'a str, &'a str)],
}

/// Build an authorization URL against `origin`, replacing its path with `path`.
///
/// # Errors
///
/// [`OauthError::AuthorizeUrl`] if `origin` cannot carry a path — a non-hierarchical URL such as
/// `mailto:`.
pub fn authorize_url(
    origin: &Url,
    path: &str,
    params: &AuthorizeParams<'_>,
) -> Result<String, OauthError> {
    let mut url = origin.clone();
    if url.cannot_be_a_base() {
        return Err(OauthError::AuthorizeUrl);
    }
    url.set_path(path);
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("client_id", params.client_id)
            .append_pair("redirect_uri", params.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", params.scope)
            .append_pair("state", params.state);
        if let Some(challenge) = params.code_challenge {
            query
                .append_pair("code_challenge", challenge)
                .append_pair("code_challenge_method", "S256");
        }
        for (name, value) in params.extra {
            query.append_pair(name, value);
        }
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_random_token_is_unpadded_url_safe_and_the_requested_width() {
        let value = random_token(32).expect("randomness");
        assert_eq!(
            value.len(),
            43,
            "32 bytes is 43 unpadded base64url characters"
        );
        assert!(
            !value.contains('='),
            "padding would need escaping in a fragment"
        );
        assert!(value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
    }

    #[test]
    fn two_random_tokens_differ() {
        assert_ne!(
            random_token(32).expect("randomness"),
            random_token(32).expect("randomness"),
            "a constant here would make every state value forgeable"
        );
    }

    #[test]
    fn the_pkce_challenge_is_the_s256_of_the_verifier() {
        let pkce = Pkce::generate().expect("randomness");
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(pkce.verifier().as_bytes()));
        assert_eq!(pkce.challenge(), expected);
        assert_eq!(
            pkce.verifier().len(),
            64,
            "48 bytes encodes to 64 characters, inside RFC 7636's 43..=128"
        );
    }

    #[test]
    fn an_authorize_url_carries_the_pkce_pair_only_for_a_public_client() {
        let origin = Url::parse("https://gitlab.example.com").expect("origin");
        let with = authorize_url(
            &origin,
            "/oauth/authorize",
            &AuthorizeParams {
                client_id: "cid",
                redirect_uri: "https://b10x.example/callback",
                scope: "api",
                state: "st",
                code_challenge: Some("ch"),
                extra: &[],
            },
        )
        .expect("authorize url");
        assert!(with.contains("code_challenge=ch"));
        assert!(with.contains("code_challenge_method=S256"));

        let without = authorize_url(
            &origin,
            "/oauth/authorize",
            &AuthorizeParams {
                client_id: "cid",
                redirect_uri: "https://b10x.example/callback",
                scope: "api",
                state: "st",
                code_challenge: None,
                extra: &[],
            },
        )
        .expect("authorize url");
        assert!(!without.contains("code_challenge"));
        assert!(!without.contains("S256"));
    }

    #[test]
    fn authorize_url_percent_encodes_the_redirect_and_appends_extras_in_order() {
        let origin = Url::parse("https://auth.atlassian.com").expect("origin");
        let built = authorize_url(
            &origin,
            "/authorize",
            &AuthorizeParams {
                client_id: "cid",
                redirect_uri: "https://b10x.example/cb?x=1",
                scope: "read:jira-work write:jira-work",
                state: "st",
                code_challenge: None,
                extra: &[("audience", "api.atlassian.com"), ("prompt", "consent")],
            },
        )
        .expect("authorize url");
        assert!(built.contains("redirect_uri=https%3A%2F%2Fb10x.example%2Fcb%3Fx%3D1"));
        assert!(built.contains("scope=read%3Ajira-work+write%3Ajira-work"));
        let audience = built.find("audience=").expect("audience");
        let prompt = built.find("prompt=").expect("prompt");
        assert!(audience < prompt, "extras keep the caller's order");
    }

    #[test]
    fn authorize_url_refuses_a_url_that_cannot_carry_a_path() {
        let origin = Url::parse("mailto:someone@example.com").expect("origin");
        assert_eq!(
            authorize_url(
                &origin,
                "/oauth/authorize",
                &AuthorizeParams {
                    client_id: "cid",
                    redirect_uri: "https://b10x.example/callback",
                    scope: "api",
                    state: "st",
                    code_challenge: None,
                    extra: &[],
                },
            ),
            Err(OauthError::AuthorizeUrl)
        );
    }
}
