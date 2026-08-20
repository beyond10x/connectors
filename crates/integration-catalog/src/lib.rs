#![forbid(unsafe_code)]

//! **One adapter for every declared provider.**
//!
//! # Why this exists
//!
//! GitLab is fourteen catalogued operations with complete HTTP request templates, five declared
//! credential mechanisms, a declared verify probe, and an authority to address its credential by.
//! It also had a 2,875-line hand-written Rust backend. So did Jira. So, in their own shapes, did
//! Slack, Grafana and Kubernetes — and each grew its own copy of credential handling, its own
//! dispatch, and its own idea of what an error is. Timo, on being shown the GitLab one:
//! *"WTF is there a backend for gitlab, it's just HTTP."*
//!
//! He is right, and the pieces to prove it were already committed:
//!
//! | already existed | what it does |
//! |---|---|
//! | [`catalog`] | the reviewed facts: operations, credentials, config fields, risk, direction |
//! | [`connector_resolve::document`] | the request template, embedded in the binary |
//! | [`connector_resolve::assemble_credentials`] | store → placed credential, per declared mechanism |
//! | [`connector_resolve::resolve`] | template + input + credential → a finished request |
//! | `service::plan_operation` | the zero-I/O admission plan |
//! | `server::egress` | execution inside a fixed destination aperture |
//!
//! Every one of those is provider-neutral. Nobody had joined them up, so each Integration joined
//! them up again privately. This crate is the join, written once.
//!
//! # What it deliberately does not do
//!
//! **Acquisition.** A credential arrives here already stored; this crate reads it through
//! [`SecretStore`] at the address the resolver derives and has no idea how it got there. That is
//! the seam that lets a pasted token, an imported file and — later — an OAuth2 authorization code
//! be three producers into one port, with no execution code changing. Auth is separated from
//! execution, which is the point of the platform.
//!
//! **Datasources and events.** This slice serves operations. A datasource is a projection with its
//! own scaffolding, which currently exists in three drifting copies across the Integration crates
//! and should be extracted before a fourth is written.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use catalog::{HostEffect, OperationDirection, Risk};
use connector_address::CredentialRef;
use connector_secrets::{Secret, SecretStore};
use connectors_config::{CatalogIntegrationConfig, InitiationConfig};
use domain::InitiationPolicy;
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, EffectClass, InvocationResult, OperationDescription,
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OperationSummary,
};
use service::{
    BackendCapabilities, ConnectorBackend, EgressHttpRequest, EgressTransport, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

mod config;
pub use config::DeclaredConfig;

/// The most caller input one operation may carry. A declared operation's input is a small JSON
/// object of catalogue-declared fields; anything larger is a caller mistake, not a payload.
const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CatalogIntegrationError {
    #[error("provider `{0}` is not in the catalogue")]
    UnknownProvider(String),
    #[error("provider `{0}` declares no authority, so its credential has no address")]
    NoAuthority(String),
    #[error("provider `{0}` declares no credential named `{1}`")]
    UnknownCredential(String, String),
    #[error("the credential file for `{0}` is not an owner-only regular file")]
    UnsafeCredentialFile(String),
    #[error("the credential file for `{0}` could not be read")]
    UnreadableCredentialFile(String),
    #[error("the credential for `{0}` could not be stored")]
    CredentialStore(String),
    #[error("provider `{0}` needs a configuration value its base URL declares: `{1}`")]
    MissingEndpointValue(String, String),
    #[error("the state root is not usable")]
    StateRoot,
}

/// One configured provider connection, resolved from the catalogue at composition time.
struct Binding {
    provider: &'static catalog::Provider,
    connection_ref: String,
    label: String,
    /// The Grant this Connection's authority derives from.
    ///
    /// Carried rather than consumed in this slice: the ceiling is enforced from declared risk and
    /// effects (see [`Binding::admits`]), and the reference is what an audit record names when
    /// that decision is written down. Kept on the binding so the audit seam has it to hand rather
    /// than having to re-derive which grant admitted a call after the fact.
    #[allow(dead_code)]
    grant_ref: String,
    initiation: InitiationPolicy,
    config: DeclaredConfig,
    /// The grant ceiling. False admits reads with no declared host effect and nothing else.
    allow_writes: bool,
}

impl Binding {
    /// Whether this connection's grant admits `operation`.
    ///
    /// Read from what the catalogue *declares* — direction, risk, host effects — never from a list
    /// of operation ids someone maintains by hand. That is the vision's second principle, and it is
    /// what makes the ceiling hold for an operation added after the grant was written.
    ///
    /// # The effects test, and the mistake it replaces
    ///
    /// `effects` is the **host** vocabulary — what executing this operation makes *this machine*
    /// do — not a statement about vendor state. Every HTTP operation therefore declares
    /// `[read, network]` at minimum, and an earlier version of this predicate asked for
    /// `effects.is_empty()`. That admitted nothing at all: a read-only grant refused all fourteen
    /// GitLab reads, because reaching GitLab is a network effect. Caught by the test below, which
    /// asserts a read-only grant still admits something rather than only that it refuses writes.
    ///
    /// So the test is not emptiness but **escalation**: `Read` and `Network` are what any declared
    /// HTTP call already needs, and anything further — `Write`, `Process`, `Browser`, `Filesystem`,
    /// `LocalSystem` — is authority this grant did not give. Direction remains the vendor-state
    /// axis and is checked separately, so an operation that reads vendor state while writing the
    /// local filesystem is still refused.
    fn admits(&self, operation: &catalog::Operation) -> bool {
        if self.allow_writes {
            return true;
        }
        matches!(operation.direction, OperationDirection::Read)
            && !matches!(operation.risk, Risk::Destructive)
            && operation
                .effects
                .iter()
                .all(|effect| matches!(effect, HostEffect::Read | HostEffect::Network))
    }
}

/// A description a caller read, which invocation then requires.
///
/// Invocation takes a `description_ref` rather than trusting the caller's memory of an operation:
/// the lease says *this caller has seen this operation's current shape against this connection*.
/// Without it a caller could invoke an operation whose input schema or risk had changed since it
/// last looked.
struct Lease {
    operation_ref: String,
    connection_ref: String,
}

/// The generic Integration.
pub struct CatalogBackend {
    inner: Arc<Inner>,
}

struct Inner {
    owner: PrincipalContext,
    bindings: Vec<Binding>,
    secrets: Arc<dyn SecretStore>,
    egress: Arc<dyn EgressTransport>,
    leases: Mutex<BTreeMap<String, Lease>>,
}

impl CatalogBackend {
    /// Bind every configured provider, importing any declared credential file exactly once.
    ///
    /// # Errors
    ///
    /// A provider not in the catalogue, one without an authority to address a credential by, an
    /// unsafe credential file, or a store that refused the value.
    pub async fn open(
        owner: PrincipalContext,
        configured: &[CatalogIntegrationConfig],
        state_root: &Path,
        secrets: Arc<dyn SecretStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, CatalogIntegrationError> {
        ensure_owner_directory(state_root)?;
        let mut bindings = Vec::with_capacity(configured.len());
        for entry in configured {
            let provider = catalog::provider(catalog::ProviderKey::id(&entry.provider))
                .ok_or_else(|| CatalogIntegrationError::UnknownProvider(entry.provider.clone()))?;
            let authority = provider
                .authority
                .ok_or_else(|| CatalogIntegrationError::NoAuthority(entry.provider.clone()))?;

            if let Some(path) = entry.credential_file.as_ref() {
                let leaf = credential_leaf(provider, entry.credential.as_deref())?;
                let reference = CredentialRef::new(
                    owner.tenant_id(),
                    authority,
                    connector_address::DEFAULT_SERVICE,
                    leaf,
                )
                .map_err(|_| CatalogIntegrationError::NoAuthority(entry.provider.clone()))?;
                import_credential(&entry.provider, path, &reference, secrets.as_ref()).await?;
            }

            bindings.push(Binding {
                provider,
                connection_ref: connection_ref(&entry.provider, entry.name()),
                label: entry.label(),
                grant_ref: entry.grant_ref.clone(),
                initiation: match entry.initiation {
                    InitiationConfig::B10x => InitiationPolicy::b10x_only(),
                    InitiationConfig::Provider => InitiationPolicy::provider_only(),
                    InitiationConfig::Both => InitiationPolicy::bidirectional(),
                },
                config: DeclaredConfig::new(entry.endpoints.clone(), entry.operator_approved),
                allow_writes: entry.allow_writes,
            });
        }
        Ok(Self {
            inner: Arc::new(Inner {
                owner,
                bindings,
                secrets,
                egress,
                leases: Mutex::new(BTreeMap::new()),
            }),
        })
    }

    /// How many provider connections this adapter published.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.inner.bindings.len()
    }
}

impl Inner {
    fn binding_for_operation(&self, operation: &catalog::Operation) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|binding| binding.provider.id == operation.provider)
    }

    fn binding_by_ref(&self, connection_ref: &str) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|binding| binding.connection_ref == connection_ref)
    }

    fn summary(&self, binding: &Binding) -> ConnectionSummary {
        ConnectionSummary {
            connection_ref: binding.connection_ref.clone(),
            label: binding.label.clone(),
            provider: binding.provider.id.to_owned(),
            audiences: binding
                .provider
                .audiences
                .iter()
                .map(|audience| audience.as_str().to_owned())
                .collect(),
            purpose: None,
        }
    }

    /// Every operation this deployment can currently call, filtered by the caller's query.
    fn search(&self, query: &str, limit: u16) -> Vec<OperationSummary> {
        let needle = query.trim().to_ascii_lowercase();
        let mut found = Vec::new();
        for binding in &self.bindings {
            for operation in binding.provider.operations {
                if found.len() >= limit as usize {
                    return found;
                }
                if !binding.admits(operation) {
                    continue;
                }
                if !needle.is_empty() && !operation.id.to_ascii_lowercase().contains(&needle) {
                    continue;
                }
                found.push(OperationSummary {
                    operation_ref: operation.id.to_owned(),
                    title: operation.id.to_owned(),
                    effect: effect_class(operation),
                    approval: ApprovalPosture::NotRequired,
                    connections: vec![self.summary(binding)],
                });
            }
        }
        found
    }

    fn describe(&self, operation_ref: &str) -> Result<OperationDescription, OperationError> {
        let operation = catalog::operation(catalog::OperationKey::id(operation_ref))
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "no such catalogued operation"))?;
        let binding = self
            .binding_for_operation(operation)
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "no Connection for its provider"))?;
        if !binding.admits(operation) {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "this Connection's grant admits reads only; connect with --allow writes to raise it",
            ));
        }
        let description_ref = lease_ref(operation_ref, &binding.connection_ref);
        self.leases.lock().expect("the lease map is not poisoned").insert(
            description_ref.clone(),
            Lease {
                operation_ref: operation_ref.to_owned(),
                connection_ref: binding.connection_ref.clone(),
            },
        );
        Ok(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: operation_ref.to_owned(),
            description: operation.description.to_owned(),
            input_schema: serde_json::from_str(operation.input_schema)
                .unwrap_or(serde_json::Value::Null),
            output_schema: serde_json::Value::Null,
            effect: effect_class(operation),
            approval: ApprovalPosture::NotRequired,
            connections: vec![self.summary(binding)],
            description_ref,
        })
    }

    /// Resolve one declared operation to a request, and execute it.
    ///
    /// The order is the whole safety argument: admit by grant, then resolve the credential, then
    /// build the request, then execute inside the aperture. A credential is read only after the
    /// operation has been admitted for this connection.
    async fn invoke(
        &self,
        operation_ref: &str,
        connection_ref: &str,
        description_ref: &str,
        input: serde_json::Value,
    ) -> Result<InvocationResult, OperationError> {
        if serde_json::to_vec(&input).map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES) {
            return Err(refusal(OperationErrorCode::InvalidInput, "caller input is too large"));
        }
        {
            let leases = self.leases.lock().expect("the lease map is not poisoned");
            let lease = leases.get(description_ref).ok_or_else(|| {
                refusal(
                    OperationErrorCode::StaleAuthority,
                    "read a fresh description before invoking",
                )
            })?;
            if lease.operation_ref != operation_ref || lease.connection_ref != connection_ref {
                return Err(refusal(
                    OperationErrorCode::StaleAuthority,
                    "the description lease is for a different operation or Connection",
                ));
            }
        }

        let operation = catalog::operation(catalog::OperationKey::id(operation_ref))
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "no such catalogued operation"))?;
        let binding = self
            .binding_by_ref(connection_ref)
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "no such Connection"))?;
        if binding.provider.id != operation.provider {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "that operation does not belong to this Connection's provider",
            ));
        }
        if !binding.admits(operation) {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "this Connection's grant admits reads only; connect with --allow writes to raise it",
            ));
        }
        if !binding.initiation.allows(domain::ConnectionInitiator::B10x) {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "this Connection does not permit B10x to initiate operations",
            ));
        }

        let document = connector_resolve::document::provider(binding.provider.id)
            .ok_or_else(|| refusal(OperationErrorCode::Unavailable, "the provider document is absent"))?;
        let declared = document
            .operation(operation_ref)
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "the operation has no request template"))?;

        let assembly = connector_resolve::assemble_credentials(
            operation,
            binding.provider,
            self.owner.tenant_id(),
            None,
            self.secrets.as_ref(),
            &binding.config,
        )
        .await
        .map_err(|_| {
            refusal(
                OperationErrorCode::NotGranted,
                "no stored credential satisfies this operation's declared mechanisms",
            )
        })?;

        let endpoints = connector_resolve::resolve_endpoints(
            declared,
            binding.provider,
            self.owner.tenant_id(),
            &binding.config,
        )
        .map_err(|_| {
            refusal(
                OperationErrorCode::InvalidInput,
                "a configuration variable this operation's URL needs was not supplied",
            )
        })?;

        let base_url = document
            .base_url(&operation.service)
            .ok_or_else(|| refusal(OperationErrorCode::Unavailable, "the service has no base URL"))?;

        let plan = connector_resolve::resolve(declared, base_url, &input, &endpoints, &assembly.credentials)
            .map_err(|_| refusal(OperationErrorCode::InvalidInput, "caller input did not satisfy the declared request"))?;

        let response = self
            .egress
            .execute(
                connection_ref,
                EgressHttpRequest {
                    request: plan.request,
                    maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                    response_headers: Vec::new(),
                },
            )
            .await
            // Two different failures wear one message otherwise, and the difference is the whole
            // diagnosis: a destination the deployment never admitted is a configuration answer, and
            // a destination it admitted and could not reach is a network one. Collapsing them cost
            // this crate's first live invocation a wrong guess — the host resolved to a private
            // address and the aperture was public, which reads exactly like the site being down.
            .map_err(|error| match error {
                // `Refused` covers both a destination the aperture never admitted and a transport
                // that failed, because the egress layer deliberately does not tell a caller which
                // — distinguishing them to a caller would confirm whether an address exists. The
                // operator-facing hint belongs here, where the configuration that decides it is.
                service::EgressTransportError::Refused => refusal(
                    OperationErrorCode::Unavailable,
                    "the provider was not reached: either it is unreachable, or this Connection's \
                     destination aperture does not admit its address — a self-hosted instance on \
                     your own network needs `network = \"operator\"`",
                ),
                service::EgressTransportError::ResponseTooLarge => refusal(
                    OperationErrorCode::ResultTooLarge,
                    "the provider's response exceeded the admitted bound",
                ),
            })?;

        if !response.is_success() {
            return Err(refusal(
                OperationErrorCode::Unavailable,
                "the provider refused the request",
            ));
        }
        let output = serde_json::from_slice(&response.body)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&response.body).into_owned()));

        Ok(InvocationResult {
            operation_ref: operation_ref.to_owned(),
            output,
            connector_audit_ref: audit_ref(operation_ref, connection_ref),
            execution_ref: None,
        })
    }
}

