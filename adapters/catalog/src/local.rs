//! Executable composition for one provider bundle: native configuration and
//! protected entry terminate here. The engine receives only an immutable
//! authenticated HTTP port; the host keeps admission, approval and the ledger.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection, feed};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{
        filesystem,
        oauth2_grant::ENTRY_LIMIT,
        registry,
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
/// A connector declaration, `providers/<name>/connector.json`.
pub const DECLARATION_FORMAT: &str = "connectors-catalog-connector/1";
const DOCUMENT_LIMIT: usize = 64 * 1024;
/// The longest provider interval a bounded read may take; its own, possibly
/// shorter cutoff bounds it further.
const PROVIDER_BUDGET_LIMIT: Duration = Duration::from_secs(30);

fn bounded_http(http: ScopedHttp, until: Option<Instant>) -> Result<ScopedHttp> {
    match until {
        // `until` was selected once for the whole native sequence. The maximum
        // duration here cannot extend that original, possibly shorter cutoff.
        Some(until) => http
            .with_provider_budget(PROVIDER_BUDGET_LIMIT, until)
            .map_err(Failure::from_service),
        None => Ok(http),
    }
}

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
    /// A read against the provider API at `path` whose `200` proves the
    /// credential; its body is not read, and the subject is the configured
    /// instance id. For a provider whose API names no account or user, so the
    /// subject is the configured connection (`connectors_catalog.identity`).
    Configuration,
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
/// A read validation makes after the identity read, so a credential that identifies
/// its holder but cannot read what the selection needs is refused as
/// `insufficient_scope` instead of connected. The path is relative to the provider
/// authority; the body is not read.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessProbe {
    path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    query: BTreeMap<String, String>,
}
impl AccessProbe {
    fn usable(&self) -> bool {
        !segments(&self.path).is_empty()
            && !self.path.contains('?')
            && self.query.len() <= 16
            && self
                .query
                .iter()
                .all(|(k, v)| !k.is_empty() && k.len() <= 128 && v.len() <= 1024)
    }
}

/// The alternative descriptions of one instance (`connectors_catalog.binding.Bindings`):
/// the configuration's own provider, selection set and `auth.access` are the
/// binding `id`, and each alternative is another bundle and selection set of
/// the same base, profile and credential. Validation binds to the first whose
/// access probe answers.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingsConfig {
    id: String,
    /// The answer that moves validation from the first binding to the first alternative.
    next: NextAnswer,
    alternatives: Vec<AlternativeConfig>,
}
/// `connectors_catalog.binding.Alternative`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlternativeConfig {
    id: String,
    /// The bundle loaded from the configuration's `bundle_directory`.
    provider: String,
    /// An absolute path to the alternative's reviewed selection set.
    operations_file: PathBuf,
    access: AccessProbe,
    /// Present exactly when another alternative follows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next: Option<NextAnswer>,
}
/// `connectors_catalog.binding.Next`: an access answer with exactly this
/// status whose admitted reason contains `reason`, compared case-insensitively.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NextAnswer {
    status: u16,
    reason: String,
}
impl NextAnswer {
    /// Only a refusal of the binding moves on: 401 or 403, the two answers the outcome counts
    /// when every binding tried is refused (`insufficient_scope`).
    fn usable(&self) -> bool {
        matches!(self.status, 401 | 403)
            && !self.reason.trim().is_empty()
            && self.reason.len() <= 256
            && !self.reason.chars().any(char::is_control)
    }
    /// Whether an access answer of `status` with the admitted `reason` is this one.
    fn matches(&self, status: u16, reason: Option<&str>) -> bool {
        status == self.status
            && reason.is_some_and(|reason| {
                reason
                    .to_lowercase()
                    .contains(&self.reason.trim().to_lowercase())
            })
    }
}
/// The most alternatives one configuration declares, as the host admits.
const ALTERNATIVE_BINDINGS: usize = 8;

/// One binding as validation tries it.
struct Step {
    id: String,
    access: AccessProbe,
    /// The answer that moves on to the next step; `None` on the last.
    next: Option<NextAnswer>,
    /// An alternative's engine; `None` for the first binding, whose engine is
    /// [`Local::engine`].
    engine: Option<Engine>,
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
    /// `{"client_id","client_secret"}` is exchanged at `token_url` with the
    /// client-credentials grant for an access token sent as
    /// `Authorization: Bearer`; no refresh token exists.
    #[serde(rename = "oauth2_client_credentials")]
    OAuth2ClientCredentials,
    /// The CLI runs the grant `oauth2` declares and stores
    /// `{"client_id","access_token","refresh_token","expires_at_ms","scopes","id_token"}`;
    /// the access token is sent as `Authorization: Bearer`, and validation
    /// never calls the token endpoint
    /// (`architecture-decision-record:oauth2-grants-and-access-token-issue`).
    #[serde(rename = "oauth2_grant")]
    OAuth2Grant,
    /// `{"app_id","installation_id","private_key"}`: an RS256 JWT signed with
    /// the GitHub App's key is exchanged at
    /// `{api_base}/app/installations/{installation_id}/access_tokens` for an
    /// installation token sent as `Authorization: Bearer`. The connection is
    /// the installation (story:catalog-github-app-installation-token).
    #[serde(rename = "github_app")]
    GitHubApp,
}
impl Scheme {
    fn is_token(&self) -> bool {
        *self == Self::Token
    }
    /// A profile whose stored entry is exchanged at `token_url`.
    fn is_oauth(&self) -> bool {
        matches!(self, Self::OAuth2Refresh | Self::OAuth2ClientCredentials)
    }
    /// A profile whose stored entry is exchanged for the token a request
    /// carries, which is kept in memory only ([`OAuth`]).
    fn exchanges(&self) -> bool {
        self.is_oauth() || *self == Self::GitHubApp
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
    /// The scheme word a token profile's token travels behind, `<prefix> <token>`,
    /// such as Opsgenie's `GenieKey`; never beside `bearer`. Omitted when absent,
    /// so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    /// The prompt label of the basic `account` field; only a basic profile has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    account_label: Option<String>,
    label: String,
    identity: IdentityProbe,
    #[serde(default)]
    scopes: Option<ScopesProbe>,
    /// Omitted when absent, so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    access: Option<AccessProbe>,
    #[serde(default)]
    minimum_scopes: BTreeSet<String>,
    #[serde(default = "default_evidence_lifetime")]
    evidence_lifetime_ms: u64,
    // The four OAuth fields below belong to the OAuth schemes only and are
    // omitted otherwise, so existing configuration revisions hold.
    // `oauth2_client_credentials` takes `token_url`, `token_ca_file` and
    // `requested_scopes`, sent as the grant's `scope`, and no `authorize_url`.
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
    /// The authorization server of an `oauth2_grant` profile
    /// (`connectors.oauth2.ServerDeclaration`), and of no other scheme. Omitted
    /// otherwise, so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oauth2: Option<connectors_oauth2::ServerDeclaration>,
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
    /// front of the document's paths, such as `/ex/<product>/<cloud id>`. Requests
    /// still go to the full `api_base`; only the document's base-path check
    /// starts after it.
    #[serde(default)]
    request_prefix: Option<String>,
    ca_file: Option<PathBuf>,
    /// The origins this connection admits besides `api_base`
    /// (`connectors_catalog.binary.Hosts`), which only a binary read reaches,
    /// and only those its selection names. Omitted when empty, so a
    /// configuration without it keeps its revision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hosts: Vec<AdmittedHost>,
    auth: AuthConfig,
    /// Selections written into this file.
    #[serde(default)]
    operations: Vec<Selection>,
    /// An absolute path to a reviewed selection set for `provider`, appended to
    /// `operations`. The repository ships one per provider it has reviewed.
    #[serde(default)]
    operations_file: Option<PathBuf>,
    /// The instance's alternative bindings, of which validation selects the
    /// one the credential can read. Omitted when absent, so a configuration
    /// without it keeps its revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bindings: Option<BindingsConfig>,
    /// The client id registered for this instance, for an `oauth2_grant`
    /// profile whose declaration names none. Given beside one, it must be the
    /// same. Omitted when absent, so existing configuration revisions hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oauth2_client_id: Option<String>,
}

/// One origin a connection admits besides its API base
/// (`connectors_catalog.binary.AdmittedHost`). With `credential` the
/// connection's credential header travels to it, as Slack's file host
/// requires; otherwise requests to it carry none.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmittedHost {
    origin: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    credential: bool,
}
/// The most origins a connection admits besides its API base.
const ADMITTED_HOSTS: usize = 8;

/// The document `operations_file` names.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationsFile {
    format: String,
    provider: String,
    operations: Vec<Selection>,
    /// The provider's `datasource.feed/v1alpha1` binding, declared as data.
    #[serde(default)]
    feed: Option<feed::Declaration>,
}
fn read_selection(bytes: &[u8], provider: &str) -> Result<OperationsFile> {
    let selection: OperationsFile =
        connectors_core::read_json(bytes).map_err(|_| Failure::InvalidConfiguration)?;
    if selection.format != OPERATIONS_FORMAT || selection.provider != provider {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(selection)
}
/// The selection set at `path`. A shipped selection set is ordinary repository
/// content, readable by anyone; only the configuration that names it must be
/// private.
fn read_selection_file(path: &Path, provider: &str) -> Result<OperationsFile> {
    let bytes = std::fs::File::open(path)
        .map_err(|_| Failure::InvalidConfiguration)
        .and_then(|file| {
            filesystem::read_bounded(file, 1024 * 1024).map_err(|_| Failure::InvalidConfiguration)
        })?;
    read_selection(&bytes, provider)
}

/// Whether `bindings` can be followed (`connectors_catalog.binding.Bindings`):
/// the first binding has its access probe, every id is valid and distinct, a
/// usable `next` answer stands before every alternative and after none, and
/// the identity is read from the API, after which the access probes run.
fn usable_bindings(bindings: &BindingsConfig, config: &Configuration) -> bool {
    let mut ids = BTreeSet::from([bindings.id.as_str()]);
    let last = bindings.alternatives.len().saturating_sub(1);
    config.auth.access.is_some()
        && config.auth.identity.source != IdentitySource::IdToken
        && connectors_core::valid_id(&bindings.id)
        && bindings.next.usable()
        && (1..=ALTERNATIVE_BINDINGS).contains(&bindings.alternatives.len())
        && bindings
            .alternatives
            .iter()
            .enumerate()
            .all(|(index, alternative)| {
                connectors_core::valid_id(&alternative.id)
                    && ids.insert(alternative.id.as_str())
                    && connectors_core::valid_id(&alternative.provider)
                    && alternative.access.usable()
                    && match &alternative.next {
                        None => index == last,
                        Some(next) => index != last && next.usable(),
                    }
            })
}

/// One alternative binding as the configuration loaded it.
struct LoadedAlternative {
    config: AlternativeConfig,
    engine: Engine,
    /// Its entry in the configuration revision: content, never paths.
    revision: Value,
}

/// What one catalog connector fixes for every instance of it
/// (`connectors.declarations.Connector`): the bundle it runs on, its default
/// endpoint, the selection its operations come from and the auth profiles an
/// instance may use. `build.rs` compiles every declaration into this binary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    format: String,
    connector_id: String,
    provider_id: String,
    adapter_id: String,
    /// The bundle and selection provider (`google-gmail`), digested as `provider`.
    catalog_provider: String,
    /// The endpoint of an instance that states none.
    #[serde(default)]
    default_endpoint: Option<String>,
    /// Appended to the endpoint to form `api_base`, for a connector whose users
    /// know the host (`https://code.example.com`), not the API under it. An
    /// absolute path without a trailing `/`; absent, the endpoint is `api_base`.
    #[serde(default)]
    api_path: Option<String>,
    /// The selection file beside the declaration; every operation in it is
    /// available on every instance.
    selection: String,
    /// The profiles an instance may use. A `token_url` that starts with `/` is
    /// on the instance endpoint's origin.
    profiles: Vec<AuthConfig>,
}

