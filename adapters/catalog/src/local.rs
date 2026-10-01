//! Executable composition for one provider bundle: native configuration and
//! protected entry terminate here. The engine receives only an immutable
//! authenticated HTTP port; the host keeps admission, approval and the ledger.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem, registry,
        runtime::{
            self, Acquisition, Baseline, Bootstrap, EntryField, Failure, Profile, Requirement,
            Result,
        },
    },
};
use connectors_sdk::{AuthenticatedHttp as _, Credential, Secret};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zeroize::{Zeroize as _, Zeroizing};

pub const FORMAT: &str = "connectors-catalog-local/2";
/// A selection set shipped with the repository, referenced by `operations_file`.
pub const OPERATIONS_FORMAT: &str = "connectors-catalog-operations/1";
const DOCUMENT_LIMIT: usize = 64 * 1024;

/// Where the subject of a credential comes from.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IdentitySource {
    /// A read against the provider API: `path` and `subject_pointer`.
    #[default]
    Api,
    /// An `oauth2_refresh` profile's token response: its `id_token`, or the
    /// token host's `tokeninfo` when the token endpoint returns none.
    IdToken,
}
impl IdentitySource {
    fn is_api(&self) -> bool {
        *self == Self::Api
    }
}

/// A read the profile performs to learn who the credential is, and optionally
/// which scopes it was granted. Paths are relative to the provider authority.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityProbe {
    /// Required for an API identity, absent for an `id_token` one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subject_pointer: Option<String>,
    /// Omitted for an API identity, so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "IdentitySource::is_api")]
    source: IdentitySource,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopesProbe {
    path: String,
    pointer: String,
}
/// How the protected entry becomes the credential header.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Scheme {
    /// `{"token"}` travels in `header`, prefixed `Bearer ` when `bearer` is set.
    #[default]
    Token,
    /// `{"account","token"}` travels as `Authorization: Basic base64(account:token)`.
    Basic,
    /// `{"client_id","client_secret","refresh_token"}` is exchanged at
    /// `token_url` for an access token sent as `Authorization: Bearer`.
    #[serde(rename = "oauth2_refresh")]
    OAuth2Refresh,
}
impl Scheme {
    fn is_token(&self) -> bool {
        *self == Self::Token
    }
}

/// The declarative authentication profile: where the token goes and how the
/// provider is asked who holds it. No credential value lives here.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthConfig {
    profile: String,
    /// Omitted for a token profile, so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "Scheme::is_token")]
    scheme: Scheme,
    header: String,
    bearer: bool,
    /// The prompt label of the basic `account` field; only a basic profile has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    account_label: Option<String>,
    label: String,
    identity: IdentityProbe,
    #[serde(default)]
    scopes: Option<ScopesProbe>,
    #[serde(default)]
    minimum_scopes: BTreeSet<String>,
    #[serde(default = "default_evidence_lifetime")]
    evidence_lifetime_ms: u64,
    // The four OAuth fields below belong to `oauth2_refresh` only and are
    // omitted otherwise, so existing configuration revisions hold.
    /// The token endpoint; https only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_url: Option<String>,
    /// Trust roots for `token_url` only; the platform roots when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_ca_file: Option<PathBuf>,
    /// Where consent is given; https only. The provider never calls it: it is
    /// acquisition metadata the host passes on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authorize_url: Option<String>,
    /// The scopes requested at consent. `scopes` is the granted-scope probe.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    requested_scopes: BTreeSet<String>,
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
    /// The leading part of the `api_base` path that an API gateway adds in
    /// front of the document's paths, such as `/ex/jira/<cloud id>`. Requests
    /// still go to the full `api_base`; only the document's base-path check
    /// starts after it.
    #[serde(default)]
    request_prefix: Option<String>,
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
fn optional_token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    token(deserializer).map(Some)
}
/// The basic profile's protected entry. The same token rules apply; the account
/// may be any printable text without a colon, which basic cannot carry.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BasicEntry {
    #[serde(deserialize_with = "account")]
    account: Zeroizing<String>,
    #[serde(deserialize_with = "token")]
    token: Zeroizing<String>,
}
const ACCOUNT_LIMIT: usize = 1024;
fn account<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty()
        || value.len() > ACCOUNT_LIMIT
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
}
/// `Basic ` followed by the standard padded base64 of `account:token`. Every
/// buffer holding credential bytes is sized up front and zeroized on drop.
fn basic_header(entry: &BasicEntry) -> Result<Secret> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    const PREFIX: &[u8] = b"Basic ";
    let length = entry.account.len() + 1 + entry.token.len();
    let mut joined = Zeroizing::new(Vec::with_capacity(length));
    joined.extend_from_slice(entry.account.as_bytes());
    joined.push(b':');
    joined.extend_from_slice(entry.token.as_bytes());
    let encoded = base64::encoded_len(length, true).ok_or(Failure::InvalidInput)?;
    let mut header = Secret(vec![0; PREFIX.len() + encoded]);
    header.0[..PREFIX.len()].copy_from_slice(PREFIX);
    let written = STANDARD
        .encode_slice(joined.as_slice(), &mut header.0[PREFIX.len()..])
        .map_err(|_| Failure::InvalidInput)?;
    if written != encoded {
        return Err(Failure::InvalidInput);
    }
    Ok(header)
}

