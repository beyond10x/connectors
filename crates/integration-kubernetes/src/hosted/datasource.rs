use super::*;

impl KubernetesStatusBackend {
    fn require_datasource_owner(&self, context: &PrincipalContext) -> Result<(), DatasourceError> {
        if context.tenant_id() == self.expected_tenant {
            Ok(())
        } else {
            Err(datasource_not_granted(
                "Connector tenant binding refused the datasource request",
            ))
        }
    }

    /// Resolves the namespace one binding ref names, or says which of two different things went
    /// wrong.
    ///
    /// These were one refusal — "Kubernetes datasource binding is not granted" — and a reviewing
    /// agent carried it to an operator as a missing grant when the binding ref was simply not a
    /// binding ref at all. A ref that names no namespace is the caller's mistake and says so; a
    /// namespace the principal cannot read is a real grant gap and names the namespace, the
    /// groups that carry read, and that group membership can arrive late.
    fn binding_namespace<'a>(
        &'a self,
        context: &PrincipalContext,
        binding_ref: &str,
    ) -> Result<&'a str, DatasourceError> {
        let Some((namespace, access)) = self
            .namespace_access
            .iter()
            .find(|(namespace, _)| namespace_binding_ref(namespace) == binding_ref)
        else {
            return Err(DatasourceError::new(
                DatasourceErrorCode::InvalidInput,
                format!(
                    "`{binding_ref}` is not a binding of `{DATASOURCE}`; list its bindings and read through one of those"
                ),
                false,
            ));
        };
        if !self.can_read(context, namespace) {
            return Err(
                self.namespace_read_grant_missing(std::iter::once((namespace.as_str(), access)))
            );
        }
        Ok(namespace.as_str())
    }

    /// The refusal for a principal whose groups grant read on no configured namespace.
    fn missing_read_grant(&self) -> DatasourceError {
        self.namespace_read_grant_missing(
            self.namespace_access
                .iter()
                .map(|(namespace, access)| (namespace.as_str(), access)),
        )
    }

    /// One refusal naming every namespace at stake and the Identity groups that carry read on it.
    ///
    /// Retriable, because group membership is not a property of this deployment: the same
    /// principal read one namespace in one turn against the deployed build and none five minutes
    /// later, so a caller told "not granted" flatly would escalate a condition that clears.
    fn namespace_read_grant_missing<'a>(
        &self,
        namespaces: impl Iterator<Item = (&'a str, &'a NamespaceAccess)>,
    ) -> DatasourceError {
        let mut named = Vec::new();
        for (namespace, access) in namespaces {
            let mut carriers = access.read_groups.iter().cloned().collect::<Vec<_>>();
            carriers.extend(self.operator_groups.iter().cloned());
            carriers.sort();
            carriers.dedup();
            named.push(format!("`{namespace}` (one of: {})", carriers.join(", ")));
        }
        if named.is_empty() {
            return DatasourceError::new(
                DatasourceErrorCode::NotGranted,
                format!("this deployment configures no readable namespace for `{DATASOURCE}`"),
                false,
            );
        }
        DatasourceError::new(
            DatasourceErrorCode::NotGranted,
            format!(
                "reading `{DATASOURCE}` needs Identity group membership for {}. Ask whoever administers your Identity groups to add you, then retry.",
                named.join("; ")
            ),
            true,
        )
    }

    /// Admission, then the shared read.
    ///
    /// Everything this receiver owns is above the seam: the tenant binding, the Identity groups
    /// that carry read on a configured namespace, and the audit ref this deployment stamps. What a
    /// workload *is* belongs to `crate::workloads` and is identical in every host mode.
    async fn read_datasource(
        &self,
        context: &PrincipalContext,
        request: ReadRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        let namespace = self
            .binding_namespace(context, &request.binding_ref)?
            .to_owned();
        let connector_audit_ref = audit_ref(context, DATASOURCE, &namespace, "workloads");
        read_workloads(
            self.reader.as_ref(),
            &self.cursors,
            context,
            &namespace,
            request,
            connector_audit_ref,
        )
        .await
    }
}