/// The declarations, their selections and the bundles they run on, as
/// `build.rs` found them in the source tree.
mod shipped {
    include!(concat!(env!("OUT_DIR"), "/shipped.rs"));
}

/// What an instance of a shipped connector states; its declaration fixes the rest.
pub struct Instance<'a> {
    pub connector: &'a str,
    /// The user's name for the instance; its id is `<connector>.<name>`
    /// unless `instance_id` names another.
    pub name: &'a str,
    /// The id a migrated instance keeps (`--instance-id`), in place of
    /// `<connector>.<name>` (story:migrate-writes-the-short-instance-form).
    pub instance_id: Option<&'a str>,
    pub endpoint: Option<&'a str>,
    /// Required when the connector declares more than one profile.
    pub profile: Option<&'a str>,
}

/// The bundle and selection a configuration runs on.
struct Sources {
    bundle: bundle::Bundle,
    bundle_sha256: Option<String>,
    selection: Option<OperationsFile>,
}

/// A compiled-in bundle, checked against the compiled-in index as
/// `bundle::load` checks a directory: the index names it, its bytes have the
/// digest the index recorded, and it names the same provider.
fn shipped_bundle(provider: &str) -> Result<(bundle::Bundle, String)> {
    use std::fmt::Write as _;
    let index: bundle::Index = connectors_core::read_json(shipped::BUNDLE_INDEX)
        .map_err(|_| Failure::InvalidConfiguration)?;
    let entry = index.find(provider).ok_or(Failure::InvalidConfiguration)?;
    let bytes = shipped::BUNDLES
        .iter()
        .find(|(name, _)| *name == entry.file_name)
        .map(|(_, bytes)| *bytes)
        .ok_or(Failure::InvalidConfiguration)?;
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(digest, "{byte:02x}").map_err(|_| Failure::Protocol)?;
    }
    let bundle: bundle::Bundle =
        connectors_core::read_json(bytes).map_err(|_| Failure::InvalidConfiguration)?;
    if digest != entry.bundle_sha256 || bundle.provider != entry.provider {
        return Err(Failure::InvalidConfiguration);
    }
    Ok((bundle, digest))
}

/// The native configuration of an instance of a shipped connector: exactly what
/// an equal hand-written configuration states, without its file paths, which are
/// not digested. So it has that configuration's revision.
/// The declaration of the shipped connector `connector` and the files beside
/// it; every declaration before it is checked on the way.
fn declaration(connector: &str) -> Result<(Declaration, &'static [shipped::File])> {
    for &(directory, text, files) in shipped::CONNECTORS {
        let declaration: Declaration = connectors_core::read_json(text.as_bytes())
            .map_err(|_| Failure::InvalidConfiguration)?;
        if declaration.format != DECLARATION_FORMAT
            || declaration.adapter_id != "catalog"
            || declaration.catalog_provider != directory
            || !connectors_core::valid_id(&declaration.connector_id)
            || !connectors_core::valid_id(&declaration.provider_id)
        {
            return Err(Failure::InvalidConfiguration);
        }
        if declaration.connector_id == connector {
            return Ok((declaration, files));
        }
    }
    Err(Failure::NotFound)
}

/// The auth profiles the shipped connector `connector` declares, in
/// declaration order (`--print-profiles`): what an instance of a connector
/// declaring several names with `--profile`.
pub fn profiles(connector: &str) -> Result<Vec<String>> {
    let (declaration, _) = declaration(connector)?;
    Ok(declaration
        .profiles
        .into_iter()
        .map(|profile| profile.profile)
        .collect())
}

fn derived(instance: &Instance<'_>) -> Result<(Configuration, Sources)> {
    let (declaration, files) = declaration(instance.connector)?;
    let selection_name = declaration.selection.clone();
    let config = instance_configuration(declaration, instance)?;
    let selection = files
        .iter()
        .find(|(name, _)| *name == selection_name)
        .ok_or(Failure::InvalidConfiguration)?;
    let selection = read_selection(selection.1, &config.provider)?;
    let (bundle, bundle_sha256) = shipped_bundle(&config.provider)?;
    let sources = Sources {
        bundle,
        bundle_sha256: Some(bundle_sha256),
        selection: Some(selection),
    };
    Ok((config, sources))
}

/// The native configuration `declaration` gives `instance`: its id (the one it
/// states, else `<connector>.<name>`), the endpoint and API path, and the one
/// profile it uses. A shipped declaration and a lowered definition's are read
/// alike.
fn instance_configuration(
    declaration: Declaration,
    instance: &Instance<'_>,
) -> Result<Configuration> {
    let instance_id = match instance.instance_id {
        Some(id) => id.to_owned(),
        None => format!("{}.{}", declaration.connector_id, instance.name),
    };
    if !connectors_core::valid_id(instance.name) || !connectors_core::valid_id(&instance_id) {
        return Err(Failure::InvalidInput);
    }
    let endpoint = instance
        .endpoint
        .or(declaration.default_endpoint.as_deref())
        .ok_or(Failure::InvalidConfiguration)?
        .to_owned();
    // An endpoint that is already the API base is refused, not doubled. A
    // relative-root install (`https://example.com/code`) serves the API under
    // its root, which is then a request prefix in front of the bundle's paths.
    let (api_base, request_prefix) = match declaration.api_path.as_deref() {
        None => (endpoint.clone(), None),
        Some(path) => {
            let host = endpoint.trim_end_matches('/');
            if !path.starts_with('/') || path.ends_with('/') || host.ends_with(path) {
                return Err(Failure::InvalidConfiguration);
            }
            let root = url_path(host).filter(|root| root != "/");
            (format!("{host}{path}"), root)
        }
    };
    let mut profiles = declaration.profiles;
    let mut auth = match instance.profile {
        Some(id) => profiles
            .iter()
            .position(|profile| profile.profile == id)
            .map(|index| profiles.swap_remove(index)),
        None if profiles.len() == 1 => profiles.pop(),
        None => None,
    }
    .ok_or(Failure::InvalidInput)?;
    if let Some(path) = auth.token_url.as_deref().filter(|url| url.starts_with('/')) {
        let origin =
            connectors_catalog_provider::origin(&endpoint).ok_or(Failure::InvalidConfiguration)?;
        auth.token_url = Some(format!("{origin}{path}"));
    }
    Ok(Configuration {
        format: FORMAT.into(),
        instance: instance_id,
        provider: declaration.catalog_provider,
        bundle_directory: PathBuf::new(),
        api_base,
        request_prefix,
        ca_file: None,
        hosts: Vec::new(),
        auth,
        operations: Vec::new(),
        operations_file: None,
        bindings: None,
        oauth2_client_id: None,
    })
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
    bounded_token(deserializer, 8192)
}
fn optional_token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    token(deserializer).map(Some)
}
/// A field of an `oauth2_grant` entry ([`GrantEntry`]).
fn grant_token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    bounded_token(deserializer, ENTRY_LIMIT)
}
fn optional_grant_token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    grant_token(deserializer).map(Some)
}
/// 1 to `limit` bytes of visible ASCII: no space, no control character.
fn bounded_token<'de, D: Deserializer<'de>>(
    deserializer: D,
    limit: usize,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty() || value.len() > limit || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
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
/// The `oauth2_client_credentials` profile's protected entry: the confidential
/// client alone, under the token rules.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientEntry {
    #[serde(deserialize_with = "token")]
    client_id: Zeroizing<String>,
    #[serde(deserialize_with = "token")]
    client_secret: Zeroizing<String>,
}
/// The `oauth2_grant` profile's protected entry, as the CLI's grant wrote it.
/// Its strings are tokens of visible ASCII, each bounded only by the entry the
/// CLI builds them into (`oauth2_grant::ENTRY_LIMIT`), not by the token rules:
/// a token endpoint's tokens, an OpenID Connect `id_token` above all, may be
/// longer than a token a person enters. The scopes are what the server granted.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantEntry {
    #[serde(deserialize_with = "grant_token")]
    client_id: Zeroizing<String>,
    #[serde(deserialize_with = "grant_token")]
    access_token: Zeroizing<String>,
    /// Absent when the server issued none. Validation never refreshes, so
    /// only its form is checked here.
    #[serde(
        default,
        rename = "refresh_token",
        deserialize_with = "optional_grant_token"
    )]
    _refresh_token: Option<Zeroizing<String>>,
    expires_at_ms: u64,
    scopes: BTreeSet<String>,
    /// The OpenID Connect `id_token` the token endpoint returned, when it did.
    #[serde(default, deserialize_with = "optional_grant_token")]
    id_token: Option<Zeroizing<String>>,
}
/// An `id_token`'s audience: one client id or several (OpenID Connect Core
/// §2).
#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}
/// The claims of an `oauth2_grant` entry's `id_token` this profile reads.
#[derive(Deserialize)]
struct GrantClaims {
    aud: Audience,
    #[serde(default)]
    azp: Option<String>,
    sub: String,
    exp: u64,
    #[serde(default)]
    iat: Option<u64>,
}
/// The subject of an `oauth2_grant` entry's `id_token`. The CLI received it
/// directly from the token endpoint over TLS, which OpenID Connect Core 1.0
/// §3.1.3.7 item 6 accepts in place of its signature; it must be issued to
/// `client_id` (items 3 to 5), not expired (item 9) and not issued ahead of
/// this clock. The subject is its `sub`, or the string claim at
/// `subject_pointer` when the profile names one, such as the ChatGPT account
/// id in the `https://api.openai.com/auth` claim.
fn grant_subject(id_token: &str, client_id: &str, subject_pointer: Option<&str>) -> Result<String> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut parts = id_token.split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Failure::InvalidCredential);
    };
    let payload = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .map_err(|_| Failure::InvalidCredential)?,
    );
    let document: Value =
        serde_json::from_slice(&payload).map_err(|_| Failure::InvalidCredential)?;
    // Claims are an object; an array would be read as the claims by position.
    if !document.is_object() {
        return Err(Failure::InvalidCredential);
    }
    let subject = match subject_pointer {
        None => None,
        Some(pointer) => match document.pointer(pointer) {
            Some(Value::String(text)) => Some(text.clone()),
            _ => return Err(Failure::InvalidCredential),
        },
    };
    let claims: GrantClaims =
        serde_json::from_value(document).map_err(|_| Failure::InvalidCredential)?;
    let subject = subject.unwrap_or(claims.sub);
    let now = connectors_sdk::now_ms() / 1000;
    let audience = match &claims.aud {
        Audience::One(one) => one == client_id,
        Audience::Many(many) => {
            many.iter().any(|aud| aud == client_id)
                && (many.len() == 1 || claims.azp.as_deref() == Some(client_id))
        }
    };
    if !audience
        || claims.azp.as_deref().is_some_and(|azp| azp != client_id)
        || claims.exp <= now
        || claims
            .iat
            .is_some_and(|iat| iat > now.saturating_add(ISSUED_AT_SKEW_S))
        || subject.is_empty()
        || subject.len() > 256
    {
        return Err(Failure::InvalidCredential);
    }
    Ok(subject)
}
/// The stored entry of an exchanging scheme, as the grant it makes.
enum Grant {
    Refresh(OAuthEntry),
    ClientCredentials(ClientEntry),
    App(AppEntry),
}