#[async_trait]
impl ConnectorBackend for CatalogBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::OPERATIONS
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Search(_) => !self.inner.bindings.is_empty(),
            OperationRequest::Describe(describe) => {
                self.owns_operation_ref(&describe.operation_ref)
            }
            OperationRequest::Invoke(invoke) => self.owns_operation_ref(&invoke.operation_ref),
            _ => false,
        }
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(search) => Ok(OperationResult::Search {
                operations: self.inner.search(&search.query, search.limit),
            }),
            OperationRequest::Describe(describe) => Ok(OperationResult::Describe(
                self.inner.describe(&describe.operation_ref)?,
            )),
            OperationRequest::Invoke(invoke) => Ok(OperationResult::Invoke(
                self.inner
                    .invoke(
                        &invoke.operation_ref,
                        &invoke.connection_ref,
                        &invoke.description_ref,
                        invoke.input,
                    )
                    .await?,
            )),
            _ => Err(refusal(
                OperationErrorCode::Unavailable,
                "this Integration serves declared operations only",
            )),
        }
    }
}

impl CatalogBackend {
    fn owns_operation_ref(&self, operation_ref: &str) -> bool {
        catalog::operation(catalog::OperationKey::id(operation_ref))
            .and_then(|operation| self.inner.binding_for_operation(operation))
            .is_some()
    }
}

