//! Trusted local approval coordination. Only metadata is used to prepare a
//! subject. Issuance owns the exact policy/key leases and protected publication;
//! neither path starts an owner/adapter or authorizes a business dispatch.
use super::{Code, Error, Result, selected};
use crate::local::{
    approval_keys, approval_policy, approvals,
    config::{Adapter, Config, Paths},
    filesystem, protected, registry, runtime,
};
use connectors_sdk::Secret;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

pub const REQUEST_LIMIT: usize = 272 * 1024;
pub const TARGET_LIMIT: usize = 256 * 1024;
const RESULT_LIMIT: usize = 64 * 1024;
#[cfg(test)]
mod tests;

fn check(until: Instant) -> Result<()> {
    protected::cancellation()?;
    if Instant::now() >= until {
        return Err(Code::Timeout.into());
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyInput {
    operations: Vec<String>,
}

fn policy_store(paths: &Paths, adapter: &Adapter) -> Result<approval_policy::Store> {
    approval_policy::Store::new(&paths.state, &adapter.instance_id, &adapter.adapter_id)
        .map_err(policy_error)
}
fn key_store(paths: &Paths, config: &Config, adapter: &Adapter) -> Result<approval_keys::Store> {
    approval_keys::Store::new(
        &paths.state,
        config.secret_service_socket.as_deref(),
        &adapter.instance_id,
        &adapter.adapter_id,
        &adapter.configuration_revision,
    )
    .map_err(key_error)
}

pub fn policy_status(paths: &Paths, alias: &str) -> Result<Option<approval_policy::View>> {
    let (_, adapter) = selected(paths, alias)?;
    policy_store(paths, &adapter)?
        .status()
        .map_err(policy_error)
}

fn snapshot(paths: &Paths, adapter: &Adapter) -> Result<runtime::Bootstrap> {
    let bootstrap = runtime::state::State::new(&paths.state)
        .cached(&adapter.instance_id, &adapter.selection())?
        .ok_or(Code::DescriptionUnavailable)?;
    bootstrap.validate_for(adapter.private_protocol())?;
    if bootstrap.instance != adapter.instance_id
        || bootstrap.adapter != adapter.adapter_id
        || bootstrap.configuration_revision != adapter.configuration_revision
        || bootstrap.protocol != adapter.protocol
    {
        return Err(Code::ReadinessMismatch.into());
    }
    Ok(bootstrap)
}
fn selection(
    config: &Config,
    adapter: &Adapter,
    bootstrap: &runtime::Bootstrap,
) -> Result<approval_policy::Selection> {
    Ok(approval_policy::Selection {
        configuration_revision: adapter.configuration_revision.clone(),
        descriptor_revision: bootstrap.descriptor()?.revision,
        executable_selection: adapter.selection(),
        clock_configuration_sha256: config
            .approval_clock
            .as_ref()
            .ok_or(Code::InvalidConfiguration)?
            .sha256(),
    })
}
fn required_write<'a>(
    adapter: &Adapter,
    bootstrap: &'a runtime::Bootstrap,
    operation: &str,
) -> Result<&'a runtime::Requirement> {
    let required = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Code::NotFound)?;
    if !adapter.permissions.operations.contains(operation)
        || !adapter.permissions.profiles.contains(&required.profile)
    {
        return Err(Code::Forbidden.into());
    }
    // The selected private/2 write profile requires approval. Other effect
    // modes cannot acquire this local policy by presenting an operation name.
    if adapter.private_protocol() != runtime::PrivateProtocol::V2
        || required.effect != runtime::Effect::Write
    {
        return Err(Code::Unsupported.into());
    }
    Ok(required)
}

pub fn policy_set(
    paths: &Paths,
    alias: &str,
    document: &[u8],
    expected_revision: Option<i64>,
    until: Instant,
) -> Result<approval_policy::View> {
    check(until)?;
    if document.len() > REQUEST_LIMIT {
        return Err(Code::InvalidInput.into());
    }
    runtime::channel::depth(document).map_err(|_| Code::InvalidInput)?;
    let input: PolicyInput =
        connectors_core::read_json(document).map_err(|_| Code::InvalidInput)?;
    let (config, adapter) = selected(paths, alias)?;
    let bootstrap = snapshot(paths, &adapter)?;
    let selection = selection(&config, &adapter, &bootstrap)?;
    for operation in &input.operations {
        required_write(&adapter, &bootstrap, operation)?;
    }
    check(until)?;
    let result = policy_store(paths, &adapter)?
        .set_admitted(&selection, input.operations, expected_revision)
        .map_err(policy_error)?;
    // A committed update cannot become a definite pre-publication refusal.
    check(until).map_err(|_| Code::OutcomeUnknown)?;
    Ok(result)
}