/// The longest GitHub App id a `github_app` entry holds: the numeric App id or
/// its client id, both well below it.
const APP_ID_LIMIT: usize = 128;
/// The longest installation id: the digits of a `u64`.
const INSTALLATION_ID_LIMIT: usize = 20;
/// The longest PEM private key: a 4096-bit RSA key is about 3.3 KiB.
const APP_KEY_LIMIT: usize = 16 * 1024;
/// The PEM labels of the RSA keys a `github_app` entry may hold: PKCS #1, as
/// GitHub issues it, and unencrypted PKCS #8.
const PKCS1_LABEL: &str = "RSA PRIVATE KEY";
const PKCS8_LABEL: &str = "PRIVATE KEY";
/// How far the App JWT's `iat` is set back against clock drift; GitHub's limit.
const APP_JWT_BACKDATE_S: u64 = 60;
/// How long the App JWT is valid from now: under GitHub's ten-minute limit on
/// `exp - iat` with the backdated `iat`.
const APP_JWT_LIFETIME_S: u64 = 9 * 60;
/// An installation token is used until this long before its `expires_at`.
const APP_TOKEN_SKEW_S: i128 = 5 * 60;
/// The media type GitHub's REST API answers in.
const GITHUB_JSON: &str = "application/vnd.github+json";

/// The `github_app` profile's protected entry as stored. The App id and
/// installation id are numbers or strings of them; the key is PEM text.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppDocument {
    #[serde(deserialize_with = "app_id")]
    app_id: String,
    #[serde(deserialize_with = "installation_id")]
    installation_id: u64,
    #[serde(deserialize_with = "app_key_text")]
    private_key: Zeroizing<String>,
}
/// A number or a string of a JSON document's id.
#[derive(Deserialize)]
#[serde(untagged)]
enum NumberOrText {
    Number(u64),
    Text(String),
}
/// 1 to [`APP_ID_LIMIT`] bytes of ASCII letters, digits, `.`, `_` and `-`.
fn app_id<'de, D: Deserializer<'de>>(deserializer: D) -> std::result::Result<String, D::Error> {
    let text = match NumberOrText::deserialize(deserializer)? {
        NumberOrText::Number(number) => number.to_string(),
        NumberOrText::Text(text) => text,
    };
    if text.is_empty()
        || text.len() > APP_ID_LIMIT
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(text)
}
/// A positive integer of at most [`INSTALLATION_ID_LIMIT`] digits.
fn installation_id<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u64, D::Error> {
    let id = match NumberOrText::deserialize(deserializer)? {
        NumberOrText::Number(number) => Some(number),
        NumberOrText::Text(text) => (!text.is_empty()
            && text.len() <= INSTALLATION_ID_LIMIT
            && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten(),
    };
    id.filter(|id| *id > 0)
        .ok_or_else(|| serde::de::Error::custom("invalid protected field"))
}
/// 1 to [`APP_KEY_LIMIT`] bytes of ASCII text.
fn app_key_text<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty()
        || value.len() > APP_KEY_LIMIT
        || !value
            .bytes()
            .all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
    {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
}

/// A `github_app` entry with its key parsed: every field is checked before a
/// request is made, so a bad entry is `invalid_input` and nothing is sent.
struct AppEntry {
    app_id: String,
    installation_id: u64,
    key: ring::signature::RsaKeyPair,
}
impl AppEntry {
    fn parse(document: &[u8]) -> Result<Self> {
        let document: AppDocument =
            serde_json::from_slice(document).map_err(|_| Failure::InvalidInput)?;
        Ok(Self {
            key: app_key(&document.private_key)?,
            app_id: document.app_id,
            installation_id: document.installation_id,
        })
    }
    /// An RS256 JWT for GitHub's App authentication: `iss` the App id, `iat`
    /// backdated by [`APP_JWT_BACKDATE_S`], `exp` [`APP_JWT_LIFETIME_S`] from
    /// now. It is a credential for those minutes, so it is zeroized on drop.
    fn jwt(&self) -> Result<Zeroizing<Vec<u8>>> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Failure::Protocol)?
            .as_secs();
        let claims = serde_json::to_vec(&json!({
            "iat": now.saturating_sub(APP_JWT_BACKDATE_S),
            "exp": now.saturating_add(APP_JWT_LIFETIME_S),
            "iss": self.app_id,
        }))
        .map_err(|_| Failure::Protocol)?;
        let mut jwt = Zeroizing::new(Vec::with_capacity(1024));
        jwt.extend_from_slice(
            URL_SAFE_NO_PAD
                .encode(br#"{"alg":"RS256","typ":"JWT"}"#)
                .as_bytes(),
        );
        jwt.push(b'.');
        jwt.extend_from_slice(URL_SAFE_NO_PAD.encode(&claims).as_bytes());
        let mut signature = vec![0; self.key.public().modulus_len()];
        self.key
            .sign(
                &ring::signature::RSA_PKCS1_SHA256,
                &ring::rand::SystemRandom::new(),
                &jwt,
                &mut signature,
            )
            .map_err(|_| Failure::Protocol)?;
        jwt.push(b'.');
        jwt.extend_from_slice(URL_SAFE_NO_PAD.encode(&signature).as_bytes());
        Ok(jwt)
    }
}
/// The RSA key of a PEM document holding one PKCS #1 or unencrypted PKCS #8
/// key, as ring admits it (2048 to 4096 bits). Anything else, an encrypted
/// key or one with RFC 1421 headers among them, is `invalid_input`. Every
/// buffer holding key bytes is zeroized on drop.
fn app_key(pem: &str) -> Result<ring::signature::RsaKeyPair> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let rest = pem
        .trim()
        .strip_prefix("-----BEGIN ")
        .ok_or(Failure::InvalidInput)?;
    let (label, rest) = rest.split_once("-----").ok_or(Failure::InvalidInput)?;
    if label != PKCS1_LABEL && label != PKCS8_LABEL {
        return Err(Failure::InvalidInput);
    }
    let body = rest
        .strip_suffix("-----")
        .and_then(|rest| rest.strip_suffix(label))
        .and_then(|rest| rest.strip_suffix("-----END "))
        .ok_or(Failure::InvalidInput)?;
    let mut encoded = Zeroizing::new(Vec::with_capacity(body.len()));
    for byte in body.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => encoded.push(byte),
            _ if byte.is_ascii_whitespace() => {}
            _ => return Err(Failure::InvalidInput),
        }
    }
    let der = Zeroizing::new(
        STANDARD
            .decode(encoded.as_slice())
            .map_err(|_| Failure::InvalidInput)?,
    );
    if label == PKCS1_LABEL {
        ring::signature::RsaKeyPair::from_der(&der)
    } else {
        ring::signature::RsaKeyPair::from_pkcs8(&der)
    }
    .map_err(|_| Failure::InvalidInput)
}
/// The answer of GitHub's installation-token exchange. Fields it adds, such
/// as the token's permissions, are ignored.
#[derive(Deserialize)]
struct InstallationToken {
    #[serde(deserialize_with = "token")]
    token: Zeroizing<String>,
    expires_at: String,
}
/// One `github_app` exchange through `http`, the instance's own API port: the
/// JWT for an installation token, used until [`APP_TOKEN_SKEW_S`] before its
/// `expires_at`. Every refusal is a code; no answer text reaches it.
async fn app_exchange(http: &ScopedHttp, entry: &AppEntry) -> Result<Exchanged> {
    let jwt = entry.jwt()?;
    let installation = entry.installation_id.to_string();
    let sent = Instant::now();
    let response = http
        .post_assertion(
            &["app", "installations", &installation, "access_tokens"],
            &jwt,
            GITHUB_JSON,
        )
        .await
        .map_err(Failure::from_provider)?;
    let body = Zeroizing::new(response.body);
    match response.status {
        200 | 201 => {}
        // A secondary rate limit is a 403 that names its wait.
        403 if response.headers.contains_key("retry-after")
            || response
                .headers
                .get("x-ratelimit-remaining")
                .is_some_and(|remaining| remaining.trim() == "0") =>
        {
            return Err(Failure::ProviderRateLimited);
        }
        // The JWT or the App's key refused (401), the installation suspended
        // (403), or not this App's installation (404).
        401 | 403 | 404 => return Err(Failure::InvalidCredential),
        429 => return Err(Failure::ProviderRateLimited),
        500..=599 => return Err(Failure::Unavailable),
        _ => return Err(Failure::Protocol),
    }
    if body.len() > DOCUMENT_LIMIT {
        return Err(Failure::Protocol);
    }
    let answer: InstallationToken = answer_object(&body)?;
    let expires_at = feed::instant(&answer.expires_at).ok_or(Failure::Protocol)? / 1_000_000_000;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Failure::Protocol)?
        .as_secs();
    // A token its issuer says has expired is no token, as an OAuth
    // `expires_in` of 0 is none: it is never sent.
    if expires_at <= i128::from(now) {
        return Err(Failure::Protocol);
    }
    let usable =
        (expires_at - i128::from(now) - APP_TOKEN_SKEW_S).clamp(0, i128::from(CACHE_LIFETIME_S));
    Ok(Exchanged {
        access: answer.token,
        scope: None,
        id_token: None,
        // A token within the margin is used for this request and never kept.
        usable_until: u64::try_from(usable)
            .ok()
            .filter(|usable| *usable > 0)
            .and_then(|usable| sent.checked_add(Duration::from_secs(usable))),
    })
}

