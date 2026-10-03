//! Protected receiver declarations, pinned to an exact selected adapter/cache.
use super::{
    config::{Adapter, Paths},
    filesystem,
    owner::{Code, Result},
    runtime,
};
use connectors_core::operation_metadata::{CURATION_LIMIT, Curation, Metadata};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub struct Snapshot {
    pub source_sha256: String,
    pub operations: BTreeMap<String, Metadata>,
}

pub fn path(paths: &Paths) -> PathBuf {
    let mut path = paths.config.as_os_str().to_owned();
    path.push(".operation-curation.json");
    path.into()
}

pub fn load(
    paths: &Paths,
    alias: &str,
    adapter: &Adapter,
    bootstrap: &runtime::Bootstrap,
) -> Result<Snapshot> {
    bootstrap.validate_for(adapter.private_protocol())?;
    if bootstrap.instance != adapter.instance_id
        || bootstrap.adapter != adapter.adapter_id
        || bootstrap.configuration_revision != adapter.configuration_revision
        || bootstrap.protocol != adapter.protocol
    {
        return Err(Code::ReadinessMismatch.into());
    }
    let bytes = filesystem::read_bounded(filesystem::private_file(&path(paths))?, CURATION_LIMIT)?;
    let curation = Curation::parse(&bytes).map_err(|_| Code::InvalidConfiguration)?;
    let source_sha256 = connectors_core::digest(
        &serde_json::to_value(curation.document()).map_err(|_| Code::Unavailable)?,
    );
    let Some(selected) = curation
        .document()
        .adapters
        .iter()
        .find(|entry| entry.adapter_alias == alias)
    else {
        return Ok(Snapshot {
            source_sha256,
            operations: BTreeMap::new(),
        });
    };
    if selected.executable_selection != adapter.selection()
        || selected.bootstrap_sha256
            != connectors_core::digest(
                &serde_json::to_value(bootstrap).map_err(|_| Code::Unavailable)?,
            )
    {
        return Err(Code::StaleDescription.into());
    }
    let mut operations = BTreeMap::new();
    for entry in &selected.operations {
        let value =
            serde_json::to_value(&entry.metadata).map_err(|_| Code::InvalidConfiguration)?;
        let metadata = bind(bootstrap, &entry.operation, &value)?;
        operations.insert(entry.operation.clone(), metadata);
    }
    Ok(Snapshot {
        source_sha256,
        operations,
    })
}

/// Checks declaration consistency, not native implementation support or grants.
pub(crate) fn bind(
    bootstrap: &runtime::Bootstrap,
    operation: &str,
    value: &Value,
) -> Result<Metadata> {
    let descriptor = bootstrap.descriptor()?;
    let declaration = descriptor
        .operation(operation)
        .map_err(|_| Code::InvalidConfiguration)?;
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|entry| entry.operation == operation)
        .ok_or(Code::InvalidConfiguration)?;
    let metadata = Metadata::parse(
        &serde_json::to_vec(value).map_err(|_| Code::InvalidConfiguration)?,
        &declaration.profile,
        CURATION_LIMIT,
    )
    .map_err(|_| Code::InvalidConfiguration)?;
    let write = value["effects"]
        .as_array()
        .ok_or(Code::InvalidConfiguration)?
        .iter()
        .any(|effect| effect == "external_write");
    if requirement.effect == runtime::Effect::Unknown
        || write != (requirement.effect == runtime::Effect::Write)
    {
        return Err(Code::InvalidConfiguration.into());
    }
    if let Some(alternatives) = value.get("requires_auth") {
        let alternatives = alternatives.as_array().ok_or(Code::InvalidConfiguration)?;
        if alternatives.len() != 1 || alternatives[0]["profile"] != requirement.profile {
            return Err(Code::InvalidConfiguration.into());
        }
        let scopes = alternatives[0]["scopes"]
            .as_array()
            .ok_or(Code::InvalidConfiguration)?
            .iter()
            .map(|scope| scope.as_str().ok_or(Code::InvalidConfiguration))
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        if scopes != requirement.scopes.iter().map(String::as_str).collect() {
            return Err(Code::InvalidConfiguration.into());
        }
    }
    Ok(metadata)
}
