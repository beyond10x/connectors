use super::Result;
use ess_compiler::{resolve::compile_locating, source::SourceMap};
use ess_domain::{
    component::ComponentName,
    name::QualifiedName,
    spec::{RawSpecFile, Specification},
    system::Source,
};
use ess_entity_runtime::{
    BoundPresence, BoundSource, BoundTarget, LoweringOptions, RuntimeEntrypoint, lower,
};
use ess_service_contract::extract;
use ess_synth::SynthesisPlan;
use serde_json::{Value, json};
use std::{collections::BTreeMap, num::NonZeroU32, path::Path};

const COMPONENT: &str = "local-metadata-authority";
const DEFINITIONS: &[&str] = &[
    "connectors.approval_issuers.ApprovalIssuer",
    "connectors.approval_issuers.ApprovalSigningKey",
    "connectors.artifact_provenance.Source",
    "connectors.auth_bindings.Acquisition",
    "connectors.auth_bindings.AuthProfile",
    "connectors.auth_bindings.Connection",
    "connectors.auth_bindings.CustodyVersion",
    "connectors.credential_evidence.DispatchAdmission",
    "connectors.credential_evidence.ReadUse",
    "connectors.clock.LocalClockFloor",
    "connectors.cli.ConnectionListCursor",
    "connectors.cli.LocalRuntimeRecord",
    "connectors.credentials.CredentialGeneration",
    "connectors.declarations.AdapterSpecification",
    "connectors.declarations.OperationDeclaration",
    "connectors.declarations.ServiceConfiguration",
    "connectors.delegation.ApprovalRedemption",
    "connectors.delegation.DeliveryReceipt",
    "connectors.execution_audit.AuditRecord",
    "connectors.idempotency.KeyReservation",
    "connectors.local_approval_policy.LocalApprovalPolicy",
    "connectors.mutations.AttemptRecord",
];

pub fn run(root: &Path, check: bool) -> Result<()> {
    let source = root.join("ess");
    let mut paths = Vec::new();
    collect_yaml(&source, &mut paths)?;
    paths.sort();
    let mut parsed = Vec::new();
    let mut labels = Vec::new();
    let mut sources = SourceMap::new();
    for path in paths {
        let label = path.strip_prefix(&source)?.display().to_string();
        let text = std::fs::read_to_string(&path)?;
        sources.insert(label.clone(), text.clone());
        parsed.push((
            Source::new(label.clone()),
            RawSpecFile::parse(&text).map_err(|error| format!("{label}: {error}"))?,
        ));
        labels.push(label);
    }
    let specification = Specification::assemble(parsed)?;
    let ir = compile_locating(&specification, &sources, &labels)
        .map_err(|diagnostics| format!("ESS compile diagnostics: {diagnostics:?}"))?;
    let plan = SynthesisPlan::of(&ir);
    let component = ComponentName::new(COMPONENT)?;
    let service = extract(&ir, &plan, &component)
        .map_err(|diagnostics| format!("service extraction diagnostics: {diagnostics:?}"))?;
    let options = LoweringOptions {
        definition_versions: DEFINITIONS
            .iter()
            .map(|name| {
                Ok((
                    QualifiedName::new(*name)?,
                    NonZeroU32::new(1).expect("one is nonzero"),
                ))
            })
            .collect::<std::result::Result<BTreeMap<_, _>, Box<dyn std::error::Error>>>()?,
        scales: BTreeMap::new(),
    };
    let lowered = lower(&service, &options)
        .map_err(|diagnostics| format!("Entity Runtime lowering diagnostics: {diagnostics:?}"))?;
    let definitions = lowered
        .definitions()
        .iter()
        .map(|(name, definition)| {
            Ok(json!({
                "name": name.to_string(),
                "definition": serde_json::to_value(definition.as_definition())?,
            }))
        })
        .collect::<std::result::Result<Vec<Value>, serde_json::Error>>()?;
    let commands = lowered
        .bindings()
        .commands()
        .iter()
        .map(|(name, binding)| {
            let entrypoint = match &binding.entrypoint {
                RuntimeEntrypoint::Create => Value::Null,
                RuntimeEntrypoint::Operation { name } => json!(name.to_string()),
            };
            let slots = binding
                .slots
                .iter()
                .map(|(slot, value)| {
                    let (target, outcome, field) = match &value.target {
                        BoundTarget::ExternalEvidence { outcome } => {
                            ("external_evidence", Some(json!(outcome)), None)
                        }
                        BoundTarget::LogicalIdentity { at } => (
                            "logical_identity",
                            Some(json!(&at.outcome)),
                            Some(json!(&at.field)),
                        ),
                        BoundTarget::EntityField { outcome, field } => {
                            ("entity_field", Some(json!(outcome)), Some(json!(field)))
                        }
                        BoundTarget::EventField { outcome, field, .. } => {
                            ("event_field", Some(json!(outcome)), Some(json!(field)))
                        }
                        BoundTarget::ResponseField { outcome, field } => {
                            ("response_field", Some(json!(outcome)), Some(json!(field)))
                        }
                    };
                    let source = match &value.source {
                        BoundSource::Identity => "identity",
                        BoundSource::ResponseField { .. } => "response_field",
                        BoundSource::Generated => "generated",
                        BoundSource::Undetermined => "undetermined",
                        BoundSource::Conversion { .. } => "conversion",
                        BoundSource::External { .. } => "external",
                    };
                    json!({
                        "slot": format!("b{:08}", slot.index()),
                        "target": target,
                        "outcome": outcome,
                        "field": field,
                        "source": source,
                        "required": matches!(value.presence, BoundPresence::Required),
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "name": name.to_string(),
                "entity": binding.target.entity.to_string(),
                "operation": entrypoint,
                "slots": slots,
            })
        })
        .collect::<Vec<Value>>();
    let document = json!({
        "format": "connectors.entity-runtime-definitions/1",
        "component": COMPONENT,
        "source_digest": lowered.source_digest(),
        "synthesis_digest": lowered.synthesis_digest(),
        "entity_runtime_revision": lowered.target_revision(),
        "definitions": definitions,
        "commands": commands,
    });
    let bytes = connectors_spec::v2::json_bytes(&document)?;
    let destination =
        root.join("crates/connectors-host/src/local/metadata/entity-runtime-definitions.json");
    if check {
        if std::fs::read(&destination)? != bytes {
            return Err("local metadata Entity Runtime definition drift".into());
        }
    } else {
        connectors_spec::v2::write(&destination, &bytes)?;
    }
    Ok(())
}

fn collect_yaml(directory: &Path, paths: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml(&path, paths)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
        {
            paths.push(path);
        }
    }
    Ok(())
}
