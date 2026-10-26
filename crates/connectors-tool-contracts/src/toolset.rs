//! A published contract served over an adapter's `invoke`, as a `tool-conformance/1` toolset.

use crate::{FAILED, FIELD_NOT_ALLOWED, Publication, code_text, schema};
use connectors_core::Descriptor;
use connectors_sdk::Adapter;
use schemars::Schema;
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tool_contract::{Contract, Toolset, conformance::Outcome};

/// One instance's published contract over its adapter.
///
/// `invoke("connector-<instance>.<op>", input)` refuses an input that carries a field the
/// connection fills ([`FIELD_NOT_ALLOWED`]), adds the values the host gave for those fields, calls
/// the adapter's `invoke` with the descriptor operation id and answers its output, or
/// [`FAILED`] with the error's `code`. The adapter holds the instance's connection and credential;
/// this type never sees either.
pub struct AdapterToolset {
    publication: Publication,
    descriptor: Descriptor,
    adapter: Arc<dyn Adapter>,
    filled: BTreeMap<String, Map<String, Value>>,
    runtime: tokio::runtime::Runtime,
}

impl AdapterToolset {
    /// Serves `publication`, published from `descriptor`, over `adapter`.
    ///
    /// # Errors
    ///
    /// The runtime the adapter's futures run on could not be built.
    pub fn new(
        publication: Publication,
        descriptor: Descriptor,
        adapter: Arc<dyn Adapter>,
    ) -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            publication,
            descriptor,
            adapter,
            filled: BTreeMap::new(),
            runtime,
        })
    }

    /// The value the connection gives `field` of the published operation `name`; it is sent with
    /// every call of that operation and refused in a caller's input.
    #[must_use]
    pub fn fill(mut self, name: &str, field: &str, value: Value) -> Self {
        self.filled
            .entry(name.to_owned())
            .or_default()
            .insert(field.to_owned(), value);
        self
    }

    fn failed(code: &connectors_core::ErrorCode, message: &str) -> Outcome {
        Outcome::Error {
            name: FAILED.into(),
            message: message.to_owned(),
            fields: json!({"code": code_text(code)}),
        }
    }
}

impl Toolset for AdapterToolset {
    fn contract(&self) -> &Contract {
        &self.publication.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        self.publication
            .operations
            .iter()
            .filter_map(|published| {
                let operation = self
                    .descriptor
                    .operations
                    .iter()
                    .find(|o| o.id == published.operation)?;
                let input =
                    Schema::try_from(schema::normalized(&operation.input_schema, &published.omit))
                        .ok()?;
                let output =
                    Schema::try_from(schema::normalized(&operation.output_schema, &[])).ok()?;
                Some((published.name.clone(), input, output))
            })
            .collect()
    }

    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        let Some(published) = self
            .publication
            .operations
            .iter()
            .find(|p| p.name == operation)
        else {
            return Self::failed(
                &connectors_core::ErrorCode::NotFound,
                "the contract does not publish this operation",
            );
        };
        let Some(given) = input.as_object() else {
            return Self::failed(
                &connectors_core::ErrorCode::InvalidInput,
                "the input is not an object",
            );
        };
        if let Some(field) = published.omit.iter().find(|f| given.contains_key(*f)) {
            return Outcome::Error {
                name: FIELD_NOT_ALLOWED.into(),
                message: format!("{field} is filled by the connection"),
                fields: json!({"field": field}),
            };
        }
        let mut sent = given.clone();
        if let Some(values) = self.filled.get(&published.name) {
            for (field, value) in values {
                sent.insert(field.clone(), value.clone());
            }
        }
        match self.runtime.block_on(
            self.adapter
                .invoke(&published.operation, Value::Object(sent)),
        ) {
            Ok(output) => Outcome::Ok(output),
            Err(error) => Self::failed(&error.code, &error.message),
        }
    }
}
