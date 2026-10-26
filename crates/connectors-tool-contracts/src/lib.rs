//! An instance's `tool-contract/1` document (Atlas ADR 0103; `contracts/tools/v1alpha1/semantics.md`).
//!
//! [`publish`] turns an instance's [`Descriptor`] and the [`Declarations`] authored beside its
//! adapter (`adapters/<adapter>/spec/tool-profiles.json`) into one [`Contract`]: toolset and family
//! `connector-<instance>`, one operation per admitted operation, each with the full profile its
//! author declared, accepted by `b10x-tool-contract`'s validator. It refuses the whole document,
//! naming each operation it cannot publish ([`Refusal`]); it never defaults a profile.
//! [`AdapterToolset`] serves the published contract over the adapter's own `invoke`, so the
//! contract can be held to `tool-conformance/1`.
//!
//! A published contract names no credential, configuration value, connection or grant.

mod schema;
mod toolset;

use connectors_core::{Descriptor, ErrorCode};
use serde::Deserialize;
use std::{collections::BTreeMap, fmt, path::Path};
pub use tool_contract::{self, Contract};
use tool_contract::{ErrorDecl, Field, Operation, Outcome, Profile, TypeDecl, TypeKind};
pub use toolset::AdapterToolset;

/// The `format` of a declarations document.
pub const DECLARATIONS_FORMAT: &str = "connectors.tool-profiles/1";
/// The version every published contract carries.
pub const VERSION: &str = "v1alpha1";
/// The error every published operation answers for a refusal or failure of its invocation.
pub const FAILED: &str = "connector.Failed";
/// The error an operation with `omit` answers for an input that carries an omitted field.
pub const FIELD_NOT_ALLOWED: &str = "connector.FieldNotAllowed";
/// The enum type of [`FAILED`]'s `code` field.
pub const ERROR_CODE: &str = "connector.ErrorCode";
/// The adapters whose generated contract is committed under `adapters/<adapter>/generated/`.
pub const ADAPTERS: &[&str] = &["tavily"];

/// What an adapter's author declares about its operations: `tool-profiles.json`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declarations {
    /// Always [`DECLARATIONS_FORMAT`].
    pub format: String,
    /// The contract's one paragraph.
    pub summary: String,
    /// Per descriptor operation id, its declaration.
    pub operations: BTreeMap<String, Declaration>,
}

/// One operation's declaration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// The `<op>` segment of `connector-<instance>.<op>`.
    pub name: String,
    /// One line; the descriptor's description when absent.
    #[serde(default)]
    pub summary: Option<String>,
    /// How to call it well.
    #[serde(default)]
    pub guidance: Option<String>,
    /// The `tool-contract/1` profile. Absent is refused.
    #[serde(default)]
    pub profile: Option<Profile>,
    /// Input fields the connection fills; they leave the published input.
    #[serde(default)]
    pub omit: Vec<String>,
}

impl Declarations {
    /// Parses a declarations document.
    ///
    /// # Errors
    ///
    /// The text is not JSON of this grammar, or its `format` is not [`DECLARATIONS_FORMAT`].
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let declarations: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if declarations.format != DECLARATIONS_FORMAT {
            return Err(format!(
                "format is {:?}, not {DECLARATIONS_FORMAT}",
                declarations.format
            ));
        }
        Ok(declarations)
    }
}

/// Why a document was not published; the model's `PublicationDecision`
/// (`ess/domains/tool_contracts.yaml`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalKind {
    /// An admitted operation without a profile, or a declaration for an operation the descriptor
    /// does not admit.
    ProfileMissing,
    /// A schema the `tool-contract/1` grammar cannot type, or an `omit` naming no input field.
    SchemaUnrepresentable,
    /// A risk under the floor its effects and consequences set (rule `risk`).
    RiskBelowFloor,
    /// Any other validator rule, or a profile that names a secret.
    ProfileInvalid,
    /// An instance or operation name outside its grammar.
    NameInvalid,
}

impl RefusalKind {
    /// The outcome name the model gives this refusal.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileMissing => "profile-missing",
            Self::SchemaUnrepresentable => "schema-unrepresentable",
            Self::RiskBelowFloor => "risk-below-floor",
            Self::ProfileInvalid => "profile-invalid",
            Self::NameInvalid => "name-invalid",
        }
    }
}

/// One reason the document was not published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Which refusal.
    pub kind: RefusalKind,
    /// The descriptor operation id, the published name, or `document`.
    pub operation: String,
    /// What was refused.
    pub detail: String,
}

impl Refusal {
    fn new(kind: RefusalKind, operation: &str, detail: impl Into<String>) -> Self {
        Self {
            kind,
            operation: operation.to_owned(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}: {}",
            self.operation,
            self.kind.as_str(),
            self.detail
        )
    }
}