/// The `oauth2_refresh` profile's protected entry. Every field follows the
/// token rules; the stored material never changes after acquisition.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OAuthEntry {
    #[serde(deserialize_with = "token")]
    client_id: Zeroizing<String>,
    #[serde(deserialize_with = "token")]
    client_secret: Zeroizing<String>,
    #[serde(deserialize_with = "token")]
    refresh_token: Zeroizing<String>,
}

/// A token endpoint's successful answer. Fields it adds are ignored.
#[derive(Deserialize)]
struct TokenAnswer {
    #[serde(deserialize_with = "token")]
    access_token: Zeroizing<String>,
    token_type: String,
    expires_in: u64,
    #[serde(default, deserialize_with = "optional_token")]
    refresh_token: Option<Zeroizing<String>>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default, deserialize_with = "optional_token")]
    id_token: Option<Zeroizing<String>>,
}
/// A token endpoint's refusal; only its code is read.
#[derive(Deserialize)]
struct TokenRefusal {
    error: String,
}
/// The claims of an `id_token` this profile reads.
#[derive(Deserialize)]
struct IdClaims {
    iss: String,
    aud: String,
    sub: String,
    /// Seconds since the epoch; the token is refused unless this is after now.
    exp: u64,
    /// Seconds since the epoch; refused when further ahead than the skew.
    #[serde(default)]
    iat: Option<u64>,
}
/// How far an `id_token` may claim to be issued ahead of this clock.
const ISSUED_AT_SKEW_S: u64 = 5 * 60;
/// The token host's `tokeninfo` answer for an access token.
#[derive(Deserialize)]
struct TokenInfo {
    aud: String,
    sub: String,
    #[serde(default)]
    scope: String,
}
/// The issuers an `id_token` may name.
const ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
/// An access token is used until this long before the endpoint said it expires.
const EXPIRY_SKEW_S: u64 = 60;
/// The longest an access token is cached, whatever the endpoint says.
const CACHE_LIFETIME_S: u64 = 24 * 60 * 60;
const CACHE_ENTRIES: usize = 64;

