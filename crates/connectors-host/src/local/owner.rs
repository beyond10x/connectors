//! Owner-side composition for the production local CLI. Provider libraries never
//! receive this authority, the metadata database, or a credential resolver.
pub mod approval_issuance;
mod lifecycle;
mod maintenance;
pub mod mutation;
mod supervisor;
#[cfg(unix)]
mod tether;
mod transport;
use super::{
    config::{Adapter, Config, Paths},
    keyring::custody,
    registry, runtime,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
#[cfg(test)]
pub(crate) use transport::bound_allocator;
pub use transport::{
    Capture, Client, OwnerStop, WriteClient, enable_checkpoints, serve, stop_owner,
};

const VERSION: &str = "connectors-owner/1";
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    InvalidInput,
    ProtectedEntryUnavailable,
    InvalidConfiguration,
    MetadataUnavailable,
    /// A metadata write was definitely not committed because the store's
    /// recorded revision moved under it; the store itself is readable.
    RevisionConflict,
    CustodyUnavailable,
    /// The selected custody binding is read-only (`mounted_secret`): it writes,
    /// rotates and deletes nothing, so the command cannot proceed under it.
    CustodyReadOnly,
    OutcomeUnknown,
    Forbidden,
    NotFound,
    LifecycleConflict,
    Revoked,
    NotGranted,
    IdentityMismatch,
    Unavailable,
    Timeout,
    Interrupted,
    Capacity,
    Unsupported,
    ReadinessMismatch,
    IncarnationMismatch,
    StaleDescription,
    StaleCursor,
    DescriptionUnavailable,
    ServiceFailure,
    OwnerBuildMismatch,
    /// The store was written by a newer binary with another record model;
    /// [`Error::store_version`] names its release or `unrecorded`.
    MetadataSchemaNewer,
}
/// The transient class a revalidation failure ended on after the owner's
/// bounded retries (`connectors.cli.FailureCause`, contracts/cli/v1alpha1/semantics.md
/// §6 "Revalidation retries").
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    ProviderUnavailable,
    ProviderTimeout,
    ProviderRateLimited,
    ProviderInternal,
    ProviderCapacity,
    AdapterExited,
    OwnerStopping,
}
/// Who refused. The same [`Code`] can be the host's own admission refusal or
/// the provider's answer to a dispatched call; the CLI reports them at
/// different stages.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    #[default]
    Host,
    Provider,
}
impl Origin {
    fn is_host(&self) -> bool {
        *self == Self::Host
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: Code,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_code: Option<connectors_core::ErrorCode>,
    /// The upstream's own bounded, redacted reason for a dispatched read it
    /// refused (`runtime::Refusal`); only beside a `service_code` or on a
    /// provider `forbidden`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_reason: Option<String>,
    /// The delay in whole seconds the provider named on a dispatched read it
    /// answered as rate limited (`runtime::Refusal`); only beside
    /// `service_code = rate_limited`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Origin::is_host")]
    pub origin: Origin,
    /// The connection's authentication changed under the configuration; only a
    /// new connection helps (`registry::Failure::BindingChanged`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reconnect: bool,
    /// The connection's provider authority or profile authentication changed
    /// under the configuration: no command under this instance id helps, a new
    /// instance id does (`registry::Failure::AuthenticationChanged`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reconfigure: bool,
    /// The instance holds a published Live or Stale connection, so a connect
    /// is refused; `connections reconnect` replaces that connection's
    /// credential (`registry::Failure::InstanceConnected`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
    /// The instance holds a connection no repair can replace, admitted under
    /// another binding, or more than one: a connect is refused until it is
    /// revoked (`registry::Failure::InstanceConnectionChanged`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub revoke: bool,
    /// A `not_granted` read whose connection's validation evidence expired
    /// while its credential is intact; a revalidation helps, not a repair
    /// (`registry::Failure::EvidenceExpired`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub revalidate: bool,
    /// On `metadata_schema_newer` only: the release the store's writer marker
    /// names, or `unrecorded` (semantics.md §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_version: Option<String>,
    /// On `invalid_configuration` from a clock use only: the configured
    /// `[approval_clock]` is the retired `roughtime-clock/1` table
    /// (`approval_clock_format_retired`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clock_format_retired: bool,
    /// On a revalidation's transient failure only: the class of its last
    /// attempt, and the number of validations the owner sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempts: Option<u32>,
}
impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Self {
            code,
            acquisition: None,
            service_code: None,
            service_reason: None,
            retry_after_seconds: None,
            origin: Origin::Host,
            reconnect: false,
            reconfigure: false,
            held: false,
            revoke: false,
            revalidate: false,
            store_version: None,
            clock_format_retired: false,
            cause: None,
            attempts: None,
        }
    }
}
impl Error {
    /// A clock use that got no clock: a retired `roughtime-clock/1` table or
    /// another invalid clock configuration is `invalid_configuration`, and
    /// any other refusal is `unavailable`.
    pub(crate) fn clock(
        selected: &super::clock::ApprovalClock,
        failure: super::clock::Failure,
    ) -> Self {
        match failure {
            super::clock::Failure::InvalidConfiguration => {
                let mut error = Self::from(Code::InvalidConfiguration);
                error.clock_format_retired = selected.is_retired();
                error
            }
            super::clock::Failure::Unavailable => Code::Unavailable.into(),
        }
    }
    /// `metadata_schema_newer`, naming the marker's release or `unrecorded`.
    #[must_use]
    pub fn schema_newer(store: Option<super::Release>) -> Self {
        let mut error = Self::from(Code::MetadataSchemaNewer);
        error.store_version =
            Some(store.map_or_else(|| "unrecorded".to_owned(), |release| release.to_string()));
        error
    }
}
impl From<super::Failure> for Error {
    fn from(e: super::Failure) -> Self {
        match e {
            super::Failure::MetadataSchemaNewer(store) => return Self::schema_newer(store),
            super::Failure::InvalidConfiguration | super::Failure::ConfigurationExists => {
                Code::InvalidConfiguration
            }
            super::Failure::MetadataUnavailable => Code::MetadataUnavailable,
            super::Failure::ConcurrentRevision => Code::RevisionConflict,
            super::Failure::OutcomeUnknown => Code::OutcomeUnknown,
        }
        .into()
    }
}
impl From<registry::Failure> for Error {
    fn from(e: registry::Failure) -> Self {
        use registry::Failure as F;
        if let F::MetadataSchemaNewer(store) = e {
            return Self::schema_newer(store);
        }
        // Only a new connection helps: repair refuses a changed binding.
        let reconnect = matches!(e, F::BindingChanged | F::UpgradeIdentityMismatch);
        let mut error = Error::from(match e {
            F::BindingChanged => Code::LifecycleConflict,
            F::MetadataUnavailable => Code::MetadataUnavailable,
            F::MetadataSchemaNewer(_) => Code::MetadataSchemaNewer,
            F::ConcurrentRevision => Code::RevisionConflict,
            F::OutcomeUnknown => Code::OutcomeUnknown,
            F::NotFound => Code::NotFound,
            F::Conflict
            | F::UpgradeRequired
            | F::ExecutableChanged
            | F::AuthenticationChanged
            | F::InstanceConnected
            | F::InstanceConnectionChanged => Code::LifecycleConflict,
            F::Revoked => Code::Revoked,
            F::IdentityMismatch | F::UpgradeIdentityMismatch => Code::IdentityMismatch,
            F::Expired => Code::Timeout,
            F::InvalidInput => Code::InvalidInput,
            F::StaleCursor => Code::StaleCursor,
            F::Capacity => Code::Capacity,
            F::NotReady | F::EvidenceExpired | F::InsufficientScope => Code::NotGranted,
            F::CustodyUnavailable => Code::CustodyUnavailable,
            F::CustodyReadOnly => Code::CustodyReadOnly,
        });
        error.reconnect = reconnect;
        // Only a new instance id helps: connect refuses it under this one too.
        error.reconfigure = e == F::AuthenticationChanged;
        // The instance already holds its connection: reconnect replaces it.
        error.held = e == F::InstanceConnected;
        // What it holds no reconnect replaces: only a revocation clears it.
        error.revoke = e == F::InstanceConnectionChanged;
        // Revalidation helps: it recollects expired evidence, and it follows a
        // configuration upgrade of the connection's instance.
        error.revalidate = matches!(
            e,
            F::EvidenceExpired | F::UpgradeRequired | F::ExecutableChanged
        );
        error
    }
}
impl From<runtime::Failure> for Error {
    fn from(e: runtime::Failure) -> Self {
        use runtime::Failure as F;
        let code = match e {
            F::InvalidConfiguration => Code::InvalidConfiguration,
            F::InvalidInput => Code::InvalidInput,
            F::Forbidden | F::ProviderForbidden => Code::Forbidden,
            F::NotFound => Code::NotFound,
            F::Unavailable => Code::Unavailable,
            F::Timeout | F::ProviderTimeout => Code::Timeout,
            F::Interrupted => Code::Interrupted,
            F::Capacity | F::ProviderCapacity => Code::Capacity,
            F::Unsupported => Code::Unsupported,
            F::ReadinessMismatch => Code::ReadinessMismatch,
            F::IncarnationMismatch => Code::IncarnationMismatch,
            F::StaleDescription => Code::StaleDescription,
            F::IdentityMismatch => Code::IdentityMismatch,
            F::InsufficientScope => Code::NotGranted,
            F::InvalidCredential
            | F::Protocol
            | F::StaleCursor
            | F::ProviderNotFound
            | F::ProviderRateLimited
            | F::ProviderInternal => Code::ServiceFailure,
        };
        let service_code = match e {
            F::InvalidCredential => Some(connectors_core::ErrorCode::Unauthorized),
            F::Protocol => Some(connectors_core::ErrorCode::UpstreamProtocol),
            F::StaleCursor => Some(connectors_core::ErrorCode::StaleCursor),
            F::ProviderNotFound => Some(connectors_core::ErrorCode::NotFound),
            F::ProviderRateLimited => Some(connectors_core::ErrorCode::RateLimited),
            F::ProviderInternal => Some(connectors_core::ErrorCode::Internal),
            _ => None,
        };
        // Provider only where the child reports the upstream's own answer to a
        // dispatched call (see `Failure::from_provider`). A child's plain
        // `Forbidden` is a refusal it raised from its own configuration before
        // any request, or a connection probe's 403, and keeps reading as
        // admission; the host's own admission refusals are owner codes that
        // never pass through here. Likewise a plain `Timeout` / `Capacity` is
        // the host's or the adapter's own deadline or limit.
        let origin = match e {
            F::ProviderForbidden
            | F::ProviderNotFound
            | F::ProviderTimeout
            | F::ProviderCapacity => Origin::Provider,
            _ => Origin::Host,
        };
        Self {
            code,
            acquisition: None,
            service_code,
            service_reason: None,
            retry_after_seconds: None,
            origin,
            reconnect: false,
            reconfigure: false,
            held: false,
            revoke: false,
            revalidate: false,
            store_version: None,
            clock_format_retired: false,
            cause: None,
            attempts: None,
        }
    }
}
impl From<runtime::Refusal> for Error {
    /// The failure's projection, with the upstream's reason kept only beside
    /// a `service_code` or on the provider's own `forbidden`; a provider
    /// timeout or capacity answer carries none. The delay the provider named
    /// is kept only beside `service_code = rate_limited`, and only within
    /// 0..=4294967295 seconds.
    fn from(refusal: runtime::Refusal) -> Self {
        let forbidden = refusal.failure == runtime::Failure::ProviderForbidden;
        let mut error = Error::from(refusal.failure);
        if error.service_code.is_some() || forbidden {
            error.service_reason = refusal.reason;
        }
        if error.service_code == Some(connectors_core::ErrorCode::RateLimited) {
            error.retry_after_seconds = runtime::admitted_delay(refusal.retry_after_seconds);
        }
        error
    }
}
fn until(deadline: u64) -> Result<Instant> {
    deadline
        .checked_sub(connectors_sdk::now_ms())
        .filter(|n| *n > 0)
        .map(|n| Instant::now() + Duration::from_millis(n))
        .ok_or_else(|| Code::Timeout.into())
}
fn input(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| connectors_core::valid_id(s))
        .map(str::to_owned)
        .ok_or_else(|| Code::InvalidInput.into())
}
fn selected(paths: &Paths, alias: &str) -> Result<(Config, Adapter)> {
    // An invoked definition's instance is selected as a configured one is.
    let config = Config::load_at(paths)?;
    let adapter = config.adapters.get(alias).cloned().ok_or(Code::NotFound)?;
    Ok((config, adapter))
}
/// Whether `source` is another instance of the connector `alias` is an
/// instance of (story:invoke-definition-reuse-credential): both are instances
/// of the configuration's `[connectors]` tables or invoked definitions, under
/// the same connector id. A legacy `[adapters]` entry names no connector, so
/// it is never one.
pub(crate) fn same_connector(config: &Config, alias: &str, source: &str) -> bool {
    let legacy = config.legacy_aliases();
    if alias == source || legacy.contains(&alias) || legacy.contains(&source) {
        return false;
    }
    matches!(
        (config.names(alias), config.names(source)),
        (Some((connector, _)), Some((other, _))) if connector == other
    )
}
/// A stored connection a new connection of an instance may copy its
/// credential from, with the names its instance is selected by.
#[derive(Clone, Debug)]
pub struct Reusable {
    /// The source instance's alias, which a reuse request names.
    pub alias: String,
    pub connector: String,
    pub instance: String,
    pub candidate: registry::ReuseCandidate,
}
/// The connections other instances of `alias`'s connector hold whose
/// credential a new connection of `alias` bound to `target` may reuse
/// (`registry::Registry::reuse_candidates`), by instance and reference. None
/// under a read-only custody binding, which writes no copy. Records nothing
/// and grants nothing: the owner decides a reuse again.
pub fn reusable(paths: &Paths, alias: &str, target: &registry::Binding) -> Result<Vec<Reusable>> {
    let config = Config::load_at(paths)?;
    if config.selection().read_only() || !super::definitions::receives_reuse(paths, &config, alias)?
    {
        return Ok(Vec::new());
    }
    let registry = registry::Registry::with_system_clock(&paths.state);
    let mut found = Vec::new();
    for (source, entry) in &config.adapters {
        if !same_connector(&config, alias, source)
            || entry.instance_id == target.instance_id
            || entry.adapter_id != target.adapter_id
        {
            continue;
        }
        let Some((connector, instance)) = config.names(source) else {
            continue;
        };
        for candidate in registry.reuse_candidates(
            &entry.instance_id,
            &entry.adapter_id,
            entry.configured(),
            target,
            connectors_sdk::now_ms(),
        )? {
            found.push(Reusable {
                alias: source.clone(),
                connector: connector.to_owned(),
                instance: instance.to_owned(),
                candidate,
            });
        }
    }
    Ok(found)
}
/// The adapter's cached description. Outside the owner, a running owner of
/// this build answers it from its held metadata handle; without one the store
/// is read directly, which verifies it completely (`docs/local-er-metadata.md`).
pub fn cached(paths: &Paths, alias: &str) -> Result<runtime::Bootstrap> {
    let mut bootstrap = cached_unbound(paths, alias)?;
    // Bound to the configured entry it was cached under (the executable is
    // never carried by a cached record).
    let (_, adapter) = selected(paths, alias)?;
    bootstrap.configure(&adapter);
    Ok(bootstrap)
}
/// The description a read command (`operations describe`, `operations list`,
/// `operations invoke`) works from: the cached one, or, when none is cached for
/// the instance's configured selection, the one the owner caches by starting
/// the configured, digest-pinned adapter, without reading or asking for any
/// credential. It starts it only when the instance holds a connection, the
/// named `connection` when there is one, that is current under the configured
/// entry and usable (`registry::Registry::launch_admits`); otherwise the
/// command answers `description_unavailable` as before
/// (contracts/cli/v1alpha1/semantics.md, "Launching for a description").
pub fn described(
    paths: &Paths,
    alias: &str,
    connection: Option<&str>,
) -> Result<runtime::Bootstrap> {
    match cached(paths, alias) {
        Err(error) if error.code == Code::DescriptionUnavailable => {
            let (_, adapter) = selected(paths, alias)?;
            if transport::in_owner()
                || runtime::state::State::new(&paths.state).suppressed(&adapter.instance_id)?
                || !registry::Registry::new(&paths.state).launch_admits(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    adapter.configured(),
                    connection,
                )?
            {
                return Err(error);
            }
            let mut bootstrap = Client::connect(paths, true)?.launched(alias, connection)?;
            bootstrap.configure(&adapter);
            Ok(bootstrap)
        }
        other => other,
    }
}
fn cached_unbound(paths: &Paths, alias: &str) -> Result<runtime::Bootstrap> {
    cached_through(paths, alias, |paths, alias| {
        if transport::in_owner() {
            return None;
        }
        let client = Client::connect(paths, false)
            .ok()
            .filter(Client::is_same_build)?;
        Some(client.cached(alias))
    })
}
/// `through_owner` asks a running owner, or answers `None` when there is none
/// to ask. Only its answer is taken; a failed or broken exchange after the
/// greeting, a refusal included, is answered by the direct read, which gives
/// a refusal its own code.
fn cached_through(
    paths: &Paths,
    alias: &str,
    through_owner: impl FnOnce(&Paths, &str) -> Option<Result<runtime::Bootstrap>>,
) -> Result<runtime::Bootstrap> {
    match through_owner(paths, alias) {
        Some(Ok(bootstrap)) => Ok(bootstrap),
        Some(Err(_)) | None => cached_direct(paths, alias),
    }
}
/// The adapter's cached description, read from this process's metadata handle.
fn cached_direct(paths: &Paths, alias: &str) -> Result<runtime::Bootstrap> {
    let (_, adapter) = selected(paths, alias)?;
    let bootstrap = runtime::state::State::new(&paths.state)
        .cached(&adapter.instance_id, &adapter.selection())?
        .ok_or(Code::DescriptionUnavailable)?;
    bootstrap.validate_for(adapter.private_protocol())?;
    Ok(bootstrap)
}
pub fn schema(bootstrap: &runtime::Bootstrap, operation: &str) -> Result<String> {
    let descriptor = bootstrap.descriptor()?;
    let operation = descriptor
        .operation(operation)
        .map_err(|_| Code::NotFound)?;
    Ok(connectors_core::digest(
        &json!({"instance":bootstrap.instance,"revision":descriptor.revision,"operation":operation.id,
        "input":operation.input_schema,"output":operation.output_schema}),
    ))
}
/// Read-only preflight before a CLI is allowed to start an owner or read secrets.
/// The owner repeats it; this is never a reusable grant across launch.
pub fn admit_capture(
    paths: &Paths,
    alias: &str,
    profile: Option<&str>,
    connection: Option<&str>,
    revision: Option<&str>,
) -> Result<String> {
    let (config, adapter) = selected(paths, alias)?;
    let profile = match (profile, connection, revision) {
        (Some(profile), None, None) if connectors_core::valid_id(profile) => profile.to_owned(),
        (None, Some(connection), Some(revision)) => {
            let old = registry::Registry::with_system_clock(&paths.state).describe(
                &adapter.instance_id,
                &adapter.adapter_id,
                adapter.configured(),
                connection,
                connectors_sdk::now_ms(),
                false,
            )?;
            if old.revision != revision {
                return Err(Code::LifecycleConflict.into());
            }
            if old.state == registry::State::Revoked {
                return Err(Code::Revoked.into());
            }
            old.profile
        }
        _ => return Err(Code::InvalidInput.into()),
    };
    if !adapter.permissions.profiles.contains(&profile) {
        return Err(Code::Forbidden.into());
    }
    // Metadata must exist before any private owner startup or protected input.
    super::metadata::Metadata::inspect(&paths.state)?;
    if !custody::available_for(&config.selection()) {
        return Err(Code::CustodyUnavailable.into());
    }
    Ok(profile)
}
/// Admission of a named operation, in the contract's order: an operation the
/// selected description does not expose is `not_found`, one it exposes but the
/// configuration does not grant is `forbidden`. Callers compare the selected
/// revision and schema only after this.
pub fn admit_operation<'a>(
    adapter: &Adapter,
    bootstrap: &'a runtime::Bootstrap,
    operation: &str,
) -> Result<&'a runtime::Requirement> {
    bootstrap
        .descriptor()?
        .operation(operation)
        .map_err(|_| Code::NotFound)?;
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Code::NotFound)?;
    if !adapter.permissions.operations.contains(operation)
        || !adapter.permissions.profiles.contains(&requirement.profile)
    {
        return Err(Code::Forbidden.into());
    }
    Ok(requirement)
}
pub fn operation_snapshot(paths: &Paths, alias: &str, call: &Value) -> Result<runtime::Bootstrap> {
    let (_, adapter) = selected(paths, alias)?;
    let bootstrap = described(paths, alias, call.get("connection").and_then(Value::as_str))?;
    let operation = input(call, "operation")?;
    admit_operation(&adapter, &bootstrap, &operation)?;
    if bootstrap.descriptor()?.revision != input(call, "revision")?
        || schema(&bootstrap, &operation)? != input(call, "schema")?
    {
        return Err(Code::StaleDescription.into());
    }
    Ok(bootstrap)
}
/// Revalidation can use expired baseline evidence but cannot revive material
/// already known to be invalid. Admission precedes all owner/adapter startup.
pub fn admit_revalidation(
    paths: &Paths,
    alias: &str,
    connection: &str,
    revision: &str,
) -> Result<String> {
    let (config, adapter) = selected(paths, alias)?;
    let recorded = registry::Registry::new(&paths.state).revalidation_binding(
        &adapter.instance_id,
        &adapter.adapter_id,
        connection,
        revision,
    )?;
    let profile = recorded.profile.id.clone();
    if !adapter.permissions.profiles.contains(&profile) {
        return Err(Code::Forbidden.into());
    }
    if !custody::available_for(&config.selection()) {
        return Err(Code::CustodyUnavailable.into());
    }
    // Nothing is cached for a configuration until the adapter is launched
    // under it, as after a configuration upgrade. Admit against the binding the
    // connection was admitted under; the owner's capture compares it with the
    // launched provider's binding before any material is read.
    let binding = match cached(paths, alias) {
        Ok(bootstrap) => saved_binding(&bootstrap, &profile)?,
        Err(Error {
            code: Code::DescriptionUnavailable,
            ..
        }) => recorded,
        Err(error) => return Err(error),
    };
    registry::Registry::with_system_clock(&paths.state).admit_revalidation(
        &binding,
        connection,
        revision,
        connectors_sdk::now_ms(),
    )?;
    Ok(profile)
}
/// The configured binding of a saved connection's profile. A profile the
/// configured adapter no longer declares (its `auth.profile` id changed) is a
/// changed binding, refused as a read refuses one (`create_connection`): a new
/// connection under a declared profile follows it, and no command answers
/// `unsupported` with nothing to do next.
pub(crate) fn saved_binding(
    bootstrap: &runtime::Bootstrap,
    profile: &str,
) -> Result<registry::Binding> {
    bootstrap.binding(profile).map_err(|failure| match failure {
        runtime::Failure::Unsupported => registry::Failure::BindingChanged.into(),
        other => other.into(),
    })
}
/// Validate the original text carrier, including duplicate-key and depth checks.
pub fn validate_document(schema: &Value, document: &[u8], limit: usize) -> Result<()> {
    if document.len() > limit {
        return Err(Code::InvalidInput.into());
    }
    runtime::channel::depth(document).map_err(|_| Code::InvalidInput)?;
    let value: Value = connectors_core::read_json(document).map_err(|_| Code::InvalidInput)?;
    connectors_sdk::validate(schema, &value).map_err(|_| Code::InvalidInput.into())
}
pub fn admit_invoke(paths: &Paths, alias: &str, call: &Value, document: &[u8]) -> Result<()> {
    let (config, adapter) = selected(paths, alias)?;
    let bootstrap = operation_snapshot(paths, alias, call)?;
    let operation = input(call, "operation")?;
    let descriptor = bootstrap.descriptor()?;
    let requirement = admit_operation(&adapter, &bootstrap, &operation)?;
    if requirement.effect != runtime::Effect::Read {
        return Err(Code::Unsupported.into());
    }
    validate_document(
        &descriptor
            .operation(&operation)
            .map_err(|_| Code::NotFound)?
            .input_schema,
        document,
        runtime::INPUT_LIMIT,
    )?;
    let connection = input(call, "connection")?;
    registry::Registry::with_system_clock(&paths.state).admit_read(
        &bootstrap.binding(&requirement.profile)?,
        &connection,
        &requirement.scopes,
        connectors_sdk::now_ms(),
    )?;
    if !custody::available_for(&config.selection()) {
        return Err(Code::CustodyUnavailable.into());
    }
    Ok(())
}
/// What an admitted consumer launch may use: the pinned consumer entry and the
/// binding its connection is read under. Never a reusable grant: the owner
/// repeats admission and re-reads the configuration around the credential read.
pub struct LaunchAdmission {
    pub consumer: super::config::Consumer,
    pub binding: registry::Binding,
}
/// Admission of a consumer launch, in order: the adapter and the consumer are
/// configured (`not_found`); the consumer lists the adapter (`forbidden`); its
/// pinned image matches its digest (`invalid_configuration`); the connection is
/// visible (`not_found`, `revoked`) and its profile granted (`forbidden`); its
/// evidence is current (`unavailable`); custody is available
/// (`custody_unavailable`). The binding comes from the saved connection record:
/// no adapter starts and no credential is read here.
pub fn admit_launch(
    paths: &Paths,
    alias: &str,
    connection: &str,
    consumer: &str,
) -> Result<LaunchAdmission> {
    let (config, adapter) = selected(paths, alias)?;
    if !connectors_core::valid_id(connection) || !connectors_core::valid_id(consumer) {
        return Err(Code::InvalidInput.into());
    }
    let entry = config
        .consumers
        .get(consumer)
        .cloned()
        .ok_or(Code::NotFound)?;
    if !entry.permissions.connections.contains(alias) {
        return Err(Code::Forbidden.into());
    }
    entry.executable.check()?;
    let registry = registry::Registry::with_system_clock(&paths.state);
    let recorded =
        registry.recorded_binding(&adapter.instance_id, &adapter.adapter_id, connection)?;
    if !adapter.permissions.profiles.contains(&recorded.profile.id) {
        return Err(Code::Forbidden.into());
    }
    // A connection saved under another configuration revision is a binding
    // conflict here, as for any read: `connections revalidate` moves it,
    // unless only its operation side moved (D12). A consumer launch starts no
    // provider, so the provider executable is not compared.
    let binding = registry::Binding {
        instance_id: adapter.instance_id.clone(),
        adapter_id: adapter.adapter_id.clone(),
        configuration_revision: adapter.configuration_revision.clone(),
        provider_authority: recorded.provider_authority,
        profile: recorded.profile,
        credential_revision: adapter.credential_revision.clone(),
        executable: None,
    };
    registry
        .admit_read(
            &binding,
            connection,
            &Default::default(),
            connectors_sdk::now_ms(),
        )
        .map_err(launch_failure)?;
    if !custody::available_for(&config.selection()) {
        return Err(Code::CustodyUnavailable.into());
    }
    Ok(LaunchAdmission {
        consumer: entry,
        binding,
    })
}
/// A connection whose provider executable alone changed since it was saved
/// (`registry::Failure::ExecutableChanged`, architecture-decision-record D12):
/// its stored credential is validated once under the configured executable,
/// which the registry follows when the recorded identity answers and refuses,
/// changing nothing, otherwise. No credential entry, no question. `admit`
/// rechecks the caller's own fences before dispatch; `validate` sends the
/// validation and answers its result, attempts and transient cause. Only a
/// change the capture itself classifies as executable-only is followed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn follow_executable<G>(
    paths: &Paths,
    selection: &super::config::Selection,
    registry: &registry::Registry,
    binding: &registry::Binding,
    connection: &str,
    deadline: u64,
    admit: impl FnOnce() -> Result<G>,
    validate: impl FnOnce(
        &connectors_sdk::Secret,
    ) -> (runtime::Result<runtime::Baseline>, u32, Option<Cause>),
) -> Result<()> {
    let revision =
        registry.recorded_revision(&binding.instance_id, &binding.adapter_id, connection)?;
    let now = connectors_sdk::now_ms();
    let captured = registry.capture_revalidation(
        binding,
        connection,
        &revision,
        now,
        deadline.min(now + 30_000),
    )?;
    if !captured.follows_executable() {
        registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
        return Err(registry::Failure::UpgradeRequired.into());
    }
    let version = captured.version();
    let material = custody::Store::open_for(version.scope(), selection, &paths.state)
        .and_then(|s| s.read(version));
    let material = match material {
        Ok(value) => value,
        Err(error) => {
            registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
            return Err(read_error(selection, error));
        }
    };
    // Whatever `admit` holds (the owner's control lock) is held until the
    // dispatch is recorded, as for a read.
    let held = match admit() {
        Ok(held) => held,
        Err(error) => {
            registry.cancel_revalidation(captured, connectors_sdk::now_ms())?;
            return Err(error);
        }
    };
    let dispatched = registry.dispatch_revalidation(captured, connectors_sdk::now_ms())?;
    drop(held);
    let (result, attempts, cause) = validate(&material);
    match result {
        Ok(baseline) => Ok(registry.finish_revalidation(
            dispatched,
            Ok(baseline.into_registry()),
            connectors_sdk::now_ms(),
        )?),
        Err(failure) => {
            registry.finish_revalidation(dispatched, Err(None), connectors_sdk::now_ms())?;
            let mut error = Error::from(failure);
            if cause.is_some() {
                error.cause = cause;
                error.attempts = Some(attempts);
            }
            Err(error)
        }
    }
}
/// Whether a failed custody read proves the version's bytes gone for good, so
/// the version is invalidated. Under the read-only binding `Missing` means the
/// mounted item changed or went away underneath: a revalidation re-pins it, so
/// it never invalidates.
pub(crate) fn read_invalidates(
    selection: &super::config::Selection,
    failure: custody::Failure,
) -> bool {
    failure == custody::Failure::Missing && !selection.read_only()
}
/// The owner's answer to a failed custody read: `custody_unavailable`, naming
/// a revalidation when the read-only binding's pinned bytes are no longer the
/// item's. The changed bytes are never used.
pub(crate) fn read_error(selection: &super::config::Selection, failure: custody::Failure) -> Error {
    let mut error: Error = Code::CustodyUnavailable.into();
    error.revalidate = failure == custody::Failure::Missing && selection.read_only();
    error
}
/// The read-only binding's connect, repair and re-pin, after the claim is
/// consumed: the owner reads the item `<alias>/<profile>` once under every
/// check, the provider validates exactly those bytes, and the binding pins
/// them to the prepared version, which is published. Nothing is written to the
/// mounted directory; a missing or refused item is `custody_unavailable`.
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_mounted(
    paths: &Paths,
    selection: &super::config::Selection,
    registry: &registry::Registry,
    claim: &registry::Claim,
    alias: &str,
    profile: &str,
    validate: impl FnOnce(connectors_sdk::Secret) -> Result<registry::ValidatedBaseline>,
    recheck: impl FnOnce() -> Result<()>,
) -> Result<String> {
    let item = custody::Item::new(alias, profile).map_err(|_| Code::InvalidInput)?;
    let reading = custody::read_mounted(selection, &paths.state, &item)
        .map_err(|_| Code::CustodyUnavailable)?;
    let baseline = validate(connectors_sdk::Secret(
        reading.material().as_bytes().to_vec(),
    ))?;
    recheck()?;
    let prepared = registry.prepare(
        claim,
        baseline,
        reading.material().len(),
        connectors_sdk::now_ms(),
    )?;
    let store = custody::Store::open_for(prepared.version().scope(), selection, &paths.state)
        .map_err(|_| Code::CustodyUnavailable)?;
    Ok(registry.pin_and_publish(prepared, &item, &reading, &store, connectors_sdk::now_ms)?)
}
/// Evidence that is not current is readiness, `unavailable`, as the
/// connection's own observation reports it; never a grant refusal.
fn launch_failure(error: registry::Failure) -> Error {
    match error {
        registry::Failure::NotReady | registry::Failure::EvidenceExpired => {
            Code::Unavailable.into()
        }
        error => error.into(),
    }
}
pub fn connection_value(alias: &str, value: registry::ObservedConnection) -> Value {
    json!({"summary":{"adapter":alias,"instance_id":value.instance,"connection":value.reference,"profile":value.profile,"revision":value.revision,"state":value.state},
        "external_identity":value.identity,"observed_at_ms":value.observed_at_ms,"valid_until_ms":value.valid_until_ms,"source":"authority","stale":value.stale})
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Hello {
        version: String,
        challenge: String,
        configuration: PathBuf,
        authority: String,
        /// SHA-256 of the caller's own executable. Absent only from callers built
        /// before the build handshake; an owner that predates it refuses the field.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        build: Option<String>,
    },
    Begin {
        adapter: String,
        profile: Option<String>,
        connection: Option<String>,
        expected_revision: Option<String>,
    },
    Complete,
    /// story:invoke-definition-reuse-credential: a new connection of `adapter`
    /// holding the current credential of `connection`, a connection of
    /// `source`, another instance of the same connector, offered under
    /// `expected_revision` (`connectors.cli.LocalOwnerReuseRequest`). Carries
    /// no document and no material.
    Reuse {
        adapter: String,
        source: String,
        connection: String,
        expected_revision: String,
        profile: String,
    },
    Revalidate {
        adapter: String,
        connection: String,
        expected_revision: String,
        deadline_ms: u64,
    },
    Invoke {
        adapter: String,
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        deadline_ms: u64,
    },
    Status {
        adapter: String,
    },
    /// A CLI's read of the adapter's cached description, answered from the
    /// owner's held metadata handle (`connectors.cli.LocalOwnerCachedRequest`).
    Cached {
        adapter: String,
        /// When nothing is cached for the configured selection, start the
        /// configured adapter to cache it, if the instance holds a connection
        /// (`connection`, when named) current under the configured entry
        /// (`registry::Registry::launch_admits`). Omitted when false.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        launch: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connection: Option<String>,
    },
    /// A consumer launch. Answered by `Reply::Launch`, then the consumer image
    /// and the sealed credential as descriptors, then the final answer.
    Launch {
        adapter: String,
        connection: String,
        consumer: String,
        deadline_ms: u64,
    },
    Stop {
        adapter: String,
        configuration_revision: String,
        host_incarnation: String,
        child_incarnation: String,
    },
    Shutdown {
        host_incarnation: String,
    },
    /// Asks for the owner's executable digest after a greeting without `build`.
    /// An owner from before the build handshake does not know this request and
    /// closes the stream without a reply.
    Build,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Hello {
        version: String,
        challenge: String,
        host_incarnation: String,
        authority: String,
        /// SHA-256 of the owner's own executable, returned only when asked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        build: Option<String>,
    },
    Capture {
        acquisition: String,
        expires_at_ms: u64,
        profile: runtime::Profile,
    },
    /// The admitted consumer's pinned argv prefix after argv\[0\], and the
    /// environment name prefixes its entry passes.
    Launch {
        args: Vec<String>,
        pass_env: std::collections::BTreeSet<String>,
    },
    Success,
    Failed {
        error: Error,
    },
}