/// A published contract and how each of its operations reaches the adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Publication {
    /// The document.
    pub contract: Contract,
    /// One per contract operation, in the contract's order.
    pub operations: Vec<Published>,
}

/// One published operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {
    /// The published name, `connector-<instance>.<op>`.
    pub name: String,
    /// The descriptor operation id the adapter answers.
    pub operation: String,
    /// The input fields the connection fills.
    pub omit: Vec<String>,
}

/// Every service error code, as the contract's [`ERROR_CODE`] enum spells it.
pub fn error_codes() -> Vec<String> {
    // A new variant fails to compile here until it is listed below.
    fn listed(code: &ErrorCode) {
        match code {
            ErrorCode::InvalidInput
            | ErrorCode::Unsupported
            | ErrorCode::Unauthorized
            | ErrorCode::Forbidden
            | ErrorCode::NotFound
            | ErrorCode::StaleDescription
            | ErrorCode::StaleCursor
            | ErrorCode::RateLimited
            | ErrorCode::Unavailable
            | ErrorCode::Capacity
            | ErrorCode::Timeout
            | ErrorCode::UpstreamProtocol
            | ErrorCode::Internal
            | ErrorCode::Conflict => {}
        }
    }
    [
        ErrorCode::InvalidInput,
        ErrorCode::Unsupported,
        ErrorCode::Unauthorized,
        ErrorCode::Forbidden,
        ErrorCode::NotFound,
        ErrorCode::StaleDescription,
        ErrorCode::StaleCursor,
        ErrorCode::RateLimited,
        ErrorCode::Unavailable,
        ErrorCode::Capacity,
        ErrorCode::Timeout,
        ErrorCode::UpstreamProtocol,
        ErrorCode::Internal,
        ErrorCode::Conflict,
    ]
    .iter()
    .map(|code| {
        listed(code);
        code_text(code)
    })
    .collect()
}

/// An error code as the contract spells it, `invalid_input` for [`ErrorCode::InvalidInput`].
pub fn code_text(code: &ErrorCode) -> String {
    match serde_json::to_value(code) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "internal".to_owned(),
    }
}

fn is_name(text: &str, rest: impl Fn(char) -> bool) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(rest)
}

