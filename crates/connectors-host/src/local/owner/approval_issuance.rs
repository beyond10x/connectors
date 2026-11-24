//! Trusted local approval coordination. Only metadata is used to prepare a
//! subject. Issuance owns the exact policy/key leases and protected publication;
//! neither path starts an owner/adapter or authorizes a business dispatch.
use super::{Argument, Code, Error, Result, selected};
use crate::local::{
    approval_keys, approval_policy, approvals, clock,
    config::{Adapter, Config, Paths},
    filesystem,
    mutations::ClockInterval,
    protected, registry, runtime, standing,
};
use connectors_sdk::Secret;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub const REQUEST_LIMIT: usize = 272 * 1024;
pub const TARGET_LIMIT: usize = 256 * 1024;
const RESULT_LIMIT: usize = 64 * 1024;
#[cfg(test)]
mod tests;

pub(super) fn check(until: Instant) -> Result<()> {
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

pub(super) fn policy_store(paths: &Paths, adapter: &Adapter) -> Result<approval_policy::Store> {
    approval_policy::Store::new(&paths.state, &adapter.instance_id, &adapter.adapter_id)
        .map_err(policy_error)
}
pub(super) fn key_store(
    paths: &Paths,
    config: &Config,
    adapter: &Adapter,
) -> Result<approval_keys::Store> {
    approval_keys::Store::for_selection(
        &paths.state,
        &config.selection(),
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
    let mut bootstrap = runtime::state::State::new(&paths.state)
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
    bootstrap.configure(adapter);
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
        clock_configuration_sha256: {
            let selected = config
                .approval_clock
                .as_ref()
                .ok_or(Code::InvalidConfiguration)?;
            selected
                .configuration()
                .map_err(|failure| Error::clock(selected, failure))?
                .sha256()
        },
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
    let document_refused = || Error::invalid_input(Argument::InputFile);
    if document.len() > REQUEST_LIMIT {
        return Err(document_refused());
    }
    runtime::channel::depth(document).map_err(|_| document_refused())?;
    let input: PolicyInput =
        connectors_core::read_json(document).map_err(|_| document_refused())?;
    // A policy revision is a positive JSON-safe integer (approval_policy.rs).
    if expected_revision.is_some_and(|revision| !(1..=(1 << 53) - 1).contains(&revision)) {
        return Err(Error::invalid_input(Argument::ExpectedRevision));
    }
    let (config, adapter) = selected(paths, alias)?;
    let bootstrap = snapshot(paths, &adapter)?;
    let selection = selection(&config, &adapter, &bootstrap)?;
    for operation in &input.operations {
        required_write(&adapter, &bootstrap, operation)?;
    }
    check(until)?;
    // What the store refuses as input past the revision is the operation
    // list: too many, or one named twice.
    let result = policy_store(paths, &adapter)?
        .set_admitted(&selection, input.operations, expected_revision)
        .map_err(|failure| match policy_error(failure) {
            error if error.code == Code::InvalidInput => document_refused(),
            error => error,
        })?;
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

pub(super) struct Resolved {
    pub config: Config,
    pub adapter: Adapter,
    pub policy: approval_policy::View,
    pub preparation: Preparation,
}
pub(super) fn resolve(
    paths: &Paths,
    alias: &str,
    request: &Request<'_>,
    until: Instant,
) -> Result<Resolved> {
    check(until)?;
    // Each refusal names the argument it refused, as the CLI input model
    // names it (issue #180: `issue` named none).
    let input_refused = || Error::invalid_input(Argument::Input);
    if request.input.len() > TARGET_LIMIT {
        return Err(input_refused());
    }
    for (field, value) in [
        (Argument::Connection, request.connection),
        (Argument::Operation, request.operation),
        (Argument::Schema, request.schema),
        (Argument::Revision, request.revision),
    ] {
        if value.is_empty() || value.len() > 256 || !connectors_core::valid_id(value) {
            return Err(Error::invalid_input(field));
        }
    }
    runtime::channel::depth(request.input.as_bytes()).map_err(|_| input_refused())?;
    let input: Value =
        connectors_core::read_json(request.input.as_bytes()).map_err(|_| input_refused())?;
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
        return Err(input_refused());
    }
    let (config, adapter) = selected(paths, alias)?;
    let bootstrap = snapshot(paths, &adapter)?;
    let descriptor = bootstrap.descriptor()?;
    super::admit_operation(&adapter, &bootstrap, request.operation)?;
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
        .map_err(|_| input_refused())?;
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
    // Serializing values the host built is no argument's fault: like the
    // protected document in `issue`, a failure there is `unavailable`.
    let subject_sha256 =
        connectors_core::digest(&serde_json::to_value(&subject).map_err(|_| Code::Unavailable)?);
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
        .map_err(|_| Code::Unavailable)?
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
    // A new file, named by an absolute plain path, in an existing directory
    // only its owner can enter: anything else refuses naming the argument.
    let output_refused = || Error::invalid_input(Argument::ProofOutput);
    filesystem::validate_path(output).map_err(|_| output_refused())?;
    let parent = filesystem::directory(output.parent().ok_or_else(output_refused)?, false, true)
        .map_err(|_| output_refused())?;
    let name = output.file_name().ok_or_else(output_refused)?;
    filesystem::require_absent(&parent, name).map_err(|_| output_refused())?;
    check(until)?;
    // Network acquisition must precede every policy/key/custody lease. A
    // below-quorum attempt is made once more while one query deadline and
    // the reserve for the work below are left (`clock::acquire_within`).
    let selected = first
        .config
        .approval_clock
        .as_ref()
        .ok_or(Code::InvalidConfiguration)?;
    let clock = selected
        .acquire_within(until, clock::RESERVE_AFTER_CLOCK)
        .map_err(|refused| Error::clock(selected, refused.failure))?;
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

pub(super) struct Admission<'a> {
    pub policy: &'a approval_policy::PolicyUse,
    pub key: &'a approval_keys::KeyUse,
    pub subject: &'a approvals::Subject,
}
pub(super) struct Guard<'a>(&'a approvals::ConfiguredApprovalKey);
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
pub(super) fn policy_error(failure: approval_policy::Failure) -> Error {
    use approval_policy::Failure::*;
    let mut error: Error = match failure {
        InvalidInput => Code::InvalidInput,
        Conflict | BindingChanged => Code::LifecycleConflict,
        NotFound => Code::NotFound,
        NotAdmitted => Code::Forbidden,
        MetadataUnavailable => Code::MetadataUnavailable,
        MetadataSchemaNewer(store) => return Error::schema_newer(store),
        RevisionConflict => Code::RevisionConflict,
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into();
    // The instance's recorded configuration revision is not the configured
    // one: a new connection under the configured revision moves it.
    error.reconnect = failure == BindingChanged;
    error
}
pub(super) fn key_error(failure: approval_keys::Failure) -> Error {
    use approval_keys::Failure::*;
    let mut error: Error = match failure {
        InvalidInput => Code::InvalidInput,
        Conflict | BindingChanged => Code::LifecycleConflict,
        NotFound => Code::NotFound,
        MetadataUnavailable => Code::MetadataUnavailable,
        RevisionConflict => Code::RevisionConflict,
        CustodyUnavailable => Code::CustodyUnavailable,
        CustodyReadOnly => Code::CustodyReadOnly,
        MetadataSchemaNewer(store) => return Error::schema_newer(store),
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into();
    // The instance's recorded configuration revision is not the configured
    // one: a new connection under the configured revision moves it.
    error.reconnect = failure == BindingChanged;
    error
}
fn proof_error(error: approvals::Failure) -> Error {
    use approvals::Failure::*;
    match error {
        Refused | Replayed => Code::Forbidden,
        Unavailable => Code::Unavailable,
        MetadataUnavailable => Code::MetadataUnavailable,
        MetadataSchemaNewer(store) => return Error::schema_newer(store),
        OutcomeUnknown => Code::OutcomeUnknown,
        Capacity => Code::Capacity,
    }
    .into()
}

// The three standing-grant commands
// (story:local-invoke-admits-a-keyed-write-under-a-standing-grant;
// contracts/cli/v1alpha1/semantics.md, "Standing grants"). They run in the
// CLI process and start no owner. Order: the instance's `standing_grants`
// member, the clock (before every lock and lease, as `issue` reads it), then
// `standing-grants.lock`, then the shared key lease.

/// What `approvals grant-standing` and `approvals revoke-standing` print.
pub struct StandingFile {
    pub grant_id: String,
    pub path: PathBuf,
}

fn standing_error(refused: standing::Refused) -> Error {
    use standing::Refused::*;
    match refused {
        InvalidInput => Error::invalid_input(Argument::InputFile),
        InvalidConfiguration => Code::InvalidConfiguration.into(),
        NotFound => Code::NotFound.into(),
        Unavailable => Code::Unavailable.into(),
        Forbidden => Code::Forbidden.into(),
        OutcomeUnknown => Code::OutcomeUnknown.into(),
    }
}

/// The grant file the instance's entry names and one clock reading. An entry
/// without `standing_grants` takes no grant: `invalid_configuration`, before
/// the clock is read. No `[approval_clock]` is `invalid_configuration` and a
/// clock that forms no reading is `unavailable`, as for `issue`.
fn standing_reading(
    config: &Config,
    adapter: &Adapter,
    until: Instant,
) -> Result<(PathBuf, ClockInterval)> {
    let path = adapter
        .standing_grants
        .clone()
        .ok_or(Code::InvalidConfiguration)?;
    check(until)?;
    let selected = config
        .approval_clock
        .as_ref()
        .ok_or(Code::InvalidConfiguration)?;
    let clock = selected
        .acquire_within(until, clock::RESERVE_AFTER_CLOCK)
        .map_err(|refused| Error::clock(selected, refused.failure))?;
    let now = standing::reading(&clock).ok_or(Code::Unavailable)?;
    check(until)?;
    Ok((path, now))
}

/// `approvals grant-standing`: adds the grant `request` describes to the
/// instance's signed grant file and returns its id and the file's path. A
/// refused `request` is `invalid_input` naming `input_file`, decided before
/// the member and the clock where it does not depend on them.
pub fn grant_standing(
    paths: &Paths,
    alias: &str,
    request: &[u8],
    until: Instant,
) -> Result<StandingFile> {
    check(until)?;
    let (config, adapter) = selected(paths, alias)?;
    standing::check_request(request, &adapter.instance_id).map_err(standing_error)?;
    let (path, now) = standing_reading(&config, &adapter, until)?;
    let lock = standing::lock(&paths.state).map_err(key_error)?;
    let key = key_store(paths, &config, &adapter)?
        .acquire_key()
        .map_err(key_error)?;
    check(until)?;
    let grant_id = key
        .with_signer(|signer| {
            Ok(standing::grant(
                &lock,
                &path,
                signer,
                key.key()?,
                &adapter.instance_id,
                now,
                request,
            ))
        })
        .map_err(key_error)?
        .map_err(standing_error)?;
    Ok(StandingFile { grant_id, path })
}

/// `approvals revoke-standing`: removes the grant `grant_id` from the
/// instance's signed grant file.
pub fn revoke_standing(
    paths: &Paths,
    alias: &str,
    grant_id: &str,
    until: Instant,
) -> Result<StandingFile> {
    check(until)?;
    let (config, adapter) = selected(paths, alias)?;
    let (path, now) = standing_reading(&config, &adapter, until)?;
    let lock = standing::lock(&paths.state).map_err(key_error)?;
    let key = key_store(paths, &config, &adapter)?
        .acquire_key()
        .map_err(key_error)?;
    check(until)?;
    key.with_signer(|signer| {
        Ok(standing::revoke(
            &lock,
            &path,
            signer,
            key.key()?,
            now,
            grant_id,
        ))
    })
    .map_err(key_error)?
    .map_err(standing_error)?;
    Ok(StandingFile {
        grant_id: grant_id.to_owned(),
        path,
    })
}

/// `approvals list-standing`: the grants of the instance's verified grant
/// file in file order, expired ones included. It signs and writes nothing.
pub fn list_standing(paths: &Paths, alias: &str, until: Instant) -> Result<Vec<standing::Grant>> {
    check(until)?;
    let (config, adapter) = selected(paths, alias)?;
    let (path, now) = standing_reading(&config, &adapter, until)?;
    let key = key_store(paths, &config, &adapter)?
        .acquire_key()
        .map_err(key_error)?;
    standing::list(&path, key.key().map_err(key_error)?, now).map_err(standing_error)
}
