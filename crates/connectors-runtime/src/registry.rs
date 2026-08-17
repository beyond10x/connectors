//! Explicit Integration routing. A domain `NotFound` is never interpreted as a routing signal.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, ConnectionResult,
};
use protocol::datasource::{
    DatasourceError, DatasourceErrorCode, DatasourceRequest, DatasourceResult, DatasourceSummary,
};
use protocol::event::{EventError, EventErrorCode, EventRequest, EventResult};
use protocol::operation::{
    OperationDescription, OperationError, OperationErrorCode, OperationRequest, OperationResult,
    OperationSummary,
};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectorBackend, HostedCompletionError,
    HostedCompletionPage, HostedCompletionSubmission, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

/// Closed, deterministic registry of configured Integration backends.
pub struct BackendRegistry {
    backends: Vec<Arc<dyn ConnectorBackend>>,
}

impl BackendRegistry {
    /// Register a closed backend set. Backend order cannot affect non-search dispatch.
    #[must_use]
    pub fn new(backends: Vec<Arc<dyn ConnectorBackend>>) -> Self {
        Self { backends }
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
        Ok(descriptions)
    }
}

#[async_trait]
impl ConnectorBackend for BackendRegistry {
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
                let mut local_refs = vec![merged.description_ref.clone()];
                for description in descriptions {
                    ensure_compatible_description(&merged, &description)?;
                    merged.connections.extend(description.connections);
                    local_refs.push(description.description_ref);
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
                    .map(|(_, description)| description.description_ref.clone())
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
            [] => {
                return Err(DatasourceError::new(
                    DatasourceErrorCode::NotFound,
                    "no Integration owns this datasource",
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

    async fn shutdown(&self) {
        futures_util::future::join_all(self.backends.iter().map(|backend| backend.shutdown()))
            .await;
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
    {
        Ok(())
    } else {
        Err(operation_protocol(
            "Integrations disagreed about one operation description",
        ))
    }
}

fn registry_description_ref(
    context: &PrincipalContext,
    operation_ref: &str,
    local_refs: &mut [String],
) -> Result<String, OperationError> {
    local_refs.sort();
    let encoded = serde_json::to_vec(&(context, operation_ref, local_refs))
        .map_err(|_| operation_protocol("description lease could not be encoded"))?;
    let mut digest = Sha256::new();
    digest.update(b"b10x/connectors-runtime-description/v1\0");
    digest.update(encoded);
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
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use protocol::connection::{
        ConnectionInitiator, ConnectionRoute, ConnectionState,
        ConnectionSummary as ResourceConnectionSummary, DescribeRequest as ConnectionDescribe,
        SearchRequest as ConnectionSearch,
    };
    use protocol::event::{
        ChannelSummary as EventChannelSummary, ReceiveRequest, SearchRequest as EventSearch,
    };
    use protocol::operation::{
        ApprovalPosture, ConnectionSummary as OperationConnectionSummary, DescribeRequest,
        EffectClass, InvocationResult, InvokeRequest, SearchRequest, SessionRequest,
    };
    use serde_json::json;

    use super::*;

    #[derive(Default)]
    struct Calls {
        operation_search: AtomicUsize,
        operation_direct: AtomicUsize,
        connection_search: AtomicUsize,
        connection_direct: AtomicUsize,
        event_search: AtomicUsize,
        event_direct: AtomicUsize,
    }

    struct SyntheticBackend {
        capabilities: BackendCapabilities,
        operations: Vec<OperationSummary>,
        description: Option<OperationDescription>,
        invoke_connection: Option<String>,
        claims_direct_operation: bool,
        connections: Vec<ResourceConnectionSummary>,
        claims_connection: bool,
        channels: Vec<EventChannelSummary>,
        claims_event: bool,
        calls: Calls,
        invocation_leases: Mutex<Vec<String>>,
    }

    struct UnavailableBackend;

    #[async_trait]
    impl ConnectorBackend for UnavailableBackend {
        async fn ready(&self) -> Result<(), BackendReadinessError> {
            Err(BackendReadinessError)
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            unreachable!("readiness never dispatches an operation")
        }
    }

    impl SyntheticBackend {
        fn empty(capabilities: BackendCapabilities) -> Arc<Self> {
            Arc::new(Self {
                capabilities,
                operations: Vec::new(),
                description: None,
                invoke_connection: None,
                claims_direct_operation: false,
                connections: Vec::new(),
                claims_connection: false,
                channels: Vec::new(),
                claims_event: false,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }

        fn with_operations(operations: Vec<OperationSummary>) -> Arc<Self> {
            Arc::new(Self {
                capabilities: BackendCapabilities::OPERATIONS,
                operations,
                description: None,
                invoke_connection: None,
                claims_direct_operation: false,
                connections: Vec::new(),
                claims_connection: false,
                channels: Vec::new(),
                claims_event: false,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }

        fn contributor(connection_ref: &str, local_lease: &str) -> Arc<Self> {
            Arc::new(Self {
                capabilities: BackendCapabilities::OPERATIONS,
                operations: vec![operation_summary("tickets.read", connection_ref)],
                description: Some(operation_description(
                    "tickets.read",
                    connection_ref,
                    local_lease,
                )),
                invoke_connection: Some(connection_ref.to_owned()),
                claims_direct_operation: false,
                connections: Vec::new(),
                claims_connection: false,
                channels: Vec::new(),
                claims_event: false,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }

        fn with_connections(connections: Vec<ResourceConnectionSummary>) -> Arc<Self> {
            Arc::new(Self {
                capabilities: BackendCapabilities {
                    operations: false,
                    connections: true,
                    events: false,
                    datasources: false,
                },
                operations: Vec::new(),
                description: None,
                invoke_connection: None,
                claims_direct_operation: false,
                connections,
                claims_connection: false,
                channels: Vec::new(),
                claims_event: false,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }

        fn with_channels(channels: Vec<EventChannelSummary>) -> Arc<Self> {
            Arc::new(Self {
                capabilities: BackendCapabilities {
                    operations: false,
                    connections: false,
                    events: true,
                    datasources: false,
                },
                operations: Vec::new(),
                description: None,
                invoke_connection: None,
                claims_direct_operation: false,
                connections: Vec::new(),
                claims_connection: false,
                channels,
                claims_event: false,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }

        fn exclusive_claims() -> Arc<Self> {
            Arc::new(Self {
                capabilities: BackendCapabilities {
                    operations: true,
                    connections: true,
                    events: true,
                    datasources: false,
                },
                operations: Vec::new(),
                description: None,
                invoke_connection: None,
                claims_direct_operation: true,
                connections: Vec::new(),
                claims_connection: true,
                channels: Vec::new(),
                claims_event: true,
                calls: Calls::default(),
                invocation_leases: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl ConnectorBackend for SyntheticBackend {
        async fn ready(&self) -> Result<(), BackendReadinessError> {
            // Registry routing fixtures carry no configured runtime dependency.
            Ok(())
        }

        fn capabilities(&self) -> BackendCapabilities {
            self.capabilities
        }

        fn owns_operation(&self, request: &OperationRequest) -> bool {
            match request {
                OperationRequest::Describe(request) => self
                    .description
                    .as_ref()
                    .is_some_and(|description| description.operation_ref == request.operation_ref),
                OperationRequest::Invoke(request) => {
                    self.description.as_ref().is_some_and(|description| {
                        description.operation_ref == request.operation_ref
                    }) && self.invoke_connection.as_deref() == Some(&request.connection_ref)
                }
                OperationRequest::SessionStatus(_)
                | OperationRequest::SessionTerminate(_)
                | OperationRequest::SessionReconcile(_) => self.claims_direct_operation,
                OperationRequest::Search(_) => false,
            }
        }

        fn owns_connection(&self, request: &ConnectionRequest) -> bool {
            !matches!(request, ConnectionRequest::Search(_)) && self.claims_connection
        }

        fn owns_event(&self, request: &EventRequest) -> bool {
            !matches!(request, EventRequest::Search(_)) && self.claims_event
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            match request {
                OperationRequest::Search(_) => {
                    self.calls.operation_search.fetch_add(1, Ordering::SeqCst);
                    Ok(OperationResult::Search {
                        operations: self.operations.clone(),
                    })
                }
                OperationRequest::Describe(request) => self
                    .description
                    .clone()
                    .filter(|description| description.operation_ref == request.operation_ref)
                    .map(OperationResult::Describe)
                    .ok_or_else(|| operation_not_found("synthetic operation was not found")),
                OperationRequest::Invoke(request) => {
                    self.calls.operation_direct.fetch_add(1, Ordering::SeqCst);
                    self.invocation_leases
                        .lock()
                        .unwrap()
                        .push(request.description_ref);
                    Ok(OperationResult::Invoke(InvocationResult {
                        operation_ref: request.operation_ref,
                        output: json!({"selected_connection": request.connection_ref}),
                        connector_audit_ref: "audit:test".to_owned(),
                        execution_ref: None,
                    }))
                }
                OperationRequest::SessionStatus(_)
                | OperationRequest::SessionTerminate(_)
                | OperationRequest::SessionReconcile(_) => {
                    self.calls.operation_direct.fetch_add(1, Ordering::SeqCst);
                    Err(operation_not_found(
                        "synthetic runtime reference was not found",
                    ))
                }
            }
        }

        async fn handle_connection(
            &self,
            _context: &PrincipalContext,
            request: ConnectionRequest,
        ) -> Result<ConnectionResult, ConnectionError> {
            match request {
                ConnectionRequest::Search(_) => {
                    self.calls.connection_search.fetch_add(1, Ordering::SeqCst);
                    Ok(ConnectionResult::Search {
                        connections: self.connections.clone(),
                    })
                }
                _ => {
                    self.calls.connection_direct.fetch_add(1, Ordering::SeqCst);
                    Err(ConnectionError::new(
                        ConnectionErrorCode::NotFound,
                        "synthetic Connection was not found",
                        false,
                    ))
                }
            }
        }

        async fn handle_event(
            &self,
            _context: &PrincipalContext,
            request: EventRequest,
        ) -> Result<EventResult, EventError> {
            match request {
                EventRequest::Search(_) => {
                    self.calls.event_search.fetch_add(1, Ordering::SeqCst);
                    Ok(EventResult::Search {
                        channels: self.channels.clone(),
                    })
                }
                _ => {
                    self.calls.event_direct.fetch_add(1, Ordering::SeqCst);
                    Err(EventError::new(
                        EventErrorCode::NotFound,
                        "synthetic event was not found",
                        false,
                    ))
                }
            }
        }
    }

    fn context() -> PrincipalContext {
        PrincipalContext::hosted(
            "tenant-test".to_owned(),
            "principal-test".to_owned(),
            "actor-test".to_owned(),
            None,
            "snapshot-test".to_owned(),
            "a".repeat(64),
        )
        .unwrap()
    }

    fn operation_connection(connection_ref: &str) -> OperationConnectionSummary {
        OperationConnectionSummary {
            connection_ref: connection_ref.to_owned(),
            label: connection_ref.to_owned(),
            provider: "tickets".to_owned(),
            audiences: vec!["operations".to_owned()],
        }
    }

    fn operation_summary(operation_ref: &str, connection_ref: &str) -> OperationSummary {
        OperationSummary {
            operation_ref: operation_ref.to_owned(),
            title: format!("Operation {operation_ref}"),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections: vec![operation_connection(connection_ref)],
        }
    }

    fn operation_description(
        operation_ref: &str,
        connection_ref: &str,
        lease: &str,
    ) -> OperationDescription {
        OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: format!("Operation {operation_ref}"),
            description: "Reads one ticket".to_owned(),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections: vec![operation_connection(connection_ref)],
            description_ref: lease.to_owned(),
        }
    }

    fn resource_connection(connection_ref: &str) -> ResourceConnectionSummary {
        ResourceConnectionSummary {
            connection_ref: connection_ref.to_owned(),
            integration_ref: "tickets".to_owned(),
            label: connection_ref.to_owned(),
            state: ConnectionState::Callable,
            initiation: vec![ConnectionInitiator::B10x],
            route: ConnectionRoute::Direct,
            scope: None,
            actor: None,
            auth_profile: None,
        }
    }

    fn event_channel(channel_ref: &str) -> EventChannelSummary {
        EventChannelSummary {
            channel_ref: channel_ref.to_owned(),
            connection_ref: "connection:a".to_owned(),
            integration_ref: "tickets".to_owned(),
            binding_ref: "binding:a".to_owned(),
            events: vec!["ticket.updated".to_owned()],
        }
    }

    fn registry(backends: Vec<Arc<SyntheticBackend>>) -> BackendRegistry {
        BackendRegistry::new(
            backends
                .into_iter()
                .map(|backend| backend as Arc<dyn ConnectorBackend>)
                .collect(),
        )
    }

    #[tokio::test]
    async fn readiness_requires_every_configured_backend() {
        let ready: Arc<dyn ConnectorBackend> =
            SyntheticBackend::empty(BackendCapabilities::OPERATIONS);
        let unavailable: Arc<dyn ConnectorBackend> = Arc::new(UnavailableBackend);
        let registry = BackendRegistry::new(vec![ready, unavailable]);
        assert_eq!(registry.ready().await, Err(BackendReadinessError));
    }

    #[tokio::test]
    async fn search_aggregates_compatible_operations_and_deduplicates_the_operation() {
        let first = SyntheticBackend::with_operations(vec![
            operation_summary("tickets.alpha", "connection:alpha"),
            operation_summary("tickets.read", "connection:b"),
        ]);
        let second = SyntheticBackend::with_operations(vec![operation_summary(
            "tickets.read",
            "connection:a",
        )]);
        let connection_only = SyntheticBackend::empty(BackendCapabilities {
            operations: false,
            connections: true,
            events: false,
            datasources: false,
        });
        let result = registry(vec![
            Arc::clone(&first),
            Arc::clone(&second),
            Arc::clone(&connection_only),
        ])
        .handle(
            &context(),
            OperationRequest::Search(SearchRequest {
                query: "tickets".to_owned(),
                limit: 10,
            }),
        )
        .await
        .unwrap();
        let OperationResult::Search { operations } = result else {
            panic!("registry returned the wrong result");
        };
        assert_eq!(
            operations
                .iter()
                .map(|operation| operation.operation_ref.as_str())
                .collect::<Vec<_>>(),
            ["tickets.alpha", "tickets.read"]
        );
        assert_eq!(
            operations[1]
                .connections
                .iter()
                .map(|connection| connection.connection_ref.as_str())
                .collect::<Vec<_>>(),
            ["connection:a", "connection:b"]
        );
        assert_eq!(first.calls.operation_search.load(Ordering::SeqCst), 1);
        assert_eq!(second.calls.operation_search.load(Ordering::SeqCst), 1);
        assert_eq!(
            connection_only
                .calls
                .operation_search
                .load(Ordering::SeqCst),
            0,
            "operation Search must honor the advertised route family"
        );
    }

    #[tokio::test]
    async fn direct_dispatch_selects_the_unique_claim_without_not_found_probing() {
        let owner = SyntheticBackend::exclusive_claims();
        let non_owner = SyntheticBackend::empty(BackendCapabilities::OPERATIONS);
        let error = registry(vec![Arc::clone(&owner), Arc::clone(&non_owner)])
            .handle(
                &context(),
                OperationRequest::SessionStatus(SessionRequest {
                    execution_ref: "execution:missing".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::NotFound);
        assert_eq!(owner.calls.operation_direct.load(Ordering::SeqCst), 1);
        assert_eq!(non_owner.calls.operation_direct.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch() {
        let first = SyntheticBackend::exclusive_claims();
        let second = SyntheticBackend::exclusive_claims();
        let registry = registry(vec![Arc::clone(&first), Arc::clone(&second)]);

        let operation = registry
            .handle(
                &context(),
                OperationRequest::SessionStatus(SessionRequest {
                    execution_ref: "execution:1".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(operation.code, OperationErrorCode::Protocol);

        let connection = registry
            .handle_connection(
                &context(),
                ConnectionRequest::Describe(ConnectionDescribe {
                    connection_ref: "connection:1".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(connection.code, ConnectionErrorCode::Protocol);

        let event = registry
            .handle_event(
                &context(),
                EventRequest::Receive(ReceiveRequest {
                    channel_ref: "channel:1".to_owned(),
                    after: None,
                    limit: 1,
                    wait_ms: 0,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(event.code, EventErrorCode::Protocol);
        assert_eq!(first.calls.operation_direct.load(Ordering::SeqCst), 0);
        assert_eq!(first.calls.connection_direct.load(Ordering::SeqCst), 0);
        assert_eq!(first.calls.event_direct.load(Ordering::SeqCst), 0);
        assert_eq!(second.calls.operation_direct.load(Ordering::SeqCst), 0);
        assert_eq!(second.calls.connection_direct.load(Ordering::SeqCst), 0);
        assert_eq!(second.calls.event_direct.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn describe_merges_connections_and_invoke_receives_the_selected_local_lease() {
        let first = SyntheticBackend::contributor("connection:a", "lease:a");
        let second = SyntheticBackend::contributor("connection:b", "lease:b");
        let registry = registry(vec![Arc::clone(&first), Arc::clone(&second)]);
        let described = registry
            .handle(
                &context(),
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: "tickets.read".to_owned(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::Describe(description) = described else {
            panic!("registry returned the wrong result");
        };
        assert_eq!(
            description
                .connections
                .iter()
                .map(|connection| connection.connection_ref.as_str())
                .collect::<Vec<_>>(),
            ["connection:a", "connection:b"]
        );
        assert!(description
            .description_ref
            .starts_with("description:registry:"));
        assert_ne!(description.description_ref, "lease:a");
        assert_ne!(description.description_ref, "lease:b");

        let invoked = registry
            .handle(
                &context(),
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: "tickets.read".to_owned(),
                    connection_ref: "connection:a".to_owned(),
                    description_ref: description.description_ref,
                    input: json!({}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap();
        let OperationResult::Invoke(result) = invoked else {
            panic!("registry returned the wrong result");
        };
        assert_eq!(
            result.output,
            json!({"selected_connection": "connection:a"})
        );
        assert_eq!(
            first.invocation_leases.lock().unwrap().as_slice(),
            ["lease:a"]
        );
        assert!(second.invocation_leases.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn duplicate_connection_references_fail_search() {
        let first =
            SyntheticBackend::with_connections(vec![resource_connection("connection:duplicate")]);
        let second =
            SyntheticBackend::with_connections(vec![resource_connection("connection:duplicate")]);
        let error = registry(vec![first, second])
            .handle_connection(
                &context(),
                ConnectionRequest::Search(ConnectionSearch {
                    query: String::new(),
                    limit: 10,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ConnectionErrorCode::Protocol);
    }

    #[tokio::test]
    async fn duplicate_channel_references_fail_search() {
        let error = registry(vec![
            SyntheticBackend::with_channels(vec![event_channel("channel:duplicate")]),
            SyntheticBackend::with_channels(vec![event_channel("channel:duplicate")]),
        ])
        .handle_event(
            &context(),
            EventRequest::Search(EventSearch {
                query: String::new(),
                limit: 10,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, EventErrorCode::Protocol);
    }
}
