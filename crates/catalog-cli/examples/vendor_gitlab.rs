//! Offline refresh of the complete official GitLab OpenAPI source and its coverage inventory.
//! Download the immutable URL recorded in provenance separately; this executable never networks.
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const REVISION: &str = "eaeb4b8b88fdee3fe9b1d1a54397226cf191c201";
const RAW_SHA256: &str = "f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530";
const SPEC: &str = "specs/gitlab/openapi-19.4.yaml";
const COVERAGE: &str = "specs/gitlab/coverage-19.4.toml";
const METHODS: [&str; 8] = [
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];
const SCHEDULES: [(&str, &str, &str, &str); 4] = [
    (
        "get",
        "/api/v4/projects/{id}/pipeline_schedules",
        "getApiV4ProjectsIdPipelineSchedules",
        "gitlab-pipeline-schedule-list",
    ),
    (
        "post",
        "/api/v4/projects/{id}/pipeline_schedules",
        "postApiV4ProjectsIdPipelineSchedules",
        "gitlab-pipeline-schedule-create",
    ),
    (
        "put",
        "/api/v4/projects/{id}/pipeline_schedules/{pipeline_schedule_id}",
        "putApiV4ProjectsIdPipelineSchedulesPipelineScheduleId",
        "gitlab-pipeline-schedule-update",
    ),
    (
        "delete",
        "/api/v4/projects/{id}/pipeline_schedules/{pipeline_schedule_id}",
        "deleteApiV4ProjectsIdPipelineSchedulesPipelineScheduleId",
        "gitlab-pipeline-schedule-delete",
    ),
];