/// Caller supplies target selectors and the original document, never authority.
#[derive(Serialize)]
pub struct Request<'a> {
    pub connection: &'a str,
    pub operation: &'a str,
    pub schema: &'a str,
    pub revision: &'a str,
    pub input: &'a str,
}
#[derive(Clone, Debug, Serialize)]
pub struct Preparation {
    pub subject: approvals::Subject,
    pub subject_sha256: String,
    pub presentation: Presentation,
    pub approval_policy: ApprovalPolicy,
}
#[derive(Clone, Debug, Serialize)]
pub struct Presentation {
    pub instance: String,
    pub operation: String,
    pub connection: Option<String>,
    pub descriptor_revision: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ApprovalPolicy {
    pub mode: &'static str,
    pub issuer: Option<String>,
    pub audience: Option<String>,
    pub max_lifetime_seconds: Option<u16>,
}

struct Resolved {
    config: Config,
    adapter: Adapter,
    policy: approval_policy::View,
    preparation: Preparation,
}
fn resolve(paths: &Paths, alias: &str, request: &Request<'_>, until: Instant) -> Result<Resolved> {
    check(until)?;
    if request.input.len() > TARGET_LIMIT
        || [
            &request.connection,
            &request.operation,
            &request.schema,
            &request.revision,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > 256 || !connectors_core::valid_id(s))
    {
        return Err(Code::InvalidInput.into());
    }
    runtime::channel::depth(request.input.as_bytes()).map_err(|_| Code::InvalidInput)?;
    let input: Value =
        connectors_core::read_json(request.input.as_bytes()).map_err(|_| Code::InvalidInput)?;
    // Independently bound both projected envelopes, not merely the input text.
    // The local helper has no public wire request id, executor or route.
    let target = json!({"connection":request.connection,"operation":request.operation,
        "revision":request.revision,"input":input});
    let helper = json!({"adapter":alias,"connection":request.connection,
        "operation":request.operation,"schema":request.schema,"revision":request.revision,
        "input":input});
    if connectors_core::canonical(&target).len() > TARGET_LIMIT
        || connectors_core::canonical(&helper).len() > REQUEST_LIMIT
    {
        return Err(Code::InvalidInput.into());
    }
    let (config, adapter) = selected(paths, alias)?;
    let bootstrap = snapshot(paths, &adapter)?;
    let descriptor = bootstrap.descriptor()?;
    if descriptor.revision != request.revision
        || super::schema(&bootstrap, request.operation)? != request.schema
    {
        return Err(Code::StaleDescription.into());
    }
    let required = required_write(&adapter, &bootstrap, request.operation)?;
    let operation = descriptor
        .operation(request.operation)
        .map_err(|_| Code::NotFound)?;
    connectors_sdk::validate_write_value(&operation.input_schema, &input)
        .map_err(|_| Code::InvalidInput)?;
    let connection_revision = registry::Registry::new(&paths.state).approval_target(
        &bootstrap.binding(&required.profile)?,
        request.connection,
        &required.scopes,
    )?;
    let selection = selection(&config, &adapter, &bootstrap)?;
    let policy = policy_store(paths, &adapter)?
        .status()
        .map_err(policy_error)?
        .ok_or(Code::Forbidden)?;
    if policy.selection != selection || !policy.operations.iter().any(|op| op == request.operation)
    {
        return Err(Code::Forbidden.into());
    }
    let issuer = key_store(paths, &config, &adapter)?
        .status()
        .map_err(key_error)?
        .ok_or(Code::Forbidden)?;
    if issuer.issuer != format!("connectors.local-issuer/{}", policy.issuer_id)
        || issuer.audience != format!("connectors.approval/{}", policy.issuer_id)
    {
        return Err(Code::Forbidden.into());
    }
    let subject = approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: descriptor.instance.clone(),
            operation: operation.id.clone(),
            connection: request.connection.into(),
            connection_revision,
            contract: operation.contract.clone(),
            profile: operation.profile.clone(),
            descriptor_revision: descriptor.revision.clone(),
            configuration_revision: adapter.configuration_revision.clone(),
        },
        authority: approvals::Authority {
            scope: approvals::Scope {
                tenant: None,
                realm: None,
                caller: policy.principal(),
                executor: None,
            },
            current_authority: Some(policy.snapshot().map_err(policy_error)?),
            executor: None,
        },
        origin: approvals::Origin {
            kind: approvals::OriginKind::Direct,
            authority_ref: descriptor.instance.clone(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: connectors_core::digest(&input),
        approval_mode: "required".into(),
    };
    subject.canonical_bytes().map_err(proof_error)?;
    let subject_sha256 =
        connectors_core::digest(&serde_json::to_value(&subject).map_err(|_| Code::InvalidInput)?);
    let preparation = Preparation {
        subject,
        subject_sha256,
        presentation: Presentation {
            instance: descriptor.instance.clone(),
            operation: operation.id.clone(),
            connection: Some(request.connection.into()),
            descriptor_revision: descriptor.revision,
        },
        approval_policy: ApprovalPolicy {
            mode: "required",
            issuer: Some(issuer.issuer),
            audience: Some(issuer.audience),
            max_lifetime_seconds: Some(300),
        },
    };
    if serde_json::to_vec(&json!({"adapter":alias,"preparation":preparation}))
        .map_err(|_| Code::InvalidInput)?
        .len()
        > RESULT_LIMIT
    {
        return Err(Code::Capacity.into());
    }
    check(until)?;
    Ok(Resolved {
        config,
        adapter,
        policy,
        preparation,
    })
}

pub fn prepare(
    paths: &Paths,
    alias: &str,
    request: &Request<'_>,
    until: Instant,
) -> Result<Preparation> {
    Ok(resolve(paths, alias, request, until)?.preparation)
}

/// Safe acknowledgement only. Proof bytes have a separate protected owner.
#[derive(Serialize)]
pub struct Publication {
    pub reference: String,
    pub subject_sha256: String,
    pub disposition: &'static str,
}

pub fn issue(
    paths: &Paths,
    alias: &str,
    request: &Request<'_>,
    approved: &str,
    output: &Path,
    until: Instant,
) -> Result<Publication> {
    let first = resolve(paths, alias, request, until)?;
    if first.preparation.subject_sha256 != approved {
        return Err(Code::Forbidden.into());
    }
    filesystem::validate_path(output).map_err(|_| Code::InvalidInput)?;
    let parent = filesystem::directory(output.parent().ok_or(Code::InvalidInput)?, false, true)
        .map_err(|_| Code::InvalidInput)?;
    let name = output.file_name().ok_or(Code::InvalidInput)?;
    filesystem::require_absent(&parent, name).map_err(|_| Code::InvalidInput)?;
    check(until)?;
    // Network acquisition must precede every policy/key/custody lease.
    let clock = first
        .config
        .approval_clock
        .as_ref()
        .ok_or(Code::InvalidConfiguration)?
        .acquire()
        .map_err(|_| Code::Unavailable)?;
    let current = resolve(paths, alias, request, until)?;
    if current.preparation.subject != first.preparation.subject
        || current.policy.selection != first.policy.selection
    {
        return Err(Code::Forbidden.into());
    }
    let policy = policy_store(paths, &current.adapter)?
        .acquire(&current.policy.selection, request.operation)
        .map_err(policy_error)?;
    if policy.policy().map_err(policy_error)? != &current.policy {
        return Err(Code::Forbidden.into());
    }
    let key = key_store(paths, &current.config, &current.adapter)?
        .acquire_key()
        .map_err(key_error)?;
    let admitted = Admission {
        policy: &policy,
        key: &key,
        subject: &current.preparation.subject,
    };
    admitted
        .current(
            &current.preparation.subject,
            &key.key().map_err(key_error)?.kid,
        )
        .map_err(proof_error)?;
    check(until)?;
    // The callback returns protected Evidence; it never puts compact proof
    // bytes into a Value or String. Keep both leases through publication/ack.
    let evidence = key
        .with_signer(|signer| {
            Ok(signer
                .issue(&current.preparation.subject, &admitted, &clock)
                .map_err(proof_error))
        })
        .map_err(key_error)??;
    check(until)?;
    approvals::verify(&evidence, &current.preparation.subject, &admitted, &clock)
        .map_err(proof_error)?;
    #[derive(Serialize)]
    struct ProtectedDocument<'a> {
        reference: &'a str,
        evidence: &'a str,
    }
    let mut bytes = Secret(Vec::new());
    serde_json::to_writer(
        &mut bytes.0,
        &ProtectedDocument {
            reference: evidence.reference(),
            evidence: std::str::from_utf8(evidence.bytes()).map_err(|_| Code::Unavailable)?,
        },
    )
    .map_err(|_| Code::Unavailable)?;
    publish_checked(&parent, name, &bytes, || {
        check(until)?;
        approvals::verify(&evidence, &current.preparation.subject, &admitted, &clock)
            .map_err(proof_error)?;
        Ok(())
    })?;
    Ok(Publication {
        reference: evidence.reference().into(),
        subject_sha256: approved.into(),
        disposition: "published",
    })
}

