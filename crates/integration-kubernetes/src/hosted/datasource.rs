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

    fn resolve_cursor(
        &self,
        context: &PrincipalContext,
        namespace: &str,
        cursor: Option<&str>,
    ) -> Result<Option<String>, DatasourceError> {
        let Some(cursor) = cursor else {
            return Ok(None);
        };
        let mut cursors = self
            .cursors
            .lock()
            .map_err(|_| datasource_unavailable("Kubernetes cursor state is unavailable"))?;
        let now = SystemTime::now();
        cursors.retain(|_, state| state.expires_at > now);
        let state = cursors.remove(cursor).ok_or_else(|| {
            DatasourceError::new(
                DatasourceErrorCode::CursorExpired,
                "Kubernetes datasource cursor is expired or unknown",
                false,
            )
        })?;
        if state.namespace != namespace
            || state.principal_subject != context.subject()
            || state.authority_seed != context.stable_authority_seed()
        {
            return Err(datasource_not_granted(
                "Kubernetes datasource cursor belongs to different authority",
            ));
        }
        Ok(Some(state.provider_cursor))
    }

    fn store_cursor(
        &self,
        context: &PrincipalContext,
        namespace: &str,
        provider_cursor: Option<String>,
    ) -> Result<Option<String>, DatasourceError> {
        let Some(provider_cursor) = provider_cursor.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let now = SystemTime::now();
        let nonce = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut digest = Sha256::new();
        digest.update(b"b10x/kubernetes-datasource-cursor/v2\0");
        digest.update(context.stable_authority_seed());
        digest.update(format!("\0{namespace}\0{provider_cursor}\0{nonce}").as_bytes());
        let cursor_ref = format!(
            "cursor:kubernetes:{}",
            hex::encode(&digest.finalize()[..20])
        );
        let mut cursors = self
            .cursors
            .lock()
            .map_err(|_| datasource_unavailable("Kubernetes cursor state is unavailable"))?;
        cursors.retain(|_, state| state.expires_at > now);
        if cursors.len() >= 256 {
            return Err(DatasourceError::new(
                DatasourceErrorCode::ResultTooLarge,
                "Kubernetes cursor capacity is exhausted",
                true,
            ));
        }
        cursors.insert(
            cursor_ref.clone(),
            CursorState {
                namespace: namespace.to_owned(),
                principal_subject: context.subject().to_owned(),
                authority_seed: context.stable_authority_seed(),
                provider_cursor,
                expires_at: now + CURSOR_TTL,
            },
        );
        Ok(Some(cursor_ref))
    }

    async fn read_datasource(
        &self,
        context: &PrincipalContext,
        request: ReadRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        if request.datasource_ref != DATASOURCE {
            return Err(datasource_not_found("Kubernetes datasource was not found"));
        }
        if request.description_ref != datasource_description_ref(context) {
            // Recoverable in one call, and the caller is told which one. It used to say only
            // "authority is stale", which reads like something an operator has to repair.
            return Err(DatasourceError::new(
                DatasourceErrorCode::StaleAuthority,
                "the Kubernetes datasource description lease has moved on; describe it again and retry the read",
                true,
            ));
        }
        let namespace = self
            .binding_namespace(context, &request.binding_ref)?
            .to_owned();
        let (records, next_cursor, completeness) = match request.read {
            DatasourceRead::List { limit, cursor } => {
                let provider_cursor =
                    self.resolve_cursor(context, &namespace, cursor.as_deref())?;
                let list = self
                    .reader
                    .list_workloads(&namespace, limit, provider_cursor.as_deref())
                    .await?;
                let next_cursor = self.store_cursor(context, &namespace, list.next_cursor)?;
                let records = list
                    .workloads
                    .into_iter()
                    .map(|workload| {
                        let name = workload.name.clone();
                        let value = serde_json::to_value(workload).map_err(|_| {
                            datasource_unavailable("Kubernetes workload could not be encoded")
                        })?;
                        Ok(DatasourceRecord {
                            key: json!({"name": name}),
                            view: RecordView::Compact,
                            value,
                        })
                    })
                    .collect::<Result<Vec<_>, DatasourceError>>()?;
                (records, next_cursor, Completeness::Complete)
            }
            DatasourceRead::Get { key } => {
                let key: WorkloadKey = serde_json::from_value(key).map_err(|_| {
                    DatasourceError::new(
                        DatasourceErrorCode::InvalidInput,
                        "Kubernetes workload key is invalid",
                        false,
                    )
                })?;
                if !valid_dns_label(&key.name, 253) {
                    return Err(DatasourceError::new(
                        DatasourceErrorCode::InvalidInput,
                        "Kubernetes workload name is invalid",
                        false,
                    ));
                }
                let detail = self.reader.workload_detail(&namespace, &key.name).await?;
                let completeness = if detail.related_complete {
                    Completeness::Complete
                } else {
                    Completeness::Partial
                };
                let value = serde_json::to_value(detail).map_err(|_| {
                    datasource_unavailable("Kubernetes workload detail could not be encoded")
                })?;
                (
                    vec![DatasourceRecord {
                        key: json!({"name": key.name}),
                        view: RecordView::Detail,
                        value,
                    }],
                    None,
                    completeness,
                )
            }
        };
        Ok(DatasourceResult::Read(DatasourcePage {
            datasource_ref: DATASOURCE.to_owned(),
            records,
            next_cursor,
            completeness,
            observed_at_unix_ms: now_unix_ms(),
            provenance: DatasourceProvenance {
                binding_ref: request.binding_ref,
                projection_sha256: datasource_projection_sha256(),
                connector_audit_ref: audit_ref(context, DATASOURCE, &namespace, "workloads"),
            },
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadKey {
    name: String,
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
                    STATUS_OPERATION | RESTART_OPERATION
                )
            }
            OperationRequest::Invoke(request) => {
                matches!(
                    request.operation_ref.as_str(),
                    STATUS_OPERATION | RESTART_OPERATION
                ) && request.connection_ref == CONNECTION
            }
            OperationRequest::Search(_)
            | OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_) => false,
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
                    ]
                    .iter()
                    .any(|term| query.contains(term));
                let mut operations = Vec::new();
                if matches_query && self.has_read_access(context) {
                    operations.push(status_summary());
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
                    .map(namespace_binding)
                    .collect();
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Read(read) => self.read_datasource(context, read).await,
            _ => Err(datasource_not_found("Kubernetes datasource was not found")),
        }
    }
}

