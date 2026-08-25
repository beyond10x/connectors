//! What a token response has to satisfy before it becomes a stored credential.

use zeroize::Zeroizing;

use crate::OauthError;

/// How a provider spells a scope list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeSeparator {
    /// Space-separated, as RFC 6749 specifies. GitLab, Jira, Slack user tokens.
    Whitespace,
    /// Comma-separated, as several vendors ship regardless.
    Comma,
}

/// How to read the `scope` field.
pub struct ScopePolicy<'a> {
    /// The separator this provider uses.
    pub separator: ScopeSeparator,
    /// Keep only these scopes, dropping anything else the provider volunteered.
    ///
    /// `None` keeps everything. GitLab narrows to `["read_api", "api"]` so that a token granted
    /// extra scopes is recorded as the subset the connector will actually rely on — the stored
    /// scope list is what later authorization reads, so recording more than was asked for would
    /// widen it silently.
    pub retain: Option<&'a [&'a str]>,
}

/// Split, filter, sort and dedupe a `scope` field.
///
/// Sorted and deduped because the result is persisted and compared; a provider that reorders its
/// scope list between calls must not look like a change of grant.
#[must_use]
pub fn parse_scopes(value: &str, policy: &ScopePolicy<'_>) -> Vec<String> {
    let split: Box<dyn Iterator<Item = &str>> = match policy.separator {
        ScopeSeparator::Whitespace => Box::new(value.split_whitespace()),
        ScopeSeparator::Comma => {
            Box::new(value.split(',').map(str::trim).filter(|s| !s.is_empty()))
        }
    };
    let mut scopes: Vec<String> = split
        .filter(|scope| policy.retain.is_none_or(|keep| keep.contains(scope)))
        .map(str::to_owned)
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

/// A token response as the caller deserialized it, before any judgement.
///
/// Owned `String`s rather than a borrowed view because every caller already has an owned
/// deserialization target; [`validate`] is what turns them into zeroizing secrets.
pub struct TokenResponse {
    /// The credential to spend.
    pub access_token: String,
    /// The credential to refresh with, when the provider issued one.
    pub refresh_token: Option<String>,
    /// Lifetime in seconds. Zero is always a refusal.
    pub expires_in: u64,
    /// Issuance instant, where the provider reports one.
    pub created_at: Option<u64>,
    /// The granted scope list, unparsed.
    pub scope: String,
    /// Must equal the policy's expected type.
    pub token_type: String,
}

/// What this provider's token response has to satisfy.
pub struct TokenPolicy<'a> {
    /// Required `token_type`, compared exactly. Every current caller wants `Bearer`.
    pub expect_token_type: &'a str,
    /// Whether a missing or empty refresh token is a refusal.
    ///
    /// GitLab's exchange requires one. Jira requires one on exchange and tolerates its absence on
    /// a refresh response that rotates only the access token.
    pub require_refresh_token: bool,
    /// Scopes that must **all** be present after [`parse_scopes`].
    pub required_scopes: &'a [&'a str],
    /// How to read the scope field.
    pub scopes: ScopePolicy<'a>,
    /// Whether a zero or absent `created_at` is a refusal. GitLab requires it; Jira does not send it.
    pub require_created_at: bool,
    /// Whether a zero `expires_in` is a refusal.
    ///
    /// False for GitLab's refresh response, which is the one place the field is genuinely unused:
    /// that path recomputes expiry from the separate token-info call rather than from this
    /// response, so refusing on it would reject a token the connector never asks about.
    pub require_expires_in: bool,
    /// Longest acceptable secret. Every current caller uses 4096.
    pub max_secret_len: usize,
}

/// A token response that met its policy.
///
/// The secrets are [`Zeroizing`] and this type derives no `Debug`, so it cannot reach a log by
/// accident on the way to the store.
pub struct ValidatedToken {
    /// The credential to spend.
    pub access_token: Zeroizing<String>,
    /// The credential to refresh with, if the provider issued one.
    pub refresh_token: Option<Zeroizing<String>>,
    /// The granted scopes, filtered and canonical.
    pub scopes: Vec<String>,
    /// Lifetime in seconds, as reported.
    pub expires_in: u64,
    /// Issuance instant, as reported.
    pub created_at: Option<u64>,
}