fn publish_checked(
    parent: &std::fs::File,
    name: &std::ffi::OsStr,
    bytes: &Secret,
    mut current: impl FnMut() -> Result<()>,
) -> Result<()> {
    current()?;
    filesystem::publish_new(parent, name, &bytes.0)?;
    // A failed acknowledgement check cannot turn a published proof into a
    // definite non-publication or authorize cleanup, overwrite or reissuance.
    current().map_err(|_| Code::OutcomeUnknown.into())
}

struct Admission<'a> {
    policy: &'a approval_policy::PolicyUse,
    key: &'a approval_keys::KeyUse,
    subject: &'a approvals::Subject,
}
struct Guard<'a>(&'a approvals::ConfiguredApprovalKey);
impl approvals::CurrentAdmission for Guard<'_> {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        self.0
    }
}
impl Admission<'_> {
    fn current<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<Guard<'a>> {
        let policy = self
            .policy
            .policy()
            .map_err(|_| approvals::Failure::Refused)?;
        let key = self.key.key().map_err(|_| approvals::Failure::Refused)?;
        if subject != self.subject
            || key.kid != kid
            || key.revoked
            || key.issuer != format!("connectors.local-issuer/{}", policy.issuer_id)
            || key.audience != format!("connectors.approval/{}", policy.issuer_id)
            || subject.authority.scope.caller != policy.principal()
            || subject.authority.current_authority.as_ref()
                != Some(&policy.snapshot().map_err(|_| approvals::Failure::Refused)?)
            || !policy.operations.contains(&subject.target.operation)
        {
            return Err(approvals::Failure::Refused);
        }
        Ok(Guard(key))
    }
}
impl approvals::IssuancePolicy for Admission<'_> {
    type Guard<'a>
        = Guard<'a>
    where
        Self: 'a;
    fn authorize<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<Guard<'a>> {
        self.current(subject, kid)
    }
}
impl approvals::ReceiverPolicy for Admission<'_> {
    type Guard<'a>
        = Guard<'a>
    where
        Self: 'a;
    fn admit<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<Guard<'a>> {
        self.current(subject, kid)
    }
}
fn policy_error(error: approval_policy::Failure) -> Error {
    use approval_policy::Failure::*;
    match error {
        InvalidInput => Code::InvalidInput,
        Conflict => Code::LifecycleConflict,
        NotFound => Code::NotFound,
        NotAdmitted => Code::Forbidden,
        MetadataUnavailable => Code::MetadataUnavailable,
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into()
}
fn key_error(error: approval_keys::Failure) -> Error {
    use approval_keys::Failure::*;
    match error {
        InvalidInput => Code::InvalidInput,
        Conflict => Code::LifecycleConflict,
        NotFound => Code::NotFound,
        MetadataUnavailable => Code::MetadataUnavailable,
        CustodyUnavailable => Code::CustodyUnavailable,
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into()
}
fn proof_error(error: approvals::Failure) -> Error {
    use approvals::Failure::*;
    match error {
        Refused | Replayed => Code::Forbidden,
        Unavailable => Code::Unavailable,
        MetadataUnavailable => Code::MetadataUnavailable,
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into()
}
