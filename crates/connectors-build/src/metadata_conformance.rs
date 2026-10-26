//! Runs synthesized ESS conformance suites against the local metadata authority.
//!
//! The implementation under test is the host's committed Entity Runtime definitions
//! (`crates/connectors-host/src/local/metadata/entity-runtime-definitions.json`), registered and
//! decided by `entity-core` exactly as the host's `registry()` registers them. The target is a
//! thin binding: it turns a scenario's command input into the `{input, bound}` arguments the
//! lowering declared, loads the subject, and reports the kernel's outcome, refusal and events. It
//! makes no decision of its own; every branch, guard and state check is the kernel's.
//!
//! Values the specification leaves to the host (`undetermined` and `generated` binding slots, and
//! required operation-field fulfillments) are supplied as schema-valid placeholders, because the
//! specification says nothing about them and any valid value conforms.
//!
//! `emit` is `ess verify conform mutate --emit --component local-metadata-authority`: it writes
//! the emission the pinned ESS's own emitter (`mutate::emit_for`) produces, so the manifest is in
//! the format ESS chose and admits. Each suite holds only the scenarios this component realises;
//! a mutant another component answers for is recorded out of scope, with no suite, because no
//! implementation in this repository answers for it. `run-emission` runs every suite against the
//! committed definitions, and `ess verify conform mutate --collect` scores the result.
use super::Result;
use crate::metadata_entities;
use entity_core::{
    EntityInstance, Evaluation, FieldDefinition, FieldKind, LoadedDecision, OperationFieldAction,
    PreloadDecision, Registry, Runtime,
};
use ess_conformance::{
    AdmittedSuite, CountReport, CountRun, CountStatus, Runner,
    mutate::{self, EmittedMutant, EmittedSuite, Manifest, MutantClass, RefusalKey},
    scenario::{CommandRef, ErrorRef, EventRef, OutcomeRef},
    target::*,
};
use ess_domain::{command::OutcomeName, name::QualifiedName};
use ess_entity_runtime::{BoundPresence, BoundSource, CommandBinding, InstanceBinding};
use ess_primitives::node::Node;
use serde_json::{Map, Value, json};
use std::{cell::RefCell, collections::BTreeMap, path::Path};

const DEFINITIONS: &[u8] =
    include_bytes!("../../connectors-host/src/local/metadata/entity-runtime-definitions.json");

#[derive(serde::Deserialize)]
struct Bundle {
    format: String,
    definitions: Vec<BundleEntry>,
}

#[derive(serde::Deserialize)]
struct BundleEntry {
    name: String,
    definition: entity_core::EntityDefinition,
}

/// The host's registry, built from the committed definitions the way the host builds it.
fn registry() -> Result<Registry> {
    let bundle: Bundle = serde_json::from_slice(DEFINITIONS)?;
    if bundle.format != "connectors.entity-runtime-definitions/1" {
        return Err("unexpected definition bundle format".into());
    }
    let mut registry = Registry::new();
    for entry in bundle.definitions {
        if entry.name != entry.definition.entity {
            return Err(format!("definition {} is filed under another name", entry.name).into());
        }
        registry
            .register(entry.definition)
            .map_err(|error| format!("{}: {error:?}", entry.name))?;
    }
    registry
        .validate_all()
        .map_err(|error| format!("registry: {error:?}"))?;
    Ok(registry)
}

fn digest(bytes: &[u8]) -> String {
    connectors_spec::v2::hash(bytes)
}

struct View {
    source: String,
    /// Each field's name and its IR type reference.
    fields: Vec<(String, Value)>,
}

/// What does not change between scenarios: the registry, the bindings and the views.
pub struct Authority {
    registry: Registry,
    bindings: BTreeMap<String, CommandBinding>,
    views: BTreeMap<String, View>,
    /// The IR's declared types, by name.
    types: Map<String, Value>,
    identity: String,
}