#[cfg(test)]
mod cached_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn bootstrap(instance: &str) -> runtime::Bootstrap {
        runtime::Bootstrap {
            credential_revision: None,
            executable: None,
            instance: instance.into(),
            adapter: "fixture-adapter".into(),
            protocol: "v1alpha1".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.invalid".into(),
            descriptor: "{}".into(),
            profiles: Vec::new(),
            requirements: Vec::new(),
        }
    }

    /// A configured adapter whose cached description was never recorded: the
    /// direct read refuses it as `description_unavailable`.
    fn configured() -> (tempfile::TempDir, Paths) {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        config.format = "connectors-local/2".into();
        config.adapters.insert(
            "fixture".into(),
            super::super::config::Adapter {
                credential_revision: None,
                instance_id: "fixture-instance".into(),
                adapter_id: "fixture-adapter".into(),
                configuration_revision: "cfg-1".into(),
                protocol: "v1alpha1".into(),
                private_protocol: Some(runtime::PrivateProtocol::V2),
                startup: Default::default(),
                restart: Default::default(),
                executable: super::super::config::Executable {
                    path: root.path().join("absent-adapter"),
                    sha256: "a".repeat(64),
                    args: vec![],
                },
                permissions: super::super::config::Permissions {
                    profiles: BTreeSet::new(),
                    operations: BTreeSet::new(),
                    custody: Default::default(),
                },
            },
        );
        std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
        (root, paths)
    }

    #[test]
    fn a_failed_cached_exchange_falls_back_to_the_direct_read() {
        let (_root, paths) = configured();
        let direct = cached_direct(&paths, "fixture")
            .map(|_| ())
            .map_err(|e| e.code);
        assert_eq!(direct, Err(Code::DescriptionUnavailable));
        // An exchange that broke after the greeting, or a refusal by the owner,
        // is answered by the direct read and its own code.
        for failure in [Code::Unavailable, Code::Timeout, Code::OwnerBuildMismatch] {
            let answered = cached_through(&paths, "fixture", |_, _| Some(Err(failure.into())));
            assert_eq!(
                answered.map(|_| ()).map_err(|e| e.code),
                direct,
                "{failure:?}"
            );
        }
        // No owner to ask: the direct read.
        let answered = cached_through(&paths, "fixture", |_, _| None);
        assert_eq!(answered.map(|_| ()).map_err(|e| e.code), direct);
        // The owner's answer is taken as it is.
        let answered = cached_through(&paths, "fixture", |_, _| Some(Ok(bootstrap("owned"))));
        assert_eq!(
            answered.map(|b| b.instance).map_err(|e| e.code),
            Ok("owned".into())
        );
    }

    /// story:revision-change-keeps-a-recovery-path: a saved connection whose
    /// profile the configured adapter no longer declares (its `auth.profile`
    /// id changed) is a changed binding that a new connection follows, never
    /// `unsupported` with no next action.
    #[test]
    fn a_profile_the_configuration_no_longer_declares_names_a_new_connection() {
        let error = saved_binding(&bootstrap("fixture-instance"), "gitlab.pat").unwrap_err();
        assert_eq!(error.code, Code::LifecycleConflict);
        assert!(error.reconnect);
        assert!(!error.reconfigure);
        assert!(!error.revalidate);
    }
}

