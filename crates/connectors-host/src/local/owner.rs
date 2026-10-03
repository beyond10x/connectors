//! Owner-side composition for the production local CLI. Provider libraries never
//! receive this authority, the metadata database, or a credential resolver.
pub mod approval_issuance;
mod governed;
mod lifecycle;
mod maintenance;
pub mod mutation;
mod supervisor;
mod transport;
use super::{
    config::{Adapter, Config, Paths},
    keyring::custody,
    registry, runtime,
};
pub use governed::{ProjectionMetadata, ReadPolicy};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
pub use transport::{Capture, Client, WriteClient, serve, serve_with_read_policy};

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
    OutcomeUnknown,
    Forbidden,
    NotFound,
    LifecycleConflict,
    Revoked,
    NotGranted,
    ConnectionNotReady,
    InsufficientScope,
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
    #[serde(default, skip_serializing_if = "Origin::is_host")]
    pub origin: Origin,
}
impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Self {
            code,
            acquisition: None,
            service_code: None,
            origin: Origin::Host,
        }
    }
}
impl From<super::Failure> for Error {
    fn from(e: super::Failure) -> Self {
        match e {
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
        match e {
            F::MetadataUnavailable => Code::MetadataUnavailable,
            F::ConcurrentRevision => Code::RevisionConflict,
            F::OutcomeUnknown => Code::OutcomeUnknown,
            F::NotFound => Code::NotFound,
            F::Conflict => Code::LifecycleConflict,
            F::Revoked => Code::Revoked,
            F::IdentityMismatch => Code::IdentityMismatch,
            F::Expired => Code::Timeout,
            F::InvalidInput => Code::InvalidInput,
            F::StaleCursor => Code::StaleCursor,
            F::Capacity => Code::Capacity,
            F::NotReady | F::InsufficientScope => Code::NotGranted,
            F::CustodyUnavailable => Code::CustodyUnavailable,
        }
        .into()
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
            origin,
        }
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
    let config = Config::load(&paths.config)?;
    let adapter = config.adapters.get(alias).cloned().ok_or(Code::NotFound)?;
    Ok((config, adapter))
}
pub fn cached(paths: &Paths, alias: &str) -> Result<runtime::Bootstrap> {
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
                &adapter.configuration_revision,
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
    if !custody::available_at(config.secret_service_socket.as_deref()) {
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
    let bootstrap = cached(paths, alias)?;
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
    let profile = registry::Registry::new(&paths.state).revalidation_profile(
        &adapter.instance_id,
        &adapter.adapter_id,
        connection,
        revision,
    )?;
    if !adapter.permissions.profiles.contains(&profile) {
        return Err(Code::Forbidden.into());
    }
    if !custody::available_at(config.secret_service_socket.as_deref()) {
        return Err(Code::CustodyUnavailable.into());
    }
    let binding = cached(paths, alias)?.binding(&profile)?;
    registry::Registry::with_system_clock(&paths.state).admit_revalidation(
        &binding,
        connection,
        revision,
        connectors_sdk::now_ms(),
    )?;
    Ok(profile)
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
    if !custody::available_at(config.secret_service_socket.as_deref()) {
        return Err(Code::CustodyUnavailable.into());
    }
    Ok(())
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
    GovernedRead {
        adapter: String,
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        deadline_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        projection_revision: Option<String>,
    },
    BoundedRead {
        adapter: String,
        connection: String,
        operation: String,
        schema: String,
        revision: String,
        projection_revision: String,
        budget: runtime::ReadBudget,
    },
    GovernedDescribe {
        adapter: String,
        deadline_ms: u64,
    },
    ProjectedDescribe {
        adapter: String,
        deadline_ms: u64,
    },
    Status {
        adapter: String,
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
    Success,
    Failed {
        error: Error,
    },
}