/// The specification form of a stored value: a member whose declared type is a named optional
/// (`newtype of Optional<…>`) is required and null when absent, as the host reads it back
/// (`er.rs` `approval_subject_nulls` with `restore`). A plain `Optional` member stays absent.
fn restore_nulls(types: &Map<String, Value>, type_ref: &Value, value: &mut Value) {
    if value.is_null() {
        return;
    }
    match type_ref["kind"].as_str() {
        Some("optional") => restore_nulls(types, &type_ref["of"], value),
        Some("list") => {
            if let Value::Array(items) = value {
                for item in items {
                    restore_nulls(types, &type_ref["of"], item);
                }
            }
        }
        Some("declared") => {
            let Some(body) = type_ref["name"]
                .as_str()
                .and_then(|name| types.get(name))
                .map(|declared| &declared["body"])
            else {
                return;
            };
            match body["kind"].as_str() {
                Some("newtype") => restore_nulls(types, &body["of"], value),
                Some("struct") => {
                    let Value::Object(members) = value else {
                        return;
                    };
                    for field in body["fields"].as_array().into_iter().flatten() {
                        let (Some(name), field_type) = (field["name"].as_str(), &field["type_ref"])
                        else {
                            continue;
                        };
                        match members.get_mut(name) {
                            Some(member) => restore_nulls(types, field_type, member),
                            None if named_optional(types, field_type) => {
                                members.insert(name.to_owned(), Value::Null);
                            }
                            None => {}
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn named_optional(types: &Map<String, Value>, type_ref: &Value) -> bool {
    type_ref["kind"] == "declared"
        && type_ref["name"]
            .as_str()
            .and_then(|name| types.get(name))
            .is_some_and(|declared| {
                declared["body"]["kind"] == "newtype"
                    && declared["body"]["of"]["kind"] == "optional"
            })
}

impl Authority {
    /// Loads the implementation: the committed definitions, and the binding plan and views of
    /// the unchanged specification under `root/ess`. Refuses when the committed definitions have
    /// drifted from what that specification lowers to.
    pub fn load(root: &Path) -> Result<Self> {
        metadata_entities::run(root, true)?;
        let ir = metadata_entities::compile(&metadata_entities::load(root)?)?;
        let lowered = metadata_entities::lower_component(&ir)?;
        let bindings = lowered
            .bindings()
            .commands()
            .iter()
            .map(|(name, binding)| (name.to_string(), binding.clone()))
            .collect();
        let ir_json: Value = serde_json::from_str(&ir.to_canonical_json())?;
        let mut views = BTreeMap::new();
        for (name, view) in ir_json["views"].as_object().ok_or("IR has no views")? {
            let source = view["source"].as_str().unwrap_or_default().to_owned();
            let fields = view["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|field| {
                    field["name"]
                        .as_str()
                        .map(|name| (name.to_owned(), field["type_ref"].clone()))
                })
                .collect();
            views.insert(name.clone(), View { source, fields });
        }
        let types = ir_json["types"]
            .as_object()
            .cloned()
            .ok_or("IR has no types")?;
        Ok(Self {
            registry: registry()?,
            bindings,
            views,
            types,
            identity: format!("sha256:{}", digest(DEFINITIONS)),
        })
    }
}

#[derive(Default)]
struct Scenario {
    instances: BTreeMap<(String, String), EntityInstance>,
    events: Vec<ObservedEvent>,
    generated: u64,
    writes: u64,
}

pub struct Target<'a> {
    authority: &'a Authority,
    scenario: RefCell<Option<Scenario>>,
}

impl<'a> Target<'a> {
    pub fn new(authority: &'a Authority) -> Self {
        Self {
            authority,
            scenario: RefCell::new(None),
        }
    }
}

fn unavailable(what: impl Into<String>, why: impl std::fmt::Display) -> TargetError {
    TargetError::unsupported(what, why.to_string())
}

/// What a kernel error during `command` means for the scenario.
///
/// Every command input is a declared fixture or a schema-valid value, so an invariant the kernel
/// finds violated or cannot observe is the implementation's own defect: the resulting state was discarded, no
/// declared outcome was reached and nothing was written, which fails the scenario that expected
/// one. Any other kernel error is something this target cannot express and stays unsupported.
fn kernel_refusal(
    command: &str,
    error: entity_core::CoreError,
) -> std::result::Result<SemanticCommandResult, TargetError> {
    match error {
        entity_core::CoreError::InvariantViolation { .. }
        | entity_core::CoreError::InvariantUnobservable { .. } => {
            Ok(SemanticCommandResult::undeclared())
        }
        other => Err(unavailable(command, format!("kernel: {other}"))),
    }
}

fn name(value: &str) -> std::result::Result<QualifiedName, TargetError> {
    QualifiedName::new(value).map_err(|error| unavailable(value, format!("{error:?}")))
}

fn to_json(node: &Node) -> std::result::Result<Value, TargetError> {
    let value = serde_json::to_value(node).map_err(|error| unavailable("input value", error))?;
    Ok(integral(value))
}

/// A whole binary64 is the integer the suite wrote: `1.0` in a suite is the Integer `1`.
fn integral(value: Value) -> Value {
    match value {
        Value::Number(number) if number.as_i64().is_none() && number.as_u64().is_none() => {
            match number.as_f64() {
                Some(float) if float.fract() == 0.0 && float.abs() <= 9_007_199_254_740_992.0 => {
                    json!(float as i64)
                }
                _ => Value::Number(number),
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(integral).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, integral(v))).collect())
        }
        other => other,
    }
}

/// The Entity Runtime form of a value: a required member whose value is null is an absent
/// member, as the host writes it (`er.rs` `approval_subject_nulls`).
fn absent_nulls(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(absent_nulls).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, absent_nulls(v)))
                .collect(),
        ),
        other => other,
    }
}

fn to_node(value: &Value) -> std::result::Result<Node, TargetError> {
    serde_json::from_value(value.clone()).map_err(|error| unavailable("output value", error))
}

fn to_nodes(map: &Map<String, Value>) -> std::result::Result<BTreeMap<String, Node>, TargetError> {
    map.iter()
        .map(|(key, value)| Ok((key.clone(), to_node(value)?)))
        .collect()
}