impl ValidatedToken {
    /// When this token stops being usable, measured from `now_unix_ms`.
    ///
    /// From the caller's clock rather than the provider's `created_at`: a provider whose clock
    /// runs behind ours would otherwise produce an expiry we treat as further away than it is,
    /// and the failure mode is a request refused mid-operation rather than a refresh.
    ///
    /// # Errors
    ///
    /// [`OauthError::Clock`] if the addition overflows.
    pub fn expires_at_unix_ms(&self, now_unix_ms: u64) -> Result<u64, OauthError> {
        now_unix_ms
            .checked_add(self.expires_in.saturating_mul(1_000))
            .ok_or(OauthError::Clock)
    }
}

/// Judge a token response against `policy`.
///
/// # Errors
///
/// [`OauthError::TokenResponse`] on any failed condition. Deliberately one error for all of them:
/// the caller surfaces this to a browser at the end of an OAuth redirect, and naming which
/// condition failed would describe the provider's response to whoever triggered the flow.
pub fn validate(
    response: TokenResponse,
    policy: &TokenPolicy<'_>,
) -> Result<ValidatedToken, OauthError> {
    let scopes = parse_scopes(&response.scope, &policy.scopes);
    let refresh = response.refresh_token.unwrap_or_default();
    let refresh_unusable = refresh.is_empty() || refresh.len() > policy.max_secret_len;
    if response.token_type != policy.expect_token_type
        || response.access_token.is_empty()
        || response.access_token.len() > policy.max_secret_len
        || (policy.require_expires_in && response.expires_in == 0)
        || (policy.require_refresh_token && refresh_unusable)
        || (policy.require_created_at && response.created_at.unwrap_or(0) == 0)
        || !policy
            .required_scopes
            .iter()
            .all(|required| scopes.iter().any(|scope| scope == required))
    {
        return Err(OauthError::TokenResponse);
    }
    Ok(ValidatedToken {
        access_token: Zeroizing::new(response.access_token),
        refresh_token: (!refresh.is_empty()).then(|| Zeroizing::new(refresh)),
        scopes,
        expires_in: response.expires_in,
        created_at: response.created_at,
    })
}