/// The declared risk vocabulary, projected onto the caller-facing effect class.
///
/// Read from the catalogue rather than inferred from the HTTP method: a `POST` that only searches
/// is a read, and the document is where that judgement was reviewed.
fn effect_class(operation: &catalog::Operation) -> EffectClass {
    if matches!(operation.risk, Risk::Destructive) {
        return EffectClass::Destructive;
    }
    match operation.direction {
        OperationDirection::Read => EffectClass::ReadOnly,
        OperationDirection::Write => EffectClass::Mutating,
    }
}

/// The exact origins one configured provider will reach, for the deployment to admit.
///
/// Computed from the catalogue rather than asked of the operator: a provider's services declare
/// their own base URLs, and the operator only supplies the `{variable}` values those templates
/// carry. So the destination aperture is derived from the same declaration the request is built
/// from, and cannot drift from it — a request can never go somewhere the rules did not admit,
/// because both come from one source.
///
/// # Errors
///
/// A provider not in the catalogue, or a base URL whose variables the configuration did not supply.
pub fn admitted_origins(
    entry: &CatalogIntegrationConfig,
) -> Result<Vec<String>, CatalogIntegrationError> {
    let provider = catalog::provider(catalog::ProviderKey::id(&entry.provider))
        .ok_or_else(|| CatalogIntegrationError::UnknownProvider(entry.provider.clone()))?;
    let mut origins = Vec::new();
    for service in provider.services {
        let mut base = service.base_url.to_owned();
        for (name, value) in &entry.endpoints {
            base = base.replace(&format!("{{{name}}}"), value);
        }
        // A template still carrying a placeholder means the operator did not supply a value the
        // URL needs. Refusing here keeps that from becoming a request to a literal `{origin}` host.
        if base.contains('{') {
            return Err(CatalogIntegrationError::MissingEndpointValue(
                entry.provider.clone(),
                base,
            ));
        }
        let origin = origin_of(&base);
        if !origin.is_empty() && !origins.contains(&origin) {
            origins.push(origin);
        }
    }
    Ok(origins)
}