/// The value of one fixture a command's `fixture_inputs` names. Each satisfies every invariant of
/// the entity its command writes, and is the value the host's own tests use: the fingerprint of
/// `er.rs` `prepared_attempt`, the direct namespace of `mutations/tests.rs` `candidate`, the direct
/// subject of `er.rs` `direct_subject`, and the admitted invocation of `audit/tests.rs` `anchor`.
/// A name not listed here is an error, never a default.
fn fixture(name: &str) -> Option<Value> {
    Some(match name {
        "canonical-request-fingerprint" => json!({
            "operation_ref": "[\"connectors.operation/v1\",\"instance\",\"adapter\",\"write\"]",
            "connection_ref": "connection",
            "connection_revision": "revision",
            "contract_ref": "contract",
            "profile": "profile",
            "descriptor_revision": "descriptor",
            "configuration_revision": "configuration",
            "canonicalization_version": "adapter-v1-canonical-json",
            "input_digest": "0000000000000000000000000000000000000000000000000000000000000000",
        }),
        "direct-key-namespace" => json!({
            "receiver_instance": "instance",
            "authority": {"caller": "caller"},
            "origin": {"kind": "direct", "authority_ref": "instance"},
        }),
        "direct-approval-subject" => json!({
            "format": "connectors.approval-subject/v1",
            "target": {
                "instance": "instance",
                "operation": "write",
                "connection": "connection",
                "connection_revision": "revision",
                "contract": "contract",
                "profile": "profile",
                "descriptor_revision": "descriptor",
                "configuration_revision": "configuration",
            },
            "authority": {
                "scope": {"tenant": null, "realm": null, "caller": "principal", "executor": null},
                "current_authority": null,
                "executor": null,
            },
            "origin": {"kind": "direct", "authority_ref": "instance"},
            "route": null,
            "canonicalization": "adapter-v1-canonical-json",
            "input_sha256": "0".repeat(64),
            "approval_mode": "required",
        }),
        "admission-stage" => json!("admission"),
        "invoke-activity" => json!("invoke"),
        _ => return None,
    })
}

/// Every value a scenario's fixture contract names; an unknown name fails the scenario.
fn fixture_values(
    contract: &ess_conformance::fixtures::Contract,
) -> std::result::Result<BTreeMap<String, Node>, TargetError> {
    contract
        .fields
        .iter()
        .map(|field| {
            let name = field.name.as_str();
            let value = fixture(name).ok_or_else(|| {
                TargetError::unavailable(
                    format!("fixture {name}"),
                    "no value is declared for this fixture name",
                )
            })?;
            Ok((name.to_owned(), to_node(&value)?))
        })
        .collect()
}

/// A value of `field`'s type the specification places no constraint on.
fn placeholder(field: &FieldDefinition, seed: u64) -> Value {
    if let Some(first) = field.values.first() {
        return json!(first);
    }
    match field.kind {
        FieldKind::String | FieldKind::Ref => {
            let text = format!("00000000-0000-4000-8000-{seed:012}");
            let min = field.min_length.unwrap_or(0);
            match field.max_length {
                Some(max) if text.len() > max => json!("x".repeat(min.max(1).min(max))),
                _ if text.len() < min => json!(format!("{text}{}", "x".repeat(min - text.len()))),
                _ => json!(text),
            }
        }
        FieldKind::Integer | FieldKind::Number | FieldKind::Binary64 => field
            .min
            .as_ref()
            .and_then(|min| serde_json::to_value(min).ok())
            .unwrap_or(json!(0)),
        FieldKind::Boolean => json!(false),
        FieldKind::Enum => json!(field.values.first().cloned().unwrap_or_default()),
        FieldKind::Array => json!([]),
        FieldKind::Map => json!({}),
        FieldKind::Object => {
            let mut object = Map::new();
            for (name, property) in &field.properties {
                if property.required {
                    object.insert(name.clone(), placeholder(property, seed));
                }
            }
            Value::Object(object)
        }
        FieldKind::Json | FieldKind::Union => json!({}),
    }
}

fn arguments_schema<'d>(
    definition: &'d entity_core::EntityDefinition,
    binding: &CommandBinding,
) -> Option<&'d entity_core::ObjectSchema> {
    match &binding.entrypoint {
        ess_entity_runtime::RuntimeEntrypoint::Create => Some(&definition.create.arguments),
        ess_entity_runtime::RuntimeEntrypoint::Operation { name } => definition
            .operations
            .get(&name.to_string())
            .map(|operation| &operation.arguments),
    }
}