#[async_trait]
impl ConnectorBackend for KubernetesStatusBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        // Construction validates in-cluster trust and client configuration. Kubernetes API
        // availability is provider health and remains an operation-level degradation.
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: false,
            datasources: true,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => {
                matches!(
                    request.operation_ref.as_str(),
                    STATUS_OPERATION | RESTART_OPERATION | LOGS_OPERATION
                )
            }
            OperationRequest::Invoke(request) => {
                matches!(
                    request.operation_ref.as_str(),
                    STATUS_OPERATION | RESTART_OPERATION | LOGS_OPERATION
                ) && request.connection_ref == CONNECTION
            }
            OperationRequest::Search(_)
            | OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        matches!(request, ConnectionRequest::Describe(request) if request.connection_ref == CONNECTION)
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Describe(request) => request.datasource_ref == DATASOURCE,
            DatasourceRequest::Bindings(request) => request.datasource_ref == DATASOURCE,
            DatasourceRequest::Read(request) => request.datasource_ref == DATASOURCE,
            DatasourceRequest::Search(_) => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.require_owner(context)?;
        match request {
            OperationRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let matches_query = query.is_empty()
                    || [
                        "kubernetes",
                        "deployment",
                        "backend",
                        "running",
                        "status",
                        "restart",
                        "log",
                        "logs",
                        "pod",
                    ]
                    .iter()
                    .any(|term| query.contains(term));
                let mut operations = Vec::new();
                if matches_query && self.has_read_access(context) {
                    operations.push(status_summary());
                    operations.push(logs_summary());
                }
                if matches_query && self.has_restart_access(context) {
                    operations.push(restart_summary());
                }
                operations.truncate(usize::from(search.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(DescribeRequest { operation_ref }) => Ok(
                OperationResult::Describe(self.description(context, &operation_ref)?),
            ),
            OperationRequest::Invoke(request) => self.invoke(context, request).await,
            _ => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "Kubernetes Integration operation was not found",
                false,
            )),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.require_connection_owner(context)?;
        if !self.has_read_access(context) {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "Kubernetes Connection is not granted to this principal",
                false,
            ));
        }
        match request {
            ConnectionRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let connections = (query.is_empty()
                    || ["kubernetes", "development", "cluster", "read-only"]
                        .iter()
                        .any(|term| query.contains(term)))
                .then(control_connection)
                .into_iter()
                .take(usize::from(search.limit))
                .collect();
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(ConnectionDescribeRequest { connection_ref })
                if connection_ref == CONNECTION =>
            {
                Ok(ConnectionResult::Describe(ControlConnectionDescription {
                    summary: control_connection(),
                    channels: Vec::new(),
                }))
            }
            _ => Err(ConnectionError::new(
                ConnectionErrorCode::NotFound,
                "Kubernetes Integration Connection was not found",
                false,
            )),
        }
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.require_datasource_owner(context)?;
        match request {
            DatasourceRequest::Search(DatasourceSearchRequest { query, limit }) => {
                let query = query.to_ascii_lowercase();
                let definitions = (self.has_read_access(context)
                    && (query.is_empty()
                        || ["kubernetes", "deployment", "workload", "pod", "rollout"]
                            .iter()
                            .any(|term| query.contains(term))))
                .then(datasource_summary)
                .into_iter()
                .take(usize::from(limit))
                .collect();
                Ok(DatasourceResult::Search { definitions })
            }
            // A principal without a read grant used to fall through to "Kubernetes datasource was
            // not found" — the datasource exists, the grant does not, and only one of those is
            // something the person can act on.
            DatasourceRequest::Describe(DatasourceDescribeRequest { datasource_ref })
                if datasource_ref == DATASOURCE =>
            {
                if self.has_read_access(context) {
                    Ok(DatasourceResult::Describe(datasource_description(context)))
                } else {
                    Err(self.missing_read_grant())
                }
            }
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref,
                query,
                limit,
            }) if datasource_ref == DATASOURCE => {
                let query = query.to_ascii_lowercase();
                let bindings = self
                    .readable_namespaces(context)
                    .into_iter()
                    .filter(|namespace| query.is_empty() || namespace.contains(&query))
                    .take(usize::from(limit))
                    .map(|namespace| namespace_binding(CONNECTION, namespace))
                    .collect();
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Read(read) => self.read_datasource(context, read).await,
            _ => Err(datasource_not_found("Kubernetes datasource was not found")),
        }
    }
}