/// Scheme and authority only — the aperture is an origin, never a path.
fn origin_of(base: &str) -> String {
    let Some((scheme, rest)) = base.split_once("://") else {
        return String::new();
    };
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        return String::new();
    }
    format!("{scheme}://{authority}")
}

fn credential_leaf(
    provider: &'static catalog::Provider,
    requested: Option<&str>,
) -> Result<&'static str, CatalogIntegrationError> {
    match requested {
        Some(name) => provider
            .auth
            .iter()
            .find(|credential| credential.name == name)
            .map(|credential| credential.leaf)
            .ok_or_else(|| {
                CatalogIntegrationError::UnknownCredential(
                    provider.id.to_owned(),
                    name.to_owned(),
                )
            }),
        // The provider's first declared credential. Declaration order is the catalogue's own
        // preference order, so this is the one a reviewer put first rather than an arbitrary pick.
        None => provider
            .auth
            .first()
            .map(|credential| credential.leaf)
            .ok_or_else(|| {
                CatalogIntegrationError::UnknownCredential(
                    provider.id.to_owned(),
                    "(none declared)".to_owned(),
                )
            }),
    }
}

/// Read an owner-only credential file and seal its value in the store, once.
///
/// The file is a **bootstrap**, not the custody: once the value is in the store it is never read
/// from the file again, so an operator can delete it. The checks mirror the ones
/// `integration-slack` arrived at — regular file, owner uid, no group or other bits, bounded size —
/// because a credential file readable by anyone else is a credential that has already leaked.
async fn import_credential(
    provider: &str,
    path: &Path,
    reference: &CredentialRef,
    secrets: &dyn SecretStore,
) -> Result<(), CatalogIntegrationError> {
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::fs::PermissionsExt as _;

    const MAX_CREDENTIAL_FILE_BYTES: u64 = 8 * 1024;

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| CatalogIntegrationError::UnreadableCredentialFile(provider.to_owned()))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_CREDENTIAL_FILE_BYTES
    {
        return Err(CatalogIntegrationError::UnsafeCredentialFile(
            provider.to_owned(),
        ));
    }
    let value = zeroize::Zeroizing::new(
        std::fs::read_to_string(path)
            .map_err(|_| CatalogIntegrationError::UnreadableCredentialFile(provider.to_owned()))?,
    );
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CatalogIntegrationError::UnreadableCredentialFile(
            provider.to_owned(),
        ));
    }
    secrets
        .put(reference, &Secret::new(trimmed))
        .await
        .map_err(|_| CatalogIntegrationError::CredentialStore(provider.to_owned()))
}