/// Whether a credential expiring at `expires_at_unix_ms` should be refreshed now.
///
/// True when the clock is unavailable: a refresh we did not need costs one request, and skipping
/// one we did need costs a failed operation.
///
/// This is the predicate a double-checked refresh calls twice — once before taking the lock and
/// once after — so that a caller queued behind a refresh that already happened does not perform a
/// second one.
#[must_use]
pub fn refresh_due(expires_at_unix_ms: u64, now_unix_ms: Option<u64>, skew_seconds: u64) -> bool {
    now_unix_ms.is_none_or(|now| {
        expires_at_unix_ms <= now.saturating_add(skew_seconds.saturating_mul(1_000))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gitlab_policy() -> TokenPolicy<'static> {
        TokenPolicy {
            expect_token_type: "Bearer",
            require_refresh_token: true,
            required_scopes: &["api"],
            scopes: ScopePolicy {
                separator: ScopeSeparator::Whitespace,
                retain: Some(&["read_api", "api"]),
            },
            require_created_at: true,
            require_expires_in: true,
            max_secret_len: 4_096,
        }
    }

    fn gitlab_response() -> TokenResponse {
        TokenResponse {
            access_token: "SENTINEL-NOT-A-REAL-SECRET".to_owned(),
            refresh_token: Some("SENTINEL-NOT-A-REAL-REFRESH".to_owned()),
            expires_in: 7_200,
            created_at: Some(1_700_000_000),
            scope: "api read_api sudo".to_owned(),
            token_type: "Bearer".to_owned(),
        }
    }

    #[test]
    fn a_conforming_response_validates_and_drops_unretained_scopes() {
        let token = validate(gitlab_response(), &gitlab_policy()).expect("valid");
        assert_eq!(
            token.scopes,
            vec!["api".to_owned(), "read_api".to_owned()],
            "`sudo` was granted but is not retained, so it is not recorded as held"
        );
        assert_eq!(&*token.access_token, "SENTINEL-NOT-A-REAL-SECRET");
        assert!(token.refresh_token.is_some());
    }

    #[test]
    fn every_gitlab_condition_refuses() {
        type Break = fn(&mut TokenResponse);
        let cases: &[(&str, Break)] = &[
            ("token_type", |r| r.token_type = "MAC".to_owned()),
            ("empty access", |r| r.access_token = String::new()),
            ("long access", |r| r.access_token = "a".repeat(4_097)),
            ("no refresh", |r| r.refresh_token = None),
            ("empty refresh", |r| r.refresh_token = Some(String::new())),
            ("long refresh", |r| {
                r.refresh_token = Some("a".repeat(4_097))
            }),
            ("zero expires_in", |r| r.expires_in = 0),
            ("zero created_at", |r| r.created_at = Some(0)),
            ("absent created_at", |r| r.created_at = None),
            ("missing scope", |r| r.scope = "read_api".to_owned()),
        ];
        for (name, break_it) in cases {
            let mut response = gitlab_response();
            break_it(&mut response);
            assert_eq!(
                validate(response, &gitlab_policy()).err(),
                Some(OauthError::TokenResponse),
                "{name} must refuse"
            );
        }
    }

    #[test]
    fn jira_tolerates_an_absent_created_at_and_an_absent_refresh_on_rotation() {
        let policy = TokenPolicy {
            expect_token_type: "Bearer",
            require_refresh_token: false,
            required_scopes: &["read:jira-work", "write:jira-work"],
            scopes: ScopePolicy {
                separator: ScopeSeparator::Whitespace,
                retain: None,
            },
            require_created_at: false,
            require_expires_in: true,
            max_secret_len: 4_096,
        };
        let token = validate(
            TokenResponse {
                access_token: "SENTINEL-NOT-A-REAL-SECRET".to_owned(),
                refresh_token: None,
                expires_in: 3_600,
                created_at: None,
                scope: "write:jira-work read:jira-work offline_access".to_owned(),
                token_type: "Bearer".to_owned(),
            },
            &policy,
        )
        .expect("valid");
        assert!(token.refresh_token.is_none());
        assert_eq!(
            token.scopes,
            vec![
                "offline_access".to_owned(),
                "read:jira-work".to_owned(),
                "write:jira-work".to_owned()
            ],
            "with no retain list every granted scope is recorded, sorted"
        );
    }

    #[test]
    fn a_required_scope_must_survive_the_retain_filter() {
        let policy = TokenPolicy {
            required_scopes: &["api"],
            scopes: ScopePolicy {
                separator: ScopeSeparator::Whitespace,
                retain: Some(&["read_api"]),
            },
            ..gitlab_policy()
        };
        let mut response = gitlab_response();
        response.scope = "api read_api".to_owned();
        assert_eq!(
            validate(response, &policy).err(),
            Some(OauthError::TokenResponse),
            "a retain list that drops a required scope refuses rather than passing on the raw list"
        );
    }

    #[test]
    fn a_zero_expires_in_passes_only_where_the_caller_does_not_rely_on_it() {
        let mut response = gitlab_response();
        response.expires_in = 0;
        assert_eq!(
            validate(response, &gitlab_policy()).err(),
            Some(OauthError::TokenResponse)
        );

        let mut response = gitlab_response();
        response.expires_in = 0;
        let lenient = TokenPolicy {
            require_created_at: false,
            require_expires_in: false,
            ..gitlab_policy()
        };
        assert!(
            validate(response, &lenient).is_ok(),
            "GitLab's refresh path recomputes expiry from token-info, so it never reads this field"
        );
    }

    #[test]
    fn comma_separated_scopes_are_split_and_trimmed() {
        let scopes = parse_scopes(
            "chat:write, users:read ,,chat:write",
            &ScopePolicy {
                separator: ScopeSeparator::Comma,
                retain: None,
            },
        );
        assert_eq!(
            scopes,
            vec!["chat:write".to_owned(), "users:read".to_owned()]
        );
    }

    #[test]
    fn expiry_is_measured_from_our_clock() {
        let token = validate(gitlab_response(), &gitlab_policy()).expect("valid");
        assert_eq!(
            token.expires_at_unix_ms(1_000).expect("no overflow"),
            7_201_000
        );
        assert_eq!(
            token.expires_at_unix_ms(u64::MAX).err(),
            Some(OauthError::Clock)
        );
    }

    #[test]
    fn refresh_is_due_inside_the_skew_and_whenever_the_clock_is_unavailable() {
        assert!(!refresh_due(100_000, Some(0), 60), "far from expiry");
        assert!(
            refresh_due(100_000, Some(50_000), 60),
            "inside the 60s skew"
        );
        assert!(
            refresh_due(100_000, Some(100_000), 0),
            "expiry is inclusive"
        );
        assert!(
            refresh_due(u64::MAX, None, 0),
            "no clock means refresh; one spare request beats one failed operation"
        );
        assert!(
            refresh_due(100_000, Some(u64::MAX), u64::MAX),
            "the skew arithmetic saturates rather than wrapping into `not due`"
        );
    }
}