/// Publishes `descriptor`'s operations under `declarations`.
///
/// # Errors
///
/// Every refusal, naming its operation; nothing is published when there is one.
pub fn publish(
    descriptor: &Descriptor,
    declarations: &Declarations,
) -> Result<Publication, Vec<Refusal>> {
    let mut refusals = Vec::new();
    let instance = descriptor.instance.as_str();
    if !is_name(instance, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
    }) {
        refusals.push(Refusal::new(
            RefusalKind::NameInvalid,
            "document",
            format!("instance {instance:?} is outside [a-z][a-z0-9-]*"),
        ));
    }
    let family = format!("connector-{instance}");
    for id in declarations.operations.keys() {
        if descriptor.operations.iter().all(|o| &o.id != id) {
            refusals.push(Refusal::new(
                RefusalKind::ProfileMissing,
                id,
                "declared, but the descriptor does not admit this operation",
            ));
        }
    }
    let mut types: Vec<TypeDecl> = Vec::new();
    let mut operations = Vec::new();
    let mut published = Vec::new();
    let mut any_omit = false;
    for operation in &descriptor.operations {
        let id = operation.id.as_str();
        let Some(declaration) = declarations.operations.get(id) else {
            refusals.push(Refusal::new(
                RefusalKind::ProfileMissing,
                id,
                "admitted, but no declaration names it",
            ));
            continue;
        };
        let Some(profile) = declaration.profile.clone() else {
            refusals.push(Refusal::new(
                RefusalKind::ProfileMissing,
                id,
                format!(
                    "declares no profile; the provider profile {:?} is not one",
                    operation.profile
                ),
            ));
            continue;
        };
        if !is_name(&declaration.name, |c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
        }) {
            refusals.push(Refusal::new(
                RefusalKind::NameInvalid,
                id,
                format!("name {:?} is outside [a-z][a-z0-9_-]*", declaration.name),
            ));
            continue;
        }
        if !profile.requires.secrets.is_empty() || !profile.requires.env.is_empty() {
            refusals.push(Refusal::new(
                RefusalKind::ProfileInvalid,
                id,
                "the profile names a secret or an environment variable under requires; credentials stay in the governed invocation",
            ));
        }
        let mut converter = schema::Converter::new(id, &declaration.name, &mut types);
        let input = converter.root(
            &operation.input_schema,
            "input",
            &declaration.omit,
            &mut refusals,
        );
        let output = converter.root(&operation.output_schema, "output", &[], &mut refusals);
        let mut outcomes = vec![
            Outcome {
                name: "ok".into(),
                summary: Some("The adapter answered; the output is its result.".into()),
                error: None,
            },
            Outcome {
                name: "failed".into(),
                summary: Some(
                    "The governed invocation refused the call or the provider failed it.".into(),
                ),
                error: Some(FAILED.into()),
            },
        ];
        if !declaration.omit.is_empty() {
            any_omit = true;
            outcomes.push(Outcome {
                name: "field-not-allowed".into(),
                summary: Some("The input carries a field the connection fills.".into()),
                error: Some(FIELD_NOT_ALLOWED.into()),
            });
        }
        let name = format!("{family}.{}", declaration.name);
        operations.push(Operation {
            name: name.clone(),
            summary: declaration
                .summary
                .clone()
                .unwrap_or_else(|| operation.description.clone()),
            guidance: declaration.guidance.clone(),
            renderings: BTreeMap::new(),
            input,
            output,
            outcomes,
            profile,
        });
        published.push(Published {
            name,
            operation: id.to_owned(),
            omit: declaration.omit.clone(),
        });
    }
    types.push(TypeDecl {
        name: ERROR_CODE.into(),
        kind: TypeKind::Enum,
        summary: Some("The service error code of a refused or failed invocation.".into()),
        variants: error_codes(),
        fields: Vec::new(),
        ty: None,
    });
    let mut errors = vec![ErrorDecl {
        name: FAILED.into(),
        summary: Some(
            "The governed invocation refused the call, or the provider failed it; code says which."
                .into(),
        ),
        fields: vec![Field {
            name: "code".into(),
            ty: ERROR_CODE.into(),
            description: None,
        }],
    }];
    if any_omit {
        errors.push(ErrorDecl {
            name: FIELD_NOT_ALLOWED.into(),
            summary: Some("The input carries a field the connection fills.".into()),
            fields: vec![Field {
                name: "field".into(),
                ty: "String".into(),
                description: None,
            }],
        });
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let contract = Contract {
        format: "tool-contract/1".into(),
        toolset: family,
        family: None,
        version: VERSION.into(),
        summary: declarations.summary.clone(),
        types,
        errors,
        operations,
    };
    if let Err(violations) = tool_contract::validate(&contract) {
        return Err(violations
            .into_iter()
            .map(|violation| {
                let kind = match violation.property.as_str() {
                    "risk" => RefusalKind::RiskBelowFloor,
                    "name" => RefusalKind::NameInvalid,
                    "types" | "input" | "output" => RefusalKind::SchemaUnrepresentable,
                    _ => RefusalKind::ProfileInvalid,
                };
                Refusal::new(
                    kind,
                    &violation.operation,
                    format!("{}: {}", violation.property, violation.message),
                )
            })
            .collect());
    }
    Ok(Publication {
        contract,
        operations: published,
    })
}

/// The committed bytes of a published contract.
///
/// # Errors
///
/// The contract does not serialize.
pub fn to_yaml(contract: &Contract, source: &str) -> Result<String, String> {
    let body = serde_yaml_ng::to_string(contract).map_err(|e| e.to_string())?;
    Ok(format!(
        "# Generated by connectors-tool-contracts from {source}; edit the declarations, never this file.\n{body}"
    ))
}

/// The SHA-256 of a contract's committed bytes, as lowercase hex.
pub fn digest(yaml: &str) -> String {
    tool_contract::digest::sha256_hex(yaml.as_bytes())
}

/// The contract `adapters/<adapter>/generated/tool-contract.yaml` holds, from the adapter's
/// generated descriptor and its declarations.
///
/// # Errors
///
/// A file does not read or parse, or the publication is refused (one line per refusal).
pub fn generate(root: &Path, adapter: &str) -> Result<String, String> {
    let descriptor_path = format!("adapters/{adapter}/generated/descriptor.json");
    let declarations_path = format!("adapters/{adapter}/spec/tool-profiles.json");
    let read =
        |path: &str| std::fs::read(root.join(path)).map_err(|error| format!("{path}: {error}"));
    let descriptor: Descriptor = serde_json::from_slice(&read(&descriptor_path)?)
        .map_err(|error| format!("{descriptor_path}: {error}"))?;
    let declarations = Declarations::from_json(&read(&declarations_path)?)
        .map_err(|error| format!("{declarations_path}: {error}"))?;
    let publication = publish(&descriptor, &declarations).map_err(|refusals| {
        refusals
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    to_yaml(
        &publication.contract,
        &format!("{descriptor_path} and {declarations_path}"),
    )
}

/// The committed path of an adapter's contract.
pub fn generated_path(adapter: &str) -> String {
    format!("adapters/{adapter}/generated/tool-contract.yaml")
}