fn ensure_owner_directory(root: &Path) -> Result<(), CatalogIntegrationError> {
    use std::os::unix::fs::PermissionsExt as _;

    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|_| CatalogIntegrationError::StateRoot)?;
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| CatalogIntegrationError::StateRoot)?;
    }
    Ok(())
}

fn connection_ref(provider: &str, name: &str) -> String {
    format!("connection:{provider}:{}", digest(&[provider, name]))
}

fn lease_ref(operation_ref: &str, connection_ref: &str) -> String {
    format!("description:{}", digest(&[operation_ref, connection_ref]))
}

fn audit_ref(operation_ref: &str, connection_ref: &str) -> String {
    format!("audit:{}", digest(&[operation_ref, connection_ref]))
}

fn digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hex::encode(&hasher.finalize()[..16])
}

fn refusal(code: OperationErrorCode, message: &str) -> OperationError {
    OperationError::new(code, message, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gitlab() -> &'static catalog::Provider {
        catalog::provider(catalog::ProviderKey::id("gitlab")).expect("gitlab is catalogued")
    }

    fn binding(allow_writes: bool) -> Binding {
        Binding {
            provider: gitlab(),
            connection_ref: "connection:gitlab:test".to_owned(),
            label: "GitLab".to_owned(),
            grant_ref: "grant:gitlab:test".to_owned(),
            initiation: InitiationPolicy::b10x_only(),
            config: DeclaredConfig::default(),
            allow_writes: false,
        }
        .tap(allow_writes)
    }

    impl Binding {
        fn tap(mut self, allow_writes: bool) -> Self {
            self.allow_writes = allow_writes;
            self
        }
    }

    #[test]
    fn the_default_ceiling_admits_reads_and_refuses_writes() {
        let read_only = binding(false);
        let reads = gitlab()
            .operations
            .iter()
            .filter(|operation| read_only.admits(operation))
            .count();
        let writes = gitlab()
            .operations
            .iter()
            .filter(|operation| !read_only.admits(operation))
            .count();
        assert!(reads > 0, "a read-only grant must still admit something");
        assert!(writes > 0, "gitlab declares write operations to refuse");
        // Raising the ceiling admits everything the catalogue declares, without naming any of it.
        let unrestricted = binding(true);
        assert!(gitlab()
            .operations
            .iter()
            .all(|operation| unrestricted.admits(operation)));
    }

    #[test]
    fn the_ceiling_reads_declared_facts_rather_than_an_operation_list() {
        let read_only = binding(false);
        for operation in gitlab().operations {
            let admitted = read_only.admits(operation);
            let declared_read = matches!(operation.direction, OperationDirection::Read)
                && !matches!(operation.risk, Risk::Destructive)
                && operation
                    .effects
                    .iter()
                    .all(|effect| matches!(effect, HostEffect::Read | HostEffect::Network));
            assert_eq!(
                admitted, declared_read,
                "`{}` was admitted on something other than its declaration",
                operation.id
            );
        }
    }

    #[test]
    fn a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be() {
        // The distinction the first version of `admits` got wrong. Every declared HTTP call
        // carries `[read, network]`, so treating any effect as disqualifying admitted nothing;
        // treating none as disqualifying would admit an operation that writes this machine's disk.
        let read_only = binding(false);
        let ordinary = gitlab()
            .operations
            .iter()
            .find(|operation| matches!(operation.direction, OperationDirection::Read))
            .expect("gitlab declares reads");
        assert!(ordinary
            .effects
            .iter()
            .any(|effect| matches!(effect, HostEffect::Network)));
        assert!(read_only.admits(ordinary), "a network effect must not disqualify a read");
        assert!(
            !ordinary
                .effects
                .iter()
                .any(|effect| matches!(effect, HostEffect::Filesystem | HostEffect::Process)),
            "this fixture is only meaningful while gitlab reads stay network-only"
        );
    }

    #[test]
    fn effect_class_comes_from_the_declaration_not_the_method() {
        for operation in gitlab().operations {
            let class = effect_class(operation);
            match operation.direction {
                OperationDirection::Read if !matches!(operation.risk, Risk::Destructive) => {
                    assert_eq!(class, EffectClass::ReadOnly, "{}", operation.id);
                }
                _ => assert_ne!(class, EffectClass::ReadOnly, "{}", operation.id),
            }
        }
    }

    fn entry(provider: &str, endpoints: &[(&str, &str)]) -> CatalogIntegrationConfig {
        CatalogIntegrationConfig {
            provider: provider.to_owned(),
            name: None,
            label: None,
            grant_ref: format!("grant:{provider}:test"),
            initiation: InitiationConfig::B10x,
            allow_writes: false,
            endpoints: endpoints
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            operator_approved: true,
            network: connectors_config::NetworkScopeConfig::Public,
            credential: None,
            credential_file: None,
        }
    }

    #[test]
    fn the_aperture_is_derived_from_the_same_declaration_the_request_is() {
        let origins = admitted_origins(&entry(
            "gitlab",
            &[("origin", "https://gitlab.example.test")],
        ))
        .expect("gitlab's services resolve");
        // Both declared services — the API surface and the OAuth surface — live at one origin, so
        // one rule admits the provider without admitting anything else.
        assert_eq!(origins, vec!["https://gitlab.example.test".to_owned()]);
    }

    #[test]
    fn an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder() {
        let error = admitted_origins(&entry("gitlab", &[])).expect_err("origin is not supplied");
        assert!(matches!(
            error,
            CatalogIntegrationError::MissingEndpointValue(_, _)
        ));
    }

    #[test]
    fn a_provider_with_a_fixed_base_url_needs_no_configuration() {
        // Most of the catalogue is like this: a SaaS host with nothing to ask the operator.
        let origins = admitted_origins(&entry("sentry", &[])).expect("sentry has fixed hosts");
        assert!(!origins.is_empty());
        assert!(origins.iter().all(|origin| origin.starts_with("https://")));
    }

    #[test]
    fn every_catalogued_provider_can_address_a_credential() {
        // The generic path can only reach a provider whose credential has an address. Measuring it
        // here means a catalogue change that breaks the property fails a test rather than one
        // person's connect attempt.
        let mut addressable = 0;
        for provider in catalog::providers() {
            if provider.authority.is_some() && !provider.auth.is_empty() {
                assert!(credential_leaf(provider, None).is_ok(), "{}", provider.id);
                addressable += 1;
            }
        }
        assert!(addressable >= 56, "only {addressable} providers are addressable");
    }

    #[test]
    fn a_request_template_exists_for_every_operation_this_backend_would_offer() {
        // The join this crate is: catalogue facts on one side, request template on the other. If a
        // provider's document is missing an operation the table declares, invocation would refuse
        // at runtime — so it is asserted for a real provider here.
        let document = connector_resolve::document::provider("gitlab").expect("gitlab document");
        for operation in gitlab().operations {
            assert!(
                document.operation(operation.id).is_some(),
                "`{}` is catalogued with no request template",
                operation.id
            );
        }
    }
}
