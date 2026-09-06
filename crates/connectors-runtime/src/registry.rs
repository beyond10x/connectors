//! Explicit Integration routing. A domain `NotFound` is never interpreted as a routing signal.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, ConnectionResult,
};
use protocol::datasource::{
    DatasourceError, DatasourceErrorCode, DatasourceRequest, DatasourceResult, DatasourceSummary,
    SearchRequest as DatasourceSearchRequest,
};
use protocol::event::{EventError, EventErrorCode, EventRequest, EventResult};
use protocol::operation::{
    ApprovalPosture, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary,
};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectorBackend,
    HostedCompletionError, HostedCompletionPage, HostedCompletionSubmission, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

use crate::claims::{ClaimError, EventReplyClaims};

/// Closed, deterministic registry of configured Integration backends.
pub struct BackendRegistry {
    backends: Vec<Arc<dyn ConnectorBackend>>,
    /// The personal placement's one-time claim over `event:` evidence (S-048). `None` on the
    /// hosted placement, whose `ApprovalGate` spends approval records upstream of this registry.
    event_reply_claims: Option<EventReplyClaims>,
}

impl BackendRegistry {
    /// Register a closed backend set. Backend order cannot affect non-search dispatch.
    #[must_use]
    pub fn new(backends: Vec<Arc<dyn ConnectorBackend>>) -> Self {
        Self {
            backends,
            event_reply_claims: None,
        }
    }

    /// Register a closed backend set behind the local one-time event-reply claim.
    ///
    /// The personal composition binds this shape: an invocation whose description demands
    /// approval and which presents an `event:` evidence reference spends that reference exactly
    /// once at this seam, before any Integration is reached — the honest local counterpart of
    /// the hosted route's approval redemption, and the reason no adapter reads the evidence
    /// itself (catalog invariant rule 15).
    #[must_use]
    pub fn with_event_reply_claims(
        backends: Vec<Arc<dyn ConnectorBackend>>,
        claims: EventReplyClaims,
    ) -> Self {
        Self {
            backends,
            event_reply_claims: Some(claims),
        }
    }

    fn remediation_owner(
        &self,
        route: service::RemediationRoute<'_>,
    ) -> Result<&Arc<dyn ConnectorBackend>, service::RemediationError> {
        let mut owners = self
            .backends
            .iter()
            .filter(|backend| backend.owns_remediation(route));
        let first = owners.next().ok_or(service::RemediationError::Refused)?;
        if owners.next().is_some() {
            return Err(service::RemediationError::Unavailable);
        }
        Ok(first)
    }

    fn operation_claims(&self, request: &OperationRequest) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_operation(request))
            .collect()
    }

    fn connection_claims(&self, request: &ConnectionRequest) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_connection(request))
            .collect()
    }

    fn event_claims(&self, request: &EventRequest) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_event(request))
            .collect()
    }

    /// Collect datasource definitions from every backend that publishes them, refusing a
    /// reference two Integrations both claim.
    async fn search_datasources(
        &self,
        context: &PrincipalContext,
        search: &DatasourceSearchRequest,
    ) -> Result<BTreeMap<String, DatasourceSummary>, DatasourceError> {
        let mut definitions = BTreeMap::<String, DatasourceSummary>::new();
        for backend in &self.backends {
            if !backend.capabilities().datasources {
                continue;
            }
            let result = backend
                .handle_datasource(context, DatasourceRequest::Search(search.clone()))
                .await?;
            let DatasourceResult::Search { definitions: found } = result else {
                return Err(datasource_protocol(
                    "datasource search backend returned a wrong result",
                ));
            };
            for definition in found {
                if definitions
                    .insert(definition.datasource_ref.clone(), definition)
                    .is_some()
                {
                    return Err(datasource_protocol(
                        "multiple Integrations published one datasource reference",
                    ));
                }
            }
        }
        Ok(definitions)
    }

    fn datasource_claims(&self, request: &DatasourceRequest) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_datasource(request))
            .collect()
    }

    fn completion_claims(&self, session_ref: &str) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_hosted_completion(session_ref))
            .collect()
    }

    fn oauth_claims(&self, integration_ref: &str, state: &str) -> Vec<&Arc<dyn ConnectorBackend>> {
        self.backends
            .iter()
            .filter(|backend| backend.owns_hosted_oauth_state(integration_ref, state))
            .collect()
    }

    async fn describe_contributors(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<Vec<(usize, OperationDescription)>, OperationError> {
        let request = OperationRequest::Describe(protocol::operation::DescribeRequest {
            operation_ref: operation_ref.to_owned(),
        });
        let mut descriptions = Vec::new();
        for (index, backend) in self.backends.iter().enumerate() {
            if !backend.owns_operation(&request) {
                continue;
            }
            match backend.handle(context, request.clone()).await? {
                OperationResult::Describe(description) => descriptions.push((index, description)),
                _ => {
                    return Err(operation_protocol(
                        "descriptor backend returned a wrong result",
                    ))
                }
            }
        }
        for pair in descriptions.windows(2) {
            ensure_compatible_description(&pair[0].1, &pair[1].1)?;
        }
        Ok(descriptions)
    }
}