/// One exchange's result. The access token lives in process memory only.
struct Exchanged {
    access: Zeroizing<String>,
    scope: Option<String>,
    id_token: Option<Zeroizing<String>>,
    /// When the access token stops being used: `expires_in - 60 s` after the
    /// request was sent, or `None` when that is not in the future.
    usable_until: Option<Instant>,
}
struct Cached {
    access: Zeroizing<String>,
    until: Instant,
}
/// The token endpoint of an `oauth2_refresh` profile and the access tokens it
/// issued, keyed by a digest of the protected entry that obtained each.
struct OAuth {
    http: ScopedHttp,
    path: Vec<String>,
    cache: Mutex<BTreeMap<[u8; 32], Cached>>,
}
impl OAuth {
    fn cache(&self) -> std::sync::MutexGuard<'_, BTreeMap<[u8; 32], Cached>> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn cached(&self, key: &[u8; 32]) -> Option<Zeroizing<String>> {
        let now = Instant::now();
        let mut cache = self.cache();
        cache.retain(|_, cached| cached.until > now);
        cache.get(key).map(|cached| cached.access.clone())
    }
    fn store(&self, key: [u8; 32], exchanged: &Exchanged) {
        let now = Instant::now();
        let mut cache = self.cache();
        cache.retain(|_, cached| cached.until > now);
        let Some(until) = exchanged.usable_until.filter(|until| *until > now) else {
            cache.remove(&key);
            return;
        };
        if cache.len() >= CACHE_ENTRIES && !cache.contains_key(&key) {
            cache.pop_first();
        }
        cache.insert(
            key,
            Cached {
                access: exchanged.access.clone(),
                until,
            },
        );
    }
    fn evict(&self, key: &[u8; 32]) {
        self.cache().remove(key);
    }
    /// One refresh-token grant. Every refusal is a code; no answer text,
    /// status line or credential reaches it.
    async fn exchange(&self, entry: &OAuthEntry) -> Result<Exchanged> {
        let sent = Instant::now();
        let segments: Vec<&str> = self.path.iter().map(String::as_str).collect();
        let response = self
            .http
            .post_form(
                &segments,
                &[
                    ("grant_type", b"refresh_token".as_slice()),
                    ("client_id", entry.client_id.as_bytes()),
                    ("client_secret", entry.client_secret.as_bytes()),
                    ("refresh_token", entry.refresh_token.as_bytes()),
                ],
            )
            .await
            .map_err(Failure::from_provider)?;
        let status = response.status;
        let body = Zeroizing::new(response.body);
        match status {
            200 => {}
            400 | 401 => {
                let refusal = (body.len() <= DOCUMENT_LIMIT)
                    .then(|| serde_json::from_slice::<TokenRefusal>(&body).ok())
                    .flatten();
                return Err(match refusal.as_ref().map(|r| r.error.as_str()) {
                    Some("invalid_grant" | "invalid_client") => Failure::InvalidCredential,
                    _ if status == 401 => Failure::InvalidCredential,
                    _ => Failure::Protocol,
                });
            }
            429 => return Err(Failure::ProviderRateLimited),
            500..=599 => return Err(Failure::Unavailable),
            _ => return Err(Failure::Protocol),
        }
        if body.len() > DOCUMENT_LIMIT {
            return Err(Failure::Protocol);
        }
        let answer: TokenAnswer = serde_json::from_slice(&body).map_err(|_| Failure::Protocol)?;
        if !answer.token_type.eq_ignore_ascii_case("bearer") || answer.expires_in == 0 {
            return Err(Failure::Protocol);
        }
        // The profile is declared non-rotating, and there is no path to publish
        // rotated material: a different refresh token is refused, not stored.
        if answer
            .refresh_token
            .as_ref()
            .is_some_and(|rotated| rotated.as_bytes() != entry.refresh_token.as_bytes())
        {
            return Err(Failure::InvalidCredential);
        }
        let usable = answer
            .expires_in
            .min(CACHE_LIFETIME_S)
            .saturating_sub(EXPIRY_SKEW_S);
        Ok(Exchanged {
            access: answer.access_token,
            scope: answer.scope,
            id_token: answer.id_token,
            usable_until: (usable > 0)
                .then(|| sent.checked_add(Duration::from_secs(usable)))
                .flatten(),
        })
    }
    /// The subject and granted scopes of a fresh exchange: from its `id_token`,
    /// or from the token host's `tokeninfo` when the endpoint returned none.
    async fn identity(
        &self,
        entry: &OAuthEntry,
        exchanged: &Exchanged,
    ) -> Result<(String, String)> {
        if let Some(id_token) = &exchanged.id_token {
            // The token came straight from the token endpoint over verified TLS,
            // which OpenID Connect Core 3.1.3.7 accepts in place of checking its
            // signature; issuer and audience are still checked.
            let claims = id_claims(id_token)?;
            // 3.1.3.7 also requires the current time before `exp`.
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Failure::Protocol)?
                .as_secs();
            if !ISSUERS.contains(&claims.iss.as_str())
                || claims.aud.as_bytes() != entry.client_id.as_bytes()
                || claims.exp <= now
                || claims
                    .iat
                    .is_some_and(|iat| iat > now.saturating_add(ISSUED_AT_SKEW_S))
            {
                return Err(Failure::Protocol);
            }
            return Ok((
                claims.sub,
                exchanged.scope.clone().ok_or(Failure::Protocol)?,
            ));
        }
        let mut query = [("access_token", String::from(exchanged.access.as_str()))];
        let response = self.http.get(&["tokeninfo"], &query).await;
        query[0].1.zeroize();
        let response = response.map_err(Failure::from_provider)?;
        match response.status {
            200 => {}
            400 | 401 => return Err(Failure::InvalidCredential),
            429 => return Err(Failure::ProviderRateLimited),
            500..=599 => return Err(Failure::Unavailable),
            _ => return Err(Failure::Protocol),
        }
        if response.body.len() > DOCUMENT_LIMIT {
            return Err(Failure::Protocol);
        }
        let info: TokenInfo =
            serde_json::from_slice(&response.body).map_err(|_| Failure::Protocol)?;
        if info.aud.as_bytes() != entry.client_id.as_bytes() {
            return Err(Failure::Protocol);
        }
        Ok((info.sub, info.scope))
    }
}
/// The payload of a compact JWS, read without its signature.
fn id_claims(id_token: &str) -> Result<IdClaims> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut parts = id_token.split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Failure::Protocol);
    };
    let payload = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| Failure::Protocol)?;
    serde_json::from_slice(&payload).map_err(|_| Failure::Protocol)
}
/// A space-separated OAuth scope string as a bounded set.
fn scope_set(scope: &str) -> Result<BTreeSet<String>> {
    let mut granted = BTreeSet::new();
    for scope in scope.split_ascii_whitespace() {
        if scope.len() > 256 || !scope.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(Failure::Protocol);
        }
        granted.insert(scope.to_owned());
    }
    if granted.len() > 64 {
        return Err(Failure::Protocol);
    }
    Ok(granted)
}
/// Scopes a configuration requests: each a bounded printable word.
fn valid_scopes(scopes: &BTreeSet<String>) -> bool {
    !scopes.is_empty()
        && scopes.len() <= 64
        && scopes
            .iter()
            .all(|s| !s.is_empty() && s.len() <= 256 && s.bytes().all(|b| b.is_ascii_graphic()))
}
/// A granted scope set is split on whitespace, so an entry that is empty or
/// holds whitespace could never be granted.
fn grantable(scopes: &BTreeSet<String>) -> bool {
    scopes
        .iter()
        .all(|s| !s.is_empty() && !s.chars().any(char::is_whitespace))
}
/// An https URL with no credentials, query or fragment, written as its own
/// canonical form. Returns that form with a trailing `/`.
fn https_url(value: &str) -> Result<String> {
    let canonical =
        connectors_host::http::canonical_base(value).map_err(|_| Failure::InvalidConfiguration)?;
    if !canonical.starts_with("https://")
        || (canonical != value && canonical.strip_suffix('/') != Some(value))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(canonical)
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
    oauth: Option<Arc<OAuth>>,
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
        let oauth = config.auth.scheme == Scheme::OAuth2Refresh;
        let identity = &config.auth.identity;
        if config.format != FORMAT
            || !connectors_core::valid_id(&config.instance)
            || !connectors_core::valid_id(&config.provider)
            || !connectors_core::valid_id(&config.auth.profile)
            || config.auth.header.is_empty()
            || config.auth.label.is_empty()
            || config.auth.label.len() > 128
            || config.auth.evidence_lifetime_ms == 0
            || config.auth.evidence_lifetime_ms > 300_000
            || !grantable(&config.auth.minimum_scopes)
            || !grantable(&config.auth.requested_scopes)
            || match identity.source {
                IdentitySource::Api => {
                    identity
                        .path
                        .as_deref()
                        .is_none_or(|p| segments(p).is_empty())
                        || identity.subject_pointer.is_none()
                        || (config.auth.scopes.is_none() && !config.auth.minimum_scopes.is_empty())
                }
                // The token response names the subject and its scopes; an API
                // probe beside it would be read by nobody.
                IdentitySource::IdToken => {
                    !oauth
                        || identity.path.is_some()
                        || identity.subject_pointer.is_some()
                        || config.auth.scopes.is_some()
                }
            }
            || config
                .auth
                .scopes
                .as_ref()
                .is_some_and(|s| segments(&s.path).is_empty())
            || match config.auth.scheme {
                Scheme::Token => config.auth.account_label.is_some(),
                // Basic has one fixed placement; the file states it rather than
                // leaving a reader to infer that `header` is ignored.
                Scheme::Basic => {
                    !config.auth.header.eq_ignore_ascii_case("authorization")
                        || config.auth.bearer
                        || config
                            .auth
                            .account_label
                            .as_ref()
                            .is_none_or(|label| label.is_empty() || label.len() > 128)
                }
                // So has an OAuth access token.
                Scheme::OAuth2Refresh => {
                    !config.auth.header.eq_ignore_ascii_case("authorization")
                        || !config.auth.bearer
                        || config.auth.account_label.is_some()
                        || config.auth.token_url.is_none()
                        || config.auth.authorize_url.is_none()
                        || !valid_scopes(&config.auth.requested_scopes)
                }
            }
            || (!oauth
                && (config.auth.token_url.is_some()
                    || config.auth.token_ca_file.is_some()
                    || config.auth.authorize_url.is_some()
                    || !config.auth.requested_scopes.is_empty()))
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
        let document_base = match &config.request_prefix {
            None => base_path,
            Some(prefix) => {
                document_base(&base_path, prefix).ok_or(Failure::InvalidConfiguration)?
            }
        };
        let read_ca =
            |path: &PathBuf| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024);
        let ca = config
            .ca_file
            .as_ref()
            .map(read_ca)
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let token_ca = config
            .auth
            .token_ca_file
            .as_ref()
            .map(read_ca)
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let acquisition = if oauth {
            let (Some(token_url), Some(authorize_url)) =
                (&config.auth.token_url, &config.auth.authorize_url)
            else {
                return Err(Failure::InvalidConfiguration);
            };
            https_url(authorize_url)?;
            Some(Acquisition {
                authorize_url: authorize_url.clone(),
                token_url: token_url.clone(),
                scopes: config.auth.requested_scopes.clone(),
            })
        } else {
            None
        };
        let token_client = match &config.auth.token_url {
            Some(token_url) => {
                // The client's base is the token host alone; the endpoint's path
                // is sent as segments, so `canonical_base` adds no `/` to it.
                let canonical = https_url(token_url)?;
                let path = url_path(&canonical).ok_or(Failure::InvalidConfiguration)?;
                let authority = &canonical["https://".len()..canonical.len() - path.len()];
                let path: Vec<String> = segments(&path).into_iter().map(str::to_owned).collect();
                if path.is_empty() || path.iter().any(|segment| segment.contains('%')) {
                    return Err(Failure::InvalidConfiguration);
                }
                let http = ScopedHttp::new_with_ca_bytes(
                    &HttpConfig {
                        base_url: format!("https://{authority}/"),
                        credential: None,
                        credential_header: "authorization".into(),
                        bearer: false,
                        allow_plaintext: false,
                        ca_file: None,
                    },
                    None,
                    token_ca.as_deref(),
                )
                .map_err(Failure::from_service)?;
                Some(Arc::new(OAuth {
                    http,
                    path,
                    cache: Mutex::new(BTreeMap::new()),
                }))
            }
            None => None,
        };
        let engine =
            Engine::new(&bundle, &document_base, &operations).map_err(Failure::from_service)?;
        // Trust roots enter the revision by their bytes only, as `ca_file` does, so
        // the same roots at another path keep it.
        let mut auth = serde_json::to_value(&config.auth).map_err(|_| Failure::Protocol)?;
        if let Some(auth) = auth.as_object_mut() {
            auth.remove("token_ca_file");
        }
        let mut effective = json!({
            "format": config.format,
            "instance": config.instance,
            "provider": config.provider,
            "bundle_sha256": bundle::read_index(&config.bundle_directory)
                .ok()
                .and_then(|index| index.find(&config.provider).map(|e| e.bundle_sha256.clone())),
            "source_sha256": bundle.source.source_sha256,
            "api_base": base,
            "ca_digest": ca.as_ref().map(|b| connectors_core::digest(&json!(b))),
            "auth": auth,
            "operations": serde_json::to_value(&operations).map_err(|_| Failure::Protocol)?,
        });
        // Only a profile with its own token trust roots has this key, so every
        // other configuration keeps its revision.
        if let Some(bytes) = &token_ca {
            effective["token_ca_digest"] = json!(connectors_core::digest(&json!(bytes)));
        }
        // Present only when stated, so a configuration without it keeps its revision.
        if let Some(prefix) = &config.request_prefix {
            effective["request_prefix"] = json!(prefix);
        }
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
        let field = |name: &str, label: &str, max_bytes: u32| EntryField {
            name: name.into(),
            label: label.into(),
            max_bytes,
        };
        let (scheme, capability, fields) = match (config.auth.scheme, &config.auth.account_label) {
            (Scheme::Basic, Some(label)) => (
                "http_basic",
                "http-basic",
                vec![
                    field("account", label, ACCOUNT_LIMIT as u32),
                    field("token", &config.auth.label, 8192),
                ],
            ),
            (Scheme::OAuth2Refresh, _) => (
                "http_bearer",
                "http-bearer",
                vec![
                    field("client_id", "OAuth client ID", 8192),
                    field("client_secret", "OAuth client secret", 8192),
                    field("refresh_token", &config.auth.label, 8192),
                ],
            ),
            _ => (
                "http_bearer",
                "http-bearer",
                vec![field("token", &config.auth.label, 8192)],
            ),
        };
        let mut profile = Profile {
            id: config.auth.profile.clone(),
            revision: String::new(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: scheme.into(),
            capability: capability.into(),
            minimum_scopes: config.auth.minimum_scopes.clone(),
            evidence_lifetime_ms: config.auth.evidence_lifetime_ms,
            fields,
            acquisition,
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
            oauth: token_client,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    /// The API port carrying `credential` as this profile's header.
    fn with(&self, credential: Secret) -> ScopedHttp {
        self.http.with_credential(Arc::new(Fixed(credential)))
    }
    /// The bounded protected document, taken out of the caller's buffer.
    fn document(mut document: Secret) -> Result<Zeroizing<Vec<u8>>> {
        let document = Zeroizing::new(std::mem::take(&mut document.0));
        if document.len() > DOCUMENT_LIMIT {
            return Err(Failure::InvalidInput);
        }
        Ok(document)
    }
    /// An `oauth2_refresh` entry and its cache key, the digest of its bytes.
    fn oauth_entry(&self, document: Secret) -> Result<(&OAuth, OAuthEntry, [u8; 32])> {
        let oauth = self.oauth.as_deref().ok_or(Failure::InvalidConfiguration)?;
        let document = Self::document(document)?;
        let entry: OAuthEntry =
            serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
        Ok((oauth, entry, Sha256::digest(document.as_slice()).into()))
    }
    /// The API port for one request and, for an OAuth profile, the cache key of
    /// the access token it carries, so a refusal of that token can evict it.
    async fn authenticated(&self, document: Secret) -> Result<(ScopedHttp, Option<[u8; 32]>)> {
        let credential = match self.auth.scheme {
            Scheme::Token => {
                let document = Self::document(document)?;
                let entry: ProtectedEntry =
                    serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
                Secret(entry.token.as_bytes().to_vec())
            }
            Scheme::Basic => {
                let document = Self::document(document)?;
                let entry: BasicEntry =
                    serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
                basic_header(&entry)?
            }
            Scheme::OAuth2Refresh => {
                let (oauth, entry, key) = self.oauth_entry(document)?;
                let access = match oauth.cached(&key) {
                    Some(access) => access,
                    None => {
                        let exchanged = oauth.exchange(&entry).await?;
                        oauth.store(key, &exchanged);
                        exchanged.access
                    }
                };
                return Ok((self.with(Secret(access.as_bytes().to_vec())), Some(key)));
            }
        };
        Ok((self.with(credential), None))
    }
    fn baseline(
        &self,
        subject: String,
        granted_scopes: Option<BTreeSet<String>>,
        collected_at_ms: u64,
    ) -> Result<Baseline> {
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
    /// The declared identity read, and the scope read when there is one.
    async fn probe_identity(&self, http: &ScopedHttp, collected_at_ms: u64) -> Result<Baseline> {
        let identity_path = self
            .auth
            .identity
            .path
            .as_deref()
            .ok_or(Failure::InvalidConfiguration)?;
        let subject_pointer = self
            .auth
            .identity
            .subject_pointer
            .as_deref()
            .ok_or(Failure::InvalidConfiguration)?;
        let response = http
            .get(&segments(identity_path), &[])
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
        let subject = match identity.pointer(subject_pointer) {
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
        self.baseline(subject, granted_scopes, collected_at_ms)
    }
    /// The access token a refused request carried is not used again.
    fn evict_refused<T>(&self, key: Option<[u8; 32]>, result: &Result<T>) {
        if let (Some(key), Some(oauth), Err(Failure::InvalidCredential)) =
            (key, &self.oauth, result)
        {
            oauth.evict(&key);
        }
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

/// The document base left once a gateway `prefix` is removed from the front of
/// a canonical base path. The prefix is plain path text — a leading `/`, no
/// trailing `/`, segments of unreserved characters that are not `.` or `..` —
/// and must be whole leading segments of `base_path`; anything else is `None`.
fn document_base(base_path: &str, prefix: &str) -> Option<String> {
    let segments = prefix.strip_prefix('/')?.split('/').collect::<Vec<_>>();
    let plain = |segment: &&str| {
        !segment.is_empty()
            && *segment != "."
            && *segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
    };
    if prefix.len() > 1024 || !segments.iter().all(plain) {
        return None;
    }
    let mut rest = base_path.strip_prefix('/')?;
    for segment in segments {
        rest = rest.strip_prefix(segment)?;
        rest = match rest.strip_prefix('/') {
            Some(rest) => rest,
            None if rest.is_empty() => rest,
            None => return None,
        };
    }
    Some(format!("/{rest}"))
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
        let (http, key) = self.authenticated(document).await?;
        let prepared = self
            .engine
            .prepare(&http, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider);
        self.evict_refused(key, &prepared);
        Ok(Box::new(Write {
            prepared: prepared?,
            http: http.into_write(),
            token: key.and_then(|key| Some((self.oauth.clone()?, key))),
        }))
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline> {
        if profile != self.auth.profile {
            return Err(Failure::Unsupported);
        }
        let collected_at_ms = connectors_sdk::now_ms();
        if self.auth.scheme != Scheme::OAuth2Refresh {
            let (http, _) = self.authenticated(document).await?;
            return self.probe_identity(&http, collected_at_ms).await;
        }
        // Validation always proves the refresh token itself, never a cache, and
        // caches the fresh access token only once the credential validated.
        let (oauth, entry, key) = self.oauth_entry(document)?;
        let result = async {
            let exchanged = oauth.exchange(&entry).await?;
            let baseline = match self.auth.identity.source {
                IdentitySource::IdToken => {
                    let (subject, scope) = oauth.identity(&entry, &exchanged).await?;
                    if subject.is_empty() || subject.len() > 256 {
                        return Err(Failure::Protocol);
                    }
                    let granted = scope_set(&scope)?;
                    if !self.auth.minimum_scopes.is_subset(&granted) {
                        return Err(Failure::InsufficientScope);
                    }
                    self.baseline(subject, Some(granted), collected_at_ms)?
                }
                IdentitySource::Api => {
                    let http = self.with(Secret(exchanged.access.as_bytes().to_vec()));
                    self.probe_identity(&http, collected_at_ms).await?
                }
            };
            oauth.store(key, &exchanged);
            Ok(baseline)
        }
        .await;
        self.evict_refused(Some(key), &result);
        result
    }
    async fn invoke(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value> {
        let (http, key) = self.authenticated(document).await?;
        let result = self
            .engine
            .read(&http, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider);
        self.evict_refused(key, &result);
        result
    }
}
struct Write {
    prepared: connectors_catalog_provider::Prepared,
    http: Box<dyn connectors_sdk::AuthenticatedWrite>,
    /// The cache and key of the OAuth access token the write carries.
    token: Option<(Arc<OAuth>, [u8; 32])>,
}
#[async_trait::async_trait]
impl runtime::PreparedWrite for Write {
    async fn execute(self: Box<Self>) -> connectors_sdk::WriteOutcome<Value> {
        let Write {
            prepared,
            http,
            token,
        } = *self;
        let outcome = prepared.execute(http).await;
        // A write the provider refused as unauthorized refused the token too.
        if let (Some((oauth, key)), connectors_sdk::WriteOutcome::Refused(error)) =
            (&token, &outcome)
            && error.code == connectors_core::ErrorCode::Unauthorized
        {
            oauth.evict(key);
        }
        outcome
    }
}