fn datasource_summary() -> DatasourceSummary {
    DatasourceSummary {
        datasource_ref: DATASOURCE.to_owned(),
        title: "Kubernetes workloads".to_owned(),
        access_mode: AccessMode::Live,
        verbs: vec![ReadVerb::List, ReadVerb::Get],
    }
}

fn workload_key_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["name"],
        "properties": {"name": {"type": "string", "minLength": 1, "maxLength": 253}}
    })
}

fn compact_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "name", "uid", "resource_version", "generation", "observed_generation", "desired_replicas", "updated_replicas", "ready_replicas", "available_replicas", "unavailable_replicas", "rollout_state"],
        "properties": {
            "namespace": {"type": "string"}, "name": {"type": "string"},
            "uid": {"type": "string"}, "resource_version": {"type": "string"},
            "generation": {"type": "integer"}, "observed_generation": {"type": "integer"},
            "desired_replicas": {"type": "integer"}, "updated_replicas": {"type": "integer"},
            "ready_replicas": {"type": "integer"}, "available_replicas": {"type": "integer"},
            "unavailable_replicas": {"type": "integer"},
            "rollout_state": {"enum": ["available", "progressing", "degraded"]}
        }
    })
}

fn detail_schema() -> serde_json::Value {
    let mut schema = compact_schema();
    let object = schema.as_object_mut().expect("static object schema");
    object
        .get_mut("required")
        .and_then(serde_json::Value::as_array_mut)
        .expect("static required schema")
        .extend([json!("pods"), json!("warnings"), json!("related_complete")]);
    let properties = object
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .expect("static properties schema");
    properties.insert(
        "pods".to_owned(),
        json!({
            "type": "array",
            "maxItems": MAX_RELATED_RECORDS,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name", "phase", "ready_containers", "total_containers", "restart_count", "containers"],
                "properties": {
                    "name": {"type": "string"},
                    "phase": {"type": "string"},
                    "ready_containers": {"type": "integer", "minimum": 0},
                    "total_containers": {"type": "integer", "minimum": 0},
                    "restart_count": {"type": "integer", "minimum": 0},
                    "containers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["name", "image", "image_id", "ready", "restart_count", "state_reason"],
                            "properties": {
                                "name": {"type": "string"},
                                "image": {"type": "string"},
                                "image_id": {"type": "string"},
                                "ready": {"type": "boolean"},
                                "restart_count": {"type": "integer", "minimum": 0},
                                "state_reason": {"type": ["string", "null"]}
                            }
                        }
                    }
                }
            }
        }),
    );
    properties.insert(
        "warnings".to_owned(),
        json!({
            "type": "array",
            "maxItems": MAX_RELATED_RECORDS,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["involved_kind", "involved_name", "reason", "count", "first_observed_at", "last_observed_at"],
                "properties": {
                    "involved_kind": {"type": "string"},
                    "involved_name": {"type": "string"},
                    "reason": {"type": "string"},
                    "count": {"type": "integer", "minimum": 0},
                    "first_observed_at": {"type": ["string", "null"]},
                    "last_observed_at": {"type": ["string", "null"]}
                }
            }
        }),
    );
    properties.insert("related_complete".to_owned(), json!({"type": "boolean"}));
    schema
}