/// A token endpoint's successful answer. Fields it adds are ignored.
#[derive(Deserialize)]
struct TokenAnswer {
    #[serde(deserialize_with = "token")]
    access_token: Zeroizing<String>,
    token_type: String,
    /// RFC 6749 5.1 only recommends it; `oauth2_refresh` requires it.
    #[serde(default)]
    expires_in: Option<u64>,
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
/// The token endpoint of an OAuth profile and the access tokens it issued,
/// keyed by a digest of the protected entry that obtained each.
struct OAuth {
    http: ScopedHttp,
    path: Vec<String>,
    /// The client-credentials grant's `scope`: the requested scopes joined by
    /// one space. `None` for `oauth2_refresh`, whose grant sends none.
    scope: Option<String>,
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
    /// One refresh-token or client-credentials grant. Every refusal is a code;
    /// no answer text, status line or credential reaches it.
    async fn exchange(&self, grant: &Grant) -> Result<Exchanged> {
        self.exchange_until(grant, None).await
    }
    /// [`Self::exchange`] inside a bounded read's provider cutoff, when there is one.
    async fn exchange_until(&self, grant: &Grant, until: Option<Instant>) -> Result<Exchanged> {
        let bounded = until
            .map(|until| self.http.with_provider_budget(PROVIDER_BUDGET_LIMIT, until))
            .transpose()
            .map_err(Failure::from_service)?;
        let http = bounded.as_ref().unwrap_or(&self.http);
        if let Grant::App(entry) = grant {
            return app_exchange(http, entry).await;
        }
        let sent = Instant::now();
        let segments: Vec<&str> = self.path.iter().map(String::as_str).collect();
        let form: Vec<(&str, &[u8])> = match grant {
            Grant::Refresh(entry) => vec![
                ("grant_type", b"refresh_token".as_slice()),
                ("client_id", entry.client_id.as_bytes()),
                ("client_secret", entry.client_secret.as_bytes()),
                ("refresh_token", entry.refresh_token.as_bytes()),
            ],
            Grant::ClientCredentials(entry) => vec![
                ("grant_type", b"client_credentials".as_slice()),
                ("client_id", entry.client_id.as_bytes()),
                ("client_secret", entry.client_secret.as_bytes()),
                (
                    "scope",
                    self.scope
                        .as_deref()
                        .ok_or(Failure::InvalidConfiguration)?
                        .as_bytes(),
                ),
            ],
            // Exchanged by `app_exchange` above.
            Grant::App(_) => return Err(Failure::InvalidConfiguration),
        };
        let response = http
            .post_form(&segments, &form)
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
                    // A client not allowed this grant, such as a public one.
                    Some("unauthorized_client") if matches!(grant, Grant::ClientCredentials(_)) => {
                        Failure::InvalidCredential
                    }
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
        let answer: TokenAnswer = answer_object(&body)?;
        // A client-credentials answer without `expires_in` (Zendesk clients created
        // before 2026-04-30) is cached for at most `CACHE_LIFETIME_S`; a refused
        // request evicts it sooner.
        let expires_in = match (answer.expires_in, grant) {
            (Some(0), _) | (None, Grant::Refresh(_) | Grant::App(_)) => {
                return Err(Failure::Protocol);
            }
            (Some(seconds), _) => seconds,
            (None, Grant::ClientCredentials(_)) => CACHE_LIFETIME_S,
        };
        if !answer.token_type.eq_ignore_ascii_case("bearer") {
            return Err(Failure::Protocol);
        }
        match grant {
            // The profile is declared non-rotating, and there is no path to publish
            // rotated material: a different refresh token is refused, not stored.
            Grant::Refresh(entry) => {
                if answer
                    .refresh_token
                    .as_ref()
                    .is_some_and(|rotated| rotated.as_bytes() != entry.refresh_token.as_bytes())
                {
                    return Err(Failure::InvalidCredential);
                }
            }
            // RFC 6749 4.4.3: this grant issues no refresh token, and the
            // profile has nowhere to keep one.
            Grant::ClientCredentials(_) => {
                if answer.refresh_token.is_some() {
                    return Err(Failure::Protocol);
                }
            }
            Grant::App(_) => return Err(Failure::InvalidConfiguration),
        }
        let usable = expires_in
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
        let info: TokenInfo = answer_object(&response.body)?;
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
    answer_object(&payload)
}
/// A provider's JSON answer read as `T` only when it is an object. A derived
/// struct is also read from an array, by position, and none of the answers
/// read here is one: such an array's first string would become a token.
fn answer_object<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T> {
    if body.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
        return Err(Failure::Protocol);
    }
    serde_json::from_slice(body).map_err(|_| Failure::Protocol)
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
/// The longest scheme word a token profile may send in front of its token.
const PREFIX_LIMIT: usize = 32;
/// A token profile's scheme word: 1 to 32 visible ASCII bytes, no space.
fn valid_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && prefix.len() <= PREFIX_LIMIT
        && prefix.bytes().all(|b| b.is_ascii_graphic())
}
/// `<prefix> <token>`, sized up front so no credential byte is left behind in
/// a reallocated buffer.
fn prefixed(prefix: &str, token: &str) -> Secret {
    let mut value = Vec::with_capacity(prefix.len() + 1 + token.len());
    value.extend_from_slice(prefix.as_bytes());
    value.push(b' ');
    value.extend_from_slice(token.as_bytes());
    Secret(value)
}
/// Scopes a configuration requests: each a bounded printable word.
fn valid_scopes(scopes: &BTreeSet<String>) -> bool {
    !scopes.is_empty()
        && scopes.len() <= 64
        && scopes
            .iter()
            .all(|s| !s.is_empty() && s.len() <= 256 && s.bytes().all(|b| b.is_ascii_graphic()))
}
/// An `oauth2_grant` profile's client id: the declaration's, else the one the
/// instance's configuration registers. Given both, they must be the same;
/// given neither, there is no client to run the grant as. Each is 1 to 256
/// visible ASCII bytes.
fn grant_client_id(declared: Option<&str>, configured: Option<&str>) -> Result<String> {
    let usable =
        |id: &str| (1..=256).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_graphic());
    match (declared, configured) {
        (Some(id), None) | (None, Some(id)) if usable(id) => Ok(id.to_owned()),
        (Some(declared), Some(configured)) if declared == configured && usable(declared) => {
            Ok(declared.to_owned())
        }
        _ => Err(Failure::InvalidConfiguration),
    }
}
/// A declaration the CLI can run: valid (HTTPS endpoints, the endpoint each
/// declared grant needs), declaring at least one grant, each grant once, and
/// requesting and requiring bounded scope words.
fn usable_declaration(declaration: &connectors_oauth2::ServerDeclaration) -> bool {
    let words = |scopes: &[String]| {
        scopes.len() <= 64
            && scopes.iter().all(|scope| {
                (1..=256).contains(&scope.len()) && scope.bytes().all(|b| b.is_ascii_graphic())
            })
    };
    let grants: BTreeSet<_> = declaration.grant_types.iter().map(|g| g.as_str()).collect();
    declaration.validate().is_ok()
        && !grants.is_empty()
        && grants.len() == declaration.grant_types.len()
        && words(&declaration.requested_scopes)
        && words(&declaration.required_scopes)
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
    /// Per key of the configuration revision's digest input, the digest of
    /// that key's value (`--print-revision-inputs`).
    revision_inputs: BTreeMap<String, String>,
    http: Arc<ScopedHttp>,
    /// Each admitted origin's port, without a credential, and whether the
    /// connection's credential travels to it.
    hosts: BTreeMap<String, (Arc<ScopedHttp>, bool)>,
    engine: Engine,
    /// The bindings validation tries, in order; empty without bindings.
    steps: Vec<Step>,
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
        if !config.bundle_directory.is_absolute()
            || config
                .operations_file
                .as_ref()
                .is_some_and(|path| !path.is_absolute())
            || config.bindings.as_ref().is_some_and(|bindings| {
                bindings
                    .alternatives
                    .iter()
                    .any(|alternative| !alternative.operations_file.is_absolute())
            })
        {
            return Err(Failure::InvalidConfiguration);
        }
        Self::configure(config, None)
    }
    /// An instance of a connector this binary ships, configured from its
    /// declaration and the compiled-in bundle and selection; no file is read.
    pub fn derive(instance: &Instance<'_>) -> Result<Self> {
        let (config, sources) = derived(instance)?;
        Self::configure(config, Some(sources))
    }
    /// An instance of a lowered definition (`connectors_catalog::definition`),
    /// configured from the declaration the lowering wrote as an instance of a
    /// shipped connector is from its own; `out` is the lowering's bundle
    /// directory. Everything is read here: nothing under `out` is read again.
    /// The instance is `<connector>.<name>` under the declaration's connector.
    pub fn derive_lowered(
        lowered: &connectors_catalog::definition::Lowered,
        out: &Path,
        name: &str,
        endpoint: Option<&str>,
        profile: Option<&str>,
    ) -> Result<Self> {
        let file =
            std::fs::File::open(&lowered.connector).map_err(|_| Failure::InvalidConfiguration)?;
        let bytes = filesystem::read_bounded(file, 1024 * 1024)
            .map_err(|_| Failure::InvalidConfiguration)?;
        let declaration: Declaration =
            connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        if declaration.format != DECLARATION_FORMAT
            || declaration.adapter_id != "catalog"
            || declaration.catalog_provider != lowered.provider
            || !connectors_core::valid_id(&declaration.connector_id)
            || !connectors_core::valid_id(&declaration.provider_id)
            || lowered.operations.file_name() != Some(std::ffi::OsStr::new(&declaration.selection))
        {
            return Err(Failure::InvalidConfiguration);
        }
        let connector = declaration.connector_id.clone();
        let instance = Instance {
            connector: &connector,
            name,
            instance_id: None,
            endpoint,
            profile,
        };
        let config = instance_configuration(declaration, &instance)?;
        let selection = read_selection_file(&lowered.operations, &config.provider)?;
        let bundle = bundle::load(out, &config.provider).map_err(Failure::from_service)?;
        let bundle_sha256 = bundle::read_index(out).ok().and_then(|index| {
            index
                .find(&config.provider)
                .map(|entry| entry.bundle_sha256.clone())
        });
        Self::configure(
            config,
            Some(Sources {
                bundle,
                bundle_sha256,
                selection: Some(selection),
            }),
        )
    }
    /// `shipped` carries the compiled-in bundle and selection of a derived
    /// configuration; without it they are read from the configuration's paths.
    fn configure(mut config: Configuration, shipped: Option<Sources>) -> Result<Self> {
        let oauth = config.auth.scheme.is_oauth();
        // The declaration belongs to `oauth2_grant` alone, which needs one,
        // and its client id is resolved before anything digests it.
        let grant = config.auth.scheme == Scheme::OAuth2Grant;
        if config.auth.oauth2.is_some() != grant || (!grant && config.oauth2_client_id.is_some()) {
            return Err(Failure::InvalidConfiguration);
        }
        if let Some(declaration) = &mut config.auth.oauth2 {
            declaration.client_id = Some(grant_client_id(
                declaration.client_id.as_deref(),
                config.oauth2_client_id.as_deref(),
            )?);
            if !usable_declaration(declaration) {
                return Err(Failure::InvalidConfiguration);
            }
        }
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
                        || (config.auth.scopes.is_none()
                            && !config.auth.minimum_scopes.is_empty()
                            && !grant)
                }
                // The token response names the subject and its scopes; an API
                // probe beside it would be read by nobody. An `oauth2_grant`
                // profile may name its subject by another claim of the
                // `id_token` than `sub`: `subject_pointer` into its claims.
                IdentitySource::IdToken => {
                    !matches!(config.auth.scheme, Scheme::OAuth2Refresh | Scheme::OAuth2Grant)
                        || identity.path.is_some()
                        || identity.subject_pointer.as_deref().is_some_and(|pointer| {
                            config.auth.scheme != Scheme::OAuth2Grant || !pointer.starts_with('/')
                        })
                        || config.auth.scopes.is_some()
                }
                // The read proves the credential and names nobody: a pointer
                // into it would be read by nobody. Only a static credential may
                // be named by its connection: an OAuth credential belongs to a
                // holder the provider names, and a configured subject would let
                // repair accept another holder's credential as the same identity
                // (`connectors_catalog.identity.Profile`). A `github_app`
                // connection is the installation its entry names, which the
                // exchange proves, so its subject is that installation id.
                IdentitySource::Configuration => {
                    config.auth.scheme.is_oauth()
                        || grant
                        || identity
                            .path
                            .as_deref()
                            .is_none_or(|p| segments(p).is_empty())
                        || identity.subject_pointer.is_some()
                        || (config.auth.scopes.is_none() && !config.auth.minimum_scopes.is_empty())
                }
            }
            // A grant entry names the scopes it was granted; a scope read
            // beside it would be read by nobody.
            || (grant && config.auth.scopes.is_some())
            || config
                .auth
                .scopes
                .as_ref()
                .is_some_and(|s| segments(&s.path).is_empty())
            || config.auth.access.as_ref().is_some_and(|a| !a.usable())
            || config
                .bindings
                .as_ref()
                .is_some_and(|bindings| !usable_bindings(bindings, &config))
            || config.auth.prefix.as_deref().is_some_and(|prefix| {
                // One visible-ASCII word of at most 32 bytes, in front of a
                // token only, and never beside `Bearer `
                // (`connectors_catalog.identity.Profile`).
                config.auth.scheme != Scheme::Token || config.auth.bearer || !valid_prefix(prefix)
            })
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
                // No consent: nothing calls an `authorize_url`, so the file
                // carries none.
                Scheme::OAuth2ClientCredentials => {
                    !config.auth.header.eq_ignore_ascii_case("authorization")
                        || !config.auth.bearer
                        || config.auth.account_label.is_some()
                        || config.auth.token_url.is_none()
                        || config.auth.authorize_url.is_some()
                        || !valid_scopes(&config.auth.requested_scopes)
                }
                // Its endpoints and scopes are the declaration's.
                Scheme::OAuth2Grant => {
                    !config.auth.header.eq_ignore_ascii_case("authorization")
                        || !config.auth.bearer
                        || config.auth.account_label.is_some()
                }
                // The exchange goes to the instance's own API base, and the
                // connection is the installation, never a subject read.
                Scheme::GitHubApp => {
                    !config.auth.header.eq_ignore_ascii_case("authorization")
                        || !config.auth.bearer
                        || config.auth.account_label.is_some()
                        || identity.source != IdentitySource::Configuration
                }
            }
            || (!oauth
                && (config.auth.token_url.is_some()
                    || config.auth.token_ca_file.is_some()
                    || config.auth.authorize_url.is_some()
                    || !config.auth.requested_scopes.is_empty()))
        {
            return Err(Failure::InvalidConfiguration);
        }
        // A derived configuration declares no bindings: its sources are compiled in.
        if shipped.is_some() && config.bindings.is_some() {
            return Err(Failure::InvalidConfiguration);
        }
        let Sources {
            bundle,
            bundle_sha256,
            selection,
        } = match shipped {
            Some(sources) => sources,
            None => {
                let selection = match &config.operations_file {
                    Some(path) => Some(read_selection_file(path, &config.provider)?),
                    None => None,
                };
                let bundle = bundle::load(&config.bundle_directory, &config.provider)
                    .map_err(Failure::from_service)?;
                let bundle_sha256 =
                    bundle::read_index(&config.bundle_directory)
                        .ok()
                        .and_then(|index| {
                            index
                                .find(&config.provider)
                                .map(|e| e.bundle_sha256.clone())
                        });
                Sources {
                    bundle,
                    bundle_sha256,
                    selection,
                }
            }
        };
        let mut operations = config.operations.clone();
        let mut feed = None;
        if let Some(selection) = selection {
            operations.extend(selection.operations);
            feed = selection.feed;
        }
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
        let acquisition = if config.auth.scheme == Scheme::OAuth2Refresh {
            let (Some(token_url), Some(authorize_url)) =
                (&config.auth.token_url, &config.auth.authorize_url)
            else {
                return Err(Failure::InvalidConfiguration);
            };
            https_url(authorize_url)?;
            Some(Acquisition::Consent(runtime::Consent {
                authorize_url: authorize_url.clone(),
                token_url: token_url.clone(),
                scopes: config.auth.requested_scopes.clone(),
            }))
        } else {
            config
                .auth
                .oauth2
                .clone()
                .map(|oauth2| Acquisition::Grant(runtime::Grant { oauth2 }))
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
                let scope = (config.auth.scheme == Scheme::OAuth2ClientCredentials).then(|| {
                    let scopes: Vec<&str> = config
                        .auth
                        .requested_scopes
                        .iter()
                        .map(String::as_str)
                        .collect();
                    scopes.join(" ")
                });
                Some(Arc::new(OAuth {
                    http,
                    path,
                    scope,
                    cache: Mutex::new(BTreeMap::new()),
                }))
            }
            // A GitHub App's installation token is issued under the API base,
            // so the exchange reaches the instance's own authority under its
            // trust roots, without the credential header a request carries.
            None if config.auth.scheme == Scheme::GitHubApp => Some(Arc::new(OAuth {
                http: ScopedHttp::new_with_ca_bytes(
                    &HttpConfig {
                        base_url: base.clone(),
                        credential: None,
                        credential_header: "authorization".into(),
                        bearer: false,
                        allow_plaintext: false,
                        ca_file: None,
                    },
                    None,
                    ca.as_deref(),
                )
                .map_err(Failure::from_service)?,
                path: Vec::new(),
                scope: None,
                cache: Mutex::new(BTreeMap::new()),
            })),
            None => None,
        };
        // A feed's root-relative item links resolve under the origin of the API base.
        let engine = Engine::with_feed(&bundle, &document_base, &operations, feed.as_ref())
            .and_then(|engine| engine.with_link_origin(&base))
            .map_err(Failure::from_service)?;
        // Each alternative binding is another bundle and selection set under
        // the same base, profile and credential, compiled as the first is.
        let mut alternatives = Vec::new();
        for alternative in config.bindings.iter().flat_map(|b| &b.alternatives) {
            let selection =
                read_selection_file(&alternative.operations_file, &alternative.provider)?;
            let bundle = bundle::load(&config.bundle_directory, &alternative.provider)
                .map_err(Failure::from_service)?;
            let bundle_sha256 =
                bundle::read_index(&config.bundle_directory)
                    .ok()
                    .and_then(|index| {
                        index
                            .find(&alternative.provider)
                            .map(|e| e.bundle_sha256.clone())
                    });
            let engine = Engine::with_feed(
                &bundle,
                &document_base,
                &selection.operations,
                selection.feed.as_ref(),
            )
            .and_then(|engine| engine.with_link_origin(&base))
            .map_err(Failure::from_service)?;
            let mut revision = json!({
                "id": alternative.id,
                "provider": alternative.provider,
                "bundle_sha256": bundle_sha256,
                "source_sha256": bundle.source.source_sha256,
                "access": alternative.access,
                "operations": serde_json::to_value(&selection.operations)
                    .map_err(|_| Failure::Protocol)?,
            });
            if let Some(next) = &alternative.next {
                revision["next"] = serde_json::to_value(next).map_err(|_| Failure::Protocol)?;
            }
            if let Some(feed) = &selection.feed {
                revision["feed"] = serde_json::to_value(feed).map_err(|_| Failure::Protocol)?;
            }
            alternatives.push(LoadedAlternative {
                config: alternative.clone(),
                engine,
                revision,
            });
        }
        // A connection bound to one of several descriptions reads only: a
        // write's approval and audit bind one description, not a choice of them.
        if config.bindings.is_some()
            && std::iter::once(&engine)
                .chain(alternatives.iter().map(|a| &a.engine))
                .any(|engine| !engine.declarations(&[Effect::Write]).is_empty())
        {
            return Err(Failure::InvalidConfiguration);
        }
        // Every admitted host is a distinct canonical https origin that some
        // selected binary read reaches: the selection names what it may need,
        // the connection decides what it allows. A reached host the connection
        // does not admit is not a load failure, so a connection written before
        // a selection reached it keeps loading; the read that needs it is
        // refused as `forbidden` before anything is sent there. Every binding's
        // selection counts.
        let mut reached = engine.reached_hosts();
        for alternative in &alternatives {
            reached.extend(alternative.engine.reached_hosts());
        }
        let mut admitted = BTreeSet::new();
        if config.hosts.len() > ADMITTED_HOSTS
            || config.hosts.iter().any(|host| {
                connectors_catalog_provider::origin(&host.origin).as_deref()
                    != Some(host.origin.as_str())
                    || !admitted.insert(host.origin.as_str())
                    || !reached.contains(host.origin.as_str())
            })
        {
            return Err(Failure::InvalidConfiguration);
        }
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
            "bundle_sha256": bundle_sha256,
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
        // Present only when declared, so a configuration without a feed keeps its revision.
        if let Some(feed) = &feed {
            effective["feed"] = serde_json::to_value(feed).map_err(|_| Failure::Protocol)?;
        }
        // Present only when stated, so a configuration without it keeps its revision.
        if let Some(prefix) = &config.request_prefix {
            effective["request_prefix"] = json!(prefix);
        }
        // Present only when declared, so a configuration without hosts keeps its revision.
        if !config.hosts.is_empty() {
            effective["hosts"] =
                serde_json::to_value(&config.hosts).map_err(|_| Failure::Protocol)?;
        }
        // Present only when declared, so a configuration without bindings keeps
        // its revision. Each alternative enters by its content, as the first
        // binding does above. The bindings stay out of the credential side:
        // they decide which reads are described, not where the credential goes.
        if let Some(bindings) = &config.bindings {
            effective["bindings"] = json!({
                "id": bindings.id,
                "next": bindings.next,
                "alternatives": alternatives.iter().map(|a| a.revision.clone()).collect::<Vec<_>>(),
            });
        }
        let configuration_revision = connectors_core::digest(&effective);
        let revision_inputs = effective
            .as_object()
            .ok_or(Failure::Protocol)?
            .iter()
            .map(|(key, value)| (key.clone(), connectors_core::digest(value)))
            .collect();
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
        // Each admitted origin gets its own port under the same trust roots and
        // credential placement as the API base; the credential is attached per
        // request, and only where the connection says it travels.
        let mut hosts = BTreeMap::new();
        for host in &config.hosts {
            let port = ScopedHttp::new_with_ca_bytes(
                &HttpConfig {
                    base_url: format!("{}/", host.origin),
                    credential: None,
                    credential_header: config.auth.header.clone(),
                    bearer: config.auth.bearer,
                    allow_plaintext: false,
                    ca_file: None,
                },
                None,
                ca.as_deref(),
            )
            .map_err(Failure::from_service)?;
            hosts.insert(host.origin.clone(), (Arc::new(port), host.credential));
        }
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
            (Scheme::OAuth2ClientCredentials, _) => (
                "http_bearer",
                "http-bearer",
                vec![
                    field("client_id", "OAuth client ID", 8192),
                    field("client_secret", &config.auth.label, 8192),
                ],
            ),
            // The CLI runs the grant and writes the entry; what a person
            // enters is a confidential client's secret, which a public client
            // has none of.
            (Scheme::OAuth2Grant, _) => (
                "http_bearer",
                "http-bearer",
                vec![field("client_secret", "OAuth client secret", 8192)],
            ),
            // The App, its installation and the App's private key. A PEM key
            // spans lines, so it is entered by file or mounted, not typed.
            (Scheme::GitHubApp, _) => (
                "http_bearer",
                "http-bearer",
                vec![
                    field("app_id", "GitHub App ID", APP_ID_LIMIT as u32),
                    field(
                        "installation_id",
                        "GitHub App installation ID",
                        INSTALLATION_ID_LIMIT as u32,
                    ),
                    field("private_key", &config.auth.label, APP_KEY_LIMIT as u32),
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
        // The credential side of the configuration (architecture-decision-record:
        // provider-definitions-and-runtimes D12): what decides where the
        // credential is sent and how it authenticates. The bundle, its source,
        // the selection and the feed stay out of it, so a change limited to them
        // leaves a saved connection bound. Each key has the form it has in the
        // configuration revision, which keeps its inputs and values.
        let mut credential_side = json!({
            "instance": effective["instance"],
            "provider": effective["provider"],
            "api_base": effective["api_base"],
            "ca_digest": effective["ca_digest"],
            "auth": effective["auth"],
            "profile_revision": profile.revision,
        });
        for key in ["token_ca_digest", "request_prefix", "hosts"] {
            if let Some(value) = effective.get(key) {
                credential_side[key] = value.clone();
            }
        }
        let credential_revision = connectors_core::digest(&credential_side);
        // A binding's description names it in its revision, so the revision a
        // read carries says which binding it was described under. Without
        // bindings the revision keeps its inputs.
        let descriptor = |operations: Vec<connectors_core::Operation>,
                          binding: Option<&str>|
         -> Result<String> {
            let mut revision = json!({
                "configuration_revision": configuration_revision,
                "operations": operations.iter().map(|o| o.id.clone()).collect::<Vec<_>>(),
            });
            if let Some(binding) = binding {
                revision["binding"] = json!(binding);
            }
            let descriptor = connectors_core::Descriptor {
                version: connectors_core::WIRE_VERSION.into(),
                instance: config.instance.clone(),
                adapter: "catalog".into(),
                revision: connectors_core::digest(&revision),
                operations,
                configuration_schema: json!({"type": "object"}),
            };
            serde_json::to_string(&descriptor).map_err(|_| Failure::Protocol)
        };
        let requirement = |engine: &Engine, operation: &connectors_core::Operation| Requirement {
            operation: operation.id.clone(),
            custody: engine.declares_custody(&operation.id),
            profile: config.auth.profile.clone(),
            scopes: config.auth.minimum_scopes.clone(),
            effect: match engine.effect(&operation.id) {
                Some(Effect::Read) => runtime::Effect::Read,
                Some(Effect::Write) => runtime::Effect::Write,
                None => runtime::Effect::Unknown,
            },
        };
        let first = config.bindings.as_ref().map(|b| b.id.as_str());
        let bindings = match &config.bindings {
            None => None,
            Some(declared) => Some(runtime::Bindings {
                id: declared.id.clone(),
                alternatives: alternatives
                    .iter()
                    .map(|alternative| {
                        let reads = alternative.engine.declarations(&[Effect::Read]);
                        Ok(runtime::AlternativeBinding {
                            id: alternative.config.id.clone(),
                            descriptor: descriptor(reads.clone(), Some(&alternative.config.id))?,
                            requirements: reads
                                .iter()
                                .map(|operation| requirement(&alternative.engine, operation))
                                .collect(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            }),
        };
        let reads = engine.declarations(&[Effect::Read]);
        let all = engine.declarations(&[Effect::Read, Effect::Write]);
        let bootstrap = Bootstrap {
            credential_revision: Some(credential_revision),
            executable: None,
            instance: config.instance.clone(),
            adapter: "catalog".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision: configuration_revision.clone(),
            provider_authority: base.clone(),
            descriptor: descriptor(reads.clone(), first)?,
            profiles: vec![profile],
            requirements: reads
                .iter()
                .map(|operation| requirement(&engine, operation))
                .collect(),
            bindings,
        };
        bootstrap.validate()?;
        let mut bootstrap_v2 = bootstrap.clone();
        bootstrap_v2.descriptor = descriptor(all.clone(), first)?;
        bootstrap_v2.requirements = all
            .iter()
            .map(|operation| requirement(&engine, operation))
            .collect();
        bootstrap_v2.validate_for(runtime::PrivateProtocol::V2)?;
        let mut steps = Vec::new();
        if let (Some(declared), Some(access)) = (&config.bindings, &config.auth.access) {
            steps.push(Step {
                id: declared.id.clone(),
                access: access.clone(),
                next: Some(declared.next.clone()),
                engine: None,
            });
            for alternative in alternatives {
                steps.push(Step {
                    id: alternative.config.id,
                    access: alternative.config.access,
                    next: alternative.config.next,
                    engine: Some(alternative.engine),
                });
            }
        }
        Ok(Self {
            bootstrap,
            bootstrap_v2,
            revision_inputs,
            http,
            hosts,
            engine,
            steps,
            instance: config.instance,
            auth: config.auth,
            oauth: token_client,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    /// Per key of the configuration revision's digest input, that value's
    /// digest: where two configurations' revisions differ, the keys that
    /// differ (`setup migrate` names the first).
    pub fn revision_inputs(&self) -> &BTreeMap<String, String> {
        &self.revision_inputs
    }
    /// Whether [`Self::upstream_token`] serves this profile: a `token`
    /// profile without a prefix, or a `github_app` profile.
    #[cfg(unix)]
    pub fn has_upstream_token(&self) -> bool {
        match self.auth.scheme {
            Scheme::Token => self.auth.prefix.is_none(),
            Scheme::GitHubApp => true,
            _ => false,
        }
    }
    /// The bare token the git relay sends to the forge for this profile's
    /// document (`served::git`): the stored token of a `token` profile, or
    /// the installation token a `github_app` entry is exchanged for, from the
    /// same in-memory cache the operations use. Any other profile is
    /// `unsupported`.
    #[cfg(unix)]
    pub async fn upstream_token(&self, document: Secret) -> Result<Secret> {
        if !self.has_upstream_token() {
            return Err(Failure::Unsupported);
        }
        let (token, _) = self.credential(document).await?;
        Ok(token)
    }
    /// The API port carrying `credential` as this profile's header.
    fn with(&self, credential: Secret) -> ScopedHttp {
        self.http.with_credential(Arc::new(Fixed(credential)))
    }
    /// The ports of the admitted origins for one request: carrying
    /// `credential` where the connection sends it, and nothing elsewhere.
    fn host_ports(
        &self,
        credential: &Arc<dyn Credential>,
    ) -> BTreeMap<String, Arc<dyn connectors_sdk::AuthenticatedHttp>> {
        self.hosts
            .iter()
            .map(|(origin, (port, carries))| {
                let port: Arc<dyn connectors_sdk::AuthenticatedHttp> = if *carries {
                    Arc::new(port.with_credential(credential.clone()))
                } else {
                    port.clone()
                };
                (origin.clone(), port)
            })
            .collect()
    }
    /// [`Self::host_ports`], each port inside a bounded read's provider
    /// cutoff when there is one.
    fn host_ports_until(
        &self,
        credential: &Arc<dyn Credential>,
        until: Option<Instant>,
    ) -> Result<BTreeMap<String, Arc<dyn connectors_sdk::AuthenticatedHttp>>> {
        let Some(until) = until else {
            return Ok(self.host_ports(credential));
        };
        self.hosts
            .iter()
            .map(|(origin, (port, carries))| {
                let port = if *carries {
                    bounded_http(port.with_credential(credential.clone()), Some(until))?
                } else {
                    port.with_provider_budget(PROVIDER_BUDGET_LIMIT, until)
                        .map_err(Failure::from_service)?
                };
                let port: Arc<dyn connectors_sdk::AuthenticatedHttp> = Arc::new(port);
                Ok((origin.clone(), port))
            })
            .collect()
    }
    /// The bounded protected document, taken out of the caller's buffer.
    fn document(mut document: Secret) -> Result<Zeroizing<Vec<u8>>> {
        let document = Zeroizing::new(std::mem::take(&mut document.0));
        if document.len() > DOCUMENT_LIMIT {
            return Err(Failure::InvalidInput);
        }
        Ok(document)
    }
    /// An OAuth profile's entry and its cache key, the digest of its bytes.
    fn oauth_entry(&self, document: Secret) -> Result<(&OAuth, Grant, [u8; 32])> {
        let oauth = self.oauth.as_deref().ok_or(Failure::InvalidConfiguration)?;
        let document = Self::document(document)?;
        let grant = match self.auth.scheme {
            Scheme::OAuth2Refresh => Grant::Refresh(
                serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?,
            ),
            Scheme::OAuth2ClientCredentials => Grant::ClientCredentials(
                serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?,
            ),
            Scheme::GitHubApp => Grant::App(AppEntry::parse(&document)?),
            Scheme::Token | Scheme::Basic | Scheme::OAuth2Grant => {
                return Err(Failure::InvalidConfiguration);
            }
        };
        Ok((oauth, grant, Sha256::digest(document.as_slice()).into()))
    }
    /// An `oauth2_grant` profile's entry, no larger than the CLI builds one.
    fn grant_entry(mut document: Secret) -> Result<GrantEntry> {
        let document = Zeroizing::new(std::mem::take(&mut document.0));
        if document.len() > ENTRY_LIMIT {
            return Err(Failure::InvalidInput);
        }
        serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)
    }
    /// The credential value a request carries, derived from the protected
    /// document (the token, the basic header value or an OAuth access token),
    /// and for an OAuth profile the cache key of that access token.
    async fn credential(&self, document: Secret) -> Result<(Secret, Option<[u8; 32]>)> {
        self.credential_until(document, None).await
    }
    /// [`Self::credential`], its token exchange inside a bounded read's
    /// provider cutoff when there is one.
    async fn credential_until(
        &self,
        document: Secret,
        until: Option<Instant>,
    ) -> Result<(Secret, Option<[u8; 32]>)> {
        let credential = match self.auth.scheme {
            Scheme::Token => {
                let document = Self::document(document)?;
                let entry: ProtectedEntry =
                    serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
                match &self.auth.prefix {
                    Some(prefix) => prefixed(prefix, &entry.token),
                    None => Secret(entry.token.as_bytes().to_vec()),
                }
            }
            Scheme::Basic => {
                let document = Self::document(document)?;
                let entry: BasicEntry =
                    serde_json::from_slice(&document).map_err(|_| Failure::InvalidInput)?;
                basic_header(&entry)?
            }
            // The stored access token, until the expiry the grant recorded;
            // nothing here refreshes it.
            Scheme::OAuth2Grant => {
                let entry = Self::grant_entry(document)?;
                if entry.expires_at_ms <= connectors_sdk::now_ms() {
                    return Err(Failure::InvalidCredential);
                }
                Secret(entry.access_token.as_bytes().to_vec())
            }
            Scheme::OAuth2Refresh | Scheme::OAuth2ClientCredentials | Scheme::GitHubApp => {
                let (oauth, grant, key) = self.oauth_entry(document)?;
                let access = match oauth.cached(&key) {
                    Some(access) => access,
                    None => {
                        let exchanged = oauth.exchange_until(&grant, until).await?;
                        oauth.store(key, &exchanged);
                        exchanged.access
                    }
                };
                return Ok((Secret(access.as_bytes().to_vec()), Some(key)));
            }
        };
        Ok((credential, None))
    }
    fn baseline(
        &self,
        subject: String,
        granted_scopes: Option<BTreeSet<String>>,
        collected_at_ms: u64,
        binding: Option<String>,
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
            binding,
        })
    }
    /// The access read of one binding: its status and the admitted reason of
    /// a refusal.
    async fn access(
        &self,
        http: &ScopedHttp,
        access: &AccessProbe,
    ) -> Result<(u16, Option<String>)> {
        let query: Vec<(&str, String)> = access
            .query
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();
        let response = http
            .get(&segments(&access.path), &query)
            .await
            .map_err(Failure::from_provider)?;
        let reason = (!(200..=299).contains(&response.status)
            && response.body.len() <= DOCUMENT_LIMIT)
            .then(|| connectors_core::reason::from_body(&response.body))
            .flatten();
        Ok((response.status, reason))
    }
    /// The binding the credential can read (`connectors_catalog.binding`):
    /// each binding's access read in order, the first answered binds, and a
    /// refusal moves on only when it is the answer the binding's `next`
    /// names. Any other refusal stands, as `insufficient_scope` naming every
    /// answer seen; `derived` is the credential value the reads carried, and a
    /// reason holding any of it is withheld.
    async fn select(
        &self,
        http: &ScopedHttp,
        derived: &[u8],
    ) -> std::result::Result<String, runtime::Refusal> {
        let mut answers = Vec::new();
        for step in &self.steps {
            let (status, reason) = self.access(http, &step.access).await?;
            if (200..=299).contains(&status) {
                return Ok(step.id.clone());
            }
            let moves_on = step
                .next
                .as_ref()
                .is_some_and(|next| next.matches(status, reason.as_deref()));
            answers.push(match reason {
                Some(reason) => format!("binding {} answered {status} ({reason})", step.id),
                None => format!("binding {} answered {status}", step.id),
            });
            if moves_on {
                continue;
            }
            if !matches!(status, 401 | 403) {
                return Err(probe_failure(status).into());
            }
            break;
        }
        let reason = connectors_core::reason::admit(&answers.join("; "))
            .filter(|reason| !connectors_core::reason::carries_value(reason, derived));
        Err(runtime::Refusal {
            failure: Failure::InsufficientScope,
            reason,
            retry_after_seconds: None,
        })
    }
    /// The declared identity read, the scope read when there is one, and the
    /// access read, or with bindings the binding the credential can read.
    /// `derived` is the credential value the reads carry.
    async fn probe_identity(
        &self,
        http: &ScopedHttp,
        derived: &[u8],
        collected_at_ms: u64,
    ) -> std::result::Result<Baseline, runtime::Refusal> {
        let identity_path = self
            .auth
            .identity
            .path
            .as_deref()
            .ok_or(Failure::InvalidConfiguration)?;
        let response = http
            .get(&segments(identity_path), &[])
            .await
            .map_err(Failure::from_provider)?;
        if response.status != 200 {
            return Err(probe_failure(response.status).into());
        }
        // A configured subject is the instance id: the answer proved the
        // credential and is not read, so its size and shape do not matter.
        let subject = match (
            &self.auth.identity.subject_pointer,
            self.auth.identity.source,
        ) {
            (None, IdentitySource::Configuration) => self.instance.clone(),
            (Some(subject_pointer), IdentitySource::Api) => {
                if response.body.len() > DOCUMENT_LIMIT {
                    return Err(Failure::Protocol.into());
                }
                let identity: Value =
                    connectors_core::read_json(&response.body).map_err(|_| Failure::Protocol)?;
                match identity.pointer(subject_pointer) {
                    Some(Value::String(text)) if !text.is_empty() && text.len() <= 256 => {
                        text.clone()
                    }
                    Some(Value::Number(number)) => number.to_string(),
                    _ => return Err(Failure::Protocol.into()),
                }
            }
            _ => return Err(Failure::InvalidConfiguration.into()),
        };
        let granted_scopes = match &self.auth.scopes {
            None => None,
            Some(probe) => {
                let response = http
                    .get(&segments(&probe.path), &[])
                    .await
                    .map_err(Failure::from_provider)?;
                if response.status != 200 {
                    return Err(probe_failure(response.status).into());
                }
                if response.body.len() > DOCUMENT_LIMIT {
                    return Err(Failure::Protocol.into());
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
                        return Err(Failure::Protocol.into());
                    }
                    granted.insert(scope.to_owned());
                }
                if granted.len() > 64 {
                    return Err(Failure::Protocol.into());
                }
                if !self.auth.minimum_scopes.is_subset(&granted) {
                    return Err(Failure::InsufficientScope.into());
                }
                Some(granted)
            }
        };
        // The credential identified its holder a moment ago; a refusal of an
        // access read is what it may not do, not a credential to repair.
        let binding = if !self.steps.is_empty() {
            Some(self.select(http, derived).await?)
        } else {
            if let Some(access) = &self.auth.access {
                match self.access(http, access).await?.0 {
                    200..=299 => {}
                    401 | 403 => return Err(Failure::InsufficientScope.into()),
                    status => return Err(probe_failure(status).into()),
                }
            }
            None
        };
        Ok(self.baseline(subject, granted_scopes, collected_at_ms, binding)?)
    }
    /// [`runtime::Adapter::validate_explained`].
    async fn validate_refusing(
        &self,
        profile: &str,
        document: Secret,
    ) -> std::result::Result<Baseline, runtime::Refusal> {
        if profile != self.auth.profile {
            return Err(Failure::Unsupported.into());
        }
        let collected_at_ms = connectors_sdk::now_ms();
        if self.auth.scheme == Scheme::OAuth2Grant {
            return self.validate_grant(document, collected_at_ms).await;
        }
        if !self.auth.scheme.exchanges() {
            let (credential, _) = self.credential(document).await?;
            let derived = Zeroizing::new(credential.0.clone());
            let http = self.with(credential);
            return self.probe_identity(&http, &derived, collected_at_ms).await;
        }
        // Validation always proves the stored entry itself, never a cache, and
        // caches the fresh access token only once the credential validated.
        let (oauth, grant, key) = self.oauth_entry(document)?;
        let result = async {
            let exchanged = oauth.exchange(&grant).await?;
            let baseline = match (self.auth.identity.source, &grant) {
                (IdentitySource::IdToken, Grant::Refresh(entry)) => {
                    let (subject, scope) = oauth.identity(entry, &exchanged).await?;
                    if subject.is_empty() || subject.len() > 256 {
                        return Err(Failure::Protocol.into());
                    }
                    let granted = scope_set(&scope)?;
                    if !self.auth.minimum_scopes.is_subset(&granted) {
                        return Err(Failure::InsufficientScope.into());
                    }
                    self.baseline(subject, Some(granted), collected_at_ms, None)?
                }
                (IdentitySource::IdToken, Grant::ClientCredentials(_) | Grant::App(_)) => {
                    return Err(Failure::InvalidConfiguration.into());
                }
                // The exchange proved the App's key for the installation, and
                // the read proves the token it yielded: the connection is that
                // installation.
                (IdentitySource::Configuration, Grant::App(entry)) => {
                    let derived = Zeroizing::new(exchanged.access.as_bytes().to_vec());
                    let http = self.with(Secret(derived.to_vec()));
                    let mut baseline = self
                        .probe_identity(&http, &derived, collected_at_ms)
                        .await?;
                    baseline.identity.subject = entry.installation_id.to_string();
                    baseline
                }
                // Refused at load; never reached.
                (IdentitySource::Configuration, _) => {
                    return Err(Failure::InvalidConfiguration.into());
                }
                (IdentitySource::Api, _) => {
                    let derived = Zeroizing::new(exchanged.access.as_bytes().to_vec());
                    let http = self.with(Secret(derived.to_vec()));
                    self.probe_identity(&http, &derived, collected_at_ms)
                        .await?
                }
            };
            oauth.store(key, &exchanged);
            Ok(baseline)
        }
        .await;
        self.evict_refused(
            Some(key),
            &result
                .as_ref()
                .map(|_| ())
                .map_err(|refusal: &runtime::Refusal| refusal.failure),
        );
        result
    }
    /// An `oauth2_grant` entry, proved without refreshing it: issued to this
    /// profile's client and not expired, holding the minimum and the required
    /// scopes, and naming its holder by its `id_token` or by the identity read
    /// with its access token. The entry's scopes and expiry are the evidence's.
    async fn validate_grant(
        &self,
        document: Secret,
        collected_at_ms: u64,
    ) -> std::result::Result<Baseline, runtime::Refusal> {
        let declaration = self
            .auth
            .oauth2
            .as_ref()
            .ok_or(Failure::InvalidConfiguration)?;
        let entry = Self::grant_entry(document)?;
        if entry.scopes.len() > 64
            || entry.scopes.iter().any(|scope| {
                scope.is_empty()
                    || scope.len() > 256
                    || !scope.bytes().all(|b| b.is_ascii_graphic())
            })
        {
            return Err(Failure::InvalidInput.into());
        }
        if declaration.client_id.as_deref() != Some(entry.client_id.as_str())
            || entry.expires_at_ms <= collected_at_ms
        {
            return Err(Failure::InvalidCredential.into());
        }
        if !self.auth.minimum_scopes.is_subset(&entry.scopes)
            || declaration
                .required_scopes
                .iter()
                .any(|scope| !entry.scopes.contains(scope))
        {
            return Err(Failure::InsufficientScope.into());
        }
        let mut baseline = match self.auth.identity.source {
            IdentitySource::IdToken => {
                let id_token = entry
                    .id_token
                    .as_deref()
                    .ok_or(Failure::InvalidCredential)?;
                let subject = grant_subject(
                    id_token,
                    &entry.client_id,
                    self.auth.identity.subject_pointer.as_deref(),
                )?;
                self.baseline(subject, None, collected_at_ms, None)?
            }
            IdentitySource::Api => {
                let derived = Zeroizing::new(entry.access_token.as_bytes().to_vec());
                let http = self.with(Secret(derived.to_vec()));
                self.probe_identity(&http, &derived, collected_at_ms)
                    .await?
            }
            // Refused at load; never reached.
            IdentitySource::Configuration => return Err(Failure::InvalidConfiguration.into()),
        };
        baseline.granted_scopes = Some(entry.scopes.clone());
        baseline.credential_expires_at_ms = Some(entry.expires_at_ms);
        baseline.valid_until_ms = baseline.valid_until_ms.min(entry.expires_at_ms);
        Ok(baseline)
    }
    /// The engine of `binding`: the first binding's for none and for the
    /// first, an alternative's for that alternative. The server has checked
    /// the binding against the bootstrap, so another is stale.
    fn engine_of(&self, binding: Option<&str>) -> Result<&Engine> {
        let Some(id) = binding else {
            return Ok(&self.engine);
        };
        let step = self
            .steps
            .iter()
            .find(|step| step.id == id)
            .ok_or(Failure::StaleDescription)?;
        Ok(step.engine.as_ref().unwrap_or(&self.engine))
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

/// Time kept back from a rate-limited read's second request: it must finish
/// this long before the invocation deadline, so the reply still reaches the
/// host in time.
const RETRY_MARGIN: Duration = Duration::from_millis(500);

/// How long to wait before sending a rate-limited read once more: the delay
/// the provider named on its own `429` answer, when the wait from `now`, then
/// a second request as long as the first one took (`latency`), then
/// [`RETRY_MARGIN`], all end strictly before `deadline`. Any other refusal, a
/// `429` naming no readable delay, and a delay that leaves the second request
/// no such time get no wait: the refusal is returned at once, naming the delay
/// when there is one.
fn retry_wait(
    error: &connectors_core::Error,
    latency: Duration,
    now: Instant,
    deadline: Instant,
) -> Option<Duration> {
    if error.code != connectors_core::ErrorCode::RateLimited || !error.upstream_answer {
        return None;
    }
    let wait = Duration::from_secs(error.retry_after_seconds?);
    now.checked_add(wait)?
        .checked_add(latency)?
        .checked_add(RETRY_MARGIN)
        .filter(|end| *end < deadline)
        .map(|_| wait)
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
    fn bootstrap_v3(&self) -> Result<Bootstrap> {
        Ok(self.bootstrap.clone())
    }
    /// A read under the first binding, every provider request of it, the
    /// token exchange included, inside the budget's one provider cutoff. A
    /// rate-limited answer is not waited for: the budget leaves no second
    /// request time of its own.
    async fn invoke_bounded(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> Result<Value> {
        let until = budget.provider_until()?;
        let (credential, key) = self.credential_until(document, Some(until)).await?;
        let credential: Arc<dyn Credential> = Arc::new(Fixed(credential));
        let http = bounded_http(self.http.with_credential(credential.clone()), Some(until))?;
        let hosts = self.host_ports_until(&credential, Some(until))?;
        let result = self
            .engine
            .read_reaching(&http, &hosts, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider);
        self.evict_refused(key, &result);
        result
    }
    async fn prepare_write(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Box<dyn runtime::PreparedWrite>> {
        let (credential, key) = self.credential(document).await?;
        let credential: Arc<dyn Credential> = Arc::new(Fixed(credential));
        let http = self.http.with_credential(credential.clone());
        let prepared = self
            .engine
            .prepare(&http, &self.instance, operation, input)
            .await
            .map_err(Failure::from_provider);
        self.evict_refused(key, &prepared);
        let prepared = prepared?;
        // A guard that reads after the write reads with the same credential,
        // through a port that can only read.
        let reader = prepared
            .reads_after()
            .then(|| self.http.with_credential(credential));
        Ok(Box::new(Write {
            prepared,
            http: http.into_write(),
            reader,
            token: key.and_then(|key| Some((self.oauth.clone()?, key))),
        }))
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline> {
        self.validate_refusing(profile, document)
            .await
            .map_err(|refusal| refusal.failure)
    }
    /// With bindings, a credential every binding refuses is refused as
    /// `insufficient_scope` with a reason naming each binding's answer.
    async fn validate_explained(
        &self,
        profile: &str,
        document: Secret,
    ) -> std::result::Result<Baseline, runtime::Refusal> {
        self.validate_refusing(profile, document).await
    }
    async fn invoke(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value> {
        // No deadline is known here, so no rate-limited read is waited for.
        self.invoke_explained(operation, partition, document, input, Instant::now())
            .await
            .map_err(|refusal| refusal.failure)
    }
    /// A cached OAuth access token the provider refuses is dropped and the
    /// read is sent once more with a token the stored entry is exchanged for,
    /// so `invalid_credential` names a refusal of the stored credential only:
    /// a refused exchange, or its fresh token refused. The host records that
    /// as a Stale connection (story:ess-connector-vocabulary-model decision 16).
    async fn invoke_explained(
        &self,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
        deadline: Instant,
    ) -> std::result::Result<Value, runtime::Refusal> {
        self.read(
            &self.engine,
            operation,
            partition,
            document,
            input,
            deadline,
        )
        .await
    }
    /// A read under the binding the connection's evidence selected, by that
    /// binding's engine: only its own selection's ids and requests.
    async fn invoke_bound(
        &self,
        binding: Option<&str>,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
        deadline: Instant,
    ) -> std::result::Result<Value, runtime::Refusal> {
        let engine = self.engine_of(binding)?;
        self.read(engine, operation, partition, document, input, deadline)
            .await
    }
}
impl Local {
    /// [`runtime::Adapter::invoke_explained`] by `engine`.
    async fn read(
        &self,
        engine: &Engine,
        operation: &str,
        partition: &str,
        document: Secret,
        input: Value,
        deadline: Instant,
    ) -> std::result::Result<Value, runtime::Refusal> {
        let cached = self.oauth.as_deref().is_some_and(|oauth| {
            document.0.len() <= DOCUMENT_LIMIT
                && oauth
                    .cached(&Sha256::digest(document.0.as_slice()).into())
                    .is_some()
        });
        let again = cached.then(|| Secret(document.0.clone()));
        let result = self
            .read_once(
                engine,
                operation,
                partition,
                document,
                input.clone(),
                deadline,
            )
            .await;
        match (result, again) {
            (Err(refusal), Some(document)) if refusal.failure == Failure::InvalidCredential => {
                self.read_once(engine, operation, partition, document, input, deadline)
                    .await
            }
            (result, _) => result,
        }
    }
    /// One read with the credential the stored entry yields now: the cached
    /// access token while it is usable, otherwise a fresh exchange.
    async fn read_once(
        &self,
        engine: &Engine,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
        deadline: Instant,
    ) -> std::result::Result<Value, runtime::Refusal> {
        let (credential, key) = self.credential(document).await?;
        // The host checks a reason against the document it sent; only this
        // child holds what it derived from it (the basic header value, an
        // OAuth access token), so it withholds a reason holding any of that.
        let derived = Zeroizing::new(credential.0.clone());
        let credential: Arc<dyn Credential> = Arc::new(Fixed(credential));
        let http = self.http.with_credential(credential.clone());
        let hosts = self.host_ports(&credential);
        let started = Instant::now();
        let mut result = engine
            .read_reaching(&http, &hosts, &self.instance, operation, input.clone())
            .await;
        // A read is sent once more, and only once, after the delay the
        // provider named on its own `429`, when that wait still leaves the
        // second request time of its own before the invocation deadline. The
        // second request is bounded by the deadline less the margin; if it
        // does not finish, the first answer's refusal, with its delay, stands.
        // A write never comes here.
        if let Err(error) = &result
            && let Some(wait) = retry_wait(error, started.elapsed(), Instant::now(), deadline)
            && let Some(bound) = deadline.checked_sub(RETRY_MARGIN)
        {
            tokio::time::sleep(wait).await;
            let second = engine.read_reaching(&http, &hosts, &self.instance, operation, input);
            if let Ok(second) =
                tokio::time::timeout_at(tokio::time::Instant::from_std(bound), second).await
            {
                result = second;
            }
        }
        let result = result.map_err(|error| {
            let mut refusal = runtime::Refusal::from_provider(error);
            refusal.reason = refusal.reason.filter(|reason| {
                !connectors_core::reason::carries_value(reason, derived.as_slice())
            });
            refusal
        });
        self.evict_refused(
            key,
            &result
                .as_ref()
                .map(|_| ())
                .map_err(|refusal| refusal.failure),
        );
        result
    }
}
struct Write {
    prepared: connectors_catalog_provider::Prepared,
    http: Box<dyn connectors_sdk::AuthenticatedWrite>,
    /// The read port of a guard that reads after the write; `None` otherwise.
    reader: Option<ScopedHttp>,
    /// The cache and key of the OAuth access token the write carries.
    token: Option<(Arc<OAuth>, [u8; 32])>,
}
#[async_trait::async_trait]
impl runtime::PreparedWrite for Write {
    /// A write whose selection declares custody is refused here before it is
    /// sent (`Prepared::execute_reading`): only `execute_created` holds it.
    async fn execute(self: Box<Self>) -> connectors_sdk::WriteOutcome<Value> {
        let Write {
            prepared,
            http,
            reader,
            token,
        } = *self;
        let outcome = prepared
            .execute_reading(
                http,
                reader
                    .as_ref()
                    .map(|reader| reader as &dyn connectors_sdk::AuthenticatedHttp),
            )
            .await;
        evict_refused_token(token.as_ref(), &outcome);
        outcome
    }
    /// The created credential leaves the answer here, into the protected slot
    /// of the write's result (design §8a).
    async fn execute_created(self: Box<Self>) -> connectors_sdk::WriteOutcome<runtime::Created> {
        let Write {
            prepared,
            http,
            reader,
            token,
        } = *self;
        let outcome = prepared
            .execute_holding(
                http,
                reader
                    .as_ref()
                    .map(|reader| reader as &dyn connectors_sdk::AuthenticatedHttp),
            )
            .await
            .try_map(|held| {
                Ok(runtime::Created {
                    result: held.value,
                    secret: held.secret,
                })
            });
        evict_refused_token(token.as_ref(), &outcome);
        outcome
    }
}

/// A write the provider refused as unauthorized refused the token too.
fn evict_refused_token<T>(
    token: Option<&(Arc<OAuth>, [u8; 32])>,
    outcome: &connectors_sdk::WriteOutcome<T>,
) {
    if let (Some((oauth, key)), connectors_sdk::WriteOutcome::Refused(error)) = (token, outcome)
        && error.code == connectors_core::ErrorCode::Unauthorized
    {
        oauth.evict(key);
    }
}

// Unix only, as the catalog's other local TLS tests: the trust anchor is
// written owner-only through `PermissionsExt`.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use runtime::Adapter as _;
    use std::{fs, os::unix::fs::PermissionsExt as _};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio_rustls::{
        TlsAcceptor,
        rustls::{self, pki_types::PrivatePkcs8KeyDer},
    };

    /// A JSON document holding `value` at the JSON pointer `pointer`.
    fn at_pointer(pointer: &str, value: Value) -> Value {
        let keys: Vec<&str> = pointer.split('/').skip(1).collect();
        keys.into_iter()
            .rev()
            .fold(value, |inner, key| json!({ key: inner }))
    }

    /// Every shipped connector whose declaration names an `api_path` is found
    /// by that field, not by name: a derived instance whose endpoint is a bare
    /// host validates by the profile's identity and scope reads under
    /// `<host><api_path>`. The connector's own rule is data in its
    /// declaration, which this engine reads generically.
    #[test]
    fn a_bare_host_is_probed_under_the_declared_api_path() {
        let hosted: Vec<Declaration> = shipped::CONNECTORS
            .iter()
            .map(|(_, text, _)| connectors_core::read_json::<Declaration>(text.as_bytes()).unwrap())
            .filter(|declaration| declaration.api_path.is_some())
            .collect();
        assert!(
            !hosted.is_empty(),
            "no shipped declaration names an api_path"
        );
        for declaration in hosted {
            a_bare_host_is_probed_under(declaration);
        }
    }

    /// Everything but the fixture's trust root is derived; a derived
    /// configuration names no `ca_file` of its own.
    fn a_bare_host_is_probed_under(declaration: Declaration) {
        let connector = declaration.connector_id.clone();
        let api_path = declaration.api_path.clone().unwrap();
        let auth = declaration
            .profiles
            .iter()
            .find(|profile| profile.scheme == Scheme::Token)
            .unwrap_or_else(|| panic!("{connector}: no token profile to probe"))
            .clone();
        let under = |path: &str| format!("{api_path}/{}", path.trim_start_matches('/'));
        let mut answers = BTreeMap::new();
        let identity = auth.identity.path.as_deref().expect("an API identity");
        let subject = auth.identity.subject_pointer.as_deref().unwrap_or("/id");
        answers.insert(under(identity), at_pointer(subject, json!(7)));
        if let Some(scopes) = &auth.scopes {
            let granted: Vec<&String> = auth.minimum_scopes.iter().collect();
            answers.insert(
                under(&scopes.path),
                at_pointer(&scopes.pointer, json!(granted)),
            );
        }
        if let Some(access) = &auth.access {
            answers.insert(under(&access.path), json!({}));
        }
        let expected: BTreeSet<String> = answers.keys().cloned().collect();
        let directory = tempfile::tempdir().unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.path().join("ca.pem");
        fs::write(&ca, cert.cert.pem()).unwrap();
        fs::set_permissions(&ca, fs::Permissions::from_mode(0o600)).unwrap();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let acceptor = TlsAcceptor::from(Arc::new(tls));
            let targets = Arc::new(Mutex::new(Vec::<String>::new()));
            let seen = targets.clone();
            let served = answers.clone();
            let server = tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let target = head.split_whitespace().nth(1).unwrap_or_default();
                    let target = target.split('?').next().unwrap_or_default();
                    seen.lock().unwrap().push(target.to_owned());
                    let (status, body) = match served.get(target) {
                        Some(body) => (200, body.clone()),
                        None => (404, json!({"message": "404 Not Found"})),
                    };
                    let body = serde_json::to_vec(&body).unwrap();
                    let reply = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(reply.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                    let _ = stream.flush().await;
                }
            });
            let endpoint = format!("https://localhost:{port}");
            let (mut config, sources) = derived(&Instance {
                connector: &connector,
                name: "work",
                instance_id: None,
                endpoint: Some(&endpoint),
                profile: Some(&auth.profile),
            })
            .unwrap();
            assert_eq!(config.api_base, format!("{endpoint}{api_path}"), "{connector}");
            config.ca_file = Some(ca.clone());
            let local = Local::configure(config, Some(sources)).unwrap();
            let validated = local
                .validate(
                    &auth.profile,
                    Secret(br#"{"token":"fixture-token"}"#.to_vec()),
                )
                .await;
            server.abort();
            let probed: BTreeSet<String> = targets.lock().unwrap().iter().cloned().collect();
            assert_eq!(probed, expected, "{connector}");
            assert_eq!(targets.lock().unwrap().len(), expected.len(), "{connector}");
            assert!(validated.is_ok(), "{connector}: {:?}", validated.err());
        });
    }
}
