//! Connectors owns the adapter specification kind; it is not an ESS builtin.
use connectors_core::{Descriptor, Error, Operation, Result, WIRE_VERSION, digest, valid_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub mod toolchain;
pub mod v2;
pub mod v3;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterSpec {
    pub kind: String,
    pub id: String,
    pub version: String,
    pub sources: Vec<String>,
    pub configuration_schema: Value,
    pub operations: Vec<Operation>,
}

pub fn compile(bytes: &[u8]) -> Result<Descriptor> {
    if bytes.len() > 1024 * 1024 {
        return Err(Error::invalid("adapter specification exceeds byte limit"));
    }
    let value: Value = connectors_core::read_json(bytes)?;
    if value["kind"] == "connectors.adapter/v2" {
        return v2::Spec::parse(bytes)?.descriptor();
    }
    if value["kind"] == "connectors.adapter/v3" {
        return v3::Spec::parse(bytes)?.descriptor();
    }
    let schema: Value =
        serde_json::from_str(include_str!("../../../spec-kinds/adapter/v1/schema.json"))
            .map_err(|_| Error::internal())?;
    if !jsonschema::validator_for(&schema)
        .map_err(|_| Error::internal())?
        .is_valid(&value)
    {
        return Err(Error::invalid(
            "adapter document does not match connectors.adapter/v1",
        ));
    }
    let mut spec: AdapterSpec = serde_json::from_value(value)
        .map_err(|_| Error::invalid("invalid adapter specification"))?;
    if spec.kind != "connectors.adapter/v1"
        || !valid_id(&spec.id)
        || spec.version.is_empty()
        || spec.sources.is_empty()
        || spec.sources.iter().any(|s| !s.starts_with("https://"))
        || spec.operations.is_empty()
    {
        return Err(Error::invalid(
            "invalid adapter identity, kind, version, provenance or empty operation surface",
        ));
    }
    let mut ids = BTreeSet::new();
    for operation in &spec.operations {
        if !valid_id(&operation.id)
            || !ids.insert(&operation.id)
            || operation.description.is_empty()
            || operation.contract.is_empty()
            || operation.profile.is_empty()
        {
            return Err(Error::invalid("invalid or duplicate operation declaration"));
        }
        for schema in [&operation.input_schema, &operation.output_schema] {
            jsonschema::validator_for(schema).map_err(|_| {
                Error::invalid("invalid operation schema or unsupported external reference")
            })?;
        }
    }
    connectors_host::schema::expand(&mut spec.configuration_schema)?;
    jsonschema::validator_for(&spec.configuration_schema)
        .map_err(|_| Error::invalid("invalid configuration schema"))?;
    let revision = digest(&serde_json::to_value(&spec).map_err(|_| Error::internal())?);
    Ok(Descriptor {
        version: WIRE_VERSION.into(),
        instance: spec.id.clone(),
        adapter: spec.id,
        revision,
        operations: spec.operations,
        configuration_schema: spec.configuration_schema,
    })
}

pub fn descriptor_bytes(descriptor: &Descriptor) -> Result<Vec<u8>> {
    let value = serde_json::to_value(descriptor).map_err(|_| Error::internal())?;
    let mut bytes = connectors_core::canonical(&value);
    bytes.push(b'\n');
    Ok(bytes)
}

/// Generate only explicitly selected supported bundle formats. Legacy entry
/// points remain closed to newer formats.
pub fn generate(
    specification: &std::path::Path,
    out: &std::path::Path,
    ess: &std::path::Path,
    check: bool,
) -> Result<()> {
    let bytes =
        std::fs::read(specification).map_err(|_| Error::invalid("cannot read specification"))?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::invalid("specification exceeds byte limit"));
    }
    let value: Value = connectors_core::read_json(&bytes)?;
    match value["kind"].as_str() {
        Some("connectors.adapter/v2") => v2::generate(specification, out, ess, check),
        Some("connectors.adapter/v3") => v3::generate(specification, out, ess, check),
        _ => Err(Error::invalid("unsupported generated bundle specification")),
    }
}
