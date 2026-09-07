#![forbid(unsafe_code)]

//! **One adapter for every declared provider.**
//!
//! # Why this exists
//!
//! GitLab is fourteen catalogued operations with complete HTTP request templates, five declared
//! credential mechanisms, a declared verify probe, and an authority to address its credential by.
//! It also had a 2,875-line hand-written Rust backend. So did Jira. So, in their own shapes, did
//! Slack, Grafana and Kubernetes — and each grew its own copy of credential handling, its own
//! dispatch, and its own idea of what an error is. An operator, on being shown the GitLab one:
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
use connector_address::{CredentialRef, InstanceId};
use connector_secrets::{Secret, SecretStore};
use connectors_config::{CatalogIntegrationConfig, InitiationConfig};
use domain::InitiationPolicy;
use protocol::connection as connection_api;
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, EffectClass, InvocationResult, OperationDescription,
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OperationSummary,
};
use service::{
    BackendCapabilities, ConnectorBackend, EgressHttpRequest, EgressTransport, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

mod config;
mod confluence_reads;
mod incremental_reads;
pub use config::DeclaredConfig;
mod custody;
pub mod endpoint;
mod oauth;
pub use oauth::{
    personal_oauth_admitted_connection_ref, personal_oauth_admitted_origins, PersonalOAuthBackend,
    PersonalOAuthError,
};
mod hosted;
pub use hosted::{hosted_admitted_origins, HostedCatalogBackend, HostedCatalogError};

/// Minting an Argo CD API token from an operator's password, so the `argocd` CLI is not a
/// prerequisite for connecting Argo CD.
///
/// It lives beside invocation rather than in the operator console because acquisition is a network
/// act, and the console is fenced against transports. The runtime-owned [`service::EgressTransport`]
/// port arrives here exactly as invocation's does — this crate composes no adapter either.
pub mod argocd;

/// The most caller input one operation may carry. A declared operation's input is a small JSON
/// object of catalogue-declared fields; anything larger is a caller mistake, not a payload.
const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CatalogIntegrationError {
    #[error("personal OAuth requires its dedicated custody owner")]
    OAuthCustodyRequired,
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
    /// Which instance this Connection's credential is addressed under, when it names one.
    instance: Option<InstanceId>,
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
/// the lease says *this caller has seen this operation's current shape*. Without it a caller could
/// invoke an operation whose input schema or risk had changed since it last looked.
///
/// **Bound to the operation, not to a Connection.** The shape a caller must have seen is the
/// catalogue's — the same for every Connection of a provider — so binding the lease to one
/// Connection only made a person holding two Slack identities describe the operation twice to call
/// it twice, and refused the second with `stale_authority`, which says something untrue about why.
/// Which Connection may serve the call is checked at invocation from the grant, where it belongs.
struct Lease {
    operation_ref: String,
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
            if entry.oauth.is_some() {
                return Err(CatalogIntegrationError::OAuthCustodyRequired);
            }
            let provider = catalog::provider(catalog::ProviderKey::id(&entry.provider))
                .ok_or_else(|| CatalogIntegrationError::UnknownProvider(entry.provider.clone()))?;
            let authority = provider
                .authority
                .ok_or_else(|| CatalogIntegrationError::NoAuthority(entry.provider.clone()))?;

            if let Some(path) = entry.credential_file.as_ref() {
                let leaf = credential_leaf(provider, entry.credential.as_deref())?;
                let reference = credential_address(owner.tenant_id(), authority, entry, leaf)?;
                import_credential(&entry.provider, path, &reference, secrets.as_ref()).await?;
            }

            bindings.push(Binding {
                provider,
                connection_ref: connection_ref(&entry.provider, entry.instance()),
                label: entry.label(),
                grant_ref: entry.grant_ref.clone(),
                initiation: match entry.initiation {
                    InitiationConfig::Platform => InitiationPolicy::platform_only(),
                    InitiationConfig::Provider => InitiationPolicy::provider_only(),
                    InitiationConfig::Both => InitiationPolicy::bidirectional(),
                },
                config: declared_config(entry),
                allow_writes: entry.allow_writes,
                instance: match entry.instance.as_deref() {
                    Some(name) => Some(instance_for(&entry.provider, name)?),
                    None => None,
                },
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

    /// Bind already-stored catalog Connections without importing credential material.
    ///
    /// Hosted self-service uses this constructor after filtering durable connection rows to the
    /// authenticated principal. Credential acquisition remains in [`HostedCatalogBackend`]; this
    /// adapter only resolves and invokes the resulting, already-addressed values.
    pub fn bind_stored(
        owner: PrincipalContext,
        configured: &[CatalogIntegrationConfig],
        secrets: Arc<dyn SecretStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, CatalogIntegrationError> {
        let mut bindings = Vec::with_capacity(configured.len());
        for entry in configured {
            if entry.oauth.is_some() {
                return Err(CatalogIntegrationError::OAuthCustodyRequired);
            }
            let provider = catalog::provider(catalog::ProviderKey::id(&entry.provider))
                .ok_or_else(|| CatalogIntegrationError::UnknownProvider(entry.provider.clone()))?;
            provider
                .authority
                .ok_or_else(|| CatalogIntegrationError::NoAuthority(entry.provider.clone()))?;
            credential_leaf(provider, entry.credential.as_deref())?;
            bindings.push(Binding {
                provider,
                connection_ref: connection_ref(&entry.provider, entry.instance()),
                label: entry.label(),
                grant_ref: entry.grant_ref.clone(),
                initiation: match entry.initiation {
                    InitiationConfig::Platform => InitiationPolicy::platform_only(),
                    InitiationConfig::Provider => InitiationPolicy::provider_only(),
                    InitiationConfig::Both => InitiationPolicy::bidirectional(),
                },
                config: declared_config(entry),
                allow_writes: entry.allow_writes,
                instance: match entry.instance.as_deref() {
                    Some(name) => Some(instance_for(&entry.provider, name)?),
                    None => None,
                },
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

    /// Configured bindings are known, but this passive view has no persisted verification evidence.
    /// Reuse operation discovery's opaque identity without probing custody or the provider.
    fn connections(&self, query: &str, limit: u16) -> Vec<connection_api::ConnectionSummary> {
        self.bindings
            .iter()
            .filter(|binding| matches_query(query, &[binding.provider.id, &binding.label]))
            .take(usize::from(limit))
            .map(|binding| connection_api::ConnectionSummary {
                connection_ref: binding.connection_ref.clone(),
                integration_ref: binding.provider.id.to_owned(),
                label: binding.label.clone(),
                state: connection_api::ConnectionState::Created,
                initiation: [
                    (
                        domain::ConnectionInitiator::Platform,
                        connection_api::ConnectionInitiator::Platform,
                    ),
                    (
                        domain::ConnectionInitiator::Provider,
                        connection_api::ConnectionInitiator::Provider,
                    ),
                ]
                .into_iter()
                .filter_map(|(declared, exposed)| {
                    binding.initiation.allows(declared).then_some(exposed)
                })
                .collect(),
                route: connection_api::ConnectionRoute::Direct,
                scope: None,
                actor: None,
                auth_profile: None,
            })
            .collect()
    }

    /// Every operation this deployment can currently call, filtered by the caller's query.
    ///
    /// **Grouped by operation, then limited.** One operation that several Connections can serve is
    /// one row carrying all of them, not one row per Connection — a person holding two Slack
    /// identities has one `slack-users-info`, answerable as either. Doing it the other way round
    /// also truncated wrongly: applying the caller's limit while still walking Connections dropped
    /// the later identities from an operation instead of dropping later operations, so
    /// `--limit 1` reported `slack-users-info` as reachable through exactly one identity when three
    /// could serve it.
    fn search(&self, query: &str, limit: u16) -> Vec<OperationSummary> {
        // Insertion-ordered so the result is stable across runs: a caller diffing two searches
        // should see real changes, not map iteration order.
        let mut grouped: Vec<(&'static catalog::Operation, Vec<ConnectionSummary>)> = Vec::new();
        for binding in &self.bindings {
            for operation in binding.provider.operations {
                if !binding.admits(operation) {
                    continue;
                }
                if !matches_query(
                    query,
                    &[operation.id, operation.description, binding.provider.id],
                ) {
                    continue;
                }
                match grouped
                    .iter_mut()
                    .find(|(existing, _)| existing.id == operation.id)
                {
                    Some((_, connections)) => connections.push(self.summary(binding)),
                    None => grouped.push((operation, vec![self.summary(binding)])),
                }
            }
        }
        grouped
            .into_iter()
            .take(limit as usize)
            .map(|(operation, connections)| OperationSummary {
                operation_ref: operation.id.to_owned(),
                title: operation.id.to_owned(),
                effect: effect_class(operation),
                approval: approval_posture(operation),
                connections,
            })
            .collect()
    }

    fn describe(&self, operation_ref: &str) -> Result<OperationDescription, OperationError> {
        let operation = catalog::operation(catalog::OperationKey::id(operation_ref))
            .ok_or_else(|| refusal(OperationErrorCode::NotFound, "no such catalogued operation"))?;
        // Any admitting Connection proves the operation is describable here; which one serves a
        // call is the caller's choice at invocation.
        self.binding_for_operation(operation).ok_or_else(|| {
            refusal(
                OperationErrorCode::NotFound,
                "no Connection for its provider",
            )
        })?;
        if !self
            .bindings
            .iter()
            .any(|binding| binding.provider.id == operation.provider && binding.admits(operation))
        {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "no Connection admits this operation; for a personal catalog instance, authorize \
                 allow_writes = true in its [[catalog]] policy and restart its daemon \
                 (--allow writes applies when enrolling a new instance)",
            ));
        }
        let description_ref = lease_ref(operation_ref);
        self.leases
            .lock()
            .expect("the lease map is not poisoned")
            .insert(
                description_ref.clone(),
                Lease {
                    operation_ref: operation_ref.to_owned(),
                },
            );
        Ok(OperationDescription {
            rate_advice: service::operation_rate_advice(operation),
            operation_ref: operation_ref.to_owned(),
            title: operation_ref.to_owned(),
            description: operation.description.to_owned(),
            input_schema: incremental_reads::input_schema(operation_ref).unwrap_or_else(|| {
                serde_json::from_str(operation.input_schema).unwrap_or(serde_json::Value::Null)
            }),
            output_schema: operation
                .output_schema
                .map(|schema| {
                    serde_json::from_str(schema).expect("catalog output schema is generated JSON")
                })
                .unwrap_or(serde_json::Value::Null),
            effect: effect_class(operation),
            approval: approval_posture(operation),
            // Every Connection that could serve it, so a caller reading one description can pick.
            connections: self
                .bindings
                .iter()
                .filter(|candidate| {
                    candidate.provider.id == operation.provider && candidate.admits(operation)
                })
                .map(|candidate| self.summary(candidate))
                .collect(),
            description_ref,
        })
    }

    /// Resolve one declared operation to a request, and execute it.
    ///
    /// The order is the whole safety argument: admit by grant, then resolve the credential, then
    /// build the request, then execute inside the aperture. A credential is read only after the
    /// operation has been admitted for this connection.
    fn admit_invocation(
        &self,
        operation_ref: &str,
        connection_ref: &str,
        description_ref: &str,
        input: &serde_json::Value,
    ) -> Result<(&'static catalog::Operation, &Binding), OperationError> {
        if serde_json::to_vec(input).map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES) {
            return Err(refusal(
                OperationErrorCode::InvalidInput,
                "caller input is too large",
            ));
        }
        {
            let leases = self.leases.lock().expect("the lease map is not poisoned");
            let lease = leases.get(description_ref).ok_or_else(|| {
                refusal(
                    OperationErrorCode::StaleAuthority,
                    "read a fresh description before invoking",
                )
            })?;
            if lease.operation_ref != operation_ref {
                return Err(refusal(
                    OperationErrorCode::StaleAuthority,
                    "the description lease is for a different operation",
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
                "this Connection's grant admits reads only; for this personal catalog instance, \
                 authorize allow_writes = true in its [[catalog]] policy and restart its daemon \
                 (--allow writes applies when enrolling a new instance)",
            ));
        }
        if !binding
            .initiation
            .allows(domain::ConnectionInitiator::Platform)
        {
            return Err(refusal(
                OperationErrorCode::NotGranted,
                "this Connection does not permit the platform to initiate operations",
            ));
        }

        Ok((operation, binding))
    }

    async fn invoke(
        &self,
        operation_ref: &str,
        connection_ref: &str,
        description_ref: &str,
        input: serde_json::Value,
    ) -> Result<InvocationResult, OperationError> {
        let (operation, binding) =
            self.admit_invocation(operation_ref, connection_ref, description_ref, &input)?;

        let document =
            connector_resolve::document::provider(binding.provider.id).ok_or_else(|| {
                refusal(
                    OperationErrorCode::Unavailable,
                    "the provider document is absent",
                )
            })?;
        let declared = document.operation(operation_ref).ok_or_else(|| {
            refusal(
                OperationErrorCode::NotFound,
                "the operation has no request template",
            )
        })?;

        incremental_reads::validate_input(operation_ref, &input)?;

        let assembly = connector_resolve::assemble_credentials(
            operation,
            binding.provider,
            self.owner.tenant_id(),
            binding.instance.as_ref(),
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

        let base_url = document.base_url(operation.service).ok_or_else(|| {
            refusal(
                OperationErrorCode::Unavailable,
                "the service has no base URL",
            )
        })?;

        let mut plan = connector_resolve::resolve(
            declared,
            base_url,
            &input,
            &endpoints,
            &assembly.credentials,
        )
        .map_err(|_| {
            refusal(
                OperationErrorCode::InvalidInput,
                "caller input did not satisfy the declared request",
            )
        })?;

        incremental_reads::prepare_request(operation_ref, &input, &mut plan.request)?;
        let request_url = plan.request.url.clone();
        let response = self
            .egress
            .execute(
                connection_ref,
                EgressHttpRequest {
                    request: plan.request,
                    maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                    response_headers: incremental_reads::response_headers(operation_ref),
                },
            )
            .await
            // Two different failures wear one message otherwise, and the difference is the whole
            // diagnosis: a destination the deployment never admitted is a configuration answer, and
            // a destination it admitted and could not reach is a network one. Collapsing them cost
            // this crate's first live invocation a wrong guess — the host resolved to a private
            // address and the aperture was public, which reads exactly like the site being down.
            .map_err(|error| match error {
                // `Refused` covers a destination the aperture never admitted and `Transport` an
                // admitted one that failed; they wear one caller-facing message deliberately —
                // distinguishing them to a caller would confirm whether an address exists. The
                // operator-facing hint belongs here, where the configuration that decides it is.
                service::EgressTransportError::Refused
                | service::EgressTransportError::Transport(_) => refusal(
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

        if response.status == 429 {
            return Err(OperationError::rate_limited(
                "the provider refused this request with HTTP 429",
                service::retry_after_seconds(
                    response.headers.get("retry-after").map(String::as_str),
                ),
            ));
        }
        if incremental_reads::handles(operation_ref) {
            return Ok(InvocationResult {
                operation_ref: operation_ref.to_owned(),
                output: incremental_reads::project(operation_ref, &input, &request_url, response)?,
                connector_audit_ref: audit_ref(operation_ref, connection_ref),
                execution_ref: None,
            });
        }
        if !response.is_success() {
            // **The status code, because without it the message names no cause.** A wrong issue
            // key, an unauthenticated credential and a permission the token does not carry are
            // three different jobs, and "the provider refused the request" is the same sentence
            // for all of them — the first live Atlassian invocation cost exactly this, with a
            // stored token, a configured user half and nothing saying which end was wrong.
            //
            // The **body is deliberately not carried**: a vendor's error text is unbounded, is not
            // covered by the catalogue's declared output schema, and is the one place a rejected
            // request can echo what was sent. A number is the whole diagnosis.
            let hint = match response.status {
                401 => " — the credential was not accepted; a `basic` mechanism has two halves, the stored secret and the `[catalog.usernames]` user half",
                403 => " — the credential is valid but lacks permission for this resource",
                404 => " — the resource does not exist, or is not visible to this credential",
                _ => "",
            };
            return Err(refusal(
                OperationErrorCode::Unavailable,
                format!(
                    "the provider refused the request with HTTP {}{hint}",
                    response.status
                ),
            ));
        }
        let output = serde_json::from_slice(&response.body).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&response.body).into_owned())
        });

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
    fn supports_ephemeral_invocation(&self, request: &protocol::operation::InvokeRequest) -> bool {
        catalog::operation(catalog::OperationKey::id(&request.operation_ref)).is_some_and(
            |operation| operation.interaction_shape == catalog::InteractionShape::Unary,
        )
    }

    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            connections: true,
            ..BackendCapabilities::OPERATIONS
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Search(_) => !self.inner.bindings.is_empty(),
            OperationRequest::Describe(describe) => {
                self.owns_operation_ref(&describe.operation_ref)
            }
            OperationRequest::Invoke(invoke) => {
                self.owns_operation_ref(&invoke.operation_ref)
                    && self.inner.binding_by_ref(&invoke.connection_ref).is_some()
            }
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

    async fn handle_connection(
        &self,
        _context: &PrincipalContext,
        request: connection_api::ConnectionRequest,
    ) -> Result<connection_api::ConnectionResult, connection_api::ConnectionError> {
        match request {
            connection_api::ConnectionRequest::Search(search) => {
                Ok(connection_api::ConnectionResult::Search {
                    connections: self.inner.connections(&search.query, search.limit),
                })
            }
            _ => Err(connection_api::ConnectionError::new(
                connection_api::ConnectionErrorCode::Unavailable,
                "this Integration serves passive Connection search only",
                false,
            )),
        }
    }
}

/// All whitespace-separated words match public catalog text, independent of punctuation in ids.
fn matches_query(query: &str, fields: &[&str]) -> bool {
    let text = fields.join(" ").to_ascii_lowercase();
    query
        .split_whitespace()
        .all(|word| text.contains(&word.to_ascii_lowercase()))
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

fn approval_posture(operation: &catalog::Operation) -> ApprovalPosture {
    match effect_class(operation) {
        EffectClass::ReadOnly => ApprovalPosture::NotRequired,
        EffectClass::Mutating | EffectClass::Destructive => ApprovalPosture::Required,
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

/// **Both halves of a Connection's declared configuration**, from the one entry that carries them.
///
/// Endpoint values fill the `{variable}` slots a base URL declares. Usernames are the non-secret
/// user half of a `basic` credential — an Atlassian account email against `jira.api_token` — which
/// [`connector_resolve::assemble_credentials`] asks for through
/// [`ConfigField::Username`](connector_resolve::ConfigField) and refuses the whole mechanism
/// without.
///
/// It exists as one function because both constructors need it and neither may have a different
/// answer: [`CatalogBackend::open`] built the endpoints alone and [`CatalogBackend::bind_stored`]
/// built the same thing again, so a stored Atlassian token resolved to
/// `MissingCredentialConfig` — surfaced to a caller as `not_granted: no stored credential
/// satisfies this operation's declared mechanisms` — while `auth status` reported it as stored.
/// Both statements were true and neither was the problem.
fn declared_config(entry: &CatalogIntegrationConfig) -> DeclaredConfig {
    entry.usernames.iter().fold(
        DeclaredConfig::new(entry.endpoints.clone(), entry.operator_approved),
        |config, (credential, user)| config.with_username(credential, user),
    )
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
                CatalogIntegrationError::UnknownCredential(provider.id.to_owned(), name.to_owned())
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

/// The instance a named Connection's credential is addressed under.
///
/// # Why a derived UUID rather than the name
///
/// `connector_address::validate_instance` requires the canonical 36-character hyphenated form, so a
/// human name cannot be the address. Deriving one keeps the address stable across restarts — the
/// same name always yields the same instance — with no registry mapping names to ids.
///
/// **The provider is inside the domain separator**, not only the name. Without it `support-bot`
/// on Slack and `support-bot` on another provider would derive the same instance, and two
/// unrelated credentials would collide at one address. This generalises the Slack-only derivation
/// `integration-slack` arrived at first; the namespace is versioned so a later change to what goes
/// into the digest cannot silently collide with ids an earlier build derived.
fn instance_for(provider: &str, name: &str) -> Result<InstanceId, CatalogIntegrationError> {
    let mut hasher = Sha256::new();
    hasher.update(b"b10x/instance/v1\0");
    hasher.update(provider.as_bytes());
    hasher.update([0]);
    hasher.update(name.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hasher.finalize()[..16]);
    // Version 4 and RFC 4122 variant bits, so the result is a well-formed UUID rather than sixteen
    // random-looking bytes wearing hyphens.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex::encode(bytes);
    let text = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    InstanceId::parse(&text)
        .map_err(|_| CatalogIntegrationError::UnknownProvider(provider.to_owned()))
}

/// The credential address for one configured entry.
///
/// An entry that names no instance keeps the **elided** address — byte-identical to what a
/// single-connection deployment already stored — so adding instance support does not orphan a
/// credential connected before it existed.
///
/// # Errors
///
/// An address that will not render, which for a catalogued provider means a missing authority.
pub fn credential_address(
    tenant: &str,
    authority: &str,
    entry: &CatalogIntegrationConfig,
    leaf: &str,
) -> Result<CredentialRef, CatalogIntegrationError> {
    match entry.instance.as_deref() {
        Some(name) => {
            let instance = instance_for(&entry.provider, name)?;
            CredentialRef::for_instance(
                tenant,
                authority,
                instance.as_str(),
                connector_address::DEFAULT_SERVICE,
                leaf,
            )
        }
        None => CredentialRef::new(tenant, authority, connector_address::DEFAULT_SERVICE, leaf),
    }
    .map_err(|_| CatalogIntegrationError::NoAuthority(entry.provider.clone()))
}

fn lease_ref(operation_ref: &str) -> String {
    let advice = catalog::operation(catalog::OperationKey::id(operation_ref))
        .and_then(service::operation_rate_advice);
    let encoded = serde_json::to_string(&advice).expect("typed rate advice serializes");
    format!("description:{}", digest(&[operation_ref, &encoded]))
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

fn refusal(code: OperationErrorCode, message: impl Into<String>) -> OperationError {
    OperationError::new(code, message.into(), false)
}

include!("tests.rs");