#[derive(Parser)]
#[command(about = "Vendor the complete pinned GitLab OpenAPI source, offline")]
struct Args {
    /// Downloaded immutable official source; the exact upstream SHA-256 is required.
    #[arg(long)]
    input: PathBuf,
    /// Repository root containing the reviewed GitLab provider.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Verify deterministic output without writing it.
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let raw = std::fs::read(&args.input).context("read downloaded source")?;
    ensure!(
        sha256(&raw) == RAW_SHA256,
        "upstream SHA-256 differs from the reviewed pin"
    );
    let original: Value = serde_norway::from_slice(&raw).context("parse official OpenAPI")?;
    validate_source(&original, true)?;
    let mut scrubbed = original.clone();
    let removed = scrub_examples(&mut scrubbed, Mode::Regular);
    validate_source(&scrubbed, true)?;
    let yaml = serde_norway::to_string(&scrubbed)?;
    let provider = std::fs::read_to_string(args.root.join("providers/gitlab.toml"))?;
    let coverage = coverage(&scrubbed, &provider)?;
    let coverage_text = toml::to_string_pretty(&coverage)?;
    let provenance = provenance(&yaml, &coverage_text, removed)?;
    for (path, bytes) in [
        (SPEC, yaml.as_bytes()),
        (COVERAGE, coverage_text.as_bytes()),
        ("specs/gitlab.provenance.toml", provenance.as_bytes()),
    ] {
        let path = args.root.join(path);
        if args.check {
            ensure!(
                std::fs::read(&path).with_context(|| format!("read {}", path.display()))? == bytes,
                "{} differs from its deterministic projection",
                path.display()
            );
        } else {
            std::fs::create_dir_all(path.parent().context("output parent")?)?;
            std::fs::write(&path, bytes)?;
        }
    }
    println!(
        "{} source operations accounted for; {} example values removed; source SHA-256 {}",
        coverage.operation.len(),
        removed,
        sha256(yaml.as_bytes())
    );
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_source(document: &Value, pinned: bool) -> Result<()> {
    ensure!(document["openapi"] == "3.0.0", "expected OpenAPI 3.0.0");
    let paths = document["paths"]
        .as_object()
        .context("paths must be an object")?;
    let mut ids = BTreeSet::new();
    for (path, item) in paths {
        for method in METHODS {
            if let Some(operation) = item.get(method) {
                let id = operation["operationId"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .with_context(|| format!("missing operationId: {method} {path}"))?;
                ensure!(ids.insert(id), "duplicate operationId: {id}");
            }
        }
    }
    if pinned {
        ensure!(
            paths.len() == 1367 && ids.len() == 1847 && document["info"]["version"] == "19.4",
            "pinned source inventory changed"
        );
        for (method, path, id, _) in SCHEDULES {
            ensure!(
                document["paths"][path][method]["operationId"] == id,
                "selected operation changed: {method} {path}"
            );
        }
    }
    validate_refs(document, document)?;
    Ok(())
}

fn validate_refs(value: &Value, root: &Value) -> Result<()> {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref") {
                let reference = reference.as_str().context("reference must be a string")?;
                let pointer = reference
                    .strip_prefix('#')
                    .context("external reference is unsupported")?;
                ensure!(
                    pointer.starts_with('/') && root.pointer(pointer).is_some(),
                    "unresolved reference: {reference}"
                );
            }
            for value in object.values() {
                validate_refs(value, root)?;
            }
        }
        Value::Array(array) => {
            for value in array {
                validate_refs(value, root)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn normalized_path(path: &str) -> String {
    path.strip_prefix("/api/v4")
        .unwrap_or(path)
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
fn auth_path(path: &str) -> bool {
    path.split('/').any(|segment| segment == "oauth")
}

#[derive(Clone, Copy)]
enum Mode {
    Regular,
    Declarations,
    Components,
    ExampleDeclarations,
    ExampleObject,
}

fn scrub_examples(value: &mut Value, mode: Mode) -> usize {
    const DECLARATIONS: [&str; 17] = [
        "$defs",
        "callbacks",
        "dependentSchemas",
        "definitions",
        "encoding",
        "headers",
        "links",
        "parameters",
        "pathItems",
        "patternProperties",
        "properties",
        "requestBodies",
        "responses",
        "schemas",
        "securitySchemes",
        "webhooks",
        "paths",
    ];
    match value {
        Value::Object(map) => {
            let keys: Vec<_> = map.keys().cloned().collect();
            let mut removed = 0;
            for key in keys {
                let drop = matches!(mode, Mode::Regular)
                    && ["example", "examples"].contains(&key.as_str())
                    || matches!(mode, Mode::ExampleObject)
                        && ["value", "externalValue"].contains(&key.as_str());
                if drop {
                    map.remove(&key);
                    removed += 1;
                    continue;
                }
                let child_mode = match mode {
                    Mode::Components if key == "examples" => Mode::ExampleDeclarations,
                    Mode::Components if DECLARATIONS.contains(&key.as_str()) => Mode::Declarations,
                    Mode::Components => Mode::Regular,
                    Mode::ExampleDeclarations => Mode::ExampleObject,
                    Mode::Declarations => Mode::Regular,
                    _ if key == "components" => Mode::Components,
                    _ if DECLARATIONS.contains(&key.as_str()) => Mode::Declarations,
                    _ => Mode::Regular,
                };
                removed += scrub_examples(map.get_mut(&key).expect("existing key"), child_mode);
            }
            removed
        }
        Value::Array(array) => array
            .iter_mut()
            .map(|value| scrub_examples(value, Mode::Regular))
            .sum(),
        _ => 0,
    }
}

#[derive(Serialize)]
struct Coverage {
    source: String,
    source_sha256: String,
    authority: String,
    goal: String,
    platform_auth_flow: Vec<AuthFlow>,
    operation: Vec<CoverageRow>,
}
#[derive(Serialize)]
struct AuthFlow {
    scheme: String,
    flow: String,
    field: String,
    source_url: String,
    status: String,
    note: String,
}
#[derive(Serialize)]
struct CoverageRow {
    operation_id: String,
    method: String,
    path: String,
    status: String,
    catalog_id: String,
    permission_surface: String,
    reason: String,
    importer_diagnostics: Vec<String>,
}
fn coverage(document: &Value, provider: &str) -> Result<Coverage> {
    validate_source(document, false)?;
    let text = serde_norway::to_string(document)?;
    let ingested = connector_spec::openapi::ingest_with_semantics(
        &text,
        connector_spec::RequestSemantics::OpenApi30JsonV1,
    )?;
    let readable: BTreeSet<_> = ingested.operation_ids().into_iter().collect();
    let loaded = connector_spec::provider::load_with_spec(
        "providers/gitlab.toml",
        provider,
        &[connector_spec::SpecDocument {
            path: SPEC,
            document: &text,
        }],
    )?;
    let generated: BTreeMap<_, _> = loaded
        .connector
        .provenance
        .operation_specs
        .iter()
        .map(|(catalog, source)| (source.operation_id.as_str(), catalog.as_str()))
        .collect();
    let declaration: toml::Value = toml::from_str(provider)?;
    let mut legacy = BTreeMap::new();
    if let Some(operations) = declaration
        .get("operations")
        .and_then(toml::Value::as_array)
    {
        for operation in operations {
            let method = operation["method"].as_str().context("legacy method")?;
            let path = operation["path"].as_str().context("legacy path")?;
            legacy.insert(
                (method.to_owned(), normalized_path(path)),
                operation["id"].as_str().context("legacy id")?.to_owned(),
            );
        }
    }
    let mut rows = Vec::new();
    for (path, item) in document["paths"].as_object().context("paths")? {
        for method in METHODS {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let id = operation["operationId"].as_str().context("operation id")?;
            let location = format!("{} {path}", method.to_uppercase());
            let diagnostics: Vec<_> = ingested
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.location == location)
                .map(|diagnostic| diagnostic.problem.clone())
                .collect();
            let selected = generated.get(id);
            let legacy_id = legacy.get(&(method.to_uppercase(), normalized_path(path)));
            let (status, catalog_id, surface, reason) = if let Some(catalog) = selected {
                ensure!(
                    readable.contains(id),
                    "selected operation importer gap: {location}: {diagnostics:?}"
                );
                if SCHEDULES
                    .iter()
                    .any(|(_, _, source_id, _)| *source_id == id)
                {
                    ("catalogued_generated", (*catalog).to_owned(), "project_permission", "Selected source-faithful OpenAPI 3.0 JSON projection: complete source parameters/body/response constraints, independent body presence and safe scalar path encoding. Source defects remain diagnosed without schema corrections. GitLab project roles and credential scopes still apply.")
                } else {
                    ("catalogued_generated", (*catalog).to_owned(), "unclassified", "Selected by the reviewed provider and derived from the official source. Administrative versus regular-user projection classification remains explicit review work; credentials and grants enforce authority.")
                }
            } else if let Some(catalog) = legacy_id {
                ("catalogued_legacy", catalog.clone(), "unclassified", "Existing inline catalog operation; endpoint correspondence only, not a claim of generated schema coverage. Migrate its contract to official ingest and review permission classification without changing the stable id.")
            } else if auth_path(path) {
                ("platform_auth_flow", String::new(), "platform", "Authentication flow belongs to the platform and is withheld from invocable operation projections.")
            } else if !readable.contains(id) {
                ensure!(
                    !diagnostics.is_empty(),
                    "unaccounted importer gap: {location}"
                );
                ("importer_gap", String::new(), "unclassified", "Extend the importer for the recorded diagnostic, then review effects, risk, credentials and admin/user classification before cataloguing. Never replace the vendor source with an authored definition.")
            } else {
                ("coverage_gap", String::new(), "unclassified", "The bounded source-profile importer produced IR; this is not reviewed support or admission. Review schema/source diagnostics, effects, risk, credentials and admin/user classification before cataloguing. Focused consumer projections are independent.")
            };
            rows.push(CoverageRow {
                operation_id: id.to_owned(),
                method: method.to_uppercase(),
                path: path.clone(),
                status: status.to_owned(),
                catalog_id,
                permission_surface: surface.to_owned(),
                reason: reason.to_owned(),
                importer_diagnostics: diagnostics,
            });
        }
    }
    let mut flows = Vec::new();
    if let Some(schemes) = document
        .pointer("/components/securitySchemes")
        .and_then(Value::as_object)
    {
        for (scheme, definition) in schemes {
            if let Some(flow_map) = definition.get("flows").and_then(Value::as_object) {
                for (flow, definition) in flow_map {
                    for field in ["authorizationUrl", "tokenUrl", "refreshUrl"] {
                        if let Some(url) = definition.get(field).and_then(Value::as_str) {
                            flows.push(AuthFlow { scheme: scheme.clone(), flow: flow.clone(), field: field.to_owned(), source_url: url.to_owned(), status: "platform_auth_flow".to_owned(), note: "Source metadata only, never an invocable operation or authority. GitLab source /api/oauth endpoints differ from the established /oauth provider registration; preserve source and retain the reviewed provider endpoints.".to_owned() });
                        }
                    }
                }
            }
        }
    }
    Ok(Coverage {
        source: SPEC.to_owned(),
        source_sha256: sha256(text.as_bytes()),
        authority: "credentials_and_grants".to_owned(),
        goal: "all_official_api_operations_with_explicit_platform_and_importer_accounting"
            .to_owned(),
        platform_auth_flow: flows,
        operation: rows,
    })
}
fn provenance(yaml: &str, coverage: &str, removed: usize) -> Result<String> {
    let upstream = format!(
        "https://gitlab.com/gitlab-org/gitlab/-/raw/{REVISION}/doc/api/openapi/openapi_v3.yaml"
    );
    let record = json!({
        "origin":"vendor", "upstream":upstream, "upstream_revision":REVISION,
        "retrieved":"2026-09-06", "upstream_version":"19.4", "upstream_sha256":RAW_SHA256,
        "path":SPEC, "sha256":sha256(yaml.as_bytes()), "coverage_path":COVERAGE,
        "coverage_sha256":sha256(coverage.as_bytes()), "source_paths":1367, "source_operations":1847,
        "scrub":"Remove OpenAPI example/examples values and component Example Object value/externalValue only, preserving declaration names and all endpoint/schema structure; deterministic YAML serialization.",
        "scrubbed_values":removed,
        "refresh":"cargo run -p catalog-cli --locked --example vendor_gitlab -- --input <downloaded-immutable-openapi.yaml> --root .",
        "references":["https://docs.gitlab.com/api/openapi/", "https://docs.gitlab.com/api/pipeline_schedules/", "https://docs.gitlab.com/api/rest/authentication/", "https://docs.gitlab.com/api/rest/"],
        "projection_notes":["Complete vendor source retained; the four schedule operations are a first delivery projection, not complete connector coverage.", "Existing inline operations retain their original authored provenance and explicit legacy_v1 semantics until separately migrated.", "Official schedule docs describe list, inputs and variables as arrays while the source references objects; literal source schemas are preserved without object-to-array corrections.", "Selected schemas preserve requiredness, nullable, defaults, unions and constraints through standards-based OpenAPI 3.0 to JSON Schema translation. Project parameters retain string/integer alternatives and use safe path encoding.", "Complete request bodies include heterogeneous inputs and preserve optional body/field omission separately from explicit null. The source inputs.value array alternative omits items, contrary to OpenAPI 3.0 structural requirements; its absent constraint remains absent and is diagnosed. No source-validity certification is claimed.", "OAuth source metadata is not an operation or scope grant; established provider /oauth endpoints and reviewed read_api/api requirements remain authoritative for this connector.", "Administrative versus regular-user surfaces remain unclassified until source-grounded permission review; a projection never grants authority."],
        "selected_operation": SCHEDULES.iter().map(|(method,path,id,catalog)| json!({"method":method.to_uppercase(),"path":path,"operation_id":id,"catalog_id":catalog})).collect::<Vec<_>>()
    });
    Ok(toml::to_string_pretty(&record)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"openapi":"3.0.0","info":{"title":"fixture","version":"19.4"},"paths":{
            "/things":{"get":{"operationId":"getThings","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}}
        }})
    }
    #[test]
    fn scrub_removes_values_without_erasing_example_named_declarations() {
        let mut value = json!({"components":{"schemas":{"example":{"type":"object","properties":{"example":{"type":"string","example":"private"},"examples":{"type":"string","examples":["private"]}},"example":{"example":"private"}}},"examples":{"example":{"value":"private"},"examples":{"externalValue":"https://private.example"}}},"paths":{"/x":{"get":{"responses":{"200":{"content":{"application/json":{"examples":{"sample":{"value":"private"}}}}}}}}}});
        assert_eq!(scrub_examples(&mut value, Mode::Regular), 6);
        assert_eq!(
            value["components"]["schemas"]["example"]["properties"]["example"],
            json!({"type":"string"})
        );
        assert_eq!(
            value["components"]["schemas"]["example"]["properties"]["examples"],
            json!({"type":"string"})
        );
        assert!(value["components"]["examples"].get("example").is_some());
        assert!(!value.to_string().contains("private"));
        assert_eq!(scrub_examples(&mut value, Mode::Regular), 0);
    }
    #[test]
    fn source_validation_keeps_unselected_operations_and_refuses_ambiguous_ids() {
        let mut value = fixture();
        validate_source(&value, false).unwrap();
        value["paths"]["/other"] = value["paths"]["/things"].clone();
        assert!(validate_source(&value, false)
            .unwrap_err()
            .to_string()
            .contains("duplicate operationId"));
        value["paths"]["/other"]["get"]["operationId"] = json!("");
        assert!(validate_source(&value, false)
            .unwrap_err()
            .to_string()
            .contains("missing operationId"));
    }
    #[test]
    fn source_validation_refuses_external_and_unresolved_refs() {
        for reference in ["other.yaml#/Thing", "#/components/schemas/Absent"] {
            let mut value = fixture();
            value["paths"]["/things"]["get"]["responses"]["200"]["content"]["application/json"]
                ["schema"] = json!({"$ref":reference});
            assert!(validate_source(&value, false)
                .unwrap_err()
                .to_string()
                .contains("reference"));
        }
    }
    #[test]
    fn pinned_source_refuses_changed_inventory_and_selected_operation() {
        assert!(validate_source(&fixture(), true)
            .unwrap_err()
            .to_string()
            .contains("inventory"));
    }
    #[test]
    fn coverage_accounts_for_gaps_legacy_and_platform_flows_without_authority() {
        let mut value = fixture();
        value["paths"]["/other"] =
            json!({"post":{"operationId":"postOther","responses":{"204":{"description":"ok"}}}});
        value["components"] = json!({"securitySchemes":{"oauth2":{"type":"oauth2","flows":{"authorizationCode":{"authorizationUrl":"https://vendor.example/oauth/authorize","tokenUrl":"https://vendor.example/oauth/token","scopes":{}}}}}});
        let provider = r#"
id = "fixture"
base_url = "https://vendor.example"
[[operations]]
id = "legacy-things"
method = "GET"
path = "/things"
direction = "read"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]
description = "Read fixture things"
"#;
        let inventory = coverage(&value, provider).unwrap();
        assert_eq!(inventory.operation.len(), 2);
        assert!(inventory
            .operation
            .iter()
            .any(|row| row.catalog_id == "legacy-things" && row.status == "catalogued_legacy"));
        assert!(inventory
            .operation
            .iter()
            .any(|row| row.status == "coverage_gap" && row.permission_surface == "unclassified"));
        assert_eq!(inventory.platform_auth_flow.len(), 2);
        assert_eq!(inventory.authority, "credentials_and_grants");
        assert_eq!(
            toml::to_string_pretty(&inventory).unwrap(),
            toml::to_string_pretty(&coverage(&value, provider).unwrap()).unwrap()
        );
    }
    #[test]
    fn coverage_does_not_claim_an_operation_the_provider_no_longer_selects() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let document: Value =
            serde_norway::from_str(&std::fs::read_to_string(root.join(SPEC)).unwrap()).unwrap();
        let mut provider: toml::Value =
            toml::from_str(&std::fs::read_to_string(root.join("providers/gitlab.toml")).unwrap())
                .unwrap();
        provider["patch"]["operations"]
            .as_array_mut()
            .unwrap()
            .retain(|patch| patch["select"].as_str() != Some(SCHEDULES[0].2));
        let inventory = coverage(&document, &toml::to_string(&provider).unwrap()).unwrap();
        let removed = inventory
            .operation
            .iter()
            .find(|row| row.operation_id == SCHEDULES[0].2)
            .unwrap();
        assert_eq!(removed.status, "coverage_gap");
        assert!(removed.catalog_id.is_empty());
    }

    #[test]
    fn committed_source_inventory_is_reproducible() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let text = std::fs::read_to_string(root.join(SPEC)).expect("full source");
        let document: Value = serde_norway::from_str(&text).unwrap();
        validate_source(&document, true).unwrap();
        let inventory = coverage(
            &document,
            &std::fs::read_to_string(root.join("providers/gitlab.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            toml::to_string_pretty(&inventory).unwrap(),
            std::fs::read_to_string(root.join(COVERAGE)).unwrap()
        );
    }
}