#[async_trait]
impl ConnectorBackend for BackendRegistry {
    fn owns_remediation(&self, route: service::RemediationRoute<'_>) -> bool {
        self.backends
            .iter()
            .any(|backend| backend.owns_remediation(route))
    }

    fn remediation_metadata<'a>(
        &'a self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> Result<service::RemediationMetadata<'a>, service::RemediationError> {
        match self.remediation_owner(service::RemediationRoute::Target(target)) {
            Ok(backend) => backend.remediation_metadata(context, target),
            Err(service::RemediationError::Refused) => {
                // Preserve ordinary defaults for an exact configured Connection owner. This is
                // predicate-only routing, never a fallback after an operation was dispatched.
                let connection =
                    ConnectionRequest::Describe(protocol::connection::DescribeRequest {
                        connection_ref: target.connection_ref.into(),
                    });
                let operation = OperationRequest::Describe(protocol::operation::DescribeRequest {
                    operation_ref: target.operation_ref.into(),
                });
                let mut owners = self.backends.iter().filter(|backend| {
                    backend.owns_connection(&connection) && backend.owns_operation(&operation)
                });
                let first = owners.next().ok_or(service::RemediationError::Refused)?;
                if owners.next().is_some() {
                    return Err(service::RemediationError::Unavailable);
                }
                first.remediation_metadata(context, target)
            }
            Err(error) => Err(error),
        }
    }

