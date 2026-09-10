use connectors_cli_contract::Invocation;
use connectors_host::local::{
    approval_keys::{Failure, Store},
    config::{Adapter, Config, Paths},
    owner::{Code, Error},
};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn execute(
    call: &Invocation<'_>,
    paths: &Paths,
    config: &Config,
    alias: &str,
    adapter: &Adapter,
) -> Result<Value, Error> {
    let store = Store::new(
        &paths.state,
        config.secret_service_socket.as_deref(),
        &adapter.instance_id,
        &adapter.adapter_id,
        &adapter.configuration_revision,
    )
    .map_err(error)?;
    let optional_id = |field: &str| -> Result<Option<Uuid>, Error> {
        call.input
            .get(field)
            .filter(|v| !v.is_null())
            .map(|v| {
                let text = v.as_str().ok_or(Code::InvalidInput)?;
                let id = Uuid::parse_str(text).map_err(|_| Code::InvalidInput)?;
                if id.is_nil() || id.to_string() != text {
                    return Err(Code::InvalidInput.into());
                }
                Ok(id)
            })
            .transpose()
    };
    let required_id = |field: &str| -> Result<Uuid, Error> {
        optional_id(field)?.ok_or_else(|| Code::InvalidInput.into())
    };
    let view = match call.callable {
        "approval-keys-status" => store.status().map_err(error)?,
        "approval-keys-init" => Some(
            store
                .init(optional_id("expected_revision")?)
                .map_err(error)?,
        ),
        "approval-keys-rotate" => Some(
            store
                .rotate(
                    required_id("expected_revision")?,
                    required_id("expected_key")?,
                )
                .map_err(error)?,
        ),
        "approval-keys-recover" => Some(
            store
                .recover(required_id("expected_revision")?, required_id("candidate")?)
                .map_err(error)?,
        ),
        "approval-keys-revoke" => Some(
            store
                .revoke(
                    required_id("expected_revision")?,
                    required_id("expected_key")?,
                )
                .map_err(error)?,
        ),
        "approval-keys-retire" => Some(
            store
                .retire(required_id("expected_revision")?, required_id("key")?)
                .map_err(error)?,
        ),
        _ => return Err(Code::Unsupported.into()),
    };
    let mut result = json!({"adapter":alias});
    if let Some(view) = view {
        result["issuer"] = json!(view);
    }
    Ok(result)
}
fn error(failure: Failure) -> Error {
    match failure {
        Failure::InvalidInput => Code::InvalidInput,
        Failure::Conflict => Code::LifecycleConflict,
        Failure::NotFound => Code::NotFound,
        Failure::MetadataUnavailable => Code::MetadataUnavailable,
        Failure::CustodyUnavailable => Code::CustodyUnavailable,
        Failure::OutcomeUnknown => Code::OutcomeUnknown,
        Failure::Capacity => Code::Capacity,
    }
    .into()
}
