//! Local source delegation into protocol-specific runtime backends.

use super::*;

#[async_trait]
impl ConnectorBackend for KubernetesLocalBackend {
    fn owns_endpoint(&self, request: &protocol::endpoint::EndpointRequest) -> bool {
        self.endpoint_backend()
            .is_some_and(|backend| backend.owns_endpoint(request))
    }

    async fn handle_endpoint(
        &self,
        context: &PrincipalContext,
        request: protocol::endpoint::EndpointRequest,
    ) -> Result<protocol::endpoint::EndpointResult, protocol::endpoint::EndpointError> {
        let backend = self.endpoint_backend().ok_or_else(|| {
            protocol::endpoint::EndpointError::new(
                protocol::endpoint::EndpointErrorCode::Unavailable,
                "select a Kubernetes context with setup connect kubernetes",
                false,
            )
        })?;
        backend.handle_endpoint(context, request).await
    }

    async fn resolve_endpoint(
        &self,
        context: &PrincipalContext,
        endpoint_ref: &str,
        operation_ref: &str,
    ) -> Result<String, OperationError> {
        self.endpoint_backend()
            .ok_or_else(operation_not_found)?
            .resolve_endpoint(context, endpoint_ref, operation_ref)
            .await
    }

    fn owns_event(&self, request: &protocol::event::EventRequest) -> bool {
        self.endpoint_backend()
            .is_some_and(|backend| backend.owns_event(request))
    }

    async fn handle_event(
        &self,
        context: &PrincipalContext,
        request: protocol::event::EventRequest,
    ) -> Result<protocol::event::EventResult, protocol::event::EventError> {
        self.endpoint_backend()
            .ok_or_else(|| {
                protocol::event::EventError::new(
                    protocol::event::EventErrorCode::Unavailable,
                    "Kubernetes endpoint discovery is not configured",
                    false,
                )
            })?
            .handle_event(context, request)
            .await
    }

    fn owns_event_v2(&self, request: &protocol::event::v2::EventRequest) -> bool {
        self.endpoint_backend()
            .is_some_and(|backend| backend.owns_event_v2(request))
    }