impl Target<'_> {
    fn bound(
        &self,
        scenario: &mut Scenario,
        binding: &CommandBinding,
        schema: &entity_core::ObjectSchema,
    ) -> std::result::Result<Map<String, Value>, TargetError> {
        let slots = schema.fields.get("bound");
        let mut bound = Map::new();
        for (slot, value) in &binding.slots {
            let key = format!("b{:08}", slot.index());
            let field = slots
                .and_then(|bound| bound.properties.get(&key))
                .ok_or_else(|| unavailable(&key, "slot has no declared argument"))?;
            match (&value.source, value.presence) {
                (BoundSource::Generated, _)
                | (BoundSource::Undetermined, BoundPresence::Required) => {
                    scenario.generated += 1;
                    bound.insert(key, placeholder(field, scenario.generated));
                }
                (BoundSource::Undetermined, BoundPresence::Optional) => {}
                (other, _) => {
                    return Err(unavailable(
                        key,
                        format!("binding source {other:?} is not supplied by this target"),
                    ));
                }
            }
        }
        Ok(bound)
    }

    fn execute(
        &self,
        request: SemanticCommandRequest,
    ) -> std::result::Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let binding =
            self.authority.bindings.get(&command).ok_or_else(|| {
                unavailable(&command, "no binding in the local metadata authority")
            })?;
        let entity = binding.target.entity.to_string();
        let definition = self
            .authority
            .registry
            .get(&entity, 1)
            .ok_or_else(|| unavailable(&entity, "not registered"))?;
        let schema = arguments_schema(definition, binding)
            .ok_or_else(|| unavailable(&command, "no arguments schema"))?;
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unavailable(&command, "no scenario is open"))?;
        let mut input = Map::new();
        for (field, value) in &request.input {
            input.insert(field.clone(), absent_nulls(to_json(value)?));
        }
        let bound = self.bound(scenario, binding, schema)?;
        let arguments = json!({"input": Value::Object(input.clone()), "bound": bound});
        let runtime = Runtime::new(&self.authority.registry);
        let took = |outcome: &str| -> std::result::Result<OutcomeRef, TargetError> {
            Ok(OutcomeRef::new(
                CommandRef::new(name(&command)?),
                OutcomeName::new(outcome)
                    .map_err(|error| unavailable(outcome, format!("{error:?}")))?,
            ))
        };
        let evaluation = match &binding.entrypoint {
            ess_entity_runtime::RuntimeEntrypoint::Create => {
                match runtime.decide_create_derived(&entity, 1, arguments) {
                    Ok(evaluation) => evaluation,
                    Err(error) => return kernel_refusal(&command, error),
                }
            }
            ess_entity_runtime::RuntimeEntrypoint::Operation { name: operation } => {
                let InstanceBinding::Supplied { input_field } = &binding.instance else {
                    return Err(unavailable(
                        &command,
                        "operation without a supplied instance",
                    ));
                };
                let identity_field = definition
                    .identity
                    .as_ref()
                    .map(|identity| identity.field.clone())
                    .ok_or_else(|| unavailable(&entity, "no identity field"))?;
                let kind = definition
                    .schema
                    .fields
                    .get(&identity_field)
                    .map(|field| field.kind)
                    .ok_or_else(|| unavailable(&entity, "identity field not in schema"))?;
                let value = input
                    .get(input_field)
                    .ok_or_else(|| unavailable(input_field, "instance field absent from input"))?;
                let id = entity_core::identity::address(kind, value)
                    .map_err(|error| unavailable(input_field, error))?;
                let preload = match runtime.decide_before_load(
                    &entity,
                    1,
                    id.clone(),
                    &operation.to_string(),
                    arguments,
                ) {
                    Ok(preload) => preload,
                    Err(error) => return kernel_refusal(&command, error),
                };
                match preload {
                    PreloadDecision::Refused(refusal) => Evaluation::Refused(refusal),
                    PreloadDecision::Load(prepared) => {
                        match scenario.instances.get(&(entity.clone(), id.clone())) {
                            None => {
                                // No record carries this identity: the operation's declared
                                // wrong-state branch is what the lowering binds to absence.
                                // Without one, the executor answers a revision conflict, which
                                // no outcome declares: the command is refused undeclared and
                                // nothing is written.
                                let Some(wrong) = definition
                                    .operations
                                    .get(&operation.to_string())
                                    .and_then(|operation| {
                                        operation
                                            .outcomes
                                            .iter()
                                            .find(|outcome| outcome.wrong_state)
                                    })
                                else {
                                    return Ok(SemanticCommandResult::undeclared());
                                };
                                let error = wrong
                                    .refuses
                                    .as_ref()
                                    .map(|refusal| refusal.error.clone())
                                    .ok_or_else(|| {
                                        unavailable(&command, "wrong-state branch without an error")
                                    })?;
                                Evaluation::Refused(entity_core::Refusal {
                                    outcome: wrong.name.clone(),
                                    error,
                                    message: None,
                                })
                            }
                            Some(instance) => {
                                let selected = match prepared.select_with(instance) {
                                    Ok(selected) => selected,
                                    Err(error) => return kernel_refusal(&command, error),
                                };
                                match selected {
                                    LoadedDecision::Complete(evaluation) => evaluation,
                                    LoadedDecision::NeedsFulfillment(outcome) => {
                                        let mut actions = BTreeMap::new();
                                        for (field, requirement) in outcome.requirements() {
                                            let present = instance.fields.contains_key(field);
                                            let action = if present
                                                || !matches!(
                                                    requirement.actions,
                                                    entity_core::OperationFieldActions::Required
                                                ) {
                                                OperationFieldAction::Preserve
                                            } else {
                                                scenario.generated += 1;
                                                let target = definition
                                                    .schema
                                                    .fields
                                                    .get(field)
                                                    .ok_or_else(|| {
                                                        unavailable(field, "not in schema")
                                                    })?;
                                                OperationFieldAction::Set {
                                                    value: placeholder(target, scenario.generated),
                                                }
                                            };
                                            actions.insert(field.clone(), action);
                                        }
                                        match outcome.complete(actions) {
                                            Ok(evaluation) => evaluation,
                                            Err(error) => return kernel_refusal(&command, error),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        match evaluation {
            Evaluation::Refused(refusal) => Ok(SemanticCommandResult::took(took(
                &refusal.outcome,
            )?)
            .with_error(DeclaredErrorValue::new(ErrorRef::new(name(
                &refusal.error,
            )?)))),
            Evaluation::Accepted(decision) => {
                let outcome = decision
                    .record
                    .outcome
                    .clone()
                    .ok_or_else(|| unavailable(&command, "decision names no outcome"))?;
                let mut result = SemanticCommandResult::took(took(&outcome)?);
                for event in &decision.events {
                    let payload = match &event.payload {
                        Value::Object(map) => to_nodes(map)?,
                        Value::Null => BTreeMap::new(),
                        other => {
                            return Err(unavailable(&event.event_type, format!("payload {other}")));
                        }
                    };
                    let mut observed = ObservedEvent::new(EventRef::new(name(&event.event_type)?));
                    observed.payload = payload;
                    observed.correlation = Some(request.correlation.clone());
                    scenario.events.push(observed.clone());
                    result = result.emitting(observed);
                }
                if let Some(response) = &decision.record.response {
                    result.response = Some(to_nodes(response)?);
                }
                let instance = decision.instance;
                scenario
                    .instances
                    .insert((instance.entity.clone(), instance.id.clone()), instance);
                // Decisions apply synchronously, so every read after this write already sees it.
                scenario.writes += 1;
                let token = ess_primitives::consistency::ConsistencyToken::new(format!(
                    "w{}",
                    scenario.writes
                ))
                .map_err(|error| unavailable(&command, format!("{error:?}")))?;
                result = result.with_consistency(token);
                Ok(result)
            }
        }
    }
}

impl ConformanceTarget for Target<'_> {
    fn identity(&self) -> std::result::Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            "connectors-local-metadata-authority",
            &self.authority.identity,
        ))
    }

    fn fixture_values(
        &self,
        _: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> std::result::Result<BTreeMap<String, Node>, TargetError> {
        fixture_values(contract)
    }

    fn begin_scenario(&self, _: &ScenarioContext) -> std::result::Result<(), TargetError> {
        *self.scenario.borrow_mut() = Some(Scenario::default());
        Ok(())
    }

    fn end_scenario(&self, _: &ScenarioContext) -> std::result::Result<(), TargetError> {
        *self.scenario.borrow_mut() = None;
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> std::result::Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let result = self.execute(request)?;
        if result.consistency.is_some() {
            return Ok(result);
        }
        // A refusal writes nothing; its token names the write level it left unchanged.
        let writes = self
            .scenario
            .borrow()
            .as_ref()
            .map_or(0, |scenario| scenario.writes);
        let token = ess_primitives::consistency::ConsistencyToken::new(format!("w{writes}"))
            .map_err(|error| unavailable(&command, format!("{error:?}")))?;
        Ok(result.with_consistency(token))
    }

    fn query_view(
        &self,
        request: SemanticViewRequest,
    ) -> std::result::Result<SemanticViewResult, TargetError> {
        let view_name = request.view.to_string();
        let view = self
            .authority
            .views
            .get(&view_name)
            .ok_or_else(|| unavailable(&view_name, "no such view"))?;
        if !request.params.is_empty() {
            return Err(unavailable(
                &view_name,
                "parameterized views are not projected",
            ));
        }
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unavailable(&view_name, "no scenario is open"))?;
        if let Some(token) = request.consistency.token() {
            let written = token
                .as_str()
                .strip_prefix('w')
                .and_then(|n| n.parse::<u64>().ok())
                .ok_or_else(|| unavailable(&view_name, "foreign consistency token"))?;
            if written > scenario.writes {
                return Err(unavailable(
                    &view_name,
                    "token from a write this scenario never made",
                ));
            }
        }
        let mut rows = Vec::new();
        for ((entity, _), instance) in &scenario.instances {
            if *entity != view.source {
                continue;
            }
            let mut row = BTreeMap::new();
            for (field, type_ref) in &view.fields {
                let value = match instance.fields.get(field) {
                    Some(value) => {
                        let mut value = value.clone();
                        restore_nulls(&self.authority.types, type_ref, &mut value);
                        to_node(&value)?
                    }
                    None if field == "state" => Node::Text(instance.lifecycle_state.clone()),
                    None => Node::Null,
                };
                row.insert(field.clone(), value);
            }
            rows.push(row);
        }
        Ok(SemanticViewResult::of(rows))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> std::result::Result<Vec<ObservedEvent>, TargetError> {
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unavailable("events", "no scenario is open"))?;
        Ok(scenario
            .events
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> std::result::Result<(), TargetError> {
        Err(unavailable(
            format!("{request:?}"),
            "the local metadata authority has no external outcomes",
        ))
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> std::result::Result<(), TargetError> {
        Err(unavailable(
            request.event.to_string(),
            "the local metadata authority has no event bindings",
        ))
    }
}

/// Runs one admitted suite and returns its `ess-conformance-report/2` and detailed run.
fn run_suite(authority: &Authority, text: &str) -> Result<(CountReport, String)> {
    let admitted = AdmittedSuite::from_json(text)?;
    let target = Target::new(authority);
    let executed = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);
    let report = CountReport::from_run(&executed, &admitted)?;
    let details = CountRun::from_run(&executed, &admitted)?.to_canonical_json()?;
    Ok((report, details))
}

pub fn run(root: &Path, suite: &Path, report_out: &Path) -> Result<()> {
    let authority = Authority::load(root)?;
    let (report, details) = run_suite(&authority, &std::fs::read_to_string(suite)?)?;
    std::fs::write(report_out, report.to_canonical_json()?)?;
    std::fs::write(report_out.with_extension("run.json"), details)?;
    println!("{}", serde_json::to_string(report.counts())?);
    // An ordinary (non-coverage) suite declares no coverage, so its conformance status is
    // `inconclusive` even when every scenario passed; the per-scenario counts decide here.
    let counts = report.counts();
    if report.execution_status() != CountStatus::Passed || counts.passed != counts.total {
        return Err("conformance failed".into());
    }
    if report.conformance_status() != CountStatus::Passed {
        eprintln!(
            "coverage undeclared: conformance status {:?}",
            report.conformance_status()
        );
    }
    Ok(())
}

/// Runs every suite of an emission and writes `report.json` (and `run.json`) beside it.
pub fn run_emission(root: &Path, dir: &Path) -> Result<()> {
    let authority = Authority::load(root)?;
    let manifest = Manifest::from_json(&std::fs::read_to_string(dir.join(mutate::MANIFEST_FILE))?)?;
    let mut dirs = vec![manifest.baseline.dir.clone()];
    dirs.extend(
        manifest
            .mutants
            .iter()
            .filter_map(|mutant| mutant.dir.clone()),
    );
    for relative in dirs {
        let suite_dir = dir.join(&relative);
        let (report, details) = run_suite(
            &authority,
            &std::fs::read_to_string(suite_dir.join(mutate::SUITE_FILE))?,
        )?;
        std::fs::write(
            suite_dir.join(mutate::REPORT_FILE),
            report.to_canonical_json()?,
        )?;
        std::fs::write(suite_dir.join("run.json"), details)?;
        println!("{relative}: {}", serde_json::to_string(report.counts())?);
    }
    Ok(())
}

/// The gate's suite: this component's synthesis of `ir`, written under `out/dir`.
fn emit_suite(ir: &ess_compiler::ir::EssIr, dir: &str, out: &Path) -> Result<EmittedSuite> {
    let synthesis = ess_conformance::synthesize::synthesize_for(ir, metadata_entities::COMPONENT)
        .map_err(|error| error.to_string())?;
    let json = synthesis.suite.to_canonical_json()?;
    let mut refused: Vec<RefusalKey> = synthesis.refusals.iter().map(RefusalKey::of).collect();
    refused.sort();
    AdmittedSuite::from_json(&json)?;
    let target = out.join(dir);
    std::fs::create_dir_all(&target)?;
    std::fs::write(target.join(mutate::SUITE_FILE), &json)?;
    std::fs::write(
        target.join(mutate::MODEL_FILE),
        format!("{}\n", ir.to_compact_json()),
    )?;
    Ok(EmittedSuite {
        dir: dir.to_owned(),
        refusals: refused.len(),
        refused: Some(refused),
        scenarios: synthesis.suite.len(),
        spec_digest: synthesis.suite.provenance.spec_digest.to_string(),
        // A manifest field; the gate writes no manifest.
        suite_digest: None,
    })
}

/// Writes ESS's own mutation emission of `classes`, scoped to the component the host implements,
/// under `out` (which must be absent or empty), and returns its manifest.
///
/// `mutate::emit_for` is what `ess verify conform mutate --emit --component` runs: it decides the
/// manifest format from the mutants it writes, records each suite's digest, which guard mutants
/// left their outcome unsatisfiable, which answer identically and which overlap, and marks the
/// mutants and unavailable sites of other components out of scope. Writing its files unchanged
/// keeps every claim in the manifest one the pinned ESS made itself.
fn emit_classes(root: &Path, out: &Path, classes: &[MutantClass]) -> Result<Manifest> {
    if out.exists() && std::fs::read_dir(out)?.next().is_some() {
        return Err(format!("{} is not empty", out.display()).into());
    }
    let loaded = metadata_entities::load(root)?;
    let emission = mutate::emit_for(
        &loaded.documents,
        &loaded.sources,
        classes,
        Some(metadata_entities::COMPONENT),
    )?;
    for (relative, contents) in &emission.files {
        let relative = Path::new(relative);
        if !relative
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!("emission file {} leaves the output", relative.display()).into());
        }
        let file = out.join(relative);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, contents)?;
    }
    // What `mutate --collect` reads back, read back the same way.
    Ok(Manifest::from_json(&std::fs::read_to_string(
        out.join(mutate::MANIFEST_FILE),
    )?)?)
}

/// `mutate --emit`, scoped to the component the host implements.
pub fn emit(root: &Path, out: &Path) -> Result<()> {
    let manifest = emit_classes(root, out, MutantClass::ALL)?;
    let count = |keep: fn(&EmittedMutant) -> bool| {
        manifest
            .mutants
            .iter()
            .filter(|mutant| keep(mutant))
            .count()
    };
    let unavailable = manifest
        .unavailable_sites
        .as_ref()
        .map_or(0, std::vec::Vec::len);
    println!(
        "emitted {} ({} mutant(s): {} with a suite, {} stillborn, {} outside {}; {unavailable} \
         unavailable site(s)); baseline {} scenario(s)",
        manifest.format,
        manifest.mutants.len(),
        count(|mutant| mutant.dir.is_some()),
        count(|mutant| mutant.stillborn.is_some()),
        count(|mutant| mutant.out_of_scope),
        metadata_entities::COMPONENT,
        manifest.baseline.scenarios
    );
    Ok(())
}

/// The gate step: synthesizes this component's suite from the current specification, emits it
/// under `out`, and runs it against the committed definitions. Every scenario must pass.
pub fn gate(root: &Path, out: &Path) -> Result<()> {
    let loaded = metadata_entities::load(root)?;
    let ir = mutate::compile(loaded.documents.clone(), &loaded.sources)
        .map_err(|stillborn| format!("{stillborn:?}"))?;
    let suite = emit_suite(&ir, mutate::BASELINE_DIR, out)?;
    let dir = out.join(&suite.dir);
    run(
        root,
        &dir.join(mutate::SUITE_FILE),
        &dir.join("report.json"),
    )?;
    println!(
        "gate: local metadata authority conformance, {} scenarios, {} synthesis refusals; exit=0",
        suite.scenarios, suite.refusals
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn contract(name: &str) -> ess_conformance::fixtures::Contract {
        serde_json::from_value(json!({
            "fields": [{"name": name, "type": "String"}],
            "declarations": {},
        }))
        .unwrap()
    }

    /// `mutate --collect` reads what `emit` writes, so the manifest must be one the pinned ESS
    /// admits: its format for the mutants it holds, and each suite's refusals as that format
    /// requires them. Its mutants are precedence swaps, a class `ess-mutation-manifest/2` and `/3`
    /// cannot hold (the six of this component take ESS about 20 s; emit-swap takes minutes, and
    /// the full-class emission is the mutation audit's), so a manifest in a format too old for
    /// its mutants is refused here.
    #[test]
    fn emitted_manifest_is_admitted_by_the_pinned_ess() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let classes = [MutantClass::PrecedenceSwap];
        let out = tempfile::tempdir().unwrap();
        let admitted = emit_classes(&root, out.path(), &classes).unwrap();
        assert!(
            [
                mutate::MANIFEST_FORMAT_4,
                mutate::MANIFEST_FORMAT_5,
                mutate::MANIFEST_FORMAT_6,
            ]
            .contains(&admitted.format.as_str()),
            "{} is not a format that holds these classes",
            admitted.format
        );
        assert_eq!(
            admitted.component.as_deref(),
            Some(metadata_entities::COMPONENT)
        );
        for class in classes {
            assert!(
                admitted
                    .mutants
                    .iter()
                    .any(|mutant| mutant.class == class && mutant.dir.is_some()),
                "no {class:?} mutant with a suite"
            );
        }
        for mutant in &admitted.mutants {
            if let Some(dir) = &mutant.dir {
                let text =
                    std::fs::read_to_string(out.path().join(dir).join(mutate::SUITE_FILE)).unwrap();
                let suite = AdmittedSuite::from_json(&text).unwrap();
                assert_eq!(
                    mutant.suite_digest.as_deref(),
                    Some(suite.digest()),
                    "{} records another suite's digest",
                    mutant.id
                );
            }
        }

        let loaded = metadata_entities::load(&root).unwrap();
        let ir = mutate::compile(loaded.documents.clone(), &loaded.sources).unwrap();
        // The refusals a synthesis run separate from `emit` reports for the same model.
        let independent =
            ess_conformance::synthesize::synthesize_for(&ir, metadata_entities::COMPONENT).unwrap();
        let mut expected: Vec<RefusalKey> =
            independent.refusals.iter().map(RefusalKey::of).collect();
        expected.sort();
        assert!(
            !expected.is_empty(),
            "the model synthesizes with no refusals"
        );
        assert_eq!(admitted.baseline.refused.as_ref(), Some(&expected));
        assert_eq!(admitted.baseline.refusals, expected.len());

        // No refused scenario is in the suite the manifest points at.
        let suite: Value = serde_json::from_slice(
            &std::fs::read(
                out.path()
                    .join(&admitted.baseline.dir)
                    .join(mutate::SUITE_FILE),
            )
            .unwrap(),
        )
        .unwrap();
        let scenarios = suite["scenarios"].as_object().unwrap();
        for key in &expected {
            if let Some(scenario) = &key.scenario {
                assert!(
                    !scenarios.contains_key(scenario),
                    "{scenario} is refused and emitted"
                );
            }
        }
    }

    /// `story:oauth2-refresh-metadata-authority`: the local metadata authority owns
    /// `connectors.refresh` and `connectors.oauth2`. The suite the gate step synthesizes holds a
    /// scenario for every outcome of their ten commands, and each one passes against the
    /// committed definitions.
    #[test]
    fn every_refresh_and_oauth2_outcome_passes_against_the_committed_definitions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let loaded = metadata_entities::load(&root).unwrap();
        let ir = mutate::compile(loaded.documents.clone(), &loaded.sources).unwrap();
        let model: Value = serde_json::from_str(&ir.to_canonical_json()).unwrap();
        let commands: Vec<(&String, &Value)> = model["commands"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(name, _)| {
                name.starts_with("connectors.refresh.") || name.starts_with("connectors.oauth2.")
            })
            .collect();
        assert_eq!(commands.len(), 10, "the two domains declare ten commands");
        let expected: BTreeSet<String> = commands
            .iter()
            .flat_map(|(command, declaration)| {
                declaration["outcomes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(move |outcome| {
                        format!("{command}/outcome/{}", outcome["name"].as_str().unwrap())
                    })
            })
            .collect();

        let out = tempfile::tempdir().unwrap();
        let suite = emit_suite(&ir, mutate::BASELINE_DIR, out.path()).unwrap();
        let text =
            std::fs::read_to_string(out.path().join(&suite.dir).join(mutate::SUITE_FILE)).unwrap();
        let authority = Authority::load(&root).unwrap();
        let (report, _) = run_suite(&authority, &text).unwrap();
        let statuses = report.statuses();
        let unpassed: Vec<(&String, Option<&&str>)> = expected
            .iter()
            .map(|scenario| (scenario, statuses.get(scenario)))
            .filter(|(_, status)| *status != Some(&"passed"))
            .collect();
        assert!(
            unpassed.is_empty(),
            "outcomes without a passing scenario (status None: not in the suite): {unpassed:#?}"
        );
        let counts = report.counts();
        assert_eq!(
            counts.passed, counts.total,
            "every scenario of the suite passes"
        );
    }

    /// `IssueAccessToken`: a refreshed issue names the attempt that published its generation, and
    /// a current one names none. Either decision with the other shape is answered `unavailable`
    /// and issues nothing.
    #[test]
    fn only_a_refreshed_access_token_issue_names_a_refresh_attempt() {
        let registry = registry().unwrap();
        let runtime = Runtime::new(&registry);
        let attempt = "00000000-0000-4000-8000-08976e317bc2";
        let decide = |decision: &str, refresh_attempt_id: Option<&str>| {
            let mut input = json!({
                "decision": decision,
                "connection_ref": "connection",
                "generation_id": "00000000-0000-4000-8000-5a183923d085",
                "expires_at": "2020-01-01T00:00:00Z",
                "scopes": ["read"],
                "delivery": "metadata_only",
            });
            if let Some(attempt) = refresh_attempt_id {
                input["refresh_attempt_id"] = json!(attempt);
            }
            runtime
                .decide_create_derived(
                    "connectors.oauth2.AccessTokenIssue",
                    1,
                    json!({"input": input, "bound": {"b00000000": "issue"}}),
                )
                .unwrap()
        };
        let refused = |evaluation: Evaluation| match evaluation {
            Evaluation::Refused(refusal) => refusal.outcome,
            other => panic!("issued: {other:?}"),
        };
        let issued = |evaluation: Evaluation| match evaluation {
            Evaluation::Accepted(decision) => (
                decision.record.outcome.clone(),
                decision.instance.fields.get("refresh_attempt_id").cloned(),
            ),
            other => panic!("not issued: {other:?}"),
        };

        assert_eq!(
            issued(decide("current", None)),
            (Some("issued-current".to_owned()), None)
        );
        assert_eq!(
            issued(decide("refreshed", Some(attempt))),
            (Some("issued-refreshed".to_owned()), Some(json!(attempt)))
        );
        assert_eq!(refused(decide("current", Some(attempt))), "unavailable");
        assert_eq!(refused(decide("refreshed", None)), "unavailable");
    }

    #[test]
    fn unknown_fixture_is_an_error_not_a_default() {
        let error = fixture_values(&contract("no-such-fixture")).unwrap_err();
        assert!(!error.is_unsupported(), "{error}");
    }

    #[test]
    fn named_optional_members_round_trip_through_the_runtime_form() {
        let types: Map<String, Value> = serde_json::from_value(json!({
            "t.Nullable": {"body": {"kind": "newtype", "of": {"kind": "optional", "of": {"kind": "primitive", "name": "string"}}}},
            "t.S": {"body": {"kind": "struct", "fields": [
                {"name": "a", "type_ref": {"kind": "declared", "name": "t.Nullable"}},
                {"name": "b", "type_ref": {"kind": "optional", "of": {"kind": "primitive", "name": "string"}}},
            ]}},
        }))
        .unwrap();
        let spec = json!({"a": null});
        let mut stored = absent_nulls(spec.clone());
        assert_eq!(stored, json!({}));
        restore_nulls(
            &types,
            &json!({"kind": "declared", "name": "t.S"}),
            &mut stored,
        );
        assert_eq!(stored, spec);
    }

    /// Fixture inputs satisfy every invariant by construction, so a state the kernel discards for
    /// an invariant is the implementation's defect: the command reached no declared outcome, and
    /// the scenario that expected one fails. It is not an observation the target cannot expose.
    #[test]
    fn kernel_invariant_violation_is_a_failed_command_not_unsupported() {
        let result = kernel_refusal(
            "connectors.test.Command",
            entity_core::CoreError::InvariantViolation {
                rule: Some("rule".to_owned()),
                message: "does not hold".to_owned(),
            },
        )
        .expect("an invariant violation is an answer, not a target error");
        assert_eq!(result.outcome, None);
        assert_eq!(result.error, None);
        assert!(result.direct_events.is_empty());
    }

    #[test]
    fn other_kernel_errors_stay_unsupported() {
        let error = kernel_refusal(
            "connectors.test.Command",
            entity_core::CoreError::OperationNotFound {
                operation: "missing".to_owned(),
            },
        )
        .unwrap_err();
        assert!(error.is_unsupported(), "{error}");
    }
}