fn datasource_projection_sha256() -> String {
    let declaration = json!({
        "protocol": "b10x.value-projection.v1",
        "datasource_ref": DATASOURCE,
        "version": 1,
        "key_schema": workload_key_schema(),
        "compact_schema": compact_schema(),
        "detail_schema": detail_schema(),
        "excluded": ["annotations", "labels", "environment", "secret_references", "event_messages", "raw_objects"]
    });
    hex::encode(Sha256::digest(
        serde_json::to_vec(&declaration).expect("static projection declaration"),
    ))
}

fn datasource_description(context: &PrincipalContext) -> DatasourceDescription {
    DatasourceDescription {
        summary: datasource_summary(),
        description: "Live, namespace-scoped Deployment status with bounded Pod, image digest, restart-count, and warning-reason summaries. Raw Kubernetes objects, event messages, labels, annotations, environment values, and Secret data are never returned.".to_owned(),
        key_schema: workload_key_schema(),
        compact_schema: compact_schema(),
        detail_schema: detail_schema(),
        projection_protocol: "b10x.value-projection.v1".to_owned(),
        projection_sha256: datasource_projection_sha256(),
        description_ref: datasource_description_ref(context),
    }
}

/// The description lease for the workload datasource.
///
/// Seeded from the stable authority and never from the authority snapshot id or sha: the hosted
/// receiver fills those from the access token, every datasource request travels on the cached
/// `connectors.catalog.read` token, and that token is refreshed inside five minutes. A describe
/// and the read that follows can therefore straddle a refresh, and seeding the lease from the
/// token made that ordinary event look like an authority change.
pub(crate) fn datasource_description_ref(context: &PrincipalContext) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/kubernetes-datasource-description/v2\0");
    digest.update(context.stable_authority_seed());
    digest.update(b"\0");
    digest.update(datasource_projection_sha256().as_bytes());
    format!(
        "description:kubernetes:datasource:{}",
        hex::encode(&digest.finalize()[..16])
    )
}

pub(super) fn namespace_binding(namespace: &str) -> DatasourceBinding {
    let digest = Sha256::digest(format!("{DATASOURCE}\0{namespace}\0v1"));
    let mut generation_bytes = [0_u8; 8];
    generation_bytes.copy_from_slice(&digest[..8]);
    DatasourceBinding {
        datasource_ref: DATASOURCE.to_owned(),
        binding_ref: namespace_binding_ref(namespace),
        connection_ref: CONNECTION.to_owned(),
        label: namespace.to_owned(),
        generation: u64::from_be_bytes(generation_bytes),
    }
}

fn namespace_binding_ref(namespace: &str) -> String {
    let digest = Sha256::digest(format!("{DATASOURCE}\0{namespace}\0binding-v1"));
    format!("binding:kubernetes:{}", hex::encode(&digest[..16]))
}