/// story:invoke-definition-reuse-credential: a reuse names another instance of
/// the same connector id; a legacy `[adapters]` entry names no connector, and
/// an instance is never its own source.
#[cfg(test)]
mod reuse_tests {
    use super::*;
    use crate::local::config::{Connector, Instance};
    use std::collections::BTreeMap;

    fn entry(instance: &str) -> Adapter {
        Adapter {
            credential_revision: None,
            instance_id: instance.into(),
            adapter_id: "catalog".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: super::super::config::Executable {
                path: PathBuf::from("absent-adapter"),
                sha256: "a".repeat(64),
                args: vec![],
            },
            permissions: Default::default(),
        }
    }

    fn instance(alias: &str) -> Instance {
        Instance {
            description: None,
            endpoint: None,
            alias: alias.into(),
            derived: false,
            profile: None,
            awaiting_profile: None,
        }
    }

    fn config() -> Config {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        for alias in [
            "forge.main",
            "forge.other",
            "gitlab.main",
            "forge",
            "forge.definition",
        ] {
            config.adapters.insert(alias.into(), entry(alias));
        }
        config.connectors = Some(BTreeMap::from([
            (
                "forge".to_owned(),
                Connector {
                    enabled: true,
                    instances: BTreeMap::from([
                        ("main".to_owned(), instance("forge.main")),
                        ("other".to_owned(), instance("forge.other")),
                    ]),
                },
            ),
            (
                "gitlab".to_owned(),
                Connector {
                    enabled: true,
                    instances: BTreeMap::from([("main".to_owned(), instance("gitlab.main"))]),
                },
            ),
        ]));
        config
            .definitions
            .insert("forge.definition".into(), "forge".into());
        config
    }

    #[test]
    fn only_another_instance_of_the_same_connector_is_a_source() {
        let config = config();
        assert!(same_connector(&config, "forge.definition", "forge.main"));
        assert!(same_connector(&config, "forge.definition", "forge.other"));
        assert!(same_connector(&config, "forge.main", "forge.other"));
        assert!(!same_connector(&config, "forge.definition", "gitlab.main"));
        assert!(!same_connector(
            &config,
            "forge.definition",
            "forge.definition"
        ));
        // A legacy entry: its alias is no connector, whatever it spells.
        assert_eq!(config.legacy_aliases(), ["forge"]);
        assert!(!same_connector(&config, "forge.definition", "forge"));
        assert!(!same_connector(&config, "forge", "forge.main"));
        assert!(!same_connector(&config, "forge.definition", "absent"));
    }
}
