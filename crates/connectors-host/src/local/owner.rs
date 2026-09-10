//! Owner-side composition for the production local CLI. Provider libraries never
//! receive this authority, the metadata database, or a credential resolver.
mod lifecycle;
mod supervisor;
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
pub use transport::{Capture, Client, serve};

const VERSION: &str = "connectors-owner/1";
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    InvalidInput,
    ProtectedEntryUnavailable,
    InvalidConfiguration,
    MetadataUnavailable,
    CustodyUnavailable,
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
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: Code,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_code: Option<connectors_core::ErrorCode>,
}
impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Self {
            code,
            acquisition: None,
            service_code: None,
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
            F::Forbidden => Code::Forbidden,
            F::NotFound => Code::NotFound,
            F::Unavailable => Code::Unavailable,
            F::Timeout => Code::Timeout,
            F::Interrupted => Code::Interrupted,
            F::Capacity => Code::Capacity,
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
        Self {
            code,
            acquisition: None,
            service_code,
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
    runtime::state::State::new(&paths.state)
        .cached(&adapter.instance_id, &adapter.selection())?
        .ok_or_else(|| Code::DescriptionUnavailable.into())
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
pub fn operation_snapshot(paths: &Paths, alias: &str, call: &Value) -> Result<runtime::Bootstrap> {
    let bootstrap = cached(paths, alias)?;
    let operation = input(call, "operation")?;
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
    let profile = admit_capture(paths, alias, None, Some(connection), Some(revision))?;
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
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Code::NotFound)?;
    if !adapter.permissions.operations.contains(&operation)
        || !adapter.permissions.profiles.contains(&requirement.profile)
    {
        return Err(Code::Forbidden.into());
    }
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
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Hello {
        version: String,
        challenge: String,
        host_incarnation: String,
        authority: String,
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