    fn personal_remediation_admission(
        &self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> Result<service::RemediationAdmission, service::RemediationError> {
        self.remediation_owner(service::RemediationRoute::Target(target))?
            .personal_remediation_admission(context, target)
    }

    async fn credential_readiness(
        &self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> service::CredentialReadiness {
        match self.remediation_owner(service::RemediationRoute::Target(target)) {
            Ok(backend) => backend.credential_readiness(context, target).await,
            Err(service::RemediationError::Refused) => service::CredentialReadiness::Unsupported,
            Err(_) => service::CredentialReadiness::DependencyUnavailable,
        }
    }

    async fn handle_remediation(
        &self,
        context: &PrincipalContext,
        request: service::RemediationRequest,
        authority: Arc<dyn service::RemediationAuthority>,
    ) -> Result<service::RemediationResult, service::RemediationError> {
        let route = match &request {
            service::RemediationRequest::Start(binding) => {
                service::RemediationRoute::Target(service::RemediationTarget {
                    operation_ref: &binding.operation_ref,
                    connection_ref: &binding.connection_ref,
                })
            }
            service::RemediationRequest::Status(request) => {
                service::RemediationRoute::Session(&request.connect_session_ref)
            }
            service::RemediationRequest::Acknowledge(request) => {
                service::RemediationRoute::Session(&request.connect_session_ref)
            }
        };
        self.remediation_owner(route)?
            .handle_remediation(context, request, authority)
            .await
    }

    fn supports_ephemeral_invocation(&self, request: &protocol::operation::InvokeRequest) -> bool {
        unique_operation_claim(self.operation_claims(&OperationRequest::Invoke(request.clone())))
            .is_ok_and(|backend| backend.supports_ephemeral_invocation(request))
    }

    async fn ready(&self) -> Result<(), BackendReadinessError> {
        for backend in &self.backends {
            backend.ready().await?;
        }
        Ok(())
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(search) => {
                let mut operations = BTreeMap::<String, OperationSummary>::new();
                for backend in &self.backends {
                    if !backend.capabilities().operations {
                        continue;
                    }
                    let result = backend
                        .handle(context, OperationRequest::Search(search.clone()))
                        .await?;
                    let OperationResult::Search { operations: found } = result else {
                        return Err(operation_protocol("search backend returned a wrong result"));
                    };
                    for operation in found {
                        merge_summary(&mut operations, operation)?;
                    }
                }
                Ok(OperationResult::Search {
                    operations: operations
                        .into_values()
                        .take(usize::from(search.limit))
                        .collect(),
                })
            }
            OperationRequest::Describe(describe) => {
                let contributors = self
                    .describe_contributors(context, &describe.operation_ref)
                    .await?;
                let mut descriptions = contributors
                    .into_iter()
                    .map(|(_, description)| description)
                    .collect::<Vec<_>>();
                let mut merged = descriptions
                    .pop()
                    .ok_or_else(|| operation_not_found("no Integration owns this operation"))?;
                let mut local_refs = vec![registry_contributor_ref(&merged)];
                for description in descriptions {
                    ensure_compatible_description(&merged, &description)?;
                    local_refs.push(registry_contributor_ref(&description));
                    merged.connections.extend(description.connections);
                }
                merged
                    .connections
                    .sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
                merged
                    .connections
                    .dedup_by(|left, right| left.connection_ref == right.connection_ref);
                merged.description_ref =
                    registry_description_ref(context, &merged.operation_ref, &mut local_refs)?;
                Ok(OperationResult::Describe(merged))
            }
            OperationRequest::Invoke(mut invoke) => {
                let claims = self.operation_claims(&OperationRequest::Invoke(invoke.clone()));
                let backend = unique_operation_claim(claims)?;
                let contributors = self
                    .describe_contributors(context, &invoke.operation_ref)
                    .await?;
                let mut local_refs = contributors
                    .iter()
                    .map(|(_, description)| registry_contributor_ref(description))
                    .collect::<Vec<_>>();
                let expected =
                    registry_description_ref(context, &invoke.operation_ref, &mut local_refs)?;
                if invoke.description_ref != expected {
                    return Err(OperationError::new(
                        OperationErrorCode::StaleAuthority,
                        "operation description lease is stale",
                        false,
                    ));
                }
                let selected = self
                    .backends
                    .iter()
                    .position(|candidate| Arc::ptr_eq(candidate, backend))
                    .and_then(|index| {
                        contributors
                            .iter()
                            .find(|(candidate, _)| *candidate == index)
                    })
                    .ok_or_else(|| {
                        operation_protocol("invocation backend did not describe its operation")
                    })?;
                invoke
                    .description_ref
                    .clone_from(&selected.1.description_ref);
                // The local one-time claim (S-048), spent before the backend can produce the
                // outward effect: at most one reply per triggering event, even when the claim's
                // dispatch then fails. Only an approval-demanding operation presenting an
                // `event:` reference is claimed — a reference nothing demands stays unspent.
                if let Some(claims) = &self.event_reply_claims {
                    if selected.1.approval == ApprovalPosture::Required {
                        if let Some(reference) = invoke
                            .approval_evidence_ref
                            .as_deref()
                            .filter(|reference| reference.starts_with("event:"))
                        {
                            claims
                                .claim(reference, &invoke.operation_ref)
                                .map_err(claim_refusal)?;
                        }
                    }
                }
                backend
                    .handle(context, OperationRequest::Invoke(invoke))
                    .await
            }
            other => {
                let claims = self.operation_claims(&other);
                unique_operation_claim(claims)?.handle(context, other).await
            }
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        if let ConnectionRequest::Search(search) = request {
            let mut connections = BTreeMap::new();
            for backend in &self.backends {
                if !backend.capabilities().connections {
                    continue;
                }
                let result = backend
                    .handle_connection(context, ConnectionRequest::Search(search.clone()))
                    .await?;
                let ConnectionResult::Search { connections: found } = result else {
                    return Err(connection_protocol(
                        "search backend returned a wrong result",
                    ));
                };
                for connection in found {
                    if connections
                        .insert(connection.connection_ref.clone(), connection)
                        .is_some()
                    {
                        return Err(connection_protocol(
                            "multiple Integrations published one Connection reference",
                        ));
                    }
                }
            }
            return Ok(ConnectionResult::Search {
                connections: connections
                    .into_values()
                    .take(usize::from(search.limit))
                    .collect(),
            });
        }
        let claims = self.connection_claims(&request);
        unique_connection_claim(claims)?
            .handle_connection(context, request)
            .await
    }

    async fn handle_event(
        &self,
        context: &PrincipalContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        if let EventRequest::Search(search) = request {
            let mut channels = BTreeMap::new();
            for backend in &self.backends {
                if !backend.capabilities().events {
                    continue;
                }
                let result = backend
                    .handle_event(context, EventRequest::Search(search.clone()))
                    .await?;
                let EventResult::Search { channels: found } = result else {
                    return Err(event_protocol("search backend returned a wrong result"));
                };
                for channel in found {
                    if channels
                        .insert(channel.channel_ref.clone(), channel)
                        .is_some()
                    {
                        return Err(event_protocol(
                            "multiple Integrations published one channel reference",
                        ));
                    }
                }
            }
            return Ok(EventResult::Search {
                channels: channels
                    .into_values()
                    .take(usize::from(search.limit))
                    .collect(),
            });
        }
        let claims = self.event_claims(&request);
        unique_event_claim(claims)?
            .handle_event(context, request)
            .await
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        if let DatasourceRequest::Search(search) = request {
            let mut definitions = self.search_datasources(context, &search).await?;
            // A caller that asks with a topical word ("connector", "read") and gets nothing back
            // concludes the deployment has no datasources at all — the exact wrong reading, made
            // twice by review agents against the live workbench. The query narrows an admitted
            // set; when it narrows to nothing, the admitted set is the honest answer.
            if definitions.is_empty() && !search.query.is_empty() {
                let unfiltered = DatasourceSearchRequest {
                    query: String::new(),
                    limit: search.limit,
                };
                definitions = self.search_datasources(context, &unfiltered).await?;
            }
            return Ok(DatasourceResult::Search {
                definitions: definitions
                    .into_values()
                    .take(usize::from(search.limit))
                    .collect(),
            });
        }
        let claims = self.datasource_claims(&request);
        let backend = match claims.as_slice() {
            [backend] => backend,
            // The backend set is closed at composition (`BackendRegistry::new`), so this can only
            // mean the deployment configures no Integration for this datasource — never that one
            // has not registered yet. It used to read as "no Integration owns this datasource"
            // with no reference attached, which a reviewing agent took to mean a datasource it
            // had just described had lost its owner. Name the reference so the two cannot be
            // confused.
            [] => {
                return Err(DatasourceError::new(
                    DatasourceErrorCode::NotFound,
                    format!(
                        "no Integration in this deployment is configured for datasource `{}`",
                        datasource_reference(&request)
                    ),
                    false,
                ))
            }
            _ => {
                return Err(datasource_protocol(
                    "multiple Integrations claimed one datasource request",
                ))
            }
        };
        backend.handle_datasource(context, request).await
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: self
                .backends
                .iter()
                .any(|backend| backend.capabilities().operations),
            connections: self
                .backends
                .iter()
                .any(|backend| backend.capabilities().connections),
            events: self
                .backends
                .iter()
                .any(|backend| backend.capabilities().events),
            datasources: self
                .backends
                .iter()
                .any(|backend| backend.capabilities().datasources),
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        matches!(request, OperationRequest::Search(_)) || !self.operation_claims(request).is_empty()
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        matches!(request, ConnectionRequest::Search(_))
            || !self.connection_claims(request).is_empty()
    }

    fn connect_session_access(
        &self,
        request: &protocol::connection::ConnectSessionCreateRequest,
    ) -> ConnectSessionAccess {
        let wrapped = ConnectionRequest::ConnectSessionCreate(request.clone());
        match unique_connection_claim(self.connection_claims(&wrapped)) {
            Ok(backend) => backend.connect_session_access(request),
            Err(_) => ConnectSessionAccess::Operator,
        }
    }

    /// Union the registered backends' self-service flows for one provider.
    ///
    /// Composition registers curated backends before the generic catalog adapter. Keeping the
    /// first exact profile makes the curated experience authoritative while allowing the generic
    /// adapter to add every other declared profile. Dispatch still fails closed if two backends
    /// claim the same create request; projection precedence never grants routing authority.
    fn setup_profiles(&self, provider_ref: &str) -> Vec<protocol::catalog::SetupProfileSummary> {
        let mut published = BTreeMap::<String, protocol::catalog::SetupProfileSummary>::new();
        for backend in &self.backends {
            for profile in backend.setup_profiles(provider_ref) {
                published
                    .entry(profile.auth_profile.clone())
                    .or_insert(profile);
            }
        }
        published.into_values().collect()
    }

    fn owns_event(&self, request: &EventRequest) -> bool {
        matches!(request, EventRequest::Search(_)) || !self.event_claims(request).is_empty()
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        matches!(request, DatasourceRequest::Search(_))
            || !self.datasource_claims(request).is_empty()
    }

    fn owns_hosted_completion(&self, session_ref: &str) -> bool {
        !self.completion_claims(session_ref).is_empty()
    }

    fn hosted_completion_page(
        &self,
        session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        unique_completion_claim(self.completion_claims(session_ref))?
            .hosted_completion_page(session_ref)
    }

    async fn complete_hosted_session(
        &self,
        session_ref: &str,
        capability: &str,
        submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        unique_completion_claim(self.completion_claims(session_ref))?
            .complete_hosted_session(session_ref, capability, submission)
            .await
    }

    fn owns_hosted_oauth_state(&self, integration_ref: &str, state: &str) -> bool {
        !self.oauth_claims(integration_ref, state).is_empty()
    }

    async fn complete_hosted_oauth(
        &self,
        integration_ref: &str,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        unique_completion_claim(self.oauth_claims(integration_ref, state))?
            .complete_hosted_oauth(integration_ref, state, code, error)
            .await
    }

    async fn shutdown(&self) {
        futures_util::future::join_all(self.backends.iter().map(|backend| backend.shutdown()))
            .await;
    }
}

/// The datasource reference one non-search request names.
fn datasource_reference(request: &DatasourceRequest) -> &str {
    match request {
        DatasourceRequest::Search(_) => "",
        DatasourceRequest::Describe(request) => &request.datasource_ref,
        DatasourceRequest::Bindings(request) => &request.datasource_ref,
        DatasourceRequest::Read(request) => &request.datasource_ref,
    }
}

fn unique_completion_claim(
    claims: Vec<&Arc<dyn ConnectorBackend>>,
) -> Result<&Arc<dyn ConnectorBackend>, HostedCompletionError> {
    match claims.as_slice() {
        [backend] => Ok(backend),
        [] => Err(HostedCompletionError::NotFound),
        _ => Err(HostedCompletionError::Unavailable),
    }
}

fn unique_operation_claim(
    claims: Vec<&Arc<dyn ConnectorBackend>>,
) -> Result<&Arc<dyn ConnectorBackend>, OperationError> {
    match claims.as_slice() {
        [] => Err(operation_not_found("no Integration owns this operation")),
        [backend] => Ok(backend),
        _ => Err(operation_protocol(
            "multiple Integrations claimed exclusive operation dispatch",
        )),
    }
}

fn unique_connection_claim(
    claims: Vec<&Arc<dyn ConnectorBackend>>,
) -> Result<&Arc<dyn ConnectorBackend>, ConnectionError> {
    match claims.as_slice() {
        [] => Err(ConnectionError::new(
            ConnectionErrorCode::NotFound,
            "no Integration owns this Connection request",
            false,
        )),
        [backend] => Ok(backend),
        _ => Err(connection_protocol(
            "multiple Integrations claimed one Connection request",
        )),
    }
}

fn unique_event_claim(
    claims: Vec<&Arc<dyn ConnectorBackend>>,
) -> Result<&Arc<dyn ConnectorBackend>, EventError> {
    match claims.as_slice() {
        [] => Err(EventError::new(
            EventErrorCode::NotFound,
            "no Integration owns this event request",
            false,
        )),
        [backend] => Ok(backend),
        _ => Err(event_protocol(
            "multiple Integrations claimed one event request",
        )),
    }
}

fn merge_summary(
    operations: &mut BTreeMap<String, OperationSummary>,
    mut incoming: OperationSummary,
) -> Result<(), OperationError> {
    let Some(existing) = operations.get_mut(&incoming.operation_ref) else {
        operations.insert(incoming.operation_ref.clone(), incoming);
        return Ok(());
    };
    if existing.title != incoming.title
        || existing.effect != incoming.effect
        || existing.approval != incoming.approval
    {
        return Err(operation_protocol(
            "Integrations disagreed about one operation contract",
        ));
    }
    existing.connections.append(&mut incoming.connections);
    existing
        .connections
        .sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
    existing
        .connections
        .dedup_by(|left, right| left.connection_ref == right.connection_ref);
    Ok(())
}

fn ensure_compatible_description(
    left: &OperationDescription,
    right: &OperationDescription,
) -> Result<(), OperationError> {
    if left.operation_ref == right.operation_ref
        && left.title == right.title
        && left.description == right.description
        && left.input_schema == right.input_schema
        && left.output_schema == right.output_schema
        && left.effect == right.effect
        && left.approval == right.approval
        && left.rate_advice == right.rate_advice
    {
        Ok(())
    } else {
        Err(operation_protocol(
            "Integrations disagreed about one operation description",
        ))
    }
}

fn registry_contributor_ref(description: &OperationDescription) -> String {
    serde_json::to_string(&(&description.description_ref, &description.rate_advice))
        .expect("typed description advice serializes")
}

fn registry_description_ref(
    context: &PrincipalContext,
    operation_ref: &str,
    local_refs: &mut [String],
) -> Result<String, OperationError> {
    local_refs.sort();
    let mut digest = Sha256::new();
    digest.update(b"b10x/connectors-runtime-description/v2\0");
    digest.update(context.stable_authority_seed());
    digest.update(b"\0");
    digest.update(operation_ref.as_bytes());
    for local_ref in local_refs.iter() {
        digest.update(b"\0");
        digest.update(local_ref.as_bytes());
    }
    Ok(format!(
        "description:registry:{}",
        hex_digest(digest.finalize())
    ))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn claim_refusal(error: ClaimError) -> OperationError {
    match error {
        ClaimError::Replayed => OperationError::new(
            OperationErrorCode::ApprovalDenied,
            "the triggering event has already authorized one reply",
            false,
        ),
        // Fail closed, in both cases: a claim that could not be made durable must not reply,
        // because a restarted daemon would grant it again.
        ClaimError::Capacity => OperationError::new(
            OperationErrorCode::Unavailable,
            "the local reply-claim journal is full",
            false,
        ),
        ClaimError::Unavailable => OperationError::new(
            OperationErrorCode::Unavailable,
            "the local reply-claim journal is unavailable",
            true,
        ),
    }
}

fn operation_not_found(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::NotFound, message, false)
}

fn operation_protocol(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::Protocol, message, false)
}

fn datasource_protocol(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::Protocol, message, false)
}

fn connection_protocol(message: &'static str) -> ConnectionError {
    ConnectionError::new(ConnectionErrorCode::Protocol, message, false)
}

fn event_protocol(message: &'static str) -> EventError {
    EventError::new(EventErrorCode::Protocol, message, false)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "remediation_tests.rs"]
mod remediation_tests;