    async fn handle_event_v2(
        &self,
        context: &PrincipalContext,
        request: protocol::event::v2::EventRequest,
    ) -> Result<protocol::event::v2::EventResult, protocol::event::EventError> {
        self.endpoint_backend()
            .ok_or_else(|| {
                protocol::event::EventError::new(
                    protocol::event::EventErrorCode::Unavailable,
                    "Kubernetes endpoint discovery is not configured",
                    false,
                )
            })?
            .handle_event_v2(context, request)
            .await
    }

    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        // Construction validates kubeconfig and local state. Cluster reachability is provider
        // health and remains an operation-level degradation.
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: self.endpoint_backend().is_some(),
            // `kubernetes.workloads`, read through whichever kubeconfig context the operator
            // activated. Declared unconditionally: a placement that advertised datasources only
            // once a cluster was attached would make the surface appear and disappear under a
            // person mid-session, and the refusal for "nothing attached yet" says what to do.
            datasources: true,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        if self
            .endpoint_backend()
            .is_some_and(|backend| backend.owns_operation(request))
        {
            return true;
        }
        match request {
            OperationRequest::Describe(request) => !self
                .connections_for_operation(&request.operation_ref)
                .is_empty(),
            OperationRequest::Invoke(request) => {
                lock(&self.state)
                    .children
                    .contains_key(&request.connection_ref)
                    || (matches!(
                        request.operation_ref.as_str(),
                        STATUS_OPERATION
                            | RESTART_OPERATION
                            | NAMESPACE_OPERATION
                            | WORKLOAD_OPERATION
                    ) && self.is_cluster_connection(&request.connection_ref))
            }
            OperationRequest::Search(_) => false,
            _ => false,
        }
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        WorkloadSurface::owns(request)
    }

    fn supports_ephemeral_invocation(&self, request: &InvokeRequest) -> bool {
        // Deployment pages carry a process-owned cursor and therefore require the daemon even
        // though their provider exchange is unary. Other reviewed members have no continuation.
        request.operation_ref != WORKLOAD_OPERATION
            && local_operations().contains(&request.operation_ref.as_str())
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        if self
            .endpoint_backend()
            .is_some_and(|backend| backend.owns_connection(request))
        {
            return true;
        }
        match request {
            ConnectionRequest::CandidateSearch(request) => request.integration_ref == KUBERNETES,
            ConnectionRequest::CandidateActivate(request) => {
                self.candidates.contains_key(&request.candidate_ref)
            }
            ConnectionRequest::Describe(request) => lock(&self.state)
                .connections
                .contains_key(&request.connection_ref),
            ConnectionRequest::ObservationSearch(request) => self.observations(request).is_some(),
            ConnectionRequest::Materialize(request) => {
                self.observation(&request.observation_ref).is_some()
            }
            ConnectionRequest::Search(_) => false,
            _ => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.check_operation_context(context)?;
        if !matches!(request, OperationRequest::Search(_)) {
            if let Some(backend) = self
                .endpoint_backend()
                .filter(|backend| backend.owns_operation(&request))
            {
                return backend.handle(context, request).await;
            }
        }
        match request {
            OperationRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let mut operations = local_operations()
                    .into_iter()
                    .filter(|operation_ref| {
                        query.is_empty()
                            || operation_ref.contains(&query)
                            || monitoring_model::provider_for_operation(operation_ref)
                                .contains(&query)
                    })
                    .filter_map(|operation_ref| self.operation_summary(operation_ref))
                    .collect::<Vec<_>>();
                if let Some(backend) = self.endpoint_backend() {
                    if let OperationResult::Search {
                        operations: discovered,
                    } = backend
                        .handle(context, OperationRequest::Search(search.clone()))
                        .await?
                    {
                        operations.extend(discovered);
                    }
                }
                operations.truncate(usize::from(search.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(DescribeRequest { operation_ref })
                if !self.connections_for_operation(&operation_ref).is_empty() =>
            {
                self.operation_description(context, &operation_ref)
                    .map(OperationResult::Describe)
            }
            OperationRequest::Invoke(request)
                if matches!(
                    request.operation_ref.as_str(),
                    STATUS_OPERATION | RESTART_OPERATION | NAMESPACE_OPERATION | WORKLOAD_OPERATION
                ) =>
            {
                self.invoke_workload(context, request).await
            }
            OperationRequest::Invoke(request)
                if lock(&self.state)
                    .children
                    .contains_key(&request.connection_ref) =>
            {
                self.invoke_service(context, request).await
            }
            _ => Err(operation_not_found()),
        }
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.check_datasource_context(context)?;
        self.workloads
            .handle(
                context,
                request,
                self.attached_cluster(),
                &self.readable_namespaces().await?,
            )
            .await
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.check_context(context)?;
        if !matches!(request, ConnectionRequest::Search(_)) {
            if let Some(backend) = self
                .endpoint_backend()
                .filter(|backend| backend.owns_connection(&request))
            {
                return backend.handle_connection(context, request).await;
            }
        }
        match request {
            ConnectionRequest::CandidateSearch(request)
                if request.integration_ref == KUBERNETES =>
            {
                Ok(ConnectionResult::CandidateSearch {
                    candidates: self.search_candidates(&request),
                })
            }
            ConnectionRequest::CandidateActivate(request)
                if self.candidates.contains_key(&request.candidate_ref) =>
            {
                self.activate(request)
                    .await
                    .map(ConnectionResult::CandidateActivate)
            }
            ConnectionRequest::Search(request) => {
                let mut connections = self.search_connections(&request.query);
                if let Some(backend) = self.endpoint_backend() {
                    if let ConnectionResult::Search {
                        connections: discovered,
                    } = backend
                        .handle_connection(context, ConnectionRequest::Search(request.clone()))
                        .await?
                    {
                        connections.extend(discovered);
                    }
                }
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                let description = {
                    lock(&self.state)
                        .connections
                        .get(&request.connection_ref)
                        .cloned()
                };
                description
                    .map(ConnectionResult::Describe)
                    .ok_or_else(connection_not_found)
            }
            ConnectionRequest::ObservationSearch(request) => self
                .observations(&request)
                .map(|observations| ConnectionResult::ObservationSearch { observations })
                .ok_or_else(connection_not_found),
            ConnectionRequest::Materialize(request)
                if self.observation(&request.observation_ref).is_some() =>
            {
                self.materialize(&request)
                    .map(ConnectionResult::Materialize)
            }
            _ => Err(connection_not_found()),
        }
    }
}
